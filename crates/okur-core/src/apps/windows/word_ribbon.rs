//! Microsoft Word's ribbon, read as JAWS reads it [obs, 1043]: shorter than a NetUI ribbon
//! elsewhere (WordPad's names its panes, its row of tabs, a tab's place and state, 970/s1).
//! - Alt or F10 enters its tabs: "Upper Ribbon | Ribbon | Home Tab", the ribbon by its own name;
//!   the arrows then say a tab alone, "Insert Tab".
//! - Down enters the lower ribbon as a menu: "menu | | Pages group | Cover Page sub menu", the
//!   commands' group with its role, a split button as a sub menu; the arrows and Tab in the
//!   group then say a command alone, "Blank Page Button", with no place and no hint.
//!
//! - Escape back to the document: "Leaving menus | Leaving ribbons | Edit" [obs, 1043/s8,
//!   1042/s4], the engine's "Leaving menus", then this module's; Word's window, focused on the
//!   way, says nothing of its own.
//!
//! A command this module does not word (only Word's buttons and split buttons are recorded)
//! is the engine's.

use crate::announcement::Announcement;
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;
use crate::linearize::{role_marker, state_marker};
use crate::messages::msg;

/// Word's process.
const WORD: &str = "WINWORD.EXE";

/// Where in Word's ribbon the focus last was.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Place {
    #[default]
    Outside,
    Tabs,
    /// A command in the lower ribbon, in the group (its tool bar) named.
    Lower(Option<String>),
}

/// Word's ribbon.
#[derive(Debug, Default)]
pub struct WordRibbon {
    place: Place,
    /// The ribbon was just left for Word's own window, before its document takes the focus.
    leaving: bool,
}

/// Whether the focus is in Word.
fn in_word(focus: &FocusEvent) -> bool {
    focus
        .app
        .as_deref()
        .is_some_and(|app| app.eq_ignore_ascii_case(WORD))
}

impl WordRibbon {
    /// What the focus `now` (a tab or a lower ribbon's command) says, from the place before.
    fn speech(&self, focus: &FocusEvent, now: &Place) -> Option<String> {
        let name = focus
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())?;
        let mut lines = Vec::new();
        match now {
            Place::Tabs => {
                if self.place == Place::Outside {
                    lines.push(role_marker(msg::ribbon_upper().as_str()));
                    lines.extend(focus.ribbon_name.clone());
                }
                lines.push(format!("{name} {}", role_marker(msg::role_tab().as_str())));
            }
            Place::Lower(group) => {
                let control = match focus.role {
                    Role::Button => format!("{name} {}", role_marker(msg::role_button().as_str())),
                    Role::SplitButton => {
                        format!("{name} {}", state_marker(msg::state_sub_menu().as_str()))
                    }
                    _ => return None,
                };
                // JAWS's empty part after "menu" says nothing.
                if !matches!(self.place, Place::Lower(_)) {
                    lines.push(role_marker(msg::role_menu().as_str()));
                }
                if self.place != *now {
                    lines.extend(
                        group
                            .as_ref()
                            .map(|g| format!("{g} {}", role_marker(msg::role_group().as_str()))),
                    );
                }
                lines.push(control);
            }
            Place::Outside => return None,
        }
        Some(lines.join("\n"))
    }
}

impl AppModule for WordRibbon {
    fn owns(&self, focus: &FocusEvent) -> bool {
        in_word(focus) && focus.in_ribbon
    }

    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        // Out of the ribbon into Word's window, then its document.
        let left = self.place != Place::Outside || self.leaving;
        if left && in_word(focus) && !focus.in_ribbon {
            self.place = Place::Outside;
            match focus.role {
                Role::Window => {
                    self.leaving = true;
                    return FocusResponse::Silent;
                }
                Role::Document => {
                    self.leaving = false;
                    return FocusResponse::Speak(Announcement::from_text(&format!(
                        "{}\n{}",
                        msg::ribbons_left(),
                        role_marker(msg::role_edit().as_str())
                    )));
                }
                _ => {}
            }
        }
        self.leaving = false;
        let now = if in_word(focus) && focus.in_ribbon && focus.ribbon_menu {
            if focus.role == Role::Tab {
                Place::Tabs
            } else {
                Place::Lower(
                    focus
                        .groups
                        .iter()
                        .rev()
                        .find(|g| g.tool_bar)
                        .map(|g| g.name.trim().to_owned()),
                )
            }
        } else {
            Place::Outside
        };
        // A key tip's tab (Alt, then W): JAWS says nothing until a command acts [obs, 1042/s3].
        if cx.letter_key && now == Place::Tabs && self.place != Place::Outside {
            self.place = now;
            return FocusResponse::Silent;
        }
        // A command it does not word, unavailable, or outside the ribbon's menus: the engine's.
        let speech = (!focus.states.disabled)
            .then(|| self.speech(focus, &now))
            .flatten();
        self.place = now;
        match speech {
            Some(speech) => FocusResponse::Speak(Announcement::from_text(&speech)),
            None => FocusResponse::Pass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::AppMode;
    use crate::event::GroupEntry;

    fn said(ribbon: &mut WordRibbon, focus: &FocusEvent) -> Option<String> {
        let mut mode = AppMode::NativeControl;
        let (mut title, mut kind) = (None, None);
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: false,
        };
        match ribbon.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) => Some(announcement.render()),
            _ => None,
        }
    }

    fn word(role: Role, name: &str) -> FocusEvent {
        let mut focus = FocusEvent::new(role).with_name(name.to_owned());
        focus.app = Some("WINWORD.EXE".into());
        focus.in_ribbon = true;
        focus.ribbon_menu = true;
        focus.ribbon_name = Some("Ribbon".into());
        focus.item_index = Some(1);
        focus.item_count = Some(8);
        focus
    }

    /// Word's ribbon as JAWS reads it: entered by its own name, tabs and commands said alone,
    /// the lower ribbon as a menu with the group's role [obs, 1043/s1-s7].
    #[test]
    fn words_ribbon_is_read_as_jaws_reads_it() {
        let mut ribbon = WordRibbon::default();
        let home = word(Role::Tab, "Home");
        assert_eq!(
            said(&mut ribbon, &home).as_deref(),
            Some(" \u{1d} Upper Ribbon \u{1e} \nRibbon\nHome  \u{1d} Tab \u{1e} ")
        );
        let insert = word(Role::Tab, "Insert");
        assert_eq!(
            said(&mut ribbon, &insert).as_deref(),
            Some("Insert  \u{1d} Tab \u{1e} ")
        );
        let mut cover = word(Role::SplitButton, "Cover Page");
        cover.groups = vec![
            GroupEntry::group("Lower Ribbon"),
            GroupEntry::tool_bar("Pages"),
        ];
        assert_eq!(
            said(&mut ribbon, &cover).as_deref(),
            Some(" \u{1d} menu \u{1e} \nPages  \u{1d} group \u{1e} \nCover Page  \u{1d}sub menu\u{1e} ")
        );
        let blank = FocusEvent {
            role: Role::Button,
            name: Some("Blank Page".into()),
            ..cover.clone()
        };
        assert_eq!(
            said(&mut ribbon, &blank).as_deref(),
            Some("Blank Page  \u{1d} Button \u{1e} ")
        );
        // Escape: Word's window says nothing, its document what JAWS says of it after the
        // ribbon; the ribbon is entered anew after.
        let mut window = FocusEvent::new(Role::Window);
        window.app = Some("WINWORD.EXE".into());
        assert_eq!(said(&mut ribbon, &window), None);
        let mut document = FocusEvent::new(Role::Document);
        document.app = Some("WINWORD.EXE".into());
        assert_eq!(
            said(&mut ribbon, &document).as_deref(),
            Some("Leaving ribbons\n \u{1d} Edit \u{1e} ")
        );
        assert_eq!(said(&mut ribbon, &document), None);
        assert!(said(&mut ribbon, &home).is_some_and(|s| s.starts_with(" \u{1d} Upper Ribbon")));
        // Another application's ribbon is not this module's.
        let mut wordpad = word(Role::Tab, "Home");
        wordpad.app = Some("wordpad.exe".into());
        assert_eq!(said(&mut ribbon, &wordpad), None);
    }
}
