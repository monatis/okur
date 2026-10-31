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
    Some(okur_core::SheetPlace {
        row_at: at(left, column)?,
        column_at: at(top, row)?,
        row: row_cells,
        column: column_cells,
    })
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
