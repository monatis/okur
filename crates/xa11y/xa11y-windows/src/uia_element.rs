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

/// The list item at `index` (from 0) among `list`'s children in the control view, stepping
/// over the others (a details view's header): a Win32 list view's focused item, whose index
/// the list view gives while UI Automation gives the list as the focus. At most `max_steps`
/// children are looked at, each a call into the provider; `None` when `list` is not a list,
/// or its items are not its children (a list in groups).
pub fn nth_list_item(
    uia: &IUIAutomation,
    list: &IUIAutomationElement,
    index: usize,
    max_steps: usize,
) -> Option<IUIAutomationElement> {
    // SAFETY: UIA client calls on valid automation objects.
    unsafe {
        if list.CurrentControlType().ok()? != UIA_ListControlTypeId {
            return None;
        }
        let walker = uia.ControlViewWalker().ok()?;
        let mut element = walker.GetFirstChildElement(list).ok()?;
        let mut items = 0;
        for _ in 0..max_steps {
            if element.CurrentControlType().ok()? == UIA_ListItemControlTypeId {
                if items == index {
                    return Some(element);
                }
                items += 1;
            }
            element = walker.GetNextSiblingElement(&element).ok()?;
        }
        None
    }
}

/// UI Automation's focused element for a focus event in `window`, unless UI Automation still
/// gives a window outside `window`'s top-level window: then `window`'s own element. Windows
/// sends an application's focus as its window comes to the front while UI Automation can still
/// answer with the last focus (the taskbar, as Programs and Features first opened on old-pc,
/// 2026-11-03).
pub fn focus_in_window(
    uia: &IUIAutomation,
    window: windows::Win32::Foundation::HWND,
) -> Option<IUIAutomationElement> {
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};
    // SAFETY: UIA client calls on a valid automation object; plain Win32 queries on window
    // handles, a handle that is gone giving none.
    unsafe {
        let focused = uia.GetFocusedElement().ok();
        let elsewhere = focused.as_ref().is_some_and(|element| {
            element
                .CurrentNativeWindowHandle()
                .ok()
                .filter(|handle| !handle.is_invalid())
                .is_some_and(|handle| GetAncestor(handle, GA_ROOT) != GetAncestor(window, GA_ROOT))
        });
        if elsewhere && !window.is_invalid() {
            uia.ElementFromHandle(window).ok().or(focused)
        } else {
            focused
        }
    }
}

/// The names of the texts directly in window `hwnd`'s element, in order, in the control view:
/// a Store app's core window gives its own title and its page's ("Settings", "System"). At
/// most `max_children` children are looked at.
pub fn window_texts(
    uia: &IUIAutomation,
    hwnd: windows::Win32::Foundation::HWND,
    max_children: usize,
) -> Vec<String> {
    let mut texts = Vec::new();
    // SAFETY: UIA client calls on valid automation objects.
    unsafe {
        let (Ok(window), Ok(walker)) = (uia.ElementFromHandle(hwnd), uia.ControlViewWalker())
        else {
            return texts;
        };
        let mut child = walker.GetFirstChildElement(&window).ok();
        for _ in 0..max_children {
            let Some(element) = child else { break };
            if element.CurrentControlType().ok() == Some(UIA_TextControlTypeId) {
                if let Ok(name) = element.CurrentName() {
                    let name = name.to_string();
                    if !name.trim().is_empty() {
                        texts.push(name);
                    }
                }
            }
            child = walker.GetNextSiblingElement(&element).ok();
        }
    }
    texts
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
