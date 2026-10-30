//! Word's own object model, reached through its document window (`OBJID_NATIVEOM`), for what
//! its UI Automation does not give: a paragraph's line spacing, which JAWS's Say Font says
//! ("Line spacing: Double" [obs, 1040/s17]). Windows hands the running Word's `Window` object
//! to an assistive technology through the document window, as NVDA's Word support takes it;
//! nothing is started, so Word's activation state does not stand in the way, as it did for a
//! new automation instance (the fixtures' writer, 2026-10-24). Calls are cross-process: made off
//! the keys' path.

use std::ptr::null_mut;

use windows::core::{w, Interface, GUID, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, SYSTEMTIME};
use windows::Win32::Globalization::{GetDateFormatEx, ENUM_DATE_FORMATS_FLAGS};
use windows::Win32::System::Com::{
    IDispatch, DISPATCH_FLAGS, DISPATCH_METHOD, DISPATCH_PROPERTYGET, DISPATCH_PROPERTYPUT,
    DISPPARAMS,
};
use windows::Win32::System::Variant::{VariantTimeToSystemTime, VARIANT, VT_DATE, VT_DISPATCH};
use windows::Win32::UI::Accessibility::AccessibleObjectFromWindow;

use okur_core::{NoteKind, RevisionKind};

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

/// A comment on the selection's paragraph: how many of the paragraph's characters come before
/// the text it is on, that text's length, the comment's text and its author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedComment {
    pub before: usize,
    pub length: usize,
    pub text: String,
    pub author: String,
}

/// Whether the document in Word's document window `document` has comments at all
/// (`Document.Comments.Count`): a document without them is looked through no further.
pub fn has_comments(document: HWND) -> Option<bool> {
    let document = object(&property(&word_window(document)?, "Document")?)?;
    let comments = object(&property(&document, "Comments")?)?;
    Some(number(&comments, "Count")? > 0)
}

/// The comments on the selection's paragraph in Word's document window `document`, in order,
/// each placed in the paragraph's text; `None` where Word's object model does not answer.
pub fn paragraph_comments(document: HWND) -> Option<Vec<PlacedComment>> {
    let window = word_window(document)?;
    let whole = object(&property(&window, "Document")?)?;
    let selection = object(&property(&window, "Selection")?)?;
    let paragraph = item(&object(&property(&selection, "Paragraphs")?)?, 1)?;
    let range = object(&property(&paragraph, "Range")?)?;
    let start = number(&range, "Start")?;
    let comments = object(&property(&range, "Comments")?)?;
    let count = number(&comments, "Count").unwrap_or(0);
    Some(
        (1..=count.clamp(0, MAX_FIELDS))
            .filter_map(|index| item(&comments, index))
            .filter_map(|comment| {
                let scope = object(&property(&comment, "Scope")?)?;
                let (from, to) = (number(&scope, "Start")?, number(&scope, "End")?);
                if to < from {
                    return None;
                }
                let before = object(&invoke(
                    &whole,
                    "Range",
                    DISPATCH_METHOD,
                    &mut [VARIANT::from(start), VARIANT::from(from.max(start))],
                )?)?;
                let said = object(&property(&comment, "Range")?)?;
                Some(PlacedComment {
                    before: text(&before, "Text").chars().count(),
                    length: text(&scope, "Text").chars().count(),
                    text: text(&said, "Text").trim().to_owned(),
                    author: text(&comment, "Author").trim().to_owned(),
                })
            })
            .collect(),
    )
}

/// Where the selection of Word's document window `document` is on its pages
/// (`Selection.Information`: `wdActiveEndPageNumber` 3, `wdActiveEndSectionNumber` 2,
/// `wdFirstCharacterLineNumber` 10, `wdFirstCharacterColumnNumber` 9); `None` where Word's
/// object model does not answer, or a number is not one (-1 outside a page's view).
pub fn selection_place(document: HWND) -> Option<okur_core::PagePlace> {
    let selection = object(&property(&word_window(document)?, "Selection")?)?;
    let information = |kind: i32| -> Option<u32> {
        let value = invoke(
            &selection,
            "Information",
            DISPATCH_PROPERTYGET,
            &mut [VARIANT::from(kind)],
        )?;
        i32::try_from(&value)
            .ok()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|&n| n > 0)
    };
    Some(okur_core::PagePlace {
        page: information(3)?,
        section: information(2)?,
        line: information(10)?,
        column: information(9)?,
    })
}

/// A revision on the selection's paragraph: how many of the paragraph's characters come before
/// its text, that text's length, the text (a deletion's too, which Word's text holds), what it
/// changed, its author and date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedRevision {
    pub before: usize,
    pub length: usize,
    pub text: String,
    pub kind: RevisionKind,
    pub author: String,
    pub date: String,
}

/// The revisions on the selection's paragraph in Word's document window `document`, in order,
/// each placed in the paragraph's text as its comments are; `None` where Word's object model
/// does not answer.
pub fn paragraph_revisions(document: HWND) -> Option<Vec<PlacedRevision>> {
    let window = word_window(document)?;
    let whole = object(&property(&window, "Document")?)?;
    let selection = object(&property(&window, "Selection")?)?;
    let paragraph = item(&object(&property(&selection, "Paragraphs")?)?, 1)?;
    let range = object(&property(&paragraph, "Range")?)?;
    let start = number(&range, "Start")?;
    let revisions = notes_of(&range, NoteKind::Revision)?;
    let count = number(&revisions, "Count").unwrap_or(0);
    Some(
        (1..=count.clamp(0, MAX_FIELDS))
            .filter_map(|index| item(&revisions, index))
            .filter_map(|revision| {
                let scope = object(&property(&revision, "Range")?)?;
                let (from, to) = (number(&scope, "Start")?, number(&scope, "End")?);
                if to < from {
                    return None;
                }
                let before = object(&invoke(
                    &whole,
                    "Range",
                    DISPATCH_METHOD,
                    &mut [VARIANT::from(start), VARIANT::from(from.max(start))],
                )?)?;
                let said = text(&scope, "Text");
                Some(PlacedRevision {
                    before: text(&before, "Text").chars().count(),
                    length: said.chars().count(),
                    text: said.trim().to_owned(),
                    kind: number(&revision, "Type").map_or(RevisionKind::Other, revision_kind),
                    author: text(&revision, "Author").trim().to_owned(),
                    date: property(&revision, "Date")
                        .as_ref()
                        .and_then(date_of)
                        .and_then(note_date)
                        .unwrap_or_default(),
                })
            })
            .collect(),
    )
}

/// A comment, footnote, endnote, bookmark or revision of Word's document, as JAWS's lists of
/// them show it: its text (a bookmark's, the text it marks; a revision's, the text it inserted
/// or deleted); a comment's or revision's author and its date, as the user's locale writes
/// "ddd, MMM dd, yy" ("Fri, Jan 02, 26" [obs, 1082/s1]); a note's number (`Index`) and its
/// mark as the document shows it ("i" for the first endnote [obs, 1090/s6]); a bookmark's
/// name; what a revision changed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WordNote {
    pub text: String,
    pub author: String,
    pub date: String,
    pub number: Option<u32>,
    pub mark: String,
    pub name: String,
    pub revision: Option<RevisionKind>,
}

/// What a revision changed, from its `WdRevisionType`: 1 text inserted, 2 deleted.
fn revision_kind(kind: i32) -> RevisionKind {
    match kind {
        1 => RevisionKind::Inserted,
        2 => RevisionKind::Deleted,
        _ => RevisionKind::Other,
    }
}

/// The collection of the document's (or a range's) comments, footnotes, endnotes, bookmarks or
/// revisions.
fn notes_of(whole: &IDispatch, kind: NoteKind) -> Option<IDispatch> {
    object(&property(
        whole,
        match kind {
            NoteKind::Comment => "Comments",
            NoteKind::Footnote => "Footnotes",
            NoteKind::Endnote => "Endnotes",
            NoteKind::Bookmark => "Bookmarks",
            NoteKind::Revision => "Revisions",
        },
    )?)
}

/// The document's bookmarks in the document's order (Word's collection is in their names'),
/// each with where it starts: JAWS lists them so, "Opening" before "Borrowing" [obs, 1091/s1].
fn bookmarks(whole: &IDispatch) -> Option<Vec<(i32, WordNote)>> {
    let marks = notes_of(whole, NoteKind::Bookmark)?;
    let count = number(&marks, "Count")?;
    let mut found: Vec<(i32, WordNote)> = (1..=count.clamp(0, MAX_FIELDS))
        .filter_map(|index| item(&marks, index))
        .filter_map(|mark| {
            let range = object(&property(&mark, "Range")?)?;
            Some((
                number(&range, "Start")?,
                WordNote {
                    text: text(&range, "Text").trim().to_owned(),
                    name: text(&mark, "Name").trim().to_owned(),
                    ..WordNote::default()
                },
            ))
        })
        .collect();
    found.sort_by_key(|(start, _)| *start);
    Some(found)
}

/// How the document numbers its footnotes or endnotes: the collection's numbering style
/// (`WdNoteNumberStyle`) and the number of its first note.
fn numbering(whole: &IDispatch, kind: NoteKind) -> (i32, u32) {
    let notes = notes_of(whole, kind);
    let style = notes.as_ref().and_then(|n| number(n, "NumberStyle"));
    let first = notes
        .as_ref()
        .and_then(|n| number(n, "StartingNumber"))
        .and_then(|n| u32::try_from(n).ok());
    (style.unwrap_or(0), first.unwrap_or(1))
}

/// The comments, footnotes, endnotes, bookmarks or revisions of the document in Word's document
/// window `document`, in the document's order; `None` where Word's object model does not
/// answer.
pub fn document_notes(document: HWND, kind: NoteKind) -> Option<Vec<WordNote>> {
    let whole = object(&property(&word_window(document)?, "Document")?)?;
    if kind == NoteKind::Bookmark {
        return Some(bookmarks(&whole)?.into_iter().map(|(_, b)| b).collect());
    }
    let notes = notes_of(&whole, kind)?;
    let count = number(&notes, "Count")?;
    let (style, first) = numbering(&whole, kind);
    Some(
        (1..=count.clamp(0, MAX_FIELDS))
            .filter_map(|index| item(&notes, index))
            .map(|note| {
                let said = property(&note, "Range")
                    .as_ref()
                    .and_then(object)
                    .map(|range| text(&range, "Text"))
                    .unwrap_or_default();
                let mut found = WordNote {
                    text: said.trim().to_owned(),
                    ..WordNote::default()
                };
                if matches!(kind, NoteKind::Comment | NoteKind::Revision) {
                    found.author = text(&note, "Author").trim().to_owned();
                    found.date = property(&note, "Date")
                        .as_ref()
                        .and_then(date_of)
                        .and_then(note_date)
                        .unwrap_or_default();
                    if kind == NoteKind::Revision {
                        found.revision = number(&note, "Type").map(revision_kind);
                    }
                } else {
                    found.number = number(&note, "Index").and_then(|n| u32::try_from(n).ok());
                    found.mark = found
                        .number
                        .map(|n| note_mark(style, (first + n).saturating_sub(1)))
                        .unwrap_or_default();
                }
                found
            })
            .collect(),
    )
}

/// Puts the caret on the mark of the document's comment, footnote, endnote or bookmark `index`
/// (from 0, as [`document_notes`] lists them): where the text a comment is on ends, before its
/// mark; on a note's reference, which Right then moves over; at a bookmark's start [obs,
/// 1087/s6-s8, 1088/s6-s8, 1090/s6-s8, 1091/s9-s11]; at a revision's start. False where Word's
/// object model does not answer.
pub fn select_note(document: HWND, kind: NoteKind, index: usize) -> bool {
    let select = || -> Option<()> {
        let window = word_window(document)?;
        let whole = object(&property(&window, "Document")?)?;
        let start = if kind == NoteKind::Bookmark {
            bookmarks(&whole)?.get(index)?.0
        } else {
            let notes = notes_of(&whole, kind)?;
            let note = item(&notes, i32::try_from(index).ok()?.checked_add(1)?)?;
            let mark = if kind == NoteKind::Revision {
                "Range"
            } else {
                "Reference"
            };
            number(&object(&property(&note, mark)?)?, "Start")?
        };
        let selection = object(&property(&window, "Selection")?)?;
        invoke(
            &selection,
            "SetRange",
            DISPATCH_METHOD,
            &mut [VARIANT::from(start), VARIANT::from(start)],
        )?;
        Some(())
    };
    select().is_some()
}

/// A date Word's object model gave (`VT_DATE`, days since 1899-12-30).
fn date_of(value: &VARIANT) -> Option<f64> {
    // SAFETY: the discriminant read, then for VT_DATE the number it holds.
    unsafe {
        let inner = &value.Anonymous.Anonymous;
        (inner.vt == VT_DATE).then_some(inner.Anonymous.date)
    }
}

/// A comment's date as JAWS's list of comments writes it, "Fri, Jan 02, 26", in the names of
/// the user's locale.
fn note_date(date: f64) -> Option<String> {
    let mut time = SYSTEMTIME::default();
    // SAFETY: converts into a local.
    if unsafe { VariantTimeToSystemTime(date, &mut time) } == 0 {
        return None;
    }
    let mut written = [0u16; 64];
    // SAFETY: the user's locale (no name), the date and the picture, into a local buffer.
    let length = unsafe {
        GetDateFormatEx(
            PCWSTR::null(),
            ENUM_DATE_FORMATS_FLAGS(0),
            Some(&time),
            w!("ddd, MMM dd, yy"),
            Some(&mut written),
            PCWSTR::null(),
        )
    };
    let length = usize::try_from(length).ok()?.checked_sub(1)?;
    Some(String::from_utf16_lossy(written.get(..length)?))
}

/// A note's mark as Word shows it, for its collection's numbering style (`WdNoteNumberStyle`):
/// 1 upper and 2 lower roman ("i", Word's endnotes' own), 3 upper and 4 lower letters repeated
/// past the alphabet ("aa"), 9 symbols repeated past their four ("**"); any other, arabic.
pub fn note_mark(style: i32, number: u32) -> String {
    const SYMBOLS: [char; 4] = ['*', '\u{2020}', '\u{2021}', '\u{a7}'];
    let repeated = |kinds: u32| -> (u32, usize) {
        let past = number.saturating_sub(1);
        (past % kinds, usize::try_from(past / kinds).unwrap_or(0) + 1)
    };
    match style {
        1 => roman(number),
        2 => roman(number).to_lowercase(),
        3 | 4 => {
            let (letter, times) = repeated(26);
            let base = if style == 3 { b'A' } else { b'a' };
            let letter = char::from(base.saturating_add(u8::try_from(letter).unwrap_or(0)));
            std::iter::repeat_n(letter, times).collect()
        }
        9 => {
            let (symbol, times) = repeated(4);
            let symbol = SYMBOLS
                .get(usize::try_from(symbol).unwrap_or(0))
                .copied()
                .unwrap_or('*');
            std::iter::repeat_n(symbol, times).collect()
        }
        _ => number.to_string(),
    }
}

/// `number` in upper-case roman numerals ("XIV"); arabic past what they write (3999).
fn roman(number: u32) -> String {
    const NUMERALS: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    if number == 0 || number > 3999 {
        return number.to_string();
    }
    let mut left = number;
    let mut written = String::new();
    for (value, numeral) in NUMERALS {
        while left >= value {
            written.push_str(numeral);
            left -= value;
        }
    }
    written
}

/// Whether the document in Word's document window `document` has footnotes, or endnotes, at
/// all (`Document.Footnotes.Count`): a document without them is looked through no further.
pub fn has_notes(document: HWND, kind: NoteKind) -> Option<bool> {
    let document = object(&property(&word_window(document)?, "Document")?)?;
    Some(number(&notes_of(&document, kind)?, "Count")? > 0)
}

/// A footnote's or endnote's reference on the selection's paragraph: how many of the
/// paragraph's characters, as UI Automation shows them, come before it, the note's number in
/// the document, its mark and text, and whether the selection starts on the reference itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedNote {
    pub before: usize,
    pub number: u32,
    pub mark: String,
    pub text: String,
    pub at_caret: bool,
    pub selected: bool,
}

/// The footnote or endnote references on the selection's paragraph in Word's document window
/// `document`, in order; `None` where Word's object model does not answer.
pub fn paragraph_notes(document: HWND, kind: NoteKind) -> Option<Vec<PlacedNote>> {
    let window = word_window(document)?;
    let whole = object(&property(&window, "Document")?)?;
    let selection = object(&property(&window, "Selection")?)?;
    let paragraph = item(&object(&property(&selection, "Paragraphs")?)?, 1)?;
    let range = object(&property(&paragraph, "Range")?)?;
    let start = number(&range, "Start")?;
    let caret = number(&selection, "Start");
    let selection_end = number(&selection, "End");
    let notes = notes_of(&range, kind)?;
    let count = number(&notes, "Count").unwrap_or(0);
    let (style, first) = numbering(&whole, kind);
    let span = |from: i32, to: i32| {
        object(&invoke(
            &whole,
            "Range",
            DISPATCH_METHOD,
            &mut [VARIANT::from(from), VARIANT::from(to)],
        )?)
    };
    Some(
        (1..=count.clamp(0, MAX_FIELDS))
            .filter_map(|index| item(&notes, index))
            .filter_map(|note| {
                let at = number(&object(&property(&note, "Reference")?)?, "Start")?;
                // A range's notes count from its own first: the document's before it give the
                // note's number.
                let earlier = number(&notes_of(&span(0, at)?, kind)?, "Count")?;
                let number = u32::try_from(earlier).ok()?.checked_add(1)?;
                let said = property(&note, "Range")
                    .as_ref()
                    .and_then(object)
                    .map(|range| text(&range, "Text"))
                    .unwrap_or_default();
                Some(PlacedNote {
                    before: shown_chars(&text(&span(start, at.max(start))?, "Text")),
                    number,
                    mark: note_mark(style, (first + number).saturating_sub(1)),
                    text: said.trim().to_owned(),
                    at_caret: caret == Some(at),
                    selected: caret.is_some_and(|c| c <= at)
                        && selection_end.is_some_and(|end| end > at),
                })
            })
            .collect(),
    )
}

/// How many of `text`'s characters UI Automation shows: Word's range text holds a mark for
/// each footnote or endnote reference (U+0002) and comment (U+0005), which its UI Automation
/// text does not.
fn shown_chars(text: &str) -> usize {
    text.chars()
        .filter(|c| !matches!(c, '\u{2}' | '\u{5}'))
        .count()
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

/// The font size of the selection in Word's document window `document`, in points
/// (`Selection.Font.Size`); `None` for a selection of several sizes, or where Word's object
/// model does not answer.
pub fn selection_font_size(document: HWND) -> Option<f32> {
    let selection = object(&property(&word_window(document)?, "Selection")?)?;
    let font = object(&property(&selection, "Font")?)?;
    let size = f64::try_from(&property(&font, "Size")?).ok()?;
    // Word's wdUndefined (9999999) for several sizes.
    (size > 0.0 && size < 1639.0).then_some(size as f32)
}

/// How far the selection's paragraph in Word's document window `document` starts from the
/// page's left edge, in points: the page's left margin and the paragraph's left indent
/// (`Selection.PageSetup.LeftMargin`, `Selection.ParagraphFormat.LeftIndent`), as JAWS says an
/// indent ("indent | 1.5 inches" for 0.5 inch on a 1-inch margin [obs, 1080, round-1080c]);
/// `None` where Word's object model does not answer, or for paragraphs of several indents.
pub fn paragraph_left_edge(document: HWND) -> Option<f32> {
    let selection = object(&property(&word_window(document)?, "Selection")?)?;
    let format = object(&property(&selection, "ParagraphFormat")?)?;
    let indent = f64::try_from(&property(&format, "LeftIndent")?).ok()?;
    let setup = object(&property(&selection, "PageSetup")?)?;
    let margin = f64::try_from(&property(&setup, "LeftMargin")?).ok()?;
    // Word's wdUndefined (9999999) for several indents or margins.
    let edge = indent + margin;
    (indent.abs() < 1584.0 && margin.abs() < 1584.0).then_some(edge as f32)
}

/// The alignment of the selection's paragraph in Word's document window `document`
/// (`Selection.ParagraphFormat.Alignment`: 0 left, 1 centered, 2 right, 3 justified); `None`
/// where Word's object model does not answer.
pub fn paragraph_alignment(document: HWND) -> Option<i32> {
    let selection = object(&property(&word_window(document)?, "Selection")?)?;
    let format = object(&property(&selection, "ParagraphFormat")?)?;
    i32::try_from(&property(&format, "Alignment")?).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A note's mark as Word numbers it: endnotes in lower-case roman ("i" [obs, 1090/s6]),
    /// footnotes in arabic, letters and symbols repeated past their set.
    #[test]
    fn note_marks_follow_the_numbering_style() {
        assert_eq!(note_mark(0, 1), "1");
        assert_eq!(note_mark(2, 1), "i");
        assert_eq!(note_mark(2, 14), "xiv");
        assert_eq!(note_mark(1, 1999), "MCMXCIX");
        assert_eq!(note_mark(4, 2), "b");
        assert_eq!(note_mark(3, 28), "BB");
        assert_eq!(note_mark(9, 1), "*");
        assert_eq!(note_mark(9, 6), "\u{2020}\u{2020}");
        assert_eq!(note_mark(14, 3), "3");
    }
}
