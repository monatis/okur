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

/// The active cell of the sheet in Excel's grid window `grid`, with the cells of its row and
/// its column within the sheet's used range, each as Excel shows it (`Range.Text`); `None`
/// where Excel's object model does not answer.
pub fn sheet_place(grid: HWND) -> Option<okur_core::SheetPlace> {
    let window = word_window(grid)?;
    let cell = object(&property(&window, "ActiveCell")?)?;
    let (row, column) = (number(&cell, "Row")?, number(&cell, "Column")?);
    let sheet = object(&property(&window, "ActiveSheet")?)?;
    let used = object(&property(&sheet, "UsedRange")?)?;
    let (top, left) = (number(&used, "Row")?, number(&used, "Column")?);
    let rows = number(&object(&property(&used, "Rows")?)?, "Count")?.clamp(0, MAX_CELLS);
    let columns = number(&object(&property(&used, "Columns")?)?, "Count")?.clamp(0, MAX_CELLS);
    let shown = |r: i32, c: i32| -> String {
        sheet_cell(&sheet, r, c)
            .map(|cell| text(&cell, "Text").trim().to_owned())
            .unwrap_or_default()
    };
    // The active cell within its row and column, outside the used range at either end.
    let row_cells: Vec<String> = (left..left + columns).map(|c| shown(row, c)).collect();
    let column_cells: Vec<String> = (top..top + rows).map(|r| shown(r, column)).collect();
    let at = |from: i32, here: i32| usize::try_from(here.saturating_sub(from)).ok();
    // The selection's corners, as Excel shows them (`Selection.Cells(1, 1)` and its last).
    let selected = property(&window, "Selection").as_ref().and_then(object);
    let corner = |last: bool| -> String {
        let Some(selected) = selected.as_ref() else {
            return String::new();
        };
        let count = |name: &str| {
            property(selected, name)
                .as_ref()
                .and_then(object)
                .and_then(|range| number(&range, "Count"))
                .unwrap_or(1)
        };
        let (r, c) = if last {
            (count("Rows"), count("Columns"))
        } else {
            (1, 1)
        };
        sheet_cell(selected, r, c)
            .map(|cell| text(&cell, "Text").trim().to_owned())
            .unwrap_or_default()
    };
    Some(okur_core::SheetPlace {
        address: address(&cell).unwrap_or_default(),
        selection: selected.as_ref().and_then(address).unwrap_or_default(),
        selection_first: corner(false),
        selection_last: corner(true),
        row_at: at(left, column)?,
        column_at: at(top, row)?,
        row: row_cells,
        column: column_cells,
    })
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
        okur_core::SheetListKind::Column | okur_core::SheetListKind::Formulas => {
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
                    if kind == okur_core::SheetListKind::Column {
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
    Some((letters, entries))
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
