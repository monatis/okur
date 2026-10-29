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
use windows::Win32::System::Com::{
    IDispatch, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT,
    DISPPARAMS,
};
use windows::Win32::System::Variant::{VARIANT, VT_DISPATCH};
use windows::Win32::UI::Accessibility::AccessibleObjectFromWindow;

/// `OBJID_NATIVEOM`: the window's application object model.
const OBJID_NATIVEOM: u32 = 0xFFFF_FFF0;
/// `LOCALE_USER_DEFAULT`, for property names, which Word takes in English whatever its locale.
const LOCALE_USER_DEFAULT: u32 = 0x0400;
/// `DISPID_PROPERTYPUT`: the named argument a property's new value is passed as.
const DISPID_PROPERTYPUT: i32 = -3;

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

/// A field of Word's document, as Word keeps it: its type (`WdFieldType`: 88 a hyperlink, 70
/// a text form field, 71 a check box), its code (" HYPERLINK \"https://example.org/\" ",
/// " FORMTEXT ") and the text it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WordField {
    pub kind: i32,
    pub code: String,
    pub result: String,
}

/// The fields of the document in Word's document window `document`, in the document's order;
/// `None` where Word's object model does not answer.
pub fn document_fields(document: HWND) -> Option<Vec<WordField>> {
    let fields = object(&property(
        &object(&property(&word_window(document)?, "Document")?)?,
        "Fields",
    )?)?;
    let count = i32::try_from(&property(&fields, "Count")?).ok()?;
    let mut found = Vec::new();
    // Word's collections count from 1; a document with many fields is read in part.
    for index in 1..=count.clamp(0, MAX_FIELDS) {
        let Some(field) = item(&fields, index) else {
            continue;
        };
        let text = |name: &str| {
            property(&field, name)
                .as_ref()
                .and_then(object)
                .and_then(|range| property(&range, "Text"))
                .and_then(|text| windows::core::BSTR::try_from(&text).ok())
                .map(|text| text.to_string())
                .unwrap_or_default()
        };
        found.push(WordField {
            kind: property(&field, "Type")
                .and_then(|kind| i32::try_from(&kind).ok())
                .unwrap_or(0),
            code: text("Code"),
            result: text("Result"),
        });
    }
    Some(found)
}

/// Selects the document's field `index` (from 0, as [`document_fields`] lists them), which puts
/// the caret on it. False where Word's object model does not answer.
pub fn select_field(document: HWND, index: usize) -> bool {
    let select = || -> Option<()> {
        let window = word_window(document)?;
        let fields = object(&property(
            &object(&property(&window, "Document")?)?,
            "Fields",
        )?)?;
        let field = item(&fields, i32::try_from(index).ok()?.checked_add(1)?)?;
        invoke(&field, "Select", DISPATCH_METHOD, &mut [])?;
        Some(())
    };
    select().is_some()
}

/// A legacy form field of Word's, as its object model describes it: its type (`WdFieldType`: 70
/// a text field, 71 a check box, 83 a drop-down), its bookmark name, the text it shows, a check
/// box's value, a drop-down's entries and the one chosen (from 1), and its help and status
/// texts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormField {
    pub kind: i32,
    pub name: String,
    pub result: String,
    pub checked: Option<bool>,
    pub entries: Vec<String>,
    pub chosen: Option<i32>,
    pub help: String,
    pub status: String,
}

/// A form field of the selection's paragraph with its place there: the characters of the
/// paragraph's text before it, and the characters it shows (a text field's result; a check box
/// and a drop-down show none in the text UI Automation reads). Read lean, for every line read:
/// its type, name and result, a check box's value, a drop-down's chosen entry alone (in
/// `entries`, at `chosen`'s place, the others empty), and no help or status text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedFormField {
    pub field: FormField,
    pub before: usize,
    pub length: usize,
}

/// Whether the document in Word's document window `document` has form fields at all
/// (`Document.FormFields.Count`): a document without them is looked through no further.
pub fn has_form_fields(document: HWND) -> Option<bool> {
    let document = object(&property(&word_window(document)?, "Document")?)?;
    let fields = object(&property(&document, "FormFields")?)?;
    Some(number(&fields, "Count")? > 0)
}

/// The form fields of the selection's paragraph in Word's document window `document`, in
/// order, each placed in the paragraph's text; `None` where Word's object model does not
/// answer.
pub fn paragraph_form_fields(document: HWND) -> Option<Vec<PlacedFormField>> {
    let window = word_window(document)?;
    let whole = object(&property(&window, "Document")?)?;
    let selection = object(&property(&window, "Selection")?)?;
    let paragraph = item(&object(&property(&selection, "Paragraphs")?)?, 1)?;
    let range = object(&property(&paragraph, "Range")?)?;
    let start = number(&range, "Start")?;
    let fields = object(&property(&range, "FormFields")?)?;
    let count = number(&fields, "Count").unwrap_or(0);
    Some(
        (1..=count.clamp(0, MAX_FIELDS))
            .filter_map(|index| item(&fields, index))
            .filter_map(|field| {
                let at = number(&object(&property(&field, "Range")?)?, "Start")?;
                let before = object(&invoke(
                    &whole,
                    "Range",
                    DISPATCH_METHOD,
                    &mut [VARIANT::from(start), VARIANT::from(at)],
                )?)?;
                let before = text(&before, "Text").chars().count();
                let field = lean_form_field(&field);
                let length = if field.kind == TEXT_INPUT {
                    field.result.chars().count()
                } else {
                    0
                };
                Some(PlacedFormField {
                    field,
                    before,
                    length,
                })
            })
            .collect(),
    )
}

/// The form fields in the selection of Word's document window `document` (one, where Tab put
/// it in a protected form), then those of the selection's paragraph. `None` where Word's object
/// model does not answer.
pub fn selection_form_fields(document: HWND) -> Option<(Vec<FormField>, Vec<FormField>)> {
    let selection = object(&property(&word_window(document)?, "Selection")?)?;
    let selected = form_fields(&object(&property(&selection, "FormFields")?)?);
    let paragraph = item(&object(&property(&selection, "Paragraphs")?)?, 1)
        .and_then(|paragraph| object(&property(&paragraph, "Range")?))
        .and_then(|range| object(&property(&range, "FormFields")?))
        .map(|fields| form_fields(&fields))
        .unwrap_or_default();
    Some((selected, paragraph))
}

/// The form fields of a `FormFields` collection, in order.
fn form_fields(collection: &IDispatch) -> Vec<FormField> {
    let count = number(collection, "Count").unwrap_or(0);
    (1..=count.clamp(0, MAX_FIELDS))
        .filter_map(|index| item(collection, index))
        .map(|field| form_field(&field))
        .collect()
}

/// A `FormField` object with what a line read says of it, in as few calls into Word as can
/// be: each is a call into another process.
fn lean_form_field(field: &IDispatch) -> FormField {
    let kind = number(field, "Type").unwrap_or(0);
    let (checked, entries, chosen) = match kind {
        CHECK_BOX => (
            object(&property(field, "CheckBox").unwrap_or_default())
                .and_then(|check| property(&check, "Value"))
                .and_then(|value| bool::try_from(&value).ok()),
            Vec::new(),
            None,
        ),
        DROP_DOWN => {
            let list = object(&property(field, "DropDown").unwrap_or_default());
            let chosen = list.as_ref().and_then(|list| number(list, "Value"));
            let entries = list
                .as_ref()
                .and_then(|list| object(&property(list, "ListEntries")?));
            let count = entries
                .as_ref()
                .and_then(|entries| number(entries, "Count"))
                .unwrap_or(0);
            let name = entries
                .as_ref()
                .zip(chosen)
                .and_then(|(entries, chosen)| item(entries, chosen))
                .map(|entry| text(&entry, "Name"))
                .unwrap_or_default();
            // The chosen entry at its place, the others unread.
            let mut all =
                vec![String::new(); usize::try_from(count.clamp(0, MAX_FIELDS)).unwrap_or(0)];
            if let Some(slot) = chosen
                .and_then(|c| usize::try_from(c).ok()?.checked_sub(1))
                .and_then(|at| all.get_mut(at))
            {
                *slot = name;
            }
            (None, all, chosen)
        }
        _ => (None, Vec::new(), None),
    };
    FormField {
        kind,
        name: text(field, "Name"),
        result: if kind == TEXT_INPUT {
            text(field, "Result")
        } else {
            String::new()
        },
        checked,
        entries,
        chosen,
        help: String::new(),
        status: String::new(),
    }
}

/// A `FormField` object as Word describes it.
fn form_field(field: &IDispatch) -> FormField {
    let kind = number(field, "Type").unwrap_or(0);
    let check_box = (kind == CHECK_BOX)
        .then(|| object(&property(field, "CheckBox")?))
        .flatten();
    let drop_down = (kind == DROP_DOWN)
        .then(|| object(&property(field, "DropDown")?))
        .flatten();
    let entries = drop_down
        .as_ref()
        .and_then(|list| object(&property(list, "ListEntries")?))
        .map(|entries| {
            let count = number(&entries, "Count").unwrap_or(0);
            (1..=count.clamp(0, MAX_FIELDS))
                .filter_map(|index| item(&entries, index))
                .map(|entry| text(&entry, "Name"))
                .collect()
        })
        .unwrap_or_default();
    FormField {
        kind,
        name: text(field, "Name"),
        result: text(field, "Result"),
        checked: check_box
            .as_ref()
            .and_then(|check| property(check, "Value"))
            .and_then(|value| bool::try_from(&value).ok()),
        entries,
        chosen: drop_down.as_ref().and_then(|list| number(list, "Value")),
        help: text(field, "HelpText"),
        status: text(field, "StatusText"),
    }
}

/// The text property `name` of `object`; empty where it has none.
fn text(object: &IDispatch, name: &str) -> String {
    property(object, name)
        .and_then(|text| windows::core::BSTR::try_from(&text).ok())
        .map(|text| text.to_string())
        .unwrap_or_default()
}

/// The number property `name` of `object`.
fn number(object: &IDispatch, name: &str) -> Option<i32> {
    property(object, name).and_then(|value| i32::try_from(&value).ok())
}

/// `WdFieldType` of a text, a check box and a drop-down form field.
pub const TEXT_INPUT: i32 = 70;
pub const CHECK_BOX: i32 = 71;
pub const DROP_DOWN: i32 = 83;

/// Whether the document in Word's document window `document` tracks its changes
/// (`Document.TrackRevisions`, Ctrl+Shift+E's); `None` where Word's object model does not
/// answer.
pub fn track_revisions(document: HWND) -> Option<bool> {
    let document = object(&property(&word_window(document)?, "Document")?)?;
    bool::try_from(&property(&document, "TrackRevisions")?).ok()
}

/// Marks the document in Word's document window `document` saved (`Document.Saved = True`),
/// so that closing its window asks nothing: the harness's, for a fixture a scenario changed
/// (a form filled in). False where Word's object model does not answer.
pub fn mark_saved(document: HWND) -> bool {
    let mark = || -> Option<()> {
        let document = object(&property(&word_window(document)?, "Document")?)?;
        invoke(
            &document,
            "Saved",
            DISPATCH_PROPERTYPUT,
            &mut [VARIANT::from(true)],
        )?;
        Some(())
    };
    mark().is_some()
}

/// At most this many of a document's fields are listed.
const MAX_FIELDS: i32 = 2000;

/// The item `index` (from 1) of a Word collection.
fn item(collection: &IDispatch, index: i32) -> Option<IDispatch> {
    let flags = DISPATCH_FLAGS(DISPATCH_METHOD.0 | DISPATCH_PROPERTYGET.0);
    object(&invoke(
        collection,
        "Item",
        flags,
        &mut [VARIANT::from(index)],
    )?)
}

/// The property `name` of `object`, read by late binding.
fn property(object: &IDispatch, name: &str) -> Option<VARIANT> {
    invoke(object, name, DISPATCH_PROPERTYGET, &mut [])
}

/// A late-bound call of `name` on `object` with `arguments` (in order; one at most here); a
/// property put passes its one argument as the property's new value.
fn invoke(
    object: &IDispatch,
    name: &str,
    flags: DISPATCH_FLAGS,
    arguments: &mut [VARIANT],
) -> Option<VARIANT> {
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
    // IDispatch takes its arguments last first.
    arguments.reverse();
    let mut named = DISPID_PROPERTYPUT;
    let put = flags == DISPATCH_PROPERTYPUT;
    let parameters = DISPPARAMS {
        rgvarg: if arguments.is_empty() {
            null_mut()
        } else {
            arguments.as_mut_ptr()
        },
        cArgs: u32::try_from(arguments.len()).ok()?,
        rgdispidNamedArgs: if put { &mut named } else { null_mut() },
        cNamedArgs: u32::from(put),
    };
    // SAFETY: the arguments, the named argument and the result are locals alive for the call.
    unsafe {
        object.Invoke(
            id,
            &GUID::zeroed(),
            LOCALE_USER_DEFAULT,
            flags,
            &parameters,
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
