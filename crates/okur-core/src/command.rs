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

    // Moving, then reading.
    SayNextLine,
    SayPriorLine,
    SayNextWord,
    SayPriorWord,
    SayNextCharacter,
    SayPriorCharacter,
    SayNextParagraph,
    SayPriorParagraph,
    TopOfFile,
    BottomOfFile,

    // Quick navigation.
    MoveToNext(Element),
    MoveToPrior(Element),
    MoveToMainRegion,

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
    SaySelectedText,

    // Finding.
    VirtualFind,
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
            "^{END}" | "BottomOfFile" => BottomOfFile,

            "H" | "h" | "MoveToNextHeading" => MoveToNext(Heading),
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
            "+!{DOWN}" | "SaySelection" | "SaySelectedText" => SaySelectedText,

            "^f" | "Ctrl+F" | "VirtualFind" | "JAWSFind" => VirtualFind,
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
            | "JAWSKey+F5" | "Insert+Ctrl+F" | "JAWSKey+Ctrl+F" => SelectAFormField,
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
/// Ctrl and Alt, with or without Shift.
const CTRL_ALT: Chord = ANY.ctrl(Yes).alt(Yes);
/// Ctrl and the JAWS key: the element lists.
const CTRL_INSERT: Chord = ANY.ctrl(Yes).insert(Yes).alt(No).shift(No);

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
        // Say Selected Text: JAWS key or Alt, with Shift.
        bind(
            DownArrow,
            ANY.insert(Yes).shift(Yes).ctrl(No),
            SaySelectedText,
        ),
        bind(DownArrow, ANY.alt(Yes).shift(Yes).ctrl(No), SaySelectedText),
        // Table rows and columns.
        bind(DownArrow, CTRL_ALT.shift(Yes), MoveToBottomOfColumn),
        bind(UpArrow, CTRL_ALT.shift(Yes), MoveToTopOfColumn),
        bind(RightArrow, CTRL_ALT.shift(Yes), MoveToEndOfRow),
        bind(LeftArrow, CTRL_ALT.shift(Yes), MoveToStartOfRow),
        bind(DownArrow, CTRL_ALT, TableCellDown),
        bind(UpArrow, CTRL_ALT, TableCellUp),
        bind(RightArrow, CTRL_ALT, TableCellRight),
        bind(LeftArrow, CTRL_ALT, TableCellLeft),
        // Combo boxes.
        bind(DownArrow, ANY.alt(Yes).ctrl(No).shift(No), ExpandComboBox),
        bind(UpArrow, ANY.alt(Yes).ctrl(No).shift(No), CollapseComboBox),
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
        bind(Numpad5, JAWS, SayWord),
        bind(Numpad5, ALONE, SayCharacter),
        // Top and bottom, of a table or the page.
        bind(Home, CTRL_ALT, TableFirstCell),
        bind(End, CTRL_ALT, TableLastCell),
        bind(Home, ANY.ctrl(Yes), TopOfFile),
        bind(End, ANY.ctrl(Yes), BottomOfFile),
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
        // Before Shift+2 (the prior heading of level 2), which also matches with the JAWS key.
        bind(
            Digit(2),
            ANY.insert(Yes).shift(Yes).ctrl(No).alt(No),
            SetPunctuationLevel,
        ),
        bind(Digit(2), JAWS, NextTypingEcho),
        // Finding, and the form fields list.
        bind(
            Alpha('F'),
            ANY.ctrl(Yes).insert(No).alt(No).shift(No),
            VirtualFind,
        ),
        bind(
            Alpha('F'),
            ANY.ctrl(Yes).insert(Yes).alt(No).shift(No),
            SelectAFormField,
        ),
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
        bind(Digit(5), CTRL_ALT, SayCurrentCell),
        // Quick navigation: the key moves to the next, Shift with it to the prior.
        bind(Alpha('H'), SHIFT, MoveToPrior(Heading)),
        bind(Alpha('H'), ALONE, MoveToNext(Heading)),
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
        KeyboardLayout::Laptop => lookup(LAPTOP_BINDINGS, event),
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
        let jaws = with(|m| m.insert = true);
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
