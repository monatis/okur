//! Focus speech for native application controls (outside a browser document).
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

/// Win32 list-view controls (`SysListView32`, also as WinForms `WindowsForms10.SysListView32…`).
fn is_list_view(focus: &FocusEvent) -> bool {
    focus
        .container_class
        .as_deref()
        .is_some_and(|c| c.contains("SysListView32"))
}

fn named(name: &str) -> String {
    name.trim().to_owned()
}

/// The entry line for the focused control's container, or `None` when the container has
/// none (windows, panes, unnamed groups).
fn container_entry(focus: &FocusEvent) -> Option<String> {
    let name = named(focus.container_name.as_deref().unwrap_or_default());
    match focus.container_role.as_ref()? {
        Role::Group if !name.is_empty() => Some(name),
        Role::List if is_list_view(focus) => Some(format!("{name}{}", role_marker("List view"))),
        Role::List => Some(format!("{name}{}", role_marker("List box"))),
        Role::Tree => Some(format!("{name}{}", role_marker("Tree view"))),
        _ => None,
    }
}

fn position(focus: &FocusEvent) -> Option<String> {
    match (focus.item_index, focus.item_count) {
        (Some(i), Some(n)) => Some(format!("{i} of {n}")),
        _ => None,
    }
}

fn checked(focus: &FocusEvent) -> String {
    if focus.states.checked == Some(true) {
        state_marker("checked")
    } else {
        state_marker("not checked")
    }
}

/// The control's own lines (2–4 above).
/// `entering` is false when focus moved within the same container (arrow keys in a list),
/// where JAWS speaks the item without the container's usage hint [obs, 303/s1, s10–s11].
fn control_lines(focus: &FocusEvent, entering: bool) -> Vec<String> {
    let name = named(focus.name.as_deref().unwrap_or_default());
    let value = focus.value.as_deref().map(named).filter(|v| !v.is_empty());
    let mut lines = Vec::new();
    match focus.role {
        Role::Edit => {
            lines.push(format!("{name}{}", role_marker("Edit")));
            lines.extend(value);
            lines.push("Type in text.".to_owned());
        }
        Role::CheckBox => {
            lines.push(format!(
                "{name}{}{}",
                role_marker("check box"),
                checked(focus)
            ));
            lines.push(if focus.states.checked == Some(true) {
                "To clear checkmark press Spacebar.".to_owned()
            } else {
                "To check press Spacebar.".to_owned()
            });
        }
        Role::RadioButton => {
            lines.push(format!(
                "{name}{}{}",
                role_marker("radio button"),
                checked(focus)
            ));
            lines.extend(position(focus));
            lines.push("To change the selection press Up or Down Arrow.".to_owned());
        }
        Role::ComboBox => {
            lines.push(format!("{name}{}", role_marker("Combo box")));
            lines.extend(value);
            lines.extend(position(focus));
            lines.push("To change the selection use the Arrow keys.".to_owned());
        }
        Role::ListItem => {
            lines.push(name);
            lines.extend(focus.columns.iter().map(|c| named(c)));
            lines.extend(position(focus));
            if entering {
                lines.push(if is_list_view(focus) {
                    "To move to items use the Arrow keys.".to_owned()
                } else {
                    "To move to an item press the Arrow keys.".to_owned()
                });
            }
        }
        Role::TreeItem => {
            // Expanded items say "open" [obs, 303/s7]. Moving up to an already visited
            // parent drops the position (303/s6); that rule is not known yet.
            let state = match focus.states.expanded {
                Some(true) => state_marker("open"),
                Some(false) => state_marker("closed"),
                None => String::new(),
            };
            lines.push(format!("{name}{state}"));
            lines.extend(position(focus));
            if entering {
                lines.push("To move through or expand items use the Arrow keys.".to_owned());
            }
        }
        Role::Tab => {
            let selected = if focus.states.selected == Some(true) {
                state_marker("selected")
            } else {
                String::new()
            };
            lines.push(format!("{name}{}{selected}", role_marker("Tab")));
            lines.push("To switch pages, press Control+PageDown.".to_owned());
        }
        Role::MenuButton => {
            lines.push(format!("{name}{}", role_marker("button menu")));
            lines
                .push("Press space to activate the menu, then navigate with arrow keys".to_owned());
        }
        Role::Button => {
            lines.push(format!("{name}{}", role_marker("Button")));
            lines.push("To activate press spacebar.".to_owned());
        }
        _ => {
            lines.push(name);
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
    pub fn focus(&mut self, focus: &FocusEvent) -> Option<String> {
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
                    speech = format!("level {level}\n{speech}");
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
pub fn focus_speech(focus: &FocusEvent, previous: Option<&ContainerKey>) -> String {
    let mut lines = Vec::new();
    let container = ContainerKey::of(focus);
    let within = container.is_some() && container.as_ref() == previous;
    if !within {
        lines.extend(container_entry(focus));
    }
    lines.extend(
        control_lines(focus, !within)
            .into_iter()
            .filter(|l| !l.is_empty()),
    );
    lines.join("\n")
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
        let speech = if !self.active {
            self.active = true;
            Some(format!(
                "{}\n{name}\nTo navigate press Left or Right Arrow.",
                role_marker("Menu bar")
            ))
        } else if self.popup_pending {
            self.popup_pending = false;
            let parent = self.open.last().cloned().unwrap_or_default();
            let first = !self.popup_shown;
            self.popup_shown = true;
            Some(if first {
                format!(
                    "{}\n\n{parent} menu\n{name}\nTo move through items press up or down arrow.",
                    role_marker("menu")
                )
            } else {
                format!("{parent} menu\n{name}")
            })
        } else if switching {
            // The next menu's popup opens right away and introduces the item (302/s5).
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
        let item = self.closed.take()?;
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
        e.container_class = Some(class.to_owned());
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
            norm(&focus_speech(&name, None)),
            norm("Name:  \u{1d} Edit \u{1e} \nAda\nType in text.")
        );
        assert_eq!(
            norm(&focus_speech(&event(Role::Edit, "Notes:"), None)),
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
            norm(&focus_speech(&on, None)),
            norm("Subscribe to updates  \u{1d} check box \u{1e}   \u{1d}checked\u{1e} \nTo clear checkmark press Spacebar.")
        );
        let off = event(Role::CheckBox, "Remember me");
        assert_eq!(
            norm(&focus_speech(&off, None)),
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
            norm(&focus_speech(&pro, None)),
            norm("Plan \nPro  \u{1d} radio button \u{1e}   \u{1d}checked\u{1e} \n2 of 3\nTo change the selection press Up or Down Arrow.")
        );
        let inside = ContainerKey::of(&pro);
        assert!(!focus_speech(&pro, inside.as_ref()).starts_with("Plan"));
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
            norm(&focus_speech(&cherries, list.as_ref())),
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
            norm(&focus_speech(&notes, view.as_ref())),
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
            norm(&focus_speech(&fruit, None)),
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
            norm(&focus_speech(&file, None)),
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
            norm(&focus_speech(&docs, None)),
            norm(" \u{1d} Tree view \u{1e} \nDocuments  \u{1d}closed\u{1e} \n1 of 2\nTo move through or expand items use the Arrow keys.")
        );
    }

    #[test]
    fn combo_box_tab_and_button() {
        let mut country = at(event(Role::ComboBox, "Country:"), 2, 3);
        country.value = Some("Germany".to_owned());
        assert_eq!(
            norm(&focus_speech(&country, None)),
            norm("Country:  \u{1d} Combo box \u{1e} \nGermany\n2 of 3\nTo change the selection use the Arrow keys.")
        );
        let mut general = event(Role::Tab, "General");
        general.states.selected = Some(true);
        assert_eq!(
            norm(&focus_speech(&general, None)),
            norm("General  \u{1d} Tab \u{1e}   \u{1d}selected\u{1e} \nTo switch pages, press Control+PageDown.")
        );
        assert_eq!(
            norm(&focus_speech(&event(Role::Button, "OK"), None)),
            norm("OK  \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
    }
}
