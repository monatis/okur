//! The Insert+Space layer: Insert+Space, released, then one key (JAWS 2025 help, Layered
//! Keystrokes). H opens the Speech History, Shift+H clears it and Ctrl+H copies it; S turns
//! speech off and on, and Shift+S (Speech On Demand) is not available in full speech; Shift+N
//! says the last notification again (RepeatLastNotification). Escape,
//! Space, Tab and Enter leave the layer, as any other key does [obs, 517, 566, 570].

use super::{AppMode, ScreenReaderEngine};
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
        let shift_only = event.modifiers.shift
            && !event.modifiers.ctrl
            && !event.modifiers.alt
            && !event.modifiers.insert;
        let ctrl_only = event.modifiers.ctrl
            && !event.modifiers.shift
            && !event.modifiers.alt
            && !event.modifiers.insert;
        match event.key {
            Key::Alpha('H') if plain => self.open_speech_history(),
            // Shift+H forgets the Speech History, silently [obs, 570/s6].
            Key::Alpha('H') if shift_only => self.speech.clear_history(),
            Key::Alpha('H') if ctrl_only => self.copy_speech_history(),
            // Speech On Demand and muting (the speech_mode module).
            Key::Alpha('S') if shift_only => self.speech_on_demand_toggle(),
            Key::Alpha('S') if plain => self.mute_synthesizer(),
            Key::Alpha('T') if plain => self.start_table_layer(),
            Key::Alpha('N') if shift_only => self.repeat_last_notification(),
            // Flexible Web's wizard (DoFlexibleWebDialog) [obs, 705/s3].
            Key::Alpha('X') if plain => {
                let _ = self.open_flexible_web();
            }
            _ => {}
        }
        Some(KeyAction::Consume)
    }

    /// Insert+Space, T in a page's table: "Table layer", and the arrows move by cell until
    /// Escape [obs, 535/s5].
    fn start_table_layer(&mut self) {
        let in_table = self.app_mode == AppMode::VirtualBuffer
            && self
                .cursor
                .read_current_cell()
                .is_some_and(|said| said != msg::table_not_in_table().as_str());
        let speech = if in_table {
            self.table_layer = true;
            msg::table_layer()
        } else {
            msg::table_not_in_table()
        };
        self.speech.speak(speech.as_str(), true);
    }

    /// A key while the table layer is on: the arrows move by cell, Numpad5 says the cell,
    /// Ctrl+Home and Ctrl+End go to the first and last cell, Ctrl and an arrow to the row's or
    /// column's end, Shift+Up reads the row and Shift+Numpad5 the column; Escape leaves the
    /// layer silently [obs, 535/s6-s16]. Another key leaves it and acts as usual.
    pub(super) fn table_layer_key(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        use crate::command::Command;
        if !self.table_layer
            || !event.is_down
            || matches!(
                event.key,
                Key::Shift | Key::Control | Key::Alt | Key::Insert | Key::CapsLock | Key::Windows
            )
        {
            return None;
        }
        let m = event.modifiers;
        let plain = m.is_empty();
        let ctrl = m.ctrl && !m.alt && !m.shift && !m.insert && !m.win;
        let shift = m.shift && !m.ctrl && !m.alt && !m.insert && !m.win;
        let command = match event.key {
            Key::Escape if plain => {
                self.table_layer = false;
                return Some(KeyAction::Consume);
            }
            Key::RightArrow if plain => Command::TableCellRight,
            Key::LeftArrow if plain => Command::TableCellLeft,
            Key::DownArrow if plain => Command::TableCellDown,
            Key::UpArrow if plain => Command::TableCellUp,
            Key::Numpad5 if plain => Command::SayCurrentCell,
            Key::Home if ctrl => Command::TableFirstCell,
            Key::End if ctrl => Command::TableLastCell,
            Key::DownArrow if ctrl => Command::MoveToBottomOfColumn,
            Key::UpArrow if ctrl => Command::MoveToTopOfColumn,
            Key::RightArrow if ctrl => Command::MoveToEndOfRow,
            Key::LeftArrow if ctrl => Command::MoveToStartOfRow,
            Key::UpArrow if shift => Command::SayCurrentRow,
            Key::Numpad5 if shift => Command::SayCurrentColumn,
            _ => {
                self.table_layer = false;
                return None;
            }
        };
        let _ = self.execute(command);
        Some(KeyAction::Consume)
    }

    /// The Speech History's lines, the oldest first.
    fn speech_history_lines(&self) -> Vec<String> {
        self.speech
            .history()
            .iter()
            .flat_map(|said| said.lines().map(str::to_owned).collect::<Vec<_>>())
            .filter(|line| !line.trim().is_empty())
            .collect()
    }

    /// Ctrl+H: the Speech History on the clipboard, a line each, then "Copy speech history to
    /// clipboard" [obs, 570/s3].
    fn copy_speech_history(&mut self) {
        let text = self.speech_history_lines().join("\r\n");
        if let Some(actions) = &self.actions {
            actions.copy_text(&text);
        }
        self.speech
            .speak(msg::speech_history_copied().as_str(), true);
    }

    /// The Speech History in the Virtual Viewer, on the latest announcement: "Speech History"
    /// [obs, 517/s3].
    fn open_speech_history(&mut self) {
        let lines = self.speech_history_lines();
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

    /// Speech that keeps what it speaks, and speaks nothing it is told to keep unspoken.
    #[derive(Clone, Default)]
    struct Speech {
        said: Arc<Mutex<Vec<String>>>,
        unspoken: Arc<Mutex<bool>>,
    }

    impl SpeechOutput for Speech {
        fn speak(&mut self, text: &str, _interrupt: bool) {
            if !self.unspoken.lock().is_ok_and(|m| *m) {
                if let Ok(mut said) = self.said.lock() {
                    said.push(text.to_owned());
                }
            }
        }
        fn stop(&mut self) {}
        fn history(&self) -> Vec<String> {
            self.said.lock().map(|s| s.clone()).unwrap_or_default()
        }
        fn clear_history(&mut self) {
            if let Ok(mut said) = self.said.lock() {
                said.clear();
            }
        }
        fn set_unspoken(&mut self, unspoken: bool) {
            if let Ok(mut m) = self.unspoken.lock() {
                *m = unspoken;
            }
        }
    }

    fn press(engine: &mut ScreenReaderEngine, key: Key, insert: bool) -> KeyAction {
        let mut modifiers = Modifiers::none();
        modifiers.insert = insert;
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true))
    }

    /// Insert+Space, S goes to Speech On Demand, saying so, where Down says nothing; again, to
    /// full speech [obs, 706/s3-s8].
    #[test]
    fn insert_space_then_s_goes_to_speech_on_demand_and_back() {
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
        assert!(said(&speech).iter().any(|s| s == "Speech On Demand"));
        let _ = press(&mut engine, Key::DownArrow, false);
        // Down moved to "Second" and said nothing.
        assert!(!said(&speech).iter().any(|s| s.contains("Second")));
        let _ = press(&mut engine, Key::Space, true);
        let _ = press(&mut engine, Key::Alpha('S'), false);
        assert!(said(&speech).iter().any(|s| s == "Full Speech"));
        let _ = press(&mut engine, Key::DownArrow, false);
        assert!(said(&speech).iter().any(|s| s.contains("Third")));
        // Outside the layer, S is a quick key again.
        let _ = press(&mut engine, Key::Alpha('S'), false);
        assert!(!engine.layer_open);
    }

    /// Ctrl+H puts the Speech History on the clipboard and says so; Shift+H clears it,
    /// silently [obs, 570/s3, s6].
    #[test]
    fn insert_space_then_ctrl_h_copies_the_history_and_shift_h_clears_it() {
        let page = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("First"),
            AomNode::new(2, Role::Paragraph).with_text("Second"),
        ]);
        let speech = Speech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page)),
            Box::new(speech.clone()),
        );
        let actions = crate::actions::RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));
        engine.cursor.move_to_start();
        let _ = press(&mut engine, Key::DownArrow, false);
        let ctrl_h = |engine: &mut ScreenReaderEngine, modifiers: Modifiers| {
            let _ = press(engine, Key::Space, true);
            engine.handle_key_event(&KeyEvent::new(Key::Alpha('H'), modifiers, true))
        };
        let ctrl = {
            let mut m = Modifiers::none();
            m.ctrl = true;
            m
        };
        assert_eq!(ctrl_h(&mut engine, ctrl), KeyAction::Consume);
        assert_eq!(
            crate::actions::ApplicationActions::clipboard_text(&actions).as_deref(),
            Some("Second")
        );
        assert_eq!(
            speech.history().last().map(String::as_str),
            Some("Copy speech history to clipboard")
        );
        let shift = {
            let mut m = Modifiers::none();
            m.shift = true;
            m
        };
        assert_eq!(ctrl_h(&mut engine, shift), KeyAction::Consume);
        assert!(speech.history().is_empty());
    }
}
