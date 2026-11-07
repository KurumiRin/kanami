//! Rule data structures: produced by the config layer and compiled by [`Engine::new`](crate::Engine::new).

use std::fmt;

use kanami_i18n::tr;

use crate::{Key, ModPattern, ModSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// Time limit for deciding tap vs hold (milliseconds).
    pub tap_timeout: u32,
    /// Window for deciding simultaneous presses (milliseconds).
    pub combo_timeout: u32,
    /// Typing streak window (milliseconds): when less than this has passed since the previous key press, tap/hold keys act as taps immediately
    /// and combos do not wait, so typing is neither delayed nor misfired. 0 turns it off.
    pub typing_streak: u32,
    /// Shows this layer's key layout at the bottom of the screen while a layer key is held. Does not affect keys.
    pub layer_hud: bool,
    /// The UI language; `None` follows the system language.
    pub language: Option<kanami_i18n::Lang>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            tap_timeout: 200,
            combo_timeout: 50,
            typing_streak: 150,
            layer_hud: true,
            language: None,
        }
    }
}

/// One output: a set of modifiers plus an optional key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    pub mods: ModSet,
    pub key: Option<Key>,
}

impl Chord {
    pub const fn key(key: Key) -> Self {
        Chord {
            mods: ModSet::empty(),
            key: Some(key),
        }
    }

    pub const fn mods(mods: ModSet) -> Self {
        Chord { mods, key: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// With one key expression, press/release follow the trigger key; with several, they are tapped in turn.
    Keys(Vec<Chord>),
    Disable,
    /// A shell command run in the background.
    Run(String),
    /// Calls a function in the script.
    Call(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemapTarget {
    Key(Key),
    Mods(ModSet),
    Disable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Remap {
    pub from: Key,
    pub to: RemapTarget,
    /// Applies only to this keyboard (an index into [`Rules::devices`]); `None` means all keyboards.
    pub device: Option<usize>,
}

/// The key in a trigger. `mods` being `None` means a bare key, which matches any modifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trigger {
    pub key: Key,
    pub mods: Option<ModPattern>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Condition {
    /// The frontmost app's Bundle ID is in the list (empty means no restriction).
    pub app: Vec<String>,
    pub app_not: Vec<String>,
    /// The current input method ID is in the list (empty means no restriction).
    pub input_source: Vec<String>,
    pub input_source_not: Vec<String>,
    /// Script variables equal the given values; an unset variable counts as the empty string.
    pub vars: Vec<(String, String)>,
    /// The key comes from one of the keyboards with these names in `[device.*]`.
    pub device: Vec<String>,
    pub device_not: Vec<String>,
}

/// `[device.<name>]`: describes a keyboard; every field that is written must match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DeviceSpec {
    pub name: String,
    pub vendor_id: Option<u32>,
    pub product_id: Option<u32>,
    /// The product name contains this text (case-insensitive).
    pub product: Option<String>,
    /// Use only this keyboard's own rules and remaps, not those that apply to all keyboards.
    pub exclusive: bool,
}

impl DeviceSpec {
    pub fn matches(&self, device: &crate::backend::DeviceInfo) -> bool {
        self.vendor_id.is_none_or(|v| v == device.vendor_id)
            && self.product_id.is_none_or(|p| p == device.product_id)
            && self
                .product
                .as_ref()
                .is_none_or(|p| device.product.to_lowercase().contains(&p.to_lowercase()))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Strategy {
    Timeout,
    #[default]
    Permissive,
    Eager,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapHold {
    pub tap: Action,
    pub hold: Chord,
    pub strategy: Strategy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleKind {
    Action(Action),
    TapHold(TapHold),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub from: Trigger,
    pub kind: RuleKind,
    pub when: Condition,
    /// The condition of the scope the rule sits in (a rule group, a keyboard); it takes effect only when this and `when` are both met.
    pub scope: Condition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Activation {
    /// Active while held; when `tap` is `None`, a tap posts the trigger key itself.
    Hold {
        key: Key,
        tap: Option<Action>,
        strategy: Strategy,
    },
    Toggle(Trigger),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub name: String,
    pub activation: Activation,
    pub map: Vec<(Key, Action)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Combo {
    pub keys: Vec<Key>,
    pub action: Action,
    /// Overrides [`Settings::combo_timeout`].
    pub timeout: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rules {
    pub settings: Settings,
    pub remaps: Vec<Remap>,
    pub rules: Vec<Rule>,
    pub layers: Vec<Layer>,
    pub combos: Vec<Combo>,
    pub devices: Vec<DeviceSpec>,
}

/// Where the faulty rule is in [`Rules`], which the config layer uses to locate it in the source file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Rule(usize),
    Layer(usize),
    Combo(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleErrorKind {
    TapHoldNeedsBareKey,
    DuplicateLayerTrigger(Key),
    EmptyKeys,
    ComboTooFewKeys,
    ComboModifier(Key),
    ComboDuplicateKey(Key),
    TooMany { what: &'static str, max: usize },
    UnknownDevice(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleError {
    pub at: Location,
    pub kind: RuleErrorKind,
}

impl fmt::Display for RuleErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuleErrorKind::TapHoldNeedsBareKey => f.write_str(&tr!(
                "The from of a tap/hold rule must be a single key without modifiers"
            )),
            RuleErrorKind::DuplicateLayerTrigger(key) => f.write_str(&tr!(
                "{key} is already the trigger key of another layer",
                key = key.name()
            )),
            RuleErrorKind::EmptyKeys => f.write_str(&tr!("The key list cannot be empty")),
            RuleErrorKind::ComboTooFewKeys => f.write_str(&tr!("A combo needs at least two keys")),
            RuleErrorKind::ComboModifier(key) => f.write_str(&tr!(
                "The modifier {key} cannot be used in a combo",
                key = key.name()
            )),
            RuleErrorKind::ComboDuplicateKey(key) => f.write_str(&tr!(
                "{key} appears more than once in the combo",
                key = key.name()
            )),
            RuleErrorKind::TooMany { what, max } => f.write_str(&tr!(
                "The maximum number of {what} is {max}",
                what = kanami_i18n::text(what),
                max = max
            )),
            RuleErrorKind::UnknownDevice(name) => f.write_str(&tr!(
                "There is no device named {name}; define it in [device.{name}] first",
                name = name
            )),
        }
    }
}

impl fmt::Display for RuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl std::error::Error for RuleError {}
