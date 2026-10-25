pub mod actions;
pub mod announcement;
pub mod aom;
pub mod apps;
pub mod buffer;
pub mod clock;
pub mod color;
pub mod command;
pub mod cursor;
pub mod editing;
pub mod element_lists;
pub mod engine;
pub mod event;
pub mod flexible_web;
pub mod input;
pub mod keyboard_help;
pub mod labels;
pub mod linearize;
pub mod live_text;
pub mod messages;
pub mod native_focus;
pub mod placemarkers;
pub mod pronounce;
pub mod recognized_file;
pub mod retire;
pub mod screen_echo;
pub mod screen_model;
pub mod settings;
pub mod settings_layers;
pub mod sound;
pub mod speech;
pub mod text_analysis;
pub mod ui;
pub mod viewer;
pub mod viewer_text;
pub mod word_list;

pub use aom::{AomNode, NodeStates, Role};
pub use buffer::{
    BufferLine, BufferParagraph, BufferSpan, BufferWord, FormFieldListItem, HeadingListItem,
    LinkListItem, VirtualBuffer,
};
pub use cursor::{SearchState, VirtualCursor};
pub use engine::ScreenReaderEngine;
pub use event::{
    AccessibilityEvent, ChangeKind, DocumentField, DocumentHeading, DocumentLink,
    DocumentParagraph, FocusEvent, FontInfo, GroupEntry, LineSpacing, LineStructure,
    LiveRegionEvent, MenuEvent, MousePoint, PlatformIds, Politeness, TableCell, TextContext,
    UnitTexts, WindowItem,
};
pub use input::{Key, KeyAction, KeyEvent, Modifiers};
pub use speech::{MockSpeechOutput, SpeechFormatter, SpeechOutput};

pub fn init() {
    tracing::info!("okur-core initialized");
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

    /// At the page's top Left says the first character again and Ctrl+Left the first word
    /// [obs, 957/s6-s8, s20].
    #[test]
    fn left_and_ctrl_left_at_the_top_say_the_first_character_and_word_again() {
        let mut cursor = VirtualCursor::new(VirtualBuffer::from_aom(&create_test_aom_tree()));
        cursor.move_to_start();
        assert_eq!(cursor.move_prior_char().as_deref(), Some("D"));
        assert_eq!(cursor.move_prior_char().as_deref(), Some("D"));
        assert_eq!(cursor.offset, 0);
        let first_word = cursor.current_word();
        assert_eq!(cursor.move_prior_word(), Some(first_word));
        assert_eq!(cursor.offset, 0);
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

        // The first one says the level 1 heading passed: see
        // `heading_level_keys_say_the_section_heading_entered` [obs, 12/s1].
        let _ = cursor.move_next_heading_level(2);

        let s2 = cursor.move_next_heading_level(2).unwrap_or_default();
        assert_eq!(s2, "Architecture Guide\n \u{001d} heading level 2 \u{001e}");

        let prior = cursor.move_prior_heading_level(2).unwrap_or_default();
        assert_eq!(
            prior,
            "Section Overview\n \u{001d} heading level 2 \u{001e}"
        );
    }

    /// From the top of the page, 2 enters the level 1 heading's section: JAWS says that
    /// heading first, then the level 2 heading [obs, 12/s1, 104/s1].
    #[test]
    fn heading_level_keys_say_the_section_heading_entered() {
        let mut cursor =
            VirtualCursor::new(VirtualBuffer::from_aom(&create_structural_test_tree()));
        cursor.move_to_start();
        assert_eq!(
            cursor.move_next_heading_level(2).unwrap_or_default(),
            "System Documentation \u{001d} heading level 1 \u{001e} \nSection Overview\n \u{001d} heading level 2 \u{001e}"
        );
    }

    /// Scenario 275's page: two level 1 sections, each with level 2 and 3 headings.
    fn sections_tree() -> AomNode {
        let headings = [
            (1, "User guide"),
            (2, "Install"),
            (3, "Windows"),
            (3, "macOS"),
            (2, "Configure"),
            (3, "Voices"),
            (1, "Reference"),
            (2, "Commands"),
            (3, "Reading"),
            (2, "Settings"),
        ];
        let mut children = Vec::new();
        for (id, (level, text)) in (1usize..).step_by(2).zip(headings) {
            children.push(
                AomNode::new(id, Role::Heading { level })
                    .with_name(text)
                    .with_text(text),
            );
            children
                .push(AomNode::new(id + 1, Role::Paragraph).with_text(format!("About {text}.")));
        }
        AomNode::new(0, Role::Document)
            .with_name("Headings in Sections")
            .with_children(children)
    }

    /// The headings above the level that a level key walks past, still open where it stops,
    /// are said before the heading it reaches; going back too [obs, 275].
    #[test]
    fn heading_level_keys_say_the_headings_passed() {
        let mut cursor = VirtualCursor::new(VirtualBuffer::from_aom(&sections_tree()));
        cursor.move_to_start();
        let mut key = |level: u32, forward: bool| {
            let said = if forward {
                cursor.move_next_heading_level(level)
            } else {
                cursor.move_prior_heading_level(level)
            };
            said.unwrap_or_default()
                .replace("\n \u{001d} heading level ", "/")
                .replace(" \u{001d} heading level ", "/")
                .replace(" \u{001e} \n", " | ")
                .replace(" \u{001e}", "")
        };
        assert_eq!(key(3, true), "User guide/1 | Install/2 | Windows/3");
        assert_eq!(key(3, true), "macOS/3");
        assert_eq!(key(3, true), "Configure/2 | Voices/3");
        assert_eq!(key(3, true), "Reference/1 | Commands/2 | Reading/3");
        assert_eq!(key(3, false), "Reference/1 | Voices/3");
        assert_eq!(key(2, false), "Configure/2");
        assert_eq!(key(2, false), "Install/2");
        assert_eq!(key(1, true), "Reference/1");
        assert_eq!(key(1, false), "User guide/1");
        // From a heading, the walk forward leaves it out and the walk back counts it [obs,
        // 277/s2, s4, s11].
        assert_eq!(key(2, true), "Install/2");
        assert_eq!(key(1, true), "Reference/1");
        assert_eq!(key(2, false), "Reference/1 | Configure/2");
        assert_eq!(key(2, true), "Reference/1 | Commands/2");
        assert_eq!(key(2, true), "Settings/2");
        assert_eq!(key(3, false), "Settings/2 | Reading/3");
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
        assert_eq!(
            norm(&b1),
            norm("Save Preferences  \u{001d} Button \u{001e}")
        );

        let b2 = cursor.move_next_button().unwrap_or_default();
        assert_eq!(
            norm(&b2),
            norm("Submit Application  \u{001d} Button \u{001e}")
        );
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
        // The cursor is on the first cell: Say Line reads it [obs, 257/s4].
        assert_eq!(
            cursor.current_node().map(|n| &n.role),
            Some(&Role::TableCell)
        );
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

        // Edge check at col 1: "beginning of row" [obs, 615/s2]
        let edge_left = cursor.move_table_cell_left().unwrap_or_default();
        assert_eq!(edge_left, "beginning of row");

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

        // Edge check at row 1: "top of column" [obs, 592/s8]
        let edge_up = cursor.move_table_cell_up().unwrap_or_default();
        assert_eq!(edge_up, "top of column");
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

    /// An engine with Okur's windows, and the dialog it showed for `command`.
    fn open_list_dialog(
        aom: &AomNode,
        command: &str,
    ) -> (ScreenReaderEngine, ui::RecordingUiHost, ui::DialogSpec) {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(aom));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let host = ui::RecordingUiHost::default();
        engine.set_ui(Box::new(host.clone()));
        let _ = engine.execute_command(command);
        let dialog = host.take().into_iter().find_map(|message| match message {
            ui::ToUi::DialogShow { dialog } => Some(dialog),
            _ => None,
        });
        assert!(dialog.is_some(), "{command} showed no dialog");
        let dialog = dialog.unwrap_or_else(|| ui::DialogSpec {
            id: 0,
            title: String::new(),
            controls: Vec::new(),
            keys: Vec::new(),
        });
        (engine, host, dialog)
    }

    /// The dialog's list: its id and items.
    fn list_of(dialog: &ui::DialogSpec) -> (String, Vec<String>) {
        dialog
            .controls
            .iter()
            .find_map(|c| match &c.kind {
                ui::ControlKind::List { items, .. } => Some((c.id.clone(), items.clone())),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// What the dialog's window reports: an item selected, then a button pressed.
    fn choose(
        engine: &mut ScreenReaderEngine,
        dialog: &ui::DialogSpec,
        item: usize,
        button: &str,
    ) -> String {
        let (list, _) = list_of(dialog);
        let _ =
            engine.handle_accessibility_event(&AccessibilityEvent::Ui(ui::FromUi::DialogChanged {
                id: dialog.id,
                control: list,
                value: ui::Value::Selected(item),
            }));
        engine
            .handle_accessibility_event(&AccessibilityEvent::Ui(ui::FromUi::DialogPressed {
                id: dialog.id,
                control: button.to_owned(),
            }))
            .unwrap_or_default()
    }

    #[test]
    fn test_milestone_11_headings_list_and_jump() {
        let (mut engine, _host, dialog) =
            open_list_dialog(&create_structural_test_tree(), "SelectAHeading");
        assert_eq!(dialog.title, "Heading List");
        let (_, items) = list_of(&dialog);
        assert_eq!(items.len(), 5);
        assert_eq!(
            items.first().map(String::as_str),
            Some("System Documentation : 1")
        );
        // Move To Heading on the second: the virtual cursor is on it.
        let landing = choose(&mut engine, &dialog, 1, "move");
        assert!(landing.contains("Section Overview"), "{landing:?}");
        assert!(landing.contains("heading level 2"), "{landing:?}");
        assert_eq!(
            engine.cursor.current_node().map(|n| n.role.clone()),
            Some(Role::Heading { level: 2 })
        );
    }

    #[test]
    fn test_milestone_11_links_list_and_jump() {
        let (mut engine, _host, dialog) =
            open_list_dialog(&create_structural_test_tree(), "SelectALink");
        assert_eq!(dialog.title, "Links List");
        let (_, items) = list_of(&dialog);
        assert_eq!(items.len(), 3);
        let landing = choose(&mut engine, &dialog, 1, "move");
        assert!(landing.contains("Architecture Link"), "{landing:?}");
        assert!(landing.contains("same page link"), "{landing:?}");
        assert!(engine
            .cursor
            .current_node()
            .is_some_and(|n| matches!(n.role, Role::Link { .. })));
    }

    #[test]
    fn test_milestone_11_form_fields_list_and_escape() {
        let (mut engine, _host, dialog) =
            open_list_dialog(&create_forms_test_tree(), "SelectAFormField");
        assert_eq!(dialog.title, "Select a Form Field");
        let (_, items) = list_of(&dialog);
        assert!(
            items.iter().any(|i| i.starts_with("Full Name: Edit")),
            "{items:?}"
        );
        // Escape closes the window; the page comes back.
        let _ =
            engine.handle_accessibility_event(&AccessibilityEvent::Ui(ui::FromUi::DialogClosed {
                id: dialog.id,
            }));
        assert_eq!(engine.app_mode, engine::AppMode::VirtualBuffer);
    }
}
