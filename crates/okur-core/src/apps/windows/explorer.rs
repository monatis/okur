//! File Explorer's file list. Entering it reads the window, the folder, the list ("Items
//! View MultiSelect ListBox") and the item; moving reads the item and its position.

use crate::announcement::{Announcement, Line, Part, PartKind};
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::engine::AppMode;
use crate::event::FocusEvent;
use crate::messages::msg;

/// Whether `focus` is in a File Explorer window, by its window's class, its list's name or
/// its title.
pub(crate) fn is_explorer_window(focus: &FocusEvent) -> bool {
    matches!(
        focus.platform.window_class.as_deref(),
        Some("CabinetWClass" | "ShellTabWindowClass" | "UIItemsView")
    ) || matches!(
        focus.container_name.as_deref(),
        Some("Items View" | "Items view" | "Shell Folder View")
    ) || focus
        .window_title
        .as_deref()
        .is_some_and(|t| t.contains("File Explorer"))
}

/// File Explorer.
#[derive(Debug, Default)]
pub struct Explorer {
    window_title: Option<String>,
    folder_name: Option<String>,
    /// The item last read, which a repeated focus event does not read again.
    last_index: Option<usize>,
    last_name: Option<String>,
}

/// The window of the folder view around the file list, which File Explorer focuses on the way
/// to an item: JAWS says nothing for it (dev box, 2026-10-17; an empty folder not checked).
const FOLDER_VIEW_CLASS: &str = "SHELLDLL_DefView";

impl AppModule for Explorer {
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        if focus.role == Role::Generic
            && focus.platform.window_class.as_deref() == Some(FOLDER_VIEW_CLASS)
        {
            return FocusResponse::Silent;
        }
        if !(is_explorer_window(focus) && matches!(focus.role, Role::ListItem | Role::List)) {
            return FocusResponse::Pass;
        }
        // Entering from another window (another folder too: Enter on one, Alt+Up) names it;
        // entering from within (Tab from the navigation pane) does not. From another
        // application the list's hint is left out.
        let window_changed = focus.window_title.is_some()
            && cx.window_title.as_deref() != focus.window_title.as_deref();
        let first_entry = *cx.app_mode != AppMode::NativeExplorer || window_changed;
        *cx.app_mode = AppMode::NativeExplorer;
        let from_other_app = cx.window_kind.is_none()
            || cx.window_kind.as_deref() != focus.platform.top_window_class.as_deref();
        cx.note_window(focus);
        if let Some(title) = &focus.window_title {
            self.window_title = Some(title.clone());
        }
        if let Some(folder) = &focus.folder_name {
            self.folder_name = Some(folder.clone());
        }

        let name = focus
            .name
            .as_deref()
            .unwrap_or(msg::explorer_item_unnamed().as_str());
        let index = focus.item_index.unwrap_or(1);
        let total = focus.item_count.unwrap_or(1);
        if !first_entry && self.last_index == Some(index) && self.last_name.as_deref() == Some(name)
        {
            return FocusResponse::Silent;
        }
        self.last_index = Some(index);
        self.last_name = Some(name.to_string());

        FocusResponse::Speak(if first_entry {
            let window = window_changed.then(|| {
                let title = focus
                    .window_title
                    .as_deref()
                    .or(self.window_title.as_deref())
                    .unwrap_or(msg::explorer_title().as_str());
                let folder = focus.folder_name.as_deref().or(self.folder_name.as_deref());
                (title, folder)
            });
            // The item's row after its name, else its type and size.
            let kind = if focus.item_details.is_empty() {
                [focus.item_type.as_deref(), focus.item_size.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" ")
            } else {
                focus.item_details.join(" ")
            };
            entry(&Entry {
                window,
                selected: focus.states.selected,
                name,
                kind: Some(kind.as_str()).filter(|k| !k.is_empty()),
                index,
                total,
                hint: !(window_changed && from_other_app),
            })
        } else if cx.selecting && focus.states.selected == Some(true) {
            // Shift+Down extending the selection (JAWS on the dev box, 2026-10-17):
            // "selected | handoff", no position.
            Announcement::from_text(&format!("{}\n{name}", msg::state_selected()))
        } else {
            Announcement::from_text(&format!("{name}\n{}", msg::position(index, total)))
        })
    }
}

/// What entering the file list says.
struct Entry<'a> {
    /// The window's title and folder, when the focus was in another window.
    window: Option<(&'a str, Option<&'a str>)>,
    selected: Option<bool>,
    name: &'a str,
    /// The item's type and size ("Markdown Kaynak Dosyası 1 KB", "File folder").
    kind: Option<&'a str>,
    index: usize,
    total: usize,
    hint: bool,
}

/// Entering the file list: the window and folder when the focus was in another window, the
/// list, the item, its type and size, its position, and the list's hint. JAWS 2025, Windows
/// 11, on the dev box (research): from another application, "live - File Explorer | Items
/// View MultiSelect ListBox | Not Selected combo.cs | 1 of 8"; Enter on a folder, "backlog -
/// File Explorer | backlog | Items View MultiSelect ListBox | Not Selected
/// complex-web-apps-….md | Markdown Kaynak Dosyası 1 KB | 1 of 2 | To move to an item press
/// the Arrow keys."; Tab from the navigation pane onto a selected folder, "Items View
/// MultiSelect ListBox | backlog | File folder | 1 of 4 | To move to an item press the Arrow
/// keys." (2026-10-17).
fn entry(e: &Entry<'_>) -> Announcement {
    let mut lines = Vec::new();
    if let Some((title, folder)) = e.window {
        lines.push(Line::of(PartKind::Container, title));
        lines.extend(folder.map(|f| Line::of(PartKind::Container, f)));
    }
    lines.push(Line::new(vec![
        Part::new(PartKind::Container, "Items View"),
        Part::role(msg::role_list_box_multi_select().as_str()),
    ]));
    lines.push(Line::of(
        PartKind::Name,
        if e.selected == Some(false) {
            msg::list_item_not_selected(e.name).render()
        } else {
            e.name.to_owned()
        },
    ));
    lines.extend(e.kind.map(|k| Line::of(PartKind::Value, k)));
    lines.push(Line::of(
        PartKind::Position,
        msg::position(e.index, e.total),
    ));
    if e.hint {
        lines.push(Line::of(PartKind::Hint, msg::hint_move_to_an_item_arrows()));
    }
    Announcement::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPLORER_WINDOW: &str = "CabinetWClass";

    /// What entering the list says, after a focus in the window `before` (title, top window
    /// class).
    fn entering(focus: &FocusEvent, before: (&str, &str)) -> String {
        let mut explorer = Explorer::default();
        let mut mode = AppMode::NativeControl;
        let mut title = Some(before.0.to_owned());
        let mut kind = Some(before.1.to_owned());
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: false,
            escape_key: false,
            key: None,
            ask_view: false,
        };
        match explorer.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) => announcement.render(),
            _ => String::new(),
        }
    }

    fn item(window: &str, folder: &str, name: &str, selected: bool) -> FocusEvent {
        let mut item = FocusEvent::new(Role::ListItem)
            .with_window_title(window)
            .with_folder_name(folder)
            .with_container_name("Items View")
            .with_name(name)
            .with_item_index(1);
        item.platform.top_window_class = Some(EXPLORER_WINDOW.to_owned());
        item.states.selected = Some(selected);
        item
    }

    /// JAWS on the dev box, 2026-10-17.
    #[test]
    fn entering_the_list_says_the_item_its_kind_and_the_hint() {
        // Tab from the navigation pane onto the selected folder: no window, no "selected".
        let mut backlog = item("docs - File Explorer", "docs", "backlog", true).with_item_count(4);
        backlog.item_type = Some("File folder".to_owned());
        assert_eq!(
            entering(&backlog, ("docs - File Explorer", EXPLORER_WINDOW)),
            "Items View \u{1d} MultiSelect ListBox \u{1e} \nbacklog\nFile folder\n1 of 4\nTo move to an item press the Arrow keys."
        );
        // Enter on that folder: the new window and folder, the type and size.
        let mut file =
            item("backlog - File Explorer", "backlog", "notes.md", false).with_item_count(2);
        file.item_type = Some("Markdown Kaynak Dosyası".to_owned());
        file.item_size = Some("1 KB".to_owned());
        assert_eq!(
            entering(&file, ("docs - File Explorer", EXPLORER_WINDOW)),
            "backlog - File Explorer\nbacklog\nItems View \u{1d} MultiSelect ListBox \u{1e} \nNot Selected notes.md\nMarkdown Kaynak Dosyası 1 KB\n1 of 2\nTo move to an item press the Arrow keys."
        );
        // From another application: no hint.
        assert!(!entering(&file, ("Settings", "ApplicationFrameWindow")).contains("To move"));
    }

    #[test]
    fn the_folder_views_own_focus_says_nothing() {
        let mut view = FocusEvent::new(Role::Generic).with_name("Shell Folder View");
        view.platform.window_class = Some(FOLDER_VIEW_CLASS.to_owned());
        let mut explorer = Explorer::default();
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
            key: None,
            ask_view: false,
        };
        assert!(matches!(
            explorer.focus(&view, &mut cx),
            FocusResponse::Silent
        ));
    }

    /// Shift+Down extends the selection: "selected | handoff" (JAWS on the dev box, 2026-10-17).
    #[test]
    fn extending_the_selection_says_selected_and_the_item() {
        let mut explorer = Explorer::default();
        let mut mode = AppMode::NativeControl;
        let mut title = Some("docs - File Explorer".to_owned());
        let mut kind = Some(EXPLORER_WINDOW.to_owned());
        let mut say = |focus: &FocusEvent, selecting: bool| {
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting,
                alt_held: false,
                letter_key: false,
                escape_key: false,
                key: None,
                ask_view: false,
            };
            match explorer.focus(focus, &mut cx) {
                FocusResponse::Speak(announcement) => announcement.render(),
                _ => String::new(),
            }
        };
        say(
            &item("docs - File Explorer", "docs", "backlog", true).with_item_count(4),
            false,
        );
        let handoff = item("docs - File Explorer", "docs", "handoff", true)
            .with_item_index(2)
            .with_item_count(4);
        assert_eq!(say(&handoff, true), "selected\nhandoff");
    }
}
