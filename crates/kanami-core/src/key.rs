/// Defines the `Key` enum and generates conversions in both directions between key names and enum values.
macro_rules! keys {
    ($($variant:ident => $name:literal),* $(,)?) => {
        /// A platform-independent physical key. Each platform backend maps its own key codes to this.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Key {
            $($variant),*
        }

        impl Key {
            pub const ALL: &'static [Key] = &[$(Key::$variant),*];
            pub const COUNT: usize = Self::ALL.len();

            /// The key name used in config files.
            pub fn name(self) -> &'static str {
                match self {
                    $(Key::$variant => $name),*
                }
            }

            pub fn from_name(name: &str) -> Option<Key> {
                match name {
                    $($name => Some(Key::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

keys! {
    A => "a", B => "b", C => "c", D => "d", E => "e", F => "f", G => "g",
    H => "h", I => "i", J => "j", K => "k", L => "l", M => "m", N => "n",
    O => "o", P => "p", Q => "q", R => "r", S => "s", T => "t", U => "u",
    V => "v", W => "w", X => "x", Y => "y", Z => "z",

    Num1 => "1", Num2 => "2", Num3 => "3", Num4 => "4", Num5 => "5",
    Num6 => "6", Num7 => "7", Num8 => "8", Num9 => "9", Num0 => "0",

    Escape => "escape",
    Enter => "enter",
    Tab => "tab",
    Space => "space",
    Backspace => "backspace",
    Delete => "delete",
    Grave => "grave",
    Minus => "minus",
    Equal => "equal",
    LeftBracket => "left_bracket",
    RightBracket => "right_bracket",
    Backslash => "backslash",
    Semicolon => "semicolon",
    Quote => "quote",
    Comma => "comma",
    Period => "period",
    Slash => "slash",

    Left => "left", Right => "right", Up => "up", Down => "down",
    Home => "home", End => "end", PageUp => "page_up", PageDown => "page_down",

    F1 => "f1", F2 => "f2", F3 => "f3", F4 => "f4", F5 => "f5",
    F6 => "f6", F7 => "f7", F8 => "f8", F9 => "f9", F10 => "f10",
    F11 => "f11", F12 => "f12", F13 => "f13", F14 => "f14", F15 => "f15",
    F16 => "f16", F17 => "f17", F18 => "f18", F19 => "f19", F20 => "f20",

    CapsLock => "caps_lock",
    LeftControl => "left_control",
    LeftShift => "left_shift",
    LeftOption => "left_option",
    LeftCommand => "left_command",
    RightControl => "right_control",
    RightShift => "right_shift",
    RightOption => "right_option",
    RightCommand => "right_command",
    Fn => "fn",

    Keypad0 => "keypad_0", Keypad1 => "keypad_1", Keypad2 => "keypad_2",
    Keypad3 => "keypad_3", Keypad4 => "keypad_4", Keypad5 => "keypad_5",
    Keypad6 => "keypad_6", Keypad7 => "keypad_7", Keypad8 => "keypad_8",
    Keypad9 => "keypad_9",
    KeypadPeriod => "keypad_period",
    KeypadPlus => "keypad_plus",
    KeypadMinus => "keypad_minus",
    KeypadMultiply => "keypad_multiply",
    KeypadDivide => "keypad_divide",
    KeypadEnter => "keypad_enter",
    KeypadEqual => "keypad_equal",
    NumLock => "num_lock",

    Insert => "insert",
    PrintScreen => "print_screen",
    ScrollLock => "scroll_lock",
    Pause => "pause",
    Menu => "menu",
}

impl Key {
    /// Keys on the numeric keypad (including Num Lock).
    pub fn is_keypad(self) -> bool {
        (Key::Keypad0 as u8..=Key::NumLock as u8).contains(&(self as u8))
    }

    /// Modifier keys (including Caps Lock and Fn). The system reports them as flags events rather than ordinary key events.
    pub fn is_modifier(self) -> bool {
        matches!(
            self,
            Key::CapsLock
                | Key::LeftControl
                | Key::LeftShift
                | Key::LeftOption
                | Key::LeftCommand
                | Key::RightControl
                | Key::RightShift
                | Key::RightOption
                | Key::RightCommand
                | Key::Fn
        )
    }
}

impl Key {
    /// The key code on the USB HID Keyboard/Keypad page (0x07); Fn is not on this page, so it returns `None`.
    pub const fn hid_usage(self) -> Option<u16> {
        Some(match self {
            Key::A => 0x04,
            Key::B => 0x05,
            Key::C => 0x06,
            Key::D => 0x07,
            Key::E => 0x08,
            Key::F => 0x09,
            Key::G => 0x0A,
            Key::H => 0x0B,
            Key::I => 0x0C,
            Key::J => 0x0D,
            Key::K => 0x0E,
            Key::L => 0x0F,
            Key::M => 0x10,
            Key::N => 0x11,
            Key::O => 0x12,
            Key::P => 0x13,
            Key::Q => 0x14,
            Key::R => 0x15,
            Key::S => 0x16,
            Key::T => 0x17,
            Key::U => 0x18,
            Key::V => 0x19,
            Key::W => 0x1A,
            Key::X => 0x1B,
            Key::Y => 0x1C,
            Key::Z => 0x1D,
            Key::Num1 => 0x1E,
            Key::Num2 => 0x1F,
            Key::Num3 => 0x20,
            Key::Num4 => 0x21,
            Key::Num5 => 0x22,
            Key::Num6 => 0x23,
            Key::Num7 => 0x24,
            Key::Num8 => 0x25,
            Key::Num9 => 0x26,
            Key::Num0 => 0x27,
            Key::Enter => 0x28,
            Key::Escape => 0x29,
            Key::Backspace => 0x2A,
            Key::Tab => 0x2B,
            Key::Space => 0x2C,
            Key::Minus => 0x2D,
            Key::Equal => 0x2E,
            Key::LeftBracket => 0x2F,
            Key::RightBracket => 0x30,
            Key::Backslash => 0x31,
            Key::Semicolon => 0x33,
            Key::Quote => 0x34,
            Key::Grave => 0x35,
            Key::Comma => 0x36,
            Key::Period => 0x37,
            Key::Slash => 0x38,
            Key::CapsLock => 0x39,
            Key::F1 => 0x3A,
            Key::F2 => 0x3B,
            Key::F3 => 0x3C,
            Key::F4 => 0x3D,
            Key::F5 => 0x3E,
            Key::F6 => 0x3F,
            Key::F7 => 0x40,
            Key::F8 => 0x41,
            Key::F9 => 0x42,
            Key::F10 => 0x43,
            Key::F11 => 0x44,
            Key::F12 => 0x45,
            Key::Home => 0x4A,
            Key::PageUp => 0x4B,
            Key::Delete => 0x4C,
            Key::End => 0x4D,
            Key::PageDown => 0x4E,
            Key::Right => 0x4F,
            Key::Left => 0x50,
            Key::Down => 0x51,
            Key::Up => 0x52,
            Key::F13 => 0x68,
            Key::F14 => 0x69,
            Key::F15 => 0x6A,
            Key::F16 => 0x6B,
            Key::F17 => 0x6C,
            Key::F18 => 0x6D,
            Key::F19 => 0x6E,
            Key::F20 => 0x6F,
            Key::LeftControl => 0xE0,
            Key::LeftShift => 0xE1,
            Key::LeftOption => 0xE2,
            Key::LeftCommand => 0xE3,
            Key::RightControl => 0xE4,
            Key::RightShift => 0xE5,
            Key::RightOption => 0xE6,
            Key::RightCommand => 0xE7,
            Key::PrintScreen => 0x46,
            Key::ScrollLock => 0x47,
            Key::Pause => 0x48,
            Key::Insert => 0x49,
            Key::NumLock => 0x53,
            Key::KeypadDivide => 0x54,
            Key::KeypadMultiply => 0x55,
            Key::KeypadMinus => 0x56,
            Key::KeypadPlus => 0x57,
            Key::KeypadEnter => 0x58,
            Key::Keypad1 => 0x59,
            Key::Keypad2 => 0x5A,
            Key::Keypad3 => 0x5B,
            Key::Keypad4 => 0x5C,
            Key::Keypad5 => 0x5D,
            Key::Keypad6 => 0x5E,
            Key::Keypad7 => 0x5F,
            Key::Keypad8 => 0x60,
            Key::Keypad9 => 0x61,
            Key::Keypad0 => 0x62,
            Key::KeypadPeriod => 0x63,
            Key::Menu => 0x65,
            Key::KeypadEqual => 0x67,
            Key::Fn => return None,
        })
    }
}

const _: () = assert!(Key::COUNT <= 128, "KeySet is a 128-bit bitmap");

/// A set of keys stored as a bitmap, with no allocation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct KeySet(u128);

impl KeySet {
    pub(crate) const fn empty() -> Self {
        KeySet(0)
    }

    pub(crate) fn insert(&mut self, key: Key) {
        self.0 |= 1 << key as u32;
    }

    pub(crate) fn remove(&mut self, key: Key) {
        self.0 &= !(1 << key as u32);
    }

    pub(crate) const fn contains(self, key: Key) -> bool {
        self.0 & (1 << key as u32) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for &key in Key::ALL {
            assert_eq!(Key::from_name(key.name()), Some(key));
        }
    }

    #[test]
    fn hid_usages_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for &key in Key::ALL {
            if let Some(usage) = key.hid_usage() {
                assert!(seen.insert(usage), "duplicate HID usage for {}", key.name());
            }
        }
    }

    #[test]
    fn discriminants_index_all() {
        for (i, &key) in Key::ALL.iter().enumerate() {
            assert_eq!(key as usize, i);
        }
    }
}
