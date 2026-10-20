//! IAccessible2 (IA2) client support: COM bindings and document tree capture.
//!
//! Chromium and Firefox expose their richest accessibility model through IA2 on top of
//! MSAA: HTML tags and CSS display (`tag:`, `display:` object attributes), visited state,
//! hypertext with embedded-object characters, and rendered line boundaries via
//! `IAccessibleText`. UI Automation exposes none of these, and JAWS and NVDA both read
//! browsers through IA2.
//!
//! Interface layouts follow the IA2 IDL (IAccessible2 1.3, <https://github.com/LinuxA11y/IAccessible2>,
//! BSD licensed): methods are declared in exact vtable order. Only interfaces the capture
//! uses are bound; [`Ia2Node`] and [`parse_attributes`] are platform independent so hermetic
//! tests can load IA2 snapshots on any OS.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// `IA2_TEXT_BOUNDARY_LINE`.
pub const IA2_TEXT_BOUNDARY_LINE: i32 = 4;
/// `IA2_TEXT_OFFSET_LENGTH`: the end of the text.
pub const IA2_TEXT_OFFSET_LENGTH: i32 = -1;
/// The embedded object character standing in for a child object inside hypertext.
pub const EMBEDDED_OBJECT_CHAR: char = '\u{FFFC}';

/// Selected IA2 roles (`AccessibleRole.idl`); MSAA roles are used for everything else.
pub mod roles {
    pub const IA2_ROLE_CAPTION: i32 = 0x402;
    pub const IA2_ROLE_FOOTER: i32 = 0x40E;
    pub const IA2_ROLE_FORM: i32 = 0x410;
    pub const IA2_ROLE_HEADER: i32 = 0x413;
    pub const IA2_ROLE_HEADING: i32 = 0x414;
    pub const IA2_ROLE_LABEL: i32 = 0x419;
    pub const IA2_ROLE_PARAGRAPH: i32 = 0x41E;
    pub const IA2_ROLE_SECTION: i32 = 0x424;
    pub const IA2_ROLE_TOGGLE_BUTTON: i32 = 0x42A;
    pub const IA2_ROLE_COMPLEMENTARY_CONTENT: i32 = 0x42C;
    pub const IA2_ROLE_LANDMARK: i32 = 0x42D;
    pub const IA2_ROLE_BLOCK_QUOTE: i32 = 0x431;
}

/// Selected MSAA states (`oleacc.h`).
pub mod msaa_states {
    pub const STATE_SYSTEM_UNAVAILABLE: u32 = 0x1;
    pub const STATE_SYSTEM_SELECTED: u32 = 0x2;
    pub const STATE_SYSTEM_FOCUSED: u32 = 0x4;
    pub const STATE_SYSTEM_PRESSED: u32 = 0x8;
    pub const STATE_SYSTEM_CHECKED: u32 = 0x10;
    pub const STATE_SYSTEM_MIXED: u32 = 0x20;
    pub const STATE_SYSTEM_READONLY: u32 = 0x40;
    pub const STATE_SYSTEM_EXPANDED: u32 = 0x200;
    pub const STATE_SYSTEM_COLLAPSED: u32 = 0x400;
    pub const STATE_SYSTEM_INVISIBLE: u32 = 0x8000;
    pub const STATE_SYSTEM_OFFSCREEN: u32 = 0x10000;
    pub const STATE_SYSTEM_FOCUSABLE: u32 = 0x100000;
    /// A visited link.
    pub const STATE_SYSTEM_TRAVERSED: u32 = 0x800000;
    pub const STATE_SYSTEM_LINKED: u32 = 0x400000;
    pub const STATE_SYSTEM_PROTECTED: u32 = 0x20000000;
}

/// A text object's text, caret and first selection, offsets in UTF-16 code units.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ia2TextState {
    pub text: String,
    pub caret: i32,
    pub selection: Option<(i32, i32)>,
    /// The runs marked as spelling errors (`invalid:spelling`), in UTF-16 code units.
    pub misspelled: Vec<(i32, i32)>,
}

/// Where a text object's caret and first selection are, and its length, in UTF-16 code
/// units: what changes when a key moves the caret or edits the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ia2TextPosition {
    pub caret: i32,
    pub selection: Option<(i32, i32)>,
    pub length: i32,
}

/// One accessible object as exposed through MSAA + IA2.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Ia2Node {
    /// MSAA role (`ROLE_SYSTEM_*`), when reported as a number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msaa_role: Option<i32>,
    /// MSAA role, when reported as a string (custom roles).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msaa_role_name: Option<String>,
    /// IA2 role (`IA2_ROLE_*` or an MSAA role number).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ia2_role: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MSAA state bits (`STATE_SYSTEM_*`).
    pub msaa_states: u32,
    /// IA2 state bits (`IA2_STATE_*`).
    pub ia2_states: i32,
    /// `IAccessible2::uniqueID`: identifies the live object within its window, where
    /// `accChild` on the document resolves it again (focus, default action).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<i32>,
    /// Parsed IA2 object attributes (`tag`, `display`, `xml-roles`, `level`, …).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
    /// IA2 group position: `[level, similar_items_in_group, position_in_group]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_position: Option<[i32; 3]>,
    /// Hypertext: the object's text, with [`EMBEDDED_OBJECT_CHAR`] for each embedded child.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Rendered line ranges within `text`, as `[start, end)` offsets in UTF-16 code units
    /// (`IAccessibleText::textAtOffset` with `IA2_TEXT_BOUNDARY_LINE`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lines: Vec<[i32; 2]>,
    /// Text formatting runs within `text` (`IAccessibleText::attributes`), keeping the font
    /// attributes JAWS reports with Say Font: family, size, weight, style, underline and
    /// line-through.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_runs: Vec<Ia2TextRun>,
    /// IA2 relations (`IAccessible2::relations`): each relation's type (`flowsTo`,
    /// `labelledBy`, ...) and its targets' uniqueIDs.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub relations: BTreeMap<String, Vec<i32>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Ia2Node>,
}

impl Ia2Node {
    /// Total number of nodes in this subtree, including `self`.
    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(Ia2Node::node_count).sum::<usize>()
    }
}

/// Parses an IA2 object attribute string (`name:value;name:value;`), where `\` escapes
/// `\`, `:`, `;`, `=` and `,`.
/// A run of text with one formatting, as `[start, end)` UTF-16 offsets into the hypertext.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ia2TextRun {
    pub start: i32,
    pub end: i32,
    pub attributes: BTreeMap<String, String>,
}

/// The text attributes a run keeps: Say Font's, and Say Color's (`color`,
/// `background-color`, as Chromium gives them: `rgb(0, 0, 238)`).
pub const FONT_ATTRIBUTES: [&str; 8] = [
    "font-family",
    "font-size",
    "font-weight",
    "font-style",
    "text-underline-style",
    "text-line-through-style",
    "color",
    "background-color",
];

pub fn parse_attributes(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut in_value = false;
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    if in_value {
                        value.push(escaped);
                    } else {
                        key.push(escaped);
                    }
                }
            }
            ':' if !in_value => in_value = true,
            ';' => {
                if !key.is_empty() {
                    out.insert(std::mem::take(&mut key), std::mem::take(&mut value));
                }
                key.clear();
                value.clear();
                in_value = false;
            }
            _ if in_value => value.push(c),
            _ => key.push(c),
        }
    }
    if !key.is_empty() {
        out.insert(key, value);
    }
    out
}

#[cfg(target_os = "windows")]
pub use com::capture::{
    capture_chromium_document, capture_chromium_object, chromium_caret_text, chromium_text_state,
    has_chromium_document, perform_chromium_action, ChromiumTextField, Ia2Action, Ia2CaptureError,
};

#[cfg(target_os = "windows")]
#[allow(non_snake_case, clippy::too_many_arguments)]
mod com {
    //! Raw IA2 interfaces, declared in IDL vtable order.
    use std::ffi::c_void;
    use windows::core::{BSTR, HRESULT};
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::Accessibility::{IAccessible, IAccessible_Impl, IAccessible_Vtbl};
    use windows_core::{IUnknown, IUnknown_Vtbl};

    #[windows_core::interface("E89F726E-C4F4-4c19-BB19-B647D7FA8478")]
    pub unsafe trait IAccessible2: IAccessible {
        fn get_nRelations(&self, n: *mut i32) -> HRESULT;
        fn get_relation(&self, index: i32, relation: *mut *mut c_void) -> HRESULT;
        fn get_relations(&self, max: i32, relations: *mut *mut c_void, n: *mut i32) -> HRESULT;
        fn role(&self, role: *mut i32) -> HRESULT;
        fn scrollTo(&self, scroll_type: i32) -> HRESULT;
        fn scrollToPoint(&self, coordinate_type: i32, x: i32, y: i32) -> HRESULT;
        fn get_groupPosition(
            &self,
            level: *mut i32,
            similar: *mut i32,
            position: *mut i32,
        ) -> HRESULT;
        fn get_states(&self, states: *mut i32) -> HRESULT;
        fn get_extendedRole(&self, role: *mut BSTR) -> HRESULT;
        fn get_localizedExtendedRole(&self, role: *mut BSTR) -> HRESULT;
        fn get_nExtendedStates(&self, n: *mut i32) -> HRESULT;
        fn get_extendedStates(&self, max: i32, states: *mut *mut BSTR, n: *mut i32) -> HRESULT;
        fn get_localizedExtendedStates(
            &self,
            max: i32,
            states: *mut *mut BSTR,
            n: *mut i32,
        ) -> HRESULT;
        fn get_uniqueID(&self, id: *mut i32) -> HRESULT;
        fn get_windowHandle(&self, hwnd: *mut HWND) -> HRESULT;
        fn get_indexInParent(&self, index: *mut i32) -> HRESULT;
        fn get_locale(&self, locale: *mut c_void) -> HRESULT;
        fn get_attributes(&self, attributes: *mut BSTR) -> HRESULT;
    }

    #[windows_core::interface("24FD2FFB-3AAD-4a08-8335-A3AD89C0FB4B")]
    pub unsafe trait IAccessibleText: IUnknown {
        fn addSelection(&self, start: i32, end: i32) -> HRESULT;
        fn get_attributes(
            &self,
            offset: i32,
            start: *mut i32,
            end: *mut i32,
            attrs: *mut BSTR,
        ) -> HRESULT;
        fn get_caretOffset(&self, offset: *mut i32) -> HRESULT;
        fn get_characterExtents(
            &self,
            offset: i32,
            coordinate_type: i32,
            x: *mut i32,
            y: *mut i32,
            width: *mut i32,
            height: *mut i32,
        ) -> HRESULT;
        fn get_nSelections(&self, n: *mut i32) -> HRESULT;
        fn get_offsetAtPoint(
            &self,
            x: i32,
            y: i32,
            coordinate_type: i32,
            offset: *mut i32,
        ) -> HRESULT;
        fn get_selection(&self, index: i32, start: *mut i32, end: *mut i32) -> HRESULT;
        fn get_text(&self, start: i32, end: i32, text: *mut BSTR) -> HRESULT;
        fn get_textBeforeOffset(
            &self,
            offset: i32,
            boundary: i32,
            start: *mut i32,
            end: *mut i32,
            text: *mut BSTR,
        ) -> HRESULT;
        fn get_textAfterOffset(
            &self,
            offset: i32,
            boundary: i32,
            start: *mut i32,
            end: *mut i32,
            text: *mut BSTR,
        ) -> HRESULT;
        fn get_textAtOffset(
            &self,
            offset: i32,
            boundary: i32,
            start: *mut i32,
            end: *mut i32,
            text: *mut BSTR,
        ) -> HRESULT;
        fn removeSelection(&self, index: i32) -> HRESULT;
        fn setCaretOffset(&self, offset: i32) -> HRESULT;
        fn setSelection(&self, index: i32, start: i32, end: i32) -> HRESULT;
        fn get_nCharacters(&self, n: *mut i32) -> HRESULT;
        fn scrollSubstringTo(&self, start: i32, end: i32, scroll_type: i32) -> HRESULT;
        fn scrollSubstringToPoint(
            &self,
            start: i32,
            end: i32,
            coordinate_type: i32,
            x: i32,
            y: i32,
        ) -> HRESULT;
        fn get_newText(&self, segment: *mut c_void) -> HRESULT;
        fn get_oldText(&self, segment: *mut c_void) -> HRESULT;
    }

    #[windows_core::interface("7CDF86EE-C3DA-496a-BDA4-281B336E1FDC")]
    pub unsafe trait IAccessibleRelation: IUnknown {
        fn get_relationType(&self, kind: *mut BSTR) -> HRESULT;
        fn get_localizedRelationType(&self, kind: *mut BSTR) -> HRESULT;
        fn get_nTargets(&self, n: *mut i32) -> HRESULT;
        fn get_target(&self, index: i32, target: *mut *mut c_void) -> HRESULT;
        fn get_targets(&self, max: i32, targets: *mut *mut c_void, n: *mut i32) -> HRESULT;
    }

    pub mod capture {
        use super::super::{
            parse_attributes, Ia2Node, Ia2TextPosition, Ia2TextRun, Ia2TextState,
            EMBEDDED_OBJECT_CHAR, FONT_ATTRIBUTES, IA2_TEXT_BOUNDARY_LINE, IA2_TEXT_OFFSET_LENGTH,
        };
        use super::{IAccessible2, IAccessibleRelation, IAccessibleText};
        use windows::core::{Interface, BSTR, PCWSTR};
        use windows::Win32::Foundation::HWND;
        use windows::Win32::System::Com::{IDispatch, IServiceProvider};
        use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH};
        use windows::Win32::UI::Accessibility::{
            AccessibleChildren, AccessibleObjectFromWindow, IAccessible,
        };
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, IsWindowVisible};

        /// Render windows examined per top-level window, at most (Chrome keeps one per tab).
        const MAX_RENDER_WINDOWS: usize = 512;

        /// `OBJID_CLIENT`.
        const OBJID_CLIENT: u32 = 0xFFFF_FFFC;
        /// Bounds recursion and text size on pathological documents.
        const MAX_DEPTH: usize = 256;
        const MAX_TEXT_CHARS: i32 = 1_000_000;
        /// Formatting runs read per object at most.
        const MAX_TEXT_RUNS: usize = 512;

        #[derive(Debug)]
        pub struct Ia2CaptureError(pub String);

        impl std::fmt::Display for Ia2CaptureError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::error::Error for Ia2CaptureError {}

        fn err(context: &str, e: impl std::fmt::Display) -> Ia2CaptureError {
            Ia2CaptureError(format!("{context}: {e}"))
        }

        fn self_child() -> VARIANT {
            VARIANT::from(0i32)
        }

        fn string(result: windows::core::Result<BSTR>) -> Option<String> {
            result.ok().map(|b| b.to_string()).filter(|s| !s.is_empty())
        }

        /// The IA2 interface of an MSAA object, via `IServiceProvider` as the IA2 spec requires.
        fn ia2_of(acc: &IAccessible) -> Option<IAccessible2> {
            let sp: IServiceProvider = acc.cast().ok()?;
            // SAFETY: QueryService with the IAccessible service id and IAccessible2 IID.
            unsafe { sp.QueryService::<IAccessible2>(&IAccessible::IID) }.ok()
        }

        /// An MSAA object's IA2 object attributes (`text-align`, `display`, …), when it has
        /// IA2.
        fn object_attributes(
            acc: &IAccessible,
        ) -> Option<std::collections::BTreeMap<String, String>> {
            let ia2 = ia2_of(acc)?;
            let mut attrs = BSTR::default();
            // SAFETY: out-pointer to a local on a valid interface; BSTR ownership transfers.
            unsafe { ia2.get_attributes(&mut attrs) }.ok().ok()?;
            Some(parse_attributes(&attrs.to_string()))
        }

        fn read_text(text: &IAccessibleText, node: &mut Ia2Node) {
            let mut n = 0i32;
            // SAFETY: valid interface, out-pointer to a local.
            if unsafe { text.get_nCharacters(&mut n) }.is_err() || n <= 0 {
                return;
            }
            let n = n.min(MAX_TEXT_CHARS);
            let mut bstr = BSTR::default();
            // SAFETY: as above; BSTR ownership transfers to `bstr`.
            if unsafe { text.get_text(0, IA2_TEXT_OFFSET_LENGTH, &mut bstr) }.is_ok() {
                node.text = Some(bstr.to_string());
            }
            let mut offset = 0i32;
            while offset < n {
                let (mut start, mut end) = (0i32, 0i32);
                let mut line = BSTR::default();
                // SAFETY: as above.
                let hr = unsafe {
                    text.get_textAtOffset(
                        offset,
                        IA2_TEXT_BOUNDARY_LINE,
                        &mut start,
                        &mut end,
                        &mut line,
                    )
                };
                if hr.is_err() || end <= offset {
                    break;
                }
                node.lines.push([start, end]);
                offset = end;
            }
            let mut offset = 0i32;
            while offset < n && node.text_runs.len() < MAX_TEXT_RUNS {
                let (mut start, mut end) = (0i32, 0i32);
                let mut attrs = BSTR::default();
                // SAFETY: as above.
                let hr = unsafe { text.get_attributes(offset, &mut start, &mut end, &mut attrs) };
                if hr.is_err() || end <= offset {
                    break;
                }
                let attributes = parse_attributes(&attrs.to_string())
                    .into_iter()
                    .filter(|(k, _)| FONT_ATTRIBUTES.contains(&k.as_str()))
                    .collect();
                // Adjacent runs that differ only in attributes we do not keep are one run.
                match node.text_runs.last_mut() {
                    Some(last) if last.end == start && last.attributes == attributes => {
                        last.end = end;
                    }
                    _ => node.text_runs.push(Ia2TextRun {
                        start,
                        end,
                        attributes,
                    }),
                }
                offset = end;
            }
        }

        /// An object's IA2 relations, each type with its targets' uniqueIDs: aria-flowto is
        /// `flowsTo`, which JAWS follows with Equals.
        fn read_relations(ia2: &IAccessible2, node: &mut Ia2Node) {
            const MAX_RELATIONS: i32 = 32;
            const MAX_TARGETS: i32 = 64;
            let mut n = 0i32;
            // SAFETY: valid interface, out-pointer to a local.
            if unsafe { ia2.get_nRelations(&mut n) }.is_err() || n <= 0 {
                return;
            }
            for index in 0..n.min(MAX_RELATIONS) {
                let mut raw: *mut std::ffi::c_void = std::ptr::null_mut();
                // SAFETY: as above; on success `raw` holds a reference this code now owns.
                if unsafe { ia2.get_relation(index, &mut raw) }.is_err() || raw.is_null() {
                    continue;
                }
                // SAFETY: `raw` is an owned IAccessibleRelation pointer from get_relation.
                let relation = unsafe { IAccessibleRelation::from_raw(raw) };
                let mut kind = BSTR::default();
                // SAFETY: valid interface; BSTR ownership transfers to `kind`.
                if unsafe { relation.get_relationType(&mut kind) }.is_err() {
                    continue;
                }
                let mut targets = 0i32;
                // SAFETY: valid interface, out-pointer to a local.
                if unsafe { relation.get_nTargets(&mut targets) }.is_err() {
                    continue;
                }
                let mut ids = Vec::new();
                for t in 0..targets.clamp(0, MAX_TARGETS) {
                    let mut raw: *mut std::ffi::c_void = std::ptr::null_mut();
                    // SAFETY: as above; on success `raw` is an owned IUnknown pointer.
                    if unsafe { relation.get_target(t, &mut raw) }.is_err() || raw.is_null() {
                        continue;
                    }
                    // SAFETY: `raw` is an owned IUnknown pointer from get_target.
                    let target = unsafe { windows_core::IUnknown::from_raw(raw) };
                    let id = target
                        .cast::<IAccessible>()
                        .ok()
                        .and_then(|acc| ia2_of(&acc))
                        .and_then(|target| {
                            let mut id = 0i32;
                            // SAFETY: valid interface, out-pointer to a local.
                            unsafe { target.get_uniqueID(&mut id) }
                                .ok()
                                .ok()
                                .map(|()| id)
                        });
                    ids.extend(id);
                }
                if !ids.is_empty() {
                    node.relations
                        .entry(kind.to_string())
                        .or_default()
                        .extend(ids);
                }
            }
        }

        fn read_node(acc: &IAccessible, depth: usize) -> Ia2Node {
            let me = self_child();
            let mut node = Ia2Node::default();
            // SAFETY: plain MSAA property reads on a valid IAccessible.
            unsafe {
                if let Ok(role) = acc.get_accRole(&me) {
                    match i32::try_from(&role) {
                        Ok(r) => node.msaa_role = Some(r),
                        Err(_) => {
                            node.msaa_role_name = BSTR::try_from(&role).ok().map(|b| b.to_string())
                        }
                    }
                }
                node.name = string(acc.get_accName(&me));
                node.value = string(acc.get_accValue(&me));
                node.description = string(acc.get_accDescription(&me));
                if let Ok(state) = acc.get_accState(&me) {
                    node.msaa_states = i32::try_from(&state).map(|s| s as u32).unwrap_or(0);
                }
            }

            if let Some(ia2) = ia2_of(acc) {
                // SAFETY: valid IA2 interface; all out-pointers point to locals.
                unsafe {
                    let mut role = 0i32;
                    if ia2.role(&mut role).is_ok() {
                        node.ia2_role = Some(role);
                    }
                    let mut states = 0i32;
                    if ia2.get_states(&mut states).is_ok() {
                        node.ia2_states = states;
                    }
                    let mut unique_id = 0i32;
                    if ia2.get_uniqueID(&mut unique_id).is_ok() {
                        node.unique_id = Some(unique_id);
                    }
                    let mut attrs = BSTR::default();
                    if ia2.get_attributes(&mut attrs).is_ok() {
                        node.attributes = parse_attributes(&attrs.to_string());
                    }
                    let (mut level, mut similar, mut position) = (0i32, 0i32, 0i32);
                    if ia2
                        .get_groupPosition(&mut level, &mut similar, &mut position)
                        .is_ok()
                        && (level, similar, position) != (0, 0, 0)
                    {
                        node.group_position = Some([level, similar, position]);
                    }
                }
                read_relations(&ia2, &mut node);
                if let Ok(text) = ia2.cast::<IAccessibleText>() {
                    read_text(&text, &mut node);
                }
            }

            if depth < MAX_DEPTH {
                // SAFETY: valid IAccessible.
                let count = unsafe { acc.accChildCount() }.unwrap_or(0).max(0);
                if count > 0 {
                    let mut children = vec![VARIANT::default(); count as usize];
                    let mut obtained = 0i32;
                    // SAFETY: the slice length is the requested child count.
                    if unsafe { AccessibleChildren(acc, 0, &mut children, &mut obtained) }.is_ok() {
                        for child in children.iter().take(obtained.max(0) as usize) {
                            if let Some(child_acc) = dispatch_child(child) {
                                node.children.push(read_node(&child_acc, depth + 1));
                            }
                        }
                    }
                }
            }
            node
        }

        /// An MSAA child returned as `VT_DISPATCH` (Chromium returns objects, not child ids).
        fn dispatch_child(v: &VARIANT) -> Option<IAccessible> {
            // SAFETY: reading the discriminant and, for VT_DISPATCH, the dispatch pointer field.
            unsafe {
                if v.Anonymous.Anonymous.vt != VT_DISPATCH {
                    return None;
                }
                let dispatch: &Option<IDispatch> = &v.Anonymous.Anonymous.Anonymous.pdispVal;
                dispatch.as_ref()?.cast::<IAccessible>().ok()
            }
        }

        /// Captures the web document of a Chromium top-level window (`Chrome_WidgetWin_1`) through
        /// MSAA + IA2, starting from its `Chrome_RenderWidgetHostHWND` client object.
        pub fn capture_chromium_document(top_level: HWND) -> Result<Ia2Node, Ia2CaptureError> {
            Ok(read_node(&chromium_document(top_level)?, 0))
        }

        /// One object of a Chromium page with its subtree, by the child id an event names
        /// (the object's negative IA2 uniqueID), such as a live region that changed.
        pub fn capture_chromium_object(
            top_level: HWND,
            child_id: i32,
        ) -> Result<Ia2Node, Ia2CaptureError> {
            let document = chromium_document(top_level)?;
            let child = VARIANT::from(child_id);
            // SAFETY: MSAA child lookup on a valid IAccessible; Chromium returns an object.
            let object: IAccessible = unsafe { document.get_accChild(&child) }
                .map_err(|e| err("accChild", e))?
                .cast()
                .map_err(|e| err("IAccessible of the object", e))?;
            Ok(read_node(&object, 0))
        }

        /// What to do with a live object.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Ia2Action {
            /// Move keyboard focus to it (`accSelect(SELFLAG_TAKEFOCUS)`).
            Focus,
            /// Its default action: click a button or link, toggle a check box
            /// (`accDoDefaultAction`).
            DefaultAction,
        }

        /// Whether a top-level window holds a Chromium page: a `Chrome_RenderWidgetHostHWND`.
        pub fn has_chromium_document(top_level: HWND) -> bool {
            let class: Vec<u16> = "Chrome_RenderWidgetHostHWND\0".encode_utf16().collect();
            // SAFETY: the class name is NUL-terminated and outlives the call.
            unsafe {
                FindWindowExW(
                    Some(top_level),
                    None,
                    PCWSTR(class.as_ptr()),
                    PCWSTR::null(),
                )
            }
            .is_ok()
        }

        /// Performs `action` on the object with IA2 `unique_id` in the document of a Chromium
        /// top-level window, through `accChild` on its document with the (negative) uniqueID.
        pub fn perform_chromium_action(
            top_level: HWND,
            unique_id: i32,
            action: Ia2Action,
        ) -> Result<(), Ia2CaptureError> {
            let document = chromium_document(top_level)?;
            // Chromium hands out negative uniqueIDs, which are valid child ids as they are;
            // a positive one is negated, as the IA2 spec describes.
            let child_id = if unique_id < 0 { unique_id } else { -unique_id };
            let child = VARIANT::from(child_id);
            // SAFETY: MSAA child lookup on a valid IAccessible; Chromium returns an object.
            let target: IAccessible = unsafe { document.get_accChild(&child) }
                .map_err(|e| err("accChild", e))?
                .cast()
                .map_err(|e| err("IAccessible of the target", e))?;
            let me = self_child();
            // SAFETY: plain MSAA calls on a valid IAccessible.
            unsafe {
                match action {
                    // SELFLAG_TAKEFOCUS (oleacc.h).
                    Ia2Action::Focus => target.accSelect(0x1, &me),
                    Ia2Action::DefaultAction => target.accDoDefaultAction(&me),
                }
            }
            .map_err(|e| err("MSAA action", e))
        }

        /// The caret of the focused object in a Chromium window's page: `(caret offset,
        /// text of the IA2 boundary unit at the caret)`. `IA2_TEXT_BOUNDARY_CHAR` (0) gives
        /// the character after the caret, `_WORD` (1) the word, `_LINE` (4) the line. Returns
        /// `Ok(None)` when the focused object has no text.
        pub fn chromium_caret_text(
            top_level: HWND,
            boundary: i32,
        ) -> Result<Option<(i32, String)>, Ia2CaptureError> {
            let document = chromium_document(top_level)?;
            // SAFETY: MSAA focus query on a valid IAccessible.
            let focus = unsafe { document.accFocus() }.map_err(|e| err("accFocus", e))?;
            let focused = match dispatch_child(&focus) {
                Some(object) => object,
                // The document itself has focus (VT_I4 CHILDID_SELF) or nothing does.
                None => document,
            };
            let Some(ia2) = ia2_of(&focused) else {
                return Ok(None);
            };
            let Ok(text) = ia2.cast::<IAccessibleText>() else {
                return Ok(None);
            };
            let mut caret = 0i32;
            // SAFETY: out-pointer to a local on a valid interface.
            unsafe { text.get_caretOffset(&mut caret) }
                .ok()
                .map_err(|e| err("caretOffset", e))?;
            let (mut start, mut end) = (0i32, 0i32);
            let mut unit = BSTR::default();
            // SAFETY: as above; BSTR ownership transfers to `unit`.
            let read = unsafe {
                text.get_textAtOffset(caret.max(0), boundary, &mut start, &mut end, &mut unit)
            };
            // At the end of the text there is nothing at the caret (E_INVALIDARG/S_FALSE).
            let unit = if read.is_ok() {
                unit.to_string()
            } else {
                String::new()
            };
            Ok(Some((caret, unit)))
        }

        /// The focused object's text in a Chromium window's page, with its caret and first
        /// selection. `Ok(None)` when the focused object has no text.
        pub fn chromium_text_state(
            top_level: HWND,
        ) -> Result<Option<Ia2TextState>, Ia2CaptureError> {
            ChromiumTextField::focused(top_level)?
                .map(|field| field.state())
                .transpose()
        }

        /// The focused text object of a Chromium window's page, kept to be read again: while
        /// a key takes effect its caret is polled through it, without finding the focus anew
        /// (a window's accessible object and its focus cost milliseconds; a caret, a call).
        pub struct ChromiumTextField(IAccessibleText);

        impl ChromiumTextField {
            /// The focused object, when it has text. `Err` when the window is not Chromium.
            pub fn focused(top_level: HWND) -> Result<Option<Self>, Ia2CaptureError> {
                let document = chromium_document(top_level)?;
                // SAFETY: MSAA focus query on a valid IAccessible.
                let focus = unsafe { document.accFocus() }.map_err(|e| err("accFocus", e))?;
                let focused = dispatch_child(&focus).unwrap_or(document);
                Ok(ia2_of(&focused)
                    .and_then(|ia2| ia2.cast::<IAccessibleText>().ok())
                    .map(Self))
            }

            /// Its caret, first selection and length, in UTF-16 code units: enough to see
            /// that a key moved the caret or changed the text.
            pub fn position(&self) -> Result<Ia2TextPosition, Ia2CaptureError> {
                let (mut caret, mut length) = (0i32, 0i32);
                // SAFETY: out-pointers to locals on a valid interface.
                unsafe {
                    self.0
                        .get_caretOffset(&mut caret)
                        .ok()
                        .map_err(|e| err("caretOffset", e))?;
                    self.0
                        .get_nCharacters(&mut length)
                        .ok()
                        .map_err(|e| err("nCharacters", e))?;
                }
                Ok(Ia2TextPosition {
                    caret: caret.max(0),
                    selection: self.selection(),
                    length,
                })
            }

            /// Whether its text holds other objects (an [`EMBEDDED_OBJECT_CHAR`] for each): an
            /// editing host whose paragraphs are elements of their own (Google Docs' document
            /// with braille support), whose own text is those characters, not what it shows.
            /// Its start and the character at its caret are checked, so a long plain field is
            /// not read whole for it.
            pub fn is_compound(&self) -> bool {
                let text = |start: i32, end: i32| -> Option<String> {
                    let mut part = BSTR::default();
                    // SAFETY: out-pointer to a local on a valid interface; BSTR ownership
                    // transfers to `part`.
                    unsafe { self.0.get_text(start, end, &mut part) }
                        .ok()
                        .ok()?;
                    Some(part.to_string())
                };
                let (mut caret, mut length) = (0i32, 0i32);
                // SAFETY: out-pointers to locals on a valid interface.
                let read = unsafe {
                    self.0.get_caretOffset(&mut caret).is_ok()
                        && self.0.get_nCharacters(&mut length).is_ok()
                };
                if !read || length <= 0 {
                    return false;
                }
                let embedded =
                    |part: Option<String>| part.is_some_and(|p| p.contains(EMBEDDED_OBJECT_CHAR));
                embedded(text(0, length.min(256)))
                    || (caret >= 0 && caret < length && embedded(text(caret, caret + 1)))
            }

            /// The IA2 object attributes of the paragraph its caret is in (`text-align`), for a
            /// field whose paragraphs are objects of their own ([`Self::is_compound`]): the
            /// caret is on the paragraph's embedded object character, and the paragraph is the
            /// field's child object of that rank (its static text children have no such
            /// character). Otherwise the field's own attributes.
            pub fn caret_paragraph_attributes(
                &self,
            ) -> Option<std::collections::BTreeMap<String, String>> {
                const ROLE_SYSTEM_STATICTEXT: i32 = 0x29;
                let field: IAccessible = self.0.cast().ok()?;
                let text = |start: i32, end: i32| -> Option<String> {
                    let mut part = BSTR::default();
                    // SAFETY: out-pointer to a local on a valid interface; BSTR ownership
                    // transfers to `part`.
                    unsafe { self.0.get_text(start, end, &mut part) }
                        .ok()
                        .ok()?;
                    Some(part.to_string())
                };
                let mut caret = 0i32;
                // SAFETY: out-pointer to a local on a valid interface.
                unsafe { self.0.get_caretOffset(&mut caret) }.ok().ok()?;
                let on_object =
                    text(caret, caret + 1).is_some_and(|c| c.starts_with(EMBEDDED_OBJECT_CHAR));
                if !on_object {
                    return object_attributes(&field);
                }
                let rank = text(0, caret)?
                    .chars()
                    .filter(|&c| c == EMBEDDED_OBJECT_CHAR)
                    .count();
                // SAFETY: valid IAccessible.
                let count = unsafe { field.accChildCount() }.unwrap_or(0).max(0);
                let mut children = vec![VARIANT::default(); usize::try_from(count).ok()?];
                let mut obtained = 0i32;
                // SAFETY: the slice length is the requested child count.
                unsafe { AccessibleChildren(&field, 0, &mut children, &mut obtained) }.ok()?;
                let paragraph = children
                    .iter()
                    .take(usize::try_from(obtained).unwrap_or(0))
                    .filter_map(dispatch_child)
                    .filter(|child| {
                        // SAFETY: MSAA role query on a valid child for itself.
                        let role = unsafe { child.get_accRole(&self_child()) }
                            .ok()
                            .and_then(|v| i32::try_from(&v).ok());
                        role != Some(ROLE_SYSTEM_STATICTEXT)
                    })
                    .nth(rank)?;
                object_attributes(&paragraph)
            }

            /// Its whole text, with its caret and first selection.
            pub fn state(&self) -> Result<Ia2TextState, Ia2CaptureError> {
                let mut all = BSTR::default();
                let mut caret = 0i32;
                // SAFETY: out-pointers to locals on a valid interface; BSTR ownership transfers.
                unsafe {
                    self.0
                        .get_text(0, IA2_TEXT_OFFSET_LENGTH, &mut all)
                        .ok()
                        .map_err(|e| err("text", e))?;
                    self.0
                        .get_caretOffset(&mut caret)
                        .ok()
                        .map_err(|e| err("caretOffset", e))?;
                }
                let text = all.to_string();
                let misspelled = self.misspelled(text.encode_utf16().count());
                Ok(Ia2TextState {
                    text,
                    caret: caret.max(0),
                    selection: self.selection(),
                    misspelled,
                })
            }

            /// The runs of its text marked as spelling errors (`invalid:spelling`), in UTF-16
            /// code units; at most 256 runs are read.
            fn misspelled(&self, length: usize) -> Vec<(i32, i32)> {
                const MAX_RUNS: usize = 256;
                let length = i32::try_from(length).unwrap_or(i32::MAX);
                let mut ranges = Vec::new();
                let mut offset = 0i32;
                for _ in 0..MAX_RUNS {
                    if offset >= length {
                        break;
                    }
                    let (mut start, mut end) = (0i32, 0i32);
                    let mut attrs = BSTR::default();
                    // SAFETY: out-pointers to locals on a valid interface; BSTR ownership
                    // transfers to `attrs`.
                    let read = unsafe {
                        self.0
                            .get_attributes(offset, &mut start, &mut end, &mut attrs)
                    };
                    if read.is_err() || end <= offset {
                        break;
                    }
                    let spelling = parse_attributes(&attrs.to_string())
                        .get("invalid")
                        .is_some_and(|kind| kind.split(',').any(|k| k.trim() == "spelling"));
                    if spelling {
                        ranges.push((start, end));
                    }
                    offset = end;
                }
                ranges
            }

            fn selection(&self) -> Option<(i32, i32)> {
                let mut selections = 0i32;
                let (mut start, mut end) = (0i32, 0i32);
                // SAFETY: out-pointers to locals on a valid interface.
                unsafe {
                    let _ = self.0.get_nSelections(&mut selections);
                    (selections > 0
                        && self.0.get_selection(0, &mut start, &mut end).is_ok()
                        && start < end)
                        .then_some((start, end))
                }
            }
        }

        /// The MSAA client object of a Chromium window's web content.
        fn chromium_document(top_level: HWND) -> Result<IAccessible, Ia2CaptureError> {
            let class: Vec<u16> = "Chrome_RenderWidgetHostHWND\0".encode_utf16().collect();
            // Every tab can keep its own render window; the active tab's is the visible one.
            let mut first = None;
            let mut visible = None;
            let mut after: Option<HWND> = None;
            for _ in 0..MAX_RENDER_WINDOWS {
                // SAFETY: the class name is NUL-terminated and outlives the call.
                let Ok(found) = (unsafe {
                    FindWindowExW(
                        Some(top_level),
                        after,
                        PCWSTR(class.as_ptr()),
                        PCWSTR::null(),
                    )
                }) else {
                    break;
                };
                first.get_or_insert(found);
                // SAFETY: plain Win32 query on a window handle.
                if unsafe { IsWindowVisible(found) }.as_bool() {
                    visible = Some(found);
                    break;
                }
                after = Some(found);
            }
            let render = visible.or(first).ok_or_else(|| {
                Ia2CaptureError("no Chrome_RenderWidgetHostHWND in the window".to_owned())
            })?;
            let mut raw = std::ptr::null_mut();
            // SAFETY: requests IAccessible for the client object of a valid window.
            unsafe {
                AccessibleObjectFromWindow(render, OBJID_CLIENT, &IAccessible::IID, &mut raw)
            }
            .map_err(|e| err("AccessibleObjectFromWindow", e))?;
            if raw.is_null() {
                return Err(Ia2CaptureError(
                    "no accessible object for the document".to_owned(),
                ));
            }
            // SAFETY: AccessibleObjectFromWindow returned an owned IAccessible pointer.
            Ok(unsafe { IAccessible::from_raw(raw) })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ia2_object_attributes_with_escapes() {
        let attrs =
            parse_attributes(r"tag:h1;display:block;xml-roles:heading;level:1;src:a\:b\;c;");
        assert_eq!(attrs.get("tag").map(String::as_str), Some("h1"));
        assert_eq!(attrs.get("display").map(String::as_str), Some("block"));
        assert_eq!(attrs.get("level").map(String::as_str), Some("1"));
        assert_eq!(attrs.get("src").map(String::as_str), Some("a:b;c"));
        assert_eq!(attrs.len(), 5);
    }
}
