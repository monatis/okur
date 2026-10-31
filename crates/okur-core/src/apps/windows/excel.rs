//! Microsoft Excel, as JAWS's script for it reads a workbook: a cell by its value, then its
//! coordinates, "12 | B 2" [obs, 521/s3], and Insert+C the coordinates alone, "B 2" [obs,
//! 521/s6]. A cell is UI Automation's table cell in Excel's grid window (`EXCEL7`), named by
//! its reference ("B2").

use crate::aom::Role;
use crate::apps::{AppContext, AppModule, FocusResponse};
use crate::event::FocusEvent;
use crate::input::Key;
use crate::messages::msg;

/// Excel's grid window, which holds a sheet's cells.
const GRID_WINDOW: &str = "EXCEL7";
/// Excel's cell editor, F2's (and typing's) edit.
const EDITOR_WINDOW: &str = "EXCEL6";

/// Excel's module.
#[derive(Debug, Default)]
pub struct Excel {
    /// The last cell reached was empty, which a Ctrl arrow leaves for a new data region.
    from_blank: bool,
}

impl AppModule for Excel {
    fn owns(&self, focus: &FocusEvent) -> bool {
        cell(focus).is_some() || grid(focus) || editor(focus)
    }

    /// A cell Okur's list closes back onto, "22 | B 4" [obs, 521/s10, s14].
    fn focus_again(&self, focus: &FocusEvent) -> Option<String> {
        let coordinates = cell(focus)?;
        let value = focus.value.as_deref().map(str::trim).unwrap_or_default();
        Some(if value.is_empty() {
            coordinates
        } else {
            format!("{value}\n{coordinates}")
        })
    }

    /// A cell the menus give back, "Leaving menus | 14 | C 4" [obs, 1116/s7].
    fn says_focus_after_menus(&self, focus: &FocusEvent) -> bool {
        cell(focus).is_some()
    }

    /// A cell reached: its value, then its coordinates [obs, 521/s2-s5, s7, s8, s11, s12];
    /// by Ctrl+End and Ctrl+Home, which cell it is first, "Last Cell | 14 | C 4"; by a Ctrl
    /// arrow, that it is a data region's edge after, "Weekends | C 1 | edge of current data
    /// region" [obs, 1115/s1-s6].
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        // F2 into the cell's editor: JAWS says nothing [obs, 1118/s6].
        if editor(focus) {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            return FocusResponse::Silent;
        }
        // An empty cell: the grid itself has the focus, and the cell comes from Excel's object
        // model, "blank | D 1 | Active Cell: D1" [obs, 1118/s3, s4].
        if grid(focus) {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            cx.ask_sheet = true;
            self.from_blank = true;
            return FocusResponse::Silent;
        }
        let Some(coordinates) = cell(focus) else {
            return FocusResponse::Pass;
        };
        // A selecting key leaves the focus on the active cell: the selection is read from
        // Excel's object model and said for what it changed [obs, 1120, 1121].
        if cx
            .key
            .and_then(|(key, m)| SelectionKey::of(key, m))
            .is_some()
        {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            cx.ask_sheet = true;
            return FocusResponse::Silent;
        }
        cx.note_window(focus);
        // A native control's keys are the application's, as for any cell the engine reads.
        *cx.app_mode = crate::engine::AppMode::NativeControl;
        let value = focus.value.as_deref().map(str::trim).unwrap_or_default();
        let mut lines = Vec::new();
        let ctrl = cx
            .key
            .map(|(_, m)| m.ctrl && !m.shift && !m.alt && !m.insert && !m.win)
            .unwrap_or(false);
        let key = cx.key.map(|(k, _)| k).filter(|_| ctrl);
        match key {
            Some(Key::End) => lines.push(msg::excel_last_cell().to_string()),
            Some(Key::Home) => lines.push(msg::excel_first_cell().to_string()),
            _ => {}
        }
        // An empty cell by its coordinates alone [hyp: no empty cell recorded yet].
        if !value.is_empty() {
            lines.push(value.to_owned());
        }
        lines.push(coordinates);
        // From an empty cell a new data region starts, "8 | C 2 | start of new Data Region"
        // [obs, 1118/s5].
        if matches!(
            key,
            Some(Key::LeftArrow | Key::RightArrow | Key::UpArrow | Key::DownArrow)
        ) {
            lines.push(
                if self.from_blank {
                    msg::excel_data_region_start()
                } else {
                    msg::excel_data_region_edge()
                }
                .to_string(),
            );
        }
        self.from_blank = value.is_empty();
        FocusResponse::Speak(crate::announcement::Announcement::from_text(
            &lines.join("\n"),
        ))
    }
}

/// JAWS's row and column readers in Excel, from its key map: Shift+Insert+Up, Home and PageUp
/// (`ReadRow`, `ReadFromStartOfRow`, `ReadToEndOfRow`), Ctrl+Insert+Up, Home and PageUp
/// (`readColumn`, `readFromTopOfColumn`, `readToBottomOfColumn`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SheetReading {
    /// Say Line (Insert+Up): the row, cell after cell, "Branch | Weekdays | Weekends" [obs,
    /// 521/s1].
    Line,
    Row,
    RowFromStart,
    RowToEnd,
    Column,
    ColumnFromTop,
    ColumnToBottom,
}

impl SheetReading {
    /// The reader an Okur-key chord is, if any.
    pub(crate) fn of(event: &crate::input::KeyEvent) -> Option<Self> {
        let m = &event.modifiers;
        if !m.insert || m.alt || m.win {
            return None;
        }
        if !m.ctrl && !m.shift {
            return (event.key == Key::UpArrow).then_some(Self::Line);
        }
        if m.ctrl == m.shift {
            return None;
        }
        Some(match (event.key, m.shift) {
            (Key::UpArrow, true) => Self::Row,
            (Key::Home, true) => Self::RowFromStart,
            (Key::PageUp, true) => Self::RowToEnd,
            (Key::UpArrow, false) => Self::Column,
            (Key::Home, false) => Self::ColumnFromTop,
            (Key::PageUp, false) => Self::ColumnToBottom,
            _ => return None,
        })
    }

    /// What it reads of `place`: a row's cells in one breath, "Northside 10 6", from its start
    /// to the active cell, "Northside 10", or from it to the row's end, "10 6"; a column's cells
    /// one by one, "Weekdays | 12 | 10 | 22", likewise from its top or to its bottom [obs,
    /// 1114/s4-s9]. Empty cells are left out [hyp: none recorded yet]. `None` with nothing to
    /// read.
    pub(crate) fn said(self, place: &crate::event::SheetPlace) -> Option<String> {
        let (cells, at, row) = match self {
            Self::Line => (&place.row, place.row_at, false),
            Self::Row | Self::RowFromStart | Self::RowToEnd => (&place.row, place.row_at, true),
            _ => (&place.column, place.column_at, false),
        };
        let part: &[String] = match self {
            Self::Line | Self::Row | Self::Column => cells,
            Self::RowFromStart | Self::ColumnFromTop => cells.get(..=at).unwrap_or(cells),
            Self::RowToEnd | Self::ColumnToBottom => cells.get(at..).unwrap_or_default(),
        };
        let read: Vec<&str> = part
            .iter()
            .map(|c| c.trim())
            .filter(|c| !c.is_empty())
            .collect();
        (!read.is_empty()).then(|| read.join(if row { " " } else { "\n" }))
    }
}

/// Excel's keys that change the selection: Shift with an arrow, Home, End, PageUp or PageDown
/// extends it; Shift+Space selects the rows, Ctrl+Space the columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionKey {
    Extend,
    Rows,
    Columns,
}

impl SelectionKey {
    pub(crate) fn of(key: Key, m: crate::input::Modifiers) -> Option<Self> {
        if m.alt || m.insert || m.win {
            return None;
        }
        match (key, m.shift, m.ctrl) {
            (Key::Space, true, false) => Some(Self::Rows),
            (Key::Space, false, true) => Some(Self::Columns),
            (
                Key::LeftArrow
                | Key::RightArrow
                | Key::UpArrow
                | Key::DownArrow
                | Key::Home
                | Key::End
                | Key::PageUp
                | Key::PageDown,
                true,
                _,
            ) => Some(Self::Extend),
            _ => None,
        }
    }
}

/// The last column and row of an Excel sheet (XFD, 1048576).
const LAST_COLUMN: u32 = 16_384;
const LAST_ROW: u32 = 1_048_576;

/// A rectangle of cells, columns and rows from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cells {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
}

impl Cells {
    fn count(&self) -> u64 {
        u64::from(self.right - self.left + 1) * u64::from(self.bottom - self.top + 1)
    }

    fn holds(&self, other: &Cells) -> bool {
        self.left <= other.left
            && self.top <= other.top
            && self.right >= other.right
            && self.bottom >= other.bottom
    }

    /// The one cell of `self` outside `smaller`, which `self` holds with one cell more.
    fn extra(&self, smaller: &Cells) -> Option<(u32, u32)> {
        let column = if self.left < smaller.left {
            self.left
        } else if self.right > smaller.right {
            self.right
        } else {
            self.left
        };
        let row = if self.top < smaller.top {
            self.top
        } else if self.bottom > smaller.bottom {
            self.bottom
        } else {
            self.top
        };
        (!smaller.holds(&Cells {
            left: column,
            top: row,
            right: column,
            bottom: row,
        }))
        .then_some((column, row))
    }
}

/// A column's number from its letters ("XFD" 16384).
fn column_number(letters: &str) -> Option<u32> {
    letters.chars().try_fold(0u32, |n, c| {
        let digit = u32::from(c.to_ascii_uppercase()).checked_sub(u32::from('A'))? + 1;
        (digit <= 26).then(|| n * 26 + digit)
    })
}

/// A column's letters from its number (16384 "XFD").
fn column_letters(mut number: u32) -> String {
    let mut letters = Vec::new();
    while number > 0 {
        let digit = (number - 1) % 26;
        letters.push(char::from(b'A' + u8::try_from(digit).unwrap_or(0)));
        number = (number - 1) / 26;
    }
    letters.iter().rev().collect()
}

/// One end of a reference: a cell ("B2"), a whole column ("C") or a whole row ("1"), as
/// (column, row), either `None` where the reference gives none.
fn reference_end(end: &str) -> Option<(Option<u32>, Option<u32>)> {
    let split = end.find(|c: char| c.is_ascii_digit()).unwrap_or(end.len());
    let (letters, digits) = (end.get(..split)?, end.get(split..)?);
    let column = (!letters.is_empty())
        .then(|| column_number(letters))
        .flatten();
    let row = (!digits.is_empty()).then(|| digits.parse().ok()).flatten();
    (column.is_some() || row.is_some()).then_some((column, row))
}

/// A selection's reference as its cells: "A1:B2", "B1", "1:1" (a whole row), "C:C" (a whole
/// column).
fn cells(reference: &str) -> Option<Cells> {
    let reference = reference.trim();
    let (first, last) = reference.split_once(':').unwrap_or((reference, reference));
    let (c1, r1) = reference_end(first)?;
    let (c2, r2) = reference_end(last)?;
    Some(Cells {
        left: c1.unwrap_or(1),
        top: r1.unwrap_or(1),
        right: c2.unwrap_or(LAST_COLUMN),
        bottom: r2.unwrap_or(LAST_ROW),
    })
}

/// A cell's coordinates as JAWS says them, its column's letters apart and from its row: "B 2",
/// "X F D 1" [obs, 1120/s7].
fn cell_coordinates(column: u32, row: u32) -> String {
    let letters: Vec<String> = column_letters(column).chars().map(String::from).collect();
    format!("{} {row}", letters.join(" "))
}

/// The text of the cell at `column` and `row` where the active cell's row or column holds
/// it; empty elsewhere.
fn known_text(place: &crate::event::SheetPlace, active: (u32, u32), cell: (u32, u32)) -> String {
    let index = |at: usize, from: u32, to: u32| {
        let offset = i64::from(to) - i64::from(from);
        usize::try_from(i64::try_from(at).ok()? + offset).ok()
    };
    let found = if cell.1 == active.1 {
        index(place.row_at, active.0, cell.0).and_then(|i| place.row.get(i))
    } else if cell.0 == active.0 {
        index(place.column_at, active.1, cell.1).and_then(|i| place.column.get(i))
    } else {
        None
    };
    found.map(|t| t.trim().to_owned()).unwrap_or_default()
}

/// What a selecting key changed, as JAWS says it [obs, 1120, 1121, 1117]:
/// - one cell more, "select" and the cell, "select | 8 | C 2", a blank one by its coordinates
///   alone, "select | D 2"; one cell less, "unselect | Weekends | C 1";
/// - any other change, "select" and the selection's corners with their values, "select | B 2 |
///   12 | D 3 | blank";
/// - Shift+Space and Ctrl+Space first say what they selected, "selected entire row", "Selected
///   entire column"; the whole sheet selected, the active cell after it, "Selected entire
///   column | Branch | A 1".
///
/// `before` is the selection before the key, the active cell's alone where none was read.
pub(crate) fn selection_said(
    before: Option<&str>,
    place: &crate::event::SheetPlace,
    key: SelectionKey,
) -> Option<String> {
    let now = cells(&place.selection)?;
    let active = cells(&place.address)?;
    let was = before.and_then(cells).unwrap_or(active);
    let blank = msg::blank().to_string();
    let shown = |text: &str| {
        if text.trim().is_empty() {
            blank.clone()
        } else {
            text.trim().to_owned()
        }
    };
    let mut lines = Vec::new();
    match key {
        SelectionKey::Rows => lines.push(msg::excel_selected_row().to_string()),
        SelectionKey::Columns => lines.push(msg::excel_selected_column().to_string()),
        SelectionKey::Extend => {}
    }
    let whole_sheet =
        now.left == 1 && now.top == 1 && now.right == LAST_COLUMN && now.bottom == LAST_ROW;
    if whole_sheet {
        let value = known_text(place, (active.left, active.top), (active.left, active.top));
        if !value.is_empty() {
            lines.push(value);
        }
        lines.push(cell_coordinates(active.left, active.top));
        return Some(lines.join("\n"));
    }
    let single = |cell: (u32, u32), word: String| {
        let value = known_text(place, (active.left, active.top), cell);
        let mut said = vec![word];
        if !value.is_empty() {
            said.push(value);
        }
        said.push(cell_coordinates(cell.0, cell.1));
        said
    };
    if now.holds(&was) && now.count() == was.count() + 1 {
        if let Some(cell) = now.extra(&was) {
            lines.extend(single(cell, msg::excel_select().to_string()));
            return Some(lines.join("\n"));
        }
    }
    if was.holds(&now) && was.count() == now.count() + 1 {
        if let Some(cell) = was.extra(&now) {
            lines.extend(single(cell, msg::excel_unselect().to_string()));
            return Some(lines.join("\n"));
        }
    }
    lines.extend([
        msg::excel_select().to_string(),
        cell_coordinates(now.left, now.top),
        shown(&place.selection_first),
        cell_coordinates(now.right, now.bottom),
        shown(&place.selection_last),
    ]);
    Some(lines.join("\n"))
}

/// Excel's grid itself focused, as an empty cell leaves it (no cell element to focus).
fn grid(focus: &FocusEvent) -> bool {
    focus.platform.window_class.as_deref() == Some(GRID_WINDOW) && focus.role != Role::TableCell
}

/// Excel's sheet focused: one of its cells, or the grid where the cell is empty.
pub(crate) fn in_sheet(focus: &FocusEvent) -> bool {
    cell(focus).is_some() || grid(focus)
}

/// Excel's cell editor focused (F2).
fn editor(focus: &FocusEvent) -> bool {
    focus.platform.window_class.as_deref() == Some(EDITOR_WINDOW) && focus.role == Role::Edit
}

/// The active cell read from Excel's object model, where the grid had no cell to focus: an
/// empty one as JAWS says it, "blank | D 1 | Active Cell: D1" [obs, 1118/s3, s4]; one with a
/// value as any cell. `None` without its reference.
pub(crate) fn blank_or_cell(place: &crate::event::SheetPlace) -> Option<String> {
    let at = coordinates(&place.address)?;
    let value = place
        .row
        .get(place.row_at)
        .map(|v| v.trim())
        .unwrap_or_default();
    Some(if value.is_empty() {
        format!(
            "{}\n{at}\n{}",
            msg::blank(),
            msg::excel_active_cell(place.address.trim()).render()
        )
    } else {
        format!("{value}\n{at}")
    })
}

/// The coordinates of the cell `focus` is, as JAWS says them, its column's letters apart from
/// its row's number: "B 2"; `None` for any other focus.
pub(crate) fn cell(focus: &FocusEvent) -> Option<String> {
    if focus.role != Role::TableCell || focus.platform.window_class.as_deref() != Some(GRID_WINDOW)
    {
        return None;
    }
    coordinates(focus.name.as_deref()?)
}

/// A cell reference ("B2", "AA10") as JAWS says it, "B 2"; `None` for another name.
fn coordinates(reference: &str) -> Option<String> {
    let reference = reference.trim();
    let split = reference.find(|c: char| c.is_ascii_digit())?;
    let (column, row) = (reference.get(..split)?, reference.get(split..)?);
    let column_ok = !column.is_empty() && column.chars().all(|c| c.is_ascii_uppercase());
    let row_ok = !row.is_empty() && row.chars().all(|c| c.is_ascii_digit());
    (column_ok && row_ok).then(|| format!("{column} {row}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The row and column readers from B3 of 1114's sheet [obs, 1114/s4-s9].
    #[test]
    fn rows_and_columns_are_read_whole_or_from_the_cell() {
        let place = crate::event::SheetPlace {
            address: "B3".into(),
            selection: "B3".into(),
            selection_first: "10".into(),
            selection_last: "10".into(),
            row: vec!["Northside".into(), "10".into(), "6".into()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            row_at: 1,
            column_at: 2,
        };
        let said = |reading: SheetReading| reading.said(&place);
        assert_eq!(
            said(SheetReading::Line).as_deref(),
            Some("Northside\n10\n6")
        );
        assert_eq!(said(SheetReading::Row).as_deref(), Some("Northside 10 6"));
        assert_eq!(
            said(SheetReading::RowFromStart).as_deref(),
            Some("Northside 10")
        );
        assert_eq!(said(SheetReading::RowToEnd).as_deref(), Some("10 6"));
        assert_eq!(
            said(SheetReading::Column).as_deref(),
            Some("Weekdays\n12\n10\n22")
        );
        assert_eq!(
            said(SheetReading::ColumnFromTop).as_deref(),
            Some("Weekdays\n12\n10")
        );
        assert_eq!(
            said(SheetReading::ColumnToBottom).as_deref(),
            Some("10\n22")
        );
    }

    /// JAWS's words for selections [obs, 1120/s1-s9, 1121/s3-s6].
    #[test]
    fn selections_are_said_by_what_changed() {
        let place = |selection: &str, first: &str, last: &str| crate::event::SheetPlace {
            address: "B2".into(),
            selection: selection.into(),
            selection_first: first.into(),
            selection_last: last.into(),
            row: vec!["Central".into(), "12".into(), "8".into(), String::new()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            row_at: 1,
            column_at: 1,
        };
        let said = |before: Option<&str>, now: crate::event::SheetPlace, key| {
            selection_said(before, &now, key)
        };
        assert_eq!(
            said(None, place("B2:C2", "12", "8"), SelectionKey::Extend).as_deref(),
            Some("select\n8\nC 2")
        );
        assert_eq!(
            said(
                Some("B2:C2"),
                place("B2:D2", "12", ""),
                SelectionKey::Extend
            )
            .as_deref(),
            Some("select\nD 2")
        );
        assert_eq!(
            said(
                Some("B2:D2"),
                place("B2:D3", "12", ""),
                SelectionKey::Extend
            )
            .as_deref(),
            Some("select\nB 2\n12\nD 3\nblank")
        );
        assert_eq!(
            said(
                Some("B2:D2"),
                place("B2:C2", "12", "8"),
                SelectionKey::Extend
            )
            .as_deref(),
            Some("unselect\nD 2")
        );
        assert_eq!(
            said(None, place("2:2", "Central", ""), SelectionKey::Rows).as_deref(),
            Some("selected entire row\nselect\nA 2\nCentral\nX F D 2\nblank")
        );
        assert_eq!(column_letters(16_384), "XFD");
        assert_eq!(column_number("XFD"), Some(16_384));
    }

    #[test]
    fn a_cell_reference_is_said_column_apart_from_row() {
        assert_eq!(coordinates("B2").as_deref(), Some("B 2"));
        assert_eq!(coordinates("AA10").as_deref(), Some("AA 10"));
        assert_eq!(coordinates("Grid"), None);
        assert_eq!(coordinates("2B"), None);
        assert_eq!(coordinates("B"), None);
    }

    /// "12 | B 2" for the cell B2 holding 12 [obs, 521/s3]; nothing of another window's.
    #[test]
    fn a_cell_is_its_value_then_its_coordinates() {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.name = Some("B2".into());
        focus.value = Some("12".into());
        focus.platform.window_class = Some(GRID_WINDOW.into());
        assert_eq!(cell(&focus).as_deref(), Some("B 2"));
        let mut other = focus.clone();
        other.platform.window_class = Some("Chrome_RenderWidgetHostHWND".into());
        assert_eq!(cell(&other), None);
    }
}
