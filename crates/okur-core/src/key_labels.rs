//! Key labels: what the typing echo says for a key, and whether it says it, as JAWS's Manage Key
//! Labels sets them (Settings Center, Keyboard, General; JAWS 2025 help, Manage Key Labels). Each
//! key has a name, a label and a mute status: Speak, Mute, or Always Speak, which cannot be
//! muted. The keys and their defaults are a table (`locales/en-US/key-labels.txt`), JAWS's own
//! list read on old-pc (103 keys; Tab and F18 to F24 muted, the modifiers always spoken).
//!
//! The user's changes are kept apart from the table, in the settings ([`KeyLabelChange`]), as
//! JAWS keeps them in an application's settings file, `[KeyLabels] Space=0|Space` (muted) or
//! `Return=1|Return key` (spoken, relabelled) [obs, key-labels-store-probe.ps1, old-pc,
//! 2026-11-06]. Letters, Numbers and Symbols have no label of their own: muting one silences the
//! characters echo of its keys.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use crate::input::{Key, KeyEvent};

const TABLE: &str = include_str!("../locales/en-US/key-labels.txt");

/// Whether a key's label is said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MuteStatus {
    Speak,
    Mute,
    /// Said always: the modifiers, which Toggle Mute leaves as they are.
    AlwaysSpeak,
}

/// One key's row: its name, its label, and whether the label is said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLabel {
    pub name: String,
    /// Empty for a group of keys without a label of its own (Letters, Numbers, Symbols).
    pub label: String,
    pub status: MuteStatus,
}

/// A key's label or mute status the user changed: JAWS's `[KeyLabels] name=status|label`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyLabelChange {
    /// The key's name in the table ("Space", "Return").
    pub key: String,
    pub label: String,
    pub muted: bool,
}

/// The table's rows, Okur's defaults, in its order (JAWS's list's, by name).
pub fn defaults() -> &'static [KeyLabel] {
    static ROWS: OnceLock<Vec<KeyLabel>> = OnceLock::new();
    ROWS.get_or_init(|| {
        TABLE
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .filter_map(|line| {
                let mut parts = line.split('\t');
                let name = parts.next()?;
                let label = parts.next()?;
                let status = match parts.next()? {
                    "mute" => MuteStatus::Mute,
                    "always" => MuteStatus::AlwaysSpeak,
                    _ => MuteStatus::Speak,
                };
                Some(KeyLabel {
                    name: name.to_owned(),
                    label: label.to_owned(),
                    status,
                })
            })
            .collect()
    })
}

/// Every key's row with the user's `changes` over the defaults: what Manage Key Labels lists.
pub fn rows(changes: &[KeyLabelChange]) -> Vec<KeyLabel> {
    defaults().iter().map(|row| changed(row, changes)).collect()
}

/// `row` with the user's change of it, if any. An Always Speak key stays spoken.
fn changed(row: &KeyLabel, changes: &[KeyLabelChange]) -> KeyLabel {
    let Some(change) = changes
        .iter()
        .find(|c| c.key.eq_ignore_ascii_case(&row.name))
    else {
        return row.clone();
    };
    let status = match row.status {
        MuteStatus::AlwaysSpeak => MuteStatus::AlwaysSpeak,
        _ if change.muted => MuteStatus::Mute,
        _ => MuteStatus::Speak,
    };
    KeyLabel {
        name: row.name.clone(),
        label: change.label.clone(),
        status,
    }
}

/// The row of the key named `name`, with the user's change.
pub fn row(changes: &[KeyLabelChange], name: &str) -> Option<KeyLabel> {
    defaults()
        .iter()
        .find(|row| row.name.eq_ignore_ascii_case(name))
        .map(|row| changed(row, changes))
}

/// What the echo says for the key named `name`: its label, or nothing while it is muted.
pub fn spoken(changes: &[KeyLabelChange], name: &str) -> Option<String> {
    row(changes, name)
        .filter(|row| row.status != MuteStatus::Mute)
        .map(|row| row.label)
}

/// The table's name of the key `event` pressed, without its modifiers: the names JAWS's list
/// gives the keys, Return for Enter, Prior and Next for Page Up and Page Down, Back for
/// Backspace. `None` for a key the list has no row of (a typed character's group is
/// [`character_group`]).
pub fn key_name(event: &KeyEvent) -> Option<String> {
    let name = match event.key {
        Key::UpArrow => "Up",
        Key::DownArrow => "Down",
        Key::LeftArrow => "Left",
        Key::RightArrow => "Right",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "Prior",
        Key::PageDown => "Next",
        Key::Tab => "Tab",
        Key::Enter => "Return",
        Key::Escape => "Escape",
        Key::Space => "Space",
        Key::Backspace => "Back",
        Key::Delete => "Delete",
        Key::Insert => "Insert",
        Key::CapsLock => "Capital",
        Key::ScrollLock => "Scroll",
        Key::Numpad5 => "NumPad5",
        Key::NumpadPlus => "Add",
        Key::NumpadMinus => "Subtract",
        Key::NumpadSlash => "Divide",
        Key::NumpadStar => "Multiply",
        Key::Control => "Control",
        Key::Alt => "Menu",
        Key::Shift => "Shift",
        Key::Windows => "LWin",
        Key::F(number) => return Some(format!("F{number}")),
        _ => return None,
    };
    Some(name.to_owned())
}

/// The group a typed character's key is in: Letters, Numbers or Symbols.
pub fn character_group(c: char) -> &'static str {
    if c.is_alphabetic() {
        "Letters"
    } else if c.is_numeric() {
        "Numbers"
    } else {
        "Symbols"
    }
}

/// Whether the characters echo says `c`: its group is not muted.
pub fn character_spoken(changes: &[KeyLabelChange], c: char) -> bool {
    row(changes, character_group(c)).is_none_or(|row| row.status != MuteStatus::Mute)
}

/// The changes JAWS's settings hold, each key's `[KeyLabels]` value (`0|Space`: muted, its
/// label Space), where it differs from the default; `lookup` gives a key's value, `None` where
/// the file leaves it out.
pub fn from_jaws(lookup: &dyn Fn(&str) -> Option<String>) -> Vec<KeyLabelChange> {
    defaults()
        .iter()
        .filter_map(|row| {
            let value = lookup(&row.name)?;
            let (status, label) = value.split_once('|')?;
            let muted = status.trim() == "0";
            let label = label.trim();
            // A group's label is written as its name, shown as "No Label".
            let label = if row.label.is_empty() && label == row.name {
                ""
            } else {
                label
            };
            let same = label == row.label && muted == (row.status == MuteStatus::Mute);
            (!same).then(|| KeyLabelChange {
                key: row.name.clone(),
                label: label.to_owned(),
                muted,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(key: &str, label: &str, muted: bool) -> KeyLabelChange {
        KeyLabelChange {
            key: key.to_owned(),
            label: label.to_owned(),
            muted,
        }
    }

    #[test]
    fn the_table_is_jaws_list() {
        let rows = defaults();
        assert_eq!(rows.len(), 103);
        let muted: Vec<&str> = rows
            .iter()
            .filter(|r| r.status == MuteStatus::Mute)
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(
            muted,
            ["F18", "F19", "F20", "F21", "F22", "F23", "F24", "Tab"]
        );
        assert_eq!(spoken(&[], "Return").as_deref(), Some("Enter"));
        assert_eq!(spoken(&[], "Next").as_deref(), Some("Page down"));
        assert_eq!(spoken(&[], "Tab"), None);
    }

    #[test]
    fn a_change_relabels_or_mutes_but_not_an_always_spoken_key() {
        let changes = [
            change("Return", "Return key", false),
            change("space", "Space", true),
            change("Shift", "Shift", true),
            change("Tab", "Tab", false),
        ];
        assert_eq!(spoken(&changes, "Return").as_deref(), Some("Return key"));
        assert_eq!(spoken(&changes, "Space"), None);
        assert_eq!(spoken(&changes, "Shift").as_deref(), Some("Shift"));
        assert_eq!(spoken(&changes, "Tab").as_deref(), Some("Tab"));
    }

    #[test]
    fn a_muted_group_silences_its_characters() {
        let changes = [change("Letters", "", true)];
        assert!(!character_spoken(&changes, 'a'));
        assert!(character_spoken(&changes, '1'));
        assert!(character_spoken(&changes, ','));
        assert!(character_spoken(&[], 'a'));
    }

    #[test]
    fn jaws_settings_give_the_changes_that_differ() {
        let lookup = |name: &str| match name {
            "Space" => Some("0|Space".to_owned()),
            "Return" => Some("1|Return key".to_owned()),
            "Letters" => Some("0|Letters".to_owned()),
            // As the default: no change.
            "Home" => Some("1|Home".to_owned()),
            _ => None,
        };
        assert_eq!(
            from_jaws(&lookup),
            vec![
                change("Letters", "", true),
                change("Return", "Return key", false),
                change("Space", "Space", true),
            ]
        );
    }

    #[test]
    fn keys_have_their_rows() {
        let key = |key| KeyEvent::new(key, crate::input::Modifiers::default(), true);
        for k in [
            Key::Enter,
            Key::Escape,
            Key::Home,
            Key::F(2),
            Key::PageDown,
            Key::Alt,
        ] {
            let name = key_name(&key(k));
            assert!(name.as_deref().and_then(|n| row(&[], n)).is_some(), "{k:?}");
        }
    }
}
