//! Reading commands: the JAWS script names `execute_command` runs, the JAWS-key chords, and
//! the commands that need more than the buffer (Say Font, Virtualize Window, the clock).

use super::{
    ActiveElementList, AppMode, PendingViewer, Press, ScreenReaderEngine, DOUBLE_PRESS_MS,
};
use crate::aom::Role;
use crate::command::{Command, Element};
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Runs the command a name calls ([`Command::from_name`]): a JAWS script name or the key
    /// notation of JAWS's key maps. Speaks the result and returns it.
    pub fn execute_command(&mut self, name: &str) -> Option<String> {
        self.execute(Command::from_name(name)?)
    }

    /// Runs `command`, speaks what it reads, and returns it.
    pub fn execute(&mut self, command: Command) -> Option<String> {
        match command {
            Command::SayWindow => {
                // Dialog boxes; other windows are not read this way yet.
                let focus = self
                    .last_native_focus
                    .as_ref()
                    .filter(|f| !f.window_items.is_empty())?;
                let speech = crate::native_focus::say_window(
                    focus.window_title.as_deref().unwrap_or_default(),
                    &focus.window_items,
                );
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            // A page's focus is said as it was when it got the focus, without moving the
            // virtual cursor: "Document content edit | Contains text" (JAWS in Google Docs,
            // dev box 2026-10-16).
            // In a menu, its item (JAWS in Google Docs' menu: "menu | Accessibility | 10 of 10";
            // the position is not known here).
            Command::SayCurrentFocus if self.menus.current().is_some() => {
                let speech = self.menus.current()?.to_owned();
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SayCurrentFocus if self.app_mode == AppMode::VirtualBuffer => {
                let id = self.last_page_focus?;
                let (offset, node) = (self.cursor.offset, self.cursor.current_node_id);
                let speech = self.cursor.focus_node(id);
                self.cursor.offset = offset;
                self.cursor.current_node_id = node;
                let speech = speech.filter(|s| !s.trim().is_empty())?;
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SayCurrentFocus => {
                // Native controls.
                let speech = self
                    .last_native_focus
                    .as_ref()
                    .filter(|_| self.app_mode == AppMode::NativeControl)
                    .map(crate::native_focus::say_focus)
                    .filter(|s| !s.is_empty())?;
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            // "Enter key to pass through" (JAWS on the dev box, 2026-10-17, WhatsApp).
            Command::PassKeyThrough => {
                self.pass_next_key = true;
                let speech = msg::pass_key_through().to_string();
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SayWindowTitle => {
                // The Virtual Viewer is a window of its own: JAWS names it, whatever it shows
                // ("Virtual Viewer", without "Title is"; dev box, Windows Terminal and Notepad,
                // 2026-10-15).
                if self.viewer.is_some() {
                    let speech = msg::viewer_title().to_string();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                // [obs, 306/s12]: "Title is | SR Dialog Fixture". In a browser it is the browser
                // window's title, not the page's [obs, 209/s1: "... - Google Chrome - <profile>"].
                let title = self.last_window_title.clone().or_else(|| {
                    (self.app_mode == AppMode::VirtualBuffer)
                        .then(|| self.cursor.buffer.document_title().map(str::to_owned))
                        .flatten()
                })?;
                let speech = format!("{}\n{title}", msg::window_title_is());
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SaySystemTime
            | Command::SaySystemTimeWithSeconds
            | Command::SaySystemDate
            | Command::SayWeekNumber => {
                let speech = self.clock_command(&command)?;
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::VirtualizeWindow => return self.virtualize(true),
            Command::VirtualizeControl => return self.virtualize(false),
            Command::QuickSettings | Command::SettingsCenter => return self.open_settings(),
            Command::SayFont => return self.say_font(false),
            Command::SayFontInViewer => return self.say_font(true),
            _ => {}
        }
        // On the desktop and the taskbar, Windows moves and the focus is read: the page
        // commands below have nothing to act on.
        if matches!(self.app_mode, AppMode::NativeDesktop | AppMode::NativeShell) {
            return None;
        }
        // An element list (Insert+F6, F7, F5) takes the arrows, Enter and Escape.
        if let Some(ref mut list) = self.active_element_list {
            match command {
                Command::SayNextLine => {
                    let speech = list.move_next();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                Command::SayPriorLine => {
                    let speech = list.move_prior();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                Command::Enter => {
                    let target_offset = list.selected_offset();
                    self.active_element_list = None;
                    let speech = self.cursor.jump_to_element_offset(target_offset);
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                Command::Escape => {
                    self.active_element_list = None;
                    let speech = msg::cursor_virtual_pc().to_string();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                _ => {}
            }
        }

        match &command {
            Command::Find(query) => {
                let speech = self.cursor.find_text(query, false);
                if let Some(ref text) = speech {
                    self.speech.speak(text, true);
                }
                return speech;
            }
            Command::JumpToHeading(target) => {
                let list = self.cursor.buffer.get_headings_list();
                let item = list
                    .iter()
                    .find(|h| h.text.eq_ignore_ascii_case(target) || h.text.contains(target))?;
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::JumpToLink(target) => {
                let list = self.cursor.buffer.get_links_list();
                let item = list
                    .iter()
                    .find(|l| l.text.eq_ignore_ascii_case(target) || l.text.contains(target))?;
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::JumpToFormField(target) => {
                let list = self.cursor.buffer.get_form_fields_list();
                let item = list
                    .iter()
                    .find(|f| f.name.eq_ignore_ascii_case(target) || f.name.contains(target))?;
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            _ => {}
        }

        let speech = match &command {
            Command::SayNextLine => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_next_option()
                } else {
                    self.cursor.move_next_line()
                }
            }
            Command::SayPriorLine => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_prior_option()
                } else {
                    self.cursor.move_prior_line()
                }
            }
            Command::SayNextWord => self.cursor.move_next_word(),
            Command::SayPriorWord => self.cursor.move_prior_word(),
            Command::SayNextCharacter => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_expand_or_next(),
                    Some(Role::Slider) => self.cursor.slider_increase(),
                    _ => self.cursor.move_next_char(),
                }
            }
            Command::SayPriorCharacter => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_collapse_or_parent(),
                    Some(Role::Slider) => self.cursor.slider_decrease(),
                    _ => self.cursor.move_prior_char(),
                }
            }
            Command::SayNextParagraph => self.cursor.move_next_paragraph(),
            Command::SayPriorParagraph => self.cursor.move_prior_paragraph(),
            Command::TopOfFile => Some(self.cursor.move_to_start()),
            Command::BottomOfFile => Some(self.cursor.move_to_end()),

            // Quick navigation, JAWS's Quick Navigation Keys
            // (docs/references/jaws-quick-navigation-keys.md).
            Command::MoveToNext(element) => self.move_to_element(*element, true),
            Command::MoveToPrior(element) => self.move_to_element(*element, false),
            Command::MoveToMainRegion => self.cursor.move_to_main_region(),

            // Tables
            Command::TableCellRight => self.cursor.move_table_cell_right(),
            Command::TableCellLeft => self.cursor.move_table_cell_left(),
            Command::TableCellDown => self.cursor.move_table_cell_down(),
            Command::TableCellUp => self.cursor.move_table_cell_up(),
            Command::TableFirstCell => self.cursor.move_table_first_cell(),
            Command::TableLastCell => self.cursor.move_table_last_cell(),
            Command::MoveToStartOfRow => self.cursor.move_table_row_start(),
            Command::MoveToEndOfRow => self.cursor.move_table_row_end(),
            Command::MoveToTopOfColumn => self.cursor.move_table_col_top(),
            Command::MoveToBottomOfColumn => self.cursor.move_table_col_bottom(),
            Command::SayCurrentCell => self.cursor.read_current_cell(),
            Command::SayColumnHeader => self.cursor.read_current_col_header(),
            Command::SayRowHeader => self.cursor.read_current_row_header(),

            Command::EnterMathViewer => self.cursor.enter_math_viewer(),
            Command::ExpandComboBox => self.cursor.expand_combobox(),
            Command::CollapseComboBox => self.cursor.collapse_combobox(),

            // Interactive navigation & actions
            Command::NextTabStop => self.cursor.move_next_tab_stop(),
            Command::PriorTabStop => self.cursor.move_prior_tab_stop(),
            Command::Activate => {
                if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
                    let is_link = matches!(node.role, Role::Link { .. });
                    if node.role.is_interactive_control() && !is_link && actions.activate(&node) {
                        // The page's own state change and focus events report the result.
                        return Some(String::new());
                    }
                }
                None
            }
            Command::Enter => {
                let node = self.node_under_cursor();
                let same_page_link = node
                    .as_ref()
                    .is_some_and(|n| n.attributes.get("same_page").is_some_and(|v| v == "true"));
                if let (Some(actions), Some(node)) = (&self.actions, node) {
                    let handled = match node.role {
                        // Forms mode on (JAWS plays a sound) and real focus in the field; the
                        // browser's focus event then speaks it.
                        Role::Edit | Role::ComboBox => {
                            let focused = actions.focus(&node);
                            if focused {
                                self.forms_mode = true;
                                self.cursor.forms_mode_active = true;
                                self.forms_mode_by_enter = true;
                            }
                            focused
                        }
                        Role::Link { .. } => !same_page_link && actions.activate(&node),
                        Role::Button
                        | Role::MenuButton
                        | Role::CheckBox
                        | Role::RadioButton
                        | Role::Tab => actions.activate(&node),
                        _ => false,
                    };
                    if handled {
                        return Some(String::new());
                    }
                }
                if let Some(link_speech) = self.cursor.activate_current_link() {
                    Some(link_speech)
                } else if let Some(math_speech) = self.cursor.enter_math_viewer() {
                    Some(math_speech)
                } else {
                    let res = self.cursor.enter_forms_mode();
                    if res.is_some() {
                        self.forms_mode = true;
                    }
                    res
                }
            }
            Command::Escape => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.collapse_combobox()
                } else if self.forms_mode {
                    let res = self.cursor.exit_forms_mode();
                    self.forms_mode = false;
                    res
                } else if self.cursor.is_in_modal() {
                    self.cursor.exit_modal()
                } else {
                    None
                }
            }

            // Text selection commands
            Command::SelectNextCharacter => self.cursor.select_next_char(),
            Command::SelectPriorCharacter => self.cursor.select_prior_char(),
            Command::SelectNextLine => self.cursor.select_next_line(),
            Command::SelectPriorLine => self.cursor.select_prior_line(),
            Command::SaySelectedText => Some(self.cursor.say_selection()),

            // Modal dialog commands
            Command::EnterModal => {
                let target_dlg = if let Some(node_id) = self.cursor.current_node_id {
                    self.cursor.buffer.dialog_for_node(node_id)
                } else {
                    None
                }
                .or_else(|| self.cursor.buffer.dialogs.first());

                if let Some(dlg) = target_dlg {
                    self.cursor.enter_modal(dlg.node_id)
                } else {
                    None
                }
            }
            Command::ExitModal => self.cursor.exit_modal(),

            // JAWS Say Scripts
            Command::SayAll => Some(self.cursor.say_all()),
            // A page's focus with nothing of it in the buffer (the empty element Google Sheets
            // and Docs type into) has a blank line, as JAWS says (dev box, 2026-10-16), not
            // the virtual cursor's.
            Command::SayLine
                if self.forms_mode
                    && self.app_mode == AppMode::VirtualBuffer
                    && self
                        .last_page_focus
                        .is_some_and(|id| !self.cursor.buffer.shows(id)) =>
            {
                Some(msg::blank().to_string())
            }
            Command::SayLine => Some(self.cursor.current_line()),
            Command::SayWord => Some(self.cursor.current_word()),
            Command::SayCharacter => Some(self.cursor.current_char()),

            // In-buffer find commands
            Command::VirtualFind => Some(format!(
                "{}  \u{001d} {} \u{001e} \n{}  \u{001d} {} \u{001e} \n{}",
                msg::find_dialog(),
                msg::role_dialog(),
                msg::find_dialog_field(),
                msg::role_edit_combo(),
                msg::hint_set_value_arrows_or_type()
            )),
            Command::FindNext => self.cursor.find_next(),
            Command::FindPrior => self.cursor.find_prior(),

            // Element list dialogs
            Command::SelectAHeading if self.ui.is_some() => {
                return self.open_list(crate::ui::lists::ListKind::Headings)
            }
            Command::SelectAHeading => {
                let items = self.cursor.buffer.get_headings_list();
                if items.is_empty() {
                    Some(msg::quicknav_none_headings().to_string())
                } else {
                    let speech = SpeechFormatter::format_headings_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::Headings {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            // With Okur's windows, a real Links List; without (replay, headless), the list
            // is spoken, as before.
            Command::SelectALink if self.ui.is_some() => {
                return self.open_list(crate::ui::lists::ListKind::Links)
            }
            Command::SelectALink => {
                let items = self.cursor.buffer.get_links_list();
                if items.is_empty() {
                    // [obs, 221/s1]
                    Some(msg::links_list_none().to_string())
                } else {
                    let speech = SpeechFormatter::format_links_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::Links {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            Command::SelectAFormField => {
                let items = self.cursor.buffer.get_form_fields_list();
                if items.is_empty() {
                    // Then the page, as when the focus comes back to it: "no form fields were
                    // found | Document Reading Reference Fixture document" [obs, 221/s2].
                    let page = self.cursor.buffer.document_title().map(|title| {
                        format!(
                            "\n{title}{}",
                            crate::linearize::role_marker(msg::role_document().as_str())
                        )
                    });
                    Some(format!(
                        "{}{}",
                        msg::form_fields_list_none(),
                        page.unwrap_or_default()
                    ))
                } else {
                    let speech = SpeechFormatter::format_form_fields_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::FormFields {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            // The list of a page's regions itself is not built yet; a page without regions
            // says so [obs, 221/s3].
            Command::SelectARegion if self.cursor.buffer.landmarks.is_empty() => {
                Some(msg::regions_list_none().to_string())
            }

            _ => None,
        };

        if let Some(ref text) = speech {
            // Left/Right read a character, which the synthesizer names ("space").
            let character = matches!(
                command,
                Command::SayNextCharacter | Command::SayPriorCharacter
            ) && text.chars().count() == 1;
            if character {
                self.speech.speak_character(text, true);
            } else if !text.is_empty() {
                self.speech.speak(text, true);
            }
        }

        speech
    }

    /// Whether the text field typed in is a document, which reads some keys as a document
    /// does: a native one (Windows 11 Notepad), or a page's multi-line field (Google Docs'
    /// document with braille support; JAWS on the dev box, 2026-10-16).
    pub(super) fn editing_document(&self) -> bool {
        let native = self.app_mode != AppMode::VirtualBuffer
            && self
                .last_native_focus
                .as_ref()
                .is_some_and(|f| f.role == Role::Document);
        let page = self.forms_mode
            && self.app_mode == AppMode::VirtualBuffer
            && self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.node(id))
                .is_some_and(|n| n.attributes.get("multiline").is_some_and(|m| m == "true"));
        native || page
    }

    /// Whether keys go to a text field: forms mode in a page, or a focused native edit or
    /// text document (Windows 11 Notepad, Word).
    pub(super) fn editing_text(&self) -> bool {
        // A combo box one types in is a text field too (Google Sheets' cell editor).
        (self.forms_mode
            && self.node_under_cursor().is_some_and(|n| {
                n.role == Role::Edit
                    || (n.role == Role::ComboBox
                        && n.attributes.get("editable").is_some_and(|e| e == "true"))
            }))
            || (self.app_mode != AppMode::VirtualBuffer
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| matches!(f.role, Role::Edit | Role::Document)))
    }

    /// A JAWS key command (Insert+…). In a text field the reading commands read the field;
    /// elsewhere they run as virtual cursor or application commands.
    pub(super) fn jaws_key_command(&mut self, event: &KeyEvent) {
        let layout_command = crate::command::layout_command_for(&self.settings.keyboard, event);
        let say_line = layout_command == Some(Command::SayLine)
            || crate::editing::EditKey::of(event).is_some();
        if self.editing_text() && say_line {
            // Say Line does not change the field, so no report follows: speak the field's
            // last known text now, or read it when none is known [obs, 305/s9, s15]. A
            // document (Notepad, Google Docs) gives the line alone (JAWS, dev box).
            let document = self.editing_document();
            let key = if document {
                crate::editing::EditKey::Line
            } else {
                crate::editing::EditKey::SayLine
            };
            match self.last_text.clone() {
                Some(context) => {
                    let speech = if document {
                        Some(
                            crate::editing::line_at(&context)
                                .trim_matches(|c: char| c.is_whitespace() || c == '﻿')
                                .to_owned(),
                        )
                    } else {
                        crate::editing::speech(key, None, &context, &self.text_field_line())
                    };
                    match speech {
                        // An empty line is read as its line break: "blank", kept out of
                        // JAWS's history [obs, 315, Insert+Up].
                        Some(line) if line.trim().is_empty() => {
                            self.speech.speak_character("\n", true);
                        }
                        Some(speech) => self.speech.speak(&speech, true),
                        None => {}
                    }
                }
                None => self.expect_text_change(key),
            }
            return;
        }
        // A reading command that moves first is the matching arrow key: the application's
        // own in a native control (sent, then read as any arrow key is), the virtual
        // cursor's in a page.
        let moving = match layout_command {
            Some(Command::SayNextLine) => Some(vec![Key::DownArrow]),
            Some(Command::SayPriorLine) => Some(vec![Key::UpArrow]),
            Some(Command::SayNextWord) => Some(vec![Key::Control, Key::RightArrow]),
            Some(Command::SayPriorWord) => Some(vec![Key::Control, Key::LeftArrow]),
            Some(Command::SayPriorCharacter) => Some(vec![Key::LeftArrow]),
            _ => None,
        };
        if let (Some(chord), Some(command)) = (moving, &layout_command) {
            if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
                let _ = self.execute(command.clone());
            } else if let Some(actions) = &self.actions {
                actions.press(&chord);
            }
            return;
        }
        // Say All in a document reads its lines from the caret's to the last (JAWS in Google
        // Docs' document with braille support, dev box 2026-10-16), not the page around it.
        let say_all = layout_command
            .clone()
            .or_else(|| crate::command::command_for(event))
            == Some(Command::SayAll);
        if self.editing_document() && say_all {
            let lines = self
                .last_text
                .as_ref()
                .map(crate::editing::lines_from_caret)
                .unwrap_or_default();
            for (i, line) in lines.iter().enumerate() {
                self.speech.speak(line, i == 0);
            }
            return;
        }
        if self.editing_text() && layout_command == Some(Command::SayWord) {
            let word = self
                .last_text
                .as_ref()
                .map(crate::editing::word_around)
                .unwrap_or_default();
            if !word.trim().is_empty() {
                self.speech.speak(word.trim(), true);
            }
            return;
        }
        if let Some(command) = layout_command.or_else(|| crate::command::command_for(event)) {
            let command = if self.pressed_twice(Press::Command(command.clone()), event.time_ms) {
                command.pressed_twice().unwrap_or(command)
            } else {
                command
            };
            let _ = self.execute(command);
        }
    }

    /// Whether `press` repeats the last press quickly enough to be a double press
    /// ([`Command::pressed_twice`]). A double press is not the first press of another.
    fn pressed_twice(&mut self, press: Press, time_ms: u64) -> bool {
        let twice = matches!(&self.last_press, Some((last, at))
            if *last == press && time_ms != 0 && time_ms.saturating_sub(*at) <= DOUBLE_PRESS_MS);
        self.last_press = (!twice).then_some((press, time_ms));
        twice
    }

    /// CapsLock as the JAWS key: a single press is the JAWS key; pressed twice quickly it
    /// toggles Caps Lock (the second press reaches the system), which JAWS announces
    /// [obs, 310: "Caps lock on", then "Caps lock off"].
    pub(super) fn caps_lock_press(&mut self, event: &KeyEvent) -> KeyAction {
        if !self.pressed_twice(Press::CapsLock, event.time_ms) {
            return KeyAction::Consume;
        }
        let was_on = event.caps_lock_on.unwrap_or(self.caps_lock_on);
        self.caps_lock_on = !was_on;
        let speech = if was_on {
            msg::caps_lock_off()
        } else {
            msg::caps_lock_on()
        };
        self.speech.speak(speech.as_str(), true);
        KeyAction::PassThrough
    }

    /// Say System Time and its variants, from the clock in the user's locale formats.
    fn clock_command(&self, command: &Command) -> Option<String> {
        let clock = self.clock.as_deref();
        match command {
            Command::SaySystemTime => Some(crate::clock::say_time(clock?, false)),
            Command::SaySystemTimeWithSeconds => Some(crate::clock::say_time(clock?, true)),
            Command::SaySystemDate => Some(crate::clock::say_date(clock?, false)),
            Command::SayWeekNumber => Some(crate::clock::say_date(clock?, true)),
            _ => None,
        }
    }

    /// Quick navigation to the next or prior `element`.
    fn move_to_element(&mut self, element: Element, next: bool) -> Option<String> {
        let c = &mut self.cursor;
        match (element, next) {
            (Element::Heading, true) => c.move_next_heading(),
            (Element::Heading, false) => c.move_prior_heading(),
            (Element::HeadingLevel(level), true) => c.move_next_heading_level(u32::from(level)),
            (Element::HeadingLevel(level), false) => c.move_prior_heading_level(u32::from(level)),
            (Element::Link, true) => c.move_next_link(),
            (Element::Link, false) => c.move_prior_link(),
            (Element::Table, true) => c.move_next_table(),
            (Element::Table, false) => c.move_prior_table(),
            (Element::Article, true) => c.move_next_article(),
            (Element::Article, false) => c.move_prior_article(),
            (Element::OnMouseOver, true) => c.move_next_on_mouse_over(),
            (Element::OnMouseOver, false) => c.move_prior_on_mouse_over(),
            (Element::Tab, true) => c.move_next_tab(),
            (Element::Tab, false) => c.move_prior_tab(),
            // Place markers are not built yet, so a page has none [obs, 221/s4-s5].
            (Element::PlaceMarker, _) => Some(msg::quicknav_none_placemarkers().to_string()),
            (Element::Graphic, true) => c.move_next_graphic(),
            (Element::Graphic, false) => c.move_prior_graphic(),
            (Element::RadioButton, true) => c.move_next_radio_button(),
            (Element::RadioButton, false) => c.move_prior_radio_button(),
            (Element::Math, true) => c.move_next_math(),
            (Element::Math, false) => c.move_prior_math(),
            (Element::FormField, true) => c.move_next_form_control(),
            (Element::FormField, false) => c.move_prior_form_control(),
            (Element::Button, true) => c.move_next_button(),
            (Element::Button, false) => c.move_prior_button(),
            (Element::Edit, true) => c.move_next_edit(),
            (Element::Edit, false) => c.move_prior_edit(),
            (Element::CheckBox, true) => c.move_next_checkbox(),
            (Element::CheckBox, false) => c.move_prior_checkbox(),
            (Element::ComboBox, true) => c.move_next_combobox(),
            (Element::ComboBox, false) => c.move_prior_combobox(),
            (Element::List, true) => c.move_next_list(),
            (Element::List, false) => c.move_prior_list(),
            (Element::ListItem, true) => c.move_next_list_item(),
            (Element::ListItem, false) => c.move_prior_list_item(),
            (Element::Landmark, true) => c.move_next_landmark(),
            (Element::Landmark, false) => c.move_prior_landmark(),
            (Element::Frame, true) => c.move_next_frame(),
            (Element::Frame, false) => c.move_prior_frame(),
        }
    }

    /// Say Font (`Insert+F`): the font at the cursor. Pressed twice quickly, JAWS shows it in
    /// the Virtual Viewer and reads the message [obs, 209/s7, 314/s2]; with no font it just
    /// says "No font" again (313/s2). A native field reports its font at the caret (UIA text
    /// attributes) or its window's font; JAWS's size for classic Notepad (314/s1, "22
    /// point") awaits a trace that recorded the font.
    fn say_font(&mut self, in_viewer: bool) -> Option<String> {
        // A page that draws its text itself shows the font in its own controls (Google Docs'
        // toolbar): they are read as they are now, and the font is said when they arrive
        // (`say_page_font`).
        if self.app_mode == AppMode::VirtualBuffer && self.forms_mode {
            let elements = self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.node(id))
                .and_then(|focus| focus.attributes.get("id"))
                .zip(self.page_app())
                .map(|(id, app)| app.font_elements(id))
                .unwrap_or_default();
            if !elements.is_empty()
                && self
                    .actions
                    .as_ref()
                    .is_some_and(|a| a.read_page_elements(elements))
            {
                self.pending_font = Some(in_viewer);
                return None;
            }
        }
        let formatting = if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
            self.cursor.buffer.formatting_at_offset(self.cursor.offset)
        } else {
            // A text field reports the font at its caret.
            self.last_text
                .as_ref()
                .and_then(|t| t.font.as_ref())
                .map(crate::event::FontInfo::formatting)
        };
        self.speak_font(formatting, in_viewer)
    }

    /// Says the font that the page's own controls show, read for Say Font (`pending_font`).
    /// Elements nobody waits for say nothing.
    pub(super) fn say_page_font(&mut self, elements: &[crate::aom::AomNode]) -> Option<String> {
        let in_viewer = self.pending_font.take()?;
        let formatting = self
            .page_app()
            .and_then(|app| app.font_from_elements(elements));
        self.speak_font(formatting, in_viewer)
    }

    /// Says `formatting` as Say Font does, or shows it in the Virtual Viewer (`in_viewer`).
    fn speak_font(
        &mut self,
        formatting: Option<crate::aom::TextFormatting>,
        in_viewer: bool,
    ) -> Option<String> {
        let description = formatting
            .as_ref()
            .and_then(SpeechFormatter::font_description);
        // A native text field that reports its alignment adds its paragraph formatting, after
        // a comma (JAWS 2025 in Windows 11 Notepad, dev box: "Font is Consolas 11 point, |
        // Paragraph formatting left justified"), and so does a page's field (Google Docs'
        // document with braille support, dev box 2026-10-16); the virtual cursor (209/s3) and
        // fields known only by their window font (314/s1) do not.
        let in_browse_mode = self.app_mode == AppMode::VirtualBuffer && !self.forms_mode;
        let paragraph = formatting
            .as_ref()
            .and_then(|f| f.alignment.as_deref())
            .filter(|_| !in_browse_mode)
            .map(SpeechFormatter::paragraph_formatting);
        let description = match (description, &paragraph) {
            (Some(font), Some(_)) => Some(format!("{font},")),
            (font, _) => font,
        };
        let speech = match (in_viewer, description) {
            (true, Some(font)) if self.viewer.is_none() => {
                let viewer = crate::viewer::VirtualViewer::message(vec![
                    msg::viewer_font_information().to_string(),
                    font,
                    paragraph.unwrap_or_default(),
                    String::new(),
                    format!(
                        "\u{1d} {} \u{1e} {}",
                        msg::role_link(),
                        msg::viewer_hot_keys_link()
                    ),
                    msg::viewer_close_hint().to_string(),
                ]);
                let speech = viewer.message_speech();
                self.viewer = Some(viewer);
                self.show_viewer_window();
                speech
            }
            (_, Some(font)) => match paragraph {
                Some(paragraph) => format!("{}\n{paragraph}", msg::font_is(font)),
                None => msg::font_is(font).render(),
            },
            (_, None) => SpeechFormatter::format_font(None),
        };
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Virtualize Window or Virtualize Current Control: opens the Virtual Viewer on the
    /// window's or the control's text and reads its first line [obs, 311/s1, s14; 312/s1].
    fn virtualize(&mut self, window: bool) -> Option<String> {
        // A page's virtual cursor is a virtual view already: JAWS virtualizes neither the
        // window nor the control (observed on the dev box with JAWS 2025 in Chrome,
        // 2026-10-15; scenario 318 records it on old-pc).
        let in_page = self.app_mode == AppMode::VirtualBuffer && !self.forms_mode;
        if self.viewer.is_some() || in_page {
            // [obs, 311/s32]
            let speech = msg::viewer_already_virtual().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        // A console's text is its buffer as it is now: it is read fresh, and the viewer opens
        // when it arrives (`open_console_viewer`).
        let console = self
            .last_native_focus
            .as_ref()
            .and_then(|focus| self.app_for(focus))
            .is_some_and(|app| app.virtualizes_text());
        if console && self.actions.as_ref().is_some_and(|a| a.read_text()) {
            self.pending_viewer = Some(PendingViewer::Console { window });
            return None;
        }
        // A text field's text is its lines: the control's, and the window's when the text is
        // what the window is for (Notepad, where JAWS shows only "paragraph"). A field among
        // others (a form's Name field) is one row of its window. Anything else is read as it
        // is on screen, and the viewer opens when it arrives (`open_screen_viewer`).
        let text_field = self.editing_text() && (!window || self.text_is_the_window());
        let native = self.app_mode != AppMode::VirtualBuffer;
        if native && !text_field && self.actions.as_ref().is_some_and(|a| a.read_window(window)) {
            self.pending_viewer = Some(PendingViewer::Screen { window });
            return None;
        }
        let lines = self.virtualized_lines(window)?;
        self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
    }

    /// Whether the focused text is what its window shows: a document, or a field of several
    /// lines outside a dialog box (an editor's page, not a form's field).
    fn text_is_the_window(&self) -> bool {
        let Some(focus) = self.last_native_focus.as_ref() else {
            return false;
        };
        let dialog = focus.in_dialog_box;
        let lines = self
            .last_text
            .as_ref()
            .is_some_and(|t| t.text.contains(['\r', '\n']));
        focus.role == Role::Document || (lines && !dialog)
    }

    /// Opens the Virtual Viewer that waits for the screen's text (`pending_viewer`). With
    /// nothing read, it shows what the focus says, as without a screen read.
    pub(super) fn open_screen_viewer(&mut self, lines: &[String]) -> Option<String> {
        let Some(PendingViewer::Screen { window }) = self.pending_viewer else {
            return None;
        };
        self.pending_viewer = None;
        let lines = if lines.is_empty() {
            self.virtualized_lines(window)?
        } else {
            lines.to_vec()
        };
        self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
    }

    /// Opens `viewer` and speaks what JAWS speaks: what was virtualized, and the line the
    /// viewer is on.
    fn open_viewer(
        &mut self,
        viewer: crate::viewer::VirtualViewer,
        window: bool,
    ) -> Option<String> {
        let what = if window {
            msg::viewer_virtualizing_window()
        } else {
            msg::viewer_virtualizing_control()
        };
        let speech = format!("{what}\n{}", viewer.line_text());
        self.viewer = Some(viewer);
        self.show_viewer_window();
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Opens the Virtual Viewer that waits for a console's text (`pending_viewer`), at the
    /// cursor's line. Returns `None` when no viewer is waiting.
    pub(super) fn open_console_viewer(
        &mut self,
        context: &crate::event::TextContext,
    ) -> Option<Option<String>> {
        let Some(PendingViewer::Console { window }) = self.pending_viewer else {
            return None;
        };
        self.pending_viewer = None;
        self.last_text = Some(context.clone());
        // The application that virtualizes its text lays the lines out (a console).
        let (lines, line) = self
            .last_native_focus
            .as_ref()
            .and_then(|focus| {
                self.app_for(focus)?
                    .viewer_lines(focus, &context.text, context.caret)
            })
            .unwrap_or_default();
        if lines.is_empty() {
            // Nothing read: the viewer shows the control, as for any other.
            let lines = self.virtualized_lines(window);
            return Some(lines.and_then(|lines| {
                self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
            }));
        }
        let viewer = crate::viewer::VirtualViewer::at_line(lines, line);
        Some(self.open_viewer(viewer, window))
    }

    /// The Virtual Viewer's lines for the focused native control or its window:
    /// - a dialog box: its title, its text, and its buttons on one line [obs, 311/s14–s22];
    /// - a text field: its lines, after the window title for the whole window [obs, 312];
    /// - another control: its name and type, then a blank line [obs, 311/s1–s11].
    fn virtualized_lines(&self, window: bool) -> Option<Vec<String>> {
        let focus = self.last_native_focus.as_ref()?;
        let title = focus.window_title.clone().unwrap_or_default();
        if window && focus.in_dialog_box {
            let mut lines = vec![title];
            // As laid out on screen: wrapped text by its rows, a label beside its field
            // (JAWS on the Run dialog: "Open: C:", "OK Cancel Browse...").
            if !focus.dialog_lines.is_empty() {
                lines.extend(focus.dialog_lines.iter().cloned());
                return Some(lines);
            }
            lines.extend(
                focus
                    .dialog_text
                    .iter()
                    .flat_map(|t| t.lines())
                    .map(str::to_owned),
            );
            if !focus.dialog_buttons.is_empty() {
                lines.push(focus.dialog_buttons.join(" "));
            }
            return Some(lines);
        }
        if self.editing_text() {
            if let Some(text) = &self.last_text {
                let mut lines: Vec<String> = Vec::new();
                if window {
                    lines.push(title);
                }
                lines.extend(crate::editing::text_lines(&text.text));
                return Some(lines);
            }
        }
        let name = focus.name.clone().unwrap_or_default();
        let control = match crate::native_focus::role_word(&focus.role) {
            Some(role) => format!("{name} {role}"),
            None => name,
        };
        Some(vec![control, " ".to_owned()])
    }

    /// What JAWS reads when the Virtual Viewer closes: the text field's line, or the focused
    /// control's name and type [obs, 311/s12, s23; 312/s11].
    pub(super) fn speak_focus_again(&mut self) {
        let text_app = self
            .last_native_focus
            .as_ref()
            .and_then(|focus| self.app_for(focus))
            .filter(|app| app.virtualizes_text());
        let speech = if let Some(app) = text_app {
            // A console: its cursor's line, as last read (JAWS on the dev box, closing the
            // viewer: "C:\Users\Gigabyte>").
            self.last_text
                .as_ref()
                .and_then(|text| app.line_after_viewer(&text.text, text.caret))
        } else if self.editing_text() {
            let line = self.last_text.as_ref().map(crate::editing::line_at);
            // A combo box's field adds its list position (JAWS on the Run dialog, dev box:
            // "C: | 1 of 6").
            let position = self
                .last_native_focus
                .as_ref()
                .filter(|f| f.container_role == Some(Role::ComboBox))
                .and_then(|f| Some(msg::position(f.item_index?, f.item_count?).render()));
            match (line, position) {
                (Some(line), Some(position)) => Some(format!("{line}\n{position}")),
                (line, _) => line,
            }
        } else if self.app_mode == AppMode::VirtualBuffer {
            // In a page, the virtual cursor's line [obs, 209/s11].
            Some(self.cursor.current_line())
        } else {
            self.last_native_focus
                .as_ref()
                .map(crate::native_focus::focus_line)
        };
        if let Some(speech) = speech.filter(|s| !s.is_empty()) {
            self.speech.speak(&speech, true);
        }
    }
}
