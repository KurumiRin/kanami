//! The rule engine.
//!
//! Processing has two stages:
//! 1. `[remap]`: physical key -> logical key, stateless, just a table lookup;
//! 2. combos, layers and rules: stateful. Tap/hold and simultaneous presses need later events to be decided,
//!    so events during that time go into the `pending` queue and are replayed in order once decided.
//!
//! The engine manages output modifier state centrally: each held key can contribute base modifiers,
//! temporarily removed modifiers and temporarily added modifiers, and `sync` compares the wanted state with what has been posted
//! and posts the difference. That way a rule that swaps modifiers, such as `cmd+k -> ctrl+l`, restores correctly on release.
//!
//! Rules are fully compiled at construction, and event handling only reads lookup tables without allocating
//! (output buffers and queues are preallocated and reused after clearing).
//!
//! Extensions such as scripts plug in through [`Hooks`]: the engine calls back at fixed points, and callbacks drive the engine through [`Api`].

use std::sync::Arc;

use crate::Hooks;
use crate::backend::DeviceInfo;
use crate::key::KeySet;
use crate::rules::*;
use crate::{Key, KeyEvent, KeyState, ModPattern, ModSet};

const MAX_LAYERS: usize = 32;
const MAX_COMBOS: usize = 64;
const MAX_SEQUENCE: usize = 32;
const MAX_DEVICES: usize = 32;
/// The most events buffered while a decision is pending; beyond that the decision is forced.
const MAX_PENDING: usize = 32;
/// Emergency stop: holding these physical keys together (before remapping) disables or re-enables all remapping. It does not depend on the config, so it works even when the config is broken.
pub const EMERGENCY_STOP: [Key; 3] = [Key::LeftControl, Key::Space, Key::Escape];

/// The output of one call. It must be taken before the next call.
#[derive(Debug, Default)]
pub struct Out {
    pub keys: Vec<KeyEvent>,
    /// Text to type, and its position in `keys` (typed before that key event).
    pub texts: Vec<(usize, Arc<str>)>,
    /// Shell commands to run in the background.
    pub runs: Vec<Arc<str>>,
}

impl Out {
    pub const fn new() -> Self {
        Out {
            keys: Vec::new(),
            texts: Vec::new(),
            runs: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty() && self.texts.is_empty() && self.runs.is_empty()
    }

    fn clear(&mut self) {
        self.keys.clear();
        self.texts.clear();
        self.runs.clear();
    }
}

#[derive(Clone, Copy)]
enum Remapped {
    Key(Key),
    Mods(ModSet),
    Disable,
}

#[derive(Clone, Copy)]
enum Act {
    Keys { start: u16, len: u16 },
    Disable,
    Run(u16),
    Call(u16),
}

#[derive(Clone, Copy)]
enum Hold {
    Chord(Chord),
    Layer(u8),
}

#[derive(Clone, Copy)]
struct TapHoldSpec {
    tap: Act,
    hold: Hold,
    strategy: Strategy,
}

#[derive(Clone, Copy)]
enum Body {
    Act(Act),
    TapHold(TapHoldSpec),
}

struct CompiledRule {
    key: Key,
    mods: Option<ModPattern>,
    when: Cond,
    scope: Cond,
    body: Body,
}

#[derive(Default)]
struct Cond {
    app: Vec<u16>,
    app_not: Vec<u16>,
    input: Vec<u16>,
    input_not: Vec<u16>,
    vars: Vec<(Box<str>, Box<str>)>,
    /// Bitmap of `[device.*]`.
    device: u32,
    device_not: u32,
}

struct CompiledCombo {
    keys: KeySet,
    act: Act,
    timeout: u64,
}

#[derive(Clone, Copy)]
struct Pending {
    phys: Key,
    target: Remapped,
    state: KeyState,
    time: u64,
    /// Which `[device.*]` the keyboard that produced this key matched (bitmap).
    devices: u32,
    /// Whether the user was typing in a streak when this was pressed (see [`Settings::typing_streak`](crate::Settings::typing_streak)).
    streak: bool,
}

/// A record left by each physical key press, used to finish up on release.
#[derive(Clone, Copy)]
enum Held {
    None,
    /// This key was posted; post its release on release.
    Pass(Key),
    /// Held as a modifier.
    Mods(ModSet),
    /// Holding a mapping: remove `suppress`, add `inject`, and hold `key`.
    Chord {
        key: Option<Key>,
        inject: ModSet,
        suppress: ModSet,
    },
    /// A layer entered by holding. `tap`: if no other key was pressed during the hold, post a tap on release (`presses` is the key press count on entry).
    Layer {
        layer: u8,
        tap: Option<Act>,
        presses: u32,
    },
    /// Already handled; later repeats and the release are swallowed.
    Swallow,
    /// Waiting for a decision.
    Pending,
}

#[derive(Clone, Copy)]
enum Resolver {
    TapHold {
        trigger: Key,
        deadline: u64,
        spec: TapHoldSpec,
    },
    Combo {
        first: Pending,
        key: Key,
        deadline: u64,
        candidates: u64,
    },
}

enum Decision {
    /// Tap; the argument is the position of the trigger key's release event in the queue.
    Tap(usize),
    Hold,
    /// Combo matched; `consumed` is the number of events at the head of the queue that belong to this combo.
    Fire {
        combo: usize,
        consumed: usize,
    },
    Fail,
}

pub struct Engine {
    remap: [Option<Remapped>; Key::COUNT],
    /// Per-keyboard remaps: (`[device.*]` bitmap, remap table), looked up before the general remaps.
    device_remaps: Vec<(u32, Box<[Option<Remapped>; Key::COUNT]>)>,
    /// Keyboards that use only their own rules and remaps (bitmap).
    exclusive: u32,
    chords: Vec<Chord>,
    commands: Vec<Arc<str>>,
    functions: Vec<Arc<str>>,
    apps: Vec<Box<str>>,
    input_sources: Vec<Box<str>>,
    layer_names: Vec<Box<str>>,
    device_specs: Vec<DeviceSpec>,
    rules: Vec<CompiledRule>,
    layer_maps: Vec<[Option<Act>; Key::COUNT]>,
    layer_holds: [Option<TapHoldSpec>; Key::COUNT],
    layer_toggles: Vec<(Key, Option<ModPattern>, u8)>,
    combos: Vec<CompiledCombo>,
    /// Which combos each key takes part in (bitmap).
    combo_index: [u64; Key::COUNT],
    inputs: KeySet,
    tap_timeout: u64,
    typing_streak: u64,

    held: [Held; Key::COUNT],
    /// Keys physically held (before remapping), used to detect the emergency stop.
    physical: KeySet,
    /// True after an emergency stop: all keys pass through unchanged.
    stopped: bool,
    /// Number of presses handled, used to tell whether another key was pressed while a layer key was held.
    presses: u32,
    /// When the last non-modifier key was pressed, used to tell whether the user is typing in a streak.
    last_press: Option<u64>,
    /// Modifiers temporarily added/removed while tapping a chord.
    temp: (ModSet, ModSet),
    out_mods: ModSet,
    /// Active layers; later-activated ones come later.
    layers: Vec<u8>,
    /// The current frontmost app / input method: its index in the condition lists, and the raw name (for scripts to read).
    app: Option<u16>,
    app_name: Option<Box<str>>,
    /// The keyboard of the current key: the backend ID and the bitmap of `[device.*]` it matches.
    device: Option<u32>,
    devices: u32,
    input_source: Option<u16>,
    input_source_name: Option<Box<str>>,
    /// Script variables.
    vars: Vec<(Box<str>, Box<str>)>,
    /// Script timers: (expiry time, ID).
    timers: Vec<(u64, u32)>,
    next_timer: u32,
    now: u64,
    hooks: Option<Box<dyn Hooks>>,
    hooks_want_keys: bool,
    pending: Vec<Pending>,
    resolver: Option<Resolver>,
    out: Out,
}

impl Engine {
    pub fn new(rules: &Rules) -> Result<Self, RuleError> {
        let mut e = Engine {
            remap: [None; Key::COUNT],
            device_remaps: Vec::new(),
            exclusive: 0,
            chords: Vec::new(),
            commands: Vec::new(),
            functions: Vec::new(),
            apps: Vec::new(),
            input_sources: Vec::new(),
            layer_names: rules
                .layers
                .iter()
                .map(|l| l.name.as_str().into())
                .collect(),
            device_specs: rules.devices.clone(),
            rules: Vec::new(),
            layer_maps: Vec::new(),
            layer_holds: [None; Key::COUNT],
            layer_toggles: Vec::new(),
            combos: Vec::new(),
            combo_index: [0; Key::COUNT],
            inputs: KeySet::empty(),
            tap_timeout: rules.settings.tap_timeout.into(),
            typing_streak: rules.settings.typing_streak.into(),
            held: [Held::None; Key::COUNT],
            physical: KeySet::empty(),
            stopped: false,
            presses: 0,
            last_press: None,
            temp: (ModSet::empty(), ModSet::empty()),
            out_mods: ModSet::empty(),
            layers: Vec::with_capacity(MAX_LAYERS),
            app: None,
            app_name: None,
            device: None,
            devices: 0,
            input_source: None,
            input_source_name: None,
            vars: Vec::new(),
            timers: Vec::new(),
            next_timer: 0,
            now: 0,
            hooks: None,
            hooks_want_keys: false,
            pending: Vec::with_capacity(MAX_PENDING + 1),
            resolver: None,
            out: Out {
                keys: Vec::with_capacity(256),
                texts: Vec::new(),
                runs: Vec::with_capacity(4),
            },
        };
        e.compile(rules)?;
        Ok(e)
    }

    /// Processes one physical key event. `now` is a monotonically increasing time in milliseconds.
    pub fn process(&mut self, event: KeyEvent, now: u64) -> &mut Out {
        self.out.clear();
        self.now = now;
        match event.state {
            KeyState::Down => self.physical.insert(event.key),
            KeyState::Up => self.physical.remove(event.key),
            KeyState::Repeat => {}
        }
        if event.state == KeyState::Down
            && EMERGENCY_STOP.contains(&event.key)
            && EMERGENCY_STOP.iter().all(|&k| self.physical.contains(k))
        {
            // Swallow the key that completes the combo instead of sending it to the system
            if self.stopped {
                self.physical.remove(event.key);
                self.set_stopped(false);
                self.physical.insert(event.key);
                self.held[event.key as usize] = Held::Swallow;
            } else {
                self.set_stopped(true);
            }
            return &mut self.out;
        }
        if self.stopped {
            self.out.keys.push(event);
            return &mut self.out;
        }
        let target = self.remapped(event.key);
        let mut streak = false;
        if event.state == KeyState::Down {
            streak = self
                .last_press
                .is_some_and(|t| now < t + self.typing_streak);
            let modifier = match target {
                Remapped::Mods(_) => true,
                Remapped::Key(key) => ModSet::of_key(key).is_some(),
                Remapped::Disable => false,
            };
            if !modifier {
                self.last_press = Some(now);
            }
        }
        self.pending.push(Pending {
            phys: event.key,
            target,
            state: event.state,
            time: now,
            devices: self.devices,
            streak,
        });
        self.run(now);
        &mut self.out
    }

    /// Call when [`next_deadline`](Self::next_deadline) is reached, to handle timeout decisions.
    pub fn tick(&mut self, now: u64) -> &mut Out {
        self.out.clear();
        self.now = now;
        self.run(now);
        &mut self.out
    }

    /// The topmost active layer.
    pub fn active_layer(&self) -> Option<&str> {
        if self.stopped {
            return None;
        }
        self.layers.last().map(|&l| &*self.layer_names[l as usize])
    }

    /// The output of the most recent call.
    pub fn output(&self) -> &Out {
        &self.out
    }

    /// Whether the emergency stop is active (all keys pass through unchanged).
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// Disables or re-enables all remapping (the emergency stop, or Pause in the UI). Disabling releases the keys the engine pressed;
    /// re-enabling records the physical modifiers currently held as pressed, because they were sent to the system unchanged while disabled.
    pub fn set_stopped(&mut self, stop: bool) -> &mut Out {
        if stop == self.stopped {
            self.out.clear();
            return &mut self.out;
        }
        if stop {
            self.reset();
        } else {
            self.out.clear();
            for &key in Key::ALL {
                if self.physical.contains(key)
                    && let Some(mods) = ModSet::of_key(key)
                {
                    self.held[key as usize] = Held::Mods(mods);
                    self.out_mods = self.out_mods.union(mods);
                }
            }
        }
        self.stopped = stop;
        &mut self.out
    }

    /// The modifiers already posted and currently down. The backend uses them to correct the modifier flags of other events such as mouse events.
    pub fn mods(&self) -> ModSet {
        self.out_mods
    }

    /// This physical key was pressed and its release has not arrived yet.
    pub fn is_held(&self, key: Key) -> bool {
        !matches!(self.held[key as usize], Held::None)
    }

    pub fn next_deadline(&self) -> Option<u64> {
        let resolver = self.resolver.map(|r| match r {
            Resolver::TapHold { deadline, .. } | Resolver::Combo { deadline, .. } => deadline,
        });
        let timer = self.timers.iter().map(|&(t, _)| t).min();
        match (resolver, timer) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// Plugs in an extension such as a script.
    pub fn set_hooks(&mut self, hooks: Box<dyn Hooks>) {
        self.hooks_want_keys = hooks.wants_keys();
        self.hooks = Some(hooks);
    }

    /// The function names used by `call` in rules, for checking that the functions exist after a script is loaded.
    pub fn functions(&self) -> impl Iterator<Item = &str> {
        self.functions.iter().map(|f| &**f)
    }

    /// Releases every key the engine pressed and returns to the initial state. Call before reloading the config or exiting.
    pub fn reset(&mut self) -> &mut Out {
        self.out.clear();
        self.pending.clear();
        self.resolver = None;
        for i in 0..Key::COUNT {
            match std::mem::replace(&mut self.held[i], Held::None) {
                Held::Pass(key) | Held::Chord { key: Some(key), .. } => {
                    self.emit(key, KeyState::Up)
                }
                _ => {}
            }
        }
        self.layers.clear();
        self.timers.clear();
        self.last_press = None;
        self.temp = (ModSet::empty(), ModSet::empty());
        self.sync();
        &mut self.out
    }

    /// Call when the frontmost app changes.
    pub fn set_app(&mut self, bundle_id: Option<&str>) {
        self.app = bundle_id.and_then(|id| find(&self.apps, id));
        self.app_name = bundle_id.map(Into::into);
    }

    /// Tells the engine which keyboard a key came from, before [`process`](Self::process).
    pub fn set_device(&mut self, device: Option<&DeviceInfo>) {
        let id = device.map(|d| d.id);
        if id == self.device {
            return;
        }
        self.device = id;
        self.devices = device.map_or(0, |d| {
            self.device_specs
                .iter()
                .enumerate()
                .filter(|(_, spec)| spec.matches(d))
                .fold(0, |bits, (i, _)| bits | 1 << i)
        });
    }

    /// Whether the rules use `[device.*]` (the backend only needs to identify keyboards then).
    pub fn uses_devices(&self) -> bool {
        !self.device_specs.is_empty()
    }

    /// For tests: assume the key comes from the `[device.*]` named `name`.
    pub fn assume_device(&mut self, name: Option<&str>) {
        self.device = None;
        self.devices = name
            .and_then(|n| self.device_specs.iter().position(|s| s.name == n))
            .map_or(0, |i| 1 << i);
    }

    /// Call when the input method changes; the argument is the input method ID (such as `com.apple.keylayout.ABC`).
    pub fn set_input_source(&mut self, id: Option<&str>) {
        self.input_source = id.and_then(|id| find(&self.input_sources, id));
        self.input_source_name = id.map(Into::into);
    }

    /// Whether a key appears on the input side of any rule.
    pub fn uses_key(&self, key: Key) -> bool {
        self.inputs.contains(key)
    }

    // ---- Compilation ----

    fn compile(&mut self, rules: &Rules) -> Result<(), RuleError> {
        if rules.devices.len() > MAX_DEVICES {
            let kind = RuleErrorKind::TooMany {
                what: "devices",
                max: MAX_DEVICES,
            };
            return Err(err(Location::Rule(0), kind));
        }
        for (i, device) in rules.devices.iter().enumerate() {
            if device.exclusive {
                self.exclusive |= 1 << i;
            }
        }
        for remap in &rules.remaps {
            let slot = match remap.to {
                RemapTarget::Key(key) => Remapped::Key(key),
                RemapTarget::Mods(mods) => Remapped::Mods(mods),
                RemapTarget::Disable => Remapped::Disable,
            };
            match remap.device {
                None => self.remap[remap.from as usize] = Some(slot),
                Some(device) => {
                    let bit = 1 << device;
                    let i = match self.device_remaps.iter().position(|(b, _)| *b == bit) {
                        Some(i) => i,
                        None => {
                            self.device_remaps.push((bit, Box::new([None; Key::COUNT])));
                            self.device_remaps.len() - 1
                        }
                    };
                    self.device_remaps[i].1[remap.from as usize] = Some(slot);
                }
            }
            self.inputs.insert(remap.from);
        }

        for (i, rule) in rules.rules.iter().enumerate() {
            let at = Location::Rule(i);
            let body = match &rule.kind {
                RuleKind::Action(action) => Body::Act(self.act(action, at)?),
                RuleKind::TapHold(th) => {
                    if rule.from.mods.is_some() {
                        return Err(err(at, RuleErrorKind::TapHoldNeedsBareKey));
                    }
                    Body::TapHold(TapHoldSpec {
                        tap: self.act(&th.tap, at)?,
                        hold: Hold::Chord(normalize(th.hold)),
                        strategy: th.strategy,
                    })
                }
            };
            let when = self.cond(&rule.when, at)?;
            let scope = self.cond(&rule.scope, at)?;
            self.rules.push(CompiledRule {
                key: rule.from.key,
                mods: rule.from.mods,
                when,
                scope,
                body,
            });
            self.inputs.insert(rule.from.key);
        }

        if rules.layers.len() > MAX_LAYERS {
            let kind = RuleErrorKind::TooMany {
                what: "layers",
                max: MAX_LAYERS,
            };
            return Err(err(Location::Layer(MAX_LAYERS), kind));
        }
        for (i, layer) in rules.layers.iter().enumerate() {
            let at = Location::Layer(i);
            let mut map = [None; Key::COUNT];
            for (key, action) in &layer.map {
                map[*key as usize] = Some(self.act(action, at)?);
            }
            self.layer_maps.push(map);
            match &layer.activation {
                Activation::Hold { key, tap, strategy } => {
                    if self.layer_holds[*key as usize].is_some() {
                        return Err(err(at, RuleErrorKind::DuplicateLayerTrigger(*key)));
                    }
                    let tap = match tap {
                        Some(action) => self.act(action, at)?,
                        None if *key == Key::CapsLock => Act::Disable,
                        None => self.act(&Action::Keys(vec![Chord::key(*key)]), at)?,
                    };
                    self.layer_holds[*key as usize] = Some(TapHoldSpec {
                        tap,
                        hold: Hold::Layer(i as u8),
                        strategy: *strategy,
                    });
                    self.inputs.insert(*key);
                }
                Activation::Toggle(trigger) => {
                    self.layer_toggles
                        .push((trigger.key, trigger.mods, i as u8));
                    self.inputs.insert(trigger.key);
                }
            }
        }

        if rules.combos.len() > MAX_COMBOS {
            let kind = RuleErrorKind::TooMany {
                what: "combos",
                max: MAX_COMBOS,
            };
            return Err(err(Location::Combo(MAX_COMBOS), kind));
        }
        for (i, combo) in rules.combos.iter().enumerate() {
            let at = Location::Combo(i);
            if combo.keys.len() < 2 {
                return Err(err(at, RuleErrorKind::ComboTooFewKeys));
            }
            let mut keys = KeySet::empty();
            for &key in &combo.keys {
                if key.is_modifier() {
                    return Err(err(at, RuleErrorKind::ComboModifier(key)));
                }
                if keys.contains(key) {
                    return Err(err(at, RuleErrorKind::ComboDuplicateKey(key)));
                }
                keys.insert(key);
                self.combo_index[key as usize] |= 1 << i;
                self.inputs.insert(key);
            }
            let timeout = combo.timeout.unwrap_or(rules.settings.combo_timeout);
            let act = self.act(&combo.action, at)?;
            self.combos.push(CompiledCombo {
                keys,
                act,
                timeout: timeout.into(),
            });
        }
        Ok(())
    }

    fn act(&mut self, action: &Action, at: Location) -> Result<Act, RuleError> {
        Ok(match action {
            Action::Keys(chords) => {
                if chords.is_empty() {
                    return Err(err(at, RuleErrorKind::EmptyKeys));
                }
                if chords.len() > MAX_SEQUENCE {
                    let kind = RuleErrorKind::TooMany {
                        what: "keys in a sequence",
                        max: MAX_SEQUENCE,
                    };
                    return Err(err(at, kind));
                }
                let start = self.chords.len() as u16;
                for &chord in chords {
                    self.chords.push(normalize(chord));
                }
                Act::Keys {
                    start,
                    len: chords.len() as u16,
                }
            }
            Action::Disable => Act::Disable,
            Action::Run(command) => {
                self.commands.push(Arc::from(command.as_str()));
                Act::Run(self.commands.len() as u16 - 1)
            }
            Action::Call(function) => {
                let id = match self.functions.iter().position(|f| **f == **function) {
                    Some(i) => i,
                    None => {
                        self.functions.push(Arc::from(function.as_str()));
                        self.functions.len() - 1
                    }
                };
                Act::Call(id as u16)
            }
        })
    }

    fn cond(&mut self, when: &Condition, at: Location) -> Result<Cond, RuleError> {
        let intern = |list: &mut Vec<Box<str>>, names: &[String]| -> Vec<u16> {
            names
                .iter()
                .map(|name| {
                    find(list, name).unwrap_or_else(|| {
                        list.push(name.as_str().into());
                        list.len() as u16 - 1
                    })
                })
                .collect()
        };
        let devices = |names: &[String]| -> Result<u32, RuleError> {
            names.iter().try_fold(0, |bits, name| {
                match self.device_specs.iter().position(|s| s.name == *name) {
                    Some(i) => Ok(bits | 1 << i),
                    None => Err(err(at, RuleErrorKind::UnknownDevice(name.clone()))),
                }
            })
        };
        let (device, device_not) = (devices(&when.device)?, devices(&when.device_not)?);
        Ok(Cond {
            app: intern(&mut self.apps, &when.app),
            app_not: intern(&mut self.apps, &when.app_not),
            input: intern(&mut self.input_sources, &when.input_source),
            input_not: intern(&mut self.input_sources, &when.input_source_not),
            vars: when
                .vars
                .iter()
                .map(|(k, v)| (k.as_str().into(), v.as_str().into()))
                .collect(),
            device,
            device_not,
        })
    }

    // ---- Event handling ----

    fn run(&mut self, now: u64) {
        loop {
            if self.resolver.is_some() {
                match self.decide(now) {
                    Some(decision) => self.apply(decision),
                    None => break,
                }
                continue;
            }
            if self.pending.is_empty() {
                break;
            }
            let p = self.pending.remove(0);
            match p.state {
                KeyState::Down => self.press(p, false),
                KeyState::Up => self.release(p),
                KeyState::Repeat => self.repeat(p),
            }
        }
        self.fire_timers(now);
    }

    fn fire_timers(&mut self, now: u64) {
        // Fire in order of expiry; timers created in a callback that are already due also fire in this round
        while let Some(i) = (0..self.timers.len())
            .filter(|&i| self.timers[i].0 <= now)
            .min_by_key(|&i| self.timers[i])
        {
            let (_, id) = self.timers.remove(i);
            self.with_hooks(|hooks, api| hooks.timer(api, id));
        }
    }

    /// Takes the extension out to call it, while it drives the engine through [`Api`].
    fn with_hooks<R>(&mut self, f: impl FnOnce(&mut dyn Hooks, &mut Api<'_>) -> R) -> Option<R> {
        let mut hooks = self.hooks.take()?;
        let result = f(hooks.as_mut(), &mut Api(self));
        self.hooks = Some(hooks);
        Some(result)
    }

    /// Hands a logical key event to the extension's `on_key` and returns whether the extension handled it.
    fn hook_key(&mut self, key: Key, state: KeyState) -> bool {
        if !self.hooks_want_keys {
            return false;
        }
        let event = KeyEvent {
            key,
            state,
            mods: self.out_mods,
        };
        self.with_hooks(|hooks, api| hooks.on_key(api, event))
            .unwrap_or(false)
    }

    /// `replay` being true means the first key is replayed after a combo failed to match: it was already given to the extension and no longer takes part in combos.
    fn press(&mut self, p: Pending, replay: bool) {
        if !replay {
            self.presses = self.presses.wrapping_add(1);
        }
        let phys = p.phys as usize;
        let key = match p.target {
            Remapped::Disable => {
                self.held[phys] = Held::Swallow;
                return;
            }
            Remapped::Mods(mods) => {
                self.held[phys] = Held::Mods(mods);
                self.sync();
                return;
            }
            Remapped::Key(key) => key,
        };
        let k = key as usize;

        if !replay && self.hook_key(key, KeyState::Down) {
            self.held[phys] = Held::Swallow;
            return;
        }

        // Do not wait for a combo decision while typing in a streak
        if !replay && !p.streak && self.combo_index[k] != 0 {
            let candidates = self.combo_index[k];
            self.held[phys] = Held::Pending;
            self.resolver = Some(Resolver::Combo {
                first: p,
                key,
                deadline: p.time + self.combo_timeout(candidates),
                candidates,
            });
            return;
        }

        for i in (0..self.layers.len()).rev() {
            if let Some(act) = self.layer_maps[self.layers[i] as usize][k] {
                self.perform(p.phys, act, ModSet::empty());
                return;
            }
        }

        // Do not enter a layer while a modifier is held (system shortcuts such as Cmd+Space or Ctrl+Space); the trigger key acts as an ordinary key immediately
        if let Some(spec) = self.layer_holds[k]
            && self.out_mods.is_empty()
        {
            self.start_tap_hold(p, spec, ModSet::empty());
            return;
        }

        for i in 0..self.layer_toggles.len() {
            let (trigger, mods, layer) = self.layer_toggles[i];
            if trigger == key && mods.is_none_or(|m| m.matches(self.out_mods)) {
                self.toggle(layer);
                self.held[phys] = Held::Swallow;
                return;
            }
        }

        for i in 0..self.rules.len() {
            let rule = &self.rules[i];
            if rule.key != key
                || !self.cond_ok(&rule.when, p.devices)
                || !self.cond_ok(&rule.scope, p.devices)
            {
                continue;
            }
            let suppress = match rule.mods {
                None => ModSet::empty(),
                Some(pattern) if pattern.matches(self.out_mods) => pattern.matched(self.out_mods),
                Some(_) => continue,
            };
            match rule.body {
                Body::Act(act) => self.perform(p.phys, act, suppress),
                Body::TapHold(spec) => self.start_tap_hold(p, spec, suppress),
            }
            return;
        }

        match ModSet::of_key(key) {
            Some(mods) => {
                self.held[phys] = Held::Mods(mods);
                self.sync();
            }
            None => {
                self.emit(key, KeyState::Down);
                self.held[phys] = Held::Pass(key);
            }
        }
    }

    /// A release always finishes the normal way; extensions can only observe it, not swallow it, to avoid stuck keys.
    fn release(&mut self, p: Pending) {
        match std::mem::replace(&mut self.held[p.phys as usize], Held::None) {
            // The press happened before kanami took over (or before a reload): pass it through so the system sees the release
            Held::None | Held::Pending => self.emit(p.phys, KeyState::Up),
            Held::Pass(key) => self.emit(key, KeyState::Up),
            Held::Mods(_) => self.sync(),
            Held::Chord { key, .. } => {
                if let Some(key) = key {
                    self.emit(key, KeyState::Up);
                }
                self.sync();
            }
            Held::Layer {
                layer,
                tap,
                presses,
            } => {
                self.deactivate(layer);
                // Held a bit longer but the layer went unused: still treat it as a tap and do not swallow it
                if let Some(act) = tap
                    && presses == self.presses
                {
                    self.tap(act);
                }
            }
            Held::Swallow => {}
        }
        if let Remapped::Key(key) = p.target {
            self.hook_key(key, KeyState::Up);
        }
    }

    fn repeat(&mut self, p: Pending) {
        if let Remapped::Key(key) = p.target
            && self.hook_key(key, KeyState::Repeat)
        {
            return;
        }
        match self.held[p.phys as usize] {
            Held::Pass(key) | Held::Chord { key: Some(key), .. } => {
                self.emit(key, KeyState::Repeat)
            }
            _ => {}
        }
    }

    /// Runs an action on press. A single key expression is held until the trigger key is released.
    fn perform(&mut self, phys: Key, act: Act, suppress: ModSet) {
        let phys = phys as usize;
        match act {
            Act::Keys { start, len: 1 } => {
                let chord = self.chords[start as usize];
                self.held[phys] = Held::Chord {
                    key: chord.key,
                    inject: chord.mods,
                    suppress,
                };
                self.sync();
                if let Some(key) = chord.key {
                    self.emit(key, KeyState::Down);
                }
            }
            Act::Keys { start, len } => {
                for i in start..start + len {
                    let chord = self.chords[i as usize];
                    self.tap_chord(chord, suppress);
                }
                self.held[phys] = Held::Swallow;
            }
            Act::Disable => self.held[phys] = Held::Swallow,
            Act::Run(_) | Act::Call(_) => {
                self.held[phys] = Held::Swallow;
                self.tap(act);
            }
        }
    }

    /// Runs an action with tap semantics (the tap of tap/hold, and combos).
    fn tap(&mut self, act: Act) {
        match act {
            Act::Keys { start, len } => {
                for i in start..start + len {
                    let chord = self.chords[i as usize];
                    self.tap_chord(chord, ModSet::empty());
                }
            }
            Act::Disable => {}
            Act::Run(id) => {
                let command = Arc::clone(&self.commands[id as usize]);
                self.out.runs.push(command);
            }
            Act::Call(id) => {
                let function = Arc::clone(&self.functions[id as usize]);
                self.with_hooks(|hooks, api| hooks.call(api, &function));
            }
        }
    }

    fn tap_chord(&mut self, chord: Chord, suppress: ModSet) {
        self.temp = (chord.mods, suppress);
        self.sync();
        if let Some(key) = chord.key {
            self.emit(key, KeyState::Down);
            self.emit(key, KeyState::Up);
        }
        self.temp = (ModSet::empty(), ModSet::empty());
        self.sync();
    }

    fn hold(&mut self, trigger: Key, hold: Hold, tap: Option<Act>) {
        let t = trigger as usize;
        match hold {
            Hold::Layer(layer) => {
                self.activate(layer);
                self.held[t] = Held::Layer {
                    layer,
                    tap,
                    presses: self.presses,
                };
            }
            Hold::Chord(Chord { mods, key: None }) => {
                self.held[t] = Held::Mods(mods);
                self.sync();
            }
            Hold::Chord(Chord {
                mods,
                key: Some(key),
            }) => {
                self.held[t] = Held::Chord {
                    key: Some(key),
                    inject: mods,
                    suppress: ModSet::empty(),
                };
                self.sync();
                self.emit(key, KeyState::Down);
            }
        }
    }

    /// In a typing streak, treat it as a tap and hold until release, without waiting for a decision.
    fn start_tap_hold(&mut self, p: Pending, spec: TapHoldSpec, suppress: ModSet) {
        if p.streak {
            self.perform(p.phys, spec.tap, suppress);
            return;
        }
        self.held[p.phys as usize] = Held::Pending;
        self.resolver = Some(Resolver::TapHold {
            trigger: p.phys,
            deadline: p.time + self.tap_timeout,
            spec,
        });
    }

    fn decide(&self, now: u64) -> Option<Decision> {
        let full = self.pending.len() >= MAX_PENDING;
        match self.resolver? {
            Resolver::TapHold {
                trigger,
                deadline,
                spec,
            } => {
                let mut downs = KeySet::empty();
                for (i, p) in self.pending.iter().enumerate() {
                    if p.time >= deadline {
                        return Some(Decision::Hold);
                    }
                    if p.phys == trigger {
                        if p.state == KeyState::Up {
                            return Some(Decision::Tap(i));
                        }
                        continue;
                    }
                    match (p.state, spec.strategy) {
                        (KeyState::Down, Strategy::Eager) => return Some(Decision::Hold),
                        (KeyState::Down, _) => downs.insert(p.phys),
                        (KeyState::Up, Strategy::Permissive) if downs.contains(p.phys) => {
                            return Some(Decision::Hold);
                        }
                        _ => {}
                    }
                }
                (now >= deadline || full).then_some(Decision::Hold)
            }
            Resolver::Combo {
                key,
                deadline,
                candidates,
                ..
            } => {
                let mut candidates = candidates;
                let mut pressed = KeySet::empty();
                pressed.insert(key);
                let mut consumed = 0;
                let mut settled = false;
                for (i, p) in self.pending.iter().enumerate() {
                    let joins = match p.target {
                        Remapped::Key(k)
                            if p.state == KeyState::Down
                                && p.time < deadline
                                && !pressed.contains(k)
                                && candidates & self.combo_index[k as usize] != 0 =>
                        {
                            Some(k)
                        }
                        _ => None,
                    };
                    let Some(k) = joins else {
                        settled = true;
                        break;
                    };
                    candidates &= self.combo_index[k as usize];
                    pressed.insert(k);
                    consumed = i + 1;
                    // The only remaining candidate is fully pressed, so there is no need to wait
                    if let Some(combo) = self.exact_combo(candidates, pressed)
                        && candidates == 1 << combo
                    {
                        return Some(Decision::Fire { combo, consumed });
                    }
                }
                if !settled && now < deadline && !full {
                    return None;
                }
                Some(match self.exact_combo(candidates, pressed) {
                    Some(combo) => Decision::Fire { combo, consumed },
                    None => Decision::Fail,
                })
            }
        }
    }

    fn apply(&mut self, decision: Decision) {
        let Some(resolver) = self.resolver.take() else {
            return;
        };
        match (resolver, decision) {
            (Resolver::TapHold { trigger, spec, .. }, Decision::Tap(i)) => {
                self.pending.remove(i);
                self.held[trigger as usize] = Held::None;
                self.tap(spec.tap);
            }
            (Resolver::TapHold { trigger, spec, .. }, _) => {
                self.hold(trigger, spec.hold, Some(spec.tap))
            }
            (Resolver::Combo { first, .. }, Decision::Fire { combo, consumed }) => {
                self.held[first.phys as usize] = Held::Swallow;
                for i in 0..consumed {
                    self.held[self.pending[i].phys as usize] = Held::Swallow;
                }
                self.pending.drain(..consumed);
                self.tap(self.combos[combo].act);
            }
            (Resolver::Combo { first, .. }, _) => {
                self.held[first.phys as usize] = Held::None;
                self.press(first, true);
            }
        }
    }

    fn exact_combo(&self, candidates: u64, pressed: KeySet) -> Option<usize> {
        (0..self.combos.len())
            .find(|&c| candidates & (1 << c) != 0 && self.combos[c].keys == pressed)
    }

    fn combo_timeout(&self, candidates: u64) -> u64 {
        (0..self.combos.len())
            .filter(|&c| candidates & (1 << c) != 0)
            .map(|c| self.combos[c].timeout)
            .max()
            .unwrap_or(0)
    }

    fn cond_ok(&self, cond: &Cond, devices: u32) -> bool {
        let matches = |current: Option<u16>, yes: &[u16], no: &[u16]| {
            let in_list = |list: &[u16]| current.is_some_and(|c| list.contains(&c));
            (yes.is_empty() || in_list(yes)) && !in_list(no)
        };
        (cond.device == 0 || devices & cond.device != 0)
            && devices & cond.device_not == 0
            && matches(self.app, &cond.app, &cond.app_not)
            && matches(self.input_source, &cond.input, &cond.input_not)
            && cond
                .vars
                .iter()
                .all(|(name, value)| self.var(name).unwrap_or("") == &**value)
    }

    /// `[remap]` lookup: first the current keyboard's own remaps, then the general ones (skipped when the keyboard is set to use only its own).
    fn remapped(&self, key: Key) -> Remapped {
        for (bits, table) in &self.device_remaps {
            if self.devices & bits != 0
                && let Some(target) = table[key as usize]
            {
                return target;
            }
        }
        if self.devices & self.exclusive == 0
            && let Some(target) = self.remap[key as usize]
        {
            return target;
        }
        Remapped::Key(key)
    }

    fn var(&self, name: &str) -> Option<&str> {
        self.vars
            .iter()
            .find(|(n, _)| &**n == name)
            .map(|(_, v)| &**v)
    }

    fn activate(&mut self, layer: u8) {
        self.deactivate(layer);
        self.layers.push(layer);
    }

    fn deactivate(&mut self, layer: u8) {
        self.layers.retain(|&l| l != layer);
    }

    fn toggle(&mut self, layer: u8) {
        if self.layers.contains(&layer) {
            self.deactivate(layer);
        } else {
            self.layers.push(layer);
        }
    }

    // ---- Output ----

    fn emit(&mut self, key: Key, state: KeyState) {
        self.out.keys.push(KeyEvent {
            key,
            state,
            mods: self.out_mods,
        });
    }

    fn desired_mods(&self) -> ModSet {
        let (mut base, mut inject, mut suppress) = (ModSet::empty(), self.temp.0, self.temp.1);
        for held in &self.held {
            match *held {
                Held::Mods(mods) => base = base.union(mods),
                Held::Chord {
                    inject: i,
                    suppress: s,
                    ..
                } => {
                    inject = inject.union(i);
                    suppress = suppress.union(s);
                }
                _ => {}
            }
        }
        base.difference(suppress).union(inject)
    }

    /// Posts modifier presses/releases so the posted state equals the wanted state. Releases first, then presses.
    fn sync(&mut self) {
        let desired = self.desired_mods();
        for key in self.out_mods.difference(desired).keys() {
            self.out_mods = self
                .out_mods
                .difference(ModSet::of_key(key).unwrap_or_default());
            self.emit(key, KeyState::Up);
        }
        for key in desired.difference(self.out_mods).keys() {
            self.out_mods = self.out_mods.union(ModSet::of_key(key).unwrap_or_default());
            self.emit(key, KeyState::Down);
        }
    }
}

fn err(at: Location, kind: RuleErrorKind) -> RuleError {
    RuleError { at, kind }
}

fn find(list: &[Box<str>], name: &str) -> Option<u16> {
    list.iter().position(|n| &**n == name).map(|i| i as u16)
}

/// The interface for driving the engine during extension callbacks.
pub struct Api<'a>(&'a mut Engine);

impl Api<'_> {
    /// Taps a key expression.
    pub fn tap(&mut self, chord: Chord) {
        self.0.tap_chord(normalize(chord), ModSet::empty());
    }

    /// Types a piece of text (any Unicode, independent of keyboard layout and input method).
    pub fn type_text(&mut self, text: &str) {
        let at = self.0.out.keys.len();
        self.0.out.texts.push((at, Arc::from(text)));
    }

    /// Runs a shell command in the background.
    pub fn run(&mut self, command: &str) {
        self.0.out.runs.push(Arc::from(command));
    }

    /// Calls [`Hooks::timer`] after `ms` milliseconds and returns the timer ID.
    pub fn after(&mut self, ms: u64) -> u32 {
        let id = self.0.next_timer;
        self.0.next_timer = id.wrapping_add(1);
        self.0.timers.push((self.0.now + ms, id));
        id
    }

    pub fn cancel(&mut self, id: u32) {
        self.0.timers.retain(|&(_, t)| t != id);
    }

    /// Toggles a layer; returns `false` if the layer does not exist.
    pub fn set_layer(&mut self, name: &str, on: bool) -> bool {
        let Some(layer) = find(&self.0.layer_names, name) else {
            return false;
        };
        if on {
            self.0.activate(layer as u8);
        } else {
            self.0.deactivate(layer as u8);
        }
        true
    }

    pub fn layer_active(&self, name: &str) -> bool {
        find(&self.0.layer_names, name).is_some_and(|l| self.0.layers.contains(&(l as u8)))
    }

    /// Sets a script variable; `None` deletes it. Variables can be used in a rule's `when.var`.
    pub fn set_var(&mut self, name: &str, value: Option<&str>) {
        self.0.vars.retain(|(n, _)| &**n != name);
        if let Some(value) = value {
            self.0.vars.push((name.into(), value.into()));
        }
    }

    pub fn var(&self, name: &str) -> Option<&str> {
        self.0.var(name)
    }

    pub fn app(&self) -> Option<&str> {
        self.0.app_name.as_deref()
    }

    pub fn input_source(&self) -> Option<&str> {
        self.0.input_source_name.as_deref()
    }

    /// The modifiers currently held (already posted).
    pub fn mods(&self) -> ModSet {
        self.0.out_mods
    }

    pub fn now(&self) -> u64 {
        self.0.now
    }
}

/// Modifiers in output all go into `mods`; `key` keeps only ordinary keys.
fn normalize(chord: Chord) -> Chord {
    match chord.key.and_then(ModSet::of_key) {
        Some(mods) => Chord::mods(chord.mods.union(mods)),
        None => chord,
    }
}

#[cfg(test)]
mod tests;
