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

/// Excel's module.
#[derive(Debug, Default)]
pub struct Excel;

impl AppModule for Excel {
    fn owns(&self, focus: &FocusEvent) -> bool {
        cell(focus).is_some()
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
        let Some(coordinates) = cell(focus) else {
            return FocusResponse::Pass;
        };
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
        if matches!(
            key,
            Some(Key::LeftArrow | Key::RightArrow | Key::UpArrow | Key::DownArrow)
        ) {
            lines.push(msg::excel_data_region_edge().to_string());
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
