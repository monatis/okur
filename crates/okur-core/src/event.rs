pub use crate::aom::{NodeStates, Role};
use serde::{Deserialize, Serialize};

/// Politeness level for ARIA live regions matching WAI-ARIA specifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Politeness {
    #[default]
    Off,
    Polite,
    Assertive,
}

/// A control's text at a point of the screen by unit, as the control gives it (UI
/// Automation's text range at the point, expanded): what mouse echo says, as its unit chooses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnitTexts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph: Option<String>,
    /// Where the point is in `line`, in characters: the word and the character there are
    /// then found as Okur splits words, not as the control's own units give them (Notepad's
    /// word unit gave "{\r\n    " in a line's indentation; old-pc, 2026-10-22).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

impl UnitTexts {
    /// The text of `unit`: a line or paragraph without its indentation and line break, the
    /// word or the character at the point's column of the line.
    pub fn of(&self, unit: crate::settings::MouseEchoUnit) -> Option<String> {
        use crate::settings::MouseEchoUnit;
        let line = self
            .line
            .as_deref()
            .map(|l| l.trim_end_matches(['\r', '\n']));
        let at = line.zip(self.column);
        match unit {
            MouseEchoUnit::Character => match at {
                Some((line, column)) => line.chars().nth(column).map(String::from),
                None => self.character.clone(),
            },
            MouseEchoUnit::Word => match at {
                Some((line, column)) => {
                    crate::editing::web_word_at(line, column).map(|(start, end)| {
                        line.chars()
                            .skip(start)
                            .take(end.saturating_sub(start))
                            .collect()
                    })
                }
                None => self.word.as_deref().map(|w| w.trim().to_owned()),
            },
            MouseEchoUnit::Line => line.map(|l| l.trim().to_owned()),
            MouseEchoUnit::Paragraph => self.paragraph.as_deref().map(|p| p.trim().to_owned()),
        }
        .filter(|text| !text.is_empty())
    }
}

/// What the mouse points to, at `x`, `y` in screen pixels: the control there, as a focus
/// on it would be described, and its text there by unit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MousePoint {
    pub x: i32,
    pub y: i32,
    pub control: Option<FocusEvent>,
    pub texts: UnitTexts,
}

/// The focused text field's text, caret and selection, as read through the platform.
/// Offsets are in characters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextContext {
    pub text: String,
    pub caret: usize,
    /// The selection as `(start, end)`, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<(usize, usize)>,
    /// The font at the caret, when the field reports one (Say Font).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<FontInfo>,
    /// The word around the caret as `(start, end)`, by the field's own word unit (UIA), where
    /// its words differ from runs between spaces: Windows 11 Notepad stops Ctrl+Right at a
    /// comma, which JAWS reads as a word of its own ("line", ",", "with"; dev box). `None`
    /// where the field reports no word unit (Win32 edits): words then end at whitespace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub word: Option<(usize, usize)>,
    /// The words the field marks as misspelled, as `(start, end)` (Chromium's
    /// `invalid:spelling` text attribute): JAWS says "Misspelled" with them [obs, 284/s6, s9].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub misspelled: Vec<(usize, usize)>,
    /// What a word processor says of the caret's line, where it marks its structure (Word).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structure: Option<LineStructure>,
}

/// What a word processor says of the line at the caret through its text's attributes and
/// elements (Word's UI Automation): its own line, the paragraph's heading level and list, and
/// the table cell. JAWS reads Word by them: "heading level 1 Library Hours", "Entering List",
/// "table 1 uniform Table new cell Branch" [obs, 518].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineStructure {
    /// The caret's line as `(start, end)` by the field's own line unit: Word's ends at a
    /// table cell's end, where its text runs the cells together [obs, 518/s13].
    pub line: (usize, usize),
    /// The paragraph's heading level, 1 to 9, from its style.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<u8>,
    /// The paragraph is an item of a bulleted or numbered list.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub list_item: bool,
    /// The table cell holding the caret.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<TableCell>,
    /// The form fields on the caret's paragraph (Word's legacy ones), placed in the text.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub form_fields: Vec<DocumentFormField>,
    /// The links on the caret's line, each as `(start, end)` in the text, which a line read
    /// marks: "See the link opening hours" [obs, 1055/s1].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<(usize, usize)>,
    /// The comments on the caret's paragraph (Word's), placed in the text: a line read marks
    /// where one begins, "has comment" [obs, 1081/s1].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<DocumentComment>,
    /// The footnote references on the caret's paragraph (Word's), each where it stands in the
    /// text, which shows none of it, and its number: a line read says it there, "every day
    /// footnote 1." [obs, 1083/s4].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub footnotes: Vec<DocumentFootnote>,
    /// The endnote references on the caret's paragraph, as its footnotes: "three weeks end note
    /// i." [obs, 1090/s7].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endnotes: Vec<DocumentFootnote>,
    /// The revisions on the caret's paragraph (Word's tracked changes), placed in the text. A
    /// deletion's text is in Word's text, which its view does not show and JAWS does not read:
    /// "The library opens at eight in the morning." [obs, 1106/s1].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revisions: Vec<DocumentRevision>,
    /// Word tracks the document's changes: JAWS then marks its revisions as it reads, "eight |
    /// Inserted" [obs, 1108/s7], which it does not without [obs, 1106/s6].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub tracking: bool,
    /// Where the caret is on Word's pages: JAWS says a page and a section reached, "page 3 |
    /// Section 2" [obs, 1112/s5], and Insert+Delete the line and column [obs, 1112/s7].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<PagePlace>,
}

/// Which of JAWS's lists of a spreadsheet's (Excel's): its cells with formulas (Insert+Shift+F),
/// the active cell's column's cells with data (Insert+Shift+C), or its worksheets
/// (Ctrl+Shift+S).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SheetListKind {
    Formulas,
    Column,
    Worksheets,
}

impl SheetListKind {
    /// The request's name for reading them, in a log and in replay.
    pub fn read_name(self) -> &'static str {
        match self {
            Self::Formulas => "sheet formulas",
            Self::Column => "sheet column",
            Self::Worksheets => "worksheets",
        }
    }
}

/// An entry of one of those lists, as JAWS's list shows it ("B4: =SUM(B2:B3)", "Weekdays: B1",
/// "Hours"), and what moving to it goes to: a cell's reference, or a worksheet's name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetEntry {
    pub label: String,
    pub target: String,
}

/// A spreadsheet's active cell among its row's and its column's cells within the sheet's used
/// range (Excel's), each as the sheet shows it, for JAWS's row and column readers: `row_at`
/// and `column_at` are the active cell's place in `row` and `column`, past their ends where
/// it is outside the used range.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SheetPlace {
    /// The active cell's reference, "D1".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub address: String,
    /// The selection's reference, "A1:B2", the active cell's alone where nothing more is
    /// selected.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub selection: String,
    pub row: Vec<String>,
    pub column: Vec<String>,
    pub row_at: usize,
    pub column_at: usize,
}

/// The caret's place on a word processor's pages (Word's `Selection.Information`): its page and
/// section, its line on the page and its column, each from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PagePlace {
    pub page: u32,
    pub section: u32,
    pub line: u32,
    pub column: u32,
}

/// A revision of Word's on the caret's paragraph: where it is in the text, as `(start, end)`
/// (a deletion's text, once taken out of the text as Word's view shows it, where it was), what
/// it changed, its text, author and date (as a comment's).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRevision {
    pub start: usize,
    pub end: usize,
    pub kind: RevisionKind,
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
}

/// A footnote's or endnote's reference of Word's on the caret's paragraph: the place in the text
/// before which it stands, the note's number, its mark as the document shows it ("i" for the
/// first endnote), the number where it shows that, the note's text, and whether Word's caret is
/// on the reference itself (which the text does not show, so the place cannot tell).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DocumentFootnote {
    pub at: usize,
    pub number: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mark: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub at_caret: bool,
    /// Word's selection holds the reference (Shift+Right onto it), which the text does not
    /// show, so its selection cannot tell.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub selected: bool,
}

impl DocumentFootnote {
    /// The note's mark as the document shows it.
    pub fn shown(&self) -> String {
        if self.mark.is_empty() {
            self.number.to_string()
        } else {
            self.mark.clone()
        }
    }
}

/// A comment of Word's on the caret's paragraph, over the text from `start` to `end`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DocumentComment {
    pub start: usize,
    pub end: usize,
    pub text: String,
    pub author: String,
}

/// A legacy form field of Word's on the caret's paragraph, placed in the context's text: a
/// text field covers the text it shows ("     ", "ab"); a check box and a drop-down show none
/// and sit between two characters, where Tab puts the caret [obs, 1056/s2-s4].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentFormField {
    pub kind: FormFieldKind,
    /// Its bookmark name ("Name").
    pub name: String,
    pub start: usize,
    pub end: usize,
    /// A text field's text, a drop-down's choice.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value: String,
    /// A check box's state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// A drop-down's choice (from 1) and its number of entries.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub choice: Option<(usize, usize)>,
}

/// The kinds of Word's legacy form fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormFieldKind {
    Text,
    CheckBox,
    DropDown,
}

/// One of Word's keys whose result JAWS says, read from Word's object model once Word has
/// acted on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordKey {
    /// Ctrl+Shift+E: whether the document tracks its changes.
    TrackChanges,
    /// Ctrl+E, R, J and L: the paragraph's alignment.
    Alignment,
    /// Ctrl+Shift+Period and Ctrl+Shift+Comma: the selection's font size.
    FontSize,
    /// Ctrl+M (`out` false) and Ctrl+Shift+M: the paragraph's left indent.
    Indent { out: bool },
}

impl WordKey {
    /// Its name in the actions' log, which replay answers by.
    pub fn name(self) -> &'static str {
        match self {
            Self::TrackChanges => "track changes",
            Self::Alignment => "paragraph alignment",
            Self::FontSize => "font size",
            Self::Indent { out: false } => "indent",
            Self::Indent { out: true } => "outdent",
        }
    }

    /// Every key, as replay answers them.
    pub const ALL: [WordKey; 5] = [
        Self::TrackChanges,
        Self::Alignment,
        Self::FontSize,
        Self::Indent { out: false },
        Self::Indent { out: true },
    ];
}

/// What one of Word's keys left ([`WordKey`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordKeyState {
    TrackChanges(bool),
    /// Word's `WdParagraphAlignment`: 0 left, 1 centered, 2 right, 3 justified.
    Alignment(i32),
    /// In tenths of a point.
    FontSize(u32),
    /// How far the paragraph starts from the page's left edge (margin and indent), in
    /// hundredths of an inch, and the way the key moved it.
    Indent {
        out: bool,
        hundredths: u32,
    },
}

/// A paragraph's line spacing as Word keeps it, for Say Font's "Line spacing:" [obs, 1040/s17].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineSpacing {
    /// Word's rule (`WdLineSpacing`): 0 single, 1 one and a half, 2 double, 3 at least, 4
    /// exactly, 5 multiple.
    pub rule: i32,
    /// The spacing in hundredths of a point; for multiple, 1200 a line.
    pub centipoints: u32,
}

/// A field of a document the platform reads (Word's), for its list of fields (Insert+F5): its
/// code (" HYPERLINK \"https://example.org/\" ", " FORMTEXT ") and the text it shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentField {
    pub code: String,
    #[serde(default)]
    pub result: String,
}

/// Which of a document's marks JAWS lists in Word: its comments (Ctrl+Shift+Apostrophe), its
/// footnotes (Insert+Shift+F), its endnotes (Insert+Shift+E), its bookmarks (Insert+Alt+B) or
/// its revisions, Word's tracked changes (Insert+Shift+R in JAWS's key map).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoteKind {
    Comment,
    Footnote,
    Endnote,
    Bookmark,
    Revision,
}

impl NoteKind {
    /// The request's name for reading them, in a log and in replay.
    pub fn read_name(self) -> &'static str {
        match self {
            Self::Comment => "document comments",
            Self::Footnote => "document footnotes",
            Self::Endnote => "document endnotes",
            Self::Bookmark => "document bookmarks",
            Self::Revision => "document revisions",
        }
    }
}

/// A comment, footnote, endnote or bookmark of a document the platform reads (Word's), for
/// JAWS's list of them: its text (a bookmark's, the text it marks); a comment's author and its
/// date as the user's locale writes "ddd, MMM dd, yy" ("Fri, Jan 02, 26"); a note's number, and
/// its mark as the document shows it ("i" for the first endnote), the number where it shows
/// that; a bookmark's name; a revision's text, its change, author and date as a comment's.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DocumentNote {
    pub text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<u32>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mark: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<RevisionKind>,
}

/// What a revision of Word's changed: text inserted or deleted, or another change (formatting,
/// a moved paragraph, a table's cells).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevisionKind {
    Inserted,
    Deleted,
    Other,
}

impl DocumentNote {
    /// A note's mark as the document shows it.
    pub fn shown_mark(&self) -> String {
        match (self.mark.is_empty(), self.number) {
            (false, _) => self.mark.clone(),
            (true, Some(number)) => number.to_string(),
            (true, None) => String::new(),
        }
    }
}

/// A link of a document the platform reads by paragraphs (Word's hyperlinks), for its Links
/// List (Insert+F7): its text, where it goes, and the paragraph it starts in, from the
/// document's first (0), by which the caret is moved to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentLink {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub paragraph: usize,
}

/// A heading of a document the platform reads by paragraphs (Word's), for its Heading List
/// (Insert+F6): its text, its level, and the paragraph it is, from the document's first (0),
/// by which the caret is moved to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentHeading {
    pub text: String,
    pub level: u8,
    pub paragraph: usize,
}

/// A paragraph of a document the platform reads by paragraphs (Word's), as Say All reads it:
/// where it starts in the document's text, in characters, its heading level, whether it is a
/// list item, and the table cell holding it (its headers left out).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParagraph {
    pub start: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<u8>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub list_item: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<TableCell>,
}

/// A cell of a word processor's table, where the caret is.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableCell {
    /// The table's number in the document, from 1: "table 1".
    pub table: usize,
    /// The cell's row and column, from 1.
    pub row: usize,
    pub column: usize,
    /// The table's rows and columns.
    pub rows: usize,
    pub columns: usize,
    /// Its column's header text, from the table's first row, which JAWS says on entering the
    /// column: "Hours 9 to 9" [obs, 518/s15].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column_header: Option<String>,
    /// Its row's title, the row's first cell, which JAWS says on entering the row: "Branch
    /// Hours" [obs, 1034/s11]; none in a table without row titles, where JAWS says "Row 1 no
    /// title" [obs, 1040/s13].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_header: Option<String>,
}

/// The font at a text field's caret, as UIA text attributes or a rich edit report it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontInfo {
    pub family: String,
    /// Size in tenths of a point (11 points is 110).
    pub size_tenths: u32,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    /// Paragraph alignment: `left`, `right`, `centered` or `justified`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<String>,
    /// The text's color and its background's, where the field reports or draws them: Say
    /// Color names them [obs, 661: "black on white" in a text box].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<crate::color::Rgb>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<crate::color::Rgb>,
    /// The paragraph's style, where the application names it (Word's "Heading 1", "Normal"):
    /// Say Font in Word says it [obs, 1034/s1, s3].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
}

impl FontInfo {
    /// As the formatting Say Font speaks.
    pub fn formatting(&self) -> crate::aom::TextFormatting {
        crate::aom::TextFormatting {
            font_family: Some(self.family.clone()).filter(|f| !f.is_empty()),
            font_size_pt: (self.size_tenths > 0).then(|| self.size_tenths as f32 / 10.0),
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            alignment: self.alignment.clone(),
            color: self.color,
            background: self.background,
            ..crate::aom::TextFormatting::default()
        }
    }
}

/// What changed on the focused control (MSAA `EVENT_OBJECT_STATECHANGE`, `VALUECHANGE`,
/// `SELECTION`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    /// A state: checked, expanded, selected.
    State,
    /// The value: a combo box selection, a slider position.
    Value,
    /// The selected item of a container.
    Selection,
}

/// Menu mode transitions (MSAA `EVENT_SYSTEM_MENU*`). Focus within menus arrives as focus
/// events on [`Role::MenuItem`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuEvent {
    /// Menu mode started, from this window (its class): the menu bar became active, or a
    /// popup menu is about to open.
    Start {
        #[serde(default)]
        window_class: Option<String>,
    },
    /// Menu mode ended; focus returns to the application.
    End,
    /// A popup menu opened.
    PopupStart,
    /// A popup menu closed.
    PopupEnd,
}

/// One control of a window, as Say Window (Insert+B) reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowItem {
    pub role: Role,
    pub name: String,
    pub value: Option<String>,
    pub expanded: Option<bool>,
    pub checked: Option<bool>,
    /// An edit whose value cannot be changed: "read only edit" [obs, 323/s2].
    #[serde(default)]
    pub read_only: bool,
    /// The group box the control sits in: Say Window names the control after it.
    #[serde(default)]
    pub group: Option<String>,
    /// A combo box's place in its whole list, said after it and after its field: "38 of 187"
    /// [obs, 665/s3].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub place: Option<(usize, usize)>,
    /// A part of a combo box: its list ("List box") or its field ("edit combo").
    #[serde(default)]
    pub in_combo_box: bool,
}

/// Event representing OS-level keyboard focus changes across applications and controls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusEvent {
    pub node_id: Option<usize>,
    pub role: Role,
    pub name: Option<String>,
    pub value: Option<String>,
    pub states: NodeStates,
    pub item_index: Option<usize>,
    pub item_count: Option<usize>,
    pub container_role: Option<Role>,
    pub container_name: Option<String>,
    /// The container is a list view, a list of items with columns (JAWS's "List view"),
    /// not a list box.
    pub container_is_list_view: bool,
    /// The focus is a list view itself, not an item of it: an empty one, as JAWS's
    /// PlaceMarker List without PlaceMarkers [obs, 699/s6].
    #[serde(default)]
    pub list_view: bool,
    /// The list view lets its items be renamed in place (F2): JAWS ends its hint with "To
    /// edit the selected item press f2." [obs, 748/s7].
    #[serde(default)]
    pub list_edits_labels: bool,
    /// The focus is the edit a list view opens on its item to rename it (F2): unnamed, and
    /// not in the list, which the focus enters again after it [obs, 751/s18, s20].
    #[serde(default)]
    pub list_label_edit: bool,
    /// The focused menu item is on a menu bar (a page's ARIA menu bar), not in a menu.
    #[serde(default)]
    pub in_menu_bar: bool,
    /// The focused menu item is on a menu bar or in one of its menus: a page's menu, not a
    /// popup of its own (WhatsApp's message menu).
    #[serde(default)]
    pub under_menu_bar: bool,
    /// The focus is inside a page's menu without being one of its items (WhatsApp's
    /// reaction buttons at the top of its message menu): the menu keeps its keys.
    #[serde(default)]
    pub in_menu: bool,
    /// The named groups and the lists a native control is in, outermost first, up to its
    /// window: Windows Settings' "Brightness & color", "Night light"; "Advanced",
    /// "Troubleshoot common sound problems", then that list. Empty where none, and where the
    /// groups are a control's own content.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GroupEntry>,
    /// Further columns of a list-view item, after its name.
    pub columns: Vec<String>,
    /// A list-view item's column headers, in order ("Name", "Size") [obs, 514/s5].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub column_headers: Vec<String>,
    /// The focus is on a ribbon (a NetUI ribbon: WordPad's, Paint's), whose tabs JAWS reads
    /// without a property sheet's page hint [obs, 523/s2, s3].
    #[serde(default)]
    pub in_ribbon: bool,
    /// In the ribbon, on its menus: a tab, or a command on a tab's page (the lower ribbon), not
    /// the buttons around them (Minimize the Ribbon, the Quick Access Toolbar), which JAWS
    /// leaves the menus for: "Leaving menus | Leaving ribbons" [obs, 523/s4, 972/s3].
    #[serde(default)]
    pub ribbon_menu: bool,
    /// A button that drops a menu or a gallery down without being a menu's item (Word's
    /// Styles, More): JAWS says "sub menu" before it, "sub menu | Styles Button | More"
    /// [obs, 1051/s17].
    #[serde(default)]
    pub opens_menu: bool,
    /// The ribbon's own name, as its MSAA property page gives it: Word's "Ribbon", which JAWS
    /// says as the focus enters it [obs, 1043/s1].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ribbon_name: Option<String>,
    /// The focused element is inside a browser document (its focus chain has a document).
    pub in_document: bool,
    /// The element's DOM id, when it is in a document and has one.
    pub dom_id: Option<String>,
    /// The element's IA2 uniqueID, when it is in a page whose window reports it: the page's
    /// node with the same [`crate::aom::UNIQUE_ID_ATTR`] is the focus.
    pub unique_id: Option<String>,
    /// The page region with ARIA role "application" the focus is in (VS Code's workbench), by
    /// its name: the page takes every key there, as JAWS gives them.
    #[serde(default)]
    pub web_application: Option<String>,
    /// For tree items: the nesting level, 0 for top-level items.
    pub tree_level: Option<usize>,
    /// Number of child items (an expanded tree item's children).
    pub child_count: Option<usize>,
    /// A tree item's status, said after its place as the focus enters the tree: Okur's rules
    /// dialog's "0 Active", as JAWS's [obs, 748/s8].
    #[serde(default)]
    pub item_status: Option<String>,
    /// A tree item's parent, said after its place when an arrow changes the level: Okur's
    /// rules dialog's "level 2 | Current Page: ... | 1 of 1 | Current Site: localhost", as
    /// JAWS's [obs, 748/s11, s14].
    #[serde(default)]
    pub item_parent: Option<String>,
    /// The window in front is owned by another that stays available while it is open: a
    /// modeless dialog, Word's Find and Replace, which Ctrl+Tab leaves for the document [obs,
    /// 1065/s6].
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub modeless: bool,
    /// The hints an item of Okur's own dialog is entered with, in place of its role's: Settings
    /// Center's search results, "Use the arrow keys to select an item. | Press the space bar to
    /// toggle between the choices" [obs, the walk of Settings Center's search].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub own_hints: Vec<String>,
    /// For a text field: its text, caret and selection when it got focus.
    pub text: Option<TextContext>,
    /// When the active window is a dialog box (Win32 `#32770`): its static text.
    pub dialog_text: Option<String>,
    /// When the focus is on a property sheet's page in that dialog box: the page's name,
    /// announced as the dialog opens ("General page").
    pub dialog_page: Option<String>,
    /// When the focus is on a wizard's page (a property sheet showing no tabs, which takes its
    /// page's title as its own): the page's name. A wizard's page is not named as a page; its
    /// text is the dialog's [obs, 705/s3].
    #[serde(default)]
    pub wizard_page: Option<String>,
    /// The dialog boxes the active one opened over, outermost first: JAWS names them before
    /// it ("PlaceMarker List dialog | Add PlaceMarker dialog | Name: Edit" [obs, 702/s5]).
    #[serde(default)]
    pub outer_dialogs: Vec<String>,
    /// The windows a message box opened over that are not dialog boxes (a WPF dialog
    /// window), outermost first: JAWS names them plainly before it ("Manage Notification
    /// Rules | Confirm Delete dialog" [obs, 787/s17]).
    #[serde(default)]
    pub outer_windows: Vec<String>,
    /// When the focused control is inside a dialog box's group box: the group box's text,
    /// which JAWS says before the control's name ("Speech Rate: left right slider").
    #[serde(default)]
    pub group_box: Option<String>,
    /// When a dialog box is in front: its default button, the one Enter presses, which Insert+E
    /// says where the focus is not on a button [obs, 540/s2].
    #[serde(default)]
    pub default_button: Option<String>,
    /// When the active window is a dialog box: its buttons, in order (the Virtual Viewer's
    /// last line, "Yes No Cancel") [obs, 311/s14].
    pub dialog_buttons: Vec<String>,
    /// When a dialog box is in front: its text by screen rows, below its title (the Virtual
    /// Viewer's lines for Virtualize Window).
    pub dialog_lines: Vec<String>,
    /// When a dialog box is in front: its controls in reading order, below the dialog itself
    /// (Say Window, Insert+B).
    pub window_items: Vec<WindowItem>,
    /// A common file dialog's folder, as its address bar names it ("Address: C:\Users\…"):
    /// Say Focus in its file name field ends with it [obs, 524/s3].
    #[serde(default)]
    pub file_dialog_address: Option<String>,
    /// The window in front is a dialog box.
    pub in_dialog_box: bool,
    pub window_title: Option<String>,
    pub folder_name: Option<String>,
    pub help_text: Option<String>,
    /// A native control's own description, which JAWS says after its state where MSAA alone
    /// knows the control: an Office command bar's button, "Existing File or Web Page button"
    /// [obs, 1076/s5].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// What kind of item it is, where the application says (UIA `ItemType`: File Explorer's
    /// "File folder").
    pub item_type: Option<String>,
    /// Its size, where its application shows one (File Explorer's "1 KB"; none for a folder).
    pub item_size: Option<String>,
    /// A shell item's row: its columns' values after its name, in the view's order (File
    /// Explorer's "1/2/2026 3:04 AM", "PNG File", "11 KB").
    #[serde(default)]
    pub item_details: Vec<String>,
    /// The shortcut that runs its command (UIA `AcceleratorKey`: a menu item's "Ctrl+X").
    pub accelerator_key: Option<String>,
    /// The keys that reach the control from anywhere in its window ("Alt+o").
    pub access_key: Option<String>,
    /// Whether the control's value is read-only (UIA), where it reports one: an editable
    /// document is a text field.
    pub read_only: Option<bool>,
    /// A native edit of several lines (Win32's ES_MULTILINE): JAWS reads it as a document,
    /// its caret's line and "Contains text" on arrival [obs, 316/s3], the line alone on Say
    /// Line and "top of file" on Ctrl+Home [obs, 400/s1, s5].
    pub multi_line: bool,
    /// A plain Win32 edit (Edit, WinForms' EDIT), not RichEdit: JAWS reads only the first line
    /// of a selection over several lines in it [obs, 313/s7, 529/s6, s9, s10].
    #[serde(default)]
    pub plain_edit: bool,
    /// A classic RichEdit window (WordPad's document), which UI Automation's proxy names by the
    /// window's own text ("Rich Text Window"), not a label.
    #[serde(default)]
    pub rich_edit: bool,
    /// The window is read element by element, as JAWS's Scan Cursor reads an application its
    /// screen capture cannot (WPF, UWP, WinUI): the platform decides it from the window's kind
    /// [obs, 1002/s1, s2].
    #[serde(default)]
    pub read_by_elements: bool,
    /// The window shows files to select (File Explorer, the desktop): Convenient OCR's F and
    /// Enter recognize the selected file there. The platform decides it from the window's kind.
    #[serde(default)]
    pub in_file_view: bool,
    /// An edit with an up-down control attached (a spin box): "Number of copies: edit spin
    /// box" [obs, 546/s8].
    #[serde(default)]
    pub spin_box: bool,
    /// The window's menu bar items ("File", "Edit", ...) and its status bar's parts ("Ln 1,
    /// Col 1", "100%", ...): JAWS's view of the window shows them above and below its text
    /// [obs, 314/s10, s11].
    pub menu_bar: Vec<String>,
    pub status_bar: Vec<String>,
    /// The application, as the platform names it: its executable on Windows
    /// ("SearchHost.exe"), its bundle identifier on macOS. For application modules (`apps/`).
    pub app: Option<String>,
    /// The application's product name and version, as its executable gives them: what
    /// Ctrl+Insert+V says [obs, 513/s4].
    #[serde(default)]
    pub app_product: Option<String>,
    #[serde(default)]
    pub app_version: Option<String>,
    /// Where the focused element is on the screen, `[left, top, right, bottom]` in pixels:
    /// the PC cursor's place (Alt+Delete) [obs, 513/s2].
    #[serde(default)]
    pub bounds: Option<[i32; 4]>,
    /// What the platform calls the focus's windows and elements, for application modules.
    pub platform: PlatformIds,
}

/// A group or a list a control is in ([`FocusEvent::groups`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupEntry {
    pub name: String,
    /// A list (said with its role, "List box"), not a group.
    #[serde(default)]
    pub list: bool,
    /// A dialog (said with its role): Windows Settings' InfoBar, "Power mode is set
    /// automatically ... dialog".
    #[serde(default)]
    pub dialog: bool,
    /// A row of tabs (said with its role): a ribbon's, "Ribbon tabs | Tab" [obs, 970/s1].
    #[serde(default)]
    pub tab_list: bool,
    /// A tool bar (said with its role): a ribbon's group, "Clipboard Tool bar" [obs, 970/s4].
    #[serde(default)]
    pub tool_bar: bool,
    /// What is said after its name: a ribbon page's "Ribbon tab", a split button's group's
    /// description [obs, 970/s4].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// A window's pane (UI Automation's), around a ribbon's own groups: WordPad's
    /// "UIRibbonDockTop", Word's "MsoDockTop" [obs, 970/s1].
    #[serde(default)]
    pub pane: bool,
    /// Unavailable, as the grouping of Word's Undo is while there is nothing to undo: "Can't
    /// Undo Unavailable" [obs, 1051/s6].
    #[serde(default)]
    pub disabled: bool,
}

impl GroupEntry {
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            list: false,
            dialog: false,
            tab_list: false,
            tool_bar: false,
            detail: None,
            pane: false,
            disabled: false,
        }
    }

    pub fn pane(name: impl Into<String>) -> Self {
        Self {
            pane: true,
            ..Self::group(name)
        }
    }

    pub fn list(name: impl Into<String>) -> Self {
        Self {
            list: true,
            ..Self::group(name)
        }
    }

    pub fn dialog(name: impl Into<String>) -> Self {
        Self {
            dialog: true,
            ..Self::group(name)
        }
    }

    pub fn tab_list(name: impl Into<String>) -> Self {
        Self {
            tab_list: true,
            ..Self::group(name)
        }
    }

    pub fn tool_bar(name: impl Into<String>) -> Self {
        Self {
            tool_bar: true,
            ..Self::group(name)
        }
    }

    /// This entry with `detail` said after its name.
    pub fn with_detail(mut self, detail: Option<String>) -> Self {
        self.detail = detail;
        self
    }
}

/// What the platform calls things: window and element classes. Application modules (`apps/`)
/// match them, as JAWS's application scripts know their windows' classes. General code
/// never matches their values; it may compare them, to tell two windows apart.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformIds {
    /// The class of the window the event came from (a Win32 window class).
    pub window_class: Option<String>,
    /// The class of the top-level window in front, which tells windows with the same title
    /// apart (a message box titled like its main window).
    pub top_window_class: Option<String>,
    /// The focused element's own class (UIA ClassName: Windows Terminal's text area is a
    /// "TermControl").
    pub element_class: Option<String>,
    /// The class of the focus's container.
    pub container_class: Option<String>,
    /// The focused element's automation id (UIA AutomationId: Windows Settings' search box
    /// is "CommandSearchTextBox").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<String>,
}

impl FocusEvent {
    pub fn new(role: Role) -> Self {
        Self {
            node_id: None,
            role,
            name: None,
            value: None,
            states: NodeStates::default(),
            item_index: None,
            item_count: None,
            container_role: None,
            container_name: None,
            container_is_list_view: false,
            list_view: false,
            list_edits_labels: false,
            list_label_edit: false,
            in_menu_bar: false,
            under_menu_bar: false,
            in_menu: false,
            groups: Vec::new(),
            columns: Vec::new(),
            column_headers: Vec::new(),
            in_ribbon: false,
            ribbon_menu: false,
            opens_menu: false,
            ribbon_name: None,
            in_document: false,
            dom_id: None,
            unique_id: None,
            web_application: None,
            tree_level: None,
            item_status: None,
            item_parent: None,
            own_hints: Vec::new(),
            modeless: false,
            child_count: None,
            text: None,
            dialog_text: None,
            dialog_page: None,
            wizard_page: None,
            group_box: None,
            default_button: None,
            dialog_buttons: Vec::new(),
            dialog_lines: Vec::new(),
            window_items: Vec::new(),
            file_dialog_address: None,
            in_dialog_box: false,
            window_title: None,
            folder_name: None,
            help_text: None,
            description: None,
            item_type: None,
            item_size: None,
            item_details: Vec::new(),
            accelerator_key: None,
            access_key: None,
            read_only: None,
            multi_line: false,
            plain_edit: false,
            rich_edit: false,
            read_by_elements: false,
            in_file_view: false,
            spin_box: false,
            outer_dialogs: Vec::new(),
            outer_windows: Vec::new(),
            menu_bar: Vec::new(),
            status_bar: Vec::new(),
            app: None,
            app_product: None,
            app_version: None,
            bounds: None,
            platform: PlatformIds::default(),
        }
    }

    /// A native edit of several lines that holds text, which JAWS reads as a document (316/s3,
    /// 400). An empty one is read as a one-line edit: "Notes: Edit | Type in text.", and Say
    /// Line "Notes: Edit" [obs, 301/s1, 305/s15].
    pub fn reads_as_document(&self) -> bool {
        let has_text = |t: Option<&str>| t.is_some_and(|t| !t.trim().is_empty());
        self.multi_line
            && (has_text(self.text.as_ref().map(|c| c.text.as_str()))
                || has_text(self.value.as_deref()))
    }

    pub fn with_node_id(mut self, id: usize) -> Self {
        self.node_id = Some(id);
        self
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_states(mut self, states: NodeStates) -> Self {
        self.states = states;
        self
    }

    pub fn with_item_index(mut self, index: usize) -> Self {
        self.item_index = Some(index);
        self
    }

    pub fn with_item_count(mut self, count: usize) -> Self {
        self.item_count = Some(count);
        self
    }

    pub fn with_container_role(mut self, role: Role) -> Self {
        self.container_role = Some(role);
        self
    }

    pub fn with_container_name(mut self, name: impl Into<String>) -> Self {
        self.container_name = Some(name.into());
        self
    }

    pub fn with_window_class(mut self, class: impl Into<String>) -> Self {
        self.platform.window_class = Some(class.into());
        self
    }

    pub fn with_window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = Some(title.into());
        self
    }

    pub fn with_folder_name(mut self, folder: impl Into<String>) -> Self {
        self.folder_name = Some(folder.into());
        self
    }

    pub fn with_help_text(mut self, help: impl Into<String>) -> Self {
        self.help_text = Some(help.into());
        self
    }
}

/// Event representing ARIA live region DOM mutations and status announcements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveRegionEvent {
    /// The region's whole text now, one line per text run.
    pub text: String,
    pub politeness: Politeness,
    pub container_name: Option<String>,
    /// The region's DOM id, to find its previous text in the page.
    #[serde(default)]
    pub region_id: Option<String>,
    /// The region is read as a whole when it changes (`aria-atomic`), not by what was added.
    #[serde(default)]
    pub atomic: bool,
}

impl LiveRegionEvent {
    pub fn new(text: impl Into<String>, politeness: Politeness) -> Self {
        Self {
            text: text.into(),
            politeness,
            container_name: None,
            region_id: None,
            atomic: false,
        }
    }

    pub fn with_container(mut self, container: impl Into<String>) -> Self {
        self.container_name = Some(container.into());
        self
    }
}

/// An application's top-level window, as the Window List shows it (Insert+F10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopWindow {
    /// The platform's handle for it, to switch to it.
    pub id: u64,
    pub title: String,
    pub state: WindowState,
    /// Kept above the other windows (Windows' topmost windows).
    pub on_top: bool,
}

/// An icon of the taskbar's notification area, as the system tray list shows it (Insert+F11):
/// its name, and the screen point (real pixels) its clicks go to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrayIcon {
    pub name: String,
    pub x: i32,
    pub y: i32,
}

/// How a window is shown: JAWS's Window List says it after the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowState {
    Minimized,
    Maximized,
    Restored,
}

/// Primary accessibility event enum passed from platform bridges to the engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum AccessibilityEvent {
    Focus(FocusEvent),
    /// Menu mode transitions.
    Menu(MenuEvent),
    /// The focused control changed without focus moving: `control` is its new state, read
    /// like a focus landing on it.
    Change {
        kind: ChangeKind,
        control: FocusEvent,
    },
    /// The focused text field's text, caret or selection, read after a key moved or edited
    /// it, or reported by the application.
    TextContext(TextContext),
    /// An item selected in a list while the focus stays elsewhere: Windows search's results
    /// as the search box's text changes, or as Down moves through them.
    Selected(FocusEvent),
    LiveRegion(LiveRegionEvent),
    /// The Virtual Viewer's lines for the window or control in front, read for Virtualize
    /// Window or Virtualize Current Control; empty when nothing could be read.
    ViewerText(Vec<String>),
    /// The text the accessibility interfaces place on the window in front, each piece where
    /// it is drawn, read for the Okur cursor's model; empty when nothing could be read
    /// ([`crate::actions::ApplicationActions::read_screen`]).
    ScreenText(crate::screen_model::WindowText),
    /// The words recognized from the screen's pixels where they are drawn, asked for by
    /// Insert+Space, O; empty when none was recognized or recognition failed
    /// ([`crate::actions::ApplicationActions::recognize`]).
    Recognized(Vec<crate::screen_model::ScreenRun>),
    /// The text recognized in the file selected in File Explorer, asked for by Insert+Space,
    /// O, F ([`crate::actions::ApplicationActions::recognize_selected_file`]).
    FileRecognized(crate::recognized_file::FileRecognition),
    /// The selected file is being recognized: it is a picture or a PDF, and its text comes
    /// later as [`AccessibilityEvent::FileRecognized`].
    FileRecognitionStarted,
    /// The applications' windows, front to back, asked for by the Window List (Insert+F10;
    /// [`crate::actions::ApplicationActions::list_windows`]).
    TopWindows(Vec<TopWindow>),
    /// The notification area's icons, left to right, asked for by the system tray list
    /// (Insert+F11; [`crate::actions::ApplicationActions::list_tray_icons`]).
    TrayIcons(Vec<TrayIcon>),
    /// Page elements asked for by DOM id, each with its subtree, as they are now; empty when
    /// none was found ([`crate::actions::ApplicationActions::read_page_elements`]).
    PageElements(Vec<crate::aom::AomNode>),
    /// The focused document's headings, in order, asked for by its Heading List (Insert+F6
    /// in Word; [`crate::actions::ApplicationActions::read_document_headings`]); empty when it
    /// has none or could not be read.
    DocumentHeadings(Vec<DocumentHeading>),
    /// The focused document's links, in order, asked for by its Links List (Insert+F7 in
    /// Word; [`crate::actions::ApplicationActions::read_document_links`]); empty when it has
    /// none or could not be read.
    DocumentLinks(Vec<DocumentLink>),
    /// The line spacing of the paragraph at the focused document's caret (Word's), asked for
    /// by Say Font ([`crate::actions::ApplicationActions::read_line_spacing`]); `None` when it
    /// could not be read.
    LineSpacing(Option<LineSpacing>),
    /// What one of Word's keys left in the focused document, read once Word acted on it
    /// ([`crate::actions::ApplicationActions::read_word_key`]); `None` when it could not be
    /// read.
    WordKeyState(Option<WordKeyState>),
    /// The focused document's fields, in order, asked for by its list of fields (Insert+F5 in
    /// Word; [`crate::actions::ApplicationActions::read_document_fields`]); empty when it has
    /// none or could not be read.
    DocumentFields(Vec<DocumentField>),
    /// The focused document's comments or footnotes, in order, asked for by JAWS's list of them
    /// in Word ([`crate::actions::ApplicationActions::read_document_notes`]); empty when it has
    /// none or they could not be read.
    DocumentNotes(NoteKind, Vec<DocumentNote>),
    /// The focused document's paragraphs, in order, asked for by Say All in Word
    /// ([`crate::actions::ApplicationActions::read_document_paragraphs`]); empty when they could
    /// not be read.
    DocumentParagraphs(Vec<DocumentParagraph>),
    /// The view Word's window shows its document in, by the name of the status bar's view
    /// button that is on ("Print Layout"), asked for by Say Window Title
    /// ([`crate::actions::ApplicationActions::read_document_view`]); `None` when it could not
    /// be read.
    DocumentView(Option<String>),
    /// The items of the status bar of Word's window in front, as JAWS reads them: each button
    /// by its name, the view's only where it is on, none of the zoom's slider
    /// ([`crate::actions::ApplicationActions::read_status_bar`]); none where it could not be
    /// read.
    StatusBar(Vec<String>),
    /// The active cell's row and column of the spreadsheet in front
    /// ([`crate::actions::ApplicationActions::read_sheet_place`]); `None` where they could
    /// not be read.
    SheetPlace(Option<SheetPlace>),
    /// One of JAWS's lists of the spreadsheet in front
    /// ([`crate::actions::ApplicationActions::read_sheet_list`]): its entries, and for the
    /// column's, the column's letters ("B"); none where they could not be read.
    SheetList {
        kind: SheetListKind,
        column: String,
        entries: Vec<SheetEntry>,
    },
    /// What the user did to one of Okur's own windows (`okur-ui`).
    Ui(crate::ui::FromUi),
    /// The page in front changed outside its live regions: a web application added or
    /// removed content (WhatsApp renders a chat's messages after the chat opens). Its
    /// buffer is captured again ([`crate::actions::ApplicationActions::page_changed`]).
    PageChanged,
    /// The parts of the page in front read again after they changed (each an object whose
    /// children changed, with its subtree): spliced into the page where the same live objects
    /// are ([`crate::aom::UNIQUE_ID_ATTR`]), all at once.
    PagePartsLoaded(Vec<crate::aom::AomNode>),
    /// The mouse pointer moved, to `x`, `y` in screen pixels: the JAWS cursor follows it.
    MouseMoved {
        x: i32,
        y: i32,
    },
    /// What the mouse points to where it came to rest: read live when the engine asks
    /// ([`crate::actions::ApplicationActions::read_at_point`]), traced where the mouse rested.
    MousePoint(Box<MousePoint>),
    /// A Windows notification's banner as it shows (a toast), by the name the shell gives it:
    /// "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1
    /// of 1", and its sender, "Windows PowerShell" [obs, 770/s1].
    Toast {
        text: String,
        sender: Option<String>,
    },
    /// A notification an application raised through UI Automation (`UiaRaiseNotificationEvent`,
    /// as Teams notifies, not a toast): said as it comes, interrupting when the application
    /// marks it important, and kept in the Notification History under its program's name,
    /// "Upload finished: report.pdf powershell 4:53 PM" [obs, 820/s1, s3].
    AppNotification {
        text: String,
        politeness: Politeness,
        /// The program's name, its executable's without ".exe".
        app: Option<String>,
    },
    /// The console in front wrote to its screen: the rows it shows now. Read once its writing
    /// pauses, for what is new below what it showed ([`crate::screen_echo`]).
    ConsoleText(String),
    /// The window in front was renamed: a console takes the title what runs in it sets
    /// ("SR Console Fixture" after PowerShell's own, 321).
    WindowRenamed(String),
    /// The dialog in front was shown, and read again: its controls as they are now. The focus
    /// can arrive while a dialog is still hidden (Notepad's Font dialog), and Say Window reads
    /// the window as it shows [obs, 665/s3]. Nothing is said.
    WindowShown(FocusEvent),
    DocumentLoaded {
        title: String,
        root: crate::aom::AomNode,
    },
}
