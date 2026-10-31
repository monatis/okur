//! Microsoft Word's own dialogs (its `bosa_sdm_msword` windows: Font, Paragraph...), read as
//! JAWS reads them [obs, 1058]. Word reports a field's focus twice: from the field's RichEdit
//! window, nameless, and from the dialog, the field named by its label. JAWS says the second:
//! - an edit field is an edit combo, its label on a line of its own, then its value and the
//!   combo's hint: "Font: | edit combo | Calibri | To set the value use the Arrow keys or type
//!   the value." [obs, 1058/s2, s3, s5]; a new dialog is named first, "Font dialog";
//! - a page's tab is selected, with Word's page key: "Advanced Tab selected | To switch pages,
//!   press Control+Tab." [obs, 1058/s6, s7];
//! - Escape back to the document says "Edit" alone, not the window's title [obs, 1058/s8]; from
//!   a modeless dialog (Find and Replace), the window's title before it, as Ctrl+Tab to the
//!   document does: "sr-word-fixture.docx - Word | Edit" [obs, 1065/s6, s8].
//!
//! Its buttons, check boxes and lists are the engine's (they read as JAWS reads them:
//! "Font Color (Automatic) Button | To activate press spacebar." [obs, 1058/s4]).
//!
//! Word's task panes (the Spelling pane, NetUI windows outside the ribbon) read their controls
//! as the engine does; Escape back from one says the document's name and "Edit":
//! "sr-word-fixture.docx | Edit" [obs, 1069/s6].

use crate::announcement::{Announcement, Line, Part, PartKind};
use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;
use crate::messages::msg;

/// Word's dialog window class.
const DIALOG_CLASS: &str = "bosa_sdm_msword";
/// The window class of a dialog's edit field.
const FIELD_CLASS: &str = "RichEdit20W";
/// The name UI Automation gives a RichEdit field without one of its own.
const UNNAMED_FIELD: &str = "RichEdit Control";

/// Word's dialogs.
#[derive(Debug, Default)]
pub struct WordDialog {
    /// The last field said, by its name and value: Word reports it again after its RichEdit.
    said: Option<(String, Option<String>)>,
    /// A dialog of Word's had the focus: the document taking it back is said alone.
    in_dialog: bool,
    /// That dialog is modeless: the document taking the focus back names its window.
    modeless: bool,
    /// One of Word's task panes had the focus: the document taking it back says its name.
    in_pane: bool,
}

/// Whether the focus is in Word.
fn in_word(focus: &FocusEvent) -> bool {
    focus
        .app
        .as_deref()
        .is_some_and(|app| app.eq_ignore_ascii_case(super::word_ribbon::WORD))
}

/// Whether the focus is in one of Word's task panes: a NetUI window outside its ribbon.
fn in_task_pane(focus: &FocusEvent) -> bool {
    in_word(focus)
        && !focus.in_ribbon
        && focus.platform.window_class.as_deref() == Some(super::word_ribbon::NETUI_CLASS)
}

/// Whether the focus is in one of Word's dialogs.
fn in_dialog(focus: &FocusEvent) -> bool {
    let class = |c: &Option<String>| c.as_deref() == Some(DIALOG_CLASS);
    in_word(focus)
        && (class(&focus.platform.window_class) || class(&focus.platform.top_window_class))
}

/// The class of Windows' own dialog boxes, Save As's among them.
const COMMON_DIALOG_CLASS: &str = "#32770";

/// Whether the focus is in a dialog box of Windows' own that Word opened (Save As, Open): its
/// controls read as anywhere, and Escape back to the document says "Edit", as from Word's own
/// dialogs.
fn in_common_dialog(focus: &FocusEvent) -> bool {
    in_word(focus) && focus.platform.top_window_class.as_deref() == Some(COMMON_DIALOG_CLASS)
}

/// Whether the focus is back in Word from one of its dialogs: in Word, or on a document from a
/// window already gone, which names no application (the first focus as Save As closes, from
/// its own window, with Word's document in it [obs, 1077/s4]).
fn back_in_word(focus: &FocusEvent) -> bool {
    in_word(focus) || (focus.app.is_none() && focus.role == Role::Document)
}

impl AppModule for WordDialog {
    fn owns(&self, focus: &FocusEvent) -> bool {
        in_dialog(focus)
            || in_task_pane(focus)
            || in_common_dialog(focus)
            || ((self.in_dialog || self.in_pane) && back_in_word(focus))
    }

    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        if in_task_pane(focus) {
            self.in_pane = true;
            return FocusResponse::Pass;
        }
        if in_common_dialog(focus) && !in_dialog(focus) {
            self.in_dialog = true;
            self.modeless = focus.modeless;
            self.said = None;
            return FocusResponse::Pass;
        }
        // Back from a task pane: Word's window on the way says nothing, its document its name
        // and "Edit".
        if self.in_pane && !in_dialog(focus) {
            return match focus.role {
                Role::Window => FocusResponse::Silent,
                Role::Document => {
                    self.in_pane = false;
                    let mut lines: Vec<Line> = focus
                        .name
                        .as_deref()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(|name| Line::of(PartKind::Name, name))
                        .into_iter()
                        .collect();
                    lines.push(Line::new(vec![Part::role(msg::role_edit().as_str())]));
                    FocusResponse::Speak(Announcement::new(lines))
                }
                _ => {
                    self.in_pane = false;
                    FocusResponse::Pass
                }
            };
        }
        if !in_dialog(focus) {
            if !(self.in_dialog && back_in_word(focus)) {
                return FocusResponse::Pass;
            }
            // Back from the dialog: Word's window on the way says nothing, its document "Edit".
            return match focus.role {
                Role::Window => {
                    cx.note_window(focus);
                    FocusResponse::Silent
                }
                Role::Document => {
                    self.in_dialog = false;
                    self.said = None;
                    cx.note_window(focus);
                    let mut lines: Vec<Line> = focus
                        .window_title
                        .as_deref()
                        .filter(|_| std::mem::take(&mut self.modeless))
                        .map(Line::text)
                        .into_iter()
                        .collect();
                    lines.push(Line::new(vec![Part::role(msg::role_edit().as_str())]));
                    FocusResponse::Speak(Announcement::new(lines))
                }
                _ => {
                    self.in_dialog = false;
                    FocusResponse::Pass
                }
            };
        }
        self.in_dialog = true;
        self.modeless = focus.modeless;
        let name = focus.name.as_deref().map(str::trim).unwrap_or_default();
        // The field's own window, nameless: the dialog names it next.
        if focus.platform.window_class.as_deref() == Some(FIELD_CLASS)
            || name.is_empty()
            || name == UNNAMED_FIELD
        {
            return FocusResponse::Silent;
        }
        let entering = *cx.window_title != focus.window_title;
        let mut lines = Vec::new();
        if entering {
            cx.note_window(focus);
            self.said = None;
            if let Some(title) = focus.window_title.as_deref() {
                lines.push(Line::new(vec![
                    Part::new(PartKind::Name, title),
                    Part::role(msg::role_dialog().as_str()),
                ]));
            }
        }
        match focus.role {
            Role::Edit => {
                let value = focus
                    .value
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned);
                let this = (name.to_owned(), value.clone());
                if !entering && self.said.as_ref() == Some(&this) {
                    return FocusResponse::Silent;
                }
                self.said = Some(this);
                lines.push(Line::of(PartKind::Name, name));
                lines.push(Line::new(vec![Part::role(msg::role_edit_combo().as_str())]));
                lines.extend(value.map(|v| Line::of(PartKind::Value, v)));
                lines.push(Line::of(
                    PartKind::Hint,
                    msg::hint_set_value_arrows_or_type(),
                ));
            }
            Role::Tab => {
                // Word reports a page's tab twice too.
                let this = (name.to_owned(), None);
                if !entering && self.said.as_ref() == Some(&this) {
                    return FocusResponse::Silent;
                }
                self.said = Some(this);
                lines.push(Line::new(vec![
                    Part::new(PartKind::Name, name),
                    Part::role(msg::role_tab().as_str()),
                    Part::state(msg::state_selected().as_str()),
                ]));
                lines.push(Line::of(PartKind::Hint, msg::hint_switch_pages_ctrl_tab()));
            }
            _ => {
                self.said = None;
                // The engine names a dialog it enters with the control.
                return FocusResponse::Pass;
            }
        }
        FocusResponse::Speak(Announcement::new(lines))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::AppMode;

    fn said(
        dialog: &mut WordDialog,
        focus: &FocusEvent,
        title: &mut Option<String>,
    ) -> Option<String> {
        let mut mode = AppMode::NativeControl;
        let mut kind = None;
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: false,
            escape_key: false,
            key: None,
            ask_view: false,
            ask_sheet: false,
        };
        match dialog.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) => Some(announcement.render()),
            FocusResponse::Silent => Some(String::new()),
            _ => None,
        }
    }

    fn field(class: &str, role: Role, name: &str, value: Option<&str>) -> FocusEvent {
        let mut focus = FocusEvent::new(role).with_name(name.to_owned());
        focus.app = Some("WINWORD.EXE".into());
        focus.window_title = Some("Font".into());
        focus.platform.window_class = Some(class.into());
        focus.platform.top_window_class = Some(DIALOG_CLASS.into());
        focus.value = value.map(str::to_owned);
        focus
    }

    /// Word's Font dialog as JAWS reads it: named as it opens, a field once though Word
    /// reports it twice, as an edit combo; its pages' tabs with Word's page key; Escape back to
    /// the document "Edit" [obs, 1058].
    #[test]
    fn words_font_dialog_is_read_as_jaws_reads_it() {
        let mut dialog = WordDialog::default();
        let mut title = Some("sr-word-fixture.docx - Word".to_owned());
        let font = field(DIALOG_CLASS, Role::Edit, "Font:", Some("Calibri"));
        assert_eq!(
            said(&mut dialog, &font, &mut title).as_deref(),
            Some("Font \u{1d} dialog \u{1e} \nFont:\n \u{1d} edit combo \u{1e} \nCalibri\nTo set the value use the Arrow keys or type the value.")
        );
        let rich = field(FIELD_CLASS, Role::Edit, "", Some("Calibri"));
        assert_eq!(said(&mut dialog, &rich, &mut title).as_deref(), Some(""));
        assert_eq!(said(&mut dialog, &font, &mut title).as_deref(), Some(""));
        let style = field(DIALOG_CLASS, Role::Edit, "Font style:", Some("Regular"));
        assert_eq!(
            said(&mut dialog, &style, &mut title).as_deref(),
            Some("Font style:\n \u{1d} edit combo \u{1e} \nRegular\nTo set the value use the Arrow keys or type the value.")
        );
        let advanced = field(DIALOG_CLASS, Role::Tab, "Advanced", None);
        assert_eq!(
            said(&mut dialog, &advanced, &mut title).as_deref(),
            Some("Advanced \u{1d} Tab \u{1e}  \u{1d}selected\u{1e} \nTo switch pages, press Control+Tab.")
        );
        let button = field(DIALOG_CLASS, Role::Button, "Font Color (Automatic)", None);
        assert_eq!(said(&mut dialog, &button, &mut title), None);
        let mut window = FocusEvent::new(Role::Window);
        window.app = Some("WINWORD.EXE".into());
        assert_eq!(said(&mut dialog, &window, &mut title).as_deref(), Some(""));
        let mut document = FocusEvent::new(Role::Document);
        document.app = Some("WINWORD.EXE".into());
        assert_eq!(
            said(&mut dialog, &document, &mut title).as_deref(),
            Some(" \u{1d} Edit \u{1e} ")
        );
        assert_eq!(said(&mut dialog, &document, &mut title), None);
    }

    /// Word's task pane's controls are the engine's; Escape back from it says the document's
    /// name and "Edit" [obs, 1069/s2, s6].
    #[test]
    fn the_document_after_a_task_pane_says_its_name() {
        let mut dialog = WordDialog::default();
        let mut title = Some("sr-word-fixture.docx - Word".to_owned());
        let mut ignore = FocusEvent::new(Role::Button).with_name("Ignore All");
        ignore.app = Some("WINWORD.EXE".into());
        ignore.platform.window_class = Some("NetUIHWND".into());
        assert_eq!(said(&mut dialog, &ignore, &mut title), None);
        let mut window = FocusEvent::new(Role::Window);
        window.app = Some("WINWORD.EXE".into());
        assert_eq!(said(&mut dialog, &window, &mut title).as_deref(), Some(""));
        let mut document = FocusEvent::new(Role::Document).with_name("sr-word-fixture.docx");
        document.app = Some("WINWORD.EXE".into());
        assert_eq!(
            said(&mut dialog, &document, &mut title).as_deref(),
            Some("sr-word-fixture.docx\n \u{1d} Edit \u{1e} ")
        );
        assert_eq!(said(&mut dialog, &document, &mut title), None);
    }

    /// Back in the document from a modeless dialog (Find and Replace, by Ctrl+Tab or Escape),
    /// the window's title is said before "Edit" [obs, 1065/s6, s8].
    #[test]
    fn the_document_after_a_modeless_dialog_names_its_window() {
        let mut dialog = WordDialog::default();
        let mut title = Some("sr-word-fixture.docx - Word".to_owned());
        let mut find = field(DIALOG_CLASS, Role::Edit, "Find what:", None);
        find.modeless = true;
        assert!(said(&mut dialog, &find, &mut title).is_some());
        let mut document = FocusEvent::new(Role::Document);
        document.app = Some("WINWORD.EXE".into());
        document.window_title = Some("sr-word-fixture.docx - Word".into());
        assert_eq!(
            said(&mut dialog, &document, &mut title).as_deref(),
            Some("sr-word-fixture.docx - Word\n \u{1d} Edit \u{1e} ")
        );
    }

    /// A dialog box of Windows' own that Word opened (Save As) reads as anywhere; back in the
    /// document, the first focus can come from the closed dialog's window, naming no
    /// application, and says "Edit" as from Word's own dialogs [obs, 1077/s4].
    #[test]
    fn the_document_after_save_as_says_edit() {
        let mut dialog = WordDialog::default();
        let mut title = Some("Save As".to_owned());
        let mut name = FocusEvent::new(Role::Edit).with_name("File name:");
        name.app = Some("WINWORD.EXE".into());
        name.platform.top_window_class = Some(COMMON_DIALOG_CLASS.into());
        assert_eq!(said(&mut dialog, &name, &mut title), None);
        let document = FocusEvent::new(Role::Document).with_name("sr-word-fixture.docx");
        assert_eq!(
            said(&mut dialog, &document, &mut title).as_deref(),
            Some(" \u{1d} Edit \u{1e} ")
        );
        assert!(!dialog.owns(&document));
    }
}
