//! Word's own object model, reached through its document window (`OBJID_NATIVEOM`), for what
//! its UI Automation does not give: a paragraph's line spacing, which JAWS's Say Font says
//! ("Line spacing: Double" [obs, 1040/s17]). Windows hands the running Word's `Window` object
//! to an assistive technology through the document window, as NVDA's Word support takes it;
//! nothing is started, so Word's activation state does not stand in the way, as it did for a
//! new automation instance (the fixtures' writer, 2026-10-24). Calls are cross-process: made off
//! the keys' path.

use std::ptr::null_mut;

use windows::core::{Interface, GUID, HSTRING, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{IDispatch, DISPATCH_PROPERTYGET, DISPPARAMS};
use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH};
use windows::Win32::UI::Accessibility::AccessibleObjectFromWindow;

/// `OBJID_NATIVEOM`: the window's application object model.
const OBJID_NATIVEOM: u32 = 0xFFFF_FFF0;
/// `LOCALE_USER_DEFAULT`, for property names, which Word takes in English whatever its locale.
const LOCALE_USER_DEFAULT: u32 = 0x0400;

/// A paragraph's line spacing as Word keeps it: its rule (`WdLineSpacing`: 0 single, 1 one
/// and a half, 2 double, 3 at least, 4 exactly, 5 multiple) and its points (for multiple, 12
/// points a line).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSpacing {
    pub rule: i32,
    pub points: f32,
}

/// The line spacing of the selection's paragraph in the Word document window `document`
/// (`_WwG`); `None` where Word's object model does not answer.
pub fn selection_line_spacing(document: HWND) -> Option<LineSpacing> {
    let window = word_window(document)?;
    let selection = object(&property(&window, "Selection")?)?;
    let format = object(&property(&selection, "ParagraphFormat")?)?;
    let rule = i32::try_from(&property(&format, "LineSpacingRule")?).ok()?;
    let points = f64::try_from(&property(&format, "LineSpacing")?).ok()? as f32;
    Some(LineSpacing { rule, points })
}

/// Each stage of [`selection_line_spacing`] with what it gave or the error it met, for the
/// harness (`okur-lab word-line-spacing`).
pub fn line_spacing_stages(document: HWND) -> Vec<String> {
    let mut stages = Vec::new();
    let mut raw = null_mut();
    // SAFETY: as in [`word_window`].
    let found =
        unsafe { AccessibleObjectFromWindow(document, OBJID_NATIVEOM, &IDispatch::IID, &mut raw) };
    stages.push(format!(
        "native object model: {found:?}, null {}",
        raw.is_null()
    ));
    if found.is_err() || raw.is_null() {
        return stages;
    }
    // SAFETY: as in [`word_window`].
    let window = unsafe { IDispatch::from_raw(raw) };
    let mut at = window;
    for name in ["Selection", "ParagraphFormat"] {
        match property_traced(&at, name) {
            Ok(value) => match object(&value) {
                Some(next) => {
                    stages.push(format!("{name}: an object"));
                    at = next;
                }
                None => {
                    stages.push(format!("{name}: not an object ({value:?})"));
                    return stages;
                }
            },
            Err(e) => {
                stages.push(format!("{name}: {e}"));
                return stages;
            }
        }
    }
    for name in ["LineSpacingRule", "LineSpacing"] {
        stages.push(match property_traced(&at, name) {
            Ok(value) => format!("{name}: {value:?}"),
            Err(e) => format!("{name}: {e}"),
        });
    }
    stages
}

/// [`property`] with the stage that failed.
fn property_traced(object: &IDispatch, name: &str) -> Result<VARIANT, String> {
    let wide = HSTRING::from(name);
    let names = [PCWSTR(wide.as_ptr())];
    let mut id = 0i32;
    // SAFETY: as in [`property`].
    unsafe {
        object.GetIDsOfNames(
            &GUID::zeroed(),
            names.as_ptr(),
            1,
            LOCALE_USER_DEFAULT,
            &mut id,
        )
    }
    .map_err(|e| format!("GetIDsOfNames {e}"))?;
    let mut result = VARIANT::default();
    // SAFETY: as in [`property`].
    unsafe {
        object.Invoke(
            id,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            DISPATCH_PROPERTYGET,
            &DISPPARAMS::default(),
            Some(&mut result),
            None,
            None,
        )
    }
    .map_err(|e| format!("Invoke {e}"))?;
    Ok(result)
}

/// The Word document window (`_WwG`) inside the top-level window `top`: the first one.
pub fn document_window(top: HWND) -> Option<HWND> {
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
        if String::from_utf16_lossy(name) == "_WwG" {
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

/// Word's `Window` object for its document window.
fn word_window(document: HWND) -> Option<IDispatch> {
    let mut raw = null_mut();
    // SAFETY: asks the window for an IDispatch into the out-pointer, a local.
    unsafe { AccessibleObjectFromWindow(document, OBJID_NATIVEOM, &IDispatch::IID, &mut raw) }
        .ok()?;
    if raw.is_null() {
        return None;
    }
    // SAFETY: AccessibleObjectFromWindow returned an owned IDispatch pointer.
    Some(unsafe { IDispatch::from_raw(raw) })
}

/// The property `name` of `object`, read by late binding.
fn property(object: &IDispatch, name: &str) -> Option<VARIANT> {
    let wide = HSTRING::from(name);
    let names = [PCWSTR(wide.as_ptr())];
    let mut id = 0i32;
    // SAFETY: one name in, one id out, both locals alive for the call.
    unsafe {
        object.GetIDsOfNames(
            &GUID::zeroed(),
            names.as_ptr(),
            1,
            LOCALE_USER_DEFAULT,
            &mut id,
        )
    }
    .ok()?;
    let mut result = VARIANT::default();
    // SAFETY: a property get with no arguments into a local result.
    unsafe {
        object.Invoke(
            id,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            DISPATCH_PROPERTYGET,
            &DISPPARAMS::default(),
            Some(&mut result),
            None,
            None,
        )
    }
    .ok()?;
    Some(result)
}

/// The object a property gave: Word's are dispatch pointers (`VT_DISPATCH`).
fn object(value: &VARIANT) -> Option<IDispatch> {
    // SAFETY: the discriminant read, then for VT_DISPATCH the pointer it names.
    unsafe {
        let inner = &value.Anonymous.Anonymous;
        (inner.vt == VT_DISPATCH)
            .then(|| inner.Anonymous.pdispVal.as_ref().cloned())
            .flatten()
    }
}
