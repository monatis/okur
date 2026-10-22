//! The mouse echo layer: Insert+Space, E, then one key (JAWS 2025 help, Layered Keystrokes,
//! Mouse Echo Layer Keystrokes). O turns mouse echo on and off; C, W, L and P set what it says
//! of what the mouse points to, a character, a word, a line or a paragraph. Escape, Space, Tab
//! and Enter leave the layer, as any other key does [obs, 792/s15].
//!
//! Mouse echo itself says what the mouse points to once it rests there for the Mouse Echo
//! Delay, or as it moves with Mouse Echo Instant: the text of its unit there, then the
//! control's type and state, as JAWS's help describes it (Settings Center, Mouse Echo Group).
//! JAWS's echo said nothing to the mouse the harness moves (SendInput, in Notepad and in
//! Chrome, by 6 to 360 pixels: each move only stopped speech; old-pc, 2026-10-22), so this
//! part follows the help, not recordings.

use super::ScreenReaderEngine;
use crate::event::MousePoint;
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
        self.last_mouse_echo = None;
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

    /// The mouse moved to `x`, `y`: while mouse echo is on, what it points to is read at
    /// once with Mouse Echo Instant, else once it rests there ([`Self::mouse_echo_idle`]).
    pub(super) fn mouse_moved(&mut self, x: i32, y: i32) {
        self.mouse = Some((x, y));
        // Okur's own moves (routing, the Okur cursor, Alt+Shift and the arrows) are not
        // echoed, as JAWS's were not (791/s6, s7).
        if self
            .own_mouse_move_until
            .is_some_and(|until| std::time::Instant::now() < until)
        {
            self.mouse_moved_at = None;
            return;
        }
        self.mouse_moved_at = Some(std::time::Instant::now());
        let echo = self.settings.mouse_echo;
        if echo.enabled && echo.instant {
            self.mouse_moved_at = None;
            if let Some(actions) = &self.actions {
                actions.read_at_point(x, y);
            }
        }
    }

    /// Okur moved the mouse itself: the moves the platform reports meanwhile are not echoed.
    pub(super) fn note_own_mouse_move(&mut self) {
        const OWN_MOVE: std::time::Duration = std::time::Duration::from_millis(400);
        self.own_mouse_move_until = Some(std::time::Instant::now() + OWN_MOVE);
        self.mouse_moved_at = None;
    }

    /// While mouse echo is on, what the mouse points to is read once it has rested for the
    /// Mouse Echo Delay ("If you move the mouse pointer away before the delay has elapsed,
    /// JAWS does not read the text").
    pub(super) fn mouse_echo_idle(&mut self) {
        let echo = self.settings.mouse_echo;
        let rested = self.mouse_moved_at.is_some_and(|moved| {
            moved.elapsed() >= std::time::Duration::from_millis(u64::from(echo.delay_ms))
        });
        if !echo.enabled || !rested {
            return;
        }
        self.mouse_moved_at = None;
        if let (Some((x, y)), Some(actions)) = (self.mouse, &self.actions) {
            actions.read_at_point(x, y);
        }
    }

    /// What the mouse points to, read where it is: said while mouse echo is on. The text of
    /// the unit there, or, where the control gives no text by unit, all it gives, its name
    /// ("If the control cannot provide the requested unit of text, then JAWS will read all of
    /// the text provided by the control"); then its type and state and its description, as
    /// the settings choose. What was just said is not said again while the mouse stays in it.
    pub(super) fn mouse_point(&mut self, point: &MousePoint) -> Option<String> {
        let echo = self.settings.mouse_echo;
        if !echo.enabled || self.mouse != Some((point.x, point.y)) {
            return None;
        }
        let said = mouse_echo_speech(point, &echo);
        if said.is_empty() || self.last_mouse_echo.as_deref() == Some(said.as_str()) {
            return None;
        }
        self.last_mouse_echo = Some(said.clone());
        // A character alone is said as the reading keys say it ("space").
        if said.chars().count() == 1 {
            self.speech.speak_character(&said, echo.interrupt);
        } else {
            self.speech.speak(&said, echo.interrupt);
        }
        Some(said)
    }

    /// C, W, L, P: the unit for the session, said as JAWS says it, "character", "word",
    /// "line", "paragraph" [obs, 792/s3, s6, s9, s12]. Choosing a unit turns mouse echo on: with echo
    /// off, W and then O said "off" [obs, 797/s3-s6].
    fn set_mouse_echo_unit(&mut self, unit: MouseEchoUnit) {
        self.settings_layers.set_for_session(
            &["mouse_echo", "unit"],
            serde_json::to_value(unit).unwrap_or_default(),
        );
        self.settings_layers
            .set_for_session(&["mouse_echo", "enabled"], serde_json::Value::Bool(true));
        self.last_mouse_echo = None;
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

/// What mouse echo says of `point`: its unit's text or the control's name, then the
/// control's type and state and its description, as `echo` chooses.
fn mouse_echo_speech(point: &MousePoint, echo: &crate::settings::MouseEcho) -> String {
    use crate::aom::Role;
    let mut parts: Vec<String> = Vec::new();
    let unit_text = point.texts.of(echo.unit);
    let control = point.control.as_ref();
    if let Some(text) = unit_text.or_else(|| {
        control
            .and_then(|c| c.name.clone())
            .filter(|n| !n.trim().is_empty())
    }) {
        parts.push(text);
    }
    if let Some(control) = control {
        if echo.control_type_and_state {
            if let Some(word) = crate::native_focus::role_word(&control.role) {
                parts.push(word.to_owned());
            }
            if matches!(control.role, Role::CheckBox | Role::RadioButton) {
                parts.push(
                    if control.states.checked == Some(true) {
                        msg::state_checked()
                    } else {
                        msg::state_not_checked()
                    }
                    .to_string(),
                );
            }
        }
        if echo.description {
            if let Some(help) = control
                .help_text
                .as_deref()
                .filter(|h| !h.trim().is_empty())
            {
                parts.push(help.to_owned());
            }
        }
    }
    parts.join(" ")
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
            // A unit turns echo on [obs, 797/s6].
            assert!(engine.settings.mouse_echo.enabled);
        }
        let _ = press(&mut engine, Key::Space, true);
        let _ = press(&mut engine, Key::Alpha('E'), false);
        let _ = speech.take();
        assert_eq!(press(&mut engine, Key::Escape, false), KeyAction::Consume);
        assert!(speech.take().is_empty());
    }

    fn notepad_point(x: i32, line: &str) -> crate::event::MousePoint {
        let mut control = crate::event::FocusEvent::new(Role::Edit);
        control.name = Some("Text Editor".into());
        crate::event::MousePoint {
            x,
            y: 87,
            control: Some(control),
            texts: crate::event::UnitTexts {
                character: line.chars().next().map(String::from),
                word: line.split(' ').next().map(String::from),
                line: Some(format!("{line}\r\n")),
                paragraph: Some(format!("{line}\r\n")),
                column: None,
            },
        }
    }

    /// While mouse echo is on, what the mouse rests on is said: the line there, then the
    /// control's type, as JAWS's help describes it; the same line is not said again as the
    /// mouse moves along it, and nothing is said while echo is off.
    #[test]
    fn mouse_echo_says_the_line_the_mouse_rests_on() {
        use crate::event::AccessibilityEvent;
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
            Box::new(speech.clone()),
        );
        let point = |engine: &mut ScreenReaderEngine, x: i32, line: &str| {
            let _ = engine.handle_accessibility_event(&AccessibilityEvent::MouseMoved { x, y: 87 });
            engine.handle_accessibility_event(&AccessibilityEvent::MousePoint(Box::new(
                notepad_point(x, line),
            )))
        };
        // Off: nothing.
        assert_eq!(point(&mut engine, 11, "fn main() {"), None);
        let _ = press(&mut engine, Key::Space, true);
        let _ = press(&mut engine, Key::Alpha('E'), false);
        let _ = press(&mut engine, Key::Alpha('O'), false);
        let _ = speech.take();
        assert_eq!(
            point(&mut engine, 16, "fn main() {").as_deref(),
            Some("fn main() { Edit")
        );
        // Along the same line: not again.
        assert_eq!(point(&mut engine, 30, "fn main() {"), None);
        // A point read where the mouse no longer is: not said.
        let _ = engine.handle_accessibility_event(&AccessibilityEvent::MouseMoved { x: 40, y: 87 });
        let stale = engine.handle_accessibility_event(&AccessibilityEvent::MousePoint(Box::new(
            notepad_point(30, "    println!(\"{total}\");"),
        )));
        assert_eq!(stale, None);
        // The word unit: the word there.
        let _ = press(&mut engine, Key::Space, true);
        let _ = press(&mut engine, Key::Alpha('E'), false);
        let _ = press(&mut engine, Key::Alpha('W'), false);
        let _ = speech.take();
        assert_eq!(
            point(&mut engine, 50, "main() {").as_deref(),
            Some("main() Edit")
        );
    }

    /// The mouse Okur moves itself (routing, the Okur cursor) is not echoed, as JAWS did not
    /// echo its own moves (791/s6, s7); the user's next move is.
    #[test]
    fn okurs_own_mouse_moves_are_not_echoed() {
        use crate::event::AccessibilityEvent;
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
            Box::new(SharedSpeech::default()),
        );
        engine.note_own_mouse_move();
        let _ = engine.handle_accessibility_event(&AccessibilityEvent::MouseMoved { x: 5, y: 5 });
        assert!(engine.mouse_moved_at.is_none());
        engine.own_mouse_move_until = None;
        let _ = engine.handle_accessibility_event(&AccessibilityEvent::MouseMoved { x: 9, y: 5 });
        assert!(engine.mouse_moved_at.is_some());
    }

    /// With the point's column known, the word and the character are the line's there, as
    /// Okur splits words; a line or paragraph loses its indentation and line break.
    #[test]
    fn the_word_and_character_are_found_in_the_line() {
        use crate::settings::MouseEchoUnit;
        let texts = crate::event::UnitTexts {
            line: Some("    let total = price * 1.5; // tax\r\n".into()),
            word: Some("{\r\n    ".into()),
            character: Some(" ".into()),
            paragraph: None,
            column: Some(9),
        };
        assert_eq!(texts.of(MouseEchoUnit::Word).as_deref(), Some("total"));
        assert_eq!(texts.of(MouseEchoUnit::Character).as_deref(), Some("o"));
        assert_eq!(
            texts.of(MouseEchoUnit::Line).as_deref(),
            Some("let total = price * 1.5; // tax")
        );
        let in_indentation = crate::event::UnitTexts {
            column: Some(1),
            ..texts.clone()
        };
        // No word on a space; the space itself as a character.
        assert_eq!(in_indentation.of(MouseEchoUnit::Word), None);
        assert_eq!(
            in_indentation.of(MouseEchoUnit::Character).as_deref(),
            Some(" ")
        );
    }

    /// A control that gives no text by unit is said by its name, then its type and state.
    #[test]
    fn mouse_echo_says_a_control_by_its_name() {
        let mut check = crate::event::FocusEvent::new(Role::CheckBox);
        check.name = Some("Word wrap".into());
        check.states.checked = Some(true);
        let point = crate::event::MousePoint {
            x: 1,
            y: 1,
            control: Some(check),
            texts: crate::event::UnitTexts::default(),
        };
        let echo = crate::settings::MouseEcho::default();
        assert_eq!(
            super::mouse_echo_speech(&point, &echo),
            "Word wrap check box checked"
        );
        let quiet = crate::settings::MouseEcho {
            control_type_and_state: false,
            ..echo
        };
        assert_eq!(super::mouse_echo_speech(&point, &quiet), "Word wrap");
    }
}
