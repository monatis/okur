//! Key chords shared with the capture tooling.
//!
//! Scenario steps of type `key` use readable chords ("Down", "Shift+H", "Ctrl+Alt+Right",
//! "Insert+Down"). The reference machine sends them to JAWS as scan-coded key presses
//! (`run-scenario.ps1`); the parity harness parses the identical string into a [`KeyEvent`]
//! and drives the engine through `handle_key_event`, the same entry point the live keyboard
//! hook uses, so key bindings are verified, not just command implementations.

use sr_core::input::{Key, KeyEvent, Modifiers};

fn key_from_name(name: &str) -> Option<Key> {
    let lower = name.to_ascii_lowercase();
    let key = match lower.as_str() {
        "down" => Key::DownArrow,
        "up" => Key::UpArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "tab" => Key::Tab,
        "enter" => Key::Enter,
        "escape" | "esc" => Key::Escape,
        "space" => Key::Space,
        "backspace" => Key::Backspace,
        "delete" => Key::Delete,
        "insert" => Key::Insert,
        "alt" => Key::Alt,
        "ctrl" | "control" => Key::Control,
        "semicolon" => Key::Semicolon,
        "apostrophe" => Key::Apostrophe,
        _ => {
            let mut chars = lower.chars();
            match (chars.next(), chars.next()) {
                // The live hook reports letters by virtual-key code, i.e. uppercase.
                (Some(c), None) if c.is_ascii_lowercase() => Key::Alpha(c.to_ascii_uppercase()),
                (Some(c), None) if c.is_ascii_digit() => Key::Digit(c as u8 - b'0'),
                (Some('f'), Some(_)) => Key::F(lower.get(1..)?.parse().ok()?),
                _ => return None,
            }
        }
    };
    Some(key)
}

/// Parses a chord such as `"Ctrl+Alt+Right"` into a key-down event.
pub fn parse_chord(chord: &str) -> Result<KeyEvent, String> {
    let parts: Vec<&str> = chord.split('+').map(str::trim).collect();
    let (last, mods) = parts
        .split_last()
        .ok_or_else(|| format!("empty key chord {chord:?}"))?;
    let mut modifiers = Modifiers::none();
    for m in mods {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" => modifiers.ctrl = true,
            "alt" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "insert" => modifiers.insert = true,
            "win" => modifiers.win = true,
            other => return Err(format!("unknown modifier {other:?} in {chord:?}")),
        }
    }
    let key = key_from_name(last).ok_or_else(|| format!("unknown key {last:?} in {chord:?}"))?;
    Ok(KeyEvent::new(key, modifiers, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chords_used_by_capture_tooling() {
        let e = parse_chord("Ctrl+Alt+Right");
        assert_eq!(
            e.as_ref()
                .map(|e| (&e.key, e.modifiers.ctrl, e.modifiers.alt)),
            Ok((&Key::RightArrow, true, true))
        );
        let e = parse_chord("Insert+Down");
        assert_eq!(
            e.as_ref().map(|e| (&e.key, e.modifiers.insert)),
            Ok((&Key::DownArrow, true))
        );
        assert_eq!(parse_chord("Shift+H").map(|e| e.key), Ok(Key::Alpha('H')));
        assert_eq!(parse_chord("F7").map(|e| e.key), Ok(Key::F(7)));
        assert_eq!(parse_chord("2").map(|e| e.key), Ok(Key::Digit(2)));
        assert!(parse_chord("Hyper+X").is_err());
        assert!(parse_chord("Ctrl+").is_err());
    }
}
