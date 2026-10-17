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

    /// Says that menus were left, when nothing else is said with it.
    fn say_leaving_menus(&mut self) -> String {
        let speech = "Leaving menus".to_owned();
        self.speech.speak(&speech, true);
        speech
    }

    /// Handles an OS-level focus changed event.
    /// Synchronizes cursor position, activates/deactivates forms mode,
    /// and emits canonical JAWS speech announcements.
    pub fn handle_focus_event(&mut self, focus: &FocusEvent) -> Option<String> {
        self.last_selected = None;
        if focus.role != Role::MenuItem {
            self.last_text = focus.text.clone();
            self.pending_edit = None;
        }
        if focus.role == Role::MenuItem {
            // A page's menu (an ARIA menu): its keys are the page's, as in forms mode, and
            // none of them reads the field focused before it.
            if self.app_mode == AppMode::VirtualBuffer {
                self.forms_mode = true;
                self.cursor.forms_mode_active = true;
                self.page_focus_is_field = false;
                self.pending_edit = None;
            }
            let name = focus.name.as_deref().unwrap_or_default();
            let speech = if focus.in_document {
                let position = focus.item_index.zip(focus.item_count);
                self.menus.page_focus(
                    name,
                    focus.in_menu_bar,
                    position,
                    focus.states.expanded.is_some(),
                )
            } else if focus.item_index.is_some() && focus.item_count.is_some() && !focus.in_menu_bar
            {
                // A modern menu reports its items' positions; a Win32 one does not (302).
                self.menus.modern_focus(focus)
            } else {
                self.menus.focus(name)
            }?;
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        // A native focus outside the menus while they are active: a modern menu closed with
        // no menu-end event (File Explorer's context menu, Escape: "Leaving menus | Items View
        // MultiSelect ListBox | backlog | …"; JAWS on the dev box, 2026-10-17).
        let leaving_menus = !focus.in_document && self.menus.is_active() && self.menus.end();
        let with_leaving = |speech: String| {
            if leaving_menus {
                format!("Leaving menus\n{speech}")
            } else {
                speech
            }
        };
        // Applications first, as JAWS runs an application's script before the default one.
        let mut cx = AppContext {
            app_mode: &mut self.app_mode,
            window_title: &mut self.last_window_title,
            window_kind: &mut self.last_window_kind,
            selecting: self.last_key_selects,
        };
        for app in &mut self.apps {
            match app.focus(focus, &mut cx) {
                FocusResponse::Pass => {}
                FocusResponse::Silent if leaving_menus => {
                    let speech = "Leaving menus".to_owned();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                FocusResponse::Silent => return None,
                FocusResponse::Speak(announcement) => {
                    let speech = with_leaving(announcement.render());
                    self.speech.speak(&speech, true);
                    // The focus is still the last one native focus knows, for the container
                    // the next one is in: File Explorer's column header after its list's item
                    // is within that list (JAWS on the dev box, 2026-10-17: "Name split
                    // button").
                    if !focus.in_document {
                        let _ = self.native_focus.focus(focus);
                    }
                    return Some(speech);
                }
            }
        }

        let is_in_document = focus.in_document
            || focus
                .node_id
                .is_some_and(|id| self.cursor.buffer.contains_node(id));

        if is_in_document {
            // A page's menu closed with the focus back in the page: a browser ends no menu
            // mode (no menu-end event), and JAWS says it left menus (Google Docs, dev box,
            // 2026-10-16).
            let leaving_menus = self.menus.is_active() && self.menus.end();
            // Back in the page from the browser's own controls in the same window (Escape in
            // the address bar).
            let from_browser_ui = self.app_mode == AppMode::NativeControl
                && focus.window_title.is_some()
                && self.last_window_title.as_deref() == focus.window_title.as_deref();
            // Another tab's page (Ctrl+Tab, Ctrl+W): the window's title changed with it.
            // A page just loaded has been named already: its load keeps the page's title as the
            // window's.
            let tab_switched = focus.window_title.is_some()
                && self.last_window_title.is_some()
                && self.last_window_title.as_deref() != focus.window_title.as_deref()
                && self.last_window_title.as_deref() != focus.name.as_deref();
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
                // JAWS names the page coming back to it: "Web Menus document" (Escape in
                // Chrome's address bar), and another tab's page with the window then ("Web
                // Menus document | Web Menus - Google Chrome", Ctrl+Tab and Ctrl+W; dev box,
                // 2026-10-17). The page is named by its document, which the buffer may not
                // hold yet.
                if (from_browser_ui || tab_switched) && !leaving_menus {
                    let title = focus
                        .name
                        .clone()
                        .filter(|n| !n.trim().is_empty())
                        .or_else(|| self.cursor.buffer.document_title().map(str::to_owned));
                    if let Some(title) = title {
                        let mut speech =
                            format!("{title}{}", crate::linearize::role_marker("document"));
                        if let Some(window) = focus.window_title.as_deref().filter(|_| tab_switched)
                        {
                            speech.push('\n');
                            speech.push_str(window);
                        }
                        self.speech.speak(&speech, true);
                        return Some(speech);
                    }
                }
                return leaving_menus.then(|| self.say_leaving_menus());
            }
            // A document the buffer does not hold: a frame inside the page, which the focus
            // passes through on its way to an element (Google Docs' frame for typing, after
            // its menus close). The page keeps its focus.
            if focus.role == Role::Document && target_id.is_none() {
                return leaving_menus.then(|| self.say_leaving_menus());
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
            // does not repeat the field, except after leaving menus: "Leaving menus | Document
            // content edit | Contains text" (Google Docs with braille support, dev box
            // 2026-10-16). An element with nothing to say is not repeated then either (Docs'
            // empty element for typing: "Leaving menus" alone).
            let nothing_to_say = target_id
                .and_then(|id| self.cursor.buffer.node(id))
                .is_some_and(|n| {
                    n.name.as_deref().unwrap_or_default().trim().is_empty()
                        && !self.cursor.buffer.shows(n.id)
                });
            if target_id.is_some()
                && target_id == self.last_page_focus
                && !from_options
                && (!leaving_menus || nothing_to_say)
            {
                return leaving_menus.then(|| self.say_leaving_menus());
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
                // A combo box's list and value change as it is typed in (an autocomplete), and
                // its name can too (Google Sheets' cell editor is named by its cell, "E1").
                let combo_changed = |n: &crate::aom::AomNode| {
                    n.role == Role::ComboBox
                        && ((focus.states.expanded.is_some()
                            && n.states.expanded != focus.states.expanded)
                            || (focus.value.is_some() && n.value != focus.value)
                            || (focus.name.is_some() && n.name != focus.name))
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
                            if focus.name.is_some() {
                                n.name = focus.name.clone();
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
            // To set the value use the Arrow keys or type the value." (JAWS on the dev box),
            // and into a combo box one types in: Google Sheets' cell editor, "D1 edit combo |
            // n | To set the value use the Arrow keys or type the value." (dev box, 2026-10-16).
            if (from_options || editable_combo) && target_role == Some(Role::ComboBox) {
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
            if leaving_menus {
                speech = Some(match speech {
                    Some(s) if !s.is_empty() => format!("Leaving menus\n{s}"),
                    _ => "Leaving menus".to_owned(),
                });
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
        let Some(control) = self.native_focus.focus(focus) else {
            return leaving_menus.then(|| self.say_leaving_menus());
        };
        self.last_native_focus = Some(focus.clone());

        let app = self.app_for(focus);
        let names_window = app.is_none_or(|app| app.names_window(focus));
        let announcement = if window_changed && names_window {
            self.window_heading(focus, app, control)
        } else {
            control
        };
        let speech = with_leaving(announcement.render());

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
