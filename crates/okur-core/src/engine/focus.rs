//! Focus and menu events: what the engine says when the focus or a menu changes.

use super::{AppMode, ScreenReaderEngine};
use crate::announcement::{Announcement, Line, Part};
use crate::aom::Role;
use crate::apps::{AppContext, FocusResponse};
use crate::event::FocusEvent;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// An option the focused field points at, said as JAWS says it (dev box, 2026-10-16): its
    /// list on the way in ("Fruits list | with 3 items"), then "Apple | 1 of 3 | To move to an
    /// item press the Arrow keys."
    fn speak_active_option(&mut self, focus: &FocusEvent) -> Option<String> {
        let list = focus.container_name.clone().unwrap_or_default();
        let mut lines = Vec::new();
        if self.option_list.as_deref() != Some(list.as_str()) {
            if let Some(count) = focus.item_count {
                lines.push(format!("{list}  \u{1d} list \u{1e} \nwith {count} items"));
            }
        }
        self.option_list = Some(list);
        lines.push(focus.name.clone().unwrap_or_default());
        if let (Some(index), Some(count)) = (focus.item_index, focus.item_count) {
            lines.push(format!("{index} of {count}"));
        }
        lines.push("To move to an item press the Arrow keys.".to_owned());
        let speech = lines.join("\n");
        self.speech.speak(&speech, true);
        Some(speech)
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
            // Where the virtual cursor was, for the containers the focus enters.
            let cursor_offset = self.cursor.offset;
            let cursor_node = self
                .cursor
                .buffer
                .line_at_offset(cursor_offset)
                .map(|l| l.node_id);
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
            // An option the focused field points at (aria-activedescendant: an autocomplete's
            // list, an editor's completions) is the field's: forms mode stays on.
            if self.forms_mode && focus.role == Role::ListItem {
                return self.speak_active_option(focus);
            }
            let from_options = self.option_list.take().is_some();
            // Into a page's application region (ARIA role "application"): its name, and every
            // key goes to the page, as JAWS does (dev box, 2026-10-16: "Workbench | First tool
            // Button").
            let entering_application =
                focus.web_application.is_some() && focus.web_application != self.web_application;
            self.web_application = focus.web_application.clone();
            if target_role == Some(Role::Document) {
                // The page itself has focus: the next control focus enters the page.
                self.last_page_focus = None;
                self.tab_start = None;
                return None;
            }
            // Forms mode follows focus into edits; a combo box keeps the forms mode Enter
            // turned on [obs, 205/s8].
            let is_edit = target_role == Some(Role::Edit) || focus.role == Role::Edit;
            self.page_focus_is_field =
                is_edit || focus.role == Role::ComboBox || target_role == Some(Role::ComboBox);
            let in_forms_mode = is_edit
                || (target_role == Some(Role::ComboBox) && self.forms_mode)
                || self.web_application.is_some();
            self.forms_mode = in_forms_mode;
            self.cursor.forms_mode_active = in_forms_mode;
            // Chrome can report focus on the same element again (while typing in it); JAWS
            // does not repeat the field.
            if target_id.is_some() && target_id == self.last_page_focus && !from_options {
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
                // A combo box's list and value change as it is typed in (an autocomplete).
                let combo_changed = |n: &crate::aom::AomNode| {
                    n.role == Role::ComboBox
                        && ((focus.states.expanded.is_some()
                            && n.states.expanded != focus.states.expanded)
                            || (focus.value.is_some() && n.value != focus.value))
                };
                let stale = self.cursor.buffer.node(id).is_some_and(|n| {
                    (focus.states.checked.is_some() && n.states.checked != focus.states.checked)
                        || combo_changed(n)
                });
                if stale {
                    if let Some(rebuilt) = self.cursor.buffer.with_node_changed(id, |n| {
                        if focus.states.checked.is_some() {
                            n.states.checked = focus.states.checked;
                        }
                        if n.role == Role::ComboBox {
                            n.states.expanded = focus.states.expanded.or(n.states.expanded);
                            if focus.value.is_some() {
                                n.value = focus.value.clone();
                            }
                        }
                    }) {
                        self.cursor.buffer = rebuilt;
                    }
                }
            }
            if let Some(offset) = previous_focus_offset {
                self.cursor.offset = offset;
            }
            let mut speech = target_id
                .and_then(|id| self.cursor.focus_node_from(id, Some(cursor_offset)))
                .or_else(|| focus.name.clone());
            let editable_combo = target_id
                .and_then(|id| self.cursor.buffer.node(id))
                .is_some_and(|n| n.attributes.get("editable").is_some_and(|e| e == "true"));
            if target_role == Some(Role::ComboBox) && in_forms_mode && !editable_combo {
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
            // Back in an autocomplete from its list: "Fruit edit combo collapsed | Apricot |
            // To set the value use the Arrow keys or type the value." (JAWS on the dev box).
            if from_options && target_role == Some(Role::ComboBox) {
                speech = speech.map(|s| {
                    format!("{s}\nTo set the value use the Arrow keys or type the value.")
                });
            }
            if entering_application {
                let name = focus.web_application.clone().unwrap_or_default();
                if !name.is_empty() {
                    speech = speech.map(|s| format!("{name}\n{s}"));
                }
            }
            // Lists and landmarks the focus entered from where the virtual cursor was, as
            // JAWS says them (dev box, GitHub's README tab from its section's heading:
            // "Repository files NavigationRegion | list | with 1 items | README ...").
            if !entering_page {
                if let (Some(from), Some(to)) = (cursor_node, target_id) {
                    let entered = self.entered_containers(from, to);
                    if !entered.is_empty() {
                        speech = speech.map(|s| format!("{entered}{s}"));
                    }
                }
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
        self.web_application = None;
        self.option_list = None;
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
