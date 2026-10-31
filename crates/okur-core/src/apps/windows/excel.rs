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

    /// A cell reached: its value, then its coordinates [obs, 521/s2-s5, s7, s8, s11, s12];
    /// by Ctrl+End and Ctrl+Home, which cell it is first, "Last Cell | 14 | C 4"; by a Ctrl
    /// arrow, that it is a data region's edge after, "Weekends | C 1 | edge of current data
    /// region" [obs, 1115/s1-s6].
    fn focus(&mut self, focus: &FocusEvent, cx: &mut AppContext<'_>) -> FocusResponse {
        let Some(coordinates) = cell(focus) else {
            return FocusResponse::Pass;
        };
        cx.note_window(focus);
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
