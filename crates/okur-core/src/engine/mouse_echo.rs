//! The mouse echo layer: Insert+Space, E, then one key (JAWS 2025 help, Layered Keystrokes,
//! Mouse Echo Layer Keystrokes). O turns mouse echo on and off; C, W, L and P set what it says
//! of what the mouse points to, a character, a word, a line or a paragraph. Escape, Space, Tab
//! and Enter leave the layer, as any other key does [obs, 792/s15].

use super::ScreenReaderEngine;
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;
use crate::settings::MouseEchoUnit;

impl ScreenReaderEngine {
    /// Insert+Space, E: "Mouse echo layer", and the next key is its [obs, 791/s4].
    pub(super) fn start_mouse_echo_layer(&mut self) {
        self.mouse_echo_layer = true;
        self.speech.speak(msg::mouse_echo_layer().as_str(), true);
    }

    /// The mouse echo layer's key, when the layer is open; `None` leaves `event` to the rest
    /// of the engine. A modifier pressed on its own keeps the layer open.
    pub(super) fn mouse_echo_layer_key(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        if !self.mouse_echo_layer
            || !event.is_down
            || matches!(
                event.key,
                Key::Shift | Key::Control | Key::Alt | Key::Insert | Key::CapsLock | Key::Windows
            )
        {
            return None;
        }
        self.mouse_echo_layer = false;
        if event.modifiers.is_empty() {
            match event.key {
                Key::Alpha('O') => self.toggle_mouse_echo(),
                Key::Alpha('C') => self.set_mouse_echo_unit(MouseEchoUnit::Character),
                Key::Alpha('W') => self.set_mouse_echo_unit(MouseEchoUnit::Word),
                Key::Alpha('L') => self.set_mouse_echo_unit(MouseEchoUnit::Line),
                Key::Alpha('P') => self.set_mouse_echo_unit(MouseEchoUnit::Paragraph),
                _ => {}
            }
        }
        Some(KeyAction::Consume)
    }

    /// O: mouse echo on, "on", or off, "off", for the session [obs, 791/s5, s10].
    fn toggle_mouse_echo(&mut self) {
        let on = !self.settings.mouse_echo.enabled;
        self.settings_layers
            .set_for_session(&["mouse_echo", "enabled"], serde_json::Value::Bool(on));
        self.resolve_settings();
        let said = if on {
            msg::mouse_echo_on()
        } else {
            msg::mouse_echo_off()
        };
        self.speech.speak(said.as_str(), true);
    }

    /// C, W, L, P: the unit for the session, said as JAWS says it, "character", "word",
    /// "line", "paragraph" [obs, 792/s3, s6, s9, s12].
    fn set_mouse_echo_unit(&mut self, unit: MouseEchoUnit) {
        self.settings_layers.set_for_session(
            &["mouse_echo", "unit"],
            serde_json::to_value(unit).unwrap_or_default(),
        );
        self.resolve_settings();
        let said = match unit {
            MouseEchoUnit::Character => msg::mouse_echo_unit_character(),
            MouseEchoUnit::Word => msg::mouse_echo_unit_word(),
            MouseEchoUnit::Line => msg::mouse_echo_unit_line(),
            MouseEchoUnit::Paragraph => msg::mouse_echo_unit_paragraph(),
        };
        self.speech.speak(said.as_str(), true);
    }
}

#[cfg(test)]
mod tests {
    use crate::aom::{AomNode, Role};
    use crate::buffer::VirtualBuffer;
    use crate::cursor::VirtualCursor;
    use crate::engine::tests::SharedSpeech;
    use crate::engine::ScreenReaderEngine;
    use crate::input::{Key, KeyAction, KeyEvent, Modifiers};

    fn press(engine: &mut ScreenReaderEngine, key: Key, insert: bool) -> KeyAction {
        let mut modifiers = Modifiers::none();
        modifiers.insert = insert;
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true))
    }

    /// Insert+Space, E says "Mouse echo layer"; O then turns mouse echo on, "on", and the
    /// next time off, "off" [obs, 791/s3-s5, s8-s10].
    #[test]
    fn insert_space_e_o_turns_mouse_echo_on_and_off() {
        let page =
            AomNode::new(0, Role::Document)
                .with_children(vec![AomNode::new(1, Role::Paragraph).with_text("Text")]);
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page)),
            Box::new(speech.clone()),
        );
        assert!(!engine.settings.mouse_echo.enabled);
        for (on, said) in [(true, "on"), (false, "off")] {
            let _ = press(&mut engine, Key::Space, true);
            assert_eq!(
                press(&mut engine, Key::Alpha('E'), false),
                KeyAction::Consume
            );
            assert_eq!(speech.take(), ["Mouse echo layer"]);
            assert_eq!(
                press(&mut engine, Key::Alpha('O'), false),
                KeyAction::Consume
            );
            assert_eq!(speech.take(), [said]);
            assert_eq!(engine.settings.mouse_echo.enabled, on);
        }
        // The layer is closed: O is not its key any more.
        assert!(!engine.mouse_echo_layer);
    }

    /// C, W, P and L set the unit and say it; Escape leaves the layer silently [obs, 792].
    #[test]
    fn the_layer_sets_the_mouse_echo_unit() {
        use crate::settings::MouseEchoUnit;
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
            Box::new(speech.clone()),
        );
        for (key, unit, said) in [
            ('C', MouseEchoUnit::Character, "character"),
            ('W', MouseEchoUnit::Word, "word"),
            ('P', MouseEchoUnit::Paragraph, "paragraph"),
            ('L', MouseEchoUnit::Line, "line"),
        ] {
            let _ = press(&mut engine, Key::Space, true);
            let _ = press(&mut engine, Key::Alpha('E'), false);
            let _ = speech.take();
            let _ = press(&mut engine, Key::Alpha(key), false);
            assert_eq!(speech.take(), [said]);
            assert_eq!(engine.settings.mouse_echo.unit, unit);
        }
        let _ = press(&mut engine, Key::Space, true);
        let _ = press(&mut engine, Key::Alpha('E'), false);
        let _ = speech.take();
        assert_eq!(press(&mut engine, Key::Escape, false), KeyAction::Consume);
        assert!(speech.take().is_empty());
    }
}
