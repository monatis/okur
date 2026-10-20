//! Jump To Line in the engine (J and Shift+J in a page): the dialog opens on the cursor's line;
//! OK closes it and puts the virtual cursor on the line asked for; Shift+J returns to where
//! the jump was made (JAWS 2025 help, General HTML Hints: Jump to Line).

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;
use crate::ui::jump::{JumpDialog, Outcome};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// J: Jump To Line opens over the page. Okur says nothing itself: the dialog is read as it
    /// takes the focus [obs, 559/s1].
    pub(super) fn open_jump(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let ui = self.ui.as_ref()?;
        let buffer = &self.cursor.buffer;
        let current = buffer
            .line_at_offset(self.cursor.offset)
            .map_or(1, |line| line.line_index + 1);
        self.dialog_count += 1;
        let dialog = JumpDialog::new(self.dialog_count, buffer.lines.len(), current);
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.jump_dialog = Some(dialog);
        None
    }

    /// Whether `message` is Jump To Line's.
    pub(super) fn is_jump_event(&self, message: &FromUi) -> bool {
        self.jump_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to Jump To Line. The jump itself says nothing: JAWS's Enter in the
    /// dialog said nothing, and Say Line then read the line [obs, 559/s4, s5].
    pub(super) fn handle_jump_event(&mut self, message: &FromUi) -> Option<String> {
        match self.jump_dialog.as_mut()?.handle(message) {
            Outcome::Nothing => {}
            Outcome::Jump(number) => {
                let dialog = self.jump_dialog.take()?;
                if let Some(ui) = &self.ui {
                    ui.send(ToUi::DialogClose { id: dialog.id });
                }
                let line = self
                    .cursor
                    .buffer
                    .lines
                    .get(number.saturating_sub(1))
                    .map(|line| (line.start_offset, line.node_id));
                if let Some((start, node)) = line {
                    self.jump_return = Some(self.cursor.offset);
                    self.cursor.selection = None;
                    self.cursor.offset = start;
                    self.cursor.current_node_id = Some(node);
                }
            }
            Outcome::Closed => {
                if let Some(dialog) = self.jump_dialog.take() {
                    if let Some(ui) = &self.ui {
                        ui.send(ToUi::DialogClose { id: dialog.id });
                    }
                }
            }
        }
        None
    }

    /// Shift+J: back to where the last jump was made, its line read; "No previous jump" before
    /// any [obs, 559/s7].
    pub(super) fn jump_back(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let speech = match self.jump_return.take() {
            Some(offset) => {
                self.cursor.selection = None;
                self.cursor.offset = offset;
                self.cursor.current_node_id = self
                    .cursor
                    .buffer
                    .line_at_offset(offset)
                    .map(|line| line.node_id);
                self.cursor.current_line()
            }
            None => msg::jump_none().to_string(),
        };
        self.speech.speak(&speech, true);
        Some(speech)
    }
}
