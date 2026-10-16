//! Fous speech for native application controls (outside a browser document).
//!
//! Implements `docs/references/jaws-native-focus-spec.md`. JAWS speaks, in order:
//! 1. The container being entered, once, when focus moves into it from outside: a named
//!    group (`Plan`), a list box (`Fruit: ‹ List box ›`), a tree view (`‹ Tree view ›`), or
//!    a list view (`‹ List view ›`).
//! 2. The control: name, role and state (`Remember me ‹ check box › ‹not checked›`).
//! 3. Its value, list-view columns and position (`Germany`, `24 KB`, `2 of 3`).
//! 4. The usage hint for the control type (`To check press Spacebar.`).
//!
//! Lines are separated by `\n`, as in JAWS's speech history.

use crate::announcement::{Announcement, Line, Part};
use crate::aom::Role;
use crate::event::FocusEvent;
use crate::linearize::{role_marker, state_marker};

/// The container a focused control sits in, as far as entry announcements go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerKey {
    role: Role,
    name: Option<String>,
}

impl ContainerKey {
    pub fn of(focus: &FocusEvent) -> Option<Self> {
        Some(Self {
            role: focus.container_role.clone()?,
            name: focus.container_name.clone(),
        })
    }
}

fn named(name: &str) -> String {
    name.trim().to_owned()
}

fn text(text: impl Into<String>) -> Part {
    Part::Text(text.into())
}

fn role(role: &str) -> Part {
    Part::Role(role.to_owned())
}

fn state(state: &str) -> Part {
    Part::State(state.to_owned())
}

/// The entry line for the focused control's container, or `None` when the container has
/// none (windows, panes, unnamed groups).
fn container_entry(focus: &FocusEvent) -> Option<Line> {
    let name = named(focus.container_name.as_deref().unwrap_or_default());
    let kind = match focus.container_role.as_ref()? {
        Role::Group if !name.is_empty() => return Some(Line::text(name)),
        Role::List if focus.container_is_list_view => "List view",
        Role::List => "List box",
        Role::Tree => "Tree view",
        _ => return None,
    };
    Some(Line::new(vec![text(name), role(kind)]))
}

fn position(focus: &FocusEvent) -> Option<Line> {
    match (focus.item_index, focus.item_count) {
        (Some(i), Some(n)) => Some(Line::text(format!("{i} of {n}"))),
        _ => None,
    }
}

fn checked(focus: &FocusEvent) -> Part {
    if focus.states.checked == Some(true) {
        state("checked")
    } else {
        state("not checked")
    }
}

/// The control's own lines (2–4 above).
/// `entering` is false when focus moved within the same container (arrow keys in a list),
/// where JAWS speaks the item without the container's usage hint [obs, 303/s1, s10–s11].
fn control_lines(focus: &FocusEvent, entering: bool) -> Vec<Line> {
    let name = named(focus.name.as_deref().unwrap_or_default());
    let value = focus
        .value
        .as_deref()
        .map(named)
        .filter(|v| !v.is_empty())
        .map(Line::text);
    let mut lines = Vec::new();
    match focus.role {
        // A combo box's text field. JAWS 2025 on the Run dialog's Open field, observed on the
        // dev box (2026-10-15; research): "Open: edit combo | C: | 1 of 6 | To set the
        // value use the Arrow keys or type the value. | Alt+o".
        Role::Edit if focus.container_role == Some(Role::ComboBox) => {
            lines.push(Line::new(vec![text(name), role("edit combo")]));
            lines.extend(value);
            lines.extend(position(focus));
            lines.push(Line::text(EDIT_COMBO_HINT).on_arrival());
            lines.extend(focus.access_key.clone().map(|k| Line::text(k).on_arrival()));
        }
        // A native text document (Windows 11 Notepad's RichEdit, MSAA "editable text"): an
        // edit whose text is not read on arrival. JAWS 2025, dev box: "viewer-test.txt -
        // Notepad | Text editor edit | Contains text".
        Role::Document if !focus.in_document && focus.read_only == Some(false) => {
            lines.push(Line::new(vec![text(name), role("edit")]));
            if focus
                .text
                .as_ref()
                .is_some_and(|t| !t.text.trim().is_empty())
            {
                lines.push(Line::text(CONTAINS_TEXT).on_arrival());
            }
        }
        Role::Edit => {
            // [hyp] An unlabelled edit takes the window title as its name (306/s5).
            let name = if name.is_empty() {
                named(focus.window_title.as_deref().unwrap_or_default())
            } else {
                name
            };
            lines.push(Line::new(vec![text(name), role("Edit")]));
            lines.extend(value);
            lines.push(Line::text("Type in text."));
        }
        Role::CheckBox => {
            lines.push(Line::new(vec![
                text(name),
                role("check box"),
                checked(focus),
            ]));
            lines.push(Line::text(if focus.states.checked == Some(true) {
                "To clear checkmark press Spacebar."
            } else {
                "To check press Spacebar."
            }));
        }
        Role::RadioButton => {
            lines.push(Line::new(vec![
                text(name),
                role("radio button"),
                checked(focus),
            ]));
            lines.extend(position(focus));
            lines.push(Line::text(
                "To change the selection press Up or Down Arrow.",
            ));
        }
        Role::ComboBox => {
            lines.push(Line::new(vec![text(name), role("Combo box")]));
            lines.extend(value);
            lines.extend(position(focus));
            lines.push(Line::text("To change the selection use the Arrow keys."));
        }
        Role::ListItem => {
            lines.push(Line::text(name));
            lines.extend(focus.columns.iter().map(|c| Line::text(named(c))));
            lines.extend(position(focus));
            if entering {
                lines.push(Line::text(if focus.container_is_list_view {
                    "To move to items use the Arrow keys."
                } else {
                    "To move to an item press the Arrow keys."
                }));
            }
        }
        Role::TreeItem => {
            // Expanded items say "open" [obs, 303/s7]. Moving up to an already visited
            // parent drops the position (303/s6); that rule is not known yet.
            let mut parts = vec![text(name)];
            match focus.states.expanded {
                Some(true) => parts.push(state("open")),
                Some(false) => parts.push(state("closed")),
                None => {}
            }
            lines.push(Line::new(parts));
            lines.extend(position(focus));
            if entering {
                lines.push(Line::text(
                    "To move through or expand items use the Arrow keys.",
                ));
            }
        }
        Role::Tab => {
            let mut parts = vec![text(name), role("Tab")];
            if focus.states.selected == Some(true) {
                parts.push(state("selected"));
            }
            lines.push(Line::new(parts));
            lines.push(Line::text("To switch pages, press Control+PageDown."));
        }
        Role::MenuButton => {
            lines.push(Line::new(vec![text(name), role("button menu")]));
            lines.push(Line::text(
                "Press space to activate the menu, then navigate with arrow keys",
            ));
        }
        Role::Button => {
            lines.push(Line::new(vec![text(name), role("Button")]));
            lines.push(Line::text("To activate press spacebar."));
        }
        // A window taking focus is announced by its title alone (the window change).
        Role::Window => {}
        _ => {
            lines.push(Line::text(name));
            lines.extend(value);
        }
    }
    lines
}

/// What identifies one focus landing, to drop repeats: Windows focuses a list, tree or tab
/// control and then its item within milliseconds, and UIA reports the item both times. JAWS
/// speaks once [obs, 301/s6–s9].
#[derive(Debug, Clone, PartialEq)]
struct FocusKey {
    role: Role,
    name: Option<String>,
    item_index: Option<usize>,
    container: Option<ContainerKey>,
}

impl FocusKey {
    fn of(focus: &FocusEvent) -> Self {
        Self {
            role: focus.role.clone(),
            name: focus.name.clone(),
            item_index: focus.item_index,
            container: ContainerKey::of(focus),
        }
    }
}

/// Focus speech across a sequence of native focus events.
#[derive(Debug, Default)]
pub struct NativeFocus {
    container: Option<ContainerKey>,
    last: Option<FocusKey>,
    /// The tree level last announced in the current tree.
    tree_level: Option<usize>,
}

impl NativeFocus {
    /// Forgets the context, as when another window becomes active.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Speech for a focus event, or `None` for a repeat of the previous one.
    pub fn focus(&mut self, focus: &FocusEvent) -> Option<Announcement> {
        let key = FocusKey::of(focus);
        if self.last.as_ref() == Some(&key) {
            return None;
        }
        let previous = self.container.take();
        let within = key.container.is_some() && key.container == previous;
        let mut speech = focus_speech(focus, previous.as_ref());
        if focus.role == Role::TreeItem {
            if !within {
                self.tree_level = None;
            } else if let Some(level) = focus.tree_level {
                // Arrow keys in a tree announce a level different from the last one
                // announced [obs, 303/s5, s8]; Tab into the tree does not (303/s4).
                if self.tree_level != Some(level) {
                    speech.lines.insert(0, Line::text(format!("level {level}")));
                }
                self.tree_level = Some(level);
            }
        }
        self.container = key.container.clone();
        self.last = Some(key);
        Some(speech)
    }
}

/// Speech for focus landing on a native control. `previous` is the container of the
/// previously focused control; the container is announced only when it differs.
pub fn focus_speech(focus: &FocusEvent, previous: Option<&ContainerKey>) -> Announcement {
    let mut lines = Vec::new();
    let container = ContainerKey::of(focus);
    let within = container.is_some() && container.as_ref() == previous;
    if !within {
        lines.extend(container_entry(focus));
    }
    lines.extend(control_lines(focus, !within));
    Announcement::new(lines)
}

/// The focused control's own line (name, role and state, without its hint), as JAWS reads
/// it again when the Virtual Viewer closes [obs, 311/s12: "Save... Button"].
/// Say Window (Insert+B) for a dialog box: its title and "dialog", then each control in
/// reading order as its name, a lowercase type word, its state and its value. JAWS 2025 on
/// the Run dialog, observed on the dev box (research): "Run | dialog | Type the name of a
/// program, … | Open: | Open: | combo box | collapsed | C: | Open: | edit | C: | Open |
/// button | OK | button | Cancel | button | Browse... | button".
pub fn say_window(title: &str, items: &[crate::event::WindowItem]) -> String {
    let mut lines = vec![title.to_owned(), "dialog".to_owned()];
    let checked = |c: Option<bool>| {
        if c == Some(true) {
            "checked"
        } else {
            "not checked"
        }
    };
    for item in items {
        let name = item.name.trim().to_owned();
        let mut parts = vec![name];
        match item.role {
            Role::ComboBox => {
                parts.push("combo box".to_owned());
                parts.push(
                    if item.expanded == Some(true) {
                        "expanded"
                    } else {
                        "collapsed"
                    }
                    .to_owned(),
                );
            }
            Role::Edit => parts.push("edit".to_owned()),
            Role::Button => parts.push("button".to_owned()),
            Role::CheckBox => {
                parts.push("check box".to_owned());
                parts.push(checked(item.checked).to_owned());
            }
            Role::RadioButton => {
                parts.push("radio button".to_owned());
                parts.push(checked(item.checked).to_owned());
            }
            Role::Link { .. } => parts.push("link".to_owned()),
            _ => {}
        }
        parts.extend(item.value.clone());
        lines.extend(parts.into_iter().filter(|p| !p.is_empty()));
    }
    lines.join("\n")
}

/// The hint for a combo box's text field.
const EDIT_COMBO_HINT: &str = "To set the value use the Arrow keys or type the value.";
/// Said for a text document with text when focus arrives, in place of its text.
const CONTAINS_TEXT: &str = "Contains text";

/// Say Current Focus (Insert+Tab): the focused control's lines again, without what JAWS
/// says only as focus arrives (a text field's hint and access key, "Contains text"); a
/// button keeps its hint. JAWS 2025 on the dev box (research): "Open: edit combo | C: |
/// 1 of 6", "OK Button | To activate press spacebar.", "Text editor edit".
pub fn say_focus(focus: &FocusEvent) -> String {
    control_lines(focus, false)
        .iter()
        .filter(|l| !l.on_arrival)
        .map(Line::render)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn focus_line(focus: &FocusEvent) -> String {
    control_lines(focus, false)
        .iter()
        .map(Line::render)
        .find(|l| !l.is_empty())
        .unwrap_or_default()
}

/// The word JAWS uses for a control's type, as in the Virtual Viewer's "Save... Button".
pub fn role_word(role: &Role) -> Option<&'static str> {
    Some(match role {
        Role::Button => "Button",
        Role::MenuButton => "button menu",
        Role::Edit => "Edit",
        Role::CheckBox => "check box",
        Role::RadioButton => "radio button",
        Role::ComboBox => "Combo box",
        Role::Tab => "Tab",
        _ => return None,
    })
}

/// Lines joined as they are, empty ones included.
fn joined(lines: Vec<Line>) -> String {
    lines
        .iter()
        .map(Line::render)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Speech for a change of the focused native control, given its state before and after.
/// `None` when nothing JAWS speaks changed (Windows fires several events per change).
pub fn change_speech(before: &FocusEvent, after: &FocusEvent) -> Option<String> {
    let name = named(after.name.as_deref().unwrap_or_default());
    match after.role {
        Role::CheckBox if before.states.checked != after.states.checked => {
            Some(checked(after).render().trim().to_owned())
        }
        // Arrow keys in a radio group uncheck the old radio before focus moves to the new
        // one; JAWS speaks only the radio that becomes checked [obs, 304/s6–s7].
        Role::RadioButton
            if before.states.checked != after.states.checked
                && after.states.checked == Some(true) =>
        {
            Some(checked(after).render().trim().to_owned())
        }
        Role::TreeItem if before.states.expanded != after.states.expanded => {
            // [obs, 303/s7, 304/s16–s17]: "Documents | ‹open› | 2 items",
            // "Documents | ‹closed› | 1 of 2".
            let mut lines = vec![Line::text(name)];
            match after.states.expanded {
                Some(true) => {
                    lines.push(Line::new(vec![state("open")]));
                    lines.extend(after.child_count.map(|n| Line::text(format!("{n} items"))));
                }
                _ => {
                    lines.push(Line::new(vec![state("closed")]));
                    lines.extend(position(after));
                }
            }
            Some(joined(lines))
        }
        // A combo box reports no expanded state through UIA while collapsed.
        Role::ComboBox
            if before.states.expanded.unwrap_or(false)
                != after.states.expanded.unwrap_or(false) =>
        {
            if after.states.expanded == Some(true) {
                // Alt+Down [obs, 304/s11]: "Open List Box | Germany | 2 of 3".
                let mut lines = vec![Line::text("Open List Box")];
                lines.extend(after.value.iter().map(|v| Line::text(named(v))));
                lines.extend(position(after));
                Some(joined(lines))
            } else {
                // Closing it re-announces the combo box as on focus [obs, 304/s13].
                Some(focus_speech(after, None).render())
            }
        }
        Role::ComboBox if before.value != after.value => {
            let mut lines: Vec<Line> = after.value.iter().map(|v| Line::text(named(v))).collect();
            lines.extend(position(after));
            Some(joined(lines))
        }
        _ => None,
    }
}

/// Menu mode as JAWS announces it [obs, 302]. Focus inside menus arrives as focus events on
/// menu items; popups opening and closing arrive as [`crate::event::MenuEvent`]s.
#[derive(Debug, Default)]
pub struct Menus {
    active: bool,
    /// Items whose popups are open, outermost first.
    open: Vec<String>,
    /// A popup just opened and its first item is not announced yet.
    popup_pending: bool,
    /// The first popup of a menu session is introduced with its role and a hint; later
    /// ones only with their name (302/s2 versus s5).
    popup_shown: bool,
    current: Option<String>,
    /// The item whose popup just closed. If focus does not move on (switching menus with
    /// Left/Right), JAWS announces the item once speech goes idle (302/s7).
    closed: Option<String>,
}

impl Menus {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Focus landed on a menu item.
    pub fn focus(&mut self, name: &str) -> Option<String> {
        let name = name.trim();
        let switching = self.closed.take().is_some() && self.open.is_empty();
        let speech = if !self.active && self.popup_pending {
            // A page's menu opened before any item had the focus (Google Docs' Alt+Shift+A):
            // the item is its popup's first.
            self.active = true;
            self.popup_pending = false;
            self.popup_shown = true;
            Some(format!(
                "{}\n\n{name}\nTo move through items press up or down arrow.",
                role_marker("menu")
            ))
        } else if !self.active {
            self.active = true;
            Some(format!(
                "{}\n{name}\nTo navigate press Left or Right Arrow.",
                role_marker("Menu bar")
            ))
        } else if self.current.as_deref() == Some(name) {
            // The same item reported again (a browser reports a page's menu item twice).
            None
        } else if self.popup_pending {
            self.popup_pending = false;
            let parent = self.open.last().cloned().unwrap_or_default();
            let first = !self.popup_shown;
            self.popup_shown = true;
            // A page's menu can open before any item has the focus (Google Docs' Alt+Shift+A
            // opens its Accessibility menu): there is no item to name it by.
            let menu = if parent.is_empty() {
                String::new()
            } else {
                format!("{parent} menu\n")
            };
            Some(if first {
                format!(
                    "{}\n\n{menu}{name}\nTo move through items press up or down arrow.",
                    role_marker("menu")
                )
            } else {
                format!("{menu}{name}")
            })
        } else if switching {
            // The next menu's popup opens right away and introduces the item (302/s5). When
            // none opens (a page's menu closed with Escape, the focus back on its bar item),
            // the item is said once the events go quiet, as a native bar item is (302/s7).
            self.closed = Some(name.to_owned());
            None
        } else {
            Some(name.to_owned())
        };
        self.current = Some(name.to_owned());
        speech
    }

    pub fn popup_start(&mut self) {
        self.open.push(self.current.clone().unwrap_or_default());
        self.popup_pending = true;
        self.closed = None;
    }

    pub fn popup_end(&mut self) {
        self.closed = self.open.pop();
        self.popup_pending = false;
    }

    /// Ends menu mode; true when menus were active ("Leaving menus").
    pub fn end(&mut self) -> bool {
        let was_active = self.active;
        *self = Self::default();
        was_active
    }

    /// Speech that waits for the event stream to go quiet.
    pub fn idle(&mut self) -> Option<String> {
        let item = self.closed.take().filter(|item| !item.is_empty())?;
        let sub_menu = state_marker("sub menu");
        Some(format!("{item}\n{sub_menu}\n{item} {sub_menu}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::NodeStates;

    /// 302-desktop-menu-bar, as one session: Alt, Down ×3, Right, Down, Escape, Escape.
    #[test]
    fn menu_session() {
        let mut m = Menus::default();
        let s1 = m.focus("File");
        assert_eq!(
            s1.as_deref().map(norm),
            Some(norm(
                " \u{1d} Menu bar \u{1e} \nFile \nTo navigate press Left or Right Arrow."
            ))
        );
        m.popup_start();
        assert_eq!(
            m.focus("New").as_deref().map(norm),
            Some(norm("  \u{1d} menu \u{1e} \n  \nFile menu\nNew \nTo move through items press up or down arrow."))
        );
        assert_eq!(m.focus("Open...").as_deref(), Some("Open..."));
        assert_eq!(m.focus("Exit").as_deref(), Some("Exit"));
        // Right: the File popup closes, Edit gets focus silently, its popup opens.
        m.popup_end();
        assert_eq!(m.focus("Edit"), None);
        m.popup_start();
        assert_eq!(m.focus("Copy").as_deref(), Some("Edit menu\nCopy"));
        assert_eq!(m.idle(), None);
        assert_eq!(m.focus("Paste").as_deref(), Some("Paste"));
        // Escape closes the popup; nothing follows, so the bar item is announced on idle.
        m.popup_end();
        assert_eq!(
            m.idle().as_deref().map(norm),
            Some(norm(
                "Edit\n \u{1d}sub menu\u{1e} \nEdit  \u{1d}sub menu\u{1e}"
            ))
        );
        assert!(m.end());
        assert!(!m.is_active());
    }

    /// A page's menu opened from its editor (the web menus fixture's Alt+Shift+F; Google
    /// Docs' Alt+Shift+letter): the popup opens before any item has the focus, and Escape
    /// moves the focus back to the bar item, which is said once the events go quiet.
    #[test]
    fn a_page_menu_opened_from_its_editor() {
        let mut m = Menus::default();
        m.popup_start();
        assert_eq!(
            m.focus("New").as_deref().map(norm),
            Some(norm(
                " \u{1d} menu \u{1e} \n\nNew\nTo move through items press up or down arrow."
            ))
        );
        assert_eq!(m.focus("Open").as_deref(), Some("Open"));
        m.popup_end();
        assert_eq!(m.focus("File"), None);
        assert_eq!(
            m.idle().as_deref().map(norm),
            Some(norm(
                "File\n \u{1d}sub menu\u{1e} \nFile  \u{1d}sub menu\u{1e}"
            ))
        );
        // A popup that closes with no item to name says nothing once quiet.
        let mut m = Menus::default();
        m.popup_start();
        m.popup_end();
        assert_eq!(m.idle(), None);
    }

    /// Collapses whitespace, as the parity normalizer does.
    fn norm(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn event(role: Role, name: &str) -> FocusEvent {
        FocusEvent::new(role).with_name(name)
    }

    fn in_container(mut e: FocusEvent, role: Role, name: &str, class: &str) -> FocusEvent {
        e.container_role = Some(role);
        e.container_name = Some(name.to_owned());
        // As the platform reports a Win32 list view (also WinForms').
        e.container_is_list_view = class.contains("SysListView32");
        e.platform.container_class = Some(class.to_owned());
        e
    }

    fn at(mut e: FocusEvent, i: usize, n: usize) -> FocusEvent {
        e.item_index = Some(i);
        e.item_count = Some(n);
        e
    }

    // Expected strings: 301-desktop-tab-order evidence, all 5 trials identical.

    #[test]
    fn edit_with_and_without_value() {
        let mut name = event(Role::Edit, "Name:");
        name.value = Some("Ada".to_owned());
        assert_eq!(
            norm(&focus_speech(&name, None).render()),
            norm("Name:  \u{1d} Edit \u{1e} \nAda\nType in text.")
        );
        assert_eq!(
            norm(&focus_speech(&event(Role::Edit, "Notes:"), None).render()),
            norm("Notes:  \u{1d} Edit \u{1e} \nType in text.")
        );
    }

    #[test]
    fn check_box_states_and_hints() {
        let mut on = event(Role::CheckBox, "Subscribe to updates");
        on.states = NodeStates {
            checked: Some(true),
            ..NodeStates::default()
        };
        assert_eq!(
            norm(&focus_speech(&on, None).render()),
            norm("Subscribe to updates  \u{1d} check box \u{1e}   \u{1d}checked\u{1e} \nTo clear checkmark press Spacebar.")
        );
        let off = event(Role::CheckBox, "Remember me");
        assert_eq!(
            norm(&focus_speech(&off, None).render()),
            norm("Remember me  \u{1d} check box \u{1e}   \u{1d}not checked\u{1e} \nTo check press Spacebar.")
        );
    }

    #[test]
    fn radio_button_announces_its_group_on_entry() {
        let mut pro = at(
            in_container(
                event(Role::RadioButton, "Pro"),
                Role::Group,
                "Plan",
                "WindowsForms10.Window.8.app.0.1",
            ),
            2,
            3,
        );
        pro.states.checked = Some(true);
        assert_eq!(
            norm(&focus_speech(&pro, None).render()),
            norm("Plan \nPro  \u{1d} radio button \u{1e}   \u{1d}checked\u{1e} \n2 of 3\nTo change the selection press Up or Down Arrow.")
        );
        let inside = ContainerKey::of(&pro);
        assert!(!focus_speech(&pro, inside.as_ref())
            .render()
            .starts_with("Plan"));
    }

    /// Arrow keys within a list speak the item without the hint [obs, 303/s1, s10].
    #[test]
    fn moving_within_a_list_drops_entry_and_hint() {
        let cherries = at(
            in_container(
                event(Role::ListItem, "Cherries"),
                Role::List,
                "Fruit:",
                "WindowsForms10.LISTBOX.app.0.1",
            ),
            3,
            3,
        );
        let list = ContainerKey::of(&cherries);
        assert_eq!(
            norm(&focus_speech(&cherries, list.as_ref()).render()),
            norm("Cherries \n3 of 3")
        );
        let mut notes = at(
            in_container(
                event(Role::ListItem, "notes.txt"),
                Role::List,
                "",
                "WindowsForms10.SysListView32.app.0.1",
            ),
            2,
            3,
        );
        notes.columns = vec!["2 KB".to_owned()];
        let view = ContainerKey::of(&notes);
        assert_eq!(
            norm(&focus_speech(&notes, view.as_ref()).render()),
            norm("notes.txt\n2 KB \n2 of 3")
        );
    }

    #[test]
    fn list_box_list_view_and_tree_view() {
        let fruit = at(
            in_container(
                event(Role::ListItem, "Bananas"),
                Role::List,
                "Fruit:",
                "WindowsForms10.LISTBOX.app.0.1",
            ),
            2,
            3,
        );
        assert_eq!(
            norm(&focus_speech(&fruit, None).render()),
            norm("Fruit:  \u{1d} List box \u{1e} \nBananas \n2 of 3\nTo move to an item press the Arrow keys.")
        );

        let mut file = at(
            in_container(
                event(Role::ListItem, "budget.xlsx"),
                Role::List,
                "",
                "WindowsForms10.SysListView32.app.0.1",
            ),
            1,
            3,
        );
        file.columns = vec!["24 KB".to_owned()];
        assert_eq!(
            norm(&focus_speech(&file, None).render()),
            norm(" \u{1d} List view \u{1e} \nbudget.xlsx\n24 KB \n1 of 3\nTo move to items use the Arrow keys.")
        );

        let mut docs = at(
            in_container(
                event(Role::TreeItem, "Documents"),
                Role::Tree,
                "",
                "WindowsForms10.SysTreeView32.app.0.1",
            ),
            1,
            2,
        );
        docs.states.expanded = Some(false);
        assert_eq!(
            norm(&focus_speech(&docs, None).render()),
            norm(" \u{1d} Tree view \u{1e} \nDocuments  \u{1d}closed\u{1e} \n1 of 2\nTo move through or expand items use the Arrow keys.")
        );
    }

    #[test]
    fn combo_box_tab_and_button() {
        let mut country = at(event(Role::ComboBox, "Country:"), 2, 3);
        country.value = Some("Germany".to_owned());
        assert_eq!(
            norm(&focus_speech(&country, None).render()),
            norm("Country:  \u{1d} Combo box \u{1e} \nGermany\n2 of 3\nTo change the selection use the Arrow keys.")
        );
        let mut general = event(Role::Tab, "General");
        general.states.selected = Some(true);
        assert_eq!(
            norm(&focus_speech(&general, None).render()),
            norm("General  \u{1d} Tab \u{1e}   \u{1d}selected\u{1e} \nTo switch pages, press Control+PageDown.")
        );
        assert_eq!(
            norm(&focus_speech(&event(Role::Button, "OK"), None).render()),
            norm("OK  \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
    }

    /// The Run dialog's Open field (JAWS 2025 on Windows 11, observed on the dev box).
    #[test]
    fn a_combo_box_field_is_an_edit_combo() {
        let mut field = event(Role::Edit, "Open:");
        field.value = Some("C:".to_owned());
        field.container_role = Some(Role::ComboBox);
        field.item_index = Some(1);
        field.item_count = Some(6);
        field.access_key = Some("Alt+o".to_owned());
        assert_eq!(
            norm(&focus_speech(&field, None).render()),
            norm(
                "Open:  \u{1d} edit combo \u{1e} \nC:\n1 of 6\n\
                 To set the value use the Arrow keys or type the value.\nAlt+o"
            )
        );
    }

    /// Insert+Tab leaves out what JAWS says only as focus arrives.
    #[test]
    fn say_focus_repeats_the_control_without_arrival_hints() {
        let mut field = event(Role::Edit, "Open:");
        field.value = Some("C:".to_owned());
        field.container_role = Some(Role::ComboBox);
        field.item_index = Some(1);
        field.item_count = Some(6);
        field.access_key = Some("Alt+o".to_owned());
        assert_eq!(
            norm(&say_focus(&field)),
            norm("Open:  \u{1d} edit combo \u{1e} \nC:\n1 of 6")
        );
        assert_eq!(
            norm(&say_focus(&event(Role::Button, "OK"))),
            norm("OK  \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
        let mut notepad = event(Role::Document, "Text editor");
        notepad.read_only = Some(false);
        notepad.text = Some(crate::event::TextContext {
            text: "fn main() {}".to_owned(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
        });
        assert_eq!(
            norm(&focus_speech(&notepad, None).render()),
            norm("Text editor  \u{1d} edit \u{1e} \nContains text")
        );
        assert_eq!(
            norm(&say_focus(&notepad)),
            norm("Text editor  \u{1d} edit \u{1e} ")
        );
    }

    /// The Run dialog's controls, as JAWS's Say Window read them on the dev box.
    #[test]
    fn say_window_reads_a_dialog_in_order() {
        use crate::event::WindowItem;
        let item = |role: Role, name: &str, value: Option<&str>| WindowItem {
            role,
            name: name.to_owned(),
            value: value.map(str::to_owned),
            expanded: None,
            checked: None,
        };
        let items = [
            item(Role::Text, "Type the name of a program.", None),
            item(Role::Text, "Open:", None),
            WindowItem {
                expanded: Some(false),
                ..item(Role::ComboBox, "Open:", Some("C:"))
            },
            item(Role::Edit, "Open:", Some("C:")),
            item(Role::Button, "Open", None),
            item(Role::Button, "OK", None),
        ];
        assert_eq!(
            say_window("Run", &items),
            "Run\ndialog\nType the name of a program.\nOpen:\nOpen:\ncombo box\ncollapsed\nC:\n\
             Open:\nedit\nC:\nOpen\nbutton\nOK\nbutton"
        );
    }
}
