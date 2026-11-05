//! X11's keys as Okur's: the key a keycode and its keysym stand for, the character it types, and
//! the modifiers an event's state holds.
//!
//! JAWS takes letters, digits and named keys by what they are, and the punctuation keys by their
//! place on a US keyboard (Default.JKM's Comma, Dash, LeftBracket: `okur_core::input::Key`). So
//! the punctuation comes from the keycode, the evdev code plus 8 as X numbers keys under the
//! evdev and libinput drivers; everything else from the keysym the key gives without Shift.

use okur_core::input::{Key, Modifiers};

/// The core modifier bits of an X event's state.
pub mod state {
    pub const SHIFT: u32 = 1;
    pub const LOCK: u32 = 1 << 1;
    pub const CONTROL: u32 = 1 << 2;
    /// Alt, as X keyboards map it.
    pub const MOD1: u32 = 1 << 3;
    /// Num Lock, as X keyboards map it.
    pub const MOD2: u32 = 1 << 4;
    /// Super, the Windows key, as X keyboards map it.
    pub const MOD4: u32 = 1 << 6;
}

/// The keys Okur takes as held modifiers of its own, which X does not: the Okur key (Insert,
/// or CapsLock in the laptop layout) and ScrollLock, kept by the hook from their presses and
/// releases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Held {
    pub insert: bool,
    pub caps_lock: bool,
    pub scroll_lock: bool,
}

/// The X keycodes of the US punctuation places (evdev's `KEY_*` plus 8).
const PLACES: &[(u8, Key)] = &[
    (20, Key::Minus),
    (21, Key::Equals),
    (34, Key::LeftBracket),
    (35, Key::RightBracket),
    (47, Key::Semicolon),
    (48, Key::Apostrophe),
    (51, Key::Backslash),
    (59, Key::Comma),
    (60, Key::Period),
    (61, Key::Slash),
];

/// Named keysyms (X11's `keysymdef.h`) and the keys they are. The keypad's arrows and its
/// Home, End and Page keys are the keys themselves, as Windows reports them with Num Lock off;
/// its 5 then is `Numpad5` (KP_Begin), and its operators are the keypad's own.
const NAMED: &[(u32, Key)] = &[
    (0xff51, Key::LeftArrow),
    (0xff52, Key::UpArrow),
    (0xff53, Key::RightArrow),
    (0xff54, Key::DownArrow),
    (0xff50, Key::Home),
    (0xff57, Key::End),
    (0xff55, Key::PageUp),
    (0xff56, Key::PageDown),
    (0xff96, Key::LeftArrow),
    (0xff97, Key::UpArrow),
    (0xff98, Key::RightArrow),
    (0xff99, Key::DownArrow),
    (0xff95, Key::Home),
    (0xff9c, Key::End),
    (0xff9a, Key::PageUp),
    (0xff9b, Key::PageDown),
    (0xff9d, Key::Numpad5),
    (0xff9e, Key::Insert),
    (0xff9f, Key::Delete),
    (0xffab, Key::NumpadPlus),
    (0xffad, Key::NumpadMinus),
    (0xffaf, Key::NumpadSlash),
    (0xffaa, Key::NumpadStar),
    (0xff09, Key::Tab),
    (0xfe20, Key::Tab),
    (0xff0d, Key::Enter),
    (0xff8d, Key::Enter),
    (0xff1b, Key::Escape),
    (0x0020, Key::Space),
    (0xff08, Key::Backspace),
    (0xffff, Key::Delete),
    (0xff63, Key::Insert),
    (0xffe5, Key::CapsLock),
    (0xff14, Key::ScrollLock),
    (0xffe1, Key::Shift),
    (0xffe2, Key::Shift),
    (0xffe3, Key::Control),
    (0xffe4, Key::Control),
    (0xffe9, Key::Alt),
    (0xffea, Key::Alt),
    (0xfe03, Key::Alt),
    (0xffeb, Key::Windows),
    (0xffec, Key::Windows),
];

/// F1's keysym; F1 to F24 follow it.
const F1: u32 = 0xffbe;

/// The key a keycode stands for, given the keysym it gives without Shift.
pub fn key(keycode: u8, keysym: u32) -> Key {
    if let Some((_, key)) = PLACES.iter().find(|(code, _)| *code == keycode) {
        return *key;
    }
    if let Some((_, key)) = NAMED.iter().find(|(sym, _)| *sym == keysym) {
        return *key;
    }
    if (F1..F1 + 24).contains(&keysym) {
        // Within F1 to F24: the difference fits.
        return Key::F(u8::try_from(keysym - F1 + 1).unwrap_or(0));
    }
    match character(keysym) {
        Some(c) if c.is_ascii_digit() => Key::Digit(
            c.to_digit(10)
                .and_then(|d| u8::try_from(d).ok())
                .unwrap_or(0),
        ),
        Some(c) if c.is_alphabetic() => Key::Alpha(c.to_uppercase().next().unwrap_or(c)),
        _ => Key::Other(keysym),
    }
}

/// The character a keysym types, if it types one: Latin-1's keysyms are their code points,
/// and keysyms from 0x0100_0000 are a Unicode code point plus that.
pub fn character(keysym: u32) -> Option<char> {
    let code = match keysym {
        0x20..=0x7e | 0xa0..=0xff => keysym,
        0x0100_0100..=0x0110_ffff => keysym - 0x0100_0000,
        _ => return None,
    };
    char::from_u32(code).filter(|c| !c.is_control())
}

/// The modifiers of an event: X's state for Control, Alt, Shift and the Windows key, and the
/// keys Okur keeps as held itself.
pub fn modifiers(state: u32, held: Held) -> Modifiers {
    Modifiers {
        ctrl: state & state::CONTROL != 0,
        alt: state & state::MOD1 != 0,
        shift: state & state::SHIFT != 0,
        win: state & state::MOD4 != 0,
        insert: held.insert,
        caps_lock: held.caps_lock,
        scroll_lock: held.scroll_lock,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_and_digits_are_what_they_type_without_shift() {
        assert_eq!(key(38, 0x61), Key::Alpha('A'));
        assert_eq!(key(43, 0x68), Key::Alpha('H'));
        assert_eq!(key(10, 0x31), Key::Digit(1));
        assert_eq!(key(47, 0x0e7), Key::Semicolon);
    }

    #[test]
    fn punctuation_is_taken_by_its_us_place_whatever_the_layout_prints() {
        // The comma place gives "ö" on a Turkish Q keyboard: still Comma, as JAWS takes it.
        assert_eq!(key(59, 0xf6), Key::Comma);
        assert_eq!(key(60, 0x2e), Key::Period);
        assert_eq!(key(61, 0x2f), Key::Slash);
        assert_eq!(key(20, 0x2d), Key::Minus);
    }

    #[test]
    fn named_and_keypad_keys() {
        assert_eq!(key(116, 0xff54), Key::DownArrow);
        assert_eq!(key(88, 0xff99), Key::DownArrow);
        assert_eq!(key(84, 0xff9d), Key::Numpad5);
        assert_eq!(key(86, 0xffab), Key::NumpadPlus);
        assert_eq!(key(118, 0xff63), Key::Insert);
        assert_eq!(key(90, 0xff9e), Key::Insert);
        assert_eq!(key(67, 0xffbe), Key::F(1));
        assert_eq!(key(96, 0xffc9), Key::F(12));
        assert_eq!(key(23, 0xfe20), Key::Tab);
        assert_eq!(key(66, 0xffe5), Key::CapsLock);
    }

    #[test]
    fn characters_from_latin_1_and_unicode_keysyms() {
        assert_eq!(character(0x61), Some('a'));
        assert_eq!(character(0xf6), Some('ö'));
        assert_eq!(character(0x0100_0131), Some('ı'));
        assert_eq!(character(0xff54), None);
    }

    #[test]
    fn modifiers_from_the_state_and_the_keys_held() {
        let m = modifiers(
            state::CONTROL | state::SHIFT | state::MOD2,
            Held {
                insert: true,
                ..Held::default()
            },
        );
        assert!(m.ctrl && m.shift && m.insert);
        assert!(!m.alt && !m.win && !m.caps_lock);
        assert!(modifiers(state::MOD1 | state::MOD4, Held::default()).alt);
    }
}
