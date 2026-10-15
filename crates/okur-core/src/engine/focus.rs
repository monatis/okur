//! Focus and menu events: what the engine says when the focus or a menu changes.

use super::{AppMode, ScreenReaderEngine};
use crate::announcement::{Announcement, Line, Part};
use crate::aom::Role;
use crate::apps::{AppContext, FocusResponse};
use crate::event::FocusEvent;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
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
        // Applications first, as JAWS runs an application's script before the default one.
        let mut cx = AppContext {
            app_mode: &mut self.app_mode,
            window_title: &mut self.last_window_title,
            window_kind: &mut self.last_window_kind,
        };
        for app in &mut self.apps {
            match app.focus(focus, &mut cx) {
                FocusResponse::Pass => {}
                FocusResponse::Silent => return None,
                FocusResponse::Speak(announcement) => {
                    let speech = announcement.render();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
            }
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
            tracing::debug!(
                ?target_id,
                target = ?target_id.and_then(|id| self.cursor.buffer.node(id)).map(|n| (&n.role, &n.name)),
                "page focus target"
            );

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
                self.tab_start = None;
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
            self.tab_start = target_id;
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
        self.tab_start = None;
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
        let control = self.native_focus.focus(focus)?;
        self.last_native_focus = Some(focus.clone());

        let app = self.app_for(focus);
        let names_window = app.is_none_or(|app| app.names_window(focus));
        let announcement = if window_changed && names_window {
            self.window_heading(focus, app, control)
        } else {
            control
        };
        let speech = announcement.render();

        if !speech.is_empty() {
            self.speech.speak(&speech, true);
            Some(speech)
        } else {
            None
        }
    }

    /// A new active window: its title, or what its application calls it, then the control.
    /// A dialog box adds its role and its text [obs, 306/s1, s3, s10]; a property sheet the
    /// page it opens on (JAWS 2025 on the dev box: "Local Disk (C:) Properties dialog |
    /// General page").
    fn window_heading(
        &self,
        focus: &FocusEvent,
        app: Option<&dyn crate::apps::AppModule>,
        control: Announcement,
    ) -> Announcement {
        let window_name = app.and_then(|app| app.window_name(focus));
        let win = window_name
            .as_deref()
            .or(focus.window_title.as_deref())
            .unwrap_or("");
        let mut lines = vec![match &focus.dialog_text {
            Some(_) => Line::new(vec![
                Part::Text(win.to_owned()),
                Part::Role("dialog".to_owned()),
            ]),
            None => Line::text(win),
        }];
        lines.extend(
            focus
                .dialog_text
                .as_deref()
                .filter(|text| !text.is_empty())
                .map(Line::text),
        );
        lines.extend(focus.dialog_page.as_deref().map(|page| {
            Line::new(vec![
                Part::Text(page.to_owned()),
                Part::Role("page".to_owned()),
            ])
        }));
        // The control, unless it only repeats the window's name.
        let control_speech = control.render();
        if !control_speech.is_empty() && control_speech != win {
            lines.extend(control.lines);
        }
        Announcement::new(lines)
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
                    speech.push_str(&crate::native_focus::focus_speech(focus, None).render());
                }
                self.speech.speak(&speech, true);
                Some(speech)
            }
        }
    }
}
