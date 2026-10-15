//! Reading commands: the JAWS script names `execute_command` runs, the JAWS-key chords, and
//! the commands that need more than the buffer (Say Font, Virtualize Window, the clock).

use super::{ActiveElementList, AppMode, PendingViewer, ScreenReaderEngine, DOUBLE_PRESS_MS};
use crate::aom::Role;
use crate::input::{Key, KeyAction, KeyEvent};
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Executes a screen reader navigation command string (matching JAWS script names and keystrokes).
    /// Speaks the resulting announcement and returns the emitted speech text.
    pub fn execute_command(&mut self, command: &str) -> Option<String> {
        if command == "SayWindow" {
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
        if command == "SayCurrentFocus" {
            // Native controls; a page's focus is not read this way yet.
            let speech = self
                .last_native_focus
                .as_ref()
                .filter(|_| self.app_mode == AppMode::NativeControl)
                .map(crate::native_focus::say_focus)
                .filter(|s| !s.is_empty())?;
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        if command == "SayWindowTitle" {
            // The Virtual Viewer is a window of its own: JAWS names it, whatever it shows
            // ("Virtual Viewer", without "Title is"; dev box, Windows Terminal and Notepad,
            // 2026-10-15).
            if self.viewer.is_some() {
                let speech = "Virtual Viewer".to_owned();
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
            let speech = format!("Title is\n{title}");
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        if let Some(speech) = self.clock_command(command) {
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        if command == "VirtualizeWindow" || command == "VirtualizeControl" {
            return self.virtualize(command == "VirtualizeWindow");
        }
        if command == "Insert+F" || command == "SayFontInViewer" {
            return self.say_font(command == "SayFontInViewer");
        }
        // Native desktop navigation mode handling
        if self.app_mode == AppMode::NativeDesktop {
            match command {
                "{DOWN}" | "{RIGHT}" | "DesktopNextItem" => {
                    if let Some(ref mut ds) = self.desktop_state {
                        if let Some(item) = ds.move_next() {
                            let speech = SpeechFormatter::format_desktop_item(
                                &item.name, item.index, item.total,
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                "{UP}" | "{LEFT}" | "DesktopPriorItem" => {
                    if let Some(ref mut ds) = self.desktop_state {
                        if let Some(item) = ds.move_prior() {
                            let speech = SpeechFormatter::format_desktop_item(
                                &item.name, item.index, item.total,
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                "{TAB}" | "ShellNextElement" => {
                    self.app_mode = AppMode::NativeShell;
                    if let Some(ref mut sn) = self.shell_nav_state {
                        sn.current_index = 0;
                        if let Some(el) = sn.current_element() {
                            let speech = SpeechFormatter::format_shell_element(
                                el.container.as_deref(),
                                &el.name,
                                "Button",
                                el.help.as_deref(),
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                "+{TAB}" | "ShellPriorElement" => {
                    self.app_mode = AppMode::NativeShell;
                    if let Some(ref mut sn) = self.shell_nav_state {
                        sn.current_index = sn.elements.len().saturating_sub(1);
                        if let Some(el) = sn.current_element() {
                            let speech = SpeechFormatter::format_shell_element(
                                el.container.as_deref(),
                                &el.name,
                                "Button",
                                el.help.as_deref(),
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                _ => {
                    // Check if it's a single alphanumeric key for desktop first-letter navigation
                    if command.len() == 1 {
                        if let Some(c) = command.chars().next() {
                            if c.is_alphanumeric() {
                                if let Some(ref mut ds) = self.desktop_state {
                                    let item = ds.match_initial(c)?;
                                    let speech = SpeechFormatter::format_desktop_item(
                                        &item.name, item.index, item.total,
                                    );
                                    self.speech.speak(&speech, true);
                                    return Some(speech);
                                }
                            }
                        }
                    }
                }
            }
            return None;
        } else if self.app_mode == AppMode::NativeShell {
            match command {
                "{TAB}" | "ShellNextElement" => {
                    if let Some(ref mut sn) = self.shell_nav_state {
                        if sn.current_index + 1 >= sn.elements.len() {
                            // Cycle back to Desktop
                            self.app_mode = AppMode::NativeDesktop;
                            if let Some(ref ds) = self.desktop_state {
                                if let Some(item) = ds.current_item() {
                                    let speech = SpeechFormatter::format_desktop_entry(
                                        &ds.container_name,
                                        &item.name,
                                        item.index,
                                        item.total,
                                    );
                                    self.speech.speak(&speech, true);
                                    return Some(speech);
                                }
                            }
                        } else {
                            sn.current_index += 1;
                            if let Some(el) = sn.current_element() {
                                let speech = SpeechFormatter::format_shell_element(
                                    el.container.as_deref(),
                                    &el.name,
                                    "Button",
                                    el.help.as_deref(),
                                );
                                self.speech.speak(&speech, true);
                                return Some(speech);
                            }
                        }
                    }
                    return None;
                }
                "+{TAB}" | "ShellPriorElement" => {
                    if let Some(ref mut sn) = self.shell_nav_state {
                        if sn.current_index == 0 {
                            // Step back to Desktop
                            self.app_mode = AppMode::NativeDesktop;
                            if let Some(ref ds) = self.desktop_state {
                                if let Some(item) = ds.current_item() {
                                    let speech = SpeechFormatter::format_desktop_entry(
                                        &ds.container_name,
                                        &item.name,
                                        item.index,
                                        item.total,
                                    );
                                    self.speech.speak(&speech, true);
                                    return Some(speech);
                                }
                            }
                        } else {
                            sn.current_index -= 1;
                            if let Some(el) = sn.current_element() {
                                let speech = SpeechFormatter::format_shell_element(
                                    el.container.as_deref(),
                                    &el.name,
                                    "Button",
                                    el.help.as_deref(),
                                );
                                self.speech.speak(&speech, true);
                                return Some(speech);
                            }
                        }
                    }
                    return None;
                }
                _ => {}
            }
            return None;
        }

        // 0. Active element list navigation takes precedence for {DOWN}, {UP}, {ENTER}, {ESC}
        if let Some(ref mut list) = self.active_element_list {
            match command {
                "{DOWN}" => {
                    let speech = list.move_next();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                "{UP}" => {
                    let speech = list.move_prior();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                "{ENTER}" => {
                    let target_offset = list.selected_offset();
                    self.active_element_list = None;
                    let speech = self.cursor.jump_to_element_offset(target_offset);
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                "{ESC}" => {
                    self.active_element_list = None;
                    let speech = "virtual PC Cursor".to_string();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                _ => {}
            }
        }

        // Parameterized search commands
        if let Some(query) = command
            .strip_prefix("Find:")
            .or_else(|| command.strip_prefix("find:"))
            .or_else(|| command.strip_prefix("VirtualFind:"))
            .or_else(|| command.strip_prefix("JAWSFind:"))
            .or_else(|| command.strip_prefix("^f:"))
        {
            let speech = self.cursor.find_text(query, false);
            if let Some(ref text) = speech {
                self.speech.speak(text, true);
            }
            return speech;
        }

        // Parameterized element list selection
        if let Some(target) = command
            .strip_prefix("SelectHeading:")
            .or_else(|| command.strip_prefix("JumpToHeading:"))
        {
            let list = self.cursor.buffer.get_headings_list();
            if let Some(item) = list
                .iter()
                .find(|h| h.text.eq_ignore_ascii_case(target) || h.text.contains(target))
            {
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
        }

        if let Some(target) = command
            .strip_prefix("SelectLink:")
            .or_else(|| command.strip_prefix("JumpToLink:"))
        {
            let list = self.cursor.buffer.get_links_list();
            if let Some(item) = list
                .iter()
                .find(|l| l.text.eq_ignore_ascii_case(target) || l.text.contains(target))
            {
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
        }

        if let Some(target) = command
            .strip_prefix("SelectFormField:")
            .or_else(|| command.strip_prefix("JumpToFormField:"))
        {
            let list = self.cursor.buffer.get_form_fields_list();
            if let Some(item) = list
                .iter()
                .find(|f| f.name.eq_ignore_ascii_case(target) || f.name.contains(target))
            {
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
        }

        let speech = match command {
            "{DOWN}" => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_next_option()
                } else {
                    self.cursor.move_next_line()
                }
            }
            "{UP}" => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_prior_option()
                } else {
                    self.cursor.move_prior_line()
                }
            }
            "^{RIGHT}" => self.cursor.move_next_word(),
            "^{LEFT}" => self.cursor.move_prior_word(),
            "{RIGHT}" => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_expand_or_next(),
                    Some(Role::Slider) => self.cursor.slider_increase(),
                    _ => self.cursor.move_next_char(),
                }
            }
            "{LEFT}" => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_collapse_or_parent(),
                    Some(Role::Slider) => self.cursor.slider_decrease(),
                    _ => self.cursor.move_prior_char(),
                }
            }
            "^{DOWN}" => self.cursor.move_next_paragraph(),
            "^{UP}" => self.cursor.move_prior_paragraph(),
            "^{HOME}" => Some(self.cursor.move_to_start()),
            "^{END}" => Some(self.cursor.move_to_end()),

            // Headings
            "H" | "h" | "MoveToNextHeading" => self.cursor.move_next_heading(),
            "+H" | "+h" | "MoveToPriorHeading" => self.cursor.move_prior_heading(),
            "1" | "MoveToNextHeadingLevelOne" => self.cursor.move_next_heading_level(1),
            "2" | "MoveToNextHeadingLevelTwo" => self.cursor.move_next_heading_level(2),
            "3" | "MoveToNextHeadingLevelThree" => self.cursor.move_next_heading_level(3),
            "4" | "MoveToNextHeadingLevelFour" => self.cursor.move_next_heading_level(4),
            "5" | "MoveToNextHeadingLevelFive" => self.cursor.move_next_heading_level(5),
            "6" | "MoveToNextHeadingLevelSix" => self.cursor.move_next_heading_level(6),
            "+1" | "MoveToPriorHeadingLevelOne" => self.cursor.move_prior_heading_level(1),
            "+2" | "MoveToPriorHeadingLevelTwo" => self.cursor.move_prior_heading_level(2),
            "+3" | "MoveToPriorHeadingLevelThree" => self.cursor.move_prior_heading_level(3),
            "+4" | "MoveToPriorHeadingLevelFour" => self.cursor.move_prior_heading_level(4),
            "+5" | "MoveToPriorHeadingLevelFive" => self.cursor.move_prior_heading_level(5),
            "+6" | "MoveToPriorHeadingLevelSix" => self.cursor.move_prior_heading_level(6),

            // Links
            "U" | "u" | "MoveToNextLink" | "MoveToNextUnvisitedLink" => {
                self.cursor.move_next_link()
            }
            "+U" | "+u" | "MoveToPriorLink" | "MoveToPriorUnvisitedLink" => {
                self.cursor.move_prior_link()
            }

            // Tables
            "T" | "t" | "MoveToNextTable" => self.cursor.move_next_table(),
            "+T" | "+t" | "MoveToPriorTable" => self.cursor.move_prior_table(),
            "^%{RIGHT}" | "NextCell" | "TableCellRight" => self.cursor.move_table_cell_right(),
            "^%{LEFT}" | "PriorCell" | "TableCellLeft" => self.cursor.move_table_cell_left(),
            "^%{DOWN}" | "NextRow" | "TableCellDown" => self.cursor.move_table_cell_down(),
            "^%{UP}" | "PriorRow" | "TableCellUp" => self.cursor.move_table_cell_up(),
            "^%{HOME}" | "FirstCell" | "TableFirstCell" => self.cursor.move_table_first_cell(),
            "^%{END}" | "LastCell" | "TableLastCell" => self.cursor.move_table_last_cell(),
            "^%+LEFT" | "^%+{LEFT}" | "MoveToStartOfRow" | "RowStart" => {
                self.cursor.move_table_row_start()
            }
            "^%+RIGHT" | "^%+{RIGHT}" | "MoveToEndOfRow" | "RowEnd" => {
                self.cursor.move_table_row_end()
            }
            "^%+UP" | "^%+{UP}" | "MoveToTopOfColumn" | "ColumnTop" => {
                self.cursor.move_table_col_top()
            }
            "^%+DOWN" | "^%+{DOWN}" | "MoveToBottomOfColumn" | "ColumnBottom" => {
                self.cursor.move_table_col_bottom()
            }
            "^%{NUMPAD5}" | "^%5" | "SayCurrentCell" | "CurrentCell" => {
                self.cursor.read_current_cell()
            }
            "^%+H" | "^%+h" | "SayColumnHeader" => self.cursor.read_current_col_header(),
            "^%+R" | "^%+r" | "SayRowHeader" => self.cursor.read_current_row_header(),

            // Bindings from JAWS's Quick Navigation Keys table
            // (docs/references/jaws-quick-navigation-keys.md).
            "O" | "o" | "MoveToNextArticle" => self.cursor.move_next_article(),
            "+O" | "+o" | "MoveToPriorArticle" => self.cursor.move_prior_article(),
            "Q" | "q" | "+Q" | "+q" | "MoveToMainRegion" => self.cursor.move_to_main_region(),
            ";" | "MoveToNextOnMouseOverElement" => self.cursor.move_next_on_mouse_over(),
            "+;" | "MoveToPriorOnMouseOverElement" => self.cursor.move_prior_on_mouse_over(),
            "'" | "MoveToNextTab" => self.cursor.move_next_tab(),
            "+'" | "MoveToPriorTab" => self.cursor.move_prior_tab(),
            // Place markers are not implemented; K must not fall through to another command.
            "K" | "k" | "+K" | "+k" | "MoveToNextPlaceMarker" | "MoveToPriorPlaceMarker" => None,

            // Graphics and radio buttons (spec §16, §17)
            "G" | "g" | "MoveToNextGraphic" => self.cursor.move_next_graphic(),
            "+G" | "+g" | "MoveToPriorGraphic" => self.cursor.move_prior_graphic(),
            "A" | "a" | "MoveToNextRadioButton" => self.cursor.move_next_radio_button(),
            "+A" | "+a" | "MoveToPriorRadioButton" => self.cursor.move_prior_radio_button(),

            // MathML
            "MoveToNextMath" => self.cursor.move_next_math(),
            "MoveToPriorMath" => self.cursor.move_prior_math(),
            "EnterMathViewer" | "ShowMathEditor" => self.cursor.enter_math_viewer(),

            // Form controls
            "F" | "f" | "FocusToNextField" => self.cursor.move_next_form_control(),
            "+F" | "+f" | "FocusToPriorField" => self.cursor.move_prior_form_control(),
            "B" | "b" | "MoveToNextButton" => self.cursor.move_next_button(),
            "+B" | "+b" | "MoveToPriorButton" => self.cursor.move_prior_button(),
            "E" | "e" | "MoveToNextEdit" => self.cursor.move_next_edit(),
            "+E" | "+e" | "MoveToPriorEdit" => self.cursor.move_prior_edit(),
            "X" | "x" | "MoveToNextCheckbox" => self.cursor.move_next_checkbox(),
            "+X" | "+x" | "MoveToPriorCheckbox" => self.cursor.move_prior_checkbox(),
            "C" | "c" | "MoveToNextCombo" | "MoveToNextComboBox" => {
                self.cursor.move_next_combobox()
            }
            "+C" | "+c" | "MoveToPriorCombo" | "MoveToPriorComboBox" => {
                self.cursor.move_prior_combobox()
            }
            "!{DOWN}" | "ExpandComboBox" => self.cursor.expand_combobox(),
            "!{UP}" | "CollapseComboBox" => self.cursor.collapse_combobox(),

            // Lists & Landmarks
            "L" | "l" | "MoveToNextList" => self.cursor.move_next_list(),
            "+L" | "+l" | "MoveToPriorList" => self.cursor.move_prior_list(),
            "I" | "i" | "MoveToNextListItem" => self.cursor.move_next_list_item(),
            "+I" | "+i" | "MoveToPriorListItem" => self.cursor.move_prior_list_item(),
            "R" | "r" | "MoveToNextLandmark" | "MoveToNextRegion" => {
                self.cursor.move_next_landmark()
            }
            "+R" | "+r" | "MoveToPriorLandmark" | "MoveToPriorRegion" => {
                self.cursor.move_prior_landmark()
            }

            // Frames
            "M" | "m" | "MoveToNextFrame" => self.cursor.move_next_frame(),
            "+M" | "+m" | "MoveToPriorFrame" => self.cursor.move_prior_frame(),

            // Interactive navigation & actions
            "{TAB}" => self.cursor.move_next_tab_stop(),
            "+{TAB}" | "+TAB" => self.cursor.move_prior_tab_stop(),
            "{SPACE}" => {
                if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
                    let is_link = matches!(node.role, Role::Link { .. });
                    if node.role.is_interactive_control() && !is_link && actions.activate(&node) {
                        // The page's own state change and focus events report the result.
                        return Some(String::new());
                    }
                }
                None
            }
            "{ENTER}" | "EnterFormsMode" | "ActivateLink" => {
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
            "{ESC}" | "ExitFormsMode" => {
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
            "+{RIGHT}" | "SelectNextCharacter" => self.cursor.select_next_char(),
            "+{LEFT}" | "SelectPriorCharacter" => self.cursor.select_prior_char(),
            "+{DOWN}" | "SelectNextLine" => self.cursor.select_next_line(),
            "+{UP}" | "SelectPriorLine" => self.cursor.select_prior_line(),
            "+!{DOWN}" | "SaySelection" | "SaySelectedText" => Some(self.cursor.say_selection()),

            // Font formatting query
            "Insert+F" | "WhatFont" | "SayFont" => Some(self.cursor.say_font_formatting()),

            // Modal dialog commands
            "EnterModal" => {
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
            "ExitModal" => self.cursor.exit_modal(),

            // JAWS Say Scripts
            "SayAll" => Some(self.cursor.say_all()),
            "SayLine" => Some(self.cursor.current_line()),
            "SayWord" => Some(self.cursor.current_word()),
            "SayCharacter" => Some(self.cursor.current_char()),

            // In-buffer find commands
            "^f" | "Ctrl+F" | "VirtualFind" | "JAWSFind" => {
                Some("JAWS Find  \u{001d} dialog \u{001e} \nFind what:  \u{001d} edit combo \u{001e} \nTo set the value use the Arrow keys or type the value.".to_string())
            }
            "{F3}" | "F3" | "JAWSFindNext" | "FindNext" => self.cursor.find_next(),
            "+{F3}" | "+F3" | "Shift+F3" | "JAWSFindPrior" | "FindPrior" => self.cursor.find_prior(),

            // Element list dialogs
            "SelectAHeading" | "HeadingList" | "HeadingsList" | "Insert+F6" | "JAWSKey+F6" => {
                let items = self.cursor.buffer.get_headings_list();
                if items.is_empty() {
                    Some("There are no headings on this page.".to_string())
                } else {
                    let speech = SpeechFormatter::format_headings_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::Headings {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            "SelectALink" | "LinkList" | "LinksList" | "Insert+F7" | "JAWSKey+F7" => {
                let items = self.cursor.buffer.get_links_list();
                if items.is_empty() {
                    Some("There are no links on this page.".to_string())
                } else {
                    let speech = SpeechFormatter::format_links_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::Links {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            "SelectAFormField"
            | "FormFieldList"
            | "FormFieldsList"
            | "Insert+F5"
            | "JAWSKey+F5"
            | "Insert+Ctrl+F"
            | "JAWSKey+Ctrl+F" => {
                let items = self.cursor.buffer.get_form_fields_list();
                if items.is_empty() {
                    Some("There are no form fields on this page.".to_string())
                } else {
                    let speech = SpeechFormatter::format_form_fields_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::FormFields {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }

            _ => None,
        };

        if let Some(ref text) = speech {
            // Left/Right read a character, which the synthesizer names ("space").
            let character = matches!(command, "{RIGHT}" | "{LEFT}") && text.chars().count() == 1;
            if character {
                self.speech.speak_character(text, true);
            } else if !text.is_empty() {
                self.speech.speak(text, true);
            }
        }

        speech
    }

    /// Whether keys go to a text field: forms mode in a page, or a focused native edit or
    /// text document (Windows 11 Notepad, Word).
    pub(super) fn editing_text(&self) -> bool {
        (self.forms_mode
            && self
                .node_under_cursor()
                .is_some_and(|n| n.role == Role::Edit))
            || (self.app_mode != AppMode::VirtualBuffer
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| matches!(f.role, Role::Edit | Role::Document)))
    }

    /// A JAWS key command (Insert+…). In a text field the reading commands read the field;
    /// elsewhere they run as virtual cursor or application commands.
    pub(super) fn jaws_key_command(&mut self, event: &KeyEvent) {
        let layout_command = self.settings.keyboard.layout_command(event);
        let say_line =
            layout_command == Some("SayLine") || crate::editing::EditKey::of(event).is_some();
        if self.editing_text() && say_line {
            // Say Line does not change the field, so no report follows: speak the field's
            // last known text now, or read it when none is known [obs, 305/s9, s15]. A
            // multi-line document (Notepad) gives the line alone (JAWS, dev box).
            let key = crate::editing::EditKey::SayLine;
            match self.last_text.clone() {
                Some(context) => {
                    let document = self
                        .last_native_focus
                        .as_ref()
                        .is_some_and(|f| f.role == Role::Document);
                    let speech = if document {
                        Some(crate::editing::line_at(&context).trim().to_owned())
                            .filter(|l| !l.is_empty())
                    } else {
                        crate::editing::speech(key, None, &context, &self.text_field_line())
                    };
                    if let Some(speech) = speech {
                        self.speech.speak(&speech, true);
                    }
                }
                None => self.expect_text_change(key),
            }
            return;
        }
        // A reading command that moves first is the matching arrow key: the application's
        // own in a native control (sent, then read as any arrow key is), the virtual
        // cursor's in a page.
        let moving = layout_command.and_then(|command| {
            Some(match command {
                "SayNextLine" => (vec![Key::DownArrow], "{DOWN}"),
                "SayPriorLine" => (vec![Key::UpArrow], "{UP}"),
                "SayNextWord" => (vec![Key::Control, Key::RightArrow], "^{RIGHT}"),
                "SayPriorWord" => (vec![Key::Control, Key::LeftArrow], "^{LEFT}"),
                "SayPriorCharacter" => (vec![Key::LeftArrow], "{LEFT}"),
                _ => return None,
            })
        });
        if let Some((chord, page_command)) = moving {
            if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
                let _ = self.execute_command(page_command);
            } else if let Some(actions) = &self.actions {
                actions.press(&chord);
            }
            return;
        }
        if self.editing_text() && layout_command == Some("SayWord") {
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
        if let Some(cmd) = layout_command
            .map(str::to_owned)
            .or_else(|| event.to_command_string())
        {
            let cmd = self.repeated_command(cmd, event.time_ms);
            let _ = self.execute_command(&cmd);
        }
    }

    /// The command a JAWS-key command becomes when pressed twice quickly, e.g. Say System
    /// Time's second press speaks the date [obs, 308/s2, s4].
    fn repeated_command(&mut self, cmd: String, time_ms: u64) -> String {
        let twice = matches!(&self.last_press, Some((last, at))
            if *last == cmd && time_ms != 0 && time_ms.saturating_sub(*at) <= DOUBLE_PRESS_MS);
        self.last_press = (!twice).then(|| (cmd.clone(), time_ms));
        if !twice {
            return cmd;
        }
        match cmd.as_str() {
            "SaySystemTime" => "SaySystemDate".to_owned(),
            "SaySystemTimeWithSeconds" => "SayWeekNumber".to_owned(),
            "CapsLock" => "ToggleCapsLock".to_owned(),
            "Insert+F" => "SayFontInViewer".to_owned(),
            _ => cmd,
        }
    }

    /// CapsLock as the JAWS key: a single press is the JAWS key; pressed twice quickly it
    /// toggles Caps Lock (the second press reaches the system), which JAWS announces
    /// [obs, 310: "Caps lock on", then "Caps lock off"].
    pub(super) fn caps_lock_press(&mut self, event: &KeyEvent) -> KeyAction {
        if self.repeated_command("CapsLock".to_owned(), event.time_ms) != "ToggleCapsLock" {
            return KeyAction::Consume;
        }
        let was_on = event.caps_lock_on.unwrap_or(self.caps_lock_on);
        self.caps_lock_on = !was_on;
        let speech = if was_on {
            "Caps lock off"
        } else {
            "Caps lock on"
        };
        self.speech.speak(speech, true);
        KeyAction::PassThrough
    }

    /// Say System Time and its variants, from the clock in the user's locale formats.
    fn clock_command(&self, command: &str) -> Option<String> {
        let clock = self.clock.as_deref();
        match command {
            "SaySystemTime" => Some(crate::clock::say_time(clock?, false)),
            "SaySystemTimeWithSeconds" => Some(crate::clock::say_time(clock?, true)),
            "SaySystemDate" => Some(crate::clock::say_date(clock?, false)),
            "SayWeekNumber" => Some(crate::clock::say_date(clock?, true)),
            _ => None,
        }
    }

    /// Say Font (`Insert+F`): the font at the cursor. Pressed twice quickly, JAWS shows it in
    /// the Virtual Viewer and reads the message [obs, 209/s7, 314/s2]; with no font it just
    /// says "No font" again (313/s2). A native field reports its font at the caret (UIA text
    /// attributes) or its window's font; JAWS's size for classic Notepad (314/s1, "22
    /// point") awaits a trace that recorded the font.
    fn say_font(&mut self, in_viewer: bool) -> Option<String> {
        let formatting = if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
            self.cursor.buffer.formatting_at_offset(self.cursor.offset)
        } else {
            // A text field reports the font at its caret.
            self.last_text
                .as_ref()
                .and_then(|t| t.font.as_ref())
                .map(crate::event::FontInfo::formatting)
        };
        let description = formatting
            .as_ref()
            .and_then(SpeechFormatter::font_description);
        // A native text field that reports its alignment adds its paragraph formatting, after
        // a comma (JAWS 2025 in Windows 11 Notepad, dev box: "Font is Consolas 11 point, |
        // Paragraph formatting left justified"); pages (209/s3) and fields known only by
        // their window font (314/s1) do not.
        let paragraph = formatting
            .as_ref()
            .and_then(|f| f.alignment.as_deref())
            .filter(|_| self.app_mode != AppMode::VirtualBuffer)
            .map(SpeechFormatter::paragraph_formatting);
        let description = match (description, &paragraph) {
            (Some(font), Some(_)) => Some(format!("{font},")),
            (font, _) => font,
        };
        let speech = match (in_viewer, description) {
            (true, Some(font)) if self.viewer.is_none() => {
                let viewer = crate::viewer::VirtualViewer::message(vec![
                    "Font information:".to_owned(),
                    font,
                    paragraph.unwrap_or_default(),
                    String::new(),
                    "\u{1d} Link \u{1e} List JAWS Hot Keys".to_owned(),
                    "Press ESCAPE to close this message.".to_owned(),
                ]);
                let speech = viewer.message_speech();
                self.viewer = Some(viewer);
                speech
            }
            (_, Some(font)) => match paragraph {
                Some(paragraph) => format!("Font is {font}\n{paragraph}"),
                None => format!("Font is {font}"),
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
            let speech = "The current location is already in a virtual view".to_owned();
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        // A console's text is its buffer as it is now: it is read fresh, and the viewer opens
        // when it arrives (`open_console_viewer`).
        let console = self
            .last_native_focus
            .as_ref()
            .is_some_and(crate::apps::terminal::owns);
        if console && self.actions.as_ref().is_some_and(|a| a.read_text()) {
            self.pending_viewer = Some(PendingViewer::Console { window });
            return None;
        }
        // A text field's text is its lines (Notepad, where JAWS shows only "paragraph"),
        // unless it is one field of a dialog being virtualized whole. Anything else is read
        // as it is on screen, and the viewer opens when it arrives (`open_screen_viewer`).
        let dialog = self
            .last_native_focus
            .as_ref()
            .is_some_and(|f| f.window_kind.as_deref() == Some("#32770"));
        let text_field = self.editing_text() && !(window && dialog);
        let native = self.app_mode != AppMode::VirtualBuffer;
        if native && !text_field && self.actions.as_ref().is_some_and(|a| a.read_window(window)) {
            self.pending_viewer = Some(PendingViewer::Screen { window });
            return None;
        }
        let lines = self.virtualized_lines(window)?;
        self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
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
        let what = if window { "window" } else { "control" };
        let speech = format!("Virtualizing current {what}\n{}", viewer.line_text());
        self.viewer = Some(viewer);
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
        let (lines, line) = crate::apps::terminal::viewer_lines(&context.text, context.caret);
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
        if window && focus.window_kind.as_deref() == Some("#32770") {
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
        let console = self
            .last_native_focus
            .as_ref()
            .is_some_and(crate::apps::terminal::owns);
        let speech = if console {
            // A console: its cursor's line, as last read (JAWS on the dev box, closing the
            // viewer: "C:\Users\Gigabyte>").
            self.last_text
                .as_ref()
                .map(|text| crate::apps::terminal::cursor_line(&text.text, text.caret))
        } else if self.editing_text() {
            let line = self.last_text.as_ref().map(crate::editing::line_at);
            // A combo box's field adds its list position (JAWS on the Run dialog, dev box:
            // "C: | 1 of 6").
            let position = self
                .last_native_focus
                .as_ref()
                .filter(|f| f.container_role == Some(Role::ComboBox))
                .and_then(|f| Some(format!("{} of {}", f.item_index?, f.item_count?)));
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
