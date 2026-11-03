//! Microsoft Word's own: its document as an application module, and JAWS's keys for Word
//! (revisions, a form's keys, the browse object, the views, Word's formatting keys). What JAWS
//! says of a document's text and structure is [`crate::document`]'s.

use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::document::{form_field_at, points};
use crate::editing::EditKey;
use crate::event::{FocusEvent, FormFieldKind, TextContext};
use crate::messages::msg;
use crate::ui::lists::ListKind;

/// Word's document, where JAWS's lists of the document's links and fields have Word's own
/// words.
#[derive(Debug, Default)]
pub struct WordDocument;

impl AppModule for WordDocument {
    fn owns(&self, focus: &FocusEvent) -> bool {
        focus.role == Role::Document
            && !focus.in_ribbon
            && focus
                .app
                .as_deref()
                .is_some_and(|app| app.eq_ignore_ascii_case(super::word_ribbon::WORD))
    }

    /// Back from Office's own context menu (Shift+F10), the document says "Edit" alone, nothing
    /// of leaving menus [obs, 1125/s4].
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        // Only Word's document: Outlook's or Excel's focus after Office's context menu is
        // theirs (performance review 2026-11-02, engine finding 3).
        if !cx.office_menu_closed || !self.owns(focus) {
            return FocusResponse::Pass;
        }
        cx.note_window(focus);
        FocusResponse::SpeakWhole(crate::announcement::Announcement::new(vec![
            crate::announcement::Line::new(vec![crate::announcement::Part::role(
                msg::role_edit().as_str(),
            )]),
        ]))
    }

    /// The Links List and List Fields with nothing in them: "no links | Check for link type
    /// fields with Insert+F5." and "no fields" [obs, 1044/s1, 1047/s1].
    fn empty_list(&self, _focus: &FocusEvent, kind: &ListKind) -> Option<String> {
        match kind {
            ListKind::Links => Some(format!(
                "{}\n{}",
                msg::links_list_none(),
                msg::word_links_list_hint()
            )),
            ListKind::FormFields => Some(msg::word_fields_list_none().to_string()),
            _ => None,
        }
    }
}

/// Word's Ctrl+PageDown and Ctrl+PageUp, to the next or previous page: "Next page", "Previous
/// page" before the page and its line [obs, 1113/s1-s4]. Word's browse object can be another
/// than the page (Alt+Ctrl+Home), which JAWS's words would then name [hyp: not recorded].
pub(crate) fn browse_page(event: &crate::input::KeyEvent) -> Option<bool> {
    let m = &event.modifiers;
    if !m.ctrl || m.shift || m.alt || m.insert || m.win {
        return None;
    }
    match event.key {
        crate::input::Key::PageDown => Some(true),
        crate::input::Key::PageUp => Some(false),
        _ => None,
    }
}

/// JAWS's keys for Word's revisions, from its key map for Word: Insert+Shift+R lists them
/// (`listRevisions`), Ctrl+Insert+R says the one at the caret (`AnnounceRevision`), the
/// regions list's key elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevisionKey {
    List,
    Announce,
}

impl RevisionKey {
    /// Insert+Shift+R or Ctrl+Insert+R, without other modifiers.
    pub(crate) fn of(event: &crate::input::KeyEvent) -> Option<Self> {
        let m = &event.modifiers;
        if event.key != crate::input::Key::Alpha('R') || !m.insert || m.alt || m.win {
            return None;
        }
        match (m.ctrl, m.shift) {
            (false, true) => Some(Self::List),
            (true, false) => Some(Self::Announce),
            _ => None,
        }
    }

    /// What it says while Word does not track changes, which both need: "Track Changes is
    /// disabled. Press the Word command, CONTROL+SHIFT+E, to enable the feature.", "You must
    /// enable Track Changes in order to use this feature." [obs, 1109/s1, s3].
    pub(crate) fn untracked(self) -> String {
        match self {
            Self::List => msg::word_revisions_list_untracked(),
            Self::Announce => msg::word_revisions_untracked(),
        }
        .to_string()
    }
}

/// Tab, Shift+Tab or Space read as moving among a document's form fields or changing one: Tab
/// with form fields on the caret's paragraph, Space on a check box [obs, 1056/s2-s8; 1057/s2].
pub(crate) fn form_key(context: &TextContext, event: &crate::input::KeyEvent) -> Option<EditKey> {
    let m = &event.modifiers;
    if m.ctrl || m.alt || m.insert || m.win {
        return None;
    }
    match event.key {
        crate::input::Key::Tab
            if context
                .structure
                .as_ref()
                .is_some_and(|s| !s.form_fields.is_empty()) =>
        {
            Some(EditKey::FormField)
        }
        crate::input::Key::Space
            if !m.shift
                && form_field_at(context).is_some_and(|f| f.kind == FormFieldKind::CheckBox) =>
        {
            Some(EditKey::FormFieldState)
        }
        // Shift+F3 changes the selection's case: the selection is said as it becomes [obs,
        // 1078/s3-s5].
        crate::input::Key::F(3) if m.shift && context.selection.is_some() => {
            Some(EditKey::ChangeCase)
        }
        _ => None,
    }
}

/// The view Word shows its document in, as Say Window Title says it after the title, from the
/// name of the status bar's view button that is on: "Print Layout" is "Print View" [obs,
/// 519/s1, 1039/s7], "Web Layout" "Web Layout View" and "Read Mode" "reading layout View" [obs,
/// 1042/s5, s9].
/// The name of Word's status bar button for Read Mode.
pub(crate) const READ_MODE: &str = "Read Mode";

pub(crate) fn view_name(button: &str) -> Option<String> {
    let name = match button {
        "Print Layout" => msg::word_view_print(),
        "Web Layout" => msg::word_view_web(),
        "Read Mode" => msg::word_view_read(),
        _ => return None,
    };
    Some(name.to_string())
}

/// Word's view keys: Ctrl+Alt+N for Draft, Ctrl+Alt+O for Outline, Ctrl+Alt+P for Print Layout,
/// each said by its view's name, "Draft View", "Outline View", "Print View" [obs, 1039/s2, s4,
/// s6]. Draft and Outline have no button on Word's status bar, so the name the key gave is Say
/// Window Title's too [obs, 1039/s3, s5].
pub(crate) fn view_key(event: &crate::input::KeyEvent) -> Option<String> {
    let m = &event.modifiers;
    if !(m.ctrl && m.alt) || m.shift || m.insert || m.win {
        return None;
    }
    Some(
        match event.key {
            crate::input::Key::Alpha('N') => msg::word_view_draft(),
            crate::input::Key::Alpha('O') => msg::word_view_outline(),
            crate::input::Key::Alpha('P') => msg::word_view_print(),
            _ => return None,
        }
        .to_string(),
    )
}

/// Word's keys after which JAWS says what they left, read once Word has acted on them (Ctrl+E
/// on a centered paragraph aligns it left): Ctrl+Shift+E, Track Changes [obs, 1068/s1, s8];
/// Ctrl+E, R, J and L, the alignment [obs, 1071/s2-s5]; Ctrl+Shift+Period and Comma, the font
/// size [obs, 1074/s3-s5]; Ctrl+M and Ctrl+Shift+M, the paragraph's indent [obs, 1080/s2-s4]. The line spacing keys and Ctrl+B, I, U say nothing [obs, 1071/s6-s8,
/// 1070].
pub(crate) fn word_key(event: &crate::input::KeyEvent) -> Option<crate::event::WordKey> {
    use crate::event::WordKey;
    use crate::input::Key;
    let m = &event.modifiers;
    if !m.ctrl || m.alt || m.insert || m.win {
        return None;
    }
    match (m.shift, event.key) {
        (true, Key::Alpha('E')) => Some(WordKey::TrackChanges),
        (false, Key::Alpha('E' | 'R' | 'J' | 'L')) => Some(WordKey::Alignment),
        (true, Key::Period | Key::Comma) => Some(WordKey::FontSize),
        (out, Key::Alpha('M')) => Some(WordKey::Indent { out }),
        _ => None,
    }
}

/// What one of Word's keys left, as JAWS says it: "Track Changes on", "Centered", "aligned
/// right", "justified", "Aligned left", "16 point". Word's other justifications (4, 5, 7, 8)
/// are said as justified, its Thai one (9) not at all.
pub(crate) fn word_key_said(state: crate::event::WordKeyState) -> Option<String> {
    use crate::event::WordKeyState;
    let said = match state {
        WordKeyState::TrackChanges(true) => msg::word_track_changes_on(),
        WordKeyState::TrackChanges(false) => msg::word_track_changes_off(),
        WordKeyState::Alignment(0) => msg::word_aligned_left(),
        WordKeyState::Alignment(1) => msg::word_aligned_centered(),
        WordKeyState::Alignment(2) => msg::word_aligned_right(),
        WordKeyState::Alignment(3 | 4 | 5 | 7 | 8) => msg::word_aligned_justified(),
        WordKeyState::Alignment(_) => return None,
        WordKeyState::FontSize(tenths) => return Some(msg::font_size(points(tenths)).render()),
        WordKeyState::Indent { out, hundredths } => {
            let way = if out {
                msg::word_outdent()
            } else {
                msg::word_indent()
            };
            return Some(format!("{way}\n{}", inches(hundredths)));
        }
    };
    Some(said.to_string())
}

/// A distance in hundredths of an inch as JAWS says an indent: "1 inch", "1.5 inches".
fn inches(hundredths: u32) -> String {
    let whole = hundredths / 100;
    let part = hundredths % 100;
    let number = match (part, part % 10) {
        (0, _) => whole.to_string(),
        (_, 0) => format!("{whole}.{}", part / 10),
        _ => format!("{whole}.{part:02}"),
    };
    if hundredths == 100 {
        msg::word_one_inch().to_string()
    } else {
        msg::word_inches(number).render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{DocumentFormField, LineStructure};

    /// In Word's document, the Links List and List Fields with nothing in them say
    /// Word's words [obs, 1044/s1, 1047/s1]; its ribbon and other applications are not this
    /// module's.
    #[test]
    fn words_empty_lists_say_words_words() {
        let mut document = FocusEvent::new(Role::Document);
        document.app = Some("WINWORD.EXE".into());
        assert!(WordDocument.owns(&document));
        assert_eq!(
            WordDocument
                .empty_list(&document, &ListKind::Links)
                .as_deref(),
            Some("no links\nCheck for link type fields with Insert+F5.")
        );
        assert_eq!(
            WordDocument
                .empty_list(&document, &ListKind::FormFields)
                .as_deref(),
            Some("no fields")
        );
        assert_eq!(
            WordDocument.empty_list(&document, &ListKind::Headings),
            None
        );
        let mut ribbon = document.clone();
        ribbon.in_ribbon = true;
        assert!(!WordDocument.owns(&ribbon));
        let mut wordpad = document;
        wordpad.app = Some("wordpad.exe".into());
        assert!(!WordDocument.owns(&wordpad));
    }

    /// Back from Office's context menu, Word's document says "Edit"; another Office
    /// application's focus is not Word's to say.
    #[test]
    fn only_words_document_says_edit_after_offices_menu() {
        let said = |focus: &FocusEvent| {
            let (mut mode, mut title, mut kind) =
                (crate::engine::AppMode::NativeControl, None, None);
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: false,
                escape_key: true,
                key: None,
                ask_view: false,
                ask_sheet: false,
                repeat: false,
                office_menu_closed: true,
                sheet_titles: None,
            };
            match WordDocument.focus(focus, &mut cx) {
                FocusResponse::SpeakWhole(announcement) => Some(announcement.render()),
                _ => None,
            }
        };
        let mut document = FocusEvent::new(Role::Document);
        document.app = Some("WINWORD.EXE".into());
        assert_eq!(said(&document).as_deref(), Some(" \u{1d} Edit \u{1e} "));
        let mut message = FocusEvent::new(Role::ListItem).with_name("Weekly report");
        message.app = Some("OUTLOOK.EXE".into());
        assert_eq!(said(&message), None);
    }

    /// The form fixture's text, the caret at `caret` on the line `line`, with the form fields
    /// of that line.
    fn form(caret: usize, line: (usize, usize), fields: Vec<DocumentFormField>) -> TextContext {
        TextContext {
            text: "Card Request\rFill in the form, then save it.\rName:      \rMember: \rNewsletter: \rBranch: \rThank you."
                .to_owned(),
            caret,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: Some(LineStructure {
                line,
                form_fields: fields,
                ..LineStructure::default()
            }),
        }
    }

    fn field(kind: FormFieldKind, name: &str, start: usize, end: usize) -> DocumentFormField {
        DocumentFormField {
            kind,
            name: name.to_owned(),
            start,
            end,
            value: String::new(),
            checked: None,
            choice: None,
        }
    }

    /// Shift+F3 on a selection says it in its new case, as Word leaves it [obs, 1078/s3-s5].
    #[test]
    fn shift_f3_says_the_selection_in_its_new_case() {
        use crate::input::{Key, KeyEvent, Modifiers};
        let mut shift_f3 = KeyEvent::new(Key::F(3), Modifiers::none(), true);
        shift_f3.modifiers.shift = true;
        let mut selected = form(13, (0, 13), Vec::new());
        selected.text = "LIBRARY HOURS\rThe library opens.".into();
        selected.selection = Some((0, 13));
        assert_eq!(form_key(&selected, &shift_f3), Some(EditKey::ChangeCase));
        assert_eq!(
            crate::editing::speech(EditKey::ChangeCase, None, &selected, "").as_deref(),
            Some("LIBRARY HOURS")
        );
        // Without a selection Word changes the word at the caret; nothing is known to say.
        assert_eq!(form_key(&form(1, (0, 13), Vec::new()), &shift_f3), None);
    }

    /// Ctrl+M and Ctrl+Shift+M say the way and the paragraph's left indent in inches, as
    /// JAWS's "indent | 1 inch", "out dent | 1.5 inches" [obs, 1080/s2-s4].
    #[test]
    fn the_indent_keys_say_the_indent_in_inches() {
        use crate::event::WordKeyState;
        let said = |out, hundredths| word_key_said(WordKeyState::Indent { out, hundredths });
        assert_eq!(said(false, 100).as_deref(), Some("indent\n1 inch"));
        assert_eq!(said(false, 150).as_deref(), Some("indent\n1.5 inches"));
        assert_eq!(said(true, 50).as_deref(), Some("out dent\n0.5 inches"));
        assert_eq!(said(false, 125).as_deref(), Some("indent\n1.25 inches"));
        assert_eq!(said(true, 0).as_deref(), Some("out dent\n0 inches"));
    }

    /// Tab with form fields on the caret's paragraph moves among them, Space on a check box
    /// changes it; elsewhere neither is a form's key [obs, 1056, 1057].
    #[test]
    fn tab_and_space_are_a_forms_keys() {
        use crate::input::{Key, KeyEvent, Modifiers};
        let tab = KeyEvent::new(Key::Tab, Modifiers::none(), true);
        let space = KeyEvent::new(Key::Space, Modifiers::none(), true);
        let mut member = field(FormFieldKind::CheckBox, "Member", 65, 65);
        member.checked = Some(false);
        let at_member = form(65, (57, 66), vec![member]);
        assert_eq!(form_key(&at_member, &tab), Some(EditKey::FormField));
        assert_eq!(form_key(&at_member, &space), Some(EditKey::FormFieldState));
        let plain = form(1, (0, 13), Vec::new());
        assert_eq!(form_key(&plain, &tab), None);
        assert_eq!(form_key(&plain, &space), None);
        // A key that moves the caret onto a field says the field; one beside it, its own.
        let mut name = field(FormFieldKind::Text, "Name", 51, 56);
        name.value = "     ".into();
        assert_eq!(
            crate::editing::speech(
                EditKey::Word,
                None,
                &form(51, (45, 57), vec![name.clone()]),
                ""
            )
            .as_deref(),
            Some("Name:  \u{1d} Edit \u{1e} ")
        );
        assert_eq!(
            crate::editing::speech(
                EditKey::Character,
                None,
                &form(45, (45, 57), vec![name]),
                ""
            )
            .as_deref(),
            Some("N")
        );
    }

    /// Word's view keys say their views; others say nothing [obs, 1039/s2, s4, s6].
    #[test]
    fn the_view_keys_say_their_views() {
        use crate::input::{Key, KeyEvent, Modifiers};
        let key = |letter, alt| {
            KeyEvent::new(
                Key::Alpha(letter),
                Modifiers {
                    ctrl: true,
                    alt,
                    ..Modifiers::none()
                },
                true,
            )
        };
        assert_eq!(view_key(&key('N', true)).as_deref(), Some("Draft View"));
        assert_eq!(view_key(&key('O', true)).as_deref(), Some("Outline View"));
        assert_eq!(view_key(&key('P', true)).as_deref(), Some("Print View"));
        // The status bar's buttons [obs, 1042/s5, s9].
        assert_eq!(view_name("Web Layout").as_deref(), Some("Web Layout View"));
        assert_eq!(
            view_name("Read Mode").as_deref(),
            Some("reading layout View")
        );
        assert_eq!(view_name("Focus"), None);
        assert_eq!(view_key(&key('P', false)), None);
        assert_eq!(view_key(&key('Q', true)), None);
    }
}
