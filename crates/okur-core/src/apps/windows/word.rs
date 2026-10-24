//! Microsoft Word's documents, read as JAWS reads them. The platform gives the caret's line
//! structure from Word's text (`LineStructure`: Word's own line, the paragraph's heading level
//! and list, the table cell); this module words what JAWS says of it: a heading's level before
//! its line, a list entered or left after it, and a table's cells, entered, changed and left
//! before it [obs, 518].

use crate::editing::EditKey;
use crate::event::{DocumentParagraph, LineStructure, TableCell, TextContext};
use crate::messages::msg;

/// `line` spoken as JAWS speaks a line of Word's: a heading with its level before it,
/// " heading level 1 Library Hours" [obs, 518/s1, s9].
pub(crate) fn spoken_line(context: &TextContext, line: String) -> String {
    with_heading(context.structure.as_ref().and_then(|s| s.heading), line)
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
/// 1034/s1, s3]. JAWS also says the line spacing after the style ("Line spacing: 1 lines.",
/// "one point five lines.", "Double" [obs, 1040/s1, s17, s19]), which is not read from Word yet.
/// `None` outside a document whose structure the platform reads.
pub(crate) fn font(context: &TextContext) -> Option<String> {
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
        let size = if font.size_tenths % 10 == 0 {
            (font.size_tenths / 10).to_string()
        } else {
            format!("{}.{}", font.size_tenths / 10, font.size_tenths % 10)
        };
        said.push(msg::font_size(size).render());
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
    if let Some(alignment) = &font.alignment {
        said.push(msg::word_paragraph_aligned(alignment.as_str()).render());
    }
    said.push(match structure.heading {
        Some(level) => msg::word_outline_level(u32::from(level)).render(),
        None => msg::word_outline_body().to_string(),
    });
    Some(said.join("\n"))
}

/// The view Word shows its document in, as Say Window Title says it after the title, from the
/// name of the status bar's view button that is on: "Print Layout" is "Print View" [obs,
/// 519/s1, 1039/s7]. Web Layout's and Read Mode's are not recorded yet: `None`, the title alone.
pub(crate) fn view_name(button: &str) -> Option<String> {
    (button == "Print Layout").then(|| msg::word_view_print().to_string())
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

/// How the caret came to a cell of Word's table, which decides what JAWS says of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellMove {
    /// Across a row: Ctrl+Alt+Left or Right, Tab.
    Across,
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
    let mut said = Vec::new();
    match how {
        CellMove::Across => {
            said.extend(column_title.cloned());
            said.push(cell_line(after));
            if column_title.is_none() {
                said.push(column_place());
            }
        }
        CellMove::Along => {
            said.extend(row_title.cloned());
            said.push(cell_line(after));
            if row_title.is_none() {
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
            said.push(cell_line(after));
            if row_title.is_none() {
                said.push(row_place());
            }
            if column_title.is_none() {
                said.push(column_place());
            }
        }
    }
    if after.selection.is_some_and(|(start, end)| start < end) {
        said.push(msg::word_cell_selected().to_string());
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
    let mut said: Vec<String> = column_title.into_iter().chain(row_title).cloned().collect();
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
            }),
        }
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
            font(&heading_font()).as_deref(),
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
        let said = font(&formatted).unwrap_or_default();
        assert!(said.starts_with("bolded\nitalicized\nSingle  underline\n16 point"));
    }

    /// JAWS says the line spacing after the style [obs, 1034/s1].
    #[test]
    #[ignore = "known gap: Word's line spacing, which its UI Automation text does not report"]
    fn say_font_says_the_line_spacing() {
        assert_eq!(
            font(&heading_font()).as_deref(),
            Some(
                "16 point
black on white
Calibri
Heading 1 style
Line spacing: 0 lines.
Paragraph formatting: Aligned left
Outline level: 1"
            )
        );
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
