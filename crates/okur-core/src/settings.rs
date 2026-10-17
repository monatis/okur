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

/// What a live region's change speaks when text was appended to what it already said.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LiveRegionAppend {
    /// Only the appended text, from the start of a word the old text ended in: a streamed
    /// chat reply is read as it grows, once.
    #[default]
    Appended,
    /// The region's whole text again on every change, as JAWS 2025 does: a 40-word stream
    /// gave its growing paragraph 37 times (dev box, 2026-10-15). Appended lines (a new chat
    /// message) are read alone in both modes [obs, 206/s2-s3].
    WholeText,
}

/// What typing a character says (Basic Settings: Typing Echo).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TypingEcho {
    /// Each character as it is typed: JAWS's default (no `TypingEcho` in `DEFAULT.JCF`).
    #[default]
    Characters,
    /// Nothing: `TypingEcho=0`, the reference machine's and the dev box's. On the dev box no
    /// sound followed four letters typed in the Run box, while Backspace spoke (`okur
    /// wait-silence`, 2026-10-17). JAWS's word echo is not modeled.
    None,
}

impl TypingEcho {
    /// The echo a `DEFAULT.JCF` `TypingEcho` value sets: "0" is none; anything else is read
    /// as characters, the only other mode modeled.
    pub fn from_jcf(value: &str) -> Self {
        if value.trim() == "0" {
            Self::None
        } else {
            Self::Characters
        }
    }
}

/// Whether the Virtual Viewer also shows in a window, for someone watching the screen:
/// JAWS's "Show Virtual Viewer on Screen" (`bVirtViewer`, on in its default settings). The
/// keys stay with the viewer either way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewerDisplay {
    #[default]
    OnScreen,
    SpeechOnly,
}

/// All user settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Settings {
    pub keyboard: KeyboardSettings,
    pub live_region_append: LiveRegionAppend,
    pub typing_echo: TypingEcho,
    pub viewer_display: ViewerDisplay,
}

impl Settings {
    /// The reference machine's JAWS configuration (`old-pc`'s `DEFAULT.JCF`, unchanged since
    /// 2025-08-19, before any evidence was recorded): `KeyboardType=Laptop`, `TypingEcho=0`;
    /// and JAWS's own reading of growing live regions, so replay compares like with like.
    pub fn reference_machine() -> Self {
        Self {
            keyboard: KeyboardSettings::for_layout(KeyboardLayout::Laptop),
            live_region_append: LiveRegionAppend::WholeText,
            typing_echo: TypingEcho::None,
            viewer_display: ViewerDisplay::OnScreen,
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
