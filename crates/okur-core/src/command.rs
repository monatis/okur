//! What a key does, as a value: the commands Okur runs, named after JAWS's scripts where JAWS
//! has one, and the key map that binds keys to them.
//!
//! A command's name is a string in one place only, [`Command::from_name`]. Scenario steps and
//! tests call commands by JAWS script name ("MoveToNextHeading") or by the key notation JAWS's
//! key maps use ("{DOWN}", "^%{RIGHT}"). Everywhere else a command is a [`Command`].

use crate::element_lists::ElementList;
use crate::input::{Key, KeyEvent};
use crate::settings::{KeyboardLayout, KeyboardSettings};

/// What quick navigation moves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    Heading,
    /// A heading of one level, 1 to 6.
    HeadingLevel(u8),
    Link,
    Table,
    Article,
    OnMouseOver,
    /// An ARIA tab.
    Tab,
    PlaceMarker,
    Graphic,
    RadioButton,
    Math,
    FormField,
    Button,
    Edit,
    CheckBox,
    ComboBox,
    List,
    ListItem,
    Landmark,
    Frame,
    /// A `div` with text (Z).
    Division,
    /// A horizontal rule (Minus).
    Separator,
    /// A link that writes an e-mail (Backslash).
    MailLink,
}

/// A screen reader command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Reading where the cursor is.
    SayAll,
    SayLine,
    SayWord,
    SayCharacter,
    /// Say Character pressed twice quickly: the character spelled phonetically.
    SayCharacterPhonetic,
    /// Say Character pressed three times quickly: the character's value.
    SayCharacterValue,
    /// Say Word pressed twice quickly: the word spelled.
    SpellWord,
    /// Say Line pressed twice quickly: the line spelled.
    SpellLine,

    // Moving, then reading.
    SayNextLine,
    SayPriorLine,
    SayNextWord,
    SayPriorWord,
    SayNextCharacter,
    SayPriorCharacter,
    SayNextParagraph,
    SayPriorParagraph,
    /// Ctrl+Numpad5: the paragraph the cursor is in.
    SayParagraph,
    /// Alt+Numpad5, Alt+NumpadPlus, Alt+NumpadMinus: the sentence, the next, the prior.
    SaySentence,
    SayNextSentence,
    SayPriorSentence,
    /// The laptop layout's CapsLock+Ctrl+O: the next paragraph, or in a page the articles
    /// list (JAWS 2025 help, Laptop Keystrokes for Reading Text).
    NextParagraphOrArticles,
    /// Home, End, Page Up and Page Down with the virtual cursor (Default.JKM: JAWSHome,
    /// JAWSEnd, JAWSPageUp, JAWSPageDown).
    LineStart,
    LineEnd,
    PageUp,
    PageDown,
    TopOfFile,
    BottomOfFile,

    // Quick navigation.
    MoveToNext(Element),
    MoveToPrior(Element),
    MoveToMainRegion,
    /// N and Insert+Enter: the next text after a link (JAWS's MoveToNextNonLinkText).
    NextNonLinkText,
    /// Shift+N and Insert+Shift+Enter: the prior one.
    PriorNonLinkText,

    // Tables.
    TableCellRight,
    TableCellLeft,
    TableCellDown,
    TableCellUp,
    TableFirstCell,
    TableLastCell,
    MoveToStartOfRow,
    MoveToEndOfRow,
    MoveToTopOfColumn,
    MoveToBottomOfColumn,
    SayCurrentCell,
    SayColumnHeader,
    SayRowHeader,
    /// Alt+Insert+Home and End: the page's first and last heading.
    FirstHeading,
    LastHeading,
    /// Ctrl+Alt+Insert and a level (1 to 6), with Shift for the last: the first and last
    /// heading of that level.
    FirstHeadingAtLevel(u32),
    LastHeadingAtLevel(u32),
    /// Ctrl+Insert+Home and End: the page's first and last form field.
    FirstFormField,
    LastFormField,
    /// Windows+Alt+Numpad5 or Comma: the current row, whole.
    SayCurrentRow,
    /// Windows+Alt+Period: the current column, whole.
    SayCurrentColumn,
    /// Windows+Alt+Down and Up: the next or prior row, whole.
    ReadNextRow,
    ReadPriorRow,
    /// Windows+Alt+Right and Left: the next or prior column, whole.
    ReadNextColumn,
    ReadPriorColumn,
    /// Shift+Comma and Shift+Period, Alt+Windows+Home and End: the start and the end of the
    /// element the virtual cursor is in.
    StepToStartOfElement,
    StepToEndOfElement,

    // A page's controls.
    NextTabStop,
    PriorTabStop,
    /// Space: activate the control under the cursor.
    Activate,
    /// Enter: forms mode in a field, or activate the link or control.
    Enter,
    /// Escape: collapse, leave forms mode, or leave a modal dialog.
    Escape,
    ExpandComboBox,
    CollapseComboBox,
    EnterMathViewer,
    EnterModal,
    ExitModal,

    // Selection.
    SelectNextCharacter,
    SelectPriorCharacter,
    SelectNextLine,
    SelectPriorLine,
    /// Ctrl+Shift+Right and Left: to the next or prior word (Default.JKM: SelectNextWord).
    SelectNextWord,
    SelectPriorWord,
    /// Shift+End and Shift+Home: to the line's end or start.
    SelectToLineEnd,
    SelectToLineStart,
    /// Ctrl+Shift+End and Ctrl+Shift+Home: to the page's bottom or top (SelectToBottom).
    SelectToBottom,
    SelectToTop,
    /// Shift+PageDown and Shift+PageUp: a page of lines (SelectNextScreen).
    SelectNextPage,
    SelectPriorPage,
    /// Ctrl+A (SelectAll).
    SelectAll,
    /// F8: the element at the virtual cursor (SelectHTMLElement).
    SelectHtmlElement,
    /// Ctrl+C with a selection made with the virtual cursor (CopySelectedTextToClipboard).
    CopySelection,
    SaySelectedText,
    /// Insert+Space: the layer's next key (H, the Speech History; S, speech off and on).
    LayerStart,
    /// Insert+N: the quick keys on, off, or for Say All only (NavigationModeToggle).
    NavigationModeToggle,
    /// Insert+Shift+Z, Ctrl+Insert+N: the quick keys suspended or resumed.
    QuickKeysSuspendResume,
    /// Insert+Z: the virtual cursor off or on (VirtualPCCursorToggle).
    VirtualCursorToggle,
    /// Insert+Escape (RefreshScreen).
    RefreshScreen,
    /// Ctrl+Alt+PageUp and PageDown: the voice rate faster or slower for now.
    RateFaster,
    RateSlower,
    /// Insert+End: the window's top line (its title).
    SayTopLineOfWindow,
    /// Insert+Home: the line up to the cursor.
    SayToCursor,
    /// Insert+PageUp: the line from the cursor.
    SayFromCursor,
    /// The same three pressed twice quickly: spelled [obs, 504/s11, s18].
    SpellSelectedText,
    SpellToCursor,
    SpellFromCursor,
    /// Insert+PageDown: the window's bottom line.
    SayBottomLineOfWindow,
    /// Insert+E: the dialog box's default button, or "Not in a dialog box".
    SayDefaultButton,
    /// Insert+Q: which settings are loaded, and the application in use.
    SayConfiguration,
    /// Shift+Numpad5: the focused control's hot key.
    SayHotKey,
    /// Ctrl+Insert+1 to 0: a list view item's column (1 to 10), with its header.
    SayListColumn(u8),
    /// Ctrl+Insert+F7: the address of the link at the virtual cursor.
    SayLinkAddress,
    /// Insert+Delete: the virtual cursor to the focus.
    RouteVirtualToPc,
    /// Ctrl+Insert+Delete: the focus to the virtual cursor.
    RoutePcToVirtual,
    /// Insert+F9: the page's frames.
    SelectAFrame,
    /// Numpad Slash: a left mouse click, on the element at the virtual cursor in a page.
    LeftMouseClick,
    /// Insert+F10: the Window List, the running applications' windows to switch to.
    WindowList,

    // Help, in the Virtual Viewer, in Okur's own words.
    /// Insert+F1: what the focused control is for, and its keys.
    ScreenSensitiveHelp,
    /// Insert+H: Okur's keys for the window or page in front, each a link that uses it.
    HotKeyHelp,
    /// Insert+W: the keys Windows gives every application.
    WindowsKeysHelp,

    // Finding.
    /// Ctrl+F in a page: JAWS Find, announced as "Virtual Find" [obs, 506/s1].
    VirtualFind,
    /// Insert+Ctrl+F: JAWS Find, with no announcement [obs, 506/s12].
    JawsFind,
    FindNext,
    FindPrior,
    /// Finds the text, forward from the cursor.
    Find(String),

    // Lists of a page's elements.
    SelectAHeading,
    SelectALink,
    SelectAFormField,
    /// The lists of a page's elements (Virtual HTML Features, Insert+F3).
    VirtualHtmlFeatures,
    /// One of JAWS's element lists (Insert+Ctrl and a letter: buttons, tables, ...).
    SelectElement(crate::element_lists::ElementList),
    /// The page's regions (landmarks), JAWS's Insert+Ctrl+R.
    SelectARegion,
    /// Moves to the heading with this text.
    JumpToHeading(String),
    /// Moves to the link with this text.
    JumpToLink(String),
    /// Moves to the form field with this name.
    JumpToFormField(String),

    // Windows, focus and time.
    /// The next key goes to the application as it is (JAWS's Pass Key Through).
    PassKeyThrough,
    SayWindow,
    SayCurrentFocus,
    SayWindowTitle,
    SaySystemTime,
    SaySystemTimeWithSeconds,
    SaySystemDate,
    SayWeekNumber,

    // The Virtual Viewer and fonts.
    VirtualizeWindow,
    VirtualizeControl,
    SayFont,
    SayFontInViewer,

    // Okur's settings: one dialog for both, until there are enough settings for two.
    QuickSettings,
    SettingsCenter,
    /// The next punctuation level, until Okur exits (JAWS's SetPunctuationLevel).
    SetPunctuationLevel,
    /// The next typing echo, until Okur exits (Insert+2).
    NextTypingEcho,
}

impl Command {
    /// The command a name calls: a JAWS script name or the key notation of JAWS's key maps.
    pub fn from_name(name: &str) -> Option<Command> {
        use Command::*;
        use Element::*;
        let prefixed = |prefixes: &[&str]| {
            prefixes
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .map(str::to_owned)
        };
        if let Some(query) = prefixed(&["Find:", "find:", "VirtualFind:", "JAWSFind:", "^f:"]) {
            return Some(Find(query));
        }
        if let Some(text) = prefixed(&["SelectHeading:", "JumpToHeading:"]) {
            return Some(JumpToHeading(text));
        }
        if let Some(text) = prefixed(&["SelectLink:", "JumpToLink:"]) {
            return Some(JumpToLink(text));
        }
        if let Some(text) = prefixed(&["SelectFormField:", "JumpToFormField:"]) {
            return Some(JumpToFormField(text));
        }
        Some(match name {
            "SayAll" => SayAll,
            "SayLine" => SayLine,
            "SayWord" => SayWord,
            "SayCharacter" => SayCharacter,

            "{DOWN}" | "SayNextLine" => SayNextLine,
            "{UP}" | "SayPriorLine" => SayPriorLine,
            "^{RIGHT}" | "SayNextWord" => SayNextWord,
            "^{LEFT}" | "SayPriorWord" => SayPriorWord,
            "{RIGHT}" | "SayNextCharacter" => SayNextCharacter,
            "{LEFT}" | "SayPriorCharacter" => SayPriorCharacter,
            "^{DOWN}" | "SayNextParagraph" => SayNextParagraph,
            "^{UP}" | "SayPriorParagraph" => SayPriorParagraph,
            "^{HOME}" | "TopOfFile" => TopOfFile,
            "{HOME}" | "Home" | "JAWSHome" => LineStart,
            "{END}" | "End" | "JAWSEnd" => LineEnd,
            "{PGUP}" | "PageUp" | "JAWSPageUp" => PageUp,
            "{PGDN}" | "PageDown" | "JAWSPageDown" => PageDown,
            "^{END}" | "BottomOfFile" => BottomOfFile,

            "H" | "h" | "MoveToNextHeading" => MoveToNext(Heading),
            "P" | "p" => SayNextParagraph,
            "Shift+P" => SayPriorParagraph,
            "SayParagraph" => SayParagraph,
            "SaySentence" => SaySentence,
            "SayNextSentence" => SayNextSentence,
            "SayPriorSentence" => SayPriorSentence,
            "StepToStartOfElement" => StepToStartOfElement,
            "StepToEndOfElement" => StepToEndOfElement,
            "N" | "n" | "MoveToNextNonLinkText" | "Insert+Enter" => NextNonLinkText,
            "Shift+N" | "MoveToPriorNonLinkText" | "Insert+Shift+Enter" => PriorNonLinkText,
            "+H" | "+h" | "MoveToPriorHeading" => MoveToPrior(Heading),
            "1" | "MoveToNextHeadingLevelOne" => MoveToNext(HeadingLevel(1)),
            "2" | "MoveToNextHeadingLevelTwo" => MoveToNext(HeadingLevel(2)),
            "3" | "MoveToNextHeadingLevelThree" => MoveToNext(HeadingLevel(3)),
            "4" | "MoveToNextHeadingLevelFour" => MoveToNext(HeadingLevel(4)),
            "5" | "MoveToNextHeadingLevelFive" => MoveToNext(HeadingLevel(5)),
            "6" | "MoveToNextHeadingLevelSix" => MoveToNext(HeadingLevel(6)),
            "+1" | "MoveToPriorHeadingLevelOne" => MoveToPrior(HeadingLevel(1)),
            "+2" | "MoveToPriorHeadingLevelTwo" => MoveToPrior(HeadingLevel(2)),
            "+3" | "MoveToPriorHeadingLevelThree" => MoveToPrior(HeadingLevel(3)),
            "+4" | "MoveToPriorHeadingLevelFour" => MoveToPrior(HeadingLevel(4)),
            "+5" | "MoveToPriorHeadingLevelFive" => MoveToPrior(HeadingLevel(5)),
            "+6" | "MoveToPriorHeadingLevelSix" => MoveToPrior(HeadingLevel(6)),
            "U" | "u" | "MoveToNextLink" | "MoveToNextUnvisitedLink" => MoveToNext(Link),
            "+U" | "+u" | "MoveToPriorLink" | "MoveToPriorUnvisitedLink" => MoveToPrior(Link),
            "T" | "t" | "MoveToNextTable" => MoveToNext(Table),
            "+T" | "+t" | "MoveToPriorTable" => MoveToPrior(Table),
            "O" | "o" | "MoveToNextArticle" => MoveToNext(Article),
            "+O" | "+o" | "MoveToPriorArticle" => MoveToPrior(Article),
            "Q" | "q" | "+Q" | "+q" | "MoveToMainRegion" => MoveToMainRegion,
            ";" | "MoveToNextOnMouseOverElement" => MoveToNext(OnMouseOver),
            "+;" | "MoveToPriorOnMouseOverElement" => MoveToPrior(OnMouseOver),
            "'" | "MoveToNextTab" => MoveToNext(Element::Tab),
            "+'" | "MoveToPriorTab" => MoveToPrior(Element::Tab),
            "K" | "k" | "MoveToNextPlaceMarker" => MoveToNext(PlaceMarker),
            "+K" | "+k" | "MoveToPriorPlaceMarker" => MoveToPrior(PlaceMarker),
            "G" | "g" | "MoveToNextGraphic" => MoveToNext(Graphic),
            "+G" | "+g" | "MoveToPriorGraphic" => MoveToPrior(Graphic),
            "A" | "a" | "MoveToNextRadioButton" => MoveToNext(RadioButton),
            "+A" | "+a" | "MoveToPriorRadioButton" => MoveToPrior(RadioButton),
            "MoveToNextMath" => MoveToNext(Math),
            "MoveToPriorMath" => MoveToPrior(Math),
            "F" | "f" | "FocusToNextField" => MoveToNext(FormField),
            "+F" | "+f" | "FocusToPriorField" => MoveToPrior(FormField),
            "B" | "b" | "MoveToNextButton" => MoveToNext(Button),
            "+B" | "+b" | "MoveToPriorButton" => MoveToPrior(Button),
            "E" | "e" | "MoveToNextEdit" => MoveToNext(Edit),
            "+E" | "+e" | "MoveToPriorEdit" => MoveToPrior(Edit),
            "X" | "x" | "MoveToNextCheckbox" => MoveToNext(CheckBox),
            "+X" | "+x" | "MoveToPriorCheckbox" => MoveToPrior(CheckBox),
            "C" | "c" | "MoveToNextCombo" | "MoveToNextComboBox" => MoveToNext(ComboBox),
            "+C" | "+c" | "MoveToPriorCombo" | "MoveToPriorComboBox" => MoveToPrior(ComboBox),
            "L" | "l" | "MoveToNextList" => MoveToNext(List),
            "+L" | "+l" | "MoveToPriorList" => MoveToPrior(List),
            "I" | "i" | "MoveToNextListItem" => MoveToNext(ListItem),
            "+I" | "+i" | "MoveToPriorListItem" => MoveToPrior(ListItem),
            "R" | "r" | "MoveToNextLandmark" | "MoveToNextRegion" => MoveToNext(Landmark),
            "+R" | "+r" | "MoveToPriorLandmark" | "MoveToPriorRegion" => MoveToPrior(Landmark),
            "M" | "m" | "MoveToNextFrame" => MoveToNext(Frame),
            "+M" | "+m" | "MoveToPriorFrame" => MoveToPrior(Frame),

            "^%{RIGHT}" | "NextCell" | "TableCellRight" => TableCellRight,
            "^%{LEFT}" | "PriorCell" | "TableCellLeft" => TableCellLeft,
            "^%{DOWN}" | "NextRow" | "TableCellDown" => TableCellDown,
            "^%{UP}" | "PriorRow" | "TableCellUp" => TableCellUp,
            "^%{HOME}" | "FirstCell" | "TableFirstCell" => TableFirstCell,
            "^%{END}" | "LastCell" | "TableLastCell" => TableLastCell,
            "^%+LEFT" | "^%+{LEFT}" | "MoveToStartOfRow" | "RowStart" => MoveToStartOfRow,
            "^%+RIGHT" | "^%+{RIGHT}" | "MoveToEndOfRow" | "RowEnd" => MoveToEndOfRow,
            "^%+UP" | "^%+{UP}" | "MoveToTopOfColumn" | "ColumnTop" => MoveToTopOfColumn,
            "^%+DOWN" | "^%+{DOWN}" | "MoveToBottomOfColumn" | "ColumnBottom" => {
                MoveToBottomOfColumn
            }
            "^%{NUMPAD5}" | "^%5" | "SayCurrentCell" | "CurrentCell" => SayCurrentCell,
            "^%+H" | "^%+h" | "SayColumnHeader" => SayColumnHeader,
            "^%+R" | "^%+r" | "SayRowHeader" => SayRowHeader,

            "{TAB}" => NextTabStop,
            "+{TAB}" | "+TAB" => PriorTabStop,
            "{SPACE}" => Activate,
            "{ENTER}" | "EnterFormsMode" | "ActivateLink" => Enter,
            "{ESC}" | "ExitFormsMode" => Escape,
            "!{DOWN}" | "ExpandComboBox" => ExpandComboBox,
            "!{UP}" | "CollapseComboBox" => CollapseComboBox,
            "EnterMathViewer" | "ShowMathEditor" => EnterMathViewer,
            "EnterModal" => EnterModal,
            "ExitModal" => ExitModal,

            "+{RIGHT}" | "SelectNextCharacter" => SelectNextCharacter,
            "+{LEFT}" | "SelectPriorCharacter" => SelectPriorCharacter,
            "+{DOWN}" | "SelectNextLine" => SelectNextLine,
            "+{UP}" | "SelectPriorLine" => SelectPriorLine,
            "SelectNextWord" => SelectNextWord,
            "SelectPriorWord" => SelectPriorWord,
            "SelectToEndOfLine" => SelectToLineEnd,
            "SelectToStartOfLine" => SelectToLineStart,
            "SelectToBottom" => SelectToBottom,
            "SelectToTop" => SelectToTop,
            "SelectNextScreen" => SelectNextPage,
            "SelectPriorScreen" => SelectPriorPage,
            "SelectAll" => SelectAll,
            "SelectHTMLElement" => SelectHtmlElement,
            "CopySelectedTextToClipboard" => CopySelection,
            "+!{DOWN}" | "SaySelection" | "SaySelectedText" => SaySelectedText,
            "SayTopLineOfWindow" | "Insert+End" => SayTopLineOfWindow,
            "SayBottomLineOfWindow" | "Insert+PageDown" => SayBottomLineOfWindow,
            "SayDefaultButton" | "Insert+E" => SayDefaultButton,
            "SayConfiguration" | "Insert+Q" => SayConfiguration,
            "SayHotKey" | "Shift+Numpad5" => SayHotKey,
            "IndicateLinkSourceURL" | "Ctrl+Insert+F7" => SayLinkAddress,
            "RouteVirtualToPc" | "Insert+Delete" => RouteVirtualToPc,
            "RoutePCToVirtual" | "Ctrl+Insert+Delete" => RoutePcToVirtual,
            "SelectAFrame" | "Insert+F9" => SelectAFrame,
            "LeftMouseButton" | "NumpadSlash" => LeftMouseClick,
            "WindowList" | "Insert+F10" => WindowList,
            "ScreenSensitiveHelp" | "Insert+F1" => ScreenSensitiveHelp,
            "HotKeyHelp" | "Insert+H" => HotKeyHelp,
            "WindowKeysHelp" | "WindowsKeysHelp" | "Insert+W" => WindowsKeysHelp,
            "Ctrl+Insert+1" => SayListColumn(1),
            "Ctrl+Insert+2" => SayListColumn(2),
            "Ctrl+Insert+3" => SayListColumn(3),
            "Insert+Space" | "JAWSKey+Space" => LayerStart,
            "NavigationModeToggle" | "Insert+N" => NavigationModeToggle,
            "QuickNavigationKeysSuspendOrResume" | "Insert+Shift+Z" => QuickKeysSuspendResume,
            "VirtualPCCursorToggle" | "Insert+Z" => VirtualCursorToggle,
            "RefreshScreen" | "Insert+Escape" => RefreshScreen,
            "SayToCursor" | "Insert+Home" => SayToCursor,
            "SayFromCursor" | "Insert+PageUp" => SayFromCursor,

            "^f" | "Ctrl+F" | "VirtualFind" => VirtualFind,
            "JAWSFind" | "Insert+Ctrl+F" | "JAWSKey+Ctrl+F" => JawsFind,
            "{F3}" | "F3" | "JAWSFindNext" | "FindNext" => FindNext,
            "+{F3}" | "+F3" | "Shift+F3" | "JAWSFindPrior" | "FindPrior" => FindPrior,

            "SelectAHeading" | "HeadingList" | "HeadingsList" | "Insert+F6" | "JAWSKey+F6" => {
                SelectAHeading
            }
            "SelectALink" | "LinkList" | "LinksList" | "Insert+F7" | "JAWSKey+F7" => SelectALink,
            "VirtualHTMLFeatures" | "Insert+F3" | "JAWSKey+F3" => VirtualHtmlFeatures,
            "SelectARadioButtonFormField" => SelectElement(ElementList::RadioButtons),
            "SelectAButtonFormField" => SelectElement(ElementList::Buttons),
            "SelectAComboFormField" => SelectElement(ElementList::Controls),
            "SelectAnEditFormField" => SelectElement(ElementList::Edits),
            "SelectACheckboxFormField" => SelectElement(ElementList::CheckBoxes),
            "SelectAGraphic" => SelectElement(ElementList::Graphics),
            "SelectAList" => SelectElement(ElementList::Lists),
            "SelectAnArticle" => SelectElement(ElementList::Articles),
            "SelectAParagraph" => SelectElement(ElementList::Paragraphs),
            "SelectABlockQuote" => SelectElement(ElementList::BlockQuotes),
            "SelectATable" => SelectElement(ElementList::Tables),
            "SelectADivision" => SelectElement(ElementList::Divisions),
            "SelectATabFormField" => SelectElement(ElementList::Tabs),
            "SelectAFormField" | "FormFieldList" | "FormFieldsList" | "Insert+F5"
            | "JAWSKey+F5" => SelectAFormField,
            "SelectARegion" | "RegionsList" | "Insert+Ctrl+R" | "JAWSKey+Ctrl+R" => SelectARegion,

            "PassKeyThrough" | "Insert+3" | "JAWSKey+3" => PassKeyThrough,
            "SayWindow" => SayWindow,
            "SayCurrentFocus" => SayCurrentFocus,
            "SayWindowTitle" => SayWindowTitle,
            "SaySystemTime" => SaySystemTime,
            "SaySystemTimeWithSeconds" => SaySystemTimeWithSeconds,
            "SaySystemDate" => SaySystemDate,
            "SayWeekNumber" => SayWeekNumber,

            "VirtualizeWindow" => VirtualizeWindow,
            "VirtualizeControl" => VirtualizeControl,
            "Insert+F" | "SayFont" | "WhatFont" => SayFont,
            "SayFontInViewer" => SayFontInViewer,
            "QuickSettings" => QuickSettings,
            "SettingsCenter" => SettingsCenter,
            "SetPunctuationLevel" | "Insert+Shift+2" | "JAWSKey+Shift+2" => SetPunctuationLevel,
            "Insert+2" | "JAWSKey+2" => NextTypingEcho,
            _ => return None,
        })
    }

    /// The command pressing this command's keys twice quickly runs, e.g. Say System Time's
    /// second press speaks the date [obs, 308/s2, s4].
    pub fn pressed_twice(&self) -> Option<Command> {
        Some(match self {
            Command::SaySystemTime => Command::SaySystemDate,
            Command::SaySystemTimeWithSeconds => Command::SayWeekNumber,
            Command::SayFont => Command::SayFontInViewer,
            Command::SayCharacter => Command::SayCharacterPhonetic,
            Command::SayWord => Command::SpellWord,
            Command::SayLine => Command::SpellLine,
            Command::SaySelectedText => Command::SpellSelectedText,
            Command::SayToCursor => Command::SpellToCursor,
            Command::SayFromCursor => Command::SpellFromCursor,
            _ => return None,
        })
    }

    /// The command pressing this command's keys three times quickly runs: Say Character's
    /// third press says the character's value [obs, 401/s3].
    pub fn pressed_thrice(&self) -> Option<Command> {
        Some(match self {
            Command::SayCharacter => Command::SayCharacterValue,
            _ => return None,
        })
    }
}

/// Whether a modifier must be held, must not be held, or does not matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Held {
    Yes,
    No,
    Any,
}

impl Held {
    fn matches(self, down: bool) -> bool {
        match self {
            Held::Yes => down,
            Held::No => !down,
            Held::Any => true,
        }
    }
}

/// The modifiers a binding needs. `insert` is the JAWS key, whichever key that is.
#[derive(Debug, Clone, Copy)]
struct Chord {
    ctrl: Held,
    alt: Held,
    shift: Held,
    insert: Held,
    win: Held,
}

/// No modifier at all.
const ALONE: Chord = Chord {
    ctrl: Held::No,
    alt: Held::No,
    shift: Held::No,
    insert: Held::No,
    win: Held::No,
};

/// Any modifiers; narrowed with the methods below.
const ANY: Chord = Chord {
    ctrl: Held::Any,
    alt: Held::Any,
    shift: Held::Any,
    insert: Held::Any,
    win: Held::Any,
};

impl Chord {
    const fn ctrl(self, ctrl: Held) -> Self {
        Self { ctrl, ..self }
    }
    const fn alt(self, alt: Held) -> Self {
        Self { alt, ..self }
    }
    const fn shift(self, shift: Held) -> Self {
        Self { shift, ..self }
    }
    const fn insert(self, insert: Held) -> Self {
        Self { insert, ..self }
    }
    const fn win(self, win: Held) -> Self {
        Self { win, ..self }
    }

    fn matches(&self, event: &KeyEvent) -> bool {
        let m = &event.modifiers;
        self.ctrl.matches(m.ctrl)
            && self.alt.matches(m.alt)
            && self.shift.matches(m.shift)
            && self.insert.matches(m.insert)
            && self.win.matches(m.win)
    }
}

/// A key with the modifiers it needs, and the command it runs.
struct Binding {
    /// Letters are upper case and match either case.
    key: Key,
    chord: Chord,
    command: Command,
}

const fn bind(key: Key, chord: Chord, command: Command) -> Binding {
    Binding {
        key,
        chord,
        command,
    }
}

use Held::{No, Yes};

/// Only Shift, with or without Windows or the JAWS key: quick navigation's "prior" keys.
const SHIFT: Chord = ANY.shift(Yes).ctrl(No).alt(No);
/// The JAWS key alone.
const JAWS: Chord = ANY.insert(Yes).ctrl(No).alt(No).shift(No);
/// Ctrl alone.
const CTRL: Chord = ANY.ctrl(Yes).shift(No).alt(No).insert(No);
/// Ctrl and Shift.
const CTRL_SHIFT: Chord = ANY.ctrl(Yes).shift(Yes).alt(No).insert(No);
/// The JAWS key with Ctrl.
const JAWS_CTRL: Chord = ANY.insert(Yes).ctrl(Yes).shift(No).alt(No);
/// The JAWS key with Shift.
const JAWS_SHIFT: Chord = ANY.insert(Yes).shift(Yes).ctrl(No).alt(No);
/// Ctrl and Alt, with or without Shift.
const CTRL_ALT: Chord = ANY.ctrl(Yes).alt(Yes);
/// Ctrl and the JAWS key: the element lists.
const CTRL_INSERT: Chord = ANY.ctrl(Yes).insert(Yes).alt(No).shift(No);
/// Windows and Alt.
const WIN_ALT: Chord = ANY.win(Yes).alt(Yes).ctrl(No).shift(No).insert(No);
/// Ctrl, Alt and the JAWS key.
const CTRL_ALT_INSERT: Chord = ANY.ctrl(Yes).alt(Yes).insert(Yes).shift(No);

/// The key map, JAWS's desktop layout: the first binding that matches a key wins.
static BINDINGS: &[Binding] = &{
    use Command::*;
    use Element::*;
    use Key::*;
    [
        // Selecting text.
        bind(RightArrow, SHIFT.insert(No), SelectNextCharacter),
        bind(LeftArrow, SHIFT.insert(No), SelectPriorCharacter),
        bind(DownArrow, SHIFT.insert(No), SelectNextLine),
        bind(UpArrow, SHIFT.insert(No), SelectPriorLine),
        // Say All and Say Line.
        bind(DownArrow, JAWS, SayAll),
        bind(UpArrow, JAWS, SayLine),
        bind(RightArrow, JAWS, SayNextWord),
        bind(LeftArrow, JAWS, SayPriorWord),
        // Say Selected Text: the JAWS key with Shift (Alt+Shift+Down is a JAWS cursor key in
        // Default.JKM, MouseDown).
        bind(
            DownArrow,
            ANY.insert(Yes).shift(Yes).ctrl(No),
            SaySelectedText,
        ),
        // The quick keys and the virtual cursor off and on, the screen refreshed, the rate for
        // now [obs, 298, 512].
        bind(Space, JAWS, LayerStart),
        bind(Alpha('N'), JAWS, NavigationModeToggle),
        bind(Alpha('Z'), JAWS_SHIFT, QuickKeysSuspendResume),
        bind(Alpha('N'), CTRL_INSERT, QuickKeysSuspendResume),
        bind(Alpha('Z'), JAWS, VirtualCursorToggle),
        bind(Key::Escape, JAWS, RefreshScreen),
        bind(
            Key::PageUp,
            ANY.ctrl(Yes).alt(Yes).shift(No).insert(No),
            RateFaster,
        ),
        bind(
            Key::PageDown,
            ANY.ctrl(Yes).alt(Yes).shift(No).insert(No),
            RateSlower,
        ),
        // The window's top line, and the line to and from the cursor.
        bind(End, JAWS, SayTopLineOfWindow),
        bind(Key::PageDown, JAWS, SayBottomLineOfWindow),
        bind(Home, JAWS, SayToCursor),
        bind(Key::PageUp, JAWS, SayFromCursor),
        // Table rows and columns.
        bind(DownArrow, CTRL_ALT.shift(Yes), MoveToBottomOfColumn),
        bind(UpArrow, CTRL_ALT.shift(Yes), MoveToTopOfColumn),
        bind(RightArrow, CTRL_ALT.shift(Yes), MoveToEndOfRow),
        bind(LeftArrow, CTRL_ALT.shift(Yes), MoveToStartOfRow),
        bind(DownArrow, CTRL_ALT, TableCellDown),
        bind(UpArrow, CTRL_ALT, TableCellUp),
        bind(RightArrow, CTRL_ALT, TableCellRight),
        bind(LeftArrow, CTRL_ALT, TableCellLeft),
        // A table's rows and columns read whole: Windows+Alt and an arrow, Numpad5 or Comma
        // the row, Period the column (JAWS 2025 help, Keystrokes for Working in Tables)
        // [obs, 297]. Before Alt+Down, which they would otherwise be.
        bind(DownArrow, WIN_ALT, ReadNextRow),
        bind(UpArrow, WIN_ALT, ReadPriorRow),
        bind(RightArrow, WIN_ALT, ReadNextColumn),
        bind(LeftArrow, WIN_ALT, ReadPriorColumn),
        bind(Numpad5, WIN_ALT, SayCurrentRow),
        bind(Comma, WIN_ALT, SayCurrentRow),
        bind(Period, WIN_ALT, SayCurrentColumn),
        // The start and the end of the element (Default.JKM's [virtual keys]) [obs, 294].
        bind(Home, WIN_ALT, StepToStartOfElement),
        bind(End, WIN_ALT, StepToEndOfElement),
        // Combo boxes.
        bind(DownArrow, ANY.alt(Yes).ctrl(No).shift(No), ExpandComboBox),
        bind(UpArrow, ANY.alt(Yes).ctrl(No).shift(No), CollapseComboBox),
        // Selecting by word, before the word keys that would take it [obs, 501/s5].
        bind(RightArrow, CTRL_SHIFT, SelectNextWord),
        bind(LeftArrow, CTRL_SHIFT, SelectPriorWord),
        // Paragraphs and words.
        bind(DownArrow, ANY.ctrl(Yes), SayNextParagraph),
        bind(UpArrow, ANY.ctrl(Yes), SayPriorParagraph),
        bind(RightArrow, ANY.ctrl(Yes), SayNextWord),
        bind(LeftArrow, ANY.ctrl(Yes), SayPriorWord),
        // Lines and characters.
        bind(DownArrow, ALONE, SayNextLine),
        bind(UpArrow, ALONE, SayPriorLine),
        bind(RightArrow, ALONE, SayNextCharacter),
        bind(LeftArrow, ALONE, SayPriorCharacter),
        // The keypad's 5: Say Character, and with the JAWS key Say Word (JAWS's keystroke
        // list; research Item 4).
        // Sentences and the paragraph (JAWS 2025 help, Keystrokes for Reading Text).
        bind(
            NumpadPlus,
            ANY.alt(Yes).ctrl(No).shift(No).insert(No),
            SayNextSentence,
        ),
        bind(
            NumpadMinus,
            ANY.alt(Yes).ctrl(No).shift(No).insert(No),
            SayPriorSentence,
        ),
        bind(
            Numpad5,
            ANY.alt(Yes).ctrl(No).shift(No).insert(No),
            SaySentence,
        ),
        bind(Numpad5, CTRL, SayParagraph),
        bind(Numpad5, SHIFT, SayHotKey),
        bind(Numpad5, JAWS, SayWord),
        bind(Numpad5, ALONE, SayCharacter),
        // The first and last heading, of any level and of one, and the first and last form
        // field (Default.JKM's [virtual keys]) [obs, 295, 296]; before the Ctrl and Ctrl+Alt
        // keys that would take them.
        bind(
            Home,
            ANY.alt(Yes).insert(Yes).ctrl(No).shift(No),
            FirstHeading,
        ),
        bind(
            End,
            ANY.alt(Yes).insert(Yes).ctrl(No).shift(No),
            LastHeading,
        ),
        bind(Digit(1), CTRL_ALT_INSERT, FirstHeadingAtLevel(1)),
        bind(Digit(2), CTRL_ALT_INSERT, FirstHeadingAtLevel(2)),
        bind(Digit(3), CTRL_ALT_INSERT, FirstHeadingAtLevel(3)),
        bind(Digit(4), CTRL_ALT_INSERT, FirstHeadingAtLevel(4)),
        bind(Digit(5), CTRL_ALT_INSERT, FirstHeadingAtLevel(5)),
        bind(Digit(6), CTRL_ALT_INSERT, FirstHeadingAtLevel(6)),
        bind(Digit(1), CTRL_ALT_INSERT.shift(Yes), LastHeadingAtLevel(1)),
        bind(Digit(2), CTRL_ALT_INSERT.shift(Yes), LastHeadingAtLevel(2)),
        bind(Digit(3), CTRL_ALT_INSERT.shift(Yes), LastHeadingAtLevel(3)),
        bind(Digit(4), CTRL_ALT_INSERT.shift(Yes), LastHeadingAtLevel(4)),
        bind(Digit(5), CTRL_ALT_INSERT.shift(Yes), LastHeadingAtLevel(5)),
        bind(Digit(6), CTRL_ALT_INSERT.shift(Yes), LastHeadingAtLevel(6)),
        bind(Home, CTRL_INSERT, FirstFormField),
        bind(End, CTRL_INSERT, LastFormField),
        // Selecting with the virtual cursor (Default.JKM's Select scripts), before the plain
        // Ctrl keys that would take them [obs, 501].
        bind(End, CTRL_SHIFT, SelectToBottom),
        bind(Home, CTRL_SHIFT, SelectToTop),
        bind(End, SHIFT.insert(No), SelectToLineEnd),
        bind(Home, SHIFT.insert(No), SelectToLineStart),
        bind(Key::PageDown, SHIFT.insert(No), SelectNextPage),
        bind(Key::PageUp, SHIFT.insert(No), SelectPriorPage),
        bind(Alpha('A'), CTRL, SelectAll),
        bind(F(8), ALONE, SelectHtmlElement),
        bind(Alpha('C'), CTRL, CopySelection),
        // Top and bottom, of a table or the page.
        bind(Home, CTRL_ALT, TableFirstCell),
        bind(End, CTRL_ALT, TableLastCell),
        bind(Home, ANY.ctrl(Yes), TopOfFile),
        bind(End, ANY.ctrl(Yes), BottomOfFile),
        bind(Home, ALONE, LineStart),
        bind(End, ALONE, LineEnd),
        bind(Key::PageUp, ALONE, Command::PageUp),
        bind(Key::PageDown, ALONE, Command::PageDown),
        // A page's controls.
        bind(Key::Tab, ANY.shift(Yes), PriorTabStop),
        bind(Key::Tab, ALONE, NextTabStop),
        bind(Key::Enter, ALONE, Command::Enter),
        bind(Key::Escape, ALONE, Command::Escape),
        bind(Space, ALONE, Activate),
        // Windows and focus.
        bind(Digit(3), JAWS, PassKeyThrough),
        bind(Alpha('B'), JAWS, SayWindow),
        bind(Key::Tab, JAWS, SayCurrentFocus),
        bind(Alpha('T'), JAWS, SayWindowTitle),
        bind(
            F(12),
            ANY.insert(Yes).alt(No).shift(No).ctrl(Yes),
            SaySystemTimeWithSeconds,
        ),
        bind(
            F(12),
            ANY.insert(Yes).alt(No).shift(No).ctrl(No),
            SaySystemTime,
        ),
        // The Virtual Viewer and fonts.
        bind(
            Alpha('W'),
            ANY.insert(Yes).alt(Yes).ctrl(No).shift(No),
            VirtualizeWindow,
        ),
        bind(
            Alpha('V'),
            ANY.insert(Yes).shift(Yes).ctrl(No).alt(No),
            VirtualizeControl,
        ),
        bind(Alpha('F'), JAWS, SayFont),
        // Settings.
        bind(Alpha('V'), JAWS, QuickSettings),
        bind(Digit(6), JAWS, SettingsCenter),
        // The dialog box's default button, and the settings in use (JAWS 2025 help) [obs, 513].
        bind(Alpha('E'), JAWS, SayDefaultButton),
        bind(Alpha('Q'), JAWS, SayConfiguration),
        // A page's link address, routing the cursors, frames, and the prior heading by the
        // Ctrl+Insert+Shift+Enter of the key map (Default.JKM) [obs, 536].
        bind(Key::F(7), CTRL_INSERT, SayLinkAddress),
        bind(Key::Delete, JAWS, RouteVirtualToPc),
        bind(Key::Delete, CTRL_INSERT, RoutePcToVirtual),
        bind(Key::F(9), JAWS, SelectAFrame),
        bind(Key::NumpadSlash, ALONE, LeftMouseClick),
        bind(Key::F(10), JAWS, WindowList),
        // Help (JAWS 2025 help, Keystrokes for Accessing Help) [obs, 526, 527]. In the laptop
        // layout CapsLock+H is Say Sentence, as there.
        bind(Key::F(1), JAWS, ScreenSensitiveHelp),
        bind(Alpha('H'), JAWS, HotKeyHelp),
        bind(Alpha('W'), JAWS, WindowsKeysHelp),
        bind(
            Key::Enter,
            ANY.ctrl(Yes).insert(Yes).shift(Yes).alt(No),
            MoveToPrior(Heading),
        ),
        // A list view's columns (JAWS 2025 help, Keystrokes for Dialog Boxes) [obs, 514].
        bind(Digit(1), CTRL_INSERT, SayListColumn(1)),
        bind(Digit(2), CTRL_INSERT, SayListColumn(2)),
        bind(Digit(3), CTRL_INSERT, SayListColumn(3)),
        bind(Digit(4), CTRL_INSERT, SayListColumn(4)),
        bind(Digit(5), CTRL_INSERT, SayListColumn(5)),
        bind(Digit(6), CTRL_INSERT, SayListColumn(6)),
        bind(Digit(7), CTRL_INSERT, SayListColumn(7)),
        bind(Digit(8), CTRL_INSERT, SayListColumn(8)),
        bind(Digit(9), CTRL_INSERT, SayListColumn(9)),
        bind(Digit(0), CTRL_INSERT, SayListColumn(10)),
        // Before Shift+2 (the prior heading of level 2), which also matches with the JAWS key.
        bind(
            Digit(2),
            ANY.insert(Yes).shift(Yes).ctrl(No).alt(No),
            SetPunctuationLevel,
        ),
        bind(Digit(2), JAWS, NextTypingEcho),
        // JAWS Find: Ctrl+F in a page, Insert+Ctrl+F anywhere (Default.JKM: VirtualFind, JAWSFind).
        bind(Alpha('F'), JAWS_CTRL, JawsFind),
        bind(Alpha('F'), CTRL, VirtualFind),
        bind(
            Alpha('R'),
            ANY.ctrl(Yes).insert(Yes).alt(No).shift(No),
            SelectARegion,
        ),
        // JAWS's element lists: Insert+Ctrl and the element's letter (Default.JKM).
        bind(
            Alpha('A'),
            CTRL_INSERT,
            SelectElement(ElementList::RadioButtons),
        ),
        bind(Alpha('B'), CTRL_INSERT, SelectElement(ElementList::Buttons)),
        bind(
            Alpha('C'),
            CTRL_INSERT,
            SelectElement(ElementList::Controls),
        ),
        bind(Alpha('E'), CTRL_INSERT, SelectElement(ElementList::Edits)),
        bind(
            Alpha('X'),
            CTRL_INSERT,
            SelectElement(ElementList::CheckBoxes),
        ),
        bind(
            Alpha('G'),
            CTRL_INSERT,
            SelectElement(ElementList::Graphics),
        ),
        bind(Alpha('L'), CTRL_INSERT, SelectElement(ElementList::Lists)),
        bind(
            Alpha('O'),
            CTRL_INSERT,
            SelectElement(ElementList::Articles),
        ),
        bind(
            Alpha('P'),
            CTRL_INSERT,
            SelectElement(ElementList::Paragraphs),
        ),
        bind(
            Alpha('Q'),
            CTRL_INSERT,
            SelectElement(ElementList::BlockQuotes),
        ),
        bind(Alpha('T'), CTRL_INSERT, SelectElement(ElementList::Tables)),
        bind(
            Alpha('Z'),
            CTRL_INSERT,
            SelectElement(ElementList::Divisions),
        ),
        bind(Apostrophe, CTRL_INSERT, SelectElement(ElementList::Tabs)),
        // Table headers.
        bind(Alpha('H'), CTRL_ALT.shift(Yes), SayColumnHeader),
        bind(Alpha('R'), CTRL_ALT.shift(Yes), SayRowHeader),
        // Find next and prior, and the element lists.
        bind(F(3), SHIFT.insert(No), FindPrior),
        bind(F(3), ALONE, FindNext),
        bind(F(3), JAWS, VirtualHtmlFeatures),
        bind(F(5), JAWS, SelectAFormField),
        bind(F(6), JAWS, SelectAHeading),
        bind(F(7), JAWS, SelectALink),
        // Say Current Cell: Ctrl+Alt+Numpad5, as JAWS documents it (Keystrokes for Working in
        // Tables), and with the number row's 5.
        bind(Numpad5, CTRL_ALT, SayCurrentCell),
        bind(Digit(5), CTRL_ALT, SayCurrentCell),
        // Quick navigation: the key moves to the next, Shift with it to the prior.
        bind(Alpha('H'), SHIFT, MoveToPrior(Heading)),
        bind(Alpha('H'), ALONE, MoveToNext(Heading)),
        bind(Alpha('P'), SHIFT, SayPriorParagraph),
        bind(Alpha('P'), ALONE, SayNextParagraph),
        bind(Alpha('N'), SHIFT, PriorNonLinkText),
        bind(Alpha('N'), ALONE, NextNonLinkText),
        bind(
            Key::Enter,
            ANY.insert(Yes).shift(Yes).ctrl(No).alt(No),
            PriorNonLinkText,
        ),
        bind(Key::Enter, JAWS, NextNonLinkText),
        bind(Alpha('T'), SHIFT, MoveToPrior(Table)),
        bind(Alpha('T'), ALONE, MoveToNext(Table)),
        bind(Alpha('K'), SHIFT, MoveToPrior(PlaceMarker)),
        bind(Alpha('K'), ALONE, MoveToNext(PlaceMarker)),
        bind(Alpha('U'), SHIFT, MoveToPrior(Link)),
        bind(Alpha('U'), ALONE, MoveToNext(Link)),
        bind(Alpha('F'), SHIFT, MoveToPrior(FormField)),
        bind(Alpha('F'), ALONE, MoveToNext(FormField)),
        bind(Alpha('B'), SHIFT, MoveToPrior(Button)),
        bind(Alpha('B'), ALONE, MoveToNext(Button)),
        bind(Alpha('E'), SHIFT, MoveToPrior(Edit)),
        bind(Alpha('E'), ALONE, MoveToNext(Edit)),
        bind(Alpha('X'), SHIFT, MoveToPrior(CheckBox)),
        bind(Alpha('X'), ALONE, MoveToNext(CheckBox)),
        bind(Alpha('Z'), SHIFT, MoveToPrior(Division)),
        bind(Alpha('Z'), ALONE, MoveToNext(Division)),
        bind(Minus, SHIFT, MoveToPrior(Separator)),
        bind(Minus, ALONE, MoveToNext(Separator)),
        bind(Backslash, SHIFT, MoveToPrior(MailLink)),
        bind(Backslash, ALONE, MoveToNext(MailLink)),
        bind(Comma, SHIFT.insert(No), StepToStartOfElement),
        bind(Period, SHIFT.insert(No), StepToEndOfElement),
        bind(Alpha('L'), SHIFT, MoveToPrior(List)),
        bind(Alpha('L'), ALONE, MoveToNext(List)),
        bind(Alpha('I'), SHIFT, MoveToPrior(ListItem)),
        bind(Alpha('I'), ALONE, MoveToNext(ListItem)),
        bind(Alpha('R'), SHIFT, MoveToPrior(Landmark)),
        bind(Alpha('R'), ALONE, MoveToNext(Landmark)),
        bind(Alpha('C'), SHIFT, MoveToPrior(ComboBox)),
        bind(Alpha('C'), ALONE, MoveToNext(ComboBox)),
        bind(Alpha('M'), SHIFT, MoveToPrior(Frame)),
        bind(Alpha('M'), ALONE, MoveToNext(Frame)),
        bind(Alpha('G'), SHIFT, MoveToPrior(Graphic)),
        bind(Alpha('G'), ALONE, MoveToNext(Graphic)),
        bind(Alpha('A'), SHIFT, MoveToPrior(RadioButton)),
        bind(Alpha('A'), ALONE, MoveToNext(RadioButton)),
        bind(Alpha('O'), SHIFT, MoveToPrior(Article)),
        bind(Alpha('O'), ALONE, MoveToNext(Article)),
        // Q and Shift+Q both move to the main region.
        bind(Alpha('Q'), SHIFT, MoveToMainRegion),
        bind(Alpha('Q'), ALONE, MoveToMainRegion),
        bind(Digit(1), SHIFT, MoveToPrior(HeadingLevel(1))),
        bind(Digit(1), ALONE, MoveToNext(HeadingLevel(1))),
        bind(Digit(2), SHIFT, MoveToPrior(HeadingLevel(2))),
        bind(Digit(2), ALONE, MoveToNext(HeadingLevel(2))),
        bind(Digit(3), SHIFT, MoveToPrior(HeadingLevel(3))),
        bind(Digit(3), ALONE, MoveToNext(HeadingLevel(3))),
        bind(Digit(4), SHIFT, MoveToPrior(HeadingLevel(4))),
        bind(Digit(4), ALONE, MoveToNext(HeadingLevel(4))),
        bind(Digit(5), SHIFT, MoveToPrior(HeadingLevel(5))),
        bind(Digit(5), ALONE, MoveToNext(HeadingLevel(5))),
        bind(Digit(6), SHIFT, MoveToPrior(HeadingLevel(6))),
        bind(Digit(6), ALONE, MoveToNext(HeadingLevel(6))),
        bind(Semicolon, SHIFT, MoveToPrior(OnMouseOver)),
        bind(Semicolon, ALONE, MoveToNext(OnMouseOver)),
        bind(Apostrophe, SHIFT, MoveToPrior(Element::Tab)),
        bind(Apostrophe, ALONE, MoveToNext(Element::Tab)),
    ]
};

/// The laptop layout's own reading keys: the JAWS key alone with a letter (JAWS 2025, laptop
/// layout, observed on the dev box: CapsLock+I "First line of the viewer test", CapsLock+K
/// "First"). Checked before [`BINDINGS`].
static LAPTOP_BINDINGS: &[Binding] = &{
    use Command::*;
    use Key::{Alpha, Comma, Period};
    [
        // CapsLock+Comma, Say Character; CapsLock+Period, Say Next Character (research
        // Item 4).
        bind(Comma, JAWS, SayCharacter),
        bind(Period, JAWS, SayNextCharacter),
        bind(Alpha('I'), JAWS, SayLine),
        bind(Alpha('K'), JAWS, SayWord),
        bind(Alpha('O'), JAWS, SayNextLine),
        bind(Alpha('U'), JAWS, SayPriorLine),
        bind(Alpha('L'), JAWS, SayNextWord),
        bind(Alpha('J'), JAWS, SayPriorWord),
        bind(Alpha('M'), JAWS, SayPriorCharacter),
        // Laptop Keystrokes for Reading Text (JAWS 2025 help).
        bind(Alpha('A'), JAWS, SayAll),
        bind(Alpha('H'), JAWS, SaySentence),
        bind(Alpha('N'), JAWS, SayNextSentence),
        bind(Alpha('Y'), JAWS, SayPriorSentence),
        bind(Alpha('I'), JAWS_CTRL, SayParagraph),
        bind(Alpha('O'), JAWS_CTRL, NextParagraphOrArticles),
        bind(Alpha('U'), JAWS_CTRL, SayPriorParagraph),
        bind(Alpha('A'), JAWS_SHIFT, SaySelectedText),
        bind(Alpha('Y'), JAWS_SHIFT, SayTopLineOfWindow),
        bind(Alpha('N'), JAWS_SHIFT, SayBottomLineOfWindow),
        bind(Alpha('J'), JAWS_SHIFT, SayToCursor),
        bind(Alpha('L'), JAWS_SHIFT, SayFromCursor),
    ]
};

fn lookup(bindings: &[Binding], event: &KeyEvent) -> Option<Command> {
    if !event.is_down {
        return None;
    }
    let key = match event.key {
        Key::Alpha(c) => Key::Alpha(c.to_ascii_uppercase()),
        key => key,
    };
    bindings
        .iter()
        .find(|b| b.key == key && b.chord.matches(event))
        .map(|b| b.command.clone())
}

/// The command a key runs in JAWS's desktop key map. `event` is resolved: `modifiers.insert`
/// is the JAWS key ([`KeyboardSettings::resolve`]).
pub fn command_for(event: &KeyEvent) -> Option<Command> {
    lookup(BINDINGS, event)
}

/// The command a key runs only in `keyboard`'s layout, where that layout has its own: the
/// laptop layout's reading keys.
pub fn layout_command_for(keyboard: &KeyboardSettings, event: &KeyEvent) -> Option<Command> {
    match keyboard.layout {
        // The laptop keys are CapsLock's; Insert keeps the desktop's keys in the laptop layout.
        KeyboardLayout::Laptop if event.modifiers.caps_lock => lookup(LAPTOP_BINDINGS, event),
        KeyboardLayout::Laptop => None,
        KeyboardLayout::Desktop => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Modifiers;

    fn key(key: Key, modifiers: Modifiers) -> KeyEvent {
        KeyEvent::new(key, modifiers, true)
    }

    fn with(f: impl FnOnce(&mut Modifiers)) -> Modifiers {
        let mut m = Modifiers::none();
        f(&mut m);
        m
    }

    #[test]
    fn keys_run_their_commands() {
        use Command::*;
        let none = Modifiers::none();
        let ctrl = with(|m| m.ctrl = true);
        let shift = with(|m| m.shift = true);
        let ctrl_alt = with(|m| {
            m.ctrl = true;
            m.alt = true
        });
        let jaws = with(|m| m.insert = true);
        let jaws_shift = with(|m| {
            m.insert = true;
            m.shift = true
        });
        let cases = [
            (key(Key::DownArrow, none), Some(SayNextLine)),
            (key(Key::UpArrow, none), Some(SayPriorLine)),
            (key(Key::RightArrow, ctrl), Some(SayNextWord)),
            (key(Key::LeftArrow, ctrl), Some(SayPriorWord)),
            (key(Key::RightArrow, ctrl_alt), Some(TableCellRight)),
            (key(Key::Home, ctrl), Some(TopOfFile)),
            (
                key(Key::Alpha('h'), none),
                Some(MoveToNext(Element::Heading)),
            ),
            (
                key(Key::Alpha('h'), shift),
                Some(MoveToPrior(Element::Heading)),
            ),
            (
                key(Key::Digit(2), none),
                Some(MoveToNext(Element::HeadingLevel(2))),
            ),
            (
                key(Key::Digit(2), shift),
                Some(MoveToPrior(Element::HeadingLevel(2))),
            ),
            (key(Key::Tab, none), Some(NextTabStop)),
            (key(Key::Tab, shift), Some(PriorTabStop)),
            (key(Key::Enter, none), Some(Enter)),
            (key(Key::Escape, none), Some(Escape)),
            (key(Key::Space, none), Some(Activate)),
            (key(Key::RightArrow, shift), Some(SelectNextCharacter)),
            (key(Key::DownArrow, jaws_shift), Some(SaySelectedText)),
            (key(Key::Alpha('f'), jaws), Some(SayFont)),
            (key(Key::Alpha('q'), none), Some(MoveToMainRegion)),
            (key(Key::Alpha('q'), shift), Some(MoveToMainRegion)),
            (key(Key::F(3), jaws), Some(VirtualHtmlFeatures)),
        ];
        for (event, command) in cases {
            assert_eq!(command_for(&event), command, "{event:?}");
        }
        assert_eq!(
            command_for(&KeyEvent::new(Key::DownArrow, none, false)),
            None,
            "a key's release runs nothing"
        );
    }

    #[test]
    fn the_laptop_layout_reads_with_the_jaws_key_and_letters() {
        let laptop = KeyboardSettings::for_layout(KeyboardLayout::Laptop);
        // CapsLock held as the JAWS key, as the keyboard settings resolve it.
        let jaws = with(|m| {
            m.insert = true;
            m.caps_lock = true
        });
        // Insert keeps the desktop's keys in the laptop layout: Insert+N is not CapsLock+N
        // [obs, 298/s1].
        assert_eq!(
            layout_command_for(&laptop, &key(Key::Alpha('n'), with(|m| m.insert = true))),
            None
        );
        assert_eq!(
            layout_command_for(&laptop, &key(Key::Alpha('o'), jaws)),
            Some(Command::SayNextLine)
        );
        assert_eq!(
            layout_command_for(&KeyboardSettings::default(), &key(Key::Alpha('o'), jaws)),
            None
        );
        let jaws_shift = with(|m| {
            m.insert = true;
            m.caps_lock = true;
            m.shift = true
        });
        assert_eq!(
            layout_command_for(&laptop, &key(Key::Alpha('o'), jaws_shift)),
            None
        );
        // CapsLock+Comma and CapsLock+Period; the keypad's 5 in the desktop layout.
        assert_eq!(
            layout_command_for(&laptop, &key(Key::Comma, jaws)),
            Some(Command::SayCharacter)
        );
        assert_eq!(
            layout_command_for(&laptop, &key(Key::Period, jaws)),
            Some(Command::SayNextCharacter)
        );
        assert_eq!(
            command_for(&key(Key::Numpad5, Modifiers::none())),
            Some(Command::SayCharacter)
        );
        assert_eq!(
            command_for(&key(Key::Numpad5, jaws)),
            Some(Command::SayWord)
        );
    }

    #[test]
    fn names_call_commands() {
        assert_eq!(
            Command::from_name("MoveToNextHeadingLevelTwo"),
            Some(Command::MoveToNext(Element::HeadingLevel(2)))
        );
        assert_eq!(Command::from_name("^%{DOWN}"), Some(Command::TableCellDown));
        assert_eq!(
            Command::from_name("Find:engine"),
            Some(Command::Find("engine".to_owned()))
        );
        assert_eq!(Command::from_name("NoSuchScript"), None);
    }
}
