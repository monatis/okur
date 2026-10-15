//! User settings, modeled on JAWS's options and defaulting to JAWS's defaults.
//!
//! The reference machine's JAWS is not on defaults (its `DEFAULT.JCF` sets the laptop
//! keyboard layout, typing echo off and manual forms mode), so parity replay runs with
//! [`Settings::reference_machine`], and every behavior that depends on a setting reads it from
//! here instead of assuming the reference configuration.

use crate::input::{Key, KeyEvent};

/// JAWS's keyboard layouts (Basic Settings: Keyboard Layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyboardLayout {
    /// The default: Insert is the JAWS key.
    #[default]
    Desktop,
    /// CapsLock is the JAWS key, and "for most of those keystrokes, the INSERT key works as
    /// well" (JAWS help).
    Laptop,
}

/// The keys that can be the JAWS key (Settings Center: JAWS Key for Current Layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JawsKey {
    Insert,
    CapsLock,
    ScrollLock,
}

/// Which keys act as the JAWS key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardSettings {
    pub layout: KeyboardLayout,
    /// The JAWS key chosen for `layout`.
    pub jaws_key: JawsKey,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self::for_layout(KeyboardLayout::Desktop)
    }
}

impl KeyboardSettings {
    /// A layout with its default JAWS key: Insert for desktop, CapsLock for laptop.
    pub fn for_layout(layout: KeyboardLayout) -> Self {
        let jaws_key = match layout {
            KeyboardLayout::Desktop => JawsKey::Insert,
            KeyboardLayout::Laptop => JawsKey::CapsLock,
        };
        Self { layout, jaws_key }
    }

    /// Whether `key` acts as the JAWS key: the chosen one, and Insert in the laptop layout
    /// [obs, 309: CapsLock+T and Insert+T both speak the title in the laptop layout].
    pub fn is_jaws_key(&self, key: Key) -> bool {
        match key {
            Key::Insert => {
                self.jaws_key == JawsKey::Insert || self.layout == KeyboardLayout::Laptop
            }
            Key::CapsLock => self.jaws_key == JawsKey::CapsLock,
            Key::ScrollLock => self.jaws_key == JawsKey::ScrollLock,
            _ => false,
        }
    }

    /// The reading command a JAWS-key chord is in this layout, where the layout has its own:
    /// the laptop layout reads the current line with CapsLock+I and the current word with
    /// CapsLock+K (JAWS 2025, laptop layout, observed on the dev box: "First line of the
    /// viewer test", "First"). `event` is resolved (`modifiers.insert` is the JAWS key).
    pub fn layout_command(&self, event: &KeyEvent) -> Option<&'static str> {
        let m = &event.modifiers;
        if self.layout != KeyboardLayout::Laptop || !m.insert || m.ctrl || m.alt || m.shift {
            return None;
        }
        match event.key {
            Key::Alpha(c) if c.eq_ignore_ascii_case(&'I') => Some("SayLine"),
            Key::Alpha(c) if c.eq_ignore_ascii_case(&'K') => Some("SayWord"),
            _ => None,
        }
    }

    /// `event` with the JAWS modifier (`modifiers.insert`) set when any JAWS key is held,
    /// and cleared when only a key that is not a JAWS key is held.
    pub fn resolve(&self, event: &KeyEvent) -> KeyEvent {
        let mut resolved = event.clone();
        let m = &event.modifiers;
        resolved.modifiers.insert = (m.insert && self.is_jaws_key(Key::Insert))
            || (m.caps_lock && self.is_jaws_key(Key::CapsLock))
            || (m.scroll_lock && self.is_jaws_key(Key::ScrollLock));
        resolved.modifiers.caps_lock = false;
        resolved.modifiers.scroll_lock = false;
        resolved
    }
}

/// All user settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub keyboard: KeyboardSettings,
}

impl Settings {
    /// The reference machine's JAWS configuration (`old-pc`'s `DEFAULT.JCF`, unchanged since
    /// 2025-08-19, before any evidence was recorded): `KeyboardType=Laptop`.
    pub fn reference_machine() -> Self {
        Self {
            keyboard: KeyboardSettings::for_layout(KeyboardLayout::Laptop),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Modifiers;

    fn chord(key: Key, insert: bool, caps_lock: bool) -> KeyEvent {
        let mut e = KeyEvent::new(key, Modifiers::none(), true);
        e.modifiers.insert = insert;
        e.modifiers.caps_lock = caps_lock;
        e
    }

    #[test]
    fn desktop_layout_uses_insert_only() {
        let desktop = KeyboardSettings::default();
        assert!(desktop.is_jaws_key(Key::Insert));
        assert!(!desktop.is_jaws_key(Key::CapsLock));
        assert!(
            desktop
                .resolve(&chord(Key::Alpha('T'), true, false))
                .modifiers
                .insert
        );
        assert!(
            !desktop
                .resolve(&chord(Key::Alpha('T'), false, true))
                .modifiers
                .insert
        );
    }

    #[test]
    fn laptop_layout_uses_caps_lock_and_insert() {
        let laptop = KeyboardSettings::for_layout(KeyboardLayout::Laptop);
        assert!(laptop.is_jaws_key(Key::CapsLock));
        assert!(laptop.is_jaws_key(Key::Insert));
        let resolved = laptop.resolve(&chord(Key::Alpha('T'), false, true));
        assert!(resolved.modifiers.insert);
        assert!(!resolved.modifiers.caps_lock);
        assert!(
            laptop
                .resolve(&chord(Key::Alpha('T'), true, false))
                .modifiers
                .insert
        );
    }

    #[test]
    fn caps_lock_can_be_the_desktop_jaws_key() {
        let desktop_caps = KeyboardSettings {
            layout: KeyboardLayout::Desktop,
            jaws_key: JawsKey::CapsLock,
        };
        assert!(!desktop_caps.is_jaws_key(Key::Insert));
        assert!(
            desktop_caps
                .resolve(&chord(Key::F(12), false, true))
                .modifiers
                .insert
        );
    }
}
