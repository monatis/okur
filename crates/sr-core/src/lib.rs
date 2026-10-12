pub mod aom;
pub mod buffer;
pub mod cursor;
pub mod speech;

pub use aom::{AomNode, NodeStates, Role};
pub use buffer::{BufferLine, BufferParagraph, BufferSpan, BufferWord, VirtualBuffer};
pub use cursor::VirtualCursor;
pub use speech::SpeechFormatter;

pub fn init() {
    tracing::info!("sr-core initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_aom_tree() -> AomNode {
        AomNode::new(0, Role::Document)
            .with_name("Document Reading Reference Fixture")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 })
                    .with_text("The Accessibility Engine Specification"),
                AomNode::new(2, Role::Paragraph).with_text(
                    "The screen reader engine parses the document object model into a clean virtual buffer.",
                ),
                AomNode::new(3, Role::Paragraph).with_text(
                    "Engine components operate deterministically across synthetic speech pipelines. Speed and accuracy remain paramount.",
                ),
                AomNode::new(4, Role::Paragraph).with_text(
                    "Rust provides memory safety without garbage collection latency. This ensures predictable audio delivery.",
                ),
            ])
    }

    #[test]
    fn test_aom_tree_and_buffer_linearization() {
        let aom = create_test_aom_tree();
        assert_eq!(aom.role, Role::Document);
        assert_eq!(
            aom.name.as_deref(),
            Some("Document Reading Reference Fixture")
        );

        let buffer = VirtualBuffer::from_aom(&aom);
        assert_eq!(buffer.lines.len(), 5);

        // Line 0: Title
        assert_eq!(buffer.lines[0].text, "Document Reading Reference Fixture");
        assert_eq!(buffer.lines[0].role, Role::Document);

        // Line 1: Heading
        assert_eq!(
            buffer.lines[1].text,
            "The Accessibility Engine Specification"
        );
        assert_eq!(buffer.lines[1].role, Role::Heading { level: 1 });

        // Line 2: Paragraph 1
        assert_eq!(
            buffer.lines[2].text,
            "The screen reader engine parses the document object model into a clean virtual buffer."
        );
        assert_eq!(buffer.lines[2].role, Role::Paragraph);
    }

    #[test]
    fn test_scenario_01_say_all() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);

        let speech = cursor.say_all();
        assert!(speech.contains("The Accessibility Engine Specification"));
        assert!(speech.contains("clean virtual buffer"));
        assert!(speech.contains(" \u{001d} heading level 1 \u{001e} "));
    }

    #[test]
    fn test_scenario_02_and_03_line_navigation() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        // Reset: top of document
        cursor.move_to_start();

        // 02-doc-next-line: Down arrow
        let down_speech = cursor.move_next_line().expect("Should move down");
        assert_eq!(
            down_speech,
            " \u{001d} heading level 1 \u{001e} The Accessibility Engine Specification"
        );

        // 03-doc-prior-line: Up arrow
        let up_speech = cursor.move_prior_line().expect("Should move up");
        assert_eq!(up_speech, "Document Reading Reference Fixture");
    }

    #[test]
    fn test_scenario_04_and_05_word_navigation() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        // 04-doc-next-word: Ctrl+Right
        let next_word = cursor.move_next_word().expect("Should move next word");
        assert_eq!(next_word, "Reading");

        // 05-doc-prior-word: Ctrl+Left
        let prior_word = cursor.move_prior_word().expect("Should move prior word");
        assert_eq!(prior_word, "Document");
    }

    #[test]
    fn test_scenario_06_and_07_char_navigation() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        // 06-doc-next-char: Right Arrow
        let next_char = cursor.move_next_char().expect("Should move next char");
        assert_eq!(next_char, "o");

        // 07-doc-prior-char: Left Arrow
        let prior_char = cursor.move_prior_char().expect("Should move prior char");
        assert_eq!(prior_char, "D");
    }

    #[test]
    fn test_scenario_08_say_current_line() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();
        cursor.move_next_line(); // Move to heading

        let line_speech = cursor.current_line();
        assert_eq!(
            line_speech,
            " \u{001d} heading level 1 \u{001e} The Accessibility Engine Specification"
        );
    }

    #[test]
    fn test_scenario_09_say_current_word() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();
        cursor.move_next_line(); // Move to heading start ("The ...")

        let word_speech = cursor.current_word();
        assert_eq!(word_speech, "The");
    }

    #[test]
    fn test_scenario_10_next_paragraph() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let next_p = cursor
            .move_next_paragraph()
            .expect("Should move to next paragraph");
        assert_eq!(
            next_p,
            " \u{001d} heading level 1 \u{001e} The Accessibility Engine Specification"
        );
    }

    fn create_structural_test_tree() -> AomNode {
        AomNode::new(0, Role::Document)
            .with_name("Structural Navigation Reference Fixture")
            .with_children(vec![
                AomNode::new(1, Role::Landmark { landmark_type: "banner".to_string() })
                    .with_children(vec![
                        AomNode::new(2, Role::Heading { level: 1 })
                            .with_name("System Documentation")
                            .with_text("System Documentation"),
                        AomNode::new(3, Role::Landmark { landmark_type: "navigation".to_string() })
                            .with_name("Main Navigation")
                            .with_children(vec![
                                AomNode::new(4, Role::List)
                                    .with_children(vec![
                                        AomNode::new(5, Role::ListItem)
                                            .with_children(vec![
                                                AomNode::new(6, Role::Link { href: Some("#overview".to_string()) })
                                                    .with_name("Overview Link")
                                                    .with_text("Overview Link"),
                                            ]),
                                        AomNode::new(7, Role::ListItem)
                                            .with_children(vec![
                                                AomNode::new(8, Role::Link { href: Some("#architecture".to_string()) })
                                                    .with_name("Architecture Link")
                                                    .with_text("Architecture Link"),
                                            ]),
                                        AomNode::new(9, Role::ListItem)
                                            .with_children(vec![
                                                AomNode::new(10, Role::Link { href: Some("#download".to_string()) })
                                                    .with_name("Download Link")
                                                    .with_text("Download Link"),
                                            ]),
                                    ]),
                            ]),
                    ]),
                AomNode::new(11, Role::Landmark { landmark_type: "main".to_string() })
                    .with_children(vec![
                        AomNode::new(12, Role::Generic)
                            .with_children(vec![
                                AomNode::new(13, Role::Heading { level: 2 })
                                    .with_name("Section Overview")
                                    .with_text("Section Overview"),
                                AomNode::new(14, Role::Paragraph)
                                    .with_text("This section outlines high-level concepts for synthetic speech evaluation."),
                                AomNode::new(15, Role::Heading { level: 3 })
                                    .with_name("Subsection Details")
                                    .with_text("Subsection Details"),
                                AomNode::new(16, Role::Paragraph)
                                    .with_text("Details covering buffer generation and quick navigation keys."),
                            ]),
                        AomNode::new(17, Role::Generic)
                            .with_children(vec![
                                AomNode::new(18, Role::Heading { level: 2 })
                                    .with_name("Architecture Guide")
                                    .with_text("Architecture Guide"),
                                AomNode::new(19, Role::Paragraph)
                                    .with_text("Architecture description with ordered components:"),
                                AomNode::new(20, Role::List)
                                    .with_children(vec![
                                        AomNode::new(21, Role::ListItem).with_text("DOM Ingestion Layer"),
                                        AomNode::new(22, Role::ListItem).with_text("Virtual Buffer Engine"),
                                        AomNode::new(23, Role::ListItem).with_text("Speech Synthesis Output"),
                                    ]),
                            ]),
                    ]),
                AomNode::new(24, Role::Landmark { landmark_type: "complementary".to_string() })
                    .with_name("Quick Reference")
                    .with_children(vec![
                        AomNode::new(25, Role::Heading { level: 2 })
                            .with_name("Helpful Resources")
                            .with_text("Helpful Resources"),
                        AomNode::new(26, Role::Paragraph)
                            .with_text("Reference documentation and guidelines."),
                    ]),
                AomNode::new(27, Role::Landmark { landmark_type: "contentinfo".to_string() })
                    .with_children(vec![
                        AomNode::new(28, Role::Paragraph).with_text("Copyright 2026 Accessibility Lab."),
                    ]),
            ])
    }

    #[test]
    fn test_scenario_11_next_heading() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_heading().expect("Heading 1");
        assert_eq!(
            s1,
            "System Documentation\n \u{001d} heading level 1 \u{001e}"
        );

        let s2 = cursor.move_next_heading().expect("Heading 2");
        assert_eq!(s2, "Section Overview\n \u{001d} heading level 2 \u{001e}");

        let prior = cursor.move_prior_heading().expect("Prior heading");
        assert_eq!(
            prior,
            "System Documentation\n \u{001d} heading level 1 \u{001e}"
        );
    }

    #[test]
    fn test_scenario_12_heading_level_2() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_heading_level(2).expect("First level 2");
        assert_eq!(s1, "Section Overview\n \u{001d} heading level 2 \u{001e}");

        let s2 = cursor.move_next_heading_level(2).expect("Second level 2");
        assert_eq!(s2, "Architecture Guide\n \u{001d} heading level 2 \u{001e}");

        let prior = cursor.move_prior_heading_level(2).expect("Prior level 2");
        assert_eq!(
            prior,
            "Section Overview\n \u{001d} heading level 2 \u{001e}"
        );
    }

    #[test]
    fn test_scenario_13_link_navigation() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_link().expect("Link 1");
        assert_eq!(s1, "•  \u{001d} same page link \u{001e} Overview Link");

        let s2 = cursor.move_next_link().expect("Link 2");
        assert_eq!(s2, "•  \u{001d} same page link \u{001e} Architecture Link");

        let prior = cursor.move_prior_link().expect("Prior link");
        assert_eq!(prior, "•  \u{001d} same page link \u{001e} Overview Link");
    }

    #[test]
    fn test_scenario_14_list_navigation() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_list().expect("List 1");
        assert_eq!(s1, "List of 3 items");

        let s2 = cursor.move_next_list_item().expect("List item 1");
        assert_eq!(s2, "•  \u{001d} same page link \u{001e} Overview Link");

        let s3 = cursor.move_next_list_item().expect("List item 2");
        assert_eq!(s3, "•  \u{001d} same page link \u{001e} Architecture Link");
    }

    #[test]
    fn test_scenario_15_landmark_navigation() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_landmark().expect("Landmark 1");
        assert_eq!(s1, "banner");

        let s2 = cursor.move_next_landmark().expect("Landmark 2");
        assert_eq!(s2, "Main Navigation navigation region");

        let prior = cursor.move_prior_landmark().expect("Prior landmark");
        assert_eq!(prior, "banner");
    }

    fn create_forms_test_tree() -> AomNode {
        AomNode::new(0, Role::Document)
            .with_name("Forms and Interaction Reference Fixture")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 }).with_text("Account Registration Form"),
                AomNode::new(2, Role::Generic).with_children(vec![
                    AomNode::new(3, Role::Edit)
                        .with_name("Full Name:")
                        .with_value("Jane Doe"),
                    AomNode::new(4, Role::Edit)
                        .with_name("Email Address:")
                        .with_value("jane@example.com"),
                    {
                        let mut cb = AomNode::new(5, Role::CheckBox)
                            .with_name("I accept terms and conditions");
                        cb.states.checked = Some(false);
                        cb
                    },
                    AomNode::new(6, Role::Button)
                        .with_name("Save Preferences")
                        .with_text("Save Preferences"),
                    AomNode::new(7, Role::Button)
                        .with_name("Submit Application")
                        .with_text("Submit Application"),
                ]),
            ])
    }

    #[test]
    fn test_scenario_16_form_control_navigation() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_form_control().expect("First form control");
        assert_eq!(s1, "Full Name:  \u{001d} Edit \u{001e}  \nJane Doe");

        let s2 = cursor
            .move_next_form_control()
            .expect("Second form control");
        assert_eq!(
            s2,
            "Email Address:  \u{001d} Edit \u{001e}  \njane@example.com"
        );

        let s3 = cursor.move_next_form_control().expect("Third form control");
        assert_eq!(
            s3,
            " I accept terms and conditions  \u{001d} check box \u{001e}   \u{001d}not checked\u{001e}"
        );
    }

    #[test]
    fn test_scenario_17_button_navigation() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let b1 = cursor.move_next_button().expect("First button");
        assert_eq!(b1, "Save Preferences  \u{001d} Button \u{001e}");

        let b2 = cursor.move_next_button().expect("Second button");
        assert_eq!(b2, "Submit Application  \u{001d} Button \u{001e}");
    }

    #[test]
    fn test_scenario_18_edit_navigation() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let e1 = cursor.move_next_edit().expect("First edit");
        assert_eq!(e1, "Full Name:  \u{001d} Edit \u{001e}  \nJane Doe");

        let e2 = cursor.move_next_edit().expect("Second edit");
        assert_eq!(
            e2,
            "Email Address:  \u{001d} Edit \u{001e}  \njane@example.com"
        );
    }

    #[test]
    fn test_scenario_19_forms_mode_entry_exit() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        cursor.move_next_edit().expect("Move to edit");
        assert!(!cursor.forms_mode_active);

        let entry = cursor.enter_forms_mode().expect("Enter forms mode");
        assert!(cursor.forms_mode_active);
        assert!(entry.contains("Type in text"));
        assert!(entry.contains("Full Name:"));

        let exit = cursor.exit_forms_mode().expect("Exit forms mode");
        assert!(!cursor.forms_mode_active);
        assert_eq!(exit, "virtual PC Cursor");
    }

    #[test]
    fn test_scenario_20_checkbox_navigation_and_toggle() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let cb = cursor.move_next_checkbox().expect("Move to checkbox");
        assert!(cb.contains("not checked"));
        assert_eq!(
            cursor.current_node().and_then(|n| n.states.checked),
            Some(false)
        );

        let toggle = cursor.toggle_active_control().expect("Toggle checkbox");
        assert!(toggle.contains("checked"));
        assert_eq!(
            cursor.current_node().and_then(|n| n.states.checked),
            Some(true)
        );
    }
}
