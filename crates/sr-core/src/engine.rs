use crate::aom::Role;
use crate::buffer::VirtualBuffer;
use crate::cursor::VirtualCursor;
use crate::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
use crate::input::{Key, KeyAction, KeyEvent};
use crate::speech::{SpeechFormatter, SpeechOutput};

/// Primary runtime engine for screen reader execution, coordinating virtual cursor navigation,
/// speech output, and interactive input event handling.
pub struct ScreenReaderEngine {
    pub cursor: VirtualCursor,
    pub speech: Box<dyn SpeechOutput>,
    pub forms_mode: bool,
}

impl ScreenReaderEngine {
    pub fn new(cursor: VirtualCursor, speech: Box<dyn SpeechOutput>) -> Self {
        Self {
            cursor,
            speech,
            forms_mode: false,
        }
    }

    /// Replaces the active document buffer and resets the cursor.
    pub fn load_buffer(&mut self, buffer: VirtualBuffer) {
        self.cursor = VirtualCursor::new(buffer);
        self.cursor.move_to_start();
        self.forms_mode = false;
    }

    /// Executes a screen reader navigation command string (matching JAWS script names and keystrokes).
    /// Speaks the resulting announcement and returns the emitted speech text.
    pub fn execute_command(&mut self, command: &str) -> Option<String> {
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
            "U" | "u" | "K" | "k" | "MoveToNextLink" | "MoveToNextUnvisitedLink" => {
                self.cursor.move_next_link()
            }
            "+U" | "+u" | "+K" | "+k" | "MoveToPriorLink" | "MoveToPriorUnvisitedLink" => {
                self.cursor.move_prior_link()
            }

            // Tables
            "T" | "t" | "MoveToNextTable" => self.cursor.move_next_table(),
            "+T" | "+t" | "MoveToPriorTable" => self.cursor.move_prior_table(),
            "^%{RIGHT}" | "NextCell" | "TableCellRight" => self.cursor.move_table_cell_right(),
            "^%{LEFT}" | "PriorCell" | "TableCellLeft" => self.cursor.move_table_cell_left(),
            "^%{DOWN}" | "NextRow" | "TableCellDown" => self.cursor.move_table_cell_down(),
            "^%{UP}" | "PriorRow" | "TableCellUp" => self.cursor.move_table_cell_up(),

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
            "R" | "r" | ";" | "MoveToNextLandmark" | "MoveToNextRegion" => {
                self.cursor.move_next_landmark()
            }
            "+R" | "+r" | "+;" | "MoveToPriorLandmark" | "MoveToPriorRegion" => {
                self.cursor.move_prior_landmark()
            }

            // Interactive navigation & actions
            "{TAB}" => self.cursor.move_next_tab_stop(),
            "+{TAB}" | "+TAB" => self.cursor.move_prior_tab_stop(),
            "{SPACE}" => self.cursor.toggle_active_control(),
            "{ENTER}" | "EnterFormsMode" => {
                let res = self.cursor.enter_forms_mode();
                if res.is_some() {
                    self.forms_mode = true;
                }
                res
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

            _ => None,
        };

        if let Some(ref text) = speech {
            if !text.is_empty() {
                self.speech.speak(text, true);
            }
        }

        speech
    }

    /// Handles a live keyboard event from the low-level hook.
    /// Intercepts navigation keys, manages forms mode transitions, silences ongoing speech,
    /// and returns whether the key was consumed or should pass through to the OS.
    pub fn handle_key_event(&mut self, event: &KeyEvent) -> KeyAction {
        if !event.is_down {
            return KeyAction::PassThrough;
        }

        // 1. Any key press interrupts speech immediately (NVDA & JAWS standard behavior)
        self.speech.stop();

        // 2. Standalone Control tap is speech silence key
        if event.key == Key::Control && event.modifiers.is_empty() {
            return KeyAction::PassThrough;
        }

        // 3. Handle Forms Mode
        if self.forms_mode {
            if event.key == Key::Escape {
                self.forms_mode = false;
                let _ = self.cursor.exit_forms_mode();
                self.speech.speak("Virtual PC Cursor", true);
                return KeyAction::Consume;
            }
            // In forms mode, pass all typing and navigation through to the active control
            return KeyAction::PassThrough;
        }

        // 4. Handle Escape when inside an expanded combobox or modal dialog
        if event.key == Key::Escape && event.modifiers.is_empty() {
            if self.cursor.is_combobox_expanded() {
                if let Some(msg) = self.cursor.collapse_combobox() {
                    self.speech.speak(&msg, true);
                }
                return KeyAction::Consume;
            }
            if self.cursor.is_in_modal() {
                if let Some(msg) = self.cursor.exit_modal() {
                    self.speech.speak(&msg, true);
                }
                return KeyAction::Consume;
            }
        }

        // 4. In Virtual PC Cursor mode, check if keystroke translates to a command
        if let Some(cmd) = event.to_command_string() {
            let res = self.execute_command(&cmd);
            if res.is_some() {
                return KeyAction::Consume;
            }
            // For navigation keys, consume even if boundary reached so host window doesn't scroll
            if matches!(
                event.key,
                Key::DownArrow
                    | Key::UpArrow
                    | Key::LeftArrow
                    | Key::RightArrow
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown
                    | Key::Tab
                    | Key::Alpha(_)
                    | Key::Digit(_)
            ) {
                return KeyAction::Consume;
            }
        }

        KeyAction::PassThrough
    }

    /// Handles an incoming accessibility event from the OS event listener.
    pub fn handle_accessibility_event(&mut self, event: &AccessibilityEvent) -> Option<String> {
        match event {
            AccessibilityEvent::Focus(focus) => self.handle_focus_event(focus),
            AccessibilityEvent::LiveRegion(live) => self.handle_live_region_event(live),
        }
    }

    /// Handles an OS-level focus changed event.
    /// Synchronizes cursor position, activates/deactivates forms mode,
    /// and emits canonical JAWS speech announcements.
    pub fn handle_focus_event(&mut self, focus: &FocusEvent) -> Option<String> {
        // Try matching node in buffer
        let matched_node_id = focus
            .node_id
            .filter(|id| self.cursor.buffer.node_map.contains_key(id));

        let target_id = matched_node_id.or_else(|| {
            self.cursor.buffer.node_map.values().find_map(|node| {
                if node.role == focus.role {
                    let ev_name = focus.name.as_deref();
                    let node_name = node.name.as_deref().or(node.text.as_deref());
                    if let (Some(en), Some(nn)) = (ev_name, node_name) {
                        if en.eq_ignore_ascii_case(nn) {
                            return Some(node.id);
                        }
                    }
                }
                None
            })
        });

        if let Some(id) = target_id {
            self.cursor.move_to_node_id(id);
        }

        // If focus target is a dialog, enter modal confinement and announce Dialog <Title>
        if matches!(focus.role, Role::Dialog | Role::AlertDialog) {
            let title = focus.name.as_deref().unwrap_or("");
            let msg = if let Some(id) = target_id {
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

        let is_edit = matches!(focus.role, Role::Edit);
        let speech = if is_edit {
            let doc_title = self.cursor.buffer.document_title().unwrap_or("");
            let name = focus
                .name
                .as_deref()
                .or_else(|| {
                    target_id
                        .and_then(|id| self.cursor.buffer.node_map.get(&id))
                        .and_then(|n| n.name.as_deref())
                })
                .unwrap_or("");
            let value = focus.value.as_deref().or_else(|| {
                target_id
                    .and_then(|id| self.cursor.buffer.node_map.get(&id))
                    .and_then(|n| n.value.as_deref())
            });

            if !self.forms_mode {
                self.forms_mode = true;
                self.cursor.forms_mode_active = true;
                Some(SpeechFormatter::format_forms_mode_entry(
                    doc_title, name, value,
                ))
            } else {
                Some(SpeechFormatter::format_edit(name, value))
            }
        } else if self.forms_mode {
            // Exiting forms mode because focused element is not an Edit
            self.forms_mode = false;
            self.cursor.forms_mode_active = false;
            let exit_msg = SpeechFormatter::format_forms_mode_exit();
            let control_msg = if let Some(id) = target_id {
                if let Some(node) = self.cursor.buffer.node_map.get(&id) {
                    self.cursor.format_tab_stop_node(node, None, None)
                } else {
                    String::new()
                }
            } else {
                focus.name.clone().unwrap_or_default()
            };
            if !control_msg.is_empty() {
                Some(format!("{}\n{}", control_msg, exit_msg))
            } else {
                Some(exit_msg)
            }
        } else {
            // Staying in Virtual PC Cursor mode: announce focused control
            if let Some(id) = target_id {
                if let Some(node) = self.cursor.buffer.node_map.get(&id) {
                    Some(self.cursor.format_tab_stop_node(node, None, None))
                } else {
                    None
                }
            } else {
                let name = focus.name.as_deref().unwrap_or("");
                match focus.role {
                    Role::Button => Some(SpeechFormatter::format_button(name)),
                    Role::CheckBox => Some(SpeechFormatter::format_checkbox(
                        name,
                        focus.states.checked.unwrap_or(false),
                    )),
                    Role::Link { .. } => Some(SpeechFormatter::format_link(name, false)),
                    _ => focus.name.clone(),
                }
            }
        };

        if let Some(ref text) = speech {
            if !text.is_empty() {
                self.speech.speak(text, true);
            }
        }

        speech
    }

    /// Handles an ARIA live region announcement event with politeness consideration.
    pub fn handle_live_region_event(&mut self, live: &LiveRegionEvent) -> Option<String> {
        match live.politeness {
            Politeness::Assertive => {
                self.speech.stop();
                self.speech.speak(&live.text, true);
                Some(live.text.clone())
            }
            Politeness::Polite => {
                self.speech.speak(&live.text, false);
                Some(live.text.clone())
            }
            Politeness::Off => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::{AomNode, Role};
    use crate::input::Modifiers;
    use crate::speech::MockSpeechOutput;

    fn create_test_buffer() -> VirtualBuffer {
        let aom = AomNode::new(0, Role::Document)
            .with_name("Interactive Test Document")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 }).with_text("Main Title"),
                AomNode::new(2, Role::Paragraph).with_text("This is interactive paragraph."),
                AomNode::new(3, Role::Edit).with_name("Search Input"),
                AomNode::new(4, Role::Button).with_name("Submit Button"),
                {
                    let mut cb = AomNode::new(5, Role::CheckBox).with_name("Subscribe Checkbox");
                    cb.states.checked = Some(true);
                    cb
                },
            ]);
        VirtualBuffer::from_aom(&aom)
    }

    #[test]
    fn test_engine_navigation_and_speech() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Down arrow navigation
        let speech = engine.execute_command("{DOWN}");
        assert_eq!(
            speech,
            Some(" \u{001d} heading level 1 \u{001e} Main Title".to_string())
        );

        // Heading navigation
        let h_speech = engine.execute_command("H");
        assert_eq!(
            h_speech,
            Some("wrapping to top\nMain Title\n \u{001d} heading level 1 \u{001e}".to_string())
        );
    }

    #[test]
    fn test_engine_key_event_handling_and_forms_mode() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Move to edit field
        engine.execute_command("E");

        // Press Enter to enter forms mode
        let enter_event = KeyEvent::new(Key::Enter, Modifiers::none(), true);
        let action = engine.handle_key_event(&enter_event);
        assert_eq!(action, KeyAction::Consume);
        assert!(engine.forms_mode);

        // In forms mode, regular keys pass through
        let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);

        // Press Escape to exit forms mode
        let esc_event = KeyEvent::new(Key::Escape, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&esc_event), KeyAction::Consume);
        assert!(!engine.forms_mode);

        // Back in Virtual PC Cursor mode, Down arrow is consumed
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::Consume);
    }

    #[test]
    fn test_engine_auto_forms_mode_on_focus_edit() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        assert!(!engine.forms_mode);

        // Focus event moving into Search Input
        let focus_ev = FocusEvent::new(Role::Edit)
            .with_node_id(3)
            .with_name("Search Input");
        let speech = engine
            .handle_focus_event(&focus_ev)
            .expect("Should produce speech");

        // Engine should automatically enter Forms Mode and emit canonical JAWS prompt
        assert!(engine.forms_mode);
        assert!(engine.cursor.forms_mode_active);
        assert_eq!(engine.cursor.current_node_id, Some(3));
        assert!(speech.contains("Search Input"));
        assert!(speech.contains("Type in text."));

        // Focus transitions to Submit Button -> should auto exit forms mode
        let btn_focus = FocusEvent::new(Role::Button)
            .with_node_id(4)
            .with_name("Submit Button");
        let btn_speech = engine
            .handle_focus_event(&btn_focus)
            .expect("Should produce exit speech");

        assert!(!engine.forms_mode);
        assert!(!engine.cursor.forms_mode_active);
        assert_eq!(engine.cursor.current_node_id, Some(4));
        assert!(btn_speech.contains("Submit Button"));
        assert!(btn_speech.contains("virtual PC Cursor"));
    }

    #[test]
    fn test_engine_live_region_announcements() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Assertive live region
        let alert = LiveRegionEvent::new("Security update installed.", Politeness::Assertive);
        let alert_speech = engine
            .handle_live_region_event(&alert)
            .expect("Assertive alert speech");
        assert_eq!(alert_speech, "Security update installed.");

        // Polite live region
        let status = LiveRegionEvent::new("Download finished.", Politeness::Polite);
        let status_speech = engine
            .handle_live_region_event(&status)
            .expect("Polite status speech");
        assert_eq!(status_speech, "Download finished.");

        // Politeness::Off
        let off = LiveRegionEvent::new("Ignored mutation.", Politeness::Off);
        assert_eq!(engine.handle_live_region_event(&off), None);
    }

    #[test]
    fn test_engine_text_selection_and_font_query() {
        let aom =
            AomNode::new(0, Role::Document).with_children(vec![AomNode::new(1, Role::Paragraph)
                .with_text("Hello World")
                .with_formatting(
                    crate::aom::TextFormatting::new()
                        .with_size(16.0)
                        .with_bold(true)
                        .with_family("Arial")
                        .with_alignment("left"),
                )]);
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Font query
        let font_speech = engine.execute_command("Insert+F");
        assert_eq!(
            font_speech,
            Some("16 pt, bold, Arial on default background, left aligned".to_string())
        );

        // Select next character
        let sel1 = engine.execute_command("+{RIGHT}");
        assert_eq!(sel1, Some("selected H".to_string()));

        let sel2 = engine.execute_command("+{RIGHT}");
        assert_eq!(sel2, Some("selected e".to_string()));

        // Unselect prior character
        let unsel = engine.execute_command("+{LEFT}");
        assert_eq!(unsel, Some("unselected e".to_string()));

        // Query active selection
        let say_sel = engine.execute_command("SaySelection");
        assert_eq!(say_sel, Some("H".to_string()));

        // Moving cursor clears selection
        engine.execute_command("{RIGHT}");
        let no_sel = engine.execute_command("SaySelection");
        assert_eq!(no_sel, Some("No selection".to_string()));
    }

    #[test]
    fn test_engine_modal_dialog_confinement_and_escape() {
        let mut dlg = AomNode::new(10, Role::Dialog)
            .with_name("Confirm Deletion")
            .with_children(vec![
                AomNode::new(11, Role::Heading { level: 2 }).with_text("Are you sure?"),
                AomNode::new(12, Role::Button).with_name("Confirm"),
                AomNode::new(13, Role::Button).with_name("Cancel"),
            ]);
        dlg.states.modal = true;

        let aom = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Dashboard"),
            dlg,
            AomNode::new(20, Role::Paragraph).with_text("Backdrop footer text"),
        ]);

        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Focus event moving into dialog
        let focus_dlg = FocusEvent::new(Role::Dialog)
            .with_node_id(10)
            .with_name("Confirm Deletion");
        let speech = engine
            .handle_focus_event(&focus_dlg)
            .expect("Should announce dialog entry");
        assert_eq!(speech, "Dialog Confirm Deletion");
        assert!(engine.cursor.is_in_modal());

        // Quick navigation 'H' inside modal wraps within modal and finds level 2 heading
        let h_speech = engine.execute_command("H");
        assert!(h_speech.is_some());
        assert!(h_speech.unwrap().contains("Are you sure?"));

        // Tab moves to Confirm button
        let tab1 = engine.execute_command("{TAB}");
        assert!(tab1.unwrap().contains("Confirm"));

        // Tab moves to Cancel button
        let tab2 = engine.execute_command("{TAB}");
        assert!(tab2.unwrap().contains("Cancel"));

        // Tab wraps back to Confirm button inside dialog (cannot escape to backdrop!)
        let tab3 = engine.execute_command("{TAB}");
        assert!(tab3.unwrap().contains("Confirm"));

        // Escape exits modal dialog
        let esc_event = KeyEvent::new(Key::Escape, Modifiers::none(), true);
        let action = engine.handle_key_event(&esc_event);
        assert_eq!(action, KeyAction::Consume);
        assert!(!engine.cursor.is_in_modal());
    }
}
