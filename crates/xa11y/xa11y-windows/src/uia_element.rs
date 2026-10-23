//! One UI Automation element, found again by its RuntimeId and acted on: what a screen
//! reader needs for an element it read earlier through a cached tree walk.

use windows::Win32::System::Com::SAFEARRAY;
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayCreateVector, SafeArrayDestroy, SafeArrayPutElement,
    SafeArrayUnaccessData,
};
use windows::Win32::System::Variant::{VARIANT, VT_ARRAY, VT_I4};
use windows::Win32::UI::Accessibility::*;

/// The element's RuntimeId: what identifies it while it exists. `None` if the call fails or
/// the SAFEARRAY is not the 1-D array of `i32` UIA documents.
pub fn runtime_id(element: &IUIAutomationElement) -> Option<Vec<i32>> {
    // SAFETY: GetRuntimeId hands over a SAFEARRAY, freed here with SafeArrayDestroy; its data
    // is read only while locked with SafeArrayAccessData.
    unsafe {
        let sa: *mut SAFEARRAY = element.GetRuntimeId().ok()?;
        if sa.is_null() {
            return None;
        }
        let mut data: *mut core::ffi::c_void = core::ptr::null_mut();
        if SafeArrayAccessData(sa, &mut data).is_err() {
            let _ = SafeArrayDestroy(sa);
            return None;
        }
        let count = (*(*sa).rgsabound.as_ptr()).cElements as usize;
        let ids = std::slice::from_raw_parts(data as *const i32, count).to_vec();
        let _ = SafeArrayUnaccessData(sa);
        let _ = SafeArrayDestroy(sa);
        Some(ids)
    }
}

/// The element below (or at) `root` whose RuntimeId is `id`: one search, run in the provider.
pub fn find_by_runtime_id(
    uia: &IUIAutomation,
    root: &IUIAutomationElement,
    id: &[i32],
) -> Option<IUIAutomationElement> {
    // SAFETY: a SAFEARRAY of VT_I4 built and filled here; the VARIANT takes ownership of it
    // and frees it when dropped. The UIA calls are on valid objects.
    unsafe {
        let count = u32::try_from(id.len()).ok()?;
        let sa = SafeArrayCreateVector(VT_I4, 0, count);
        if sa.is_null() {
            return None;
        }
        for (index, value) in id.iter().enumerate() {
            let index = i32::try_from(index).ok()?;
            if SafeArrayPutElement(sa, &index, (value as *const i32).cast()).is_err() {
                let _ = SafeArrayDestroy(sa);
                return None;
            }
        }
        let mut variant = VARIANT::default();
        (*variant.Anonymous.Anonymous).vt = VT_ARRAY | VT_I4;
        (*variant.Anonymous.Anonymous).Anonymous.parray = sa;
        let condition = uia
            .CreatePropertyCondition(UIA_RuntimeIdPropertyId, &variant)
            .ok()?;
        root.FindFirst(TreeScope_Subtree, &condition).ok()
    }
}

/// The element's default action, by the first pattern it has: Invoke (a link or button),
/// Toggle (a check box), SelectionItem (a radio button or tab), then MSAA's default action.
pub fn do_default_action(element: &IUIAutomationElement) -> windows::core::Result<()> {
    // SAFETY: pattern calls on a valid element.
    unsafe {
        if let Ok(invoke) =
            element.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)
        {
            return invoke.Invoke();
        }
        if let Ok(toggle) =
            element.GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)
        {
            return toggle.Toggle();
        }
        if let Ok(select) = element
            .GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId)
        {
            return select.Select();
        }
        element
            .GetCurrentPatternAs::<IUIAutomationLegacyIAccessiblePattern>(
                UIA_LegacyIAccessiblePatternId,
            )?
            .DoDefaultAction()
    }
}

/// The rectangles a text range is drawn in, as `(left, top, right, bottom)` in physical
/// desktop pixels: one per line it spans, in order. Empty if the call fails or the SAFEARRAY is
/// not the 1-D array of `f64` quadruples (left, top, width, height) UIA documents.
pub fn text_range_rectangles(range: &IUIAutomationTextRange) -> Vec<(i32, i32, i32, i32)> {
    // SAFETY: GetBoundingRectangles hands over a SAFEARRAY, freed here with SafeArrayDestroy;
    // its data is read only while locked with SafeArrayAccessData.
    unsafe {
        let Ok(sa) = range.GetBoundingRectangles() else {
            return Vec::new();
        };
        if sa.is_null() {
            return Vec::new();
        }
        let mut data: *mut core::ffi::c_void = core::ptr::null_mut();
        if SafeArrayAccessData(sa, &mut data).is_err() {
            let _ = SafeArrayDestroy(sa);
            return Vec::new();
        }
        let count = (*(*sa).rgsabound.as_ptr()).cElements as usize;
        let values = std::slice::from_raw_parts(data as *const f64, count).to_vec();
        let _ = SafeArrayUnaccessData(sa);
        let _ = SafeArrayDestroy(sa);
        values
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[left, top, width, height]| {
                (
                    left.round() as i32,
                    top.round() as i32,
                    (left + width).round() as i32,
                    (top + height).round() as i32,
                )
            })
            .collect()
    }
}
