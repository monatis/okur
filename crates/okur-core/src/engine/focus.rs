//! Focus and menu events: what the engine says when the focus or a menu changes.

use super::{AppMode, DesktopItem, NativeDesktopState, NativeExplorerState, ScreenReaderEngine};
use crate::aom::Role;
use crate::event::FocusEvent;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Records the focused control's top-level window as the active one, for Insert+T and
    /// for announcing the next window change.
    fn note_window(title: &mut Option<String>, kind: &mut Option<String>, focus: &FocusEvent) {
        if let Some(window) = &focus.window_title {
            *title = Some(window.clone());
            *kind = focus.platform.top_window_class.clone();
        }
    }

    /// Handles an OS-level focus changed event.
    /// Synchronizes cursor position, activates/deactivates forms mode,
    /// and emits canonical JAWS speech announcements.
    pub fn handle_focus_event(&mut self, focus: &FocusEvent) -> Option<String> {
        if focus.role != Role::MenuItem {
            self.last_text = focus.text.clone();
            self.pending_edit = None;
        }
        if focus.role == Role::MenuItem {
            let speech = self
                .menus
                .focus(focus.name.as_deref().unwrap_or_default())?;
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        // Ignore spontaneous background tile updates from Windows 11 Widgets applet
        if focus
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Widgets"))
        {
            return None;
        }
        // Alt+Tab: the task switcher's items. Its window is remembered, so going back to an
        // application announces that application's window.
        if let Some(speech) = self.task_switcher.focus(focus) {
            Self::note_window(
                &mut self.last_window_title,
                &mut self.last_window_kind,
                focus,
            );
            if speech.is_empty() {
                return None;
            }
            self.speech.speak(&speech, true);
            return Some(speech);
        }

        let is_explorer_event = focus.platform.window_class.as_deref() == Some("CabinetWClass")
            || focus.platform.window_class.as_deref() == Some("ShellTabWindowClass")
            || focus.platform.window_class.as_deref() == Some("UIItemsView")
            || focus.container_name.as_deref() == Some("Items View")
            || focus.container_name.as_deref() == Some("Items view")
            || focus.container_name.as_deref() == Some("Shell Folder View")
            || focus
                .window_title
                .as_deref()
                .is_some_and(|t| t.contains("File Explorer"));

        // 0. Check if focus landed on native desktop
        let is_desktop_event = !is_explorer_event
            && (focus.container_name.as_deref() == Some("FolderView")
                || focus.platform.window_class.as_deref() == Some("Progman")
                || focus.platform.window_class.as_deref() == Some("WorkerW")
                || (focus.platform.window_class.as_deref() == Some("SysListView32")
                    && focus.container_name.as_deref() != Some("Items View")
                    && focus.container_name.as_deref() != Some("Items view"))
                // An item in the desktop's window, whatever window the event came from (on
                // Windows 11 the first of the desktop's focus events has none, seen live).
                || (focus.role == Role::ListItem
                    && focus
                        .platform
                        .top_window_class
                        .as_deref()
                        .is_some_and(|k| matches!(k, "Progman" | "WorkerW")))
                // An item still on the desktop; not one in another window (the task
                // switcher's, after Win+M then Alt+Tab).
                || (focus.role == Role::ListItem
                    && self.app_mode == AppMode::NativeDesktop
                    && focus
                        .platform
                        .top_window_class
                        .as_deref()
                        .is_none_or(|k| matches!(k, "Progman" | "WorkerW"))));

        if is_desktop_event {
            let prev_mode = self.app_mode;
            let ds = self
                .desktop_state
                .get_or_insert_with(|| NativeDesktopState {
                    container_name: "FolderView".to_string(),
                    items: Vec::new(),
                    current_index: 0,
                    entered: false,
                });

            let is_first_entry = prev_mode != AppMode::NativeDesktop || !ds.entered;
            ds.entered = true;
            self.app_mode = AppMode::NativeDesktop;
            // Insert+T on the desktop speaks the virtual desktop's name, not the window
            // focused before it (user report).
            Self::note_window(
                &mut self.last_window_title,
                &mut self.last_window_kind,
                focus,
            );

            if let Some(ref mut es) = self.explorer_state {
                es.entered = false;
            }

            let name = focus.name.as_deref().unwrap_or("Desktop");
            let index = focus
                .item_index
                .unwrap_or_else(|| ds.current_item().map(|it| it.index).unwrap_or(1));
            let total = focus
                .item_count
                .unwrap_or_else(|| ds.current_item().map(|it| it.total).unwrap_or(1));

            // Sync ds.items and check for deduplication
            if ds.items.is_empty() {
                ds.items.push(DesktopItem {
                    node_id: index,
                    name: name.to_string(),
                    index,
                    total,
                });
                ds.current_index = 0;
            } else if let Some(pos) = ds
                .items
                .iter()
                .position(|it| it.name.eq_ignore_ascii_case(name) || it.index == index)
            {
                if !is_first_entry && ds.current_index == pos {
                    return None;
                }
                ds.current_index = pos;
            } else {
                ds.items.push(DesktopItem {
                    node_id: index,
                    name: name.to_string(),
                    index,
                    total,
                });
                ds.current_index = ds.items.len().saturating_sub(1);
            }

            let speech = if is_first_entry {
                SpeechFormatter::format_desktop_entry("FolderView", name, index, total)
            } else {
                SpeechFormatter::format_desktop_item(name, index, total)
            };
            self.speech.speak(&speech, true);
            return Some(speech);
        } else if is_explorer_event && (focus.role == Role::ListItem || focus.role == Role::List) {
            let prev_mode = self.app_mode;
            let es = self
                .explorer_state
                .get_or_insert_with(NativeExplorerState::default);

            let is_first_entry = prev_mode != AppMode::NativeExplorer || !es.entered;
            es.entered = true;
            self.app_mode = AppMode::NativeExplorer;
            Self::note_window(
                &mut self.last_window_title,
                &mut self.last_window_kind,
                focus,
            );

            if let Some(ref mut ds) = self.desktop_state {
                ds.entered = false;
            }

            if let Some(ref title) = focus.window_title {
                es.window_title = Some(title.clone());
            }
            if let Some(ref folder) = focus.folder_name {
                es.folder_name = Some(folder.clone());
            }

            let name = focus.name.as_deref().unwrap_or("Item");
            let index = focus.item_index.unwrap_or(1);
            let total = focus.item_count.unwrap_or(1);
            let selected = focus.states.selected;

            if !is_first_entry
                && es.last_index == Some(index)
                && es.last_name.as_deref() == Some(name)
            {
                return None;
            }
            es.last_index = Some(index);
            es.last_name = Some(name.to_string());

            let speech = if is_first_entry {
                let window_title = focus
                    .window_title
                    .as_deref()
                    .or(es.window_title.as_deref())
                    .or(Some("File Explorer"));
                let folder_name = focus.folder_name.as_deref().or(es.folder_name.as_deref());
                SpeechFormatter::format_explorer_entry(
                    window_title,
                    folder_name,
                    selected,
                    name,
                    index,
                    total,
                )
            } else {
                SpeechFormatter::format_explorer_item(name, index, total)
            };
            self.speech.speak(&speech, true);
            return Some(speech);
        } else if focus.platform.window_class.as_deref() == Some("Shell_TrayWnd")
            || focus.container_name.as_deref() == Some("Taskbar")
        {
            if let Some(ref mut es) = self.explorer_state {
                es.entered = false;
            }
            let prev_mode = self.app_mode;
            self.app_mode = AppMode::NativeShell;
            let name = focus.name.as_deref().unwrap_or("Start Button");
            let container = if prev_mode != AppMode::NativeShell {
                focus.container_name.as_deref().or(Some("Taskbar"))
            } else {
                None
            };
            let speech = SpeechFormatter::format_shell_element(
                container,
                name,
                "Button",
                focus.help_text.as_deref(),
            );
            self.speech.speak(&speech, true);
            return Some(speech);
        }

        if let Some(ref mut ds) = self.desktop_state {
            ds.entered = false;
        }
        if let Some(ref mut es) = self.explorer_state {
            es.entered = false;
        }

        let is_in_document = focus.in_document
            || focus
                .node_id
                .is_some_and(|id| self.cursor.buffer.contains_node(id));

        if is_in_document {
            self.app_mode = AppMode::VirtualBuffer;
            // The page is in the same top-level window as anything focused next in the
            // browser (toolbar, address bar), so that focus does not announce a new window.
            if let Some(title) = &focus.window_title {
                self.last_window_title = Some(title.clone());
                self.last_window_kind = focus.platform.top_window_class.clone();
            }

            let target_id = self.page_node_for(focus);

            // If focus target is a dialog, enter modal confinement and announce Dialog <Title>
            if matches!(focus.role, Role::Dialog | Role::AlertDialog) {
                let title = focus.name.as_deref().unwrap_or("");
                let msg = if let Some(id) = target_id {
                    self.cursor.move_to_node_id(id);
                    self.cursor
                        .enter_modal(id)
                        .unwrap_or_else(|| SpeechFormatter::format_dialog_entry(title))
                } else {
                    SpeechFormatter::format_dialog_entry(title)
                };
                if !msg.is_empty() {
                    self.speech.speak(&msg, true);
                }
                return Some(msg);
            }

            // If target is inside a modal dialog, ensure modal confinement is active
            if let Some(id) = target_id {
                if let Some(dlg) = self.cursor.buffer.dialog_for_node(id) {
                    if dlg.is_modal && self.cursor.active_modal_root != Some(dlg.node_id) {
                        let _ = self.cursor.enter_modal(dlg.node_id);
                    }
                } else if self.cursor.is_in_modal() {
                    let _ = self.cursor.exit_modal();
                }
            }

            // Forms mode follows focus into edits silently: JAWS 2025 signals it with a
            // sound by default, and its speech history has no forms-mode text [obs, 204].
            let target_role = target_id
                .and_then(|id| self.cursor.buffer.node(id))
                .map(|n| n.role.clone());
            if target_role == Some(Role::Document) {
                // The page itself has focus: the next control focus enters the page.
                self.last_page_focus = None;
                return None;
            }
            // Forms mode follows focus into edits; a combo box keeps the forms mode Enter
            // turned on [obs, 205/s8].
            let is_edit = target_role == Some(Role::Edit) || focus.role == Role::Edit;
            let in_forms_mode = is_edit || (target_role == Some(Role::ComboBox) && self.forms_mode);
            self.forms_mode = in_forms_mode;
            self.cursor.forms_mode_active = in_forms_mode;
            // Chrome can report focus on the same element again (while typing in it); JAWS
            // does not repeat the field.
            if target_id.is_some() && target_id == self.last_page_focus {
                return None;
            }
            // The first control focused after the page itself is preceded by the page title
            // [obs, 205/s2; 204/s1 second variant].
            let entering_page = self.last_page_focus.is_none();
            // Group entry is judged from where focus was, not the virtual cursor [obs, 205/s6].
            let previous_focus_offset = self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.lines.iter().find(|l| l.node_id == id))
                .map(|l| l.start_offset);
            self.last_page_focus = target_id;
            // The focus event carries the live state; the buffer's copy may be older (Space
            // just toggled the control) [obs, 205/s2, s6].
            if let Some(id) = target_id {
                let stale = self.cursor.buffer.node(id).is_some_and(|n| {
                    focus.states.checked.is_some() && n.states.checked != focus.states.checked
                });
                if stale {
                    if let Some(rebuilt) = self
                        .cursor
                        .buffer
                        .with_node_changed(id, |n| n.states.checked = focus.states.checked)
                    {
                        self.cursor.buffer = rebuilt;
                    }
                }
            }
            if let Some(offset) = previous_focus_offset {
                self.cursor.offset = offset;
            }
            let mut speech = target_id
                .and_then(|id| self.cursor.focus_node(id))
                .or_else(|| focus.name.clone());
            if target_role == Some(Role::ComboBox) && in_forms_mode {
                // In forms mode a combo box gets its usage hint [obs, 205/s8].
                speech =
                    speech.map(|s| format!("{s}\nTo change the selection use the Arrow keys."));
            }
            // Forms mode turned on by Enter adds the regions the field is in and the edit's
            // hint [obs, 19/s2]; Tab's automatic forms mode does not (204/s1).
            let by_enter = std::mem::take(&mut self.forms_mode_by_enter) && is_edit;
            if by_enter {
                speech = speech.map(|s| format!("{s}\nType in text."));
            }
            if entering_page {
                let regions = if by_enter {
                    target_id
                        .map(|id| self.region_markers(id))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                if let Some(title) = self.cursor.buffer.document_title() {
                    speech = speech.map(|s| format!("{title}\n{regions}{s}"));
                }
            }
            if let Some(ref text) = speech {
                if !text.is_empty() {
                    self.speech.speak(text, true);
                }
            }

            return speech;
        }

        // Native control / application window handling
        self.last_page_focus = None;
        self.app_mode = AppMode::NativeControl;
        if self.forms_mode {
            self.forms_mode = false;
            self.cursor.forms_mode_active = false;
        }

        let window_changed = if let Some(ref wt) = focus.window_title {
            let changed = self.last_window_title.as_ref() != Some(wt)
                || (focus.platform.top_window_class.is_some()
                    && self.last_window_kind != focus.platform.top_window_class);
            if changed {
                self.last_window_title = Some(wt.clone());
                self.last_window_kind = focus.platform.top_window_class.clone();
            }
            changed
        } else {
            false
        };

        if window_changed {
            self.native_focus.reset();
        }
        let control_speech = self.native_focus.focus(focus)?;
        self.last_native_focus = Some(focus.clone());

        let speech = if window_changed && crate::apps::windows_search::names_window(focus) {
            // A new active window: its title, then the control. A dialog box adds its role
            // and its text [obs, 306/s1, s3, s10].
            // The desktop is announced as "desktop"; its title (Insert+T) is the virtual
            // desktop's name (JAWS on Windows 11: "desktop | FolderView List view").
            let win = if focus.platform.top_window_class.as_deref() == Some("Progman") {
                "desktop"
            } else {
                focus.window_title.as_deref().unwrap_or("")
            };
            let mut heading = match focus.dialog_text.as_deref() {
                Some(text) if !text.is_empty() => format!("{win}  \u{1d} dialog \u{1e} \n{text}"),
                Some(_) => format!("{win}  \u{1d} dialog \u{1e} "),
                None => win.to_owned(),
            };
            // A property sheet names the page it opens on (JAWS 2025 on the dev box:
            // "Local Disk (C:) Properties dialog | General page").
            if let Some(page) = &focus.dialog_page {
                heading.push_str(&format!("\n{page}  \u{1d} page \u{1e} "));
            }
            if control_speech.is_empty() || control_speech == win {
                heading
            } else {
                format!("{heading}\n{control_speech}")
            }
        } else {
            control_speech
        };

        if !speech.is_empty() {
            self.speech.speak(&speech, true);
            Some(speech)
        } else {
            None
        }
    }

    /// Handles menu mode transitions [obs, 302]. Leaving menus re-announces the control
    /// focused before them, as on a fresh focus: `Leaving menus` | `Name: ‹ Edit ›` | …
    pub fn handle_menu_event(&mut self, event: crate::event::MenuEvent) -> Option<String> {
        use crate::event::MenuEvent;
        match event {
            MenuEvent::Start => None,
            MenuEvent::PopupStart => {
                self.menus.popup_start();
                None
            }
            MenuEvent::PopupEnd => {
                self.menus.popup_end();
                None
            }
            MenuEvent::End => {
                if !self.menus.end() {
                    return None;
                }
                let mut speech = "Leaving menus".to_owned();
                if let Some(focus) = &self.last_native_focus {
                    speech.push('\n');
                    speech.push_str(&crate::native_focus::focus_speech(focus, None));
                }
                self.speech.speak(&speech, true);
                Some(speech)
            }
        }
    }
}
