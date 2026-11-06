//! The layer overlay logic: when to show it and what it shows. It appears only after the layer key
//! has been held for about 0.4 seconds (a quick tap does not flash it) and disappears when the
//! layer ends. Drawing is up to the platform ([`HudView`]).

use kanami_core::backend::{HudKey, HudStyle};
use kanami_core::{Action, Activation, Chord, Key, ModSet, Rules};
use kanami_i18n::tr;

/// How long after entering a layer the overlay shows (milliseconds).
const DELAY: u64 = 400;

pub trait HudView {
    fn show(&mut self, title: &str, keys: &[HudKey]);
    fn press(&mut self, key: Key, down: bool);
    fn hide(&mut self);
}

#[cfg(target_os = "macos")]
impl HudView for kanami_macos::Hud {
    fn show(&mut self, title: &str, keys: &[HudKey]) {
        kanami_macos::Hud::show(self, title, keys);
    }
    fn press(&mut self, key: Key, down: bool) {
        kanami_macos::Hud::press(self, key, down);
    }
    fn hide(&mut self) {
        kanami_macos::Hud::hide(self);
    }
}

struct LayerKeys {
    name: String,
    keys: Vec<HudKey>,
}

pub struct Hud {
    enabled: bool,
    layers: Vec<LayerKeys>,
    view: Option<Box<dyn HudView>>,
    /// The currently active layer and the time it should be shown.
    current: Option<(String, u64)>,
    shown: bool,
}

impl Hud {
    pub fn new(rules: &Rules) -> Self {
        let mut hud = Hud {
            enabled: false,
            layers: Vec::new(),
            view: None,
            current: None,
            shown: false,
        };
        hud.set_rules(rules);
        hud
    }

    /// Where nothing is drawn (command-line mode, Windows) only the logic runs.
    #[cfg_attr(windows, allow(dead_code))]
    pub fn set_view(&mut self, view: Box<dyn HudView>) {
        self.view = Some(view);
    }

    /// The config changed.
    pub fn set_rules(&mut self, rules: &Rules) {
        self.enabled = rules.settings.layer_hud;
        self.layers = rules
            .layers
            .iter()
            .map(|layer| LayerKeys {
                name: layer.name.clone(),
                keys: board(layer),
            })
            .collect();
        self.hide();
        self.current = None;
    }

    /// Called whenever the engine state may have changed. `active` is the topmost active layer.
    pub fn update(&mut self, active: Option<&str>, now: u64) {
        let active = active.filter(|_| self.enabled);
        if active != self.current.as_ref().map(|(name, _)| name.as_str()) {
            let switching = self.shown && active.is_some();
            self.hide();
            // Switching layers while already showing needs no further wait
            let at = if switching { now } else { now + DELAY };
            self.current = active.map(|name| (name.to_owned(), at));
        }
        if let Some((name, at)) = &self.current
            && !self.shown
            && now >= *at
        {
            if let (Some(view), Some(layer)) = (
                self.view.as_mut(),
                self.layers.iter().find(|l| l.name == *name),
            ) {
                view.show(name, &layer.keys);
            }
            self.shown = true;
        }
    }

    /// While not shown yet, the next time to check.
    pub fn deadline(&self) -> Option<u64> {
        match &self.current {
            Some((_, at)) if !self.shown => Some(*at),
            _ => None,
        }
    }

    pub fn press(&mut self, key: Key, down: bool) {
        if self.shown
            && let Some(view) = self.view.as_mut()
        {
            view.press(key, down);
        }
    }

    pub fn hide(&mut self) {
        if self.shown
            && let Some(view) = self.view.as_mut()
        {
            view.hide();
        }
        self.shown = false;
    }
}

/// The overlay draws the main key area; the F-key row is added when the layer changes F keys.
const F_ROW: &[(&str, f64)] = &[
    ("escape", 1.0),
    ("", 1.0),
    ("f1", 1.0),
    ("f2", 1.0),
    ("f3", 1.0),
    ("f4", 1.0),
    ("", 0.5),
    ("f5", 1.0),
    ("f6", 1.0),
    ("f7", 1.0),
    ("f8", 1.0),
    ("", 0.5),
    ("f9", 1.0),
    ("f10", 1.0),
    ("f11", 1.0),
    ("f12", 1.0),
];
const ROWS: &[&[(&str, f64)]] = &[
    &[
        ("grave", 1.0),
        ("1", 1.0),
        ("2", 1.0),
        ("3", 1.0),
        ("4", 1.0),
        ("5", 1.0),
        ("6", 1.0),
        ("7", 1.0),
        ("8", 1.0),
        ("9", 1.0),
        ("0", 1.0),
        ("minus", 1.0),
        ("equal", 1.0),
        ("backspace", 2.0),
    ],
    &[
        ("tab", 1.5),
        ("q", 1.0),
        ("w", 1.0),
        ("e", 1.0),
        ("r", 1.0),
        ("t", 1.0),
        ("y", 1.0),
        ("u", 1.0),
        ("i", 1.0),
        ("o", 1.0),
        ("p", 1.0),
        ("left_bracket", 1.0),
        ("right_bracket", 1.0),
        ("backslash", 1.5),
    ],
    &[
        ("caps_lock", 1.75),
        ("a", 1.0),
        ("s", 1.0),
        ("d", 1.0),
        ("f", 1.0),
        ("g", 1.0),
        ("h", 1.0),
        ("j", 1.0),
        ("k", 1.0),
        ("l", 1.0),
        ("semicolon", 1.0),
        ("quote", 1.0),
        ("enter", 2.25),
    ],
    &[
        ("left_shift", 2.25),
        ("z", 1.0),
        ("x", 1.0),
        ("c", 1.0),
        ("v", 1.0),
        ("b", 1.0),
        ("n", 1.0),
        ("m", 1.0),
        ("comma", 1.0),
        ("period", 1.0),
        ("slash", 1.0),
        ("right_shift", 2.75),
    ],
    &[
        ("left_control", 1.25),
        ("left_option", 1.25),
        ("left_command", 1.5),
        ("space", 6.0),
        ("right_command", 1.5),
        ("right_option", 1.25),
        ("fn", 1.0),
        ("right_control", 1.25),
    ],
];

fn board(layer: &kanami_core::Layer) -> Vec<HudKey> {
    let trigger = match &layer.activation {
        Activation::Hold { key, .. } => Some(*key),
        Activation::Toggle(_) => None,
    };
    let uses_f_row = layer.map.iter().any(|(key, _)| {
        F_ROW
            .iter()
            .any(|(name, _)| !name.is_empty() && *name == key.name())
    }) || trigger
        .is_some_and(|key| F_ROW.iter().any(|(name, _)| *name == key.name()));
    let mut keys = Vec::new();
    let mut y = 0.0;
    let mut row_keys = |row: &[(&str, f64)], y: f64| {
        let mut x = 0.0;
        for &(name, w) in row {
            if let Some(key) = Key::from_name(name) {
                let action = layer.map.iter().find(|(k, _)| *k == key).map(|(_, a)| a);
                let (label, style) = match action {
                    Some(action) => (action_label(action), HudStyle::Mapped),
                    None if trigger == Some(key) => (key_label(key), HudStyle::Trigger),
                    None => (key_label(key), HudStyle::Plain),
                };
                keys.push(HudKey {
                    key,
                    x,
                    y,
                    w,
                    label,
                    style,
                });
            }
            x += w;
        }
    };
    if uses_f_row {
        row_keys(F_ROW, y);
        y += 1.0;
    }
    for row in ROWS {
        row_keys(row, y);
        y += 1.0;
    }
    keys
}

fn action_label(action: &Action) -> String {
    match action {
        Action::Keys(chords) => match chords.as_slice() {
            [] => String::new(),
            [only] => chord_label(*only),
            [first, ..] => format!("{}…", chord_label(*first)),
        },
        Action::Disable => tr!("Disable"),
        Action::Run(_) => tr!("Run"),
        Action::Call(_) => tr!("Script"),
    }
}

fn chord_label(chord: Chord) -> String {
    let mut label = String::new();
    if chord.mods.contains(ModSet::HYPER) {
        label.push_str("hyper ");
    } else if chord.mods.contains(ModSet::MEH) {
        label.push_str("meh ");
    } else {
        for (mods, glyph) in [
            (ModSet::LEFT_CONTROL.union(ModSet::RIGHT_CONTROL), "⌃"),
            (ModSet::LEFT_OPTION.union(ModSet::RIGHT_OPTION), "⌥"),
            (ModSet::LEFT_SHIFT.union(ModSet::RIGHT_SHIFT), "⇧"),
            (ModSet::LEFT_COMMAND.union(ModSet::RIGHT_COMMAND), "⌘"),
            (ModSet::FN, "fn "),
        ] {
            if chord.mods.intersects(mods) {
                label.push_str(glyph);
            }
        }
    }
    match chord.key {
        Some(key) => label.push_str(&key_label(key)),
        None => label = label.trim_end().to_owned(),
    }
    label
}

fn key_label(key: Key) -> String {
    let name = key.name();
    let short = match name {
        "left" => "←",
        "right" => "→",
        "up" => "↑",
        "down" => "↓",
        "escape" => "esc",
        "backspace" => "⌫",
        "delete" => "⌦",
        "caps_lock" => "caps",
        "left_shift" => "shift",
        "right_shift" => "r shift",
        "left_control" => "ctrl",
        "right_control" => "r ctrl",
        "left_option" => "opt",
        "right_option" => "r opt",
        "left_command" => "cmd",
        "right_command" => "r cmd",
        "page_up" => "pgup",
        "page_down" => "pgdn",
        "grave" => "`",
        "minus" => "-",
        "equal" => "=",
        "left_bracket" => "[",
        "right_bracket" => "]",
        "backslash" => "\\",
        "semicolon" => ";",
        "quote" => "'",
        "comma" => ",",
        "period" => ".",
        "slash" => "/",
        other => other,
    };
    short.to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Log(Rc<RefCell<Vec<String>>>);

    impl HudView for Log {
        fn show(&mut self, title: &str, keys: &[HudKey]) {
            let mapped = keys
                .iter()
                .filter(|k| matches!(k.style, HudStyle::Mapped))
                .count();
            self.0.borrow_mut().push(format!("show {title} {mapped}"));
        }
        fn press(&mut self, key: Key, down: bool) {
            self.0
                .borrow_mut()
                .push(format!("press {} {down}", key.name()));
        }
        fn hide(&mut self) {
            self.0.borrow_mut().push("hide".to_owned());
        }
    }

    fn hud(enabled: bool) -> (Hud, Rc<RefCell<Vec<String>>>) {
        let config = kanami_config::parse(
            "[layer.nav]\nhold = \"space\"\nmap = { h = \"left\", j = \"cmd+down\" }\n\
             [layer.num]\ntoggle = \"hyper+n\"\nmap = { f1 = \"1\" }\n",
        )
        .unwrap();
        let mut rules = config.rules;
        rules.settings.layer_hud = enabled;
        let mut hud = Hud::new(&rules);
        let log = Log::default();
        let seen = log.0.clone();
        hud.set_view(Box::new(log));
        (hud, seen)
    }

    #[test]
    fn appears_after_delay_and_hides() {
        let (mut h, seen) = hud(true);
        h.update(Some("nav"), 1000);
        assert_eq!(h.deadline(), Some(1400));
        h.update(Some("nav"), 1399);
        assert!(seen.borrow().is_empty());
        h.update(Some("nav"), 1400);
        assert_eq!(h.deadline(), None);
        h.press(Key::H, true);
        h.update(None, 1500);
        assert_eq!(*seen.borrow(), ["show nav 2", "press h true", "hide"]);
    }

    #[test]
    fn quick_tap_never_shows() {
        let (mut h, seen) = hud(true);
        h.update(Some("nav"), 1000);
        h.update(None, 1100);
        assert_eq!(h.deadline(), None);
        h.update(None, 2000);
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn switching_layers_is_immediate() {
        let (mut h, seen) = hud(true);
        h.update(Some("nav"), 0);
        h.update(Some("nav"), 400);
        h.update(Some("num"), 500);
        assert_eq!(*seen.borrow(), ["show nav 2", "hide", "show num 1"]);
    }

    #[test]
    fn can_be_turned_off() {
        let (mut h, seen) = hud(false);
        h.update(Some("nav"), 0);
        h.update(Some("nav"), 1000);
        assert_eq!(h.deadline(), None);
        assert!(seen.borrow().is_empty());
    }

    #[test]
    fn labels() {
        assert_eq!(chord_label(Chord::key(Key::Left)), "←");
        let chord = Chord {
            mods: ModSet::LEFT_COMMAND.union(ModSet::LEFT_SHIFT),
            key: Some(Key::K),
        };
        assert_eq!(chord_label(chord), "⇧⌘k");
        assert_eq!(
            chord_label(Chord {
                mods: ModSet::HYPER,
                key: Some(Key::T)
            }),
            "hyper t"
        );
        assert_eq!(chord_label(Chord::mods(ModSet::HYPER)), "hyper");
        assert_eq!(action_label(&Action::Run("x".into())), "Run");
    }
}
