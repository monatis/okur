//! Skim Reading in the engine (Ctrl+Shift+Insert+Down). The dialog opens over the page. Start
//! Skim Reading keeps its choices for Ctrl+Insert+Down while Okur runs; Close leaves them
//! "without saving the last changes made" (JAWS 2025 help, Skim Reading Dialog). Escape and
//! Close give the page back silently, as JAWS's dialog does [obs, 760/s14]. Create Summary
//! shows what skim reading would read, in the viewer, each piece numbered, Enter moving to it.

use super::{AppMode, ScreenReaderEngine};
use crate::ui::skim::{Outcome, SkimDialog, SkimRule};
use crate::ui::{FromUi, ToUi};

impl ScreenReaderEngine {
    /// Opens Skim Reading over the page, or brings it to the front. Okur says nothing itself:
    /// it reads the dialog as the dialog takes the focus.
    pub(super) fn open_skim_dialog(&mut self) -> Option<String> {
        if self.app_mode != AppMode::VirtualBuffer {
            return None;
        }
        let ui = self.ui.as_ref()?;
        let dialog = match self.skim_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                SkimDialog::new(self.dialog_count, self.skim_options.clone())
            }
        };
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.skim_dialog = Some(dialog);
        None
    }

    /// Whether `message` is Skim Reading's.
    pub(super) fn is_skim_event(&self, message: &FromUi) -> bool {
        self.skim_dialog
            .as_ref()
            .is_some_and(|d| Some(d.id) == super::dialog_of(message))
    }

    /// What the user did to Skim Reading.
    pub(super) fn handle_skim_event(&mut self, message: &FromUi) -> Option<String> {
        let outcome = self.skim_dialog.as_mut()?.handle(message);
        match outcome {
            Outcome::Nothing => return None,
            Outcome::Send(messages) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
                return None;
            }
            _ => {}
        }
        if let (Some(dialog), Some(ui)) = (self.skim_dialog.take(), &self.ui) {
            if !matches!(outcome, Outcome::Closed(_)) {
                ui.send(ToUi::DialogClose { id: dialog.id });
            }
        }
        self.returning_from_dialog();
        match outcome {
            // Started from the dialog, skim reading ends in its summary, by either rule type
            // [obs, 769/s3, 771/s3]; Ctrl+Insert+Down does not open one [obs, 761/s1]. An
            // attribute rule's reading opens none: JAWS's left no Results Viewer [obs, 799].
            // Without the synthesizer's progress the whole reading is handed over at once, and
            // the summary follows it.
            Outcome::Start(options) => {
                self.skim_options = options.clone();
                let pieces =
                    (options.rule != SkimRule::Attributes).then(|| self.skim_pieces(&options));
                let read = self.start_skim_read();
                let Some(pieces) = pieces else {
                    return read;
                };
                if self.say_all.is_some() {
                    self.summary_after_skim = Some(pieces);
                    return read;
                }
                let summary = self.show_skim_summary(pieces);
                match (read, summary) {
                    (Some(read), Some(summary)) => Some(format!("{read}\n{summary}")),
                    (read, summary) => read.or(summary),
                }
            }
            Outcome::Summary(options) => self.open_skim_summary(&options),
            Outcome::Nothing | Outcome::Send(_) | Outcome::Closed(_) => None,
        }
    }

    /// Create Summary: what skim reading by `rule` would read from the virtual cursor, in the
    /// viewer "Skim Reading Results", each piece numbered: "1. City Library Expands Evening
    /// Hours - Riverside Gazette" [obs, 762/s2]. Enter on a piece moves there (JAWS 2025 help,
    /// Skim Reading Dialog).
    pub(super) fn open_skim_summary(
        &mut self,
        options: &crate::ui::skim::SkimOptions,
    ) -> Option<String> {
        let pieces = self.skim_pieces(options);
        self.show_skim_summary(pieces)
    }

    /// The summary's `pieces` in the viewer "Skim Reading Results", each numbered.
    fn show_skim_summary(&mut self, pieces: Vec<(usize, String)>) -> Option<String> {
        if pieces.is_empty() {
            return None;
        }
        let lines: Vec<String> = pieces
            .iter()
            .enumerate()
            .map(|(i, (_, text))| format!("{}. {text}", i + 1))
            .collect();
        let places = pieces
            .iter()
            .enumerate()
            .map(|(i, (offset, _))| (i, *offset))
            .collect();
        let title = crate::messages::msg::skim_results_title().to_string();
        let first = lines.first().cloned().unwrap_or_default();
        self.viewer = Some(
            crate::viewer::VirtualViewer::new(lines)
                .with_places(places)
                .with_title(title.clone()),
        );
        self.show_viewer_window();
        // The viewer's title, then its first piece as the link it is in JAWS's viewer: "Skim
        // Reading Results | 1. City Library Expands Evening Hours - Riverside Gazette Link |
        // To activate press Enter." [obs, 763/s23].
        let said = format!(
            "{title}\n{first}{}\n{}",
            crate::linearize::role_marker(crate::messages::msg::role_link().as_str()),
            crate::messages::msg::hint_activate_enter()
        );
        self.speech.speak(&said, false);
        Some(said)
    }

    /// The end of a skim started from the dialog: its summary [obs, 769/s3, 771/s3].
    pub(super) fn skim_ended(&mut self) {
        if let Some(pieces) = self.summary_after_skim.take() {
            let _ = self.show_skim_summary(pieces);
        }
    }

    /// Enter on a summary's piece: the virtual cursor moves there and reads its line.
    pub(super) fn move_to_summary_place(&mut self, offset: usize) {
        let said = self.cursor.jump_to_element_offset(offset);
        self.speech.speak(&said, true);
    }
}
