//! Microsoft Word's documents, read as JAWS reads them. The platform gives the caret's line
//! structure from Word's text (`LineStructure`: Word's own line, the paragraph's heading level
//! and list, the table cell); this module words what JAWS says of it: a heading's level before
//! its line, a list entered or left after it, and a table's cells, entered, changed and left
//! before it [obs, 518].

use crate::aom::Role;
use crate::apps::AppModule;
use crate::editing::EditKey;
use crate::event::{
    DocumentFormField, DocumentParagraph, FocusEvent, FormFieldKind, LineSpacing, LineStructure,
    TableCell, TextContext,
};
use crate::linearize::{role_marker, state_marker};
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

/// `line` spoken as JAWS speaks a line of Word's: a heading with its level before it,
/// " heading level 1 Library Hours" [obs, 518/s1, s9]; a line with form fields says them in
/// it ([`line_with_fields`]).
pub(crate) fn spoken_line(context: &TextContext, line: String) -> String {
    let line = line_with_fields(context)
        .map(|l| l.trim().to_owned())
        .unwrap_or(line);
    with_heading(context.structure.as_ref().and_then(|s| s.heading), line)
}

/// The form field of Word's at the caret: a text field holding it (at its end too), a check box
/// or a drop-down where it stands [obs, 1055/s4, s7; 1056/s2-s4].
pub(crate) fn form_field_at(context: &TextContext) -> Option<&DocumentFormField> {
    context
        .structure
        .as_ref()?
        .form_fields
        .iter()
        .find(|f| (f.start..=f.end).contains(&context.caret))
}

/// A form field said as a control, its line's text before it its name: "Name: Edit", with what
/// it holds, "ab"; "Member: check box not checked"; "Branch: Combo box | Central | 1 of 3"
/// [obs, 1056/s1-s4; 1057/s14].
pub(crate) fn form_field_speech(
    context: &TextContext,
    field: &DocumentFormField,
    moved_to: bool,
) -> String {
    let label = form_field_label(context, field);
    let named = |role: &str| {
        if label.is_empty() {
            role_marker(role)
        } else {
            format!("{label} {}", role_marker(role))
        }
    };
    match field.kind {
        // Its text selected, as Tab back to it selects it: "Name: Edit | selected | ab" [obs,
        // 1061/s4], by the key that moved there (`moved_to`) and not by Say Line [obs, 1061/s5];
        // an empty one's spaces selected say nothing of it [obs, 1056/s6].
        FormFieldKind::Text => {
            let mut said = named(msg::role_edit().as_str());
            let value = field.value.trim();
            if !value.is_empty() {
                let selected = context.selection.is_some_and(|(start, end)| {
                    start < end && start <= field.start && end > field.start
                });
                if selected && moved_to {
                    said.push('\n');
                    said.push_str(msg::selection_added().as_str());
                }
                said.push('\n');
                said.push_str(value);
            }
            said
        }
        FormFieldKind::CheckBox => format!(
            "{}{}",
            named(msg::role_check_box().as_str()),
            check_state(field)
        ),
        FormFieldKind::DropDown => {
            let mut lines = vec![named(msg::role_combo_box_focus().as_str())];
            lines.extend(
                Some(field.value.trim())
                    .filter(|v| !v.is_empty())
                    .map(str::to_owned),
            );
            lines.extend(
                field
                    .choice
                    .map(|(index, count)| msg::position(index, count).to_string()),
            );
            lines.join("\n")
        }
    }
}

/// A check box's state, as JAWS marks it: " checked", " not checked".
fn check_state(field: &DocumentFormField) -> String {
    state_marker(
        if field.checked == Some(true) {
            msg::state_checked()
        } else {
            msg::state_not_checked()
        }
        .as_str(),
    )
}

/// Space on a check box form field: its new state alone, "checked" [obs, 1057/s2, s5].
pub(crate) fn form_field_state(context: &TextContext) -> Option<String> {
    form_field_at(context)
        .filter(|f| f.kind == FormFieldKind::CheckBox)
        .map(check_state)
}

/// A form field's name: its line's text before it, from the field before it on the line.
fn form_field_label(context: &TextContext, field: &DocumentFormField) -> String {
    let Some(structure) = context.structure.as_ref() else {
        return String::new();
    };
    let from = structure
        .form_fields
        .iter()
        .filter(|f| f.end <= field.start && f.start < field.start)
        .map(|f| f.end)
        .chain(std::iter::once(structure.line.0))
        .max()
        .unwrap_or(0)
        .min(field.start);
    context
        .text
        .chars()
        .skip(from)
        .take(field.start - from)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The caret's line with its form fields said in it and its links marked, where it has any: a
/// text field by what it holds or, empty, by its name, "Name: Name"; a check box as one,
/// "Member: check box not checked" [obs, 1055/s2, s8]; a drop-down by its role and its choice
/// [hyp]; a link after JAWS's mark, "See the  link opening hours" [obs, 1055/s1, s11].
fn line_with_fields(context: &TextContext) -> Option<String> {
    let structure = context.structure.as_ref()?;
    let (start, end) = structure.line;
    let mut fields: Vec<&DocumentFormField> = structure
        .form_fields
        .iter()
        .filter(|f| f.start >= start && f.start <= end)
        .collect();
    let links: Vec<usize> = structure
        .links
        .iter()
        .map(|(at, _)| *at)
        .filter(|at| (start..end).contains(at))
        .collect();
    // A comment's text begins with its mark, and ends apart from what follows it: "opens at
    // has comment nine in the morning ." [obs, 1081/s1].
    let comments: Vec<(usize, usize)> = structure
        .comments
        .iter()
        .map(|c| (c.start, c.end))
        .filter(|(from, to)| *from < end && *to > start)
        .collect();
    // A footnote or endnote reference is said where it stands, by its mark: "every day
    // footnote 1.", "three weeks end note i." [obs, 1083/s4, 1090/s7].
    let notes: Vec<(
        &crate::event::DocumentFootnote,
        crate::messages::StaticMessage,
    )> = structure
        .footnotes
        .iter()
        .map(|f| (f, msg::word_footnote()))
        .chain(structure.endnotes.iter().map(|f| (f, msg::word_endnote())))
        .filter(|(f, _)| (start..=end).contains(&f.at))
        .collect();
    if fields.is_empty() && links.is_empty() && comments.is_empty() && notes.is_empty() {
        return None;
    }
    let footnote_at = |at: usize| {
        notes
            .iter()
            .filter(|(f, _)| f.at == at)
            .map(|(f, kind)| format!(" \u{1d}{kind} \u{1e} {}", f.shown()))
            .collect::<String>()
    };
    fields.sort_by_key(|f| f.start);
    let text: Vec<char> = context.text.chars().collect();
    // A stretch of the line's text, each link in it after its mark.
    let piece = |from: usize, to: usize| -> String {
        let mut said = String::new();
        for at in from..to.max(from) {
            if comments.iter().any(|(_, to)| *to == at && at > start) {
                said.push(' ');
            }
            if comments.iter().any(|(from, _)| *from == at) {
                said.push_str(&format!(" \u{1d}{} \u{1e} ", msg::word_has_comment()));
            }
            if links.contains(&at) {
                said.push_str(&format!(" \u{1d}{} \u{1e} ", msg::role_link_in_line()));
            }
            said.push_str(&footnote_at(at));
            if let Some(&c) = text.get(at) {
                said.push(if c.is_control() { ' ' } else { c });
            }
        }
        // A footnote at the line's very end.
        if to == end {
            said.push_str(&footnote_at(end));
        }
        said
    };
    let mut line = String::new();
    let mut at = start;
    for field in fields {
        line.push_str(&piece(at, field.start));
        match field.kind {
            FormFieldKind::Text => match field.value.trim() {
                "" => line.push_str(&field.name),
                value => line.push_str(value),
            },
            FormFieldKind::CheckBox => {
                line.push_str(&role_marker(msg::role_check_box().as_str()));
                line.push_str(&check_state(field));
            }
            FormFieldKind::DropDown => {
                line.push_str(&role_marker(msg::role_combo_box_focus().as_str()));
                line.push_str(field.value.trim());
            }
        }
        at = at.max(field.end);
    }
    line.push_str(&piece(at, end));
    Some(line)
}

/// The comment the caret is in, as JAWS's Alt+Shift+Apostrophe reads it: its text, then its
/// author, "Check the summer hours. | by Lab"; elsewhere "No comment mark at cursor." [obs,
/// 1081/s3, s8]. JAWS says "vispero:comment" before the text, a name of its own read out (an
/// accepted divergence). `None` outside a document Word's object model reads.
pub(crate) fn comment_at_caret(context: &TextContext) -> Option<String> {
    let structure = context.structure.as_ref()?;
    let caret = context.caret;
    Some(
        match structure
            .comments
            .iter()
            .find(|c| (c.start..=c.end).contains(&caret) && c.end > c.start)
        {
            Some(comment) => {
                let mut said = comment.text.clone();
                if !comment.author.is_empty() {
                    said.push('\n');
                    said.push_str(&msg::word_comment_by(comment.author.as_str()).render());
                }
                said
            }
            None => msg::word_no_comment().to_string(),
        },
    )
}

/// The footnote or endnote whose reference Word's caret is on, as JAWS's Alt+Shift+E reads it:
/// "end note reference: i Renewals are free.", "footnote reference: 1 Bring a photo ID.";
/// elsewhere "No footnote or EndNote reference at cursor." [obs, 1092/s1, s7, s9, s14]. `None`
/// outside a document Word's object model reads.
pub(crate) fn note_at_caret(context: &TextContext) -> Option<String> {
    let structure = context.structure.as_ref()?;
    let note = |notes: &[crate::event::DocumentFootnote]| {
        notes
            .iter()
            .find(|n| n.at_caret)
            .map(|n| format!("{} {}", n.shown(), n.text.trim()))
    };
    Some(if let Some(said) = note(&structure.footnotes) {
        msg::word_footnote_reference(said.as_str()).render()
    } else if let Some(said) = note(&structure.endnotes) {
        msg::word_endnote_reference(said.as_str()).render()
    } else {
        msg::word_no_note_at_cursor().to_string()
    })
}

/// The reference Word's caret is on, as a character key says it: " end note i" [obs, 1092/s6,
/// s13].
pub(crate) fn reference_at_caret(context: &TextContext) -> Option<String> {
    let structure = context.structure.as_ref()?;
    structure
        .footnotes
        .iter()
        .map(|n| (n, msg::word_footnote()))
        .chain(structure.endnotes.iter().map(|n| (n, msg::word_endnote())))
        .find(|(n, _)| n.at_caret)
        .map(|(n, kind)| format!(" \u{1d}{kind} \u{1e} {}", n.shown()))
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

/// `line` with its heading level before it, when it is a heading's.
fn with_heading(heading: Option<u8>, line: String) -> String {
    match heading {
        Some(level) if !line.trim().is_empty() => format!(
            "\u{1d} {} \u{1e} {line}",
            msg::role_heading_level(u32::from(level)).render()
        ),
        _ => line,
    }
}

/// What Say All reads of Word's document from the caret's paragraph to the end, paragraph by
/// paragraph (a table's cell is one), each with where it starts: the caret's paragraph as it is,
/// then each with its heading level and, before it, what changed on reaching it
/// ([`say_all_change`]): "What changes | Entering List | Weekday hours are longer. | ... | exit
/// list | heading level 2 Branches | table 1 | uniform Table | Branch | Hours | Row 2 | Central |
/// 9 to 9 | out of table | The page ends here." [obs, 1038/s4, s12]. Paragraphs without text are
/// left out.
pub(crate) fn say_all_pieces(
    context: &TextContext,
    paragraphs: &[DocumentParagraph],
) -> Vec<(usize, String)> {
    let text: Vec<char> = context.text.chars().collect();
    let first = paragraphs
        .iter()
        .rposition(|p| p.start <= context.caret)
        .unwrap_or(0);
    let mut pieces = Vec::new();
    let mut was: Option<LineStructure> = None;
    for (index, paragraph) in paragraphs.iter().enumerate().skip(first) {
        let end = paragraphs
            .get(index + 1)
            .map_or(text.len(), |next| next.start)
            .min(text.len());
        let start = paragraph.start.min(end);
        // Word's paragraph and cell marks are spaces to the reader.
        let line: String = text
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .map(|&c| if c.is_control() { ' ' } else { c })
            .collect::<String>()
            .trim()
            .to_owned();
        if line.is_empty() {
            continue;
        }
        let now = LineStructure {
            line: (start, end),
            heading: paragraph.heading,
            list_item: paragraph.list_item,
            cell: paragraph.cell.clone(),
            form_fields: Vec::new(),
            links: Vec::new(),
            comments: Vec::new(),
            footnotes: Vec::new(),
            endnotes: Vec::new(),
        };
        let said = match was.as_ref() {
            None => line,
            Some(was) => say_all_change(was, &now)
                .into_iter()
                .chain(std::iter::once(with_heading(now.heading, line)))
                .collect::<Vec<_>>()
                .join("\n"),
        };
        pieces.push((start, said));
        was = Some(now);
    }
    pieces
}

/// Say Font in Word's document: bold, italic and underline first, in JAWS's words ("bolded",
/// "italicized", "Single  underline" [obs, 1040/s3, s5, s7]), then the size, the colors, the
/// font, the paragraph's style, its alignment and its outline level, "16 point | black on white |
/// Calibri | Heading 1 style | Paragraph formatting: Aligned left | Outline level: 1" [obs,
/// 1034/s1, s3], with the line spacing after the style where Word's object model gave it
/// ([`line_spacing`]). `None` outside a document whose structure the platform reads.
pub(crate) fn font(context: &TextContext, spacing: Option<&LineSpacing>) -> Option<String> {
    let structure = context.structure.as_ref()?;
    let font = context.font.as_ref()?;
    let mut said = Vec::new();
    for (on, word) in [
        (font.bold, msg::word_bolded()),
        (font.italic, msg::word_italicized()),
        (font.underline, msg::word_underline_single()),
    ] {
        if on {
            said.push(word.to_string());
        }
    }
    if font.size_tenths > 0 {
        said.push(msg::font_size(points(font.size_tenths)).render());
    }
    let name = |color: Option<crate::color::Rgb>| {
        color
            .and_then(crate::color::name_of)
            .map_or_else(|| msg::color_default().to_string(), str::to_owned)
    };
    said.push(msg::color_on(name(font.color), name(font.background)).render());
    said.push(font.family.clone());
    if let Some(style) = &font.style {
        said.push(msg::word_style(style.as_str()).render());
    }
    said.extend(spacing.map(line_spacing));
    if let Some(alignment) = &font.alignment {
        said.push(msg::word_paragraph_aligned(alignment.as_str()).render());
    }
    said.push(match structure.heading {
        Some(level) => msg::word_outline_level(u32::from(level)).render(),
        None => msg::word_outline_body().to_string(),
    });
    Some(said.join("\n"))
}

/// A paragraph's line spacing as Say Font says it, by Word's rule: "Single", "one point five
/// lines.", "Double", "At Least 18 points", "Exactly 20 points", "3 lines." [obs, 1052]. A
/// multiple is its true number of lines, Word's 12 points a line, "1.08 lines.", where JAWS
/// divides by the font's size and drops the fraction ("1 lines.", "2 lines." for 2.5, "0
/// lines." at 16 points [obs, 1052/s3, s11; 1034/s1]): an accepted divergence.
fn line_spacing(spacing: &LineSpacing) -> String {
    // Hundredths, without the zeros after the point: "18", "1.08", "2.5".
    let number = |hundredths: u32| {
        let whole = hundredths / 100;
        match hundredths % 100 {
            0 => whole.to_string(),
            part if part % 10 == 0 => format!("{whole}.{}", part / 10),
            part => format!("{whole}.{part:02}"),
        }
    };
    let points = spacing.centipoints;
    let words = match spacing.rule {
        0 => msg::word_line_spacing_single().to_string(),
        1 => msg::word_line_spacing_one_and_a_half().to_string(),
        2 => msg::word_line_spacing_double().to_string(),
        3 => msg::word_line_spacing_at_least(number(points)).render(),
        4 => msg::word_line_spacing_exactly(number(points)).render(),
        // Lines of 12 points, to the hundredth.
        _ => msg::word_line_spacing_lines(number((points + 6) / 12)).render(),
    };
    msg::word_line_spacing(words).render()
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

/// A size in tenths of a point as Word shows it: "16", "10.5".
fn points(tenths: u32) -> String {
    if tenths.is_multiple_of(10) {
        (tenths / 10).to_string()
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

/// How the caret came to a cell of Word's table, which decides what JAWS says of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellMove {
    /// Across a row: Ctrl+Alt+Left or Right.
    Across,
    /// To the next cell: Tab, across the row or on to the next row's first.
    Next,
    /// Along a column: Ctrl+Alt+Up or Down.
    Along,
    /// To the table's first cell: Ctrl+Alt+Home.
    First,
    /// To its last: Ctrl+Alt+End.
    Last,
}

/// The cell of Word's table that holds the caret of `context`.
fn cell_of(context: &TextContext) -> Option<&TableCell> {
    context.structure.as_ref()?.cell.as_ref()
}

/// A cell's titles: its column's header unless it is in the first row, and its row's first
/// cell unless it is in the first column, where it is its own title.
fn titles(cell: &TableCell) -> (Option<&String>, Option<&String>) {
    (
        cell.column_header.as_ref().filter(|_| cell.row > 1),
        cell.row_header.as_ref().filter(|_| cell.column > 1),
    )
}

/// What JAWS says on moving to a cell of Word's table with its table keys or Tab:
/// - across a row, the column's title, the cell, else its column ("Hours | 9 to 9", "Branch |
///   column 1 of 2" [obs, 1034/s10, s12]); along a column, the row's title, else its row
///   ("Branch | Hours", "Branch | row 1 of 2" [obs, 1034/s11, 1035/s5]);
/// - to the first or last cell, "beginning of table" or "last cell in table", then the row's
///   and the column's titles, the cell, else its row and column ("beginning of table | Branch |
///   row 1 of 2 | column 1 of 2", "last cell in table | Central | Hours | 9 to 9" [obs,
///   1035/s12, s13]);
/// - "Selected" where Tab selected its text, and at the table's last cell, that Tab adds a row
///   [obs, 518/s15, 1035/s9].
///
/// `None` when a move across or along left the caret in its cell.
pub(crate) fn cell_speech(
    before: Option<&TextContext>,
    after: &TextContext,
    how: CellMove,
) -> Option<String> {
    let now = cell_of(after)?;
    let was = before.and_then(cell_of);
    if matches!(how, CellMove::Across | CellMove::Along)
        && was.is_some_and(|w| (w.table, w.row, w.column) == (now.table, now.row, now.column))
    {
        return None;
    }
    let (column_title, row_title) = titles(now);
    let column_place = || msg::word_column_of(now.column, now.columns).render();
    let row_place = || msg::word_row_of(now.row, now.rows).render();
    // "Selected" right after the cell, before its place [obs, 1040/s10, s11].
    let selected = after.selection.is_some_and(|(start, end)| start < end);
    let cell_said = |said: &mut Vec<String>| {
        said.push(cell_line(after));
        if selected {
            said.push(msg::word_cell_selected().to_string());
        }
    };
    let mut said = Vec::new();
    let next_row = how == CellMove::Next && was.is_some_and(|w| w.row != now.row);
    match how {
        // On to the next row's first cell: its row's place and its column's [obs, 1040/s11].
        CellMove::Next if next_row => {
            cell_said(&mut said);
            said.push(row_place());
            said.push(column_place());
        }
        CellMove::Across | CellMove::Next => {
            said.extend(column_title.cloned());
            cell_said(&mut said);
            if column_title.is_none() {
                said.push(column_place());
            }
        }
        CellMove::Along => {
            match (row_title, now.row_header.is_none()) {
                (Some(title), _) => said.push(title.clone()),
                // A table without row titles [obs, 1040/s13].
                (None, true) => said.push(msg::word_row_no_title(now.row).render()),
                (None, false) => {}
            }
            cell_said(&mut said);
            if row_title.is_none() && now.row_header.is_some() {
                said.push(row_place());
            }
        }
        CellMove::First | CellMove::Last => {
            said.push(if how == CellMove::First {
                msg::word_table_start().to_string()
            } else {
                msg::word_last_cell().to_string()
            });
            said.extend(row_title.cloned());
            said.extend(column_title.cloned());
            cell_said(&mut said);
            if row_title.is_none() {
                said.push(row_place());
            }
            if column_title.is_none() {
                said.push(column_place());
            }
        }
    }
    if (now.row, now.column) == (now.rows, now.columns) {
        if how != CellMove::Last {
            said.push(msg::word_last_cell().to_string());
        }
        said.push(msg::word_tab_adds_row().to_string());
    }
    Some(said.join("\n"))
}

/// Say Cell (Ctrl+Alt+Numpad 5) in Word's table, and with `table` Say Current Focus: the
/// column's and the row's titles, the cell, its column and row, at the last cell that Tab adds a
/// row, and for the focus the table ("Hours | Central | 9 to 9 | column 2 of 2 | row 2 of 2 |
/// Last cell in table. | Warning! ... | table 1", "Branch | column 1 of 2 | row 1 of 2 | table
/// 1" [obs, 1035/s10, s11; 1034/s13]). `None` outside a table.
pub(crate) fn cell_focus(context: &TextContext, table: bool) -> Option<String> {
    let cell = cell_of(context)?;
    let (column_title, row_title) = titles(cell);
    let mut said = Vec::new();
    // The first row holds the column titles; a table without row titles says so [obs,
    // 1040/s14].
    if cell.row == 1 && cell.column > 1 {
        said.push(msg::word_title_row().to_string());
    }
    said.extend(column_title.cloned());
    match row_title {
        Some(title) => said.push(title.clone()),
        None if cell.row_header.is_none() => said.push(msg::word_row_no_title(cell.row).render()),
        None => {}
    }
    said.push(cell_line(context));
    said.push(msg::word_column_of(cell.column, cell.columns).render());
    said.push(msg::word_row_of(cell.row, cell.rows).render());
    if (cell.row, cell.column) == (cell.rows, cell.columns) {
        said.push(msg::word_last_cell_said().to_string());
        said.push(msg::word_tab_adds_row().to_string());
    }
    if table {
        said.push(msg::word_table(cell.table).render());
    }
    Some(said.join("\n"))
}

/// Say Current Focus in Word's document: in a table, the cell ([`cell_focus`]); elsewhere the
/// caret's line as Say Line reads it, " heading level 1 Library Hours" [obs, 1035/s16].
/// `None` outside a document whose structure the platform reads.
pub(crate) fn document_focus(context: &TextContext) -> Option<String> {
    context.structure.as_ref()?;
    // A form field at the caret, as a control [obs, 1056/s9].
    if let Some(field) = form_field_at(context) {
        return Some(form_field_speech(context, field, false));
    }
    cell_focus(context, true).or_else(|| {
        Some(spoken_line(
            context,
            crate::editing::line_at(context).trim().to_owned(),
        ))
    })
}

/// The caret's line in a cell, "blank" when it has no text.
fn cell_line(context: &TextContext) -> String {
    let line = crate::editing::line_at(context);
    match line.trim() {
        "" => msg::blank().to_string(),
        text => text.to_owned(),
    }
}

/// `speech` for a key that moved the caret to `after`'s line, with what JAWS says around a
/// line of Word's ([`line_change`]); `None` when JAWS says nothing. Outside Word's documents
/// (no structure) and for other keys, `speech` as it is.
pub(crate) fn around_line(
    key: EditKey,
    before: Option<&TextContext>,
    after: &TextContext,
    speech: String,
) -> Option<String> {
    let (Some(now), true) = (
        after.structure.as_ref(),
        matches!(
            key,
            EditKey::Line | EditKey::TopOfFile | EditKey::BottomOfFile
        ),
    ) else {
        return Some(speech);
    };
    let was = before.and_then(|b| b.structure.as_ref());
    // A move to the document's top or bottom that leaves the caret on its line says nothing:
    // Ctrl+End on the last line [obs, 518/s17].
    if matches!(key, EditKey::TopOfFile | EditKey::BottomOfFile)
        && was.is_some_and(|w| w.line == now.line)
    {
        return None;
    }
    let (first, last) = line_change(was, now);
    Some(
        first
            .into_iter()
            .chain(std::iter::once(speech))
            .chain(last)
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

/// What Say All says before a paragraph of Word's it reads after `was`, where line reading says
/// some of it after the line ([`line_change`]): first what was left, "exit list", "out of
/// table"; then what is entered, "Entering List", "table 1 | uniform Table", and a table's next
/// row, "Row 2", with no "new cell" [obs, 1038/s4, s12: the order of two trials of three; JAWS's
/// own order varies, "Row 2" said after the row's first cell in s4's].
fn say_all_change(was: &LineStructure, now: &LineStructure) -> Vec<String> {
    let mut said = Vec::new();
    if was.list_item && !now.list_item {
        said.push(msg::word_exit_list().to_string());
    }
    if was.cell.is_some() && now.cell.is_none() {
        said.push(msg::word_out_of_table().to_string());
    }
    if !was.list_item && now.list_item {
        said.push(msg::word_entering_list().to_string());
    }
    match (was.cell.as_ref(), now.cell.as_ref()) {
        (Some(w), Some(n)) if w.table == n.table => {
            if w.row != n.row {
                said.push(msg::word_row(n.row).render());
            }
        }
        (_, Some(n)) => {
            said.push(msg::word_table(n.table).render());
            said.push(msg::word_table_uniform().to_string());
        }
        _ => {}
    }
    said
}

/// What JAWS says before and after a line of Word's it moved to from `was`:
/// - into a table, "table 1 | uniform Table | new cell", then the cell [obs, 518/s13, s18];
/// - into another cell of it, "new cell" [obs, 518/s14];
/// - out of it, "out of table" [obs, 518/s16, s19];
/// - into a list, "Entering List" after the line [obs, 518/s10]; out of it, "exit list" [obs,
///   518/s12].
fn line_change(was: Option<&LineStructure>, now: &LineStructure) -> (Vec<String>, Vec<String>) {
    let Some(was) = was else {
        return (Vec::new(), Vec::new());
    };
    let mut first = Vec::new();
    match (was.cell.as_ref(), now.cell.as_ref()) {
        (Some(w), Some(n)) if w.table == n.table => {
            if (w.row, w.column) != (n.row, n.column) {
                first.push(msg::word_new_cell().to_string());
            }
        }
        (_, Some(n)) => {
            first.push(msg::word_table(n.table).render());
            first.push(msg::word_table_uniform().to_string());
            first.push(msg::word_new_cell().to_string());
        }
        (Some(_), None) => first.push(msg::word_out_of_table().to_string()),
        (None, None) => {}
    }
    let last = match (was.list_item, now.list_item) {
        (false, true) => vec![msg::word_entering_list().to_string()],
        (true, false) => vec![msg::word_exit_list().to_string()],
        _ => Vec::new(),
    };
    (first, last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::TableCell;

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

    fn at(
        line: (usize, usize),
        heading: Option<u8>,
        list_item: bool,
        cell: Option<(usize, usize)>,
    ) -> TextContext {
        TextContext {
            text: String::new(),
            caret: line.0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: Some(LineStructure {
                line,
                heading,
                list_item,
                cell: cell.map(|(row, column)| TableCell {
                    table: 1,
                    row,
                    column,
                    rows: 2,
                    columns: 2,
                    column_header: None,
                    row_header: None,
                }),
                form_fields: Vec::new(),
                links: Vec::new(),
                comments: Vec::new(),
                footnotes: Vec::new(),
                endnotes: Vec::new(),
            }),
        }
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

    /// Word's form fields at the caret are said as controls named by their line's text before
    /// them [obs, 1056/s1-s4; 1057/s14]; a line read with them in it says a text field by its
    /// name and a check box as one [obs, 1055/s2, s8].
    #[test]
    fn form_fields_are_said_as_controls() {
        let mut name = field(FormFieldKind::Text, "Name", 51, 56);
        name.value = "     ".into();
        let at_name = form(51, (45, 57), vec![name.clone()]);
        assert_eq!(
            form_field_at(&at_name)
                .map(|f| form_field_speech(&at_name, f, true))
                .as_deref(),
            Some("Name:  \u{1d} Edit \u{1e} ")
        );
        // A line read from its start: the field by its name.
        let line_start = form(45, (45, 57), vec![name.clone()]);
        assert_eq!(form_field_at(&line_start), None);
        assert_eq!(
            spoken_line(&line_start, "Name:".into()).trim(),
            "Name: Name"
        );
        // Typed into: what it holds.
        name.value = "ab     ".into();
        let typed = form(53, (45, 57), vec![name]);
        assert_eq!(
            form_field_at(&typed)
                .map(|f| form_field_speech(&typed, f, true))
                .as_deref(),
            Some("Name:  \u{1d} Edit \u{1e} \nab")
        );
        let mut member = field(FormFieldKind::CheckBox, "Member", 65, 65);
        member.checked = Some(false);
        let at_member = form(65, (57, 66), vec![member.clone()]);
        assert_eq!(
            form_field_at(&at_member)
                .map(|f| form_field_speech(&at_member, f, true))
                .as_deref(),
            Some("Member:  \u{1d} check box \u{1e}  \u{1d}not checked\u{1e} ")
        );
        assert_eq!(
            spoken_line(&form(57, (57, 66), vec![member.clone()]), "Member:".into()),
            "Member:  \u{1d} check box \u{1e}  \u{1d}not checked\u{1e}"
        );
        member.checked = Some(true);
        assert_eq!(
            form_field_state(&form(65, (57, 66), vec![member])).as_deref(),
            Some(" \u{1d}checked\u{1e} ")
        );
        let mut branch = field(FormFieldKind::DropDown, "Branch", 87, 87);
        branch.value = "Central".into();
        branch.choice = Some((1, 3));
        let at_branch = form(87, (79, 88), vec![branch]);
        assert_eq!(
            form_field_at(&at_branch)
                .map(|f| form_field_speech(&at_branch, f, true))
                .as_deref(),
            Some("Branch:  \u{1d} Combo box \u{1e} \nCentral\n1 of 3")
        );
    }

    /// A line's comment is marked where its text begins and set apart where it ends: "The
    /// library opens at has comment nine in the morning ."; Alt+Shift+Apostrophe reads the
    /// comment at the caret with its author, or says there is none [obs, 1081/s1, s3, s8].
    #[test]
    fn a_lines_comment_is_marked_and_read() {
        let text = "Library Notes\rThe library opens at nine in the morning.\rIt is open.";
        let context = |caret: usize| TextContext {
            text: text.into(),
            caret,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: Some(LineStructure {
                line: (14, 55),
                comments: vec![crate::event::DocumentComment {
                    start: 35,
                    end: 54,
                    text: "Check the summer hours.".into(),
                    author: "Lab".into(),
                }],
                ..Default::default()
            }),
        };
        assert_eq!(
            line_with_fields(&context(14)).as_deref(),
            Some("The library opens at  \u{1d}has comment \u{1e} nine in the morning .")
        );
        assert_eq!(
            comment_at_caret(&context(14)).as_deref(),
            Some("No comment mark at cursor.")
        );
        assert_eq!(
            comment_at_caret(&context(35)).as_deref(),
            Some("Check the summer hours.\nby Lab")
        );
    }

    /// A line's footnote reference, which Word's text does not show, is said where it stands,
    /// by its number: "It is open every day footnote 1." [obs, 1083/s4]; one at the line's end
    /// too.
    #[test]
    fn a_lines_footnote_is_said_by_its_number() {
        let text = "Library Notes\rIt is open every day.\r";
        let context = |at: usize| TextContext {
            text: text.into(),
            caret: 14,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: Some(LineStructure {
                line: (14, 35),
                footnotes: vec![crate::event::DocumentFootnote {
                    at,
                    number: 1,
                    ..Default::default()
                }],
                ..Default::default()
            }),
        };
        assert_eq!(
            line_with_fields(&context(34)).as_deref(),
            Some("It is open every day \u{1d}footnote \u{1e} 1.")
        );
        assert_eq!(
            line_with_fields(&context(35)).as_deref(),
            Some("It is open every day. \u{1d}footnote \u{1e} 1")
        );
        // An endnote by its mark: "three weeks end note i." [obs, 1090/s7].
        let mut endnote = context(34);
        if let Some(s) = endnote.structure.as_mut() {
            s.endnotes = std::mem::take(&mut s.footnotes);
            if let Some(note) = s.endnotes.first_mut() {
                note.mark = "i".into();
            }
        }
        assert_eq!(
            line_with_fields(&endnote).as_deref(),
            Some("It is open every day \u{1d}end note \u{1e} i.")
        );
    }

    /// A line's links are marked as JAWS marks them: "See the  link opening hours" [obs,
    /// 1055/s1].
    #[test]
    fn a_lines_links_are_marked() {
        let mut context = form(14, (13, 52), Vec::new());
        context.text = "Library Card\rSee the opening hours before you come.\r".to_owned();
        if let Some(s) = context.structure.as_mut() {
            s.links = vec![(21, 34)];
        }
        assert_eq!(
            spoken_line(&context, String::new()),
            "See the  \u{1d}link \u{1e} opening hours before you come."
        );
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

    #[test]
    fn a_heading_is_said_with_its_level() {
        let heading = at((0, 14), Some(1), false, None);
        assert_eq!(
            spoken_line(&heading, "Library Hours".into()),
            "\u{1d} heading level 1 \u{1e} Library Hours"
        );
        let text = at((14, 90), None, false, None);
        assert_eq!(spoken_line(&text, "The library".into()), "The library");
    }

    /// A list entered and left [obs, 518/s10, s12].
    #[test]
    fn a_list_is_entered_and_left() {
        let heading = at((136, 149), Some(2), false, None);
        let item = at((149, 177), None, true, None);
        let said = around_line(EditKey::Line, Some(&heading), &item, "Weekday".into());
        assert_eq!(said.as_deref(), Some("Weekday\nEntering List"));
        let next = at((208, 217), Some(2), false, None);
        let said = around_line(EditKey::Line, Some(&item), &next, "Branches".into());
        assert_eq!(said.as_deref(), Some("Branches\nexit list"));
    }

    /// A table entered, its cell changed, and left [obs, 518/s13, s14, s16].
    #[test]
    fn a_tables_cells_are_entered_changed_and_left() {
        let heading = at((208, 217), Some(2), false, None);
        let branch = at((217, 223), None, false, Some((1, 1)));
        let central = at((228, 235), None, false, Some((2, 1)));
        let after = at((241, 260), None, false, None);
        assert_eq!(
            around_line(EditKey::Line, Some(&heading), &branch, "Branch".into()).as_deref(),
            Some("table 1\nuniform Table\nnew cell\nBranch")
        );
        assert_eq!(
            around_line(EditKey::Line, Some(&branch), &central, "Central".into()).as_deref(),
            Some("new cell\nCentral")
        );
        assert_eq!(
            around_line(
                EditKey::Line,
                Some(&central),
                &after,
                "The page ends here.".into()
            )
            .as_deref(),
            Some("out of table\nThe page ends here.")
        );
        // Ctrl+End on the last line moves no line: nothing [obs, 518/s17].
        assert_eq!(
            around_line(
                EditKey::BottomOfFile,
                Some(&after),
                &after,
                "Bottom of file".into()
            ),
            None
        );
    }

    /// The fixture's table: "Branch | Hours" over "Central | 9 to 9".
    fn cell(row: usize, column: usize, text: &str) -> TextContext {
        let mut context = at((0, text.len()), None, false, Some((row, column)));
        context.text = text.to_owned();
        context.caret = 0;
        if let Some(cell) = context.structure.as_mut().and_then(|s| s.cell.as_mut()) {
            cell.column_header = ["Branch", "Hours"].get(column - 1).map(|t| t.to_string());
            cell.row_header = ["Branch", "Central"].get(row - 1).map(|t| t.to_string());
        }
        context
    }

    /// The table keys and Tab say the title for the way the cell was entered, or its place
    /// [obs, 1034/s10-s13, 518/s15].
    #[test]
    fn a_cell_is_said_with_its_title_or_place() {
        let central = cell(2, 1, "Central");
        let last = cell(2, 2, "9 to 9");
        let hours = cell(1, 2, "Hours");
        let branch = cell(1, 1, "Branch ");
        let tab_adds_row = "Warning! Pressing tab here adds an extra row.";
        let warning = format!("last cell in table\n{tab_adds_row}");
        assert_eq!(
            cell_speech(Some(&central), &last, CellMove::Across),
            Some(format!("Hours\n9 to 9\n{warning}"))
        );
        assert_eq!(
            cell_speech(Some(&last), &hours, CellMove::Along).as_deref(),
            Some("Branch\nHours")
        );
        assert_eq!(
            cell_speech(Some(&hours), &branch, CellMove::Across).as_deref(),
            Some("Branch\ncolumn 1 of 2")
        );
        assert_eq!(
            cell_speech(Some(&central), &branch, CellMove::Along).as_deref(),
            Some("Branch\nrow 1 of 2")
        );
        let mut selected = last.clone();
        selected.selection = Some((0, 6));
        assert_eq!(
            cell_speech(Some(&central), &selected, CellMove::Across),
            Some(format!("Hours\n9 to 9\nSelected\n{warning}"))
        );
        assert_eq!(cell_speech(Some(&last), &last, CellMove::Across), None);
        // The first and last cells [obs, 1035/s12, s13].
        assert_eq!(
            cell_speech(Some(&last), &branch, CellMove::First).as_deref(),
            Some("beginning of table\nBranch\nrow 1 of 2\ncolumn 1 of 2")
        );
        assert_eq!(
            cell_speech(Some(&branch), &last, CellMove::Last),
            Some(format!(
                "last cell in table\nCentral\nHours\n9 to 9\n{tab_adds_row}"
            ))
        );
        // Say Current Focus and Say Cell [obs, 1034/s13, 1035/s10, s11].
        assert_eq!(
            cell_focus(&branch, true).as_deref(),
            Some("Branch\ncolumn 1 of 2\nrow 1 of 2\ntable 1")
        );
        assert_eq!(
            cell_focus(&last, false),
            Some(format!(
                "Hours\nCentral\n9 to 9\ncolumn 2 of 2\nrow 2 of 2\nLast cell in table.\n{tab_adds_row}"
            ))
        );
    }

    /// The fixture's first heading as Word reports its font.
    fn heading_font() -> TextContext {
        let mut heading = at((0, 14), Some(1), false, None);
        heading.font = Some(crate::event::FontInfo {
            family: "Calibri".into(),
            size_tenths: 160,
            alignment: Some("left".into()),
            color: Some(crate::color::Rgb::new(0, 0, 0)),
            background: Some(crate::color::Rgb::new(255, 255, 255)),
            style: Some("Heading 1".into()),
            ..Default::default()
        });
        heading
    }

    /// Say Font in Word names the style and the outline level [obs, 1034/s1].
    #[test]
    fn say_font_names_the_style_and_the_outline_level() {
        assert_eq!(
            font(&heading_font(), None).as_deref(),
            Some(
                "16 point
black on white
Calibri
Heading 1 style
Paragraph formatting: Aligned left
Outline level: 1"
            )
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

    /// Bold, italic and underline come first, in JAWS's words [obs, 1040/s3, s5, s7].
    #[test]
    fn say_font_says_bold_italic_and_underline_first() {
        let mut formatted = heading_font();
        if let Some(font) = formatted.font.as_mut() {
            font.bold = true;
            font.italic = true;
            font.underline = true;
        }
        let said = font(&formatted, None).unwrap_or_default();
        assert!(said.starts_with("bolded\nitalicized\nSingle  underline\n16 point"));
    }

    /// The line spacing after the style, by Word's rule [obs, 1052]; a multiple as its true
    /// number of lines, where JAWS divides by the font's size and drops the fraction ("0 lines."
    /// at 16 points [obs, 1034/s1]; an accepted divergence).
    #[test]
    fn say_font_says_the_line_spacing() {
        assert_eq!(
            font(
                &heading_font(),
                Some(&LineSpacing {
                    rule: 5,
                    centipoints: 1295
                })
            )
            .as_deref(),
            Some(
                "16 point
black on white
Calibri
Heading 1 style
Line spacing: 1.08 lines.
Paragraph formatting: Aligned left
Outline level: 1"
            )
        );
        let said = |rule, centipoints| line_spacing(&LineSpacing { rule, centipoints });
        assert_eq!(said(0, 1200), "Line spacing: Single");
        assert_eq!(said(5, 1380), "Line spacing: 1.15 lines.");
        assert_eq!(said(1, 1800), "Line spacing: one point five lines.");
        assert_eq!(said(2, 2400), "Line spacing: Double");
        assert_eq!(said(5, 3600), "Line spacing: 3 lines.");
        assert_eq!(said(5, 3000), "Line spacing: 2.5 lines.");
        assert_eq!(said(3, 1800), "Line spacing: At Least 18 points");
        assert_eq!(said(4, 2000), "Line spacing: Exactly 20 points");
    }

    /// Say All reads Word's document by its paragraphs from the caret's: the caret's as it is,
    /// a heading with its level, and before a paragraph what changed: the list entered and left,
    /// the table entered, its next row, "out of table" [obs, 1038/s4, s12].
    #[test]
    fn say_all_reads_the_paragraphs_with_their_structure() {
        // Word's text: paragraphs end in \r, a cell in \u{7}.
        let parts = [
            ("Library Hours\r", Some(1), false, None),
            ("The library opens.\r", None, false, None),
            ("What changes\r", Some(2), false, None),
            ("Weekday hours are longer.\r", None, true, None),
            ("Weekend hours stay the same.\r", None, true, None),
            ("Branches\r", Some(2), false, None),
            ("Branch\u{7}", None, false, Some((1, 1))),
            ("Hours\u{7}", None, false, Some((1, 2))),
            ("Central\u{7}", None, false, Some((2, 1))),
            ("The page ends here.\r", None, false, None),
        ];
        let mut text = String::new();
        let mut paragraphs = Vec::new();
        for (part, heading, list_item, cell) in parts {
            paragraphs.push(DocumentParagraph {
                start: text.chars().count(),
                heading,
                list_item,
                cell: cell.map(|(row, column)| TableCell {
                    table: 1,
                    row,
                    column,
                    rows: 1,
                    columns: 2,
                    column_header: None,
                    row_header: None,
                }),
            });
            text.push_str(part);
        }
        let context = TextContext {
            text,
            caret: 0,
            ..at((0, 0), Some(1), false, None)
        };
        let said: Vec<String> = say_all_pieces(&context, &paragraphs)
            .into_iter()
            .map(|(_, piece)| piece.replace('\n', " | "))
            .collect();
        assert_eq!(
            said,
            [
                "Library Hours",
                "The library opens.",
                "\u{1d} heading level 2 \u{1e} What changes",
                "Entering List | Weekday hours are longer.",
                "Weekend hours stay the same.",
                "exit list | \u{1d} heading level 2 \u{1e} Branches",
                "table 1 | uniform Table | Branch",
                "Hours",
                "Row 2 | Central",
                "out of table | The page ends here.",
            ]
        );
        // From the caret's paragraph on.
        let from_list = TextContext {
            caret: paragraphs.get(3).map_or(0, |p| p.start + 3),
            ..context
        };
        assert_eq!(
            say_all_pieces(&from_list, &paragraphs)
                .first()
                .map(|(_, p)| p.as_str()),
            Some("Weekday hours are longer.")
        );
    }
}
