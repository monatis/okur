//! What edit keys say: the text, caret and selection the application reports after a key.

use super::ScreenReaderEngine;
use crate::input::KeyEvent;
use crate::messages::msg;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Feedback for a key passed to a text field: keys that move the caret, select or
    /// delete are spoken once the application has handled them (from the field's text
    /// before and after); typed characters are echoed, as JAWS does with its default typing
    /// echo (characters).
    pub(super) fn edit_key_feedback(&mut self, event: &KeyEvent) {
        // An application that reads the key itself (Windows search: Up and Down select
        // results, read as they are selected).
        if self.last_native_focus.as_ref().is_some_and(|field| {
            self.app_for(field)
                .is_some_and(|app| app.reads_key(field, event))
        }) {
            return;
        }
        // Up and Down in a field pointing at a list's option (an autocomplete) move to the
        // next option, which its focus event says; the field's text is not read.
        if self.option_list.is_some()
            && matches!(event.key, crate::Key::UpArrow | crate::Key::DownArrow)
        {
            return;
        }
        let own_key = self.last_native_focus.as_ref().and_then(|field| {
            self.app_for(field)
                .and_then(|app| app.edit_key(field, event))
        });
        // Tab in a table of Word's moves to the next cell and selects it [obs, 518/s15].
        let m = &event.modifiers;
        let cell_key = (event.key == crate::Key::Tab
            && !m.ctrl
            && !m.alt
            && !m.insert
            && !m.win
            && self.in_document_table())
        .then_some(crate::editing::EditKey::TableCell(
            crate::apps::windows::word::CellMove::Next,
        ));
        // Tab among a form's fields, Space on its check box [obs, 1056, 1057].
        let form_key = self
            .last_text
            .as_ref()
            .filter(|_| self.editing_document())
            .and_then(|text| crate::apps::windows::word::form_key(text, event));
        if let Some(key) = crate::editing::EditKey::of(event)
            .or(own_key)
            .or(cell_key)
            .or(form_key)
        {
            // The word being typed ends where the caret moves; Backspace takes back its last
            // character.
            if key == crate::editing::EditKey::DeleteBackward {
                self.typed_word.pop();
            } else {
                self.typed_word.clear();
            }
            // A document (Windows 11 Notepad; a page's multi-line field, Google Docs' with
            // braille support, where JAWS said "top of file" for Ctrl+Home and the character
            // for Home on the dev box, 2026-10-16) reads some keys otherwise.
            // A plain multi-line edit (Windows 10 Notepad's, a WinForms text box) says "top of
            // file" for Ctrl+Home as a document does [obs, 660/s4], but nothing for Home
            // [obs, 504/s5, 664/s9, 406/s8].
            // UI Automation gives Windows 10 Notepad's multi-line edit the Document type, as it
            // gives Windows 11 Notepad's rich edit: the window's class tells them apart.
            let plain_edit = self.app_mode != super::AppMode::VirtualBuffer
                && self.last_native_focus.as_ref().is_some_and(|f| {
                    let plain = |class: Option<&str>| {
                        class.is_some_and(|c| {
                            let c = c.to_ascii_uppercase();
                            c == "EDIT" || c.contains(".EDIT.")
                        })
                    };
                    let plain_class = plain(f.platform.element_class.as_deref())
                        || plain(f.platform.window_class.as_deref());
                    f.role == crate::aom::Role::Edit || plain_class
                });
            let key = match key.in_document(event.key, event.modifiers.ctrl) {
                _ if !self.editing_document() => key,
                crate::editing::EditKey::Character
                    if plain_edit && key == crate::editing::EditKey::Silent =>
                {
                    key
                }
                document => document,
            };
            self.expect_text_change(key);
        } else if event.key == crate::Key::Enter && event.modifiers.is_empty() {
            self.echo_enter();
        } else if let Some(c) = event.text {
            self.echo_typed(c);
            // Keep the field's known text current (no speech follows), so the next edit key
            // (Backspace) is spoken from the text as typed.
            if let Some(actions) = &self.actions {
                actions.read_at_caret(
                    self.last_text.as_ref(),
                    self.app_mode == super::AppMode::VirtualBuffer,
                );
            }
        }
    }

    /// Whether the caret is in a table of a document whose structure the platform reads
    /// (Word's).
    fn in_document_table(&self) -> bool {
        self.last_text
            .as_ref()
            .and_then(|t| t.structure.as_ref())
            .is_some_and(|s| s.cell.is_some())
    }

    /// JAWS's table keys in a document whose structure the platform reads (Word's):
    /// Ctrl+Alt+arrows move the caret to the next cell that way, Ctrl+Alt+Home and End to the
    /// first and last, each read as JAWS reads it [obs, 1034/s10-s12, 1035/s12, s13]; at the
    /// table's edge, its edge, "end of row" [obs, 1035/s3, s4, s6, s8]; Say Cell (Ctrl+Alt+Numpad
    /// 5) reads the cell [obs, 1035/s11]; outside a table, "Not in a Table" [obs, 1035/s15].
    /// `None` for other keys and fields, which go on as before.
    pub(super) fn document_table_key(
        &mut self,
        event: &KeyEvent,
    ) -> Option<crate::input::KeyAction> {
        use crate::apps::windows::word::CellMove;
        use crate::command::Command;
        let command = crate::command::command_for(event)?;
        let (rows, columns): (isize, isize) = match command {
            Command::TableCellRight => (0, 1),
            Command::TableCellLeft => (0, -1),
            Command::TableCellDown => (1, 0),
            Command::TableCellUp => (-1, 0),
            Command::TableFirstCell | Command::TableLastCell | Command::SayCurrentCell => (0, 0),
            _ => return None,
        };
        if !self.editing_text() {
            return None;
        }
        let context = self.last_text.as_ref()?;
        let structure = context.structure.as_ref()?;
        let Some(cell) = structure.cell.as_ref() else {
            self.speech.speak(msg::word_not_in_table().as_str(), true);
            return Some(crate::input::KeyAction::Consume);
        };
        let (row, column, how) = match command {
            Command::SayCurrentCell => {
                if let Some(said) = crate::apps::windows::word::cell_focus(context, false) {
                    self.speech.speak(&said, true);
                }
                return Some(crate::input::KeyAction::Consume);
            }
            Command::TableFirstCell => (Some(1), Some(1), CellMove::First),
            Command::TableLastCell => (Some(cell.rows), Some(cell.columns), CellMove::Last),
            _ => (
                cell.row
                    .checked_add_signed(rows)
                    .filter(|r| (1..=cell.rows).contains(r)),
                cell.column
                    .checked_add_signed(columns)
                    .filter(|c| (1..=cell.columns).contains(c)),
                if rows == 0 {
                    CellMove::Across
                } else {
                    CellMove::Along
                },
            ),
        };
        let (Some(row), Some(column)) = (row, column) else {
            let edge = match (rows, columns) {
                (1, _) => msg::table_column_bottom(),
                (-1, _) => msg::table_column_top(),
                (_, 1) => msg::table_row_end(),
                _ => msg::table_row_start(),
            };
            self.speech.speak(edge.as_str(), true);
            return Some(crate::input::KeyAction::Consume);
        };
        tracing::debug!(row, column, "table key; moving to the cell");
        self.pending_edit = Some(crate::editing::EditKey::TableCell(how));
        if let Some(actions) = &self.actions {
            actions.move_to_cell(self.last_text.as_ref(), row, column);
        }
        Some(crate::input::KeyAction::Consume)
    }

    /// Says a typed character, as the typing echo setting has it: the character, and with
    /// word echo the word a space ends. Space is named only when characters alone are echoed
    /// [obs, 403/s10, s17, s24].
    pub(super) fn echo_typed(&mut self, c: char) {
        use crate::settings::TypingEcho;
        let echo = self.settings.typing_echo;
        if c == ' ' {
            self.echo_word_end();
            if echo == TypingEcho::Characters {
                self.speak_echo(msg::typing_echo_space().as_str());
            }
            return;
        }
        self.typed_word.push(c);
        if echo.says_characters() {
            self.speak_echo(&SpeechFormatter::format_char(c));
        }
    }

    /// Enter in a text field ends the word typed, and is named unless the echo is off
    /// [obs, 403/s13, s20, s27, s34].
    fn echo_enter(&mut self) {
        self.echo_word_end();
        if self.settings.typing_echo != crate::settings::TypingEcho::None {
            self.speak_echo(msg::typing_echo_enter().as_str());
        }
    }

    /// The word typed so far ends: word echo says it.
    fn echo_word_end(&mut self) {
        let word = std::mem::take(&mut self.typed_word);
        if self.settings.typing_echo.says_words() && !word.is_empty() {
            self.speak_echo(&word);
        }
    }

    fn speak_echo(&mut self, echo: &str) {
        tracing::debug!(%echo, "typing echo");
        self.speech.speak_echo(echo);
    }

    /// Remembers the edit key until the field reports its new text, and asks the platform
    /// to read it (applications do not all report caret moves).
    pub(super) fn expect_text_change(&mut self, key: crate::editing::EditKey) {
        tracing::debug!(?key, "edit key; reading the field");
        self.pending_edit = Some(key);
        if let Some(actions) = &self.actions {
            actions.read_at_caret(
                self.last_text.as_ref(),
                self.app_mode == super::AppMode::VirtualBuffer,
            );
        }
    }

    /// The focused text field's text changed: speaks the pending edit key's effect.
    pub(super) fn handle_text_context(
        &mut self,
        context: &crate::event::TextContext,
    ) -> Option<String> {
        // A Virtual Viewer waiting for a console's text takes it.
        if let Some(spoken) = self.open_console_viewer(context) {
            return spoken;
        }
        // A read that caught the page's document, not the field the focus is moving to: its
        // text is the document's embedded objects alone ("￼"), right after a field took the
        // focus (280/s5, s11, s20 traces). The field's own text is kept.
        if !context.text.is_empty() && context.text.chars().all(|c| c == '\u{fffc}') {
            return None;
        }
        // A field's misspellings stay while its text does: a read that does not report them
        // keeps the last ones (the word keys' reads in a page field gave none, where the read
        // at its end gave "teh" [567 traces]).
        let mut context = context.clone();
        if context.misspelled.is_empty() {
            if let Some(last) = self.last_text.as_ref().filter(|l| l.text == context.text) {
                context.misspelled = last.misspelled.clone();
            }
        }
        // A selection's caret is its end that moved, where the platform gives a selection by
        // its ends alone (a Win32 edit's, the start for the caret): Say Character after Shift+
        // Right three times reads the character after the third, "n", and after Shift+Left
        // back past where it began, the selection's start [obs, 975/s14, s21; 974/s21].
        if let (Some((start, end)), Some(last)) = (
            context.selection,
            self.last_text.as_ref().filter(|l| l.text == context.text),
        ) {
            let (last_start, last_end) = last.selection.unwrap_or((last.caret, last.caret));
            context.caret = if last.selection == Some((start, end)) {
                // The same selection read again keeps its caret.
                last.caret
            } else if start == last_start && end != last_end {
                end
            } else if end == last_end && start != last_start {
                start
            } else if start == last.caret {
                end
            } else if end == last.caret {
                start
            } else {
                context.caret
            };
        }
        let context = &context;
        let before = self.last_text.replace(context.clone());
        // A move by a document's Heading List reads the line it reached, then names the
        // document its focus came back to: "heading level 1 Library Hours | Edit" [obs,
        // 1036/s3, s7].
        let from_list = std::mem::take(&mut self.moved_by_document_list);
        let key = match self.pending_edit.take() {
            Some(key) => key,
            None if from_list => crate::editing::EditKey::Line,
            None => return None,
        };
        // What Ctrl+C or Ctrl+X took from the field is the clipboard's now (a replay keeps it,
        // for the clipboard keys [obs, 825/s3]).
        let copied = match key {
            crate::editing::EditKey::Copy => crate::editing::selected_text(context),
            crate::editing::EditKey::Cut => before
                .as_ref()
                .map(crate::editing::selected_text)
                .unwrap_or_default(),
            _ => String::new(),
        };
        if let (false, Some(actions)) = (copied.is_empty(), self.actions.as_ref()) {
            actions.application_copied(&copied);
            self.clipboard_appended = false;
        }
        let field = self.text_field_line();
        let mut speech = crate::editing::speech_in(
            key,
            before.as_ref(),
            context,
            &field,
            self.in_plain_edit(),
            crate::editing::Wording::of(&self.settings.verbosity),
        )?;
        if from_list {
            speech = format!(
                "{speech}\n{}",
                crate::linearize::role_marker(msg::role_edit().as_str())
            );
        }
        // An edit combo's line is the item its list selected: said with its position
        // ("Constantia | 39 of 187" [obs, 541/s3]), a list's, which Advanced verbosity leaves
        // out ("Constantia" [obs, 935/s3, s4]).
        if key == crate::editing::EditKey::Line
            && self
                .settings
                .verbosity
                .is_on(crate::settings::OutputType::ListPosition)
        {
            let count = self
                .last_native_focus
                .as_ref()
                .filter(|f| f.container_role == Some(crate::aom::Role::ComboBox))
                .and_then(|f| f.item_count);
            let index = self
                .combo_items
                .iter()
                .find(|(name, _)| name == speech.trim())
                .map(|(_, index)| *index);
            if let (Some(index), Some(count)) = (index, count) {
                speech = format!("{speech}\n{}", msg::position(index, count));
            }
        }
        tracing::debug!(?key, %speech, "edit speech");
        self.edit_read_in_console = self
            .last_native_focus
            .as_ref()
            .is_some_and(|field| self.app_for(field).is_some_and(|app| app.types_into(field)));
        use crate::editing::EditKey;
        // An empty line or word is read as its line break, which the synthesizer says as
        // "blank" ([`crate::editing::speech`]).
        let character = (matches!(
            key,
            EditKey::Character | EditKey::DeleteForward | EditKey::DeleteBackward
        ) && speech.chars().count() == 1)
            || (!speech.is_empty() && speech.chars().all(crate::editing::is_line_break));
        // A word and a selection are read with every symbol named [obs, 213/s11, 313/s7].
        let symbols = matches!(
            key,
            EditKey::Word | EditKey::DeleteWordBackward | EditKey::SelectMore | EditKey::SelectAll
        );
        if character {
            self.speech.speak_character(&speech, true);
        } else if symbols {
            self.speech.speak_symbols(&speech, true);
        } else {
            self.speech.speak(&speech, true);
        }
        Some(speech)
    }

    /// The focused text field is a plain Win32 edit ([`crate::event::FocusEvent::plain_edit`]).
    pub(super) fn in_plain_edit(&self) -> bool {
        self.app_mode != super::AppMode::VirtualBuffer
            && self
                .last_native_focus
                .as_ref()
                .is_some_and(|f| f.plain_edit)
    }

    /// The focused text field's name and role, as Say Line speaks them.
    pub(super) fn text_field_line(&self) -> String {
        let name = if self.forms_mode {
            self.node_under_cursor().and_then(|n| n.name.clone())
        } else {
            self.last_native_focus.as_ref().and_then(|f| f.name.clone())
        };
        // A required page field says so after its role: "Full name Edit Required" [obs,
        // 567/s3].
        let required = self.forms_mode
            && self
                .node_under_cursor()
                .is_some_and(|n| n.attributes.get("required").is_some_and(|r| r == "true"));
        let mut line = format!(
            "{}  \u{1d} {} \u{1e} ",
            name.as_deref().unwrap_or_default().trim(),
            crate::messages::msg::role_edit()
        );
        if required {
            line.push_str(&format!(
                " \u{1d}{}\u{1e} ",
                crate::messages::msg::state_required()
            ));
        }
        line
    }

    /// A page's edit field as JAWS says it when one of its lists puts it in forms mode: its
    /// name and role, then its line, as Say Line reads it in forms mode ("Email address Edit
    /// | sam@example.com" [obs, 249/s3, s4]; a text area's role is "edit" [obs, 228/s9]).
    pub(super) fn field_forms_line(&self, node_id: usize) -> Option<String> {
        use crate::messages::msg;
        let node = self.cursor.buffer.node(node_id)?;
        let role = if node
            .attributes
            .get("multiline")
            .is_some_and(|v| v == "true")
        {
            msg::role_edit_multiline()
        } else {
            msg::role_edit()
        };
        let field = format!(
            "{}  \u{1d} {role} \u{1e} ",
            node.name.as_deref().unwrap_or_default().trim()
        );
        let line = node
            .value
            .as_deref()
            .and_then(|v| v.lines().next())
            .unwrap_or_default()
            .trim();
        Some(if line.is_empty() {
            field
        } else {
            format!("{field}\n{line}")
        })
    }
}
