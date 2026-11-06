//! Two-way mapping between macOS virtual key codes (`kVK_*`, see HIToolbox/Events.h) and [`Key`].

use kanami_core::Key;

const PAIRS: &[(u16, Key)] = &[
    (0x00, Key::A),
    (0x01, Key::S),
    (0x02, Key::D),
    (0x03, Key::F),
    (0x04, Key::H),
    (0x05, Key::G),
    (0x06, Key::Z),
    (0x07, Key::X),
    (0x08, Key::C),
    (0x09, Key::V),
    (0x0B, Key::B),
    (0x0C, Key::Q),
    (0x0D, Key::W),
    (0x0E, Key::E),
    (0x0F, Key::R),
    (0x10, Key::Y),
    (0x11, Key::T),
    (0x12, Key::Num1),
    (0x13, Key::Num2),
    (0x14, Key::Num3),
    (0x15, Key::Num4),
    (0x16, Key::Num6),
    (0x17, Key::Num5),
    (0x18, Key::Equal),
    (0x19, Key::Num9),
    (0x1A, Key::Num7),
    (0x1B, Key::Minus),
    (0x1C, Key::Num8),
    (0x1D, Key::Num0),
    (0x1E, Key::RightBracket),
    (0x1F, Key::O),
    (0x20, Key::U),
    (0x21, Key::LeftBracket),
    (0x22, Key::I),
    (0x23, Key::P),
    (0x24, Key::Enter),
    (0x25, Key::L),
    (0x26, Key::J),
    (0x27, Key::Quote),
    (0x28, Key::K),
    (0x29, Key::Semicolon),
    (0x2A, Key::Backslash),
    (0x2B, Key::Comma),
    (0x2C, Key::Slash),
    (0x2D, Key::N),
    (0x2E, Key::M),
    (0x2F, Key::Period),
    (0x30, Key::Tab),
    (0x31, Key::Space),
    (0x32, Key::Grave),
    (0x33, Key::Backspace),
    (0x35, Key::Escape),
    (0x36, Key::RightCommand),
    (0x37, Key::LeftCommand),
    (0x38, Key::LeftShift),
    (0x39, Key::CapsLock),
    (0x3A, Key::LeftOption),
    (0x3B, Key::LeftControl),
    (0x3C, Key::RightShift),
    (0x3D, Key::RightOption),
    (0x3E, Key::RightControl),
    (0x3F, Key::Fn),
    (0x40, Key::F17),
    (0x4F, Key::F18),
    (0x50, Key::F19),
    (0x5A, Key::F20),
    (0x60, Key::F5),
    (0x61, Key::F6),
    (0x62, Key::F7),
    (0x63, Key::F3),
    (0x64, Key::F8),
    (0x65, Key::F9),
    (0x67, Key::F11),
    (0x69, Key::F13),
    (0x6A, Key::F16),
    (0x6B, Key::F14),
    (0x6D, Key::F10),
    (0x6F, Key::F12),
    (0x71, Key::F15),
    (0x73, Key::Home),
    (0x74, Key::PageUp),
    (0x75, Key::Delete),
    (0x76, Key::F4),
    (0x77, Key::End),
    (0x78, Key::F2),
    (0x79, Key::PageDown),
    (0x7A, Key::F1),
    (0x7B, Key::Left),
    (0x7C, Key::Right),
    (0x7D, Key::Down),
    (0x7E, Key::Up),
    (0x41, Key::KeypadPeriod),
    (0x43, Key::KeypadMultiply),
    (0x45, Key::KeypadPlus),
    (0x47, Key::NumLock),
    (0x4B, Key::KeypadDivide),
    (0x4C, Key::KeypadEnter),
    (0x4E, Key::KeypadMinus),
    (0x51, Key::KeypadEqual),
    (0x52, Key::Keypad0),
    (0x53, Key::Keypad1),
    (0x54, Key::Keypad2),
    (0x55, Key::Keypad3),
    (0x56, Key::Keypad4),
    (0x57, Key::Keypad5),
    (0x58, Key::Keypad6),
    (0x59, Key::Keypad7),
    (0x5B, Key::Keypad8),
    (0x5C, Key::Keypad9),
    // On macOS the PC keyboard Insert key is the Help key, and the Menu key is ContextualMenu
    (0x72, Key::Insert),
    (0x6E, Key::Menu),
];

const NO_CODE: u16 = u16::MAX;

static TO_KEY: [Option<Key>; 128] = {
    let mut table = [None; 128];
    let mut i = 0;
    while i < PAIRS.len() {
        table[PAIRS[i].0 as usize] = Some(PAIRS[i].1);
        i += 1;
    }
    table
};

static FROM_KEY: [u16; Key::COUNT] = {
    let mut table = [NO_CODE; Key::COUNT];
    let mut i = 0;
    while i < PAIRS.len() {
        table[PAIRS[i].1 as usize] = PAIRS[i].0;
        i += 1;
    }
    table
};

/// Unrecognized key codes (JIS/ISO-only keys and so on) give `None`, and the caller passes them through unchanged.
#[inline]
pub fn to_key(code: u16) -> Option<Key> {
    TO_KEY.get(code as usize).copied().flatten()
}

/// Gives `None` when macOS has no such key: the system treats the PC keyboard's Print Screen /
/// Scroll Lock / Pause as F13 / F14 / F15, so they have no key codes of their own.
#[inline]
pub fn from_key(key: Key) -> Option<u16> {
    Some(FROM_KEY[key as usize]).filter(|&code| code != NO_CODE)
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNSUPPORTED: [Key; 3] = [Key::PrintScreen, Key::ScrollLock, Key::Pause];

    #[test]
    fn every_key_has_a_code() {
        for &key in Key::ALL {
            assert_eq!(
                from_key(key).is_none(),
                UNSUPPORTED.contains(&key),
                "macOS key code for {}",
                key.name()
            );
        }
    }

    #[test]
    fn codes_round_trip() {
        for &key in Key::ALL {
            if let Some(code) = from_key(key) {
                assert_eq!(to_key(code), Some(key));
            }
        }
    }
}
