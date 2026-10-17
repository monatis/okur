//! File Explorer's file list. Entering it reads the window, the folder, the list ("Items
//! View MultiSelect ListBox") and the item; moving reads the item and its position.

use crate::announcement::{Announcement, Line, Part};
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::engine::AppMode;
use crate::event::FocusEvent;

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
        let first_entry = *cx.app_mode != AppMode::NativeExplorer;
        *cx.app_mode = AppMode::NativeExplorer;
        // Entering from another window names it; entering from within (Tab from the
        // navigation pane) does not.
        let window_changed = focus.window_title.is_some()
            && cx.window_title.as_deref() != focus.window_title.as_deref();
        cx.note_window(focus);
        if let Some(title) = &focus.window_title {
            self.window_title = Some(title.clone());
        }
        if let Some(folder) = &focus.folder_name {
            self.folder_name = Some(folder.clone());
        }

        let name = focus.name.as_deref().unwrap_or("Item");
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
                    .unwrap_or("File Explorer");
                let folder = focus.folder_name.as_deref().or(self.folder_name.as_deref());
                (title, folder)
            });
            entry(
                window,
                focus.states.selected,
                name,
                focus.item_type.as_deref(),
                index,
                total,
            )
        } else {
            Announcement::from_text(&format!("{name}\n{index} of {total}"))
        })
    }
}

/// Entering the file list: the window and folder when it is another window's, the list, the
/// item and its kind, its position, and, from within the window, the list's hint.
fn entry(
    window: Option<(&str, Option<&str>)>,
    selected: Option<bool>,
    item_name: &str,
    item_type: Option<&str>,
    index: usize,
    total: usize,
) -> Announcement {
    let mut lines = Vec::new();
    if let Some((title, folder)) = window {
        lines.push(Line::text(title));
        lines.extend(folder.map(Line::text));
    }
    // JAWS 2025, Windows 11, on the dev box (research): from another window, "live - File
    // Explorer | Items View MultiSelect ListBox | Not Selected combo.cs | 1 of 8"; Tab from the
    // navigation pane onto a selected folder, "Items View MultiSelect ListBox | backlog | File
    // folder | 1 of 4 | To move to an item press the Arrow keys." (2026-10-17).
    lines.push(Line::new(vec![
        Part::Text("Items View".to_owned()),
        Part::Role("MultiSelect ListBox".to_owned()),
    ]));
    lines.push(Line::text(if selected == Some(false) {
        format!("Not Selected {item_name}")
    } else {
        item_name.to_owned()
    }));
    lines.extend(item_type.map(Line::text));
    lines.push(Line::text(format!("{index} of {total}")));
    if window.is_none() {
        lines.push(Line::text("To move to an item press the Arrow keys."));
    }
    Announcement::new(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entering(focus: &FocusEvent, title_before: Option<&str>) -> String {
        let mut explorer = Explorer::default();
        let mut mode = AppMode::NativeControl;
        let mut title = title_before.map(str::to_owned);
        let mut kind = None;
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
        };
        match explorer.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) => announcement.render(),
            _ => String::new(),
        }
    }

    /// Tab from the navigation pane onto the list's selected folder (JAWS on the dev box,
    /// 2026-10-17): no window, no "selected", the item's kind and the hint.
    #[test]
    fn tab_into_the_list_says_the_item_and_its_kind() {
        let mut backlog = FocusEvent::new(Role::ListItem)
            .with_window_title("docs - File Explorer")
            .with_container_name("Items View")
            .with_name("backlog")
            .with_item_index(1)
            .with_item_count(4);
        backlog.states.selected = Some(true);
        backlog.item_type = Some("File folder".to_owned());
        assert_eq!(
            entering(&backlog, Some("docs - File Explorer")),
            "Items View \u{1d} MultiSelect ListBox \u{1e} \nbacklog\nFile folder\n1 of 4\nTo move to an item press the Arrow keys."
        );
        // From another window: the window first, and no hint.
        assert!(entering(&backlog, Some("Settings")).starts_with("docs - File Explorer\n"));
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
        };
        assert!(matches!(
            explorer.focus(&view, &mut cx),
            FocusResponse::Silent
        ));
    }
}
