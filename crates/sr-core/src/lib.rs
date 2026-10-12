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
}
