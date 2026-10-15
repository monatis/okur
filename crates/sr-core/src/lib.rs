pub mod actions;
pub mod aom;
pub mod apps;
pub mod buffer;
pub mod clock;
pub mod cursor;
pub mod editing;
pub mod engine;
pub mod event;
pub mod input;
pub mod linearize;
pub mod native_focus;
pub mod pronounce;
pub mod settings;
pub mod speech;
pub mod viewer;

pub use aom::{AomNode, NodeStates, Role};
pub use buffer::{
    BufferLine, BufferParagraph, BufferSpan, BufferWord, FormFieldListItem, HeadingListItem,
    LinkListItem, VirtualBuffer,
};
pub use cursor::{SearchState, VirtualCursor};
pub use engine::{ActiveElementList, ScreenReaderEngine};
pub use event::{
    AccessibilityEvent, CaretUnit, ChangeKind, FocusEvent, FontInfo, LiveRegionEvent, MenuEvent,
    Politeness, TextContext, WindowItem,
};
pub use input::{Key, KeyAction, KeyEvent, Modifiers};
pub use speech::{MockSpeechOutput, SpeechFormatter, SpeechOutput};

pub fn init() {
    tracing::info!("sr-core initialized");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whitespace-insensitive speech, the same equality the JAWS parity harness uses.
    fn norm(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

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
        assert_eq!(
            buffer.lines.first().map(|l| l.text.as_str()),
            Some("Document Reading Reference Fixture")
        );
        assert_eq!(
            buffer.lines.first().map(|l| l.role.clone()),
            Some(Role::Document)
        );

        // Line 1: Heading
        assert_eq!(
            buffer.lines.get(1).map(|l| l.text.as_str()),
            Some("The Accessibility Engine Specification")
        );
        assert_eq!(
            buffer.lines.get(1).map(|l| l.role.clone()),
            Some(Role::Heading { level: 1 })
        );

        // Line 2: Paragraph 1
        assert_eq!(
            buffer.lines.get(2).map(|l| l.text.as_str()),
            Some("The screen reader engine parses the document object model into a clean virtual buffer.")
        );
        assert_eq!(
            buffer.lines.get(2).map(|l| l.role.clone()),
            Some(Role::Paragraph)
        );
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
        let down_speech = cursor.move_next_line().unwrap_or_default();
        assert_eq!(
            down_speech,
            " \u{001d} heading level 1 \u{001e} The Accessibility Engine Specification"
        );

        // 03-doc-prior-line: Up arrow
        let up_speech = cursor.move_prior_line().unwrap_or_default();
        assert_eq!(up_speech, "Document Reading Reference Fixture");
    }

    #[test]
    fn test_scenario_04_and_05_word_navigation() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        // 04-doc-next-word: Ctrl+Right
        let next_word = cursor.move_next_word().unwrap_or_default();
        assert_eq!(next_word, "Reading");

        // 05-doc-prior-word: Ctrl+Left
        let prior_word = cursor.move_prior_word().unwrap_or_default();
        assert_eq!(prior_word, "Document");
    }

    #[test]
    fn test_scenario_06_and_07_char_navigation() {
        let aom = create_test_aom_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        // 06-doc-next-char: Right Arrow
        let next_char = cursor.move_next_char().unwrap_or_default();
        assert_eq!(next_char, "o");

        // 07-doc-prior-char: Left Arrow
        let prior_char = cursor.move_prior_char().unwrap_or_default();
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

        let next_p = cursor.move_next_paragraph().unwrap_or_default();
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

        let s1 = cursor.move_next_heading().unwrap_or_default();
        assert_eq!(
            s1,
            "System Documentation\n \u{001d} heading level 1 \u{001e}"
        );

        let s2 = cursor.move_next_heading().unwrap_or_default();
        assert_eq!(s2, "Section Overview\n \u{001d} heading level 2 \u{001e}");

        let prior = cursor.move_prior_heading().unwrap_or_default();
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

        let s1 = cursor.move_next_heading_level(2).unwrap_or_default();
        assert_eq!(s1, "Section Overview\n \u{001d} heading level 2 \u{001e}");

        let s2 = cursor.move_next_heading_level(2).unwrap_or_default();
        assert_eq!(s2, "Architecture Guide\n \u{001d} heading level 2 \u{001e}");

        let prior = cursor.move_prior_heading_level(2).unwrap_or_default();
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

        // The bullet JAWS speaks comes from the list marker text leaf, which real Chrome trees
        // carry (covered by 105-article-links parity); this hand-written tree has none.
        let s1 = cursor.move_next_link().unwrap_or_default();
        assert_eq!(
            norm(&s1),
            norm("\u{001d} same page link \u{001e} Overview Link")
        );

        let s2 = cursor.move_next_link().unwrap_or_default();
        assert_eq!(
            norm(&s2),
            norm("\u{001d} same page link \u{001e} Architecture Link")
        );

        let prior = cursor.move_prior_link().unwrap_or_default();
        assert_eq!(
            norm(&prior),
            norm("\u{001d} same page link \u{001e} Overview Link")
        );
    }

    #[test]
    fn test_scenario_14_list_navigation() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_list().unwrap_or_default();
        // JAWS says "list of 3 items" (lowercase): corpus/evidence/14-struct-next-list.
        assert_eq!(s1, "list of 3 items");

        let s2 = cursor.move_next_list_item().unwrap_or_default();
        assert_eq!(
            norm(&s2),
            norm("\u{001d} same page link \u{001e} Overview Link")
        );

        let s3 = cursor.move_next_list_item().unwrap_or_default();
        assert_eq!(
            norm(&s3),
            norm("\u{001d} same page link \u{001e} Architecture Link")
        );
    }

    #[test]
    fn test_scenario_15_landmark_navigation() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let s1 = cursor.move_next_landmark().unwrap_or_default();
        assert_eq!(s1, "banner");

        let s2 = cursor.move_next_landmark().unwrap_or_default();
        assert_eq!(s2, "Main Navigation navigation region");

        let prior = cursor.move_prior_landmark().unwrap_or_default();
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

        let s1 = cursor.move_next_form_control().unwrap_or_default();
        assert_eq!(
            norm(&s1),
            norm("Full Name:  \u{001d} Edit \u{001e}  \nJane Doe")
        );

        let s2 = cursor.move_next_form_control().unwrap_or_default();
        assert_eq!(
            norm(&s2),
            norm("Email Address:  \u{001d} Edit \u{001e}  \njane@example.com")
        );

        let s3 = cursor.move_next_form_control().unwrap_or_default();
        assert_eq!(
            norm(&s3),
            norm(" I accept terms and conditions  \u{001d} check box \u{001e}   \u{001d}not checked\u{001e}")
        );
    }

    #[test]
    fn test_scenario_17_button_navigation() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let b1 = cursor.move_next_button().unwrap_or_default();
        assert_eq!(b1, "Save Preferences  \u{001d} Button \u{001e}");

        let b2 = cursor.move_next_button().unwrap_or_default();
        assert_eq!(b2, "Submit Application  \u{001d} Button \u{001e}");
    }

    #[test]
    fn test_scenario_18_edit_navigation() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let e1 = cursor.move_next_edit().unwrap_or_default();
        assert_eq!(
            norm(&e1),
            norm("Full Name:  \u{001d} Edit \u{001e}  \nJane Doe")
        );

        let e2 = cursor.move_next_edit().unwrap_or_default();
        assert_eq!(
            norm(&e2),
            norm("Email Address:  \u{001d} Edit \u{001e}  \njane@example.com")
        );
    }

    #[test]
    fn test_scenario_19_forms_mode_entry_exit() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let _ = cursor.move_next_edit();
        assert!(!cursor.forms_mode_active);

        let entry = cursor.enter_forms_mode().unwrap_or_default();
        assert!(cursor.forms_mode_active);
        assert!(entry.contains("Type in text"));
        assert!(entry.contains("Full Name:"));

        let exit = cursor.exit_forms_mode().unwrap_or_default();
        assert!(!cursor.forms_mode_active);
        assert_eq!(exit, "virtual PC Cursor");
    }

    #[test]
    fn test_scenario_20_checkbox_navigation_and_toggle() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        let cb = cursor.move_next_checkbox().unwrap_or_default();
        assert!(cb.contains("not checked"));
        assert_eq!(
            cursor.current_node().and_then(|n| n.states.checked),
            Some(false)
        );

        let toggle = cursor.toggle_active_control().unwrap_or_default();
        assert!(toggle.contains("checked"));
        assert_eq!(
            cursor.current_node().and_then(|n| n.states.checked),
            Some(true)
        );
    }

    fn create_table_test_tree() -> AomNode {
        AomNode::new(0, Role::Document)
            .with_name("Tables Navigation Reference Fixture")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 })
                    .with_text("Quarterly Financial Summary"),
                AomNode::new(2, Role::Table)
                    .with_name("Q3 Financial Summary Table")
                    .with_children(vec![
                        AomNode::new(3, Role::TableRow).with_children(vec![
                            AomNode::new(4, Role::TableCell).with_text("Quarter"),
                            AomNode::new(5, Role::TableCell).with_text("Revenue"),
                            AomNode::new(6, Role::TableCell).with_text("Expenses"),
                            AomNode::new(7, Role::TableCell).with_text("Profit"),
                        ]),
                        AomNode::new(8, Role::TableRow).with_children(vec![
                            AomNode::new(9, Role::TableCell).with_text("Q1"),
                            AomNode::new(10, Role::TableCell).with_text("$10,000"),
                            AomNode::new(11, Role::TableCell).with_text("$7,000"),
                            AomNode::new(12, Role::TableCell).with_text("$3,000"),
                        ]),
                        AomNode::new(13, Role::TableRow).with_children(vec![
                            AomNode::new(14, Role::TableCell).with_text("Q2"),
                            AomNode::new(15, Role::TableCell).with_text("$14,000"),
                            AomNode::new(16, Role::TableCell).with_text("$8,500"),
                            AomNode::new(17, Role::TableCell).with_text("$5,500"),
                        ]),
                        AomNode::new(18, Role::TableRow).with_children(vec![
                            AomNode::new(19, Role::TableCell).with_text("Q3"),
                            AomNode::new(20, Role::TableCell).with_text("$18,000"),
                            AomNode::new(21, Role::TableCell).with_text("$9,200"),
                            AomNode::new(22, Role::TableCell).with_text("$8,800"),
                        ]),
                    ]),
            ])
    }

    #[test]
    fn test_scenarios_21_through_25_tables() {
        let aom = create_table_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        assert_eq!(buffer.tables.len(), 1);
        assert_eq!(buffer.tables.first().map(|t| t.rows), Some(4));
        assert_eq!(buffer.tables.first().map(|t| t.cols), Some(4));

        let mut cursor = VirtualCursor::new(buffer);
        cursor.move_to_start();

        // 21-table-next-table
        let entry = cursor.move_next_table().unwrap_or_default();
        assert_eq!(
            entry,
            "table with 4 columns and 4 rows\nQ3 Financial Summary Table\nColumn 1, Row 1\nQuarter"
        );
        assert_eq!(cursor.current_node().map(|n| &n.role), Some(&Role::Table));
        assert_eq!(cursor.current_table_col(), Some(1));
        assert_eq!(cursor.current_table_row(), Some(1));

        // 22-table-cell-right
        let right_speech = cursor.move_table_cell_right().unwrap_or_default();
        assert_eq!(right_speech, "Revenue\nColumn 2 of 4");
        assert_eq!(cursor.current_table_col(), Some(2));
        assert_eq!(cursor.current_table_row(), Some(1));

        // 23-table-cell-left
        let left_speech = cursor.move_table_cell_left().unwrap_or_default();
        assert_eq!(left_speech, "Quarter\nColumn 1 of 4");
        assert_eq!(cursor.current_table_col(), Some(1));
        assert_eq!(cursor.current_table_row(), Some(1));

        // Edge check at col 1
        let edge_left = cursor.move_table_cell_left().unwrap_or_default();
        assert_eq!(edge_left, "edge of table");

        // 24-table-row-down
        let down_speech = cursor.move_table_cell_down().unwrap_or_default();
        assert_eq!(down_speech, "Q1\nRow 2 of 4");
        assert_eq!(cursor.current_table_col(), Some(1));
        assert_eq!(cursor.current_table_row(), Some(2));

        // 25-table-row-up
        let up_speech = cursor.move_table_cell_up().unwrap_or_default();
        assert_eq!(up_speech, "Quarter\nRow 1 of 4");
        assert_eq!(cursor.current_table_col(), Some(1));
        assert_eq!(cursor.current_table_row(), Some(1));

        // Edge check at row 1
        let edge_up = cursor.move_table_cell_up().unwrap_or_default();
        assert_eq!(edge_up, "edge of table");
    }

    #[test]
    fn test_scenario_27_negative_table_search() {
        let empty_doc = AomNode::new(0, Role::Document).with_name("Empty Page");
        let buffer = VirtualBuffer::from_aom(&empty_doc);
        let mut cursor = VirtualCursor::new(buffer);

        let res = cursor.move_next_table().unwrap_or_default();
        assert_eq!(res, "There are no tables on this page.");
    }

    #[test]
    fn test_milestone_11_in_buffer_search_forward_and_wrap() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();

        // 1. Forward search for "Architecture"
        let match1 = cursor.find_text("Architecture", false).unwrap_or_default();
        assert!(
            match1.contains("Architecture Link"),
            "Expected match1 to contain Architecture Link, got {}",
            match1
        );

        // 2. Advance via find_next to "Architecture Guide"
        let match2 = cursor.find_next().unwrap_or_default();
        assert!(
            match2.contains("Architecture Guide"),
            "Expected match2 to contain Architecture Guide, got {}",
            match2
        );

        // 3. Advance to "Architecture description..."
        let match3 = cursor.find_next().unwrap_or_default();
        assert!(
            match3.contains("Architecture description with ordered components:"),
            "Expected match3 to contain paragraph, got {}",
            match3
        );

        // 4. Advance past last match -> wrap around to top!
        let wrap_top = cursor.find_next().unwrap_or_default();
        assert!(
            wrap_top.starts_with("wrapping to top"),
            "Expected wrapping to top, got {}",
            wrap_top
        );
        assert!(wrap_top.contains("Architecture Link"));

        // 5. Prior match from top -> wrap around to bottom!
        let wrap_bottom = cursor.find_prior().unwrap_or_default();
        assert!(
            wrap_bottom.starts_with("wrapping to bottom"),
            "Expected wrapping to bottom, got {}",
            wrap_bottom
        );
        assert!(wrap_bottom.contains("Architecture description"));
    }

    #[test]
    fn test_milestone_11_negative_search() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let mut cursor = VirtualCursor::new(buffer);

        cursor.move_to_start();
        let initial_offset = cursor.offset;

        let res = cursor
            .find_text("nonexistent-term", false)
            .unwrap_or_default();
        assert_eq!(res, "Text not found: nonexistent-term");
        assert_eq!(
            cursor.offset, initial_offset,
            "Cursor offset must not change on negative search"
        );
    }

    #[test]
    fn test_milestone_11_headings_list_and_jump() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Open Headings List dialog
        let dialog_speech = engine.execute_command("SelectAHeading").unwrap_or_default();
        assert!(dialog_speech.contains("Heading List"));
        assert!(dialog_speech.contains("headings  \u{001d} List view \u{001e}"));
        assert!(dialog_speech.contains("System Documentation : 1"));
        assert!(dialog_speech.contains("1 of 5"));

        // Navigate down to item 2 ("Section Overview : 2")
        let down_speech = engine.execute_command("{DOWN}").unwrap_or_default();
        assert!(down_speech.contains("Section Overview : 2"));
        assert!(down_speech.contains("2 of 5"));

        // Press Enter to jump to target heading
        let landing_speech = engine.execute_command("{ENTER}").unwrap_or_default();
        assert!(landing_speech.contains("Section Overview"));
        assert!(landing_speech.contains("heading level 2"));

        assert!(engine.cursor.current_node().is_some());
        if let Some(current_node) = engine.cursor.current_node() {
            assert_eq!(current_node.role, Role::Heading { level: 2 });
        }
    }

    #[test]
    fn test_milestone_11_links_list_and_jump() {
        let aom = create_structural_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Open Links List dialog
        let dialog_speech = engine.execute_command("SelectALink").unwrap_or_default();
        assert!(dialog_speech.contains("Links List"));
        assert!(dialog_speech.contains("links  \u{001d} List view \u{001e}"));
        assert!(dialog_speech.contains("Overview Link"));
        assert!(dialog_speech.contains("1 of 3"));

        // Navigate down to item 2 ("Architecture Link")
        let down_speech = engine.execute_command("{DOWN}").unwrap_or_default();
        assert!(down_speech.contains("Architecture Link"));
        assert!(down_speech.contains("2 of 3"));

        // Press Enter to jump to link
        let landing_speech = engine.execute_command("{ENTER}").unwrap_or_default();
        assert!(landing_speech.contains("Architecture Link"));
        assert!(landing_speech.contains("same page link"));

        assert!(engine.cursor.current_node().is_some());
        if let Some(current_node) = engine.cursor.current_node() {
            assert!(matches!(current_node.role, Role::Link { .. }));
        }
    }

    #[test]
    fn test_milestone_11_form_fields_list_and_escape() {
        let aom = create_forms_test_tree();
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Open Form Fields List dialog
        let dialog_speech = engine
            .execute_command("SelectAFormField")
            .unwrap_or_default();
        assert!(dialog_speech.contains("Select a Form Field"));
        assert!(dialog_speech.contains("List1  \u{001d} List view \u{001e}"));
        assert!(dialog_speech.contains("Full Name: Edit"));

        // Dismiss dialog via Escape
        let exit_speech = engine.execute_command("{ESC}").unwrap_or_default();
        assert_eq!(exit_speech, "virtual PC Cursor");
        assert!(engine.active_element_list.is_none());
    }
}
