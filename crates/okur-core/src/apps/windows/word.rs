//! Microsoft Word's documents, read as JAWS reads them. The platform gives the caret's line
//! structure from Word's text (`LineStructure`: Word's own line, the paragraph's heading level
//! and list, the table cell); this module words what JAWS says of it: a heading's level before
//! its line, a list entered or left after it, and a table's cells, entered, changed and left
//! before it [obs, 518].

use crate::editing::EditKey;
use crate::event::{LineStructure, TextContext};
use crate::messages::msg;

/// `line` spoken as JAWS speaks a line of Word's: a heading with its level before it,
/// " heading level 1 Library Hours" [obs, 518/s1, s9].
pub(crate) fn spoken_line(context: &TextContext, line: String) -> String {
    match context.structure.as_ref().and_then(|s| s.heading) {
        Some(level) if !line.trim().is_empty() => format!(
            "\u{1d} {} \u{1e} {line}",
            msg::role_heading_level(u32::from(level)).render()
        ),
        _ => line,
    }
}

/// What JAWS says on moving to another cell of Word's table with its table keys or Tab
/// (`across` a row, or along a column): the title for the way the cell was entered (the
/// column's header across, the row's first cell along), the cell, else its column or row
/// ("Hours | 9 to 9", "Branch | Hours", "Branch | column 1 of 2" [obs, 522/s10-s12]);
/// "Selected" where Tab selected its text, and at the table's last cell, that Tab adds a row
/// [obs, 518/s15]. `None` when the caret is in no other cell.
pub(crate) fn cell_speech(
    before: Option<&TextContext>,
    after: &TextContext,
    across: bool,
) -> Option<String> {
    let now = after.structure.as_ref()?.cell.as_ref()?;
    let was = before
        .and_then(|b| b.structure.as_ref())
        .and_then(|s| s.cell.as_ref());
    if was.is_some_and(|w| (w.table, w.row, w.column) == (now.table, now.row, now.column)) {
        return None;
    }
    // A cell of the first row is its column's title, of the first column its row's.
    let title = if across {
        now.column_header.as_ref().filter(|_| now.row > 1)
    } else {
        now.row_header.as_ref().filter(|_| now.column > 1)
    };
    let mut said: Vec<String> = title.cloned().into_iter().collect();
    said.push(cell_line(after));
    if title.is_none() {
        said.push(if across {
            msg::word_column_of(now.column, now.columns).render()
        } else {
            msg::word_row_of(now.row, now.rows).render()
        });
    }
    if after.selection.is_some_and(|(start, end)| start < end) {
        said.push(msg::word_cell_selected().to_string());
    }
    if (now.row, now.column) == (now.rows, now.columns) {
        said.push(msg::word_last_cell().to_string());
        said.push(msg::word_tab_adds_row().to_string());
    }
    Some(said.join("\n"))
}

/// Say Current Focus in a cell of Word's table: the cell, its column, row and table, "Branch
/// | column 1 of 2 | row 1 of 2 | table 1" [obs, 522/s13]. `None` outside a table.
pub(crate) fn cell_focus(context: &TextContext) -> Option<String> {
    let cell = context.structure.as_ref()?.cell.as_ref()?;
    Some(
        [
            cell_line(context),
            msg::word_column_of(cell.column, cell.columns).render(),
            msg::word_row_of(cell.row, cell.rows).render(),
            msg::word_table(cell.table).render(),
        ]
        .join("\n"),
    )
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
    /// [obs, 522/s10-s13, 518/s15].
    #[test]
    fn a_cell_is_said_with_its_title_or_place() {
        let central = cell(2, 1, "Central");
        let last = cell(2, 2, "9 to 9");
        let hours = cell(1, 2, "Hours");
        let branch = cell(1, 1, "Branch ");
        let warning = "last cell in table\nWarning! Pressing tab here adds an extra row.";
        assert_eq!(
            cell_speech(Some(&central), &last, true),
            Some(format!("Hours\n9 to 9\n{warning}"))
        );
        assert_eq!(
            cell_speech(Some(&last), &hours, false).as_deref(),
            Some("Branch\nHours")
        );
        assert_eq!(
            cell_speech(Some(&hours), &branch, true).as_deref(),
            Some("Branch\ncolumn 1 of 2")
        );
        let mut selected = last.clone();
        selected.selection = Some((0, 6));
        assert_eq!(
            cell_speech(Some(&central), &selected, true),
            Some(format!("Hours\n9 to 9\nSelected\n{warning}"))
        );
        assert_eq!(cell_speech(Some(&last), &last, true), None);
        assert_eq!(
            cell_focus(&branch).as_deref(),
            Some("Branch\ncolumn 1 of 2\nrow 1 of 2\ntable 1")
        );
    }
}
