//! The reading highlight (JAWS's visual tracking): a red rectangle around what the virtual
//! cursor read, the page scrolled to show it, so that a sighted person beside the user (a
//! teacher, a parent, a tester) can follow (`docs/references/jaws-dialogs-and-visual-features.md`
//! §3, from Freedom Scientific's guide, chapters 5 and 9; old-pc, 2026-11-02: a table key
//! framed the cell and scrolled the page to it).
//!
//! The element is framed, not the line: a line read in a long paragraph frames the paragraph,
//! where JAWS frames the line's text.

use super::{AppMode, ScreenReaderEngine};
use crate::ui::ToUi;

impl ScreenReaderEngine {
    /// The virtual cursor moved (a key, or a Say All reaching its next line): its element is
    /// scrolled into view and its place asked for, which comes back as
    /// [`crate::event::AccessibilityEvent::TrackedPlace`]. An element already framed is not
    /// asked for again.
    pub(super) fn track_virtual_cursor(&mut self) {
        if !self.settings.visual_tracking || self.app_mode != AppMode::VirtualBuffer {
            return;
        }
        let (Some(node), Some(actions)) = (self.node_under_cursor(), self.actions.as_ref()) else {
            return;
        };
        let id = node.id;
        if self.highlight_node == Some(id) {
            return;
        }
        if actions.track_node(node) {
            self.highlight_node = Some(id);
        }
    }

    /// The place of the element the virtual cursor is on: framed, unless the cursor has moved
    /// on meanwhile, or the element has no size (hidden, or scrolled out of reach).
    pub(super) fn show_reading_highlight(&mut self, node: usize, rect: [i32; 4]) {
        if !self.settings.visual_tracking
            || self.app_mode != AppMode::VirtualBuffer
            || self.highlight_node != Some(node)
        {
            return;
        }
        let [_, _, width, height] = rect;
        if width <= 0 || height <= 0 {
            self.hide_reading_highlight();
            return;
        }
        if let Some(ui) = &self.ui {
            ui.send(ToUi::HighlightShow { rect });
            self.highlight_shown = true;
        }
    }

    /// Takes the frame away: a new page, or the focus out of the page.
    pub(super) fn hide_reading_highlight(&mut self) {
        self.highlight_node = None;
        if self.highlight_shown {
            if let Some(ui) = &self.ui {
                ui.send(ToUi::HighlightHide);
            }
            self.highlight_shown = false;
        }
    }

    /// After a key or an event: the frame follows the virtual cursor while a page is read,
    /// and goes when the focus leaves the page (an Okur dialog, another window, forms mode's
    /// field keeps it).
    pub(super) fn update_reading_highlight(&mut self, cursor_moved: bool) {
        if self.app_mode != AppMode::VirtualBuffer {
            if self.highlight_node.is_some() || self.highlight_shown {
                self.hide_reading_highlight();
            }
            return;
        }
        if cursor_moved {
            self.track_virtual_cursor();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{AppMode, ScreenReaderEngine};
    use crate::actions::RecordingActions;
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::event::AccessibilityEvent;
    use crate::input::{Key, KeyEvent, Modifiers};
    use crate::speech::MockSpeechOutput;
    use crate::ui::{RecordingUiHost, ToUi};

    fn page() -> AomNode {
        AomNode::new(0, Role::Document)
            .with_name("Library")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 }).with_text("Riverside Library"),
                AomNode::new(2, Role::Paragraph).with_text("Open every day but Monday."),
                AomNode::new(3, Role::Heading { level: 2 }).with_text("Opening hours"),
                AomNode::new(4, Role::Paragraph).with_text(
                    "The reading room opens at nine in the morning and closes at eight in the \
                     evening, the children's corner closes at six, the archive is open on \
                     Tuesdays and Thursdays only, and the café stays open until the last reader \
                     leaves on the nights of the monthly lecture.",
                ),
            ])
    }

    fn setup() -> (ScreenReaderEngine, RecordingActions, RecordingUiHost) {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page())),
            Box::new(MockSpeechOutput::new()),
        );
        let actions = RecordingActions::tracking();
        engine.set_actions(Box::new(actions.clone()));
        let ui = RecordingUiHost::default();
        engine.set_ui(Box::new(ui.clone()));
        engine.cursor.move_to_start();
        (engine, actions, ui)
    }

    fn press(engine: &mut ScreenReaderEngine, key: Key) {
        engine.handle_key_event(&KeyEvent::new(key, Modifiers::none(), true));
    }

    fn tracked(actions: &RecordingActions) -> Vec<usize> {
        actions
            .requests()
            .iter()
            .filter(|(what, _)| what == "track")
            .map(|(_, node)| *node)
            .collect()
    }

    fn place(engine: &mut ScreenReaderEngine, node: usize, rect: [i32; 4]) {
        let _ = engine.handle_accessibility_event(&AccessibilityEvent::TrackedPlace { node, rect });
    }

    /// H, until the virtual cursor is on heading `id`.
    fn to_heading(engine: &mut ScreenReaderEngine, id: usize) {
        for _ in 0..4 {
            if engine.node_under_cursor().map(|n| n.id) == Some(id) {
                return;
            }
            press(engine, Key::Alpha('H'));
        }
    }

    /// A quick key moves the virtual cursor: its element is scrolled to and asked for, and
    /// framed when its place comes; a place that comes after the cursor moved on is not
    /// framed, and a key that reads without moving asks for nothing.
    #[test]
    fn the_reading_highlight_frames_what_the_virtual_cursor_read() {
        let (mut engine, actions, ui) = setup();
        press(&mut engine, Key::Alpha('H'));
        assert_eq!(engine.node_under_cursor().map(|n| n.id), Some(1));
        assert_eq!(tracked(&actions), [1]);
        place(&mut engine, 1, [40, 120, 600, 40]);
        assert_eq!(
            ui.take(),
            [ToUi::HighlightShow {
                rect: [40, 120, 600, 40]
            }]
        );

        // Down to the paragraph: asked for; the heading's late place is not framed.
        press(&mut engine, Key::DownArrow);
        assert_eq!(engine.node_under_cursor().map(|n| n.id), Some(2));
        assert_eq!(tracked(&actions), [1, 2]);
        place(&mut engine, 1, [40, 120, 600, 40]);
        assert!(ui.take().is_empty());
        place(&mut engine, 2, [40, 170, 600, 24]);
        assert_eq!(
            ui.take(),
            [ToUi::HighlightShow {
                rect: [40, 170, 600, 24]
            }]
        );

        // Insert+Up reads the line where it is.
        engine.handle_key_event(&KeyEvent::new(
            Key::UpArrow,
            Modifiers {
                insert: true,
                ..Modifiers::none()
            },
            true,
        ));
        assert_eq!(tracked(&actions), [1, 2]);
    }

    /// An element with no size (hidden, or out of reach) takes the frame away instead.
    #[test]
    fn an_element_with_no_place_takes_the_frame_away() {
        let (mut engine, _actions, ui) = setup();
        to_heading(&mut engine, 1);
        place(&mut engine, 1, [40, 120, 600, 40]);
        let _ = ui.take();
        press(&mut engine, Key::DownArrow);
        place(&mut engine, 2, [0, 0, 0, 0]);
        assert_eq!(ui.take(), [ToUi::HighlightHide]);
    }

    /// The frame goes when the focus leaves the page, and with a new page.
    #[test]
    fn the_frame_goes_with_the_page() {
        let (mut engine, _actions, ui) = setup();
        to_heading(&mut engine, 1);
        place(&mut engine, 1, [40, 120, 600, 40]);
        let _ = ui.take();
        engine.app_mode = AppMode::NativeControl;
        press(&mut engine, Key::Tab);
        assert_eq!(ui.take(), [ToUi::HighlightHide]);

        let (mut engine, _actions, ui) = setup();
        to_heading(&mut engine, 1);
        place(&mut engine, 1, [40, 120, 600, 40]);
        let _ = ui.take();
        let other = AomNode::new(0, Role::Document)
            .with_name("Events")
            .with_children(vec![
                AomNode::new(1, Role::Paragraph).with_text("No events.")
            ]);
        let _ = engine.handle_document_loaded("Events", VirtualBuffer::from_aom(&other));
        assert!(ui.take().contains(&ToUi::HighlightHide));
    }

    /// Turned off (the settings file's `visual_tracking`), nothing is asked for or drawn.
    #[test]
    fn the_reading_highlight_can_be_turned_off() {
        let (mut engine, actions, ui) = setup();
        engine.settings.visual_tracking = false;
        press(&mut engine, Key::Alpha('H'));
        press(&mut engine, Key::DownArrow);
        place(&mut engine, 2, [40, 170, 600, 24]);
        assert!(tracked(&actions).is_empty());
        assert!(ui.take().is_empty());
    }

    /// JAWS frames the line read in a long paragraph, not the whole paragraph: a Down onto
    /// the paragraph's next line asks for that line's place.
    #[test]
    #[ignore = "known gap: the reading highlight frames the element, not the line's text"]
    fn a_line_of_a_long_paragraph_is_framed_alone() {
        let (mut engine, actions, _ui) = setup();
        to_heading(&mut engine, 3);
        press(&mut engine, Key::DownArrow);
        let first = engine.cursor.offset;
        press(&mut engine, Key::DownArrow);
        assert_eq!(engine.node_under_cursor().map(|n| n.id), Some(4));
        assert_ne!(
            engine.cursor.offset, first,
            "the paragraph has a second line"
        );
        let asked = tracked(&actions);
        assert_eq!(asked.iter().filter(|&&n| n == 4).count(), 2);
    }
}
