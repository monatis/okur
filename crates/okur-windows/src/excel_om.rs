//! Excel's own object model, as Word's (`crate::word_om`): reached from Excel's grid window
//! (`EXCEL7`) with `AccessibleObjectFromWindow(OBJID_NATIVEOM)`, which gives Excel's `Window`
//! object. Read late-bound, through `IDispatch`.

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{IDispatch, DISPATCH_PROPERTYGET};
use windows::Win32::System::Variant::VARIANT;

use crate::word_om::{invoke, number, object, property, text, word_window};

/// The Excel grid window (`EXCEL7`) inside the top-level window `top`: the first one.
pub fn grid_window(top: HWND) -> Option<HWND> {
    use windows::core::BOOL;
    use windows::Win32::Foundation::LPARAM;
    use windows::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetClassNameW};
    unsafe extern "system" fn visit(window: HWND, found: LPARAM) -> BOOL {
        let mut class = [0u16; 16];
        // SAFETY: the buffer outlives the call.
        let n = unsafe { GetClassNameW(window, &mut class) };
        let name = class
            .get(..usize::try_from(n).unwrap_or(0))
            .unwrap_or_default();
        if String::from_utf16_lossy(name) == "EXCEL7" {
            // SAFETY: `found` is the address of the caller's Option<HWND>, alive for the
            // enumeration.
            unsafe { *(found.0 as *mut Option<HWND>) = Some(window) };
            return BOOL(0);
        }
        BOOL(1)
    }
    let mut found: Option<HWND> = None;
    // SAFETY: the callback writes only into `found`, alive for the call.
    let _ = unsafe {
        EnumChildWindows(
            Some(top),
            Some(visit),
            LPARAM(&mut found as *mut Option<HWND> as isize),
        )
    };
    found
}

/// The most cells of a row or a column read for the row's and column's readers.
const MAX_CELLS: i32 = 256;

/// The cells of a row or a column read for its readers: from `from`, `count` cells of it in
/// the sheet's used range, at most [`MAX_CELLS`] always holding the active cell's `at`, as the
/// first and how many. The used range's first cells where they hold it; else those around it:
/// read from the used range's start, a cell past its 256th was not read, and a cell before the
/// used range gave no place at all (performance review 2026-11-02, engine finding 10).
fn line_window(from: i32, count: i32, at: i32) -> (i32, i32) {
    let end = from.saturating_add(count.max(0));
    let start = if at < from {
        at
    } else if at - from < MAX_CELLS {
        from
    } else {
        at - MAX_CELLS / 2
    };
    let stop = end.min(start + MAX_CELLS).max(at + 1);
    (start, stop - start)
}

/// The cells beside a selection's edges in the active cell's row or column, `low` to `high`
/// along it: a selecting key's added cell is at an edge, a removed one just past it.
fn edge_cells(low: i32, high: i32) -> [i32; 4] {
    [low - 1, low, high, high + 1]
}

/// The active cell of the sheet in Excel's grid window `grid` and the cells `cells` asks for
/// around it, each as Excel shows it (`Range.Text`); `None` where Excel's object model does
/// not answer. Each cell costs a few cross-process calls: a move reads only the cells it says
/// ([`okur_core::actions::SheetCells`]).
pub fn sheet_place(
    grid: HWND,
    cells: okur_core::actions::SheetCells,
) -> Option<okur_core::SheetPlace> {
    use okur_core::actions::SheetCells;
    let window = word_window(grid)?;
    let cell = object(&property(&window, "ActiveCell")?)?;
    let (row, column) = (number(&cell, "Row")?, number(&cell, "Column")?);
    let sheet = object(&property(&window, "ActiveSheet")?)?;
    let shown = |r: i32, c: i32| -> String {
        sheet_cell(&sheet, r, c)
            .map(|cell| text(&cell, "Text").trim().to_owned())
            .unwrap_or_default()
    };
    // The selection's bounds and corners, as Excel shows them (`Selection.Cells(1, 1)` and
    // its last).
    let selected = property(&window, "Selection").as_ref().and_then(object);
    let count = |range: &IDispatch, name: &str| {
        property(range, name)
            .as_ref()
            .and_then(object)
            .and_then(|lines| number(&lines, "Count"))
            .unwrap_or(1)
    };
    let span = selected
        .as_ref()
        .map(|range| (count(range, "Rows"), count(range, "Columns")));
    let corner = |last: bool| -> String {
        let (Some(selected), Some((rows, columns))) = (selected.as_ref(), span) else {
            return String::new();
        };
        let (r, c) = if last { (rows, columns) } else { (1, 1) };
        sheet_cell(selected, r, c)
            .map(|cell| text(&cell, "Text").trim().to_owned())
            .unwrap_or_default()
    };
    let mut place = okur_core::SheetPlace {
        address: address(&cell).unwrap_or_default(),
        sheet: text(&sheet, "Name"),
        selection: selected.as_ref().and_then(address).unwrap_or_default(),
        selection_first: corner(false),
        selection_last: corner(true),
        ..okur_core::SheetPlace::default()
    };
    if cells == SheetCells::Lines {
        let used = object(&property(&sheet, "UsedRange")?)?;
        let (top, left) = (number(&used, "Row")?, number(&used, "Column")?);
        let rows = number(&object(&property(&used, "Rows")?)?, "Count")?;
        let columns = number(&object(&property(&used, "Columns")?)?, "Count")?;
        let (first_column, row_count) = line_window(left, columns, column);
        let (first_row, column_count) = line_window(top, rows, row);
        place.row = (first_column..first_column + row_count)
            .map(|c| shown(row, c))
            .collect();
        place.column = (first_row..first_row + column_count)
            .map(|r| shown(r, column))
            .collect();
        place.row_at = usize::try_from(column - first_column).ok()?;
        place.column_at = usize::try_from(row - first_row).ok()?;
        return Some(place);
    }
    let active = shown(row, column);
    place.row = vec![active.clone()];
    place.column = vec![active];
    match cells {
        SheetCells::Format => place.format = cell_format(grid),
        SheetCells::Sums => (place.column_sum, place.row_sum) = cell_sums(grid),
        SheetCells::Around(titles) => {
            let (title_row, title_column) = titles.unwrap_or_else(|| {
                let named = named_titles(&sheet);
                (place.title_row, place.title_column) = named;
                named
            });
            let as_index = |n: u32| i32::try_from(n).ok();
            let mut wanted: Vec<(i32, i32)> = Vec::new();
            if let Some(r) = title_row.and_then(as_index) {
                wanted.push((column, r));
            }
            if let Some(c) = title_column.and_then(as_index) {
                wanted.push((c, row));
            }
            let bounds = selected
                .as_ref()
                .zip(span)
                .and_then(|(range, (rows, columns))| {
                    Some((
                        number(range, "Row")?,
                        number(range, "Column")?,
                        rows,
                        columns,
                    ))
                });
            if let Some((top, left, rows, columns)) = bounds {
                for c in edge_cells(left, left + columns - 1) {
                    wanted.push((c, row));
                }
                for r in edge_cells(top, top + rows - 1) {
                    wanted.push((column, r));
                }
            }
            wanted.sort_unstable();
            wanted.dedup();
            place.cells = wanted
                .into_iter()
                .filter(|&(c, r)| c >= 1 && r >= 1 && (c, r) != (column, row))
                .filter_map(|(c, r)| {
                    Some(okur_core::SheetCell {
                        column: u32::try_from(c).ok()?,
                        row: u32::try_from(r).ok()?,
                        text: shown(r, c),
                    })
                })
                .collect();
        }
        // A range's cells' texts, wherever the active cell is, with its title cells: a monitor
        // cell's.
        SheetCells::Range(range) => {
            place.cells = range
                .titles()
                .into_iter()
                .chain(range.cells())
                .filter_map(|(c, r)| {
                    Some(okur_core::SheetCell {
                        column: c,
                        row: r,
                        text: shown(i32::try_from(r).ok()?, i32::try_from(c).ok()?),
                    })
                })
                .collect();
        }
        SheetCells::Lines => {}
    }
    Some(place)
}

/// The row of column titles and the column of row titles the workbook's names give `sheet`, as
/// JAWS's help describes them (popapps.chm, Excel Row and Column Title Reading): "Title" on the
/// cell where they meet, "ColumnTitle" on the row of column titles, "RowTitle" on the column of
/// row titles, each maybe followed by the worksheet's number or a region ("ColumnTitle4",
/// "ColumnTitle1..N9"; a region's bounds not kept yet [hyp]). A name on another sheet is not
/// this sheet's.
fn named_titles(sheet: &IDispatch) -> (Option<u32>, Option<u32>) {
    let (mut row, mut column) = (None, None);
    let Some(names) = property(sheet, "Parent")
        .as_ref()
        .and_then(object)
        .and_then(|book| property(&book, "Names"))
        .as_ref()
        .and_then(object)
    else {
        return (row, column);
    };
    let sheet_name = text(sheet, "Name");
    let count = number(&names, "Count").unwrap_or(0).clamp(0, MAX_CELLS);
    for index in 1..=count {
        let Some(name) = crate::word_om::item(&names, index) else {
            continue;
        };
        let full = text(&name, "Name");
        // A sheet's own name comes as "Hours!Title".
        let base = full
            .rsplit('!')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let Some(range) = property(&name, "RefersToRange").as_ref().and_then(object) else {
            continue;
        };
        let on_sheet = property(&range, "Worksheet")
            .as_ref()
            .and_then(object)
            .is_some_and(|ws| text(&ws, "Name") == sheet_name);
        if !on_sheet {
            continue;
        }
        let at = |name: &str| number(&range, name).and_then(|n| u32::try_from(n).ok());
        if base.starts_with("columntitle") {
            row = row.or_else(|| at("Row"));
        } else if base.starts_with("rowtitle") {
            column = column.or_else(|| at("Column"));
        } else if base.starts_with("title") {
            row = row.or_else(|| at("Row"));
            column = column.or_else(|| at("Column"));
        }
    }
    (row, column)
}

/// The active cell's font and formatting in Excel's grid window `grid`, for Insert+F: its
/// font (`Font.Name`, `FontStyle`, `Size`, `Color`), its fill (`Interior`, none for
/// `xlColorIndexNone`), its alignments, number format and style, and its size in points.
/// `None` where Excel's object model does not answer.
pub fn cell_format(grid: HWND) -> Option<okur_core::SheetFormat> {
    /// `Interior.ColorIndex` of a cell with no fill.
    const NO_FILL: i64 = -4142;
    let window = word_window(grid)?;
    let cell = object(&property(&window, "ActiveCell")?)?;
    let font = object(&property(&cell, "Font")?)?;
    let interior = object(&property(&cell, "Interior")?)?;
    let style = property(&cell, "Style").as_ref().and_then(object);
    let hundredths = |object: &IDispatch, name: &str| {
        real(object, name)
            .map(|points| (points * 100.0).round().clamp(0.0, f64::from(u32::MAX)) as u32)
    };
    let fill = (whole(&interior, "ColorIndex") != Some(NO_FILL))
        .then(|| whole(&interior, "Color").map(bgr))
        .flatten();
    Some(okur_core::SheetFormat {
        font: text(&font, "Name").trim().to_owned(),
        style: text(&font, "FontStyle").trim().to_owned(),
        size: hundredths(&font, "Size").unwrap_or_default(),
        color: whole(&font, "Color").map(bgr),
        fill,
        horizontal: number(&cell, "HorizontalAlignment").unwrap_or_default(),
        vertical: number(&cell, "VerticalAlignment").unwrap_or_default(),
        number_format: text(&cell, "NumberFormat").trim().to_owned(),
        cell_style: style
            .map(|style| text(&style, "Name").trim().to_owned())
            .unwrap_or_default(),
        width: hundredths(&cell, "Width").unwrap_or_default(),
        height: hundredths(&cell, "Height").unwrap_or_default(),
    })
}

/// The texts of the first cells with a SUM formula in the active cell's column and in its row,
/// within the sheet's used range, for JAWS's totals readers: "column total=14" [obs, 1132/s11].
/// `None` for each where there is none, or where Excel's object model does not answer.
pub fn cell_sums(grid: HWND) -> (Option<String>, Option<String>) {
    let read = || -> Option<(Option<String>, Option<String>)> {
        let window = word_window(grid)?;
        let cell = object(&property(&window, "ActiveCell")?)?;
        let (row, column) = (number(&cell, "Row")?, number(&cell, "Column")?);
        let sheet = object(&property(&window, "ActiveSheet")?)?;
        let used = object(&property(&sheet, "UsedRange")?)?;
        let (top, left) = (number(&used, "Row")?, number(&used, "Column")?);
        let rows = number(&object(&property(&used, "Rows")?)?, "Count")?.clamp(0, MAX_CELLS);
        let columns = number(&object(&property(&used, "Columns")?)?, "Count")?.clamp(0, MAX_CELLS);
        let sum_at = |r: i32, c: i32| {
            let cell = sheet_cell(&sheet, r, c)?;
            let formula = text(&cell, "Formula");
            formula
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("=SUM(")
                .then(|| text(&cell, "Text").trim().to_owned())
        };
        let in_column = (top..top + rows).find_map(|r| sum_at(r, column));
        let in_row = (left..left + columns).find_map(|c| sum_at(row, c));
        Some((in_column, in_row))
    };
    read().unwrap_or_default()
}

/// The number property `name` of `object`, whole or not.
fn real(object: &IDispatch, name: &str) -> Option<f64> {
    let value = property(object, name)?;
    f64::try_from(&value)
        .ok()
        .or_else(|| i32::try_from(&value).ok().map(f64::from))
}

/// The whole number property `name` of `object`: Excel gives colors as doubles.
fn whole(object: &IDispatch, name: &str) -> Option<i64> {
    real(object, name).map(|value| value.round() as i64)
}

/// An Office color (red, green and blue from the low byte up) as RGB.
fn bgr(value: i64) -> okur_core::color::Rgb {
    let byte = |shift: u32| u8::try_from((value >> shift) & 0xFF).unwrap_or(0);
    okur_core::color::Rgb::new(byte(0), byte(8), byte(16))
}

/// The most cells looked through for a sheet's list of cells with formulas.
const MAX_LISTED: i32 = 10_000;

/// One of JAWS's lists of the sheet in Excel's grid window `grid`, with the active cell's
/// column's letters: its cells with formulas, "B4: =SUM(B2:B3)" [obs, 1116/s1]; the active
/// cell's column's cells with data, "Weekdays: B1" [obs, 521/s9]; its workbook's worksheets,
/// "Hours" [obs, 521/s13]. In the sheet's order, by rows. `None` where Excel's object model
/// does not answer.
pub fn sheet_list(
    grid: HWND,
    kind: okur_core::SheetListKind,
) -> Option<(String, Vec<okur_core::SheetEntry>)> {
    let window = word_window(grid)?;
    let cell = object(&property(&window, "ActiveCell")?)?;
    let column = number(&cell, "Column")?;
    let row = number(&cell, "Row")?;
    let letters = address(&cell)?
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>();
    let sheet = object(&property(&window, "ActiveSheet")?)?;
    let entry = |label: String, target: String| okur_core::SheetEntry { label, target };
    let entries = match kind {
        okur_core::SheetListKind::Worksheets => {
            let book = object(&property(&sheet, "Parent")?)?;
            let sheets = object(&property(&book, "Worksheets")?)?;
            let count = number(&sheets, "Count")?.clamp(0, MAX_CELLS);
            (1..=count)
                .filter_map(|index| crate::word_om::item(&sheets, index))
                .map(|ws| text(&ws, "Name").trim().to_owned())
                .filter(|name| !name.is_empty())
                .map(|name| entry(name.clone(), name))
                .collect()
        }
        // The window's visible cells with data, row by row, "B1: Weekdays".
        okur_core::SheetListKind::Visible => {
            let visible = object(&property(&window, "VisibleRange")?)?;
            let (top, left) = (number(&visible, "Row")?, number(&visible, "Column")?);
            let rows = number(&object(&property(&visible, "Rows")?)?, "Count")?.clamp(0, MAX_CELLS);
            let columns =
                number(&object(&property(&visible, "Columns")?)?, "Count")?.clamp(0, MAX_CELLS);
            let mut found = Vec::new();
            for r in top..top + rows {
                for c in left..left + columns {
                    let Some(cell) = sheet_cell(&sheet, r, c) else {
                        continue;
                    };
                    let shown = text(&cell, "Text").trim().to_owned();
                    if shown.is_empty() {
                        continue;
                    }
                    if let Some(at) = address(&cell) {
                        found.push(entry(format!("{at}: {shown}"), at));
                    }
                }
            }
            found
        }
        // The monitor cells are the engine's: it reads each cell itself.
        okur_core::SheetListKind::Monitor => Vec::new(),
        okur_core::SheetListKind::Column
        | okur_core::SheetListKind::Row
        | okur_core::SheetListKind::Formulas => {
            let used = object(&property(&sheet, "UsedRange")?)?;
            let (top, left) = (number(&used, "Row")?, number(&used, "Column")?);
            let rows = number(&object(&property(&used, "Rows")?)?, "Count")?.clamp(0, MAX_LISTED);
            let columns =
                number(&object(&property(&used, "Columns")?)?, "Count")?.clamp(0, MAX_CELLS);
            let (first, last) = if kind == okur_core::SheetListKind::Column {
                (column, column)
            } else {
                (left, left + columns - 1)
            };
            // A row's list: the active cell's row alone.
            let (top, rows) = if kind == okur_core::SheetListKind::Row {
                (row, 1)
            } else {
                (top, rows)
            };
            let mut found = Vec::new();
            let mut looked = 0;
            for r in top..top + rows {
                for c in first..=last {
                    looked += 1;
                    if looked > MAX_LISTED {
                        break;
                    }
                    let Some(cell) = sheet_cell(&sheet, r, c) else {
                        continue;
                    };
                    let Some(at) = address(&cell) else {
                        continue;
                    };
                    if kind != okur_core::SheetListKind::Formulas {
                        let shown = text(&cell, "Text").trim().to_owned();
                        if !shown.is_empty() {
                            found.push(entry(format!("{shown}: {at}"), at));
                        }
                    } else if property(&cell, "HasFormula")
                        .and_then(|v| bool::try_from(&v).ok())
                        .unwrap_or(false)
                    {
                        let formula = text(&cell, "Formula");
                        found.push(entry(format!("{at}: {}", formula.trim()), at));
                    }
                }
            }
            found
        }
    };
    // A row's list is titled by its number, the others by the column's letters.
    // The visible cells' list selects the active cell's entry, so it carries its reference.
    let place = match kind {
        okur_core::SheetListKind::Row => row.to_string(),
        okur_core::SheetListKind::Visible => address(&cell).unwrap_or_default(),
        _ => letters,
    };
    Some((place, entries))
}

/// Goes to `target` of one of those lists in Excel's grid window `grid`: selects the cell it
/// names, or activates the worksheet. False where Excel's object model does not answer.
pub fn go_to(grid: HWND, kind: okur_core::SheetListKind, target: &str) -> bool {
    use windows::core::BSTR;
    use windows::Win32::System::Com::DISPATCH_METHOD;
    let go = || -> Option<()> {
        let window = word_window(grid)?;
        let sheet = object(&property(&window, "ActiveSheet")?)?;
        if kind == okur_core::SheetListKind::Worksheets {
            let book = object(&property(&sheet, "Parent")?)?;
            let sheets = object(&property(&book, "Worksheets")?)?;
            let named = object(&invoke(
                &sheets,
                "Item",
                DISPATCH_PROPERTYGET,
                &mut [VARIANT::from(BSTR::from(target))],
            )?)?;
            invoke(&named, "Activate", DISPATCH_METHOD, &mut [])?;
        } else {
            let range = object(&invoke(
                &sheet,
                "Range",
                DISPATCH_PROPERTYGET,
                &mut [VARIANT::from(BSTR::from(target))],
            )?)?;
            invoke(&range, "Select", DISPATCH_METHOD, &mut [])?;
        }
        Some(())
    };
    go().is_some()
}

/// Opens the sheet tabs' context menu of the Excel in grid window `grid`
/// (`Application.CommandBars("Ply").ShowPopup`). Excel holds the call until the menu closes.
/// False where Excel's object model does not answer.
pub fn show_sheet_tabs_menu(grid: HWND) -> bool {
    use windows::core::BSTR;
    use windows::Win32::System::Com::DISPATCH_METHOD;
    let show = || -> Option<()> {
        let window = word_window(grid)?;
        let application = object(&property(&window, "Application")?)?;
        let bars = object(&property(&application, "CommandBars")?)?;
        let ply = object(&invoke(
            &bars,
            "Item",
            DISPATCH_PROPERTYGET,
            &mut [VARIANT::from(BSTR::from("Ply"))],
        )?)?;
        invoke(&ply, "ShowPopup", DISPATCH_METHOD, &mut [])?;
        Some(())
    };
    show().is_some()
}

/// A cell's reference without dollar signs, "B4" (`Range.Address(False, False)`).
fn address(cell: &IDispatch) -> Option<String> {
    let value = invoke(
        cell,
        "Address",
        DISPATCH_PROPERTYGET,
        &mut [VARIANT::from(false), VARIANT::from(false)],
    )?;
    let shown = windows::core::BSTR::try_from(&value).ok()?.to_string();
    let shown = shown.trim().to_owned();
    (!shown.is_empty()).then_some(shown)
}

/// The cell at row `row` and column `column` (from 1) of `sheet` (`Worksheet.Cells`).
fn sheet_cell(sheet: &IDispatch, row: i32, column: i32) -> Option<IDispatch> {
    object(&invoke(
        sheet,
        "Cells",
        DISPATCH_PROPERTYGET,
        &mut [VARIANT::from(row), VARIANT::from(column)],
    )?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row's or column's cells read always hold the active cell: the used range's first ones
    /// where they hold it, else the 256 around it; before the used range, from the active cell.
    #[test]
    fn a_line_read_holds_the_active_cell() {
        assert_eq!(line_window(1, 10, 3), (1, 10));
        assert_eq!(line_window(1, 1000, 3), (1, 256));
        assert_eq!(line_window(1, 1000, 600), (472, 256));
        assert_eq!(line_window(1, 1000, 999), (871, 130));
        // Past the used range's end, up to the active cell; before its start, from it.
        assert_eq!(line_window(1, 3, 26), (1, 26));
        assert_eq!(line_window(2, 5, 1), (1, 6));
        assert_eq!(line_window(1, 0, 1), (1, 1));
        for (from, count, at) in [(1, 1000, 600), (1, 3, 26), (5, 100, 2), (1, 70_000, 65_000)] {
            let (start, cells) = line_window(from, count, at);
            assert!(start <= at && at < start + cells, "{from} {count} {at}");
            assert!(cells <= MAX_CELLS, "{from} {count} {at}");
        }
    }

    #[test]
    fn a_selections_edges_and_the_cells_past_them() {
        assert_eq!(edge_cells(2, 2), [1, 2, 2, 3]);
        assert_eq!(edge_cells(2, 5), [1, 2, 5, 6]);
    }
}
