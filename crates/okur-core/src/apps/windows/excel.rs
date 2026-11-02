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
/// Excel's main window, which reports the focus back on the cell as a dialog of Excel's closes
/// [obs, 1124/s7, s9].
const MAIN_WINDOW: &str = "XLMAIN";

/// Excel's module.
#[derive(Debug, Default)]
pub struct Excel {
    /// The last cell reached was empty, which a Ctrl arrow leaves for a new data region.
    from_blank: bool,
    /// The last cell said, (column, row): the next one says the title of a column or a row it
    /// enters.
    last_cell: Option<(u32, u32)>,
    /// A cell said once its titles come with the sheet's place.
    pending: Option<PendingTitles>,
}

/// A cell's speech waiting for the titles of the column or the row it entered: the row that
/// holds the column titles, the column that holds the row titles.
#[derive(Debug)]
struct PendingTitles {
    lines: Vec<String>,
    /// The cell, and the one before it.
    at: (u32, u32),
    before: (u32, u32),
    /// The workbook's titles where known; else the place's, from the workbook's names.
    titles: Option<SheetTitles>,
}

/// The row of column titles and the column of row titles a move from `before` to `at` says:
/// into another column, the column's title; into another row, the row's; not the title of a
/// title cell itself, "Central | A 2" [obs, 1132/s3-s6, s9-s13].
fn titles_entered(
    titles: SheetTitles,
    (column, row): (u32, u32),
    (was_column, was_row): (u32, u32),
) -> (Option<u32>, Option<u32>) {
    (
        titles.row.filter(|&r| column != was_column && row != r),
        titles.column.filter(|&c| row != was_row && column != c),
    )
}

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

impl AppModule for Excel {
    fn owns(&self, focus: &FocusEvent) -> bool {
        cell(focus).is_some() || grid(focus) || editor(focus)
    }

    /// The sheet's place for a cell waiting for its titles: the column's title, the row's,
    /// then the cell, "Weekdays | 12 | B 2" [obs, 1132/s4, s5]; the cell alone without the
    /// place.
    fn sheet_place(&mut self, place: Option<&crate::event::SheetPlace>) -> Option<String> {
        let pending = self.pending.take()?;
        let mut lines: Vec<String> = place
            .map(|place| {
                let titles = pending.titles.unwrap_or_else(|| named_titles(place));
                let (row, column) = titles_entered(titles, pending.at, pending.before);
                titles_of(place, row, column)
            })
            .unwrap_or_default();
        lines.extend(pending.lines);
        Some(lines.join("\n"))
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
            self.pending = None;
            return FocusResponse::Silent;
        }
        let Some(coordinates) = cell(focus) else {
            return FocusResponse::Pass;
        };
        // The cell reported again with no key between: Go To's Enter gives it back in four
        // reports, said once, "6 | C 3" [obs, 1124/s3].
        if cx.repeat {
            cx.note_window(focus);
            *cx.app_mode = crate::engine::AppMode::NativeControl;
            return FocusResponse::Silent;
        }
        // Another cell: one still waiting for its titles has gone by, and the place answering
        // for it is not this one's (said after it, it had repeated the cell left).
        self.pending = None;
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
        // A move's titles ([`titles_entered`]): where the workbook's are not known yet, its
        // names may give them [obs, 1134], and the cell waits for the place to tell.
        let at = cell_at(focus);
        let before = std::mem::replace(&mut self.last_cell, at);
        let moved = at.zip(before);
        let waits = moved.is_some_and(|(at, before)| match cx.sheet_titles {
            Some(titles) => {
                let (row, column) = titles_entered(titles, at, before);
                row.is_some() || column.is_some()
            }
            None => true,
        });
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
            // Undo said before the cell it gives back, "Undo | 7 | D 1" [obs, 1123/s8].
            Some(Key::Alpha('Z')) => lines.push(msg::edit_undo().to_string()),
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
        if let Some((at, before)) = moved.filter(|_| waits) {
            self.pending = Some(PendingTitles {
                lines,
                at,
                before,
                titles: cx.sheet_titles,
            });
            cx.ask_sheet = true;
            return FocusResponse::Silent;
        }
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
            | Self::RowTotal => return None,
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

/// The cell `focus` is, as (column, row) from 1.
pub(crate) fn cell_at(focus: &FocusEvent) -> Option<(u32, u32)> {
    cell(focus)?;
    let (column, row) = reference_end(focus.name.as_deref()?.trim())?;
    Some((column?, row?))
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
fn titles_of(
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
/// empty one as JAWS says it, "blank | D 1 | Active Cell: D1" [obs, 1118/s3, s4], or without
/// naming the active cell where Delete emptied it, "blank | D 1" [obs, 1123/s7]; one with a
/// value as any cell. `None` without its reference.
pub(crate) fn blank_or_cell(
    place: &crate::event::SheetPlace,
    names_active_cell: bool,
) -> Option<String> {
    let at = coordinates(&place.address)?;
    let value = place
        .row
        .get(place.row_at)
        .map(|v| v.trim())
        .unwrap_or_default();
    Some(if value.is_empty() && names_active_cell {
        format!(
            "{}\n{at}\n{}",
            msg::blank(),
            msg::excel_active_cell(place.address.trim()).render()
        )
    } else if value.is_empty() {
        format!("{}\n{at}", msg::blank())
    } else {
        format!("{value}\n{at}")
    })
}

/// The coordinates of the cell `focus` is, as JAWS says them, its column's letters apart from
/// its row's number: "B 2"; `None` for any other focus.
pub(crate) fn cell(focus: &FocusEvent) -> Option<String> {
    let window = focus.platform.window_class.as_deref();
    if focus.role != Role::TableCell || !matches!(window, Some(GRID_WINDOW | MAIN_WINDOW)) {
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
        // Excel's main window reports the cell as a dialog of Excel's closes [obs, 1124/s7].
        let mut back = focus.clone();
        back.platform.window_class = Some(MAIN_WINDOW.into());
        assert_eq!(cell(&back).as_deref(), Some("B 2"));
    }

    /// What the module says of `focus` after `key`, `repeat` as the engine tells it.
    fn said(
        excel: &mut Excel,
        focus: &FocusEvent,
        key: Option<(Key, crate::input::Modifiers)>,
        repeat: bool,
    ) -> Option<String> {
        let mut mode = crate::engine::AppMode::NativeControl;
        let (mut title, mut kind) = (None, None);
        let mut cx = AppContext {
            app_mode: &mut mode,
            window_title: &mut title,
            window_kind: &mut kind,
            selecting: false,
            alt_held: false,
            letter_key: false,
            escape_key: false,
            key,
            ask_view: false,
            ask_sheet: false,
            repeat,
            office_menu_closed: false,
            sheet_titles: Some(SheetTitles::default()),
        };
        match excel.focus(focus, &mut cx) {
            FocusResponse::Speak(announcement) => Some(announcement.render()),
            FocusResponse::Silent => Some(String::new()),
            _ => None,
        }
    }

    /// Ctrl+Z says "Undo" before the cell it gives back, "Undo | 7 | D 1" [obs, 1123/s8]; the
    /// cell reported again with no key between is said once [obs, 1124/s3].
    #[test]
    fn undo_comes_before_the_cell_and_a_repeat_is_silent() {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.name = Some("D1".into());
        focus.value = Some("7".into());
        focus.platform.window_class = Some(GRID_WINDOW.into());
        let ctrl = crate::input::Modifiers {
            ctrl: true,
            ..crate::input::Modifiers::none()
        };
        let mut excel = Excel::default();
        let undo = Some((Key::Alpha('Z'), ctrl));
        assert_eq!(
            said(&mut excel, &focus, undo, false).as_deref(),
            Some("Undo\n7\nD 1")
        );
        assert_eq!(said(&mut excel, &focus, undo, true).as_deref(), Some(""));
    }

    /// An empty cell from Excel's object model: "blank | D 1 | Active Cell: D1" reached by a
    /// key that moves [obs, 1118/s3], "blank | D 1" emptied by Delete [obs, 1123/s7].
    #[test]
    fn an_empty_cell_names_the_active_cell_unless_deleted() {
        let place = crate::event::SheetPlace {
            address: "D1".into(),
            selection: "D1".into(),
            selection_first: String::new(),
            selection_last: String::new(),
            row_at: 3,
            column_at: 0,
            row: vec![
                "Branch".into(),
                "Weekdays".into(),
                "Weekends".into(),
                String::new(),
            ],
            column: vec![String::new()],
            title_row: None,
            title_column: None,
            column_sum: None,
            row_sum: None,
            format: None,
            cells: Vec::new(),
        };
        assert_eq!(
            blank_or_cell(&place, true).as_deref(),
            Some("blank\nD 1\nActive Cell: D1")
        );
        assert_eq!(blank_or_cell(&place, false).as_deref(), Some("blank\nD 1"));
        assert_eq!(SheetReading::Cell.said(&place).as_deref(), Some("D 1"));
    }

    /// Row 1 as the column titles and column A as the row titles, from A1: a cell entering a
    /// column says its title first, one entering a row the row's, waiting for the sheet's place;
    /// a title cell says no title of its own [obs, 1132/s1-s10].
    #[test]
    fn titles_come_before_a_cell_entering_their_column_or_row() {
        let mut titles = SheetTitles::default();
        assert_eq!(
            TitleKey::SetColumnTitles
                .set(&mut titles, (1, 1))
                .as_deref(),
            Some("Setting column titles row range to 1")
        );
        assert_eq!(
            TitleKey::SetRowTitles.set(&mut titles, (1, 1)).as_deref(),
            Some("Setting row titles column range to A")
        );
        let cell = |name: &str, value: &str| {
            let mut focus = FocusEvent::new(Role::TableCell);
            focus.name = Some(name.into());
            focus.value = Some(value.into());
            focus.platform.window_class = Some(GRID_WINDOW.into());
            focus
        };
        // What the module says of `focus`, and whether it asked for the place.
        let step = |excel: &mut Excel, focus: &FocusEvent| {
            let mut mode = crate::engine::AppMode::NativeControl;
            let (mut title, mut kind) = (None, None);
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: false,
                escape_key: false,
                key: None,
                ask_view: false,
                ask_sheet: false,
                repeat: false,
                office_menu_closed: false,
                sheet_titles: Some(titles),
            };
            let said = match excel.focus(focus, &mut cx) {
                FocusResponse::Speak(announcement) => announcement.render(),
                _ => String::new(),
            };
            (said, cx.ask_sheet)
        };
        let mut excel = Excel::default();
        assert!(!step(&mut excel, &cell("A1", "Branch")).1);
        // A2 is in the row titles' column: no title of its own.
        assert_eq!(
            step(&mut excel, &cell("A2", "Central")),
            ("Central\nA 2".to_owned(), false)
        );
        // Into column B: its title comes with the place.
        assert_eq!(step(&mut excel, &cell("B2", "12")), (String::new(), true));
        let place = crate::event::SheetPlace {
            address: "B2".into(),
            selection: "B2".into(),
            selection_first: String::new(),
            selection_last: String::new(),
            row_at: 1,
            column_at: 1,
            row: vec!["Central".into(), "12".into(), "8".into()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            title_row: None,
            title_column: None,
            column_sum: None,
            row_sum: None,
            format: None,
            cells: Vec::new(),
        };
        assert_eq!(
            excel.sheet_place(Some(&place)).as_deref(),
            Some("Weekdays\n12\nB 2")
        );
        assert_eq!(
            SheetReading::ColumnTitle(1).said(&place).as_deref(),
            Some("Weekdays")
        );
        assert_eq!(
            SheetReading::RowTitle(1).said(&place).as_deref(),
            Some("Central")
        );
        // Without the place, the cell alone.
        assert_eq!(step(&mut excel, &cell("B3", "10")), (String::new(), true));
        assert_eq!(excel.sheet_place(None).as_deref(), Some("10\nB 3"));
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

    /// A workbook whose name "Title" sits on A1: its titles, not known before, come with the
    /// first move's place, "Weekdays | 12 | B 2" [obs, 1134/s2].
    #[test]
    fn titles_come_from_the_workbooks_names() {
        let mut focus = FocusEvent::new(Role::TableCell);
        focus.platform.window_class = Some(GRID_WINDOW.into());
        let mut excel = Excel::default();
        let mut step = |name: &str, value: &str| {
            focus.name = Some(name.into());
            focus.value = Some(value.into());
            let mut mode = crate::engine::AppMode::NativeControl;
            let (mut title, mut kind) = (None, None);
            let mut cx = AppContext {
                app_mode: &mut mode,
                window_title: &mut title,
                window_kind: &mut kind,
                selecting: false,
                alt_held: false,
                letter_key: false,
                escape_key: false,
                key: None,
                ask_view: false,
                ask_sheet: false,
                repeat: false,
                office_menu_closed: false,
                sheet_titles: None,
            };
            let said = match excel.focus(&focus, &mut cx) {
                FocusResponse::Speak(announcement) => announcement.render(),
                _ => String::new(),
            };
            (said, cx.ask_sheet)
        };
        assert_eq!(
            step("A1", "Branch"),
            (
                "Branch
A 1"
                .to_owned(),
                false
            )
        );
        assert_eq!(step("B2", "12"), (String::new(), true));
        let place = crate::event::SheetPlace {
            address: "B2".into(),
            row_at: 1,
            column_at: 1,
            row: vec!["Central".into(), "12".into(), "8".into()],
            column: vec!["Weekdays".into(), "12".into(), "10".into(), "22".into()],
            title_row: Some(1),
            title_column: Some(1),
            ..Default::default()
        };
        assert_eq!(
            excel.sheet_place(Some(&place)).as_deref(),
            Some(
                "Weekdays
Central
12
B 2"
            )
        );
    }

    /// The totals readers on C3: the column's SUM formula's value, "column total=14"; no SUM in the
    /// row [obs, 1132/s11, s12].
    #[test]
    fn the_totals_come_from_sum_formulas() {
        let place = crate::event::SheetPlace {
            address: "C3".into(),
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
