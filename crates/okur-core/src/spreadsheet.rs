//! A spreadsheet's place read as JAWS reads Excel's: a cell's coordinates ("B 2"), the row and
//! column readers, selections said by what changed, a sheet's titles and its totals, and a
//! cell's format. The platform gives the place (`SheetPlace`); this module words it. Pure
//! functions of the neutral sheet types, for any application with a sheet
//! (crate-structure-proposal.md §13.2, step 5a).

use crate::input::Key;
use crate::messages::msg;

/// A workbook's titles, as Insert+Ctrl+Alt+C and R set them: the row holding the column titles
/// and the column holding the row titles, each from 1 [obs, 1132/s1, s2].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SheetTitles {
    pub row: Option<u32>,
    pub column: Option<u32>,
}

/// Excel's title keys (Excel.jkm): Insert+Ctrl+Alt+C and R set the titles
/// (setColTitlesRange, setRowTitlesRange), the Okur key with Alt+Shift and C or R says them
/// (sayColumnTitle, sayRowTitle).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TitleKey {
    SetColumnTitles,
    SetRowTitles,
    SayColumnTitle,
    SayRowTitle,
}

impl TitleKey {
    /// The title key a chord with the Okur key is, if any.
    pub(crate) fn of(key: Key, m: crate::input::Modifiers) -> Option<Self> {
        if !m.insert || m.win {
            return None;
        }
        match (key, m.ctrl, m.alt, m.shift) {
            (Key::Alpha('C'), true, true, false) => Some(Self::SetColumnTitles),
            (Key::Alpha('R'), true, true, false) => Some(Self::SetRowTitles),
            (Key::Alpha('C'), false, true, true) => Some(Self::SayColumnTitle),
            (Key::Alpha('R'), false, true, true) => Some(Self::SayRowTitle),
            _ => None,
        }
    }

    /// Sets `titles` from the cell at (`column`, `row`), and what that says: "Setting column
    /// titles row range to 1", "Setting row titles column range to A" [obs, 1132/s1, s2];
    /// `None` for a key that says a title.
    pub(crate) fn set(self, titles: &mut SheetTitles, (column, row): (u32, u32)) -> Option<String> {
        match self {
            Self::SetColumnTitles => {
                titles.row = Some(row);
                Some(msg::excel_column_titles_set(row.to_string()).render())
            }
            Self::SetRowTitles => {
                titles.column = Some(column);
                Some(msg::excel_row_titles_set(column_letters(column)).render())
            }
            Self::SayColumnTitle | Self::SayRowTitle => None,
        }
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
    /// Insert+C where the grid has the focus, an empty cell: the active cell's coordinates,
    /// "D 1" [obs, 1123/s3].
    Cell,
    /// Say Column Title and Say Row Title: the active cell's column's title, in the row given,
    /// "Weekends"; its row's, in the column given, "Northside" [obs, 1132/s7, s8].
    ColumnTitle(u32),
    RowTitle(u32),
    /// Insert+F: the active cell's font and formatting ([`format_said`]) [obs, 1133/s4].
    Format,
    /// The totals readers, Insert+NumPad Enter and Insert+Delete: the column's or the row's SUM
    /// formula's value, "column total=14" [obs, 1132/s11, s12].
    ColumnTotal,
    RowTotal,
    /// A monitor cell read (Alt+Shift+1 to 0): its column title where the titles are set, its
    /// cells' texts, then its reference, "12 | B2" [obs, 1135/s5], "Weekdays | 12 | B2", "12 |
    /// 10 | B2:B3", an empty one's reference alone, "D1" [obs, 1166].
    Monitor(crate::actions::CellRange),
}

/// What one of Excel's places was asked for: a cell to say (its grid focused without a cell, a
/// cell waiting for its titles, the selection a selecting key changed) or a reader. Excel's
/// worker answers each ask in turn, and the engine keeps them in order: two quick moves had
/// said the second cell with the first one's place, its titles the column before's
/// (performance review 2026-11-02, engine finding 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SheetAsk {
    Cell,
    Reading(SheetReading),
    /// A monitor cell's text for the Move to Monitor Cell list; the list opens with the last.
    MonitorEntry {
        range: crate::actions::CellRange,
        last: bool,
    },
    /// The worksheet's name, for Move to Monitor Cell with none set: "No monitor cells defined
    /// for Hours" [obs, 1166/s2].
    NoMonitorCells,
}

impl SheetReading {
    /// The cells the reading needs read around the active cell.
    pub(crate) fn cells(self) -> crate::actions::SheetCells {
        use crate::actions::SheetCells;
        match self {
            Self::Line
            | Self::Row
            | Self::RowFromStart
            | Self::RowToEnd
            | Self::Column
            | Self::ColumnFromTop
            | Self::ColumnToBottom => SheetCells::Lines,
            Self::Cell => SheetCells::Around(Some((None, None))),
            Self::ColumnTitle(row) => SheetCells::Around(Some((Some(row), None))),
            Self::RowTitle(column) => SheetCells::Around(Some((None, Some(column)))),
            Self::Format => SheetCells::Format,
            Self::ColumnTotal | Self::RowTotal => SheetCells::Sums,
            Self::Monitor(range) => SheetCells::Range(range),
        }
    }

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
        match self {
            Self::Cell => return coordinates(&place.address),
            Self::ColumnTitle(row) => return titles_of(place, Some(row), None).pop(),
            Self::RowTitle(column) => return titles_of(place, None, Some(column)).pop(),
            Self::Format => return place.format.as_ref().map(format_said),
            Self::ColumnTotal => {
                return Some(match place.column_sum.as_deref() {
                    Some(total) => msg::excel_column_total(total).render(),
                    None => msg::excel_no_column_total().to_string(),
                });
            }
            Self::RowTotal => {
                return Some(match place.row_sum.as_deref() {
                    Some(total) => msg::excel_row_total(total).render(),
                    None => msg::excel_no_row_total().to_string(),
                });
            }
            Self::Monitor(range) => return Some(monitor_said(place, range)),
            _ => {}
        }
        let (cells, at, row) = match self {
            Self::Line => (&place.row, place.row_at, false),
            Self::Row | Self::RowFromStart | Self::RowToEnd => (&place.row, place.row_at, true),
            _ => (&place.column, place.column_at, false),
        };
        let part: &[String] = match self {
            Self::Line | Self::Row | Self::Column => cells,
            Self::RowFromStart | Self::ColumnFromTop => cells.get(..=at).unwrap_or(cells),
            Self::RowToEnd | Self::ColumnToBottom => cells.get(at..).unwrap_or_default(),
            Self::Cell
            | Self::ColumnTitle(_)
            | Self::RowTitle(_)
            | Self::Format
            | Self::ColumnTotal
            | Self::RowTotal
            | Self::Monitor(_) => return None,
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
/// The worksheet in front as its monitor cells are kept: the workbook's window, and the sheet
/// Excel last named with a place it read (JAWS keeps ten per worksheet, popapps.chm, Monitor
/// Cells in Excel).
pub(crate) fn monitor_sheet(focus: &crate::event::FocusEvent, sheet: &str) -> Option<String> {
    Some(format!("{}\u{1f}{sheet}", focus.window_title.as_deref()?))
}

/// A monitor cell's place among a worksheet's ten: Insert+Shift+1 sets the first, 0 the tenth.
pub(crate) fn monitor_index(digit: u8) -> usize {
    usize::from(digit).checked_sub(1).unwrap_or(9)
}

/// A monitor cell's number as JAWS says it, 1 to 10.
pub(crate) fn monitor_number(digit: u8) -> u8 {
    if digit == 0 {
        10
    } else {
        digit
    }
}

/// What a monitor cell read says of `place`: the titles' texts, the cells' texts, one a line,
/// and the range's reference [obs, 1166/s10, s16, s19].
pub(crate) fn monitor_said(
    place: &crate::event::SheetPlace,
    range: crate::actions::CellRange,
) -> String {
    let mut lines: Vec<String> = range
        .titles()
        .into_iter()
        .filter_map(|cell| cell_text(place, cell))
        .collect();
    lines.extend(
        range
            .cells()
            .into_iter()
            .filter_map(|cell| cell_text(place, cell)),
    );
    lines.push(range_reference(&range));
    lines.join("\n")
}

/// The text `place` read of a cell, where it shows one.
fn cell_text(place: &crate::event::SheetPlace, (column, row): (u32, u32)) -> Option<String> {
    place
        .cells
        .iter()
        .find(|c| (c.column, c.row) == (column, row))
        .map(|c| c.text.trim().to_owned())
        .filter(|t| !t.is_empty())
}

/// The texts of a monitor cell's cells, for its entry in Move to Monitor Cell.
pub(crate) fn range_texts(
    place: &crate::event::SheetPlace,
    range: &crate::actions::CellRange,
) -> String {
    range
        .cells()
        .into_iter()
        .filter_map(|cell| cell_text(place, cell))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A range's reference as Excel writes it, "B2", "B2:B3".
pub fn range_reference(range: &crate::actions::CellRange) -> String {
    let first = reference(range.left, range.top);
    if (range.left, range.top) == (range.right, range.bottom) {
        first
    } else {
        format!("{first}:{}", reference(range.right, range.bottom))
    }
}

/// The range a reference names, "B2" or "B2:B3", without titles.
pub(crate) fn range_of(reference: &str) -> Option<crate::actions::CellRange> {
    let cells = cells(reference)?;
    Some(crate::actions::CellRange {
        left: cells.left,
        top: cells.top,
        right: cells.right,
        bottom: cells.bottom,
        title_row: None,
        title_column: None,
    })
}

/// A cell's reference as Excel writes it, "B2".
pub fn reference(column: u32, row: u32) -> String {
    format!("{}{row}", column_letters(column))
}

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
pub(crate) fn reference_end(end: &str) -> Option<(Option<u32>, Option<u32>)> {
    let split = end.find(|c: char| c.is_ascii_digit()).unwrap_or(end.len());
    let (letters, digits) = (end.get(..split)?, end.get(split..)?);
    let column = (!letters.is_empty())
        .then(|| column_number(letters))
        .flatten();
    let row = (!digits.is_empty()).then(|| digits.parse().ok()).flatten();
    (column.is_some() || row.is_some()).then_some((column, row))
}

/// What Insert+F says of the active cell's font and formatting, line by line: "Cell Font and
/// Formatting: | text color Black | background color White | Calibri | Regular | 11 point |
/// highlighted | Horizontal alignment is General. | Vertical alignment is Bottom. | Number
/// format is general. | Normal | Cell dimensions are 48 points wide by 14 points high." [obs,
/// 1133/s4]. A cell without fill is white, as Excel shows it.
pub(crate) fn format_said(format: &crate::event::SheetFormat) -> String {
    use crate::color::{name_of, Rgb};
    // The color table's name, capitalized as JAWS says it here, "Black" [obs, 1133/s4].
    let color = |rgb: Rgb| {
        let name = name_of(rgb).unwrap_or_default();
        let mut chars = name.chars();
        chars
            .next()
            .map(|first| first.to_uppercase().chain(chars).collect::<String>())
            .unwrap_or_default()
    };
    // A size in points: whole, or with its fraction ("10.5").
    let points = |hundredths: u32| {
        if hundredths.is_multiple_of(100) {
            (hundredths / 100).to_string()
        } else {
            (f64::from(hundredths) / 100.0).to_string()
        }
    };
    let mut lines = vec![
        msg::excel_cell_format().to_string(),
        msg::excel_text_color(color(format.color.unwrap_or(Rgb::new(0, 0, 0)))).render(),
        msg::excel_background_color(color(format.fill.unwrap_or(Rgb::new(255, 255, 255)))).render(),
    ];
    lines.extend(
        [&format.font, &format.style]
            .into_iter()
            .filter(|text| !text.is_empty())
            .cloned(),
    );
    lines.push(msg::font_size(points(format.size)).render());
    lines.push(msg::excel_highlighted().to_string());
    if let Some(alignment) = horizontal_alignment(format.horizontal) {
        lines.push(msg::excel_horizontal_alignment(alignment).render());
    }
    if let Some(alignment) = vertical_alignment(format.vertical) {
        lines.push(msg::excel_vertical_alignment(alignment).render());
    }
    if !format.number_format.is_empty() {
        lines.push(msg::excel_number_format(format.number_format.to_lowercase()).render());
    }
    if !format.cell_style.is_empty() {
        lines.push(format.cell_style.clone());
    }
    lines.push(
        msg::excel_cell_dimensions(
            (format.width / 100).to_string(),
            (format.height / 100).to_string(),
        )
        .render(),
    );
    lines.join("\n")
}

/// Excel's horizontal alignment (`XlHAlign`) by its name.
fn horizontal_alignment(value: i32) -> Option<String> {
    Some(
        match value {
            1 => msg::excel_align_general(),
            -4131 => msg::excel_align_left(),
            -4108 => msg::excel_align_center(),
            -4152 => msg::excel_align_right(),
            5 => msg::excel_align_fill(),
            -4130 => msg::excel_align_justify(),
            7 => msg::excel_align_center_across(),
            -4117 => msg::excel_align_distributed(),
            _ => return None,
        }
        .to_string(),
    )
}

/// Excel's vertical alignment (`XlVAlign`) by its name.
fn vertical_alignment(value: i32) -> Option<String> {
    Some(
        match value {
            -4160 => msg::excel_align_top(),
            -4108 => msg::excel_align_center(),
            -4107 => msg::excel_align_bottom(),
            -4130 => msg::excel_align_justify(),
            -4117 => msg::excel_align_distributed(),
            _ => return None,
        }
        .to_string(),
    )
}

/// The titles the workbook's names give the place's sheet
/// ([`crate::event::SheetPlace::title_row`]).
pub(crate) fn named_titles(place: &crate::event::SheetPlace) -> SheetTitles {
    SheetTitles {
        row: place.title_row,
        column: place.title_column,
    }
}

/// The titles of the active cell of `place`: its column's, in `titles_row`, then its row's, in
/// `titles_column`, each where the active cell's row or column holds it and not empty.
pub(crate) fn titles_of(
    place: &crate::event::SheetPlace,
    titles_row: Option<u32>,
    titles_column: Option<u32>,
) -> Vec<String> {
    let Some((Some(column), Some(row))) = reference_end(place.address.trim()) else {
        return Vec::new();
    };
    let column_title = titles_row.map(|r| known_text(place, (column, row), (column, r)));
    let row_title = titles_column.map(|c| known_text(place, (column, row), (c, row)));
    column_title
        .into_iter()
        .chain(row_title)
        .filter(|title| !title.is_empty())
        .collect()
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
    // Else among the cells read around the active cell.
    found
        .or_else(|| {
            place
                .cells
                .iter()
                .find(|c| (c.column, c.row) == cell)
                .map(|c| &c.text)
        })
        .map(|t| t.trim().to_owned())
        .unwrap_or_default()
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

/// A cell reference ("B2", "AA10") as JAWS says it, "B 2"; `None` for another name.
pub(crate) fn coordinates(reference: &str) -> Option<String> {
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
            sheet: String::new(),
            selection: "B3".into(),
            selection_first: "10".into(),
            selection_last: "10".into(),
            row: vec!["Northside".into(), "10".into(), "6".into()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            row_at: 1,
            column_at: 2,
            title_row: None,
            title_column: None,
            column_sum: None,
            row_sum: None,
            format: None,
            cells: Vec::new(),
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
            sheet: String::new(),
            selection: selection.into(),
            selection_first: first.into(),
            selection_last: last.into(),
            row: vec!["Central".into(), "12".into(), "8".into(), String::new()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            row_at: 1,
            column_at: 1,
            title_row: None,
            title_column: None,
            column_sum: None,
            row_sum: None,
            format: None,
            cells: Vec::new(),
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

    /// Insert+F on the fixture's B1: Calibri 11, no fill, General and Bottom [obs, 1133/s4].
    #[test]
    fn insert_f_says_the_cells_font_and_formatting() {
        let format = crate::event::SheetFormat {
            font: "Calibri".into(),
            style: "Regular".into(),
            size: 1100,
            color: Some(crate::color::Rgb::new(0, 0, 0)),
            fill: None,
            horizontal: 1,
            vertical: -4107,
            number_format: "General".into(),
            cell_style: "Normal".into(),
            width: 4800,
            height: 1440,
        };
        assert_eq!(
            format_said(&format),
            "Cell Font and Formatting:
text color Black
background color White
Calibri
Regular
11 point
highlighted
Horizontal alignment is General.
Vertical alignment is Bottom.
Number format is general.
Normal
Cell dimensions are 48 points wide by 14 points high."
        );
    }

    /// The totals readers on C3: the column's SUM formula's value, "column total=14"; no SUM in the
    /// row [obs, 1132/s11, s12].
    #[test]
    fn the_totals_come_from_sum_formulas() {
        let place = crate::event::SheetPlace {
            address: "C3".into(),
            sheet: String::new(),
            column_sum: Some("14".into()),
            ..Default::default()
        };
        assert_eq!(
            SheetReading::ColumnTotal.said(&place).as_deref(),
            Some("column total=14")
        );
        assert_eq!(
            SheetReading::RowTotal.said(&place).as_deref(),
            Some("no totals column defined and I cannot find a sum formula in the current row.")
        );
    }
}
