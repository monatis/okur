//! Focus and menu events: what the engine says when the focus or a menu changes.

use super::{AppMode, ScreenReaderEngine};
use crate::announcement::{Announcement, Line, Part};
use crate::aom::Role;
use crate::apps::{AppContext, FocusResponse};
use crate::cursor::FocusFrom;
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
        // Access keys are said only with JAWS's Access Key output on; old-pc's JAWS has it off
        // (`ACCESS_KEY=0|0|0`, 306 recorded 2026-10-18). That takes a menu item's letter
        // ("Yeni sekme Ctrl+T | 2 of 24", without "e") and a field's shortcut (Chrome's address
        // bar without "Ctrl+L") with it; a menu item's shortcut is part of the item [obs, 220].
        let without_key;
        let menu_shortcut =
            |k: &String| focus.role == Role::MenuItem && k.trim().chars().count() > 1;
        let has_key = focus.access_key.is_some()
            || focus
                .accelerator_key
                .as_ref()
                .is_some_and(|k| !menu_shortcut(k));
        let focus = if has_key && !self.settings.access_keys {
            without_key = FocusEvent {
                access_key: None,
                accelerator_key: focus.accelerator_key.clone().filter(menu_shortcut),
                ..focus.clone()
            };
            &without_key
        } else {
            focus
        };
        self.last_selected = None;
        if focus.role != Role::MenuItem {
            self.last_text = focus.text.clone();
            self.pending_edit = None;
        }
        if focus.role == Role::MenuItem {
            // A page's menu (an ARIA menu): its keys are the page's, as in forms mode, and
            // none of them reads the field focused before it. The browser's own menu (Chrome's
            // app menu over a page) leaves forms mode as it was: menu mode takes its keys.
            if focus.in_document
                && self.app_mode == AppMode::VirtualBuffer
                && self.settings.auto_forms_mode
            {
                self.forms_mode = true;
                self.cursor.forms_mode_active = true;
                self.page_focus_is_field = false;
                self.pending_edit = None;
            }
            let name = focus.name.as_deref().unwrap_or_default();
            // Without Auto Forms Mode a page's menu stays the virtual cursor's: it goes to the
            // item and says its line, after "menu" the first time and the page's name on the
            // page's first focus, then the hint: "menu | Web Menus | File menu sub menu
            // collapsed | To move through items press left or right arrow." [obs, 218/s1].
            if focus.in_document && self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
                if let Some(speech) = self.page_menu_line(focus) {
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
            }
            let speech = if focus.in_document {
                use crate::native_focus::PageMenuPlace;
                let position = focus.item_index.zip(focus.item_count);
                let place = if focus.in_menu_bar {
                    PageMenuPlace::Bar
                } else if focus.under_menu_bar {
                    PageMenuPlace::BarMenu
                } else {
                    PageMenuPlace::Popup
                };
                self.menus
                    .page_focus(name, place, position, focus.states.expanded.is_some())
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
        let leaving_menus = !focus.in_document && self.menus.in_menu_mode() && self.menus.end();
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
            // A focus still inside the page's menu (one of WhatsApp's reactions) has not left it.
            let leaving_menus = !focus.in_menu && self.menus.in_menu_mode() && self.menus.end();
            if focus.in_menu {
                self.menus.off_items();
            }
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

            // A focus the buffer does not hold: the page changed since it was captured
            // (WhatsApp's search results, built as its search box is typed in). It is captured
            // again, and the virtual cursor goes to the focus when it arrives.
            if target_id.is_none()
                && focus.role != Role::Document
                && self.actions.as_ref().is_some_and(|a| a.refresh_page())
            {
                self.cursor_to_focus = true;
            }

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
                self.focus_table_unsaid = false;
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
            // A list's, grid's or tree's item keeps the forms mode the focus brings to it:
            // Down from WhatsApp's search box into its results, then the arrows through them,
            // are the page's (JAWS on the dev box, 2026-10-17). Tab onto one turns forms mode
            // on, as on a list box: WhatsApp's messages are rows its arrows move through.
            // JAWS 2025 stays in browse mode there, and the user has to turn forms mode on
            // (dev box, 2026-10-17); Okur gives the item its keys, as the user asked.
            let auto_forms = self.settings.auto_forms_mode;
            let widget_item = (self.forms_mode || (self.last_key_tabs && auto_forms))
                && (target_id.is_some_and(|id| self.in_widget_item(id))
                    || (target_id.is_none() && focus_in_widget(focus)));
            // Enter on such an item that sends the focus elsewhere has opened something, and
            // the page put the focus there: the virtual cursor reads on from it. WhatsApp's
            // chat, opened from its list, focuses its message field; the user goes on with
            // Ctrl+End, Shift+E and Enter, typing nothing by accident.
            let opened = self
                .activated_item
                .take()
                .is_some_and(|item| Some(item) != target_id);
            // Without Auto Forms Mode a field keeps the mode it finds: forms mode is turned on by
            // Enter, and Tab from one field to the next keeps it (old-pc's JAWS, 212 and 213).
            let in_forms_mode = !opened
                && ((is_edit && (auto_forms || self.forms_mode))
                    || (target_role == Some(Role::ComboBox) && self.forms_mode)
                    || self.web_application.is_some()
                    || (focus.in_menu && auto_forms)
                    || widget_item);
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
            // A focus the buffer does not hold yet is said from the event, as the buffer would
            // say it: WhatsApp's message options button, shown once its message has the focus
            // ("Mesaj seçeneklerini aç Button collapsed | To activate press Enter.", JAWS on
            // the dev box, 2026-10-17).
            let from = if std::mem::take(&mut self.focus_table_unsaid) {
                FocusFrom::Outside
            } else {
                FocusFrom::Offset(cursor_offset)
            };
            let first_link = target_id.filter(|_| entering_page).filter(|&id| {
                self.cursor
                    .buffer
                    .node(id)
                    .is_some_and(|n| matches!(n.role, Role::Link { .. }))
            });
            let line = first_link.and_then(|id| self.cursor.focus_line(id));
            self.focus_table_unsaid = line.is_some();
            let mut speech = line
                .or_else(|| target_id.and_then(|id| self.cursor.focus_node_from(id, from)))
                .or_else(|| {
                    let mut node = crate::aom::AomNode::new(usize::MAX, focus.role.clone());
                    node.name = focus.name.clone();
                    node.value = focus.value.clone();
                    node.states = focus.states.clone();
                    let mut speech = SpeechFormatter::format_control_focus(&node, |_| false);
                    if let Some(hint) = SpeechFormatter::web_focus_hint(&node) {
                        speech.push('\n');
                        speech.push_str(hint);
                    }
                    Some(speech).filter(|s| !s.trim().is_empty())
                })
                .or_else(|| focus.name.clone());
            // A plain element that takes the focus is said by the name the page gives it then:
            // "Siz okur 1 14:46 Okundu Daha fazla seçenek için sol veya sağ ok tuşuna basarak
            // bağlam menüsüne erişin", WhatsApp's message (JAWS on the dev box, 2026-10-17).
            let plain = target_id
                .and_then(|id| self.cursor.buffer.node(id))
                .is_some_and(|n| {
                    matches!(
                        n.role,
                        Role::Group | Role::Generic | Role::Text | Role::Paragraph | Role::TableRow
                    ) && n.attributes.get("aria_role").is_none_or(|r| r == "row")
                        && self.cursor.buffer.shows(n.id)
                });
            if let Some(name) = focus
                .name
                .as_deref()
                .map(str::trim)
                .filter(|n| plain && !n.is_empty())
            {
                speech = Some(name.to_owned());
            }
            // Inside a page's menu JAWS adds no usage hint: "👍 Toggle button Not pressed"
            // (WhatsApp's reactions, dev box, 2026-10-17).
            if focus.in_menu {
                speech = speech.map(|s| without_usage_hint(&s));
            }
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
        self.focus_table_unsaid = false;
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

    /// The window in front was renamed. An application's window keeps its focus under its new
    /// name, unsaid: the Virtual Viewer's first line is the title as it is now [obs, 321/s1]. A
    /// browser's title follows its page, which the page's own events say.
    pub(super) fn window_renamed(&mut self, title: &str) {
        if self.app_mode == AppMode::VirtualBuffer || title.trim().is_empty() {
            return;
        }
        if let Some(focus) = self.last_native_focus.as_mut() {
            focus.window_title = Some(title.to_owned());
            self.last_window_title = Some(title.to_owned());
        }
    }

    /// A page's menu item in browse mode, said as its line (see the caller), or `None` when
    /// the buffer has no line of its own for it.
    fn page_menu_line(&mut self, focus: &FocusEvent) -> Option<String> {
        use crate::native_focus::PageMenuPlace;
        let id = self.page_node_for(focus)?;
        let line = self.cursor.focus_line(id)?;
        let first = !self.menus.in_menu_mode();
        let place = if focus.in_menu_bar {
            PageMenuPlace::Bar
        } else if focus.under_menu_bar {
            PageMenuPlace::BarMenu
        } else {
            PageMenuPlace::Popup
        };
        let name = focus.name.as_deref().unwrap_or_default();
        let position = focus.item_index.zip(focus.item_count);
        // The menus know where the focus is, for Escape's "Leaving menus".
        let _ = self
            .menus
            .page_focus(name, place, position, focus.states.expanded.is_some());
        let entering_page = self.last_page_focus.is_none();
        self.last_page_focus = Some(id);
        self.tab_start = Some(id);
        let mut lines = Vec::new();
        if first && place != PageMenuPlace::Popup {
            lines.push(crate::linearize::role_marker("menu"));
        }
        if entering_page {
            lines.extend(self.cursor.buffer.document_title().map(str::to_owned));
        }
        lines.push(line);
        if first {
            lines.push(
                if place == PageMenuPlace::Bar {
                    "To move through items press left or right arrow."
                } else {
                    "To move through items press up or down arrow."
                }
                .to_owned(),
            );
        }
        Some(lines.join(
            "
",
        ))
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
        // The control, unless it only repeats the window's name or the application names its
        // windows alone.
        let control_speech = control.render();
        let with_control = app.is_none_or(|app| app.names_control_with_window(focus));
        if with_control && !control_speech.is_empty() && control_speech != win {
            lines.extend(control.lines);
        }
        Announcement::new(lines)
    }

    /// Handles menu mode transitions [obs, 302]. Leaving menus re-announces the control
    /// focused before them, as on a fresh focus: `Leaving menus` | `Name: ‹ Edit ›` | …
    pub fn handle_menu_event(&mut self, event: crate::event::MenuEvent) -> Option<String> {
        use crate::event::MenuEvent;
        match event {
            MenuEvent::Start => {
                self.menus.start();
                None
            }
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
                // The control focused before the menu, when it was a native one; a page's
                // menu returns to the page, whose focus says itself.
                if let Some(focus) = self
                    .last_native_focus
                    .as_ref()
                    .filter(|_| self.app_mode != AppMode::VirtualBuffer)
                {
                    speech.push('\n');
                    speech.push_str(&crate::native_focus::focus_speech(focus, None).render());
                }
                self.speech.speak(&speech, true);
                Some(speech)
            }
        }
    }
}

/// `speech` without the usage hint a page control's focus ends with.
fn without_usage_hint(speech: &str) -> String {
    [
        "To toggle the state press spacebar.",
        "To activate press Enter.",
    ]
    .iter()
    .find_map(|hint| speech.strip_suffix(hint))
    .map(|rest| rest.trim_end_matches('\n').to_owned())
    .unwrap_or_else(|| speech.to_owned())
}

/// Whether a focus the buffer does not hold is a list's, grid's or tree's item, by what the
/// focus event says of it: an item's role, or a plain element in such a container (WhatsApp's
/// search result, a group in a table).
fn focus_in_widget(focus: &FocusEvent) -> bool {
    matches!(
        focus.role,
        Role::TableCell | Role::TableRow | Role::ListItem | Role::TreeItem | Role::Tab
    ) || (matches!(focus.role, Role::Group | Role::Generic | Role::Text)
        && matches!(
            focus.container_role,
            Some(Role::Table | Role::List | Role::Tree | Role::TabList)
        ))
}
