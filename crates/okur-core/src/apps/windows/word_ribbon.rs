//! Microsoft Word's ribbon, read as JAWS reads it [obs, 1043]: shorter than a NetUI ribbon
//! elsewhere (WordPad's names its panes, its row of tabs, a tab's place and state, 970/s1).
//! - Alt or F10 enters its tabs: "Upper Ribbon | Ribbon | Home Tab", the ribbon by its own name;
//!   the arrows then say a tab alone, "Insert Tab".
//! - Down enters the lower ribbon's first group: its name, then its first command, "Clipboard |
//!   Paste split button" [obs, 1045/s2]; the arrows and Tab in the group say a command alone,
//!   "Blank Page Button", with no place and no hint, an unavailable one "Cut Button Unavailable",
//!   a toggle button its state, "Format Painter Button not checked" [obs, 1045/s3, s5].
//! - A command's ScreenTip title follows it, unless the one has the other in it: "Grow Font
//!   Button | Increase Font Size (Alt+Ctrl+!)", "Clear Formatting Button | Clear All
//!   Formatting", but "Cut Button" (its tip "Cut (Ctrl+X)"), "Office Clipboard... Button" (its
//!   tip "Clipboard") [obs, 1045/s3-s12].
//! - Another group's command says the group's name first; an edit combo, "Font: edit combo |
//!   Calibri", only itself, and Grow Font after it no group either [obs, 1045/s7-s9].
//! - A button that drops a list or a grid down is a sub menu, and the first one reached makes
//!   the ribbon a menu: "menu | | Font group | Change Case sub menu", the group with its role
//!   [obs, 1045/s11; 1043/s5, entered by Down].
//!
//! - Escape back to the document: "Leaving menus | Leaving ribbons | Edit" [obs, 1043/s8,
//!   1042/s4], the engine's "Leaving menus", then this module's; Word's window, focused on the
//!   way, says nothing of its own.
//!
//! A command this module does not word (only Word's buttons, split buttons, sub menus, check
//! boxes and edit combos are recorded) is the engine's.

use crate::announcement::Announcement;
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;
use crate::linearize::{role_marker, state_marker};
use crate::messages::msg;

/// Word's process.
pub(crate) const WORD: &str = "WINWORD.EXE";

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
    /// A sub menu of the lower ribbon was reached: its commands are a menu's from then on, a
    /// group said with its role, until the focus leaves the lower ribbon.
    menu: bool,
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
                let combo =
                    focus.role == Role::Edit && focus.container_role == Some(Role::ComboBox);
                let control = control_speech(focus, name)?;
                let with_role =
                    |g: &String| format!("{g} {}", role_marker(msg::role_group().as_str()));
                if focus.role == Role::MenuButton && !self.menu {
                    // JAWS's empty part after "menu" says nothing.
                    lines.push(role_marker(msg::role_menu().as_str()));
                    lines.extend(group.as_ref().map(with_role));
                } else if self.place != *now && !combo {
                    lines.extend(group.as_ref().map(|g| {
                        if self.menu {
                            with_role(g)
                        } else {
                            g.clone()
                        }
                    }));
                }
                lines.extend(control);
            }
            Place::Outside => return None,
        }
        Some(lines.join("\n"))
    }
}

/// A lower ribbon's command as JAWS says it, without its group: its name and role, its states,
/// its value for an edit combo, then its ScreenTip's title. `None` for a kind not recorded.
fn control_speech(focus: &FocusEvent, name: &str) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    let mut line = match focus.role {
        Role::Button => format!("{name} {}", role_marker(msg::role_button().as_str())),
        Role::SplitButton => format!("{name} {}", role_marker(msg::role_split_button().as_str())),
        Role::MenuButton => format!("{name} {}", state_marker(msg::state_sub_menu().as_str())),
        // A check box with its state and no hint: "Show | Ruler check box not checked"
        // [obs, 1046/s14].
        Role::CheckBox => format!("{name} {}", role_marker(msg::role_check_box().as_str())),
        Role::Edit if focus.container_role == Some(Role::ComboBox) => {
            lines.push(format!(
                "{name} {}",
                role_marker(msg::role_edit_combo().as_str())
            ));
            lines.extend(
                focus
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned),
            );
            return Some(lines);
        }
        _ => return None,
    };
    if let Some(checked) = focus.states.checked {
        let state = if checked {
            msg::state_checked()
        } else {
            msg::state_not_checked()
        };
        line.push_str(&state_marker(state.as_str()));
    }
    if focus.states.disabled {
        line.push_str(&state_marker(msg::state_unavailable().as_str()));
    }
    lines.push(line);
    lines.extend(screen_tip(focus, name));
    Some(lines)
}

/// A command's ScreenTip title (its help text), unless it has the name in it or the name has
/// it: "Cut (Ctrl+X)" and "Clipboard" (for "Office Clipboard...") say nothing more.
fn screen_tip(focus: &FocusEvent, name: &str) -> Option<String> {
    let tip = focus
        .help_text
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())?;
    let (tip_words, name_words) = (
        tip.to_lowercase(),
        name.trim().trim_end_matches('.').to_lowercase(),
    );
    (!tip_words.contains(&name_words) && !name_words.contains(&tip_words)).then(|| tip.to_owned())
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
            self.menu = false;
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
        // A command it does not word, or outside the ribbon's menus: the engine's.
        let speech = self.speech(focus, &now);
        if matches!(now, Place::Lower(_)) {
            self.menu |= focus.role == Role::MenuButton;
        } else {
            self.menu = false;
        }
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
        let mut cover = word(Role::MenuButton, "Cover Page");
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

    /// The Home tab's first commands by Tab, as JAWS reads them [obs, 1045/s2-s12]: the first
    /// group by its name alone, an unavailable command, a toggle button's state, a ScreenTip
    /// title only where it has more than the name, an edit combo with no group, and the first
    /// sub menu making the ribbon a menu.
    #[test]
    fn the_home_tabs_commands_are_read_as_jaws_reads_them() {
        let mut ribbon = WordRibbon::default();
        let home = word(Role::Tab, "Home");
        assert!(said(&mut ribbon, &home).is_some());
        let in_group = |role: Role, name: &str, group: &str| {
            let mut focus = word(role, name);
            focus.groups = vec![
                GroupEntry::group("Lower Ribbon"),
                GroupEntry::tool_bar(group),
            ];
            focus
        };
        let mut paste = in_group(Role::SplitButton, "Paste", "Clipboard");
        paste.value = Some("Paste (Ctrl+V)".into());
        paste.help_text = Some("Paste (Ctrl+V)".into());
        assert_eq!(
            said(&mut ribbon, &paste).as_deref(),
            Some("Clipboard\nPaste  \u{1d} split button \u{1e} ")
        );
        let mut cut = in_group(Role::Button, "Cut", "Clipboard");
        cut.states.disabled = true;
        cut.help_text = Some("Cut (Ctrl+X)".into());
        assert_eq!(
            said(&mut ribbon, &cut).as_deref(),
            Some("Cut  \u{1d} Button \u{1e}  \u{1d}Unavailable\u{1e} ")
        );
        let mut painter = in_group(Role::Button, "Format Painter", "Clipboard");
        painter.states.checked = Some(false);
        assert_eq!(
            said(&mut ribbon, &painter).as_deref(),
            Some("Format Painter  \u{1d} Button \u{1e}  \u{1d}not checked\u{1e} ")
        );
        let mut clipboard = in_group(Role::Button, "Office Clipboard...", "Clipboard");
        clipboard.help_text = Some("Clipboard".into());
        assert_eq!(
            said(&mut ribbon, &clipboard).as_deref(),
            Some("Office Clipboard...  \u{1d} Button \u{1e} ")
        );
        let mut font = in_group(Role::Edit, "Font:", "Font");
        font.container_role = Some(Role::ComboBox);
        font.value = Some("Calibri".into());
        assert_eq!(
            said(&mut ribbon, &font).as_deref(),
            Some("Font:  \u{1d} edit combo \u{1e} \nCalibri")
        );
        let mut grow = in_group(Role::Button, "Grow Font", "Font");
        grow.help_text = Some("Increase Font Size (Alt+Ctrl+!)".into());
        assert_eq!(
            said(&mut ribbon, &grow).as_deref(),
            Some("Grow Font  \u{1d} Button \u{1e} \nIncrease Font Size (Alt+Ctrl+!)")
        );
        let case = in_group(Role::MenuButton, "Change Case", "Font");
        assert_eq!(
            said(&mut ribbon, &case).as_deref(),
            Some(" \u{1d} menu \u{1e} \nFont  \u{1d} group \u{1e} \nChange Case  \u{1d}sub menu\u{1e} ")
        );
        let mut clear = in_group(Role::Button, "Clear Formatting", "Font");
        clear.help_text = Some("Clear All Formatting".into());
        assert_eq!(
            said(&mut ribbon, &clear).as_deref(),
            Some("Clear Formatting  \u{1d} Button \u{1e} \nClear All Formatting")
        );
        // A check box: its state, no hint [obs, 1046/s15].
        let mut gridlines = in_group(Role::CheckBox, "Gridlines", "Font");
        gridlines.states.checked = Some(false);
        assert_eq!(
            said(&mut ribbon, &gridlines).as_deref(),
            Some("Gridlines  \u{1d} check box \u{1e}  \u{1d}not checked\u{1e} ")
        );
        // Once the ribbon is a menu, another group is said with its role.
        let bold = in_group(Role::Button, "Bold", "Paragraph");
        assert_eq!(
            said(&mut ribbon, &bold).as_deref(),
            Some("Paragraph  \u{1d} group \u{1e} \nBold  \u{1d} Button \u{1e} ")
        );
    }
}
