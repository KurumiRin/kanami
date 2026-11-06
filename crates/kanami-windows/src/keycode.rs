//! Two-way mapping between Windows virtual key codes (`VK_*`) and [`Key`].

use kanami_core::Key;

const PAIRS: &[(u16, Key)] = &[
    (0x41, Key::A),
    (0x42, Key::B),
    (0x43, Key::C),
    (0x44, Key::D),
    (0x45, Key::E),
    (0x46, Key::F),
    (0x47, Key::G),
    (0x48, Key::H),
    (0x49, Key::I),
    (0x4A, Key::J),
    (0x4B, Key::K),
    (0x4C, Key::L),
    (0x4D, Key::M),
    (0x4E, Key::N),
    (0x4F, Key::O),
    (0x50, Key::P),
    (0x51, Key::Q),
    (0x52, Key::R),
    (0x53, Key::S),
    (0x54, Key::T),
    (0x55, Key::U),
    (0x56, Key::V),
    (0x57, Key::W),
    (0x58, Key::X),
    (0x59, Key::Y),
    (0x5A, Key::Z),
    (0x31, Key::Num1),
    (0x32, Key::Num2),
    (0x33, Key::Num3),
    (0x34, Key::Num4),
    (0x35, Key::Num5),
    (0x36, Key::Num6),
    (0x37, Key::Num7),
    (0x38, Key::Num8),
    (0x39, Key::Num9),
    (0x30, Key::Num0),
    (0x1B, Key::Escape),
    (0x0D, Key::Enter),
    (0x09, Key::Tab),
    (0x20, Key::Space),
    (0x08, Key::Backspace),
    (0x2E, Key::Delete),
    (0xC0, Key::Grave),
    (0xBD, Key::Minus),
    (0xBB, Key::Equal),
    (0xDB, Key::LeftBracket),
    (0xDD, Key::RightBracket),
    (0xDC, Key::Backslash),
    (0xBA, Key::Semicolon),
    (0xDE, Key::Quote),
    (0xBC, Key::Comma),
    (0xBE, Key::Period),
    (0xBF, Key::Slash),
    (0x25, Key::Left),
    (0x27, Key::Right),
    (0x26, Key::Up),
    (0x28, Key::Down),
    (0x24, Key::Home),
    (0x23, Key::End),
    (0x21, Key::PageUp),
    (0x22, Key::PageDown),
    (0x70, Key::F1),
    (0x71, Key::F2),
    (0x72, Key::F3),
    (0x73, Key::F4),
    (0x74, Key::F5),
    (0x75, Key::F6),
    (0x76, Key::F7),
    (0x77, Key::F8),
    (0x78, Key::F9),
    (0x79, Key::F10),
    (0x7A, Key::F11),
    (0x7B, Key::F12),
    (0x7C, Key::F13),
    (0x7D, Key::F14),
    (0x7E, Key::F15),
    (0x7F, Key::F16),
    (0x80, Key::F17),
    (0x81, Key::F18),
    (0x82, Key::F19),
    (0x83, Key::F20),
    (0x14, Key::CapsLock),
    (0xA2, Key::LeftControl),
    (0xA0, Key::LeftShift),
    (0xA4, Key::LeftOption),
    (0x5B, Key::LeftCommand),
    (0xA3, Key::RightControl),
    (0xA1, Key::RightShift),
    (0xA5, Key::RightOption),
    (0x5C, Key::RightCommand),
    (0x60, Key::Keypad0),
    (0x61, Key::Keypad1),
    (0x62, Key::Keypad2),
    (0x63, Key::Keypad3),
    (0x64, Key::Keypad4),
    (0x65, Key::Keypad5),
    (0x66, Key::Keypad6),
    (0x67, Key::Keypad7),
    (0x68, Key::Keypad8),
    (0x69, Key::Keypad9),
    (0x6A, Key::KeypadMultiply),
    (0x6B, Key::KeypadPlus),
    (0x6D, Key::KeypadMinus),
    (0x6E, Key::KeypadPeriod),
    (0x6F, Key::KeypadDivide),
    (0x90, Key::NumLock),
    (0x2D, Key::Insert),
    (0x2C, Key::PrintScreen),
    (0x91, Key::ScrollLock),
    (0x13, Key::Pause),
    (0x5D, Key::Menu),
];

/// Virtual key code of Return. Keypad Enter uses the same code and is told apart by the "extended key" flag.
const VK_RETURN: u16 = 0x0D;

/// No matching key code: Fn is handled by the keyboard firmware, and the keypad `=` exists only on Mac keyboards.
const NO_CODE: u16 = 0;

static TO_KEY: [Option<Key>; 256] = {
    let mut table = [None; 256];
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
    table[Key::KeypadEnter as usize] = VK_RETURN;
    table
};

/// `extended` is the extended-key flag from the low-level hook (`LLKHF_EXTENDED`).
pub fn to_key(vk: u32, extended: bool) -> Option<Key> {
    if vk == VK_RETURN.into() && extended {
        return Some(Key::KeypadEnter);
    }
    TO_KEY.get(vk as usize).copied().flatten()
}

/// Gives `None` when there is no matching key code (Fn, keypad `=`).
pub fn from_key(key: Key) -> Option<u16> {
    Some(FROM_KEY[key as usize]).filter(|&vk| vk != NO_CODE)
}

/// Keys that must be sent with `KEYEVENTF_EXTENDEDKEY`.
pub fn is_extended(key: Key) -> bool {
    matches!(
        key,
        Key::Left
            | Key::Right
            | Key::Up
            | Key::Down
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown
            | Key::Delete
            | Key::RightControl
            | Key::RightOption
            | Key::LeftCommand
            | Key::RightCommand
            | Key::Insert
            | Key::PrintScreen
            | Key::Menu
            | Key::NumLock
            | Key::KeypadDivide
            | Key::KeypadEnter
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_but_fn_has_a_code() {
        for &key in Key::ALL {
            match key {
                Key::Fn | Key::KeypadEqual => assert_eq!(from_key(key), None),
                key => assert_eq!(
                    to_key(from_key(key).unwrap().into(), is_extended(key)),
                    Some(key),
                    "{}",
                    key.name()
                ),
            }
        }
    }
}
