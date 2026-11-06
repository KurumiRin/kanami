//! Key expressions: `"a"`, `"cmd+k"`, `"hyper+t"`, `"none"`.

use kanami_core::{Action, Chord, Key, ModKind, ModPattern, ModSet, RemapTarget, Side, Trigger};

use kanami_i18n::tr;

use crate::raw::OneOrMany;

/// Writing this means disabled.
pub const NONE: &str = "none";

const MODIFIERS: &[(&str, ModKind, Side)] = &[
    ("ctrl", ModKind::Control, Side::Either),
    ("control", ModKind::Control, Side::Either),
    ("left_ctrl", ModKind::Control, Side::Left),
    ("left_control", ModKind::Control, Side::Left),
    ("right_ctrl", ModKind::Control, Side::Right),
    ("right_control", ModKind::Control, Side::Right),
    ("opt", ModKind::Option, Side::Either),
    ("option", ModKind::Option, Side::Either),
    ("alt", ModKind::Option, Side::Either),
    ("left_opt", ModKind::Option, Side::Left),
    ("left_option", ModKind::Option, Side::Left),
    ("left_alt", ModKind::Option, Side::Left),
    ("right_opt", ModKind::Option, Side::Right),
    ("right_option", ModKind::Option, Side::Right),
    ("right_alt", ModKind::Option, Side::Right),
    ("shift", ModKind::Shift, Side::Either),
    ("left_shift", ModKind::Shift, Side::Left),
    ("right_shift", ModKind::Shift, Side::Right),
    ("cmd", ModKind::Command, Side::Either),
    ("command", ModKind::Command, Side::Either),
    ("left_cmd", ModKind::Command, Side::Left),
    ("left_command", ModKind::Command, Side::Left),
    ("right_cmd", ModKind::Command, Side::Right),
    ("right_command", ModKind::Command, Side::Right),
];

/// Modifier spellings usable in a combo besides the left/right modifiers.
const SPECIAL_MODIFIERS: [&str; 3] = ["hyper", "meh", "fn"];

/// All modifier spellings, used by `kanami keys` and the JSON Schema.
pub fn modifier_names() -> impl Iterator<Item = &'static str> {
    MODIFIERS
        .iter()
        .map(|&(name, _, _)| name)
        .chain(SPECIAL_MODIFIERS)
}

/// Two readings of the same set of modifiers: the requirement when matching, and which side to use when emitting.
#[derive(Default)]
struct Mods {
    pattern: ModPattern,
    emit: ModSet,
}

impl Mods {
    fn add(&mut self, token: &str) -> bool {
        let kinds: &[ModKind] = match token {
            "hyper" => &ModKind::ALL,
            "meh" => &ModKind::ALL[..3],
            "fn" => {
                self.pattern.require_fn();
                self.emit = self.emit.union(ModSet::FN);
                return true;
            }
            _ => {
                let Some(&(_, kind, side)) = MODIFIERS.iter().find(|(name, _, _)| *name == token)
                else {
                    return false;
                };
                self.pattern.require(kind, side);
                self.emit = self.emit.union(kind.emit(side));
                return true;
            }
        };
        for &kind in kinds {
            self.pattern.require(kind, Side::Either);
            self.emit = self.emit.union(kind.emit(Side::Either));
        }
        true
    }
}

struct Parsed {
    mods: Mods,
    has_mods: bool,
    key: Option<Key>,
}

fn is_modifier(token: &str) -> bool {
    modifier_names().any(|name| name == token)
}

fn parse(expr: &str) -> Result<Parsed, String> {
    let tokens: Vec<&str> = expr.split('+').map(str::trim).collect();
    if tokens.iter().any(|t| t.is_empty()) {
        return Err(tr!(
            "Malformed key expression \"{expr}\"; write it like \"cmd+shift+k\"",
            expr = expr
        ));
    }
    let (&last, init) = tokens
        .split_last()
        .expect("split always yields at least one part");
    let mut mods = Mods::default();
    for &token in init {
        if !mods.add(token) {
            return Err(unknown_modifier(token));
        }
    }
    // A lone word is read as a key name first (such as left_command or fn); the last word of a combination is read as a modifier if it is one
    if (init.is_empty() || !is_modifier(last))
        && let Some(key) = Key::from_name(last)
    {
        return Ok(Parsed {
            mods,
            has_mods: !init.is_empty(),
            key: Some(key),
        });
    }
    if mods.add(last) {
        return Ok(Parsed {
            mods,
            has_mods: true,
            key: None,
        });
    }
    Err(unknown_key(last))
}

pub fn key(name: &str) -> Result<Key, String> {
    Key::from_name(name).ok_or_else(|| unknown_key(name))
}

/// Trigger condition: a bare key matches any modifiers, and a key with modifiers matches exactly.
pub fn trigger(expr: &str) -> Result<Trigger, String> {
    let parsed = parse(expr)?;
    let Some(key) = parsed.key else {
        return Err(tr!(
            "\"{expr}\" has only modifiers; a key is needed here",
            expr = expr
        ));
    };
    Ok(Trigger {
        key,
        mods: parsed.has_mods.then_some(parsed.mods.pattern),
    })
}

/// Outputs a key expression. `none` returns `None`.
pub fn output(expr: &str) -> Result<Option<Chord>, String> {
    if expr == NONE {
        return Ok(None);
    }
    let parsed = parse(expr)?;
    if parsed.key == Some(Key::CapsLock) {
        return Err(tr!("caps_lock cannot be used as an output"));
    }
    Ok(Some(Chord {
        mods: parsed.mods.emit,
        key: parsed.key,
    }))
}

pub fn chord(expr: &str) -> Result<Chord, String> {
    output(expr)?.ok_or_else(|| tr!("none cannot be used here"))
}

pub fn action(value: &OneOrMany) -> Result<Action, String> {
    match value {
        OneOrMany::One(expr) => Ok(match output(expr)? {
            Some(chord) => Action::Keys(vec![chord]),
            None => Action::Disable,
        }),
        OneOrMany::Many(exprs) => exprs
            .iter()
            .map(|e| chord(e))
            .collect::<Result<_, _>>()
            .map(Action::Keys),
    }
}

pub fn remap_target(expr: &str) -> Result<RemapTarget, String> {
    if expr == NONE {
        return Ok(RemapTarget::Disable);
    }
    let parsed = parse(expr)?;
    match (parsed.has_mods, parsed.key) {
        (_, Some(Key::CapsLock)) => Err(tr!("caps_lock cannot be used as an output")),
        (false, Some(key)) => Ok(RemapTarget::Key(key)),
        (_, None) => Ok(RemapTarget::Mods(parsed.mods.emit)),
        (true, Some(_)) => Err(tr!(
            "[remap] can only change to a single key or a set of modifiers; use [[rule]] for \"{expr}\"",
            expr = expr
        )),
    }
}

/// A tap in a test sequence: press the modifiers, tap the key, then release the modifiers.
pub fn tap_keys(expr: &str) -> Result<Vec<(Key, bool)>, String> {
    let parsed = parse(expr)?;
    let mods: Vec<Key> = parsed.mods.emit.keys().collect();
    let mut steps: Vec<(Key, bool)> = mods.iter().map(|&k| (k, true)).collect();
    if let Some(key) = parsed.key {
        steps.extend([(key, true), (key, false)]);
    }
    steps.extend(mods.iter().rev().map(|&k| (k, false)));
    Ok(steps)
}

/// The x in `+x` / `-x` in a test sequence: a key name, or a modifier spelling that stands for a single key (such as `ctrl`).
pub fn single_key(name: &str) -> Result<Key, String> {
    let parsed = parse(name)?;
    let mut mods = parsed.mods.emit.keys();
    match (parsed.key, mods.next(), mods.next()) {
        (Some(key), None, _) => Ok(key),
        (None, Some(key), None) => Ok(key),
        _ => Err(tr!("\"{name}\" is not a single key", name = name)),
    }
}

fn unknown_key(name: &str) -> String {
    match closest_name(name) {
        Some(guess) => tr!(
            "Unknown key \"{name}\"; the closest name is \"{guess}\"",
            name = name,
            guess = guess
        ),
        None => tr!(
            "Unknown key \"{name}\"; run `kanami keys` to see all key names",
            name = name
        ),
    }
}

fn unknown_modifier(name: &str) -> String {
    match closest_name(name) {
        Some(guess) => tr!(
            "Unknown modifier \"{name}\"; the closest name is \"{guess}\"",
            name = name,
            guess = guess
        ),
        None => tr!(
            "Unknown modifier \"{name}\"; run `kanami keys` to see all key names",
            name = name
        ),
    }
}

fn closest_name(name: &str) -> Option<&'static str> {
    let candidates = Key::ALL.iter().map(|k| k.name()).chain(modifier_names());
    closest(name, candidates)
}

fn closest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    candidates
        .map(|c| (edit_distance(name, c), c))
        .filter(|&(d, _)| d <= 2)
        .min_by_key(|&(d, _)| d)
        .map(|(_, c)| c)
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cur = row[j + 1];
            row[j + 1] = (prev + usize::from(ca != cb)).min(row[j] + 1).min(cur + 1);
            prev = cur;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_triggers() {
        assert_eq!(
            trigger("a").unwrap(),
            Trigger {
                key: Key::A,
                mods: None
            }
        );
        let t = trigger("cmd+shift+k").unwrap();
        assert_eq!(t.key, Key::K);
        let pattern = t.mods.unwrap();
        assert!(pattern.matches(ModSet::RIGHT_COMMAND.union(ModSet::LEFT_SHIFT)));
        assert!(!pattern.matches(ModSet::RIGHT_COMMAND));
        // A lone modifier name is the trigger key itself
        assert_eq!(trigger("left_command").unwrap().key, Key::LeftCommand);
        assert!(trigger("cmd").is_err());
    }

    #[test]
    fn parses_outputs() {
        assert_eq!(
            output("ctrl+l").unwrap(),
            Some(Chord {
                mods: ModSet::LEFT_CONTROL,
                key: Some(Key::L)
            })
        );
        assert_eq!(output("hyper").unwrap(), Some(Chord::mods(ModSet::HYPER)));
        assert_eq!(
            output("right_opt+meh").unwrap(),
            Some(Chord::mods(ModSet::MEH.union(ModSet::RIGHT_OPTION)))
        );
        assert_eq!(output("none").unwrap(), None);
        assert!(output("caps_lock").is_err());
    }

    #[test]
    fn remap_targets() {
        assert_eq!(
            remap_target("escape").unwrap(),
            RemapTarget::Key(Key::Escape)
        );
        assert_eq!(
            remap_target("hyper").unwrap(),
            RemapTarget::Mods(ModSet::HYPER)
        );
        assert!(remap_target("cmd+k").is_err());
    }

    #[test]
    fn suggests_close_names() {
        assert!(trigger("esacpe").unwrap_err().contains("\"escape\""));
        assert!(trigger("cmdd+k").unwrap_err().contains("\"cmd\""));
        assert!(!trigger("xyzzy").unwrap_err().contains("closest"));
    }

    #[test]
    fn rejects_malformed() {
        assert!(trigger("cmd++k").is_err());
        assert!(trigger("").is_err());
    }

    #[test]
    fn tap_keys_wraps_modifiers() {
        assert_eq!(
            tap_keys("ctrl+c").unwrap(),
            [
                (Key::LeftControl, true),
                (Key::C, true),
                (Key::C, false),
                (Key::LeftControl, false)
            ]
        );
        assert_eq!(single_key("ctrl").unwrap(), Key::LeftControl);
        assert!(single_key("hyper").is_err());
    }
}
