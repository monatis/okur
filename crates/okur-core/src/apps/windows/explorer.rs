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

impl AppModule for Explorer {
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        if !(is_explorer_window(focus) && matches!(focus.role, Role::ListItem | Role::List)) {
            return FocusResponse::Pass;
        }
        let first_entry = *cx.app_mode != AppMode::NativeExplorer;
        *cx.app_mode = AppMode::NativeExplorer;
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
            let window_title = focus
                .window_title
                .as_deref()
                .or(self.window_title.as_deref())
                .or(Some("File Explorer"));
            let folder_name = focus.folder_name.as_deref().or(self.folder_name.as_deref());
            entry(
                window_title,
                folder_name,
                focus.states.selected,
                name,
                index,
                total,
            )
        } else {
            Announcement::from_text(&format!("{name}\n{index} of {total}"))
        })
    }
}

/// Entering the file list: the window, the folder, the list and the item.
fn entry(
    window_title: Option<&str>,
    folder_name: Option<&str>,
    selected: Option<bool>,
    item_name: &str,
    index: usize,
    total: usize,
) -> Announcement {
    let mut lines = Vec::new();
    if let Some(title) = window_title {
        lines.push(Line::text(title));
    }
    if let Some(folder) = folder_name {
        lines.push(Line::text(folder));
    }
    // JAWS 2025, Windows 11, observed on the dev box (research): "live - File Explorer |
    // Items View MultiSelect ListBox | Not Selected combo.cs | 1 of 8".
    lines.push(Line::new(vec![
        Part::Text("Items View".to_owned()),
        Part::Role("MultiSelect ListBox".to_owned()),
    ]));
    match selected {
        Some(false) => lines.push(Line::text(format!("Not Selected {item_name}"))),
        Some(true) => {
            lines.push(Line::text("selected"));
            lines.push(Line::text(item_name));
        }
        None => lines.push(Line::text(item_name)),
    }
    lines.push(Line::text(format!("{index} of {total}")));
    Announcement::new(lines)
}
