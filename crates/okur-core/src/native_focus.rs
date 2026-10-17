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
            // Its shortcut (UIA AcceleratorKey): Chrome's address bar, "… Type in text. |
            // Ctrl+L" (JAWS on the dev box, 2026-10-17). Buttons' shortcuts are not said (File
            // Explorer's Back, Alt+Left).
            lines.extend(
                focus
                    .accelerator_key
                    .as_deref()
                    .map(str::trim)
                    .filter(|k| !k.is_empty())
                    .map(|k| Line::text(k).on_arrival()),
            );
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
            // The position the application reports: File Explorer's tab, "docs Tab | selected |
            // 1 of 1" (dev box, 2026-10-17); a WinForms tab reports none (303/s12).
            lines.extend(position(focus));
            lines.push(Line::text("To switch pages, press Control+PageDown."));
        }
        // File Explorer (JAWS on the dev box, 2026-10-17): a column header, "Name split button";
        // the address bar's breadcrumb, "This PC split button | 1 of 6", the position it
        // reports. No hint.
        Role::SplitButton => {
            lines.push(Line::new(vec![text(name), role("split button")]));
            lines.extend(position(focus));
        }
        Role::MenuButton => {
            lines.push(Line::new(vec![text(name), role("button menu")]));
            lines.push(Line::text(
                "Press space to activate the menu, then navigate with arrow keys",
            ));
        }
        // A button says the position its application reports (Windows Settings' breadcrumb:
        // "Home Button | 1 of 1 | To activate press spacebar."; JAWS on the dev box,
        // 2026-10-16); the platform counts none for buttons (a dialog's say none, 306).
        Role::Button => {
            // An expander's button says its state: Windows Settings' "Show more settings Button
            // collapsed" (JAWS on the dev box, 2026-10-16).
            let mut parts = vec![text(name), role("Button")];
            match focus.states.expanded {
                Some(true) => parts.push(state("expanded")),
                Some(false) => parts.push(state("collapsed")),
                None => {}
            }
            lines.push(Line::new(parts));
            // A WinUI toggle switch is a button with a toggle state, said as its switch
            // reads: Windows Settings' "Night light Button | Off" (JAWS, dev box 2026-10-16).
            if focus.platform.element_class.as_deref() == Some("ToggleSwitch") {
                match focus.states.checked {
                    Some(true) => lines.push(Line::text("On")),
                    Some(false) => lines.push(Line::text("Off")),
                    None => {}
                }
            }
            lines.extend(position(focus));
            lines.push(Line::text("To activate press spacebar."));
            // Its access key, as a key sequence: Calculator's "Open history flyout Button |
            // To activate press spacebar. | Alt followed by I" (UIA AccessKey "Alt, I"; JAWS
            // on the dev box, 2026-10-17).
            lines.extend(
                focus
                    .access_key
                    .as_deref()
                    .filter(|k| k.contains(", "))
                    .map(|k| Line::text(k.replace(", ", " followed by ")).on_arrival()),
            );
        }
        // Windows Settings' Brightness (JAWS, dev box 2026-10-16): "Brightness left right
        // slider | To increase or decrease use the Arrow keys."
        Role::Slider => {
            lines.push(Line::new(vec![text(name), role("left right slider")]));
            lines.extend(value);
            lines.push(Line::text("To increase or decrease use the Arrow keys."));
        }
        // Windows Settings' "Rename" (JAWS, dev box 2026-10-16): "Rename Link | To activate
        // press Enter."
        Role::Link { .. } => {
            lines.push(Line::new(vec![text(name), role("Link")]));
            lines.push(Line::text("To activate press Enter."));
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
    item_count: Option<usize>,
    container: Option<ContainerKey>,
    /// Tells apart controls of one role and name side by side: Windows Settings' "Night
    /// light" toggle switch and the "Night light" button next to it.
    element_class: Option<String>,
}

impl FocusKey {
    fn of(focus: &FocusEvent) -> Self {
        Self {
            role: focus.role.clone(),
            name: focus.name.clone(),
            item_index: focus.item_index,
            item_count: focus.item_count,
            container: ContainerKey::of(focus),
            element_class: focus.platform.element_class.clone(),
        }
    }
}

/// Focus speech across a sequence of native focus events.
#[derive(Debug, Default)]
pub struct NativeFocus {
    container: Option<ContainerKey>,
    last: Option<FocusKey>,
    /// The groups and lists the last focus was in (`FocusEvent::groups`).
    groups: Vec<crate::event::GroupEntry>,
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
        // Within one list also where its items have no container to tell it by: Windows
        // Settings' navigation items, in an unnamed group, said with their position and no
        // usage hint after the first (JAWS on the dev box, 2026-10-16).
        let between_items = key.container.is_none()
            && previous.is_none()
            && focus.role == Role::ListItem
            && self.last.as_ref().is_some_and(|last| {
                last.role == Role::ListItem
                    && last.container.is_none()
                    && last.item_count == focus.item_count
            });
        let within = (key.container.is_some() && key.container == previous) || between_items;
        // The groups entered: those after the ones the last focus shared with this one.
        let shared = self
            .groups
            .iter()
            .zip(&focus.groups)
            .take_while(|(a, b)| a == b)
            .count();
        let entered = focus.groups.get(shared..).unwrap_or_default();
        let mut speech = focus_speech_within(focus, within, entered);
        self.groups = focus.groups.clone();
        if focus.role == Role::TreeItem {
            if !within {
                // Tab into the tree names no level (303/s4), and the arrows go on from the
                // item's: the first Down at that level says the item alone (303/s5, trial 1:
                // "Pictures"; File Explorer's navigation pane, "Gallery", dev box 2026-10-17).
                self.tree_level = focus.tree_level;
            } else if let Some(level) = focus.tree_level {
                // Arrow keys in a tree announce a level different from the last one
                // announced, and the item's position with it [obs, 303/s8: "level 1 |
                // Reports | 1 of 2"]; within one level, no position (303/s6: "Documents
                // closed"; File Explorer: "OneDrive Closed").
                if self.tree_level != Some(level) {
                    speech.lines.insert(0, Line::text(format!("level {level}")));
                } else if let Some(at) = position(focus).map(|p| p.render()) {
                    speech.lines.retain(|line| line.render() != at);
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
    focus_speech_entering(focus, previous, &focus.groups)
}

/// [`focus_speech`], naming the groups and lists the focus `entered` (of
/// `FocusEvent::groups`), outermost first, in place of its group or list container: Windows
/// Settings' "Brightness & color | Night light | Night light Button | Off", "Advanced |
/// Troubleshoot common sound problems | Troubleshoot common sound problems List box | Output
/// devices | 1 of 2" (JAWS on the dev box, 2026-10-16).
fn focus_speech_entering(
    focus: &FocusEvent,
    previous: Option<&ContainerKey>,
    entered: &[crate::event::GroupEntry],
) -> Announcement {
    let container = ContainerKey::of(focus);
    let within = container.is_some() && container.as_ref() == previous;
    focus_speech_within(focus, within, entered)
}

/// [`focus_speech_entering`], with whether the focus moved `within` its container known.
fn focus_speech_within(
    focus: &FocusEvent,
    within: bool,
    entered: &[crate::event::GroupEntry],
) -> Announcement {
    let mut lines = Vec::new();
    // A group or list container the platform gave the groups of is named through them.
    // (Any container but a tree: Settings' InfoBar link has the page's landmark around it.)
    if !focus.groups.is_empty() && focus.container_role != Some(Role::Tree) {
        let last = entered.len().saturating_sub(1);
        for (i, entry) in entered.iter().enumerate() {
            let name = named(&entry.name);
            lines.push(if entry.dialog {
                Line::new(vec![text(name), role("dialog")])
            } else if entry.list {
                let kind = if i == last && focus.container_is_list_view {
                    "List view"
                } else {
                    "List box"
                };
                Line::new(vec![text(name), role(kind)])
            } else {
                Line::text(name)
            });
        }
    } else if !within {
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
        Role::SplitButton => "split button",
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
    /// A page's menus (ARIA), whose items say where they are themselves ([`Self::page_focus`]):
    /// popups opening and closing say nothing.
    page: bool,
}

impl Menus {
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// The menu item that has the focus, while menus are active.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref().filter(|_| self.active)
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

    /// Focus landed on an item of a modern (WinUI) menu, which reports its position: File
    /// Explorer's context menu, JAWS 2025 on the dev box (2026-10-17): "Cut\tCtrl+X | 1 of 4
    /// | To move through items press up or down arrow. | Alt followed by T", then
    /// "Copy\tCtrl+C | 2 of 4 | Alt followed by C" (`\t` a tab). Its name and shortcut, its
    /// position, the hint on the
    /// first item, and its access key as a key sequence. The same item again says nothing.
    pub fn modern_focus(&mut self, item: &FocusEvent) -> Option<String> {
        let name = item.name.as_deref().unwrap_or_default().trim();
        if self.active && self.current.as_deref() == Some(name) {
            return None;
        }
        let first = !self.active;
        self.active = true;
        self.current = Some(name.to_owned());
        self.popup_pending = false;
        self.popup_shown = true;
        self.closed = None;
        let mut lines = vec![match item.accelerator_key.as_deref() {
            Some(shortcut) => format!("{name}\t{shortcut}"),
            None => name.to_owned(),
        }];
        lines.extend(position(item).map(|p| p.render()));
        if first {
            lines.push("To move through items press up or down arrow.".to_owned());
        }
        lines.extend(
            item.access_key
                .as_deref()
                .filter(|k| k.contains(", "))
                .map(|k| k.replace(", ", " followed by ")),
        );
        Some(lines.join("\n"))
    }

    pub fn popup_start(&mut self) {
        if self.page {
            return;
        }
        self.open.push(self.current.clone().unwrap_or_default());
        self.popup_pending = true;
        self.closed = None;
    }

    pub fn popup_end(&mut self) {
        if self.page {
            return;
        }
        self.closed = self.open.pop();
        self.popup_pending = false;
    }

    /// Focus landed on a page's menu item (an ARIA menu), said as JAWS says one (Google Docs'
    /// menus, dev box 2026-10-16): on the menu bar its name and position ("Help | 9 of 10"),
    /// in a menu its name, and "sub menu" for one that opens a menu ("Verbalize to screen
    /// reader s ► sub menu"). The first item of the session begins with "menu" and ends with
    /// the hint ("menu | Accessibility | 10 of 10 | To move through items press left or
    /// right arrow."). The same item reported again says nothing.
    pub fn page_focus(
        &mut self,
        name: &str,
        on_bar: bool,
        position: Option<(usize, usize)>,
        opens_menu: bool,
    ) -> Option<String> {
        let name = name.trim();
        if self.active && self.current.as_deref() == Some(name) {
            return None;
        }
        let first = !self.active;
        self.active = true;
        self.page = true;
        self.current = Some(name.to_owned());
        self.popup_pending = false;
        self.closed = None;
        let mut lines = Vec::new();
        if first {
            lines.push(role_marker("menu"));
            lines.push(String::new());
        }
        if on_bar {
            lines.push(name.to_owned());
            if let Some((index, count)) = position {
                lines.push(format!("{index} of {count}"));
            }
        } else if opens_menu {
            lines.push(format!("{name} {}", state_marker("sub menu")));
        } else {
            lines.push(name.to_owned());
        }
        if first {
            lines.push(
                if on_bar {
                    "To move through items press left or right arrow."
                } else {
                    "To move through items press up or down arrow."
                }
                .to_owned(),
            );
        }
        Some(lines.join("\n"))
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

    /// Windows Settings' Display page with Tab, JAWS on the dev box (2026-10-16): the groups
    /// entered are named, outermost first, and not again while the focus stays in them; a
    /// toggle switch says its state, a slider its kind, an expander's button its state.
    #[test]
    fn settings_display_page_names_the_groups_it_enters() {
        let control = |role: Role, name: &str, groups: &[&str]| {
            let mut focus = FocusEvent::new(role).with_name(name);
            focus.container_role = Some(Role::Group);
            focus.container_name = groups.last().map(|g| (*g).to_owned());
            focus.groups = groups
                .iter()
                .map(|g| crate::event::GroupEntry::group(*g))
                .collect();
            focus
        };
        let mut native = NativeFocus::default();
        let mut say = |focus: &FocusEvent| {
            native
                .focus(focus)
                .map(|a| norm(&a.render()))
                .unwrap_or_default()
        };
        // Its group is the slider's button's content: the platform gives it no name.
        let slider = control(Role::Slider, "Brightness", &[]);
        assert_eq!(
            say(&slider),
            norm("Brightness \u{1d} left right slider \u{1e} \nTo increase or decrease use the Arrow keys.")
        );
        let mut toggle = control(
            Role::Button,
            "Night light",
            &["Brightness & color", "Night light"],
        );
        toggle.platform.element_class = Some("ToggleSwitch".to_owned());
        toggle.states.checked = Some(false);
        assert_eq!(
            say(&toggle),
            norm("Brightness & color\nNight light\nNight light \u{1d} Button \u{1e} \nOff\nTo activate press spacebar.")
        );
        let page_button = control(
            Role::Button,
            "Night light",
            &["Brightness & color", "Night light"],
        );
        assert_eq!(
            say(&page_button),
            norm("Night light \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
        let mut combo = control(Role::ComboBox, "Scale", &["Scale & layout", "Scale"]);
        combo.value = Some("125% (Recommended)".to_owned());
        assert_eq!(
            say(&combo),
            norm("Scale & layout\nScale\nScale \u{1d} Combo box \u{1e} \n125% (Recommended)\nTo change the selection use the Arrow keys.")
        );
        let resolution = control(Role::ComboBox, "Display resolution", &["Scale & layout"]);
        assert!(!say(&resolution).starts_with("Scale & layout"));
        let mut more = control(Role::Button, "Show more settings", &["Multiple displays"]);
        more.states.expanded = Some(false);
        assert_eq!(
            say(&more),
            norm("Multiple displays\nShow more settings \u{1d} Button \u{1e} \u{1d}collapsed\u{1e} \nTo activate press spacebar.")
        );
    }

    /// Windows Settings' Sound page, JAWS on the dev box (2026-10-16): a list among the groups
    /// is said with its role, and groups the focus stays in are not said again.
    #[test]
    fn settings_sound_page_names_lists_and_sections() {
        use crate::event::GroupEntry;
        let mut native = NativeFocus::default();
        let mut say = |role: Role, name: &str, container: Role, groups: Vec<GroupEntry>| {
            let mut focus = FocusEvent::new(role).with_name(name);
            focus.container_role = Some(container);
            focus.container_name = groups.last().map(|g| g.name.clone());
            focus.groups = groups;
            native
                .focus(&focus)
                .map(|a| norm(&a.render()))
                .unwrap_or_default()
        };
        let output = vec![
            GroupEntry::group("Output"),
            GroupEntry::group("Choose where to play sound."),
        ];
        say(
            Role::Button,
            "Show more settings",
            Role::Group,
            output.clone(),
        );
        let mut speaker = output.clone();
        speaker.push(GroupEntry::list(""));
        speaker.push(GroupEntry::group("Hoparlör"));
        assert_eq!(
            say(Role::Button, "Hoparlör", Role::Group, speaker),
            norm("\u{1d} List box \u{1e} \nHoparlör\nHoparlör \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
        let mut add = output;
        add.push(GroupEntry::group("Pair a new output device"));
        assert_eq!(
            say(Role::Button, "Add device", Role::Group, add),
            norm("Pair a new output device\nAdd device \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
        // Bluetooth & devices: a list named by the text before it (the platform's label).
        let input = vec![
            GroupEntry::group("Input"),
            GroupEntry::list("Input"),
            GroupEntry::group("USB Receiver"),
        ];
        assert!(say(Role::Button, "Show more settings", Role::Group, input)
            .starts_with(&norm("Input\nInput \u{1d} List box \u{1e} \nUSB Receiver")));
    }

    /// A modern app's button says its access key as a sequence (Calculator: UIA AccessKey
    /// "Alt, I" is "Alt followed by I").
    #[test]
    fn a_buttons_access_key_sequence_is_said() {
        let mut history = FocusEvent::new(Role::Button).with_name("Open history flyout");
        history.access_key = Some("Alt, I".to_owned());
        assert!(norm(&focus_speech(&history, None).render())
            .ends_with("To activate press spacebar. Alt followed by I"));
    }

    /// Windows Settings' Power page: a link in an InfoBar, an unnamed dialog with no
    /// container around the link, is said after the bar (JAWS: "Power mode is set
    /// automatically ... dialog | More about power mode Link").
    #[test]
    fn a_link_in_an_info_bar_says_the_bar() {
        let mut native = NativeFocus::default();
        let _ = native.focus(&FocusEvent::new(Role::Button).with_name("Energy recommendations"));
        let mut link =
            FocusEvent::new(Role::Link { href: None }).with_name("More about power mode");
        link.groups = vec![crate::event::GroupEntry::dialog(
            "Power mode is set automatically.",
        )];
        assert_eq!(
            native.focus(&link).map(|a| norm(&a.render())),
            Some(norm(
                "Power mode is set automatically.  dialog  
More about power mode  Link  
To activate press Enter."
            ))
        );
    }

    /// Windows Settings' navigation items have no container (an unnamed group): moving from
    /// one to the next is still within the list, said with the position alone.
    #[test]
    fn items_without_a_container_move_within_their_list() {
        let item = |name: &str, index: usize| {
            let mut focus = FocusEvent::new(Role::ListItem).with_name(name);
            focus.item_index = Some(index);
            focus.item_count = Some(12);
            focus
        };
        let mut native = NativeFocus::default();
        let mut say = |focus: &FocusEvent| {
            native
                .focus(focus)
                .map(|a| norm(&a.render()))
                .unwrap_or_default()
        };
        assert_eq!(
            say(&item("Home", 1)),
            norm("Home\n1 of 12\nTo move to an item press the Arrow keys.")
        );
        assert_eq!(say(&item("System", 2)), norm("System\n2 of 12"));
    }

    /// Windows Settings' home page, JAWS on the dev box (2026-10-16): a link says its role and
    /// hint, and a button the position its application reports.
    #[test]
    fn settings_links_and_breadcrumb_buttons() {
        let link = FocusEvent::new(Role::Link { href: None }).with_name("Rename");
        assert_eq!(
            norm(&focus_speech(&link, None).render()),
            norm("Rename \u{1d} Link \u{1e} \nTo activate press Enter.")
        );
        let mut home = FocusEvent::new(Role::Button).with_name("Home");
        home.item_index = Some(1);
        home.item_count = Some(1);
        assert_eq!(
            norm(&focus_speech(&home, None).render()),
            norm("Home \u{1d} Button \u{1e} \n1 of 1\nTo activate press spacebar.")
        );
        let account = FocusEvent::new(Role::Button).with_name("Gigabyte Local Account");
        assert_eq!(
            norm(&focus_speech(&account, None).render()),
            norm("Gigabyte Local Account \u{1d} Button \u{1e} \nTo activate press spacebar.")
        );
    }

    /// Google Docs' menus, JAWS on the dev box (2026-10-16): Alt+Shift+A, Left, Down, Right,
    /// Down. Bar items say their position; menu items their name, and "sub menu" for one that
    /// opens a menu; popups opening and closing say nothing, also once quiet.
    #[test]
    fn a_pages_menus_say_where_their_items_are() {
        let mut m = Menus::default();
        m.popup_start();
        assert_eq!(
            m.page_focus("Accessibility", true, Some((10, 10)), true)
                .as_deref()
                .map(norm),
            Some(norm(
                " \u{1d} menu \u{1e} \n\nAccessibility\n10 of 10\n\
                 To move through items press left or right arrow."
            ))
        );
        m.popup_end();
        assert_eq!(
            m.page_focus("Help", true, Some((9, 10)), true).as_deref(),
            Some("Help\n9 of 10")
        );
        m.popup_start();
        assert_eq!(
            m.page_focus("Search the menus", false, Some((1, 3)), false)
                .as_deref(),
            Some("Search the menus")
        );
        assert_eq!(m.page_focus("Search the menus", false, None, false), None);
        assert_eq!(
            m.page_focus("Verbalize to screen reader", false, None, true)
                .as_deref()
                .map(norm),
            Some(norm("Verbalize to screen reader \u{1d}sub menu\u{1e}"))
        );
        m.popup_end();
        assert_eq!(m.idle(), None);
        assert!(m.end());
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

    /// File Explorer's context menu (JAWS on the dev box, 2026-10-17): each item with its
    /// shortcut, position and access key; the hint on the first.
    #[test]
    fn a_modern_menu_says_its_items_shortcuts_and_keys() {
        let item = |name: &str, shortcut: &str, index: usize, key: &str| {
            let mut item = FocusEvent::new(Role::MenuItem).with_name(name);
            item.accelerator_key = Some(shortcut.to_owned());
            item.item_index = Some(index);
            item.item_count = Some(4);
            item.access_key = Some(key.to_owned());
            item
        };
        let mut menus = Menus::default();
        assert_eq!(
            menus.modern_focus(&item("Cut", "Ctrl+X", 1, "Alt, T")).as_deref(),
            Some("Cut\tCtrl+X\n1 of 4\nTo move through items press up or down arrow.\nAlt followed by T")
        );
        assert_eq!(
            menus
                .modern_focus(&item("Copy", "Ctrl+C", 2, "Alt, C"))
                .as_deref(),
            Some("Copy\tCtrl+C\n2 of 4\nAlt followed by C")
        );
        assert_eq!(
            menus.modern_focus(&item("Copy", "Ctrl+C", 2, "Alt, C")),
            None
        );
        assert!(menus.end());
    }
}

#[cfg(test)]
mod tree_tests {
    use super::*;

    fn tree_item(name: &str, level: usize, index: usize, count: usize) -> FocusEvent {
        let mut item = FocusEvent::new(Role::TreeItem).with_name(name);
        item.container_role = Some(Role::Tree);
        item.tree_level = Some(level);
        item.item_index = Some(index);
        item.item_count = Some(count);
        item.states.expanded = Some(false);
        item
    }

    /// Arrows in a tree say a position with a new level only (303/s5 trial 1, s6, s8; File
    /// Explorer's navigation pane on the dev box, 2026-10-17).
    #[test]
    fn arrows_in_a_tree_say_a_position_with_a_new_level_only() {
        let mut native = NativeFocus::default();
        let say = |native: &mut NativeFocus, focus: &FocusEvent| {
            native.focus(focus).map(|a| a.render()).unwrap_or_default()
        };
        let entered = say(&mut native, &tree_item("Documents", 0, 1, 2));
        assert!(entered.contains("1 of 2"), "{entered}");
        let mut pictures = tree_item("Pictures", 0, 2, 2);
        pictures.states.expanded = None;
        assert_eq!(say(&mut native, &pictures), "Pictures");
        assert_eq!(
            say(&mut native, &tree_item("Documents", 0, 1, 2)),
            "Documents \u{1d}closed\u{1e} "
        );
        let mut reports = tree_item("Reports", 1, 1, 2);
        reports.states.expanded = None;
        assert_eq!(say(&mut native, &reports), "level 1\nReports\n1 of 2");
    }
}
