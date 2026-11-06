use std::fmt;

use crate::Key;

/// A set of modifier keys, distinguishing left from right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ModSet(u16);

/// Modifier keys and their bits; the order decides which is pressed first when synthesizing events.
const MOD_KEYS: [(Key, ModSet); 9] = [
    (Key::LeftControl, ModSet::LEFT_CONTROL),
    (Key::LeftOption, ModSet::LEFT_OPTION),
    (Key::LeftShift, ModSet::LEFT_SHIFT),
    (Key::LeftCommand, ModSet::LEFT_COMMAND),
    (Key::RightControl, ModSet::RIGHT_CONTROL),
    (Key::RightOption, ModSet::RIGHT_OPTION),
    (Key::RightShift, ModSet::RIGHT_SHIFT),
    (Key::RightCommand, ModSet::RIGHT_COMMAND),
    (Key::Fn, ModSet::FN),
];

impl ModSet {
    pub const LEFT_CONTROL: ModSet = ModSet(1 << 0);
    pub const LEFT_OPTION: ModSet = ModSet(1 << 1);
    pub const LEFT_SHIFT: ModSet = ModSet(1 << 2);
    pub const LEFT_COMMAND: ModSet = ModSet(1 << 3);
    pub const RIGHT_CONTROL: ModSet = ModSet(1 << 4);
    pub const RIGHT_OPTION: ModSet = ModSet(1 << 5);
    pub const RIGHT_SHIFT: ModSet = ModSet(1 << 6);
    pub const RIGHT_COMMAND: ModSet = ModSet(1 << 7);
    pub const FN: ModSet = ModSet(1 << 8);
    /// ⌃⌥⇧⌘
    pub const HYPER: ModSet = ModSet(0b1111);
    /// ⌃⌥⇧
    pub const MEH: ModSet = ModSet(0b0111);

    pub const fn empty() -> Self {
        ModSet(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn contains(self, other: ModSet) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: ModSet) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn union(self, other: ModSet) -> ModSet {
        ModSet(self.0 | other.0)
    }

    pub const fn intersection(self, other: ModSet) -> ModSet {
        ModSet(self.0 & other.0)
    }

    pub const fn difference(self, other: ModSet) -> ModSet {
        ModSet(self.0 & !other.0)
    }

    /// The bit for a modifier key; Caps Lock is a lock key and is not counted.
    pub fn of_key(key: Key) -> Option<ModSet> {
        MOD_KEYS.iter().find(|(k, _)| *k == key).map(|&(_, m)| m)
    }

    /// The modifier keys in the set, in a fixed order.
    pub fn keys(self) -> impl Iterator<Item = Key> {
        MOD_KEYS
            .into_iter()
            .filter(move |&(_, m)| self.contains(m))
            .map(|(k, _)| k)
    }
}

impl fmt::Display for ModSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for kind in ModKind::ALL {
            let (left, right) = kind.bits();
            if self.intersects(left.union(right)) {
                f.write_str(kind.symbol())?;
            }
        }
        if self.contains(ModSet::FN) {
            f.write_str("fn")?;
        }
        Ok(())
    }
}

/// The four kinds of modifier key, ignoring side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModKind {
    Control,
    Option,
    Shift,
    Command,
}

impl ModKind {
    pub const ALL: [ModKind; 4] = [
        ModKind::Control,
        ModKind::Option,
        ModKind::Shift,
        ModKind::Command,
    ];

    /// (left bit, right bit)
    pub const fn bits(self) -> (ModSet, ModSet) {
        match self {
            ModKind::Control => (ModSet::LEFT_CONTROL, ModSet::RIGHT_CONTROL),
            ModKind::Option => (ModSet::LEFT_OPTION, ModSet::RIGHT_OPTION),
            ModKind::Shift => (ModSet::LEFT_SHIFT, ModSet::RIGHT_SHIFT),
            ModKind::Command => (ModSet::LEFT_COMMAND, ModSet::RIGHT_COMMAND),
        }
    }

    /// Which side to use when emitting this modifier: the left side if unspecified.
    pub const fn emit(self, side: Side) -> ModSet {
        let (left, right) = self.bits();
        match side {
            Side::Either | Side::Left => left,
            Side::Right => right,
            Side::Both => left.union(right),
        }
    }

    const fn symbol(self) -> &'static str {
        match self {
            ModKind::Control => "⌃",
            ModKind::Option => "⌥",
            ModKind::Shift => "⇧",
            ModKind::Command => "⌘",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Either,
    Left,
    Right,
    Both,
}

/// The modifier requirement of a combo, for exact matching.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModPattern {
    sides: [Option<Side>; 4],
    fn_key: bool,
}

impl ModPattern {
    pub fn require(&mut self, kind: ModKind, side: Side) {
        let slot = &mut self.sides[kind as usize];
        *slot = Some(match (*slot, side) {
            (None, s) | (Some(Side::Either), s) | (Some(s), Side::Either) => s,
            (Some(a), b) if a == b => a,
            _ => Side::Both,
        });
    }

    pub fn require_fn(&mut self) {
        self.fn_key = true;
    }

    /// Whether the pressed modifiers meet the requirement exactly: required ones must be down and unrequired ones must not be.
    pub fn matches(self, held: ModSet) -> bool {
        let sides_ok = ModKind::ALL.into_iter().all(|kind| {
            let (left, right) = kind.bits();
            let (l, r) = (held.contains(left), held.contains(right));
            match self.sides[kind as usize] {
                None => !l && !r,
                Some(Side::Either) => l || r,
                Some(Side::Left) => l && !r,
                Some(Side::Right) => r && !l,
                Some(Side::Both) => l && r,
            }
        });
        sides_ok && self.fn_key == held.contains(ModSet::FN)
    }

    /// When it matches, the part of the pressed modifiers that belongs to this requirement.
    pub fn matched(self, held: ModSet) -> ModSet {
        let mut bits = ModSet::empty();
        for kind in ModKind::ALL {
            if self.sides[kind as usize].is_some() {
                let (left, right) = kind.bits();
                bits = bits.union(left.union(right));
            }
        }
        if self.fn_key {
            bits = bits.union(ModSet::FN);
        }
        held.intersection(bits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pattern(reqs: &[(ModKind, Side)]) -> ModPattern {
        let mut p = ModPattern::default();
        for &(kind, side) in reqs {
            p.require(kind, side);
        }
        p
    }

    #[test]
    fn exact_match_rejects_extra_modifiers() {
        let cmd = pattern(&[(ModKind::Command, Side::Either)]);
        assert!(cmd.matches(ModSet::LEFT_COMMAND));
        assert!(cmd.matches(ModSet::RIGHT_COMMAND));
        assert!(!cmd.matches(ModSet::LEFT_COMMAND.union(ModSet::LEFT_SHIFT)));
        assert!(!cmd.matches(ModSet::empty()));
    }

    #[test]
    fn sided_match() {
        let right = pattern(&[(ModKind::Option, Side::Right)]);
        assert!(right.matches(ModSet::RIGHT_OPTION));
        assert!(!right.matches(ModSet::LEFT_OPTION));
        assert!(!right.matches(ModSet::LEFT_OPTION.union(ModSet::RIGHT_OPTION)));
    }

    #[test]
    fn hyper_matches_any_sides() {
        let hyper = pattern(&ModKind::ALL.map(|k| (k, Side::Either)));
        assert!(hyper.matches(ModSet::HYPER));
        assert_eq!(hyper.matched(ModSet::HYPER), ModSet::HYPER);
    }

    #[test]
    fn keys_in_order() {
        let keys: Vec<Key> = ModSet::HYPER.keys().collect();
        assert_eq!(
            keys,
            [
                Key::LeftControl,
                Key::LeftOption,
                Key::LeftShift,
                Key::LeftCommand
            ]
        );
    }
}
