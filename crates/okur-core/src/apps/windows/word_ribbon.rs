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
//! - A button that drops a list or a grid down is a sub menu, reached from any other command a
//!   menu entered: "menu | | Font group | Change Case sub menu", the group with its role
//!   [obs, 1045/s11; 1043/s5, entered by Down]. The next other command leaves it unsaid: Text
//!   Effects after Clear Formatting and the Font group's buttons is "menu | | Font group | Text
//!   Effects and Typography sub menu" again, and the Paragraph group after it is named alone,
//!   "Paragraph | Bullets split button" [obs, 1048/s19, s23, s25].
//!
//! - A gallery's item is a button with its place: "Heading 2 Button | 2 of 17"; the item its key
//!   tip opens the gallery on is not said, nor the gallery's window [obs, 1064/s3-s7].
//! - While Word's key tips lead (a key tip moved the focus, and only Escape since), Escape
//!   steps back through them, from a gallery to its button, then to its tab, and nothing is
//!   said [obs, 1064/s8, s9].
//! - Escape back to the document: "Leaving menus | Leaving ribbons | Edit" [obs, 1043/s8,
//!   1042/s4], the engine's "Leaving menus", then this module's; Word's window, focused on the
//!   way, says nothing of its own.
//!
//! Around the menus, in the ribbon still (the Quick Access Toolbar, the File tab, the buttons
//! and the search box beside the tabs, Collapse the Ribbon), commands are said as controls are,
//! with their hints and no place [obs, 1051/s4-s12]:
//! - leaving the menus or a sub menu for them says "Leaving menus", never "Leaving ribbons":
//!   "menu | Leaving menus | Customize Toolbar sub menu | To move through items press up or
//!   down arrow.", then "Leaving menus | Can't Repeat Button Unavailable";
//! - a grouping entered is named, with its state: "Can't Undo Unavailable | Can't Undo split
//!   button Unavailable | To move to an item, use the RIGHT ARROW, ..."; the toolbar is not;
//! - the row of tabs is named as it is entered, and its buttons are activated by Enter: "Ribbon
//!   Tabs Tab | Share Button | To activate press Enter.", "Sign in Link | Get to your
//!   documents ... | To activate press Enter.";
//! - the search box is in its menu item: "Tell me what you want to do... sub menu | Tell me
//!   what you want to do... Edit | Type in text.";
//! - the ribbon taking the focus itself on the way names it: "Ribbon | Collapse the Ribbon
//!   Button not checked | To activate press spacebar.";
//! - back on the menus: "menu | | Upper Ribbon | Editing group | Select sub menu" [obs,
//!   1051/s13].
//!
//! A command this module does not word (only Word's buttons, split buttons, sub menus, check
//! boxes and edit combos are recorded) is the engine's.

use crate::announcement::{Announcement, Line, Part, PartKind};
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::{FocusEvent, GroupEntry};
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
    /// In the ribbon, around its menus: the Quick Access Toolbar, the File tab, the buttons and
    /// the search box beside the tabs, Collapse the Ribbon.
    Around,
}

/// Word's ribbon.
#[derive(Debug, Default)]
pub struct WordRibbon {
    place: Place,
    /// The focus is on a sub menu of the lower ribbon: a menu, its group said with its role.
    menu: bool,
    /// The ribbon was just left for Word's own window, before its document takes the focus.
    leaving: bool,
    /// The groups the last command around the menus was in: the next one names those it
    /// enters.
    groups: Vec<GroupEntry>,
    /// The document's view was asked for, back in it from the ribbon.
    view_wanted: bool,
    /// Word's key tips lead the ribbon: a key tip moved the focus, and only Escape since.
    key_tips: bool,
}

/// Back in Word's document from the ribbon: "Leaving ribbons | Edit"; in Read Mode, the view
/// and JAWS's warning in Okur's name, "Leaving ribbons | reading layout View | WARNING! Okur
/// does not support this view. ..." [obs, 1042/s4, s8; 1059/s3]: Okur reads Read Mode as JAWS
/// does, the document's first line wherever the arrows go [obs, 1059/s4-s6].
fn back_in_document(view: Option<&str>) -> String {
    let rest = match view {
        Some(super::word::READ_MODE) => format!(
            "{}\n{}",
            msg::word_view_read(),
            msg::word_read_mode_warning()
        ),
        _ => role_marker(msg::role_edit().as_str()),
    };
    format!("{}\n{rest}", msg::ribbons_left())
}

/// A gallery's item as JAWS says it: its name, "Button" as a word of its own, not a role, and
/// its place, "Heading 2 | Button | 2 of 17" [obs, 1064/s4-s7].
fn gallery_item(focus: &FocusEvent) -> Option<String> {
    let name = focus
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())?;
    let mut lines = vec![name.to_owned(), msg::role_button().to_string()];
    if let (Some(index), Some(count)) = (focus.item_index, focus.item_count) {
        lines.push(msg::position(index, count).render());
    }
    Some(lines.join("\n"))
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
                // The menus entered again from around them: "menu | | Upper Ribbon | Editing
                // group | Select sub menu" [obs, 1051/s13].
                let upper = (self.place == Place::Around)
                    .then(|| role_marker(msg::ribbon_upper().as_str()));
                if focus.role == Role::MenuButton && !self.menu {
                    // JAWS's empty part after "menu" says nothing.
                    lines.push(role_marker(msg::role_menu().as_str()));
                    lines.extend(upper);
                    lines.extend(group.as_ref().map(with_role));
                } else if self.place != *now && !combo {
                    lines.extend(upper);
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
            Place::Outside | Place::Around => return None,
        }
        Some(lines.join("\n"))
    }

    /// What a command around the menus says, from the place before: the menus left, the
    /// groupings entered, then the control with its hint.
    fn around(&self, focus: &FocusEvent) -> Option<Announcement> {
        let name = focus
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())?;
        let mut lines = Vec::new();
        let sub_menu = focus.role == Role::MenuButton;
        if sub_menu && !self.menu {
            lines.push(Line::new(vec![Part::role(msg::role_menu().as_str())]));
        }
        if matches!(self.place, Place::Tabs | Place::Lower(_)) || self.menu {
            lines.push(Line::of(PartKind::Message, msg::menus_left()));
        }
        // The search box, in its menu item, whose name the box's placeholder repeats.
        if focus.role == Role::Edit {
            if let Some(item) = focus
                .container_name
                .as_deref()
                .filter(|_| focus.container_role == Some(Role::MenuItem))
            {
                lines.push(Line::new(vec![
                    Part::new(PartKind::Container, item),
                    Part::state(msg::state_sub_menu().as_str()),
                ]));
            }
            lines.push(Line::new(vec![
                Part::new(PartKind::Name, name),
                Part::role(msg::role_edit().as_str()),
            ]));
            lines.extend(
                focus
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty() && *v != name)
                    .map(|v| Line::of(PartKind::Value, v)),
            );
            lines.push(Line::of(PartKind::Hint, msg::hint_type_in_text()));
            return Some(Announcement::new(lines));
        }
        let control = control_speech(focus, name)?;
        // The groupings entered: not the window's panes, nor the toolbar.
        for group in focus.groups.iter().filter(|g| !g.pane && !g.tool_bar) {
            if self.groups.iter().any(|before| before.name == group.name) {
                continue;
            }
            let mut parts = vec![Part::new(PartKind::Container, group.name.trim())];
            if group.tab_list {
                parts.push(Part::role(msg::role_tab().as_str()));
            }
            if group.disabled {
                parts.push(Part::state(msg::state_unavailable().as_str()));
            }
            lines.push(Line::new(parts));
        }
        lines.extend(control.into_iter().map(Line::text));
        lines.extend(around_hint(focus).map(|hint| Line::of(PartKind::Hint, hint)));
        Some(Announcement::new(lines))
    }
}

/// The hint of a command around the menus: a sub menu's arrows, a split button's even
/// unavailable, Enter for a link and the row of tabs' buttons, the spacebar for the others
/// that are available [obs, 1051/s4-s12].
fn around_hint(focus: &FocusEvent) -> Option<String> {
    let hint = match focus.role {
        Role::MenuButton => msg::hint_menu_items_up_down(),
        Role::SplitButton => msg::hint_split_button(),
        Role::Link { .. } => msg::hint_activate_enter(),
        Role::Button if focus.states.disabled => return None,
        Role::Button if focus.groups.iter().any(|g| g.tab_list) => msg::hint_activate_enter(),
        Role::Button => msg::hint_activate_spacebar(),
        _ => return None,
    };
    Some(hint.to_string())
}

/// A lower ribbon's command as JAWS says it, without its group: its name and role, its states,
/// its value for an edit combo, then its ScreenTip's title. `None` for a kind not recorded.
fn control_speech(focus: &FocusEvent, name: &str) -> Option<Vec<String>> {
    let mut lines = Vec::new();
    // A button that opens a menu says so first: "sub menu | Styles Button | More" [obs,
    // 1051/s17].
    if focus.opens_menu {
        lines.push(state_marker(msg::state_sub_menu().as_str()));
    }
    let mut line = match focus.role {
        Role::Button => format!("{name} {}", role_marker(msg::role_button().as_str())),
        Role::SplitButton => format!("{name} {}", role_marker(msg::role_split_button().as_str())),
        Role::MenuButton => format!("{name} {}", state_marker(msg::state_sub_menu().as_str())),
        Role::Link { .. } => format!("{name} {}", role_marker(msg::role_link().as_str())),
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

/// A command's ScreenTip title (its help text), unless it has the whole name in it, its dots
/// too, or the name has it: "Cut (Ctrl+X)", "Add a Blank Page", "Font... (Ctrl+D)" and
/// "Clipboard" (for "Office Clipboard...") say nothing more; "Paragraph Settings" (for
/// "Paragraph...") and "More" (for "Styles") are said [obs, 1043, 1045, 1046, 1048, 1051/s17,
/// s18].
fn screen_tip(focus: &FocusEvent, name: &str) -> Option<String> {
    let tip = focus
        .help_text
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())?;
    let (tip_words, name_words) = (tip.to_lowercase(), name.trim().to_lowercase());
    (!tip_words.contains(&name_words) && !name_words.contains(&tip_words)).then(|| tip.to_owned())
}

impl AppModule for WordRibbon {
    fn owns(&self, focus: &FocusEvent) -> bool {
        in_word(focus) && (focus.in_ribbon || (self.view_wanted && focus.role == Role::Document))
    }

    fn document_view(&mut self, view: Option<&str>) -> Option<String> {
        std::mem::take(&mut self.view_wanted).then(|| back_in_document(view))
    }

    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        // Out of the ribbon into Word's window, then its document.
        let left = self.place != Place::Outside || self.leaving;
        if left && in_word(focus) && !focus.in_ribbon {
            self.place = Place::Outside;
            self.menu = false;
            self.key_tips = false;
            match focus.role {
                Role::Window => {
                    self.leaving = true;
                    return FocusResponse::Silent;
                }
                // A key tip can have changed the view (Alt, W, F to Read Mode): the view first
                // [obs, 1042/s8, 1059/s3].
                Role::Document if cx.letter_key => {
                    self.leaving = false;
                    self.view_wanted = true;
                    cx.ask_view = true;
                    return FocusResponse::Silent;
                }
                Role::Document => {
                    self.leaving = false;
                    return FocusResponse::Speak(Announcement::from_text(&back_in_document(None)));
                }
                _ => {}
            }
        }
        self.leaving = false;
        if in_word(focus) && focus.in_ribbon && cx.letter_key {
            self.key_tips = true;
        }
        // The ribbon itself, taking the focus on the way: named on the way from its search box,
        // "Ribbon | Collapse the Ribbon Button" [obs, 1051/s12], and nothing between its menus'
        // commands [obs, 1045/s9] or on the way into its tabs.
        if in_word(focus) && focus.in_ribbon && focus.role == Role::TabPanel {
            let name = focus
                .name
                .as_deref()
                .map(str::trim)
                .filter(|n| !n.is_empty() && self.place == Place::Around);
            return match name {
                Some(name) => FocusResponse::SpeakWhole(Announcement::new(vec![Line::of(
                    PartKind::Name,
                    name,
                )])),
                None => FocusResponse::Silent,
            };
        }
        let on_menus = in_word(focus) && focus.in_ribbon && focus.ribbon_menu;
        // A gallery's item; the one its key tip opens the gallery on is not said [obs,
        // 1064/s3-s7]. The key tips still lead below the gallery.
        if on_menus && focus.role == Role::ListItem {
            if cx.letter_key {
                return FocusResponse::Silent;
            }
            return match gallery_item(focus) {
                Some(speech) => FocusResponse::Speak(Announcement::from_text(&speech)),
                None => FocusResponse::Pass,
            };
        }
        // A nameless focus on the menus: a gallery's window on its way to its item.
        if on_menus
            && focus
                .name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty())
        {
            return FocusResponse::Silent;
        }
        if in_word(focus) && focus.in_ribbon && !cx.letter_key && !cx.escape_key {
            self.key_tips = false;
        }
        // Around the menus: said whole, with the menus it leaves.
        if in_word(focus) && focus.in_ribbon && !focus.ribbon_menu {
            let speech = self.around(focus);
            self.menu = focus.role == Role::MenuButton;
            self.place = Place::Around;
            self.groups = focus.groups.clone();
            return match speech {
                Some(speech) => FocusResponse::SpeakWhole(speech),
                None => FocusResponse::Pass,
            };
        }
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
        // Escape while the key tips lead steps back through them: a gallery's button, then its
        // tab, unsaid [obs, 1064/s8, s9].
        if self.key_tips && cx.escape_key && now != Place::Outside {
            self.place = now;
            return FocusResponse::Silent;
        }
        // A command it does not word, or outside the ribbon's menus: the engine's.
        let speech = self.speech(focus, &now);
        if matches!(now, Place::Lower(_)) {
            self.menu = focus.role == Role::MenuButton;
        } else {
            self.menu = false;
        }
        self.place = now;
        self.groups.clear();
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
            escape_key: false,
            ask_view: false,
        };
        match ribbon.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) | FocusResponse::SpeakWhole(announcement) => {
                Some(announcement.render())
            }
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
        // A sub menu after other commands enters the menu again; the next group after it is
        // named alone [obs, 1048/s19, s23, s25].
        let effects = in_group(Role::MenuButton, "Text Effects and Typography", "Font");
        assert_eq!(
            said(&mut ribbon, &effects).as_deref(),
            Some(" \u{1d} menu \u{1e} \nFont  \u{1d} group \u{1e} \nText Effects and Typography  \u{1d}sub menu\u{1e} ")
        );
        let color = in_group(Role::SplitButton, "Font Color", "Font");
        assert_eq!(
            said(&mut ribbon, &color).as_deref(),
            Some("Font Color  \u{1d} split button \u{1e} ")
        );
        // A button that opens a menu: "sub menu" first, no menu entered [obs, 1051/s17].
        let mut styles = in_group(Role::Button, "Styles", "Font");
        styles.opens_menu = true;
        styles.help_text = Some("More".into());
        assert_eq!(
            said(&mut ribbon, &styles).as_deref(),
            Some(" \u{1d}sub menu\u{1e} \nStyles  \u{1d} Button \u{1e} \nMore")
        );
        let bullets = in_group(Role::SplitButton, "Bullets", "Paragraph");
        assert_eq!(
            said(&mut ribbon, &bullets).as_deref(),
            Some("Paragraph\nBullets  \u{1d} split button \u{1e} ")
        );
    }

    /// Shift+Tab from the Home tab past the Quick Access Toolbar, the File tab, the buttons and
    /// the search box beside the tabs, Collapse the Ribbon, and back into the lower ribbon, as
    /// JAWS reads them [obs, 1051/s4-s13].
    #[test]
    fn the_commands_around_the_ribbons_menus_are_read_as_jaws_reads_them() {
        let mut ribbon = WordRibbon::default();
        assert!(said(&mut ribbon, &word(Role::Tab, "Home")).is_some());
        let around = |role: Role, name: &str, groups: Vec<GroupEntry>| {
            let mut focus = word(role, name);
            focus.ribbon_menu = false;
            focus.item_index = None;
            focus.item_count = None;
            focus.groups = [GroupEntry::pane("MsoDockTop"), GroupEntry::pane("Ribbon")]
                .into_iter()
                .chain(groups)
                .collect();
            focus
        };
        let toolbar = || vec![GroupEntry::tool_bar("Quick Access Toolbar")];
        let customize = around(Role::MenuButton, "Customize Toolbar", toolbar());
        assert_eq!(
            said(&mut ribbon, &customize).as_deref(),
            Some(" \u{1d} menu \u{1e} \nLeaving menus\nCustomize Toolbar  \u{1d}sub menu\u{1e} \nTo move through items press up or down arrow.")
        );
        let mut repeat = around(Role::Button, "Can't Repeat", toolbar());
        repeat.states.disabled = true;
        repeat.help_text = Some("Can't Repeat (Ctrl+Y)".into());
        assert_eq!(
            said(&mut ribbon, &repeat).as_deref(),
            Some("Leaving menus\nCan't Repeat  \u{1d} Button \u{1e}  \u{1d}Unavailable\u{1e} ")
        );
        let mut undo_group = GroupEntry::group("Can't Undo");
        undo_group.disabled = true;
        let mut undo = around(
            Role::SplitButton,
            "Can't Undo",
            toolbar().into_iter().chain([undo_group]).collect(),
        );
        undo.states.disabled = true;
        assert_eq!(
            said(&mut ribbon, &undo).as_deref(),
            Some("Can't Undo \u{1d}Unavailable\u{1e} \nCan't Undo  \u{1d} split button \u{1e}  \u{1d}Unavailable\u{1e} \nTo move to an item, use the RIGHT ARROW, then use UP and DOWN ARROWS to select one.")
        );
        let mut save = around(Role::Button, "Save", toolbar());
        save.help_text = Some("Save (Ctrl+S)".into());
        assert_eq!(
            said(&mut ribbon, &save).as_deref(),
            Some("Save  \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
        let tabs = || vec![GroupEntry::tab_list("Ribbon Tabs")];
        let share = around(Role::Button, "Share", tabs());
        assert_eq!(
            said(&mut ribbon, &share).as_deref(),
            Some("Ribbon Tabs \u{1d} Tab \u{1e} \nShare  \u{1d} Button \u{1e} \nTo activate press Enter.")
        );
        let mut sign_in = around(Role::Link { href: None }, "Sign in", tabs());
        sign_in.help_text = Some("Get to your documents from anywhere.".into());
        assert_eq!(
            said(&mut ribbon, &sign_in).as_deref(),
            Some("Sign in  \u{1d} Link \u{1e} \nGet to your documents from anywhere.\nTo activate press Enter.")
        );
        let mut search = around(Role::Edit, "Tell me what you want to do...", Vec::new());
        search.value = Some("Tell me what you want to do...".into());
        search.container_role = Some(Role::MenuItem);
        search.container_name = Some("Tell me what you want to do...".into());
        assert_eq!(
            said(&mut ribbon, &search).as_deref(),
            Some("Tell me what you want to do... \u{1d}sub menu\u{1e} \nTell me what you want to do... \u{1d} Edit \u{1e} \nType in text.")
        );
        // The ribbon itself on the way from the search box: its name.
        let mut page = word(Role::TabPanel, "Ribbon");
        page.item_index = None;
        page.item_count = None;
        assert_eq!(said(&mut ribbon, &page).as_deref(), Some("Ribbon"));
        let mut collapse = around(Role::Button, "Collapse the Ribbon", Vec::new());
        collapse.states.checked = Some(false);
        collapse.help_text = Some("Collapse the Ribbon (Ctrl+F1)".into());
        assert_eq!(
            said(&mut ribbon, &collapse).as_deref(),
            Some("Collapse the Ribbon  \u{1d} Button \u{1e}  \u{1d}not checked\u{1e} \nTo activate press spacebar.")
        );
        let mut select = word(Role::MenuButton, "Select");
        select.groups = vec![
            GroupEntry::group("Lower Ribbon"),
            GroupEntry::tool_bar("Editing"),
        ];
        assert_eq!(
            said(&mut ribbon, &select).as_deref(),
            Some(" \u{1d} menu \u{1e} \n \u{1d} Upper Ribbon \u{1e} \nEditing  \u{1d} group \u{1e} \nSelect  \u{1d}sub menu\u{1e} ")
        );
        // The ribbon itself between two of its menus' commands says nothing.
        assert_eq!(said(&mut ribbon, &page), None);
    }

    /// Back in the document by a key tip, the view is asked for first: Read Mode is said with
    /// JAWS's warning in Okur's name, another view as "Edit" [obs, 1059/s3, 1042/s4].
    #[test]
    fn a_key_tip_back_into_the_document_says_read_mode() {
        let mut ribbon = WordRibbon::default();
        assert!(said(&mut ribbon, &word(Role::Tab, "View")).is_some());
        let mut document = FocusEvent::new(Role::Document);
        document.app = Some("WINWORD.EXE".into());
        let mut mode = AppMode::NativeControl;
        let (mut title, mut kind) = (None, None);
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: true,
            escape_key: false,
            ask_view: false,
        };
        assert!(matches!(
            ribbon.focus(&document, &mut cx),
            FocusResponse::Silent
        ));
        assert!(cx.ask_view);
        assert_eq!(
            ribbon.document_view(Some("Read Mode")).as_deref(),
            Some("Leaving ribbons\nreading layout View\nWARNING! Okur does not support this view. Press Okur Key+F1 to learn how to disable Reading Layout view. To exit Reading Layout view, press ESC.")
        );
        // Answered once.
        assert_eq!(ribbon.document_view(Some("Read Mode")), None);
    }

    /// The Styles gallery opened by its key tip: its first item and window unsaid, the arrows
    /// saying each item as a button with its place; Escape back to its button and its tab
    /// unsaid while the key tips lead [obs, 1064/s3-s9].
    #[test]
    fn a_gallery_opened_by_its_key_tip_is_read_as_jaws_reads_it() {
        let mut ribbon = WordRibbon::default();
        let mut mode = AppMode::NativeControl;
        let (mut title, mut kind) = (None, None);
        let mut by = |ribbon: &mut WordRibbon, focus: &FocusEvent, letter: bool, escape: bool| {
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: letter,
                escape_key: escape,
                ask_view: false,
            };
            match ribbon.focus(focus, &mut cx) {
                FocusResponse::Speak(announcement) | FocusResponse::SpeakWhole(announcement) => {
                    Some(announcement.render())
                }
                FocusResponse::Silent => Some(String::new()),
                _ => None,
            }
        };
        let home = word(Role::Tab, "Home");
        assert!(by(&mut ribbon, &home, false, false).is_some_and(|s| !s.is_empty()));
        // L: the gallery's window, then its first item.
        let window = word(Role::Generic, "");
        assert_eq!(by(&mut ribbon, &window, true, false).as_deref(), Some(""));
        let item = |name: &str, index: usize| {
            let mut item = word(Role::ListItem, name);
            item.item_index = Some(index);
            item.item_count = Some(17);
            item
        };
        assert_eq!(
            by(&mut ribbon, &item("Heading 1", 1), true, false).as_deref(),
            Some("")
        );
        assert_eq!(
            by(&mut ribbon, &item("Heading 2", 2), false, false).as_deref(),
            Some("Heading 2\nButton\n2 of 17")
        );
        // Escape: the gallery's button, then its tab, unsaid.
        let mut styles = word(Role::Button, "Styles");
        styles.opens_menu = true;
        styles.groups = vec![
            GroupEntry::group("Lower Ribbon"),
            GroupEntry::tool_bar("Styles"),
        ];
        assert_eq!(by(&mut ribbon, &styles, false, true).as_deref(), Some(""));
        assert_eq!(by(&mut ribbon, &home, false, true).as_deref(), Some(""));
        // An arrow ends the key tips' lead: the tab is said again.
        let insert = word(Role::Tab, "Insert");
        assert!(by(&mut ribbon, &insert, false, false).is_some_and(|s| !s.is_empty()));
    }
}
