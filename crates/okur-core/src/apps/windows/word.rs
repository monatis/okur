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
}
