//! The Insert+Space layer: Insert+Space, released, then one key (JAWS 2025 help, Layered
//! Keystrokes). H opens the Speech History; S turns speech off and on. Escape, Space, Tab and
//! Enter leave the layer, as any other key does [obs, 517].

use super::ScreenReaderEngine;
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;

impl ScreenReaderEngine {
    /// Insert+Space: the next key is the layer's. JAWS says nothing [obs, 517/s2].
    pub(super) fn start_layer(&mut self) -> Option<String> {
        self.layer_open = true;
        Some(String::new())
    }

    /// The layer's key, when the layer is open; `None` leaves `event` to the rest of the
    /// engine. A modifier pressed on its own keeps the layer open.
    pub(super) fn layer_key(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        if !self.layer_open
            || !event.is_down
            || matches!(
                event.key,
                Key::Shift | Key::Control | Key::Alt | Key::Insert | Key::CapsLock | Key::Windows
            )
        {
            return None;
        }
        self.layer_open = false;
        let plain = event.modifiers.is_empty();
        match event.key {
            Key::Alpha('H') if plain => self.open_speech_history(),
            Key::Alpha('S') if plain => {
                self.speech_muted = !self.speech_muted;
                self.speech.set_muted(self.speech_muted);
            }
            _ => {}
        }
        Some(KeyAction::Consume)
    }

    /// The Speech History in the Virtual Viewer, on the latest announcement: "Speech History"
    /// [obs, 517/s3].
    fn open_speech_history(&mut self) {
        let lines: Vec<String> = self
            .speech
            .history()
            .iter()
            .flat_map(|said| said.lines().map(str::to_owned).collect::<Vec<_>>())
            .filter(|line| !line.trim().is_empty())
            .collect();
        let last = lines.len().saturating_sub(1);
        self.viewer = Some(crate::viewer::VirtualViewer::at_line(lines, last));
        self.show_viewer_window();
        self.speech
            .speak(msg::speech_history_title().as_str(), true);
    }
}

#[cfg(test)]
mod tests {
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::engine::ScreenReaderEngine;
    use crate::input::{Key, KeyAction, KeyEvent, Modifiers};
    use crate::speech::SpeechOutput;
    use std::sync::{Arc, Mutex};

    /// Speech that keeps what it says and honors mute.
    #[derive(Clone, Default)]
    struct Speech {
        said: Arc<Mutex<Vec<String>>>,
        muted: Arc<Mutex<bool>>,
    }

    impl SpeechOutput for Speech {
        fn speak(&mut self, text: &str, _interrupt: bool) {
            if !self.muted.lock().is_ok_and(|m| *m) {
                if let Ok(mut said) = self.said.lock() {
                    said.push(text.to_owned());
                }
            }
        }
        fn stop(&mut self) {}
        fn set_muted(&mut self, muted: bool) {
            if let Ok(mut m) = self.muted.lock() {
                *m = muted;
            }
        }
    }

    fn press(engine: &mut ScreenReaderEngine, key: Key, insert: bool) -> KeyAction {
        let mut modifiers = Modifiers::none();
        modifiers.insert = insert;
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true))
    }

    #[test]
    fn insert_space_then_s_turns_speech_off_and_on() {
        let page = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("First"),
            AomNode::new(2, Role::Paragraph).with_text("Second"),
            AomNode::new(3, Role::Paragraph).with_text("Third"),
        ]);
        let speech = Speech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page)),
            Box::new(speech.clone()),
        );
        engine.cursor.move_to_start();
        let said = |speech: &Speech| speech.said.lock().map(|s| s.clone()).unwrap_or_default();
        let _ = press(&mut engine, Key::Space, true);
        assert_eq!(
            press(&mut engine, Key::Alpha('S'), false),
            KeyAction::Consume
        );
        let _ = press(&mut engine, Key::DownArrow, false);
        // The muted Down moved to "Second" and said nothing.
        assert!(!said(&speech).iter().any(|s| s.contains("Second")));
        let _ = press(&mut engine, Key::Space, true);
        let _ = press(&mut engine, Key::Alpha('S'), false);
        let _ = press(&mut engine, Key::DownArrow, false);
        assert!(said(&speech).iter().any(|s| s.contains("Third")));
        // Outside the layer, S is a quick key again.
        let _ = press(&mut engine, Key::Alpha('S'), false);
        assert!(!engine.layer_open);
    }
}
