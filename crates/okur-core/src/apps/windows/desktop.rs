//! The Windows desktop's icons (the "FolderView" list). Entering it reads "desktop", the list
//! and the item with its position and hints; moving reads the item and its position.

use crate::announcement::Announcement;
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::engine::AppMode;
use crate::event::FocusEvent;

/// The desktop's windows, by class.
const DESKTOP_WINDOWS: [&str; 2] = ["Progman", "WorkerW"];

/// An icon read on the desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Item {
    name: String,
    index: usize,
    total: usize,
}

/// The Windows desktop.
#[derive(Debug, Default)]
pub struct Desktop {
    /// The icons read so far, and the last one, which a repeated focus does not read again.
    items: Vec<Item>,
    current: usize,
}

impl Desktop {
    /// Whether `focus` is on the desktop: its list, its window, or an item in it.
    fn is_desktop(focus: &FocusEvent, app_mode: AppMode) -> bool {
        let window_class = focus.platform.window_class.as_deref();
        let top = focus.platform.top_window_class.as_deref();
        let in_desktop_window = |w: &str| DESKTOP_WINDOWS.contains(&w);
        !super::explorer::is_explorer_window(focus)
            && (focus.container_name.as_deref() == Some("FolderView")
                || window_class.is_some_and(in_desktop_window)
                || (window_class == Some("SysListView32")
                    && !matches!(
                        focus.container_name.as_deref(),
                        Some("Items View" | "Items view")
                    ))
                // An item in the desktop's window, whatever window the event came from (on
                // Windows 11 the first of the desktop's focus events has none, seen live).
                || (focus.role == Role::ListItem && top.is_some_and(in_desktop_window))
                // An item still on the desktop; not one in another window (the task
                // switcher's, after Win+M then Alt+Tab).
                || (focus.role == Role::ListItem
                    && app_mode == AppMode::NativeDesktop
                    && top.is_none_or(in_desktop_window)))
    }
}

impl AppModule for Desktop {
    fn owns(&self, focus: &FocusEvent) -> bool {
        focus
            .platform
            .top_window_class
            .as_deref()
            .is_some_and(|w| DESKTOP_WINDOWS.contains(&w))
    }

    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        if !Self::is_desktop(focus, *cx.app_mode) {
            return FocusResponse::Pass;
        }
        let first_entry = *cx.app_mode != AppMode::NativeDesktop;
        *cx.app_mode = AppMode::NativeDesktop;
        // Insert+T on the desktop speaks the virtual desktop's name, not the window focused
        // before it (user report).
        cx.note_window(focus);

        let current = self.items.get(self.current);
        let name = focus.name.as_deref().unwrap_or("Desktop");
        let index = focus
            .item_index
            .unwrap_or_else(|| current.map(|it| it.index).unwrap_or(1));
        let total = focus
            .item_count
            .unwrap_or_else(|| current.map(|it| it.total).unwrap_or(1));

        if self.items.is_empty() {
            self.items.push(Item {
                name: name.to_string(),
                index,
                total,
            });
            self.current = 0;
        } else if let Some(pos) = self
            .items
            .iter()
            .position(|it| it.name.eq_ignore_ascii_case(name) || it.index == index)
        {
            if !first_entry && self.current == pos {
                return FocusResponse::Silent;
            }
            self.current = pos;
        } else {
            self.items.push(Item {
                name: name.to_string(),
                index,
                total,
            });
            self.current = self.items.len().saturating_sub(1);
        }

        FocusResponse::Speak(Announcement::from_text(&if first_entry {
            // JAWS heads it with "desktop", not the list's role. JAWS 2025 on Windows 11,
            // observed on the dev box (2026-10-15; research, scenario 316 records it on
            // old-pc): "desktop | FolderView List view | Dropbox | 7 of 11 | To move to items
            // use the Arrow keys. | To edit the selected item press f2."
            format!(
                "desktop\nFolderView    List view   \n{name}\n{index} of {total}\nTo move to items use the Arrow keys.\nTo edit the selected item press f2."
            )
        } else {
            format!("{name}\n{index} of {total}")
        }))
    }

    /// A window that comes to the front on the desktop is announced as "desktop"; its title
    /// (Insert+T) is the virtual desktop's name (JAWS on Windows 11: "desktop | FolderView List
    /// view").
    fn window_name(&self, focus: &FocusEvent) -> Option<String> {
        (focus.platform.top_window_class.as_deref() == Some("Progman")).then(|| "desktop".into())
    }
}
