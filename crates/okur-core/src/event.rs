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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MenuEvent {
    /// The menu bar became active.
    Start,
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
    /// The focused menu item is on a menu bar (a page's ARIA menu bar), not in a menu.
    #[serde(default)]
    pub in_menu_bar: bool,
    /// The named groups and the lists a native control is in, outermost first, up to its
    /// window: Windows Settings' "Brightness & color", "Night light"; "Advanced",
    /// "Troubleshoot common sound problems", then that list. Empty where none, and where the
    /// groups are a control's own content.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<GroupEntry>,
    /// Further columns of a list-view item, after its name.
    pub columns: Vec<String>,
    /// The focused element is inside a browser document (its focus chain has a document).
    pub in_document: bool,
    /// The element's DOM id, when it is in a document and has one.
    pub dom_id: Option<String>,
    /// The page region with ARIA role "application" the focus is in (VS Code's workbench), by
    /// its name: the page takes every key there, as JAWS gives them.
    #[serde(default)]
    pub web_application: Option<String>,
    /// For tree items: the nesting level, 0 for top-level items.
    pub tree_level: Option<usize>,
    /// Number of child items (an expanded tree item's children).
    pub child_count: Option<usize>,
    /// For a text field: its text, caret and selection when it got focus.
    pub text: Option<TextContext>,
    /// When the active window is a dialog box (Win32 `#32770`): its static text.
    pub dialog_text: Option<String>,
    /// When the focus is on a property sheet's page in that dialog box: the page's name,
    /// announced as the dialog opens ("General page").
    pub dialog_page: Option<String>,
    /// When the active window is a dialog box: its buttons, in order (the Virtual Viewer's
    /// last line, "Yes No Cancel") [obs, 311/s14].
    pub dialog_buttons: Vec<String>,
    /// When a dialog box is in front: its text by screen rows, below its title (the Virtual
    /// Viewer's lines for Virtualize Window).
    pub dialog_lines: Vec<String>,
    /// When a dialog box is in front: its controls in reading order, below the dialog itself
    /// (Say Window, Insert+B).
    pub window_items: Vec<WindowItem>,
    /// The window in front is a dialog box.
    pub in_dialog_box: bool,
    pub window_title: Option<String>,
    pub folder_name: Option<String>,
    pub help_text: Option<String>,
    /// The keys that reach the control from anywhere in its window ("Alt+o").
    pub access_key: Option<String>,
    /// Whether the control's value is read-only (UIA), where it reports one: an editable
    /// document is a text field.
    pub read_only: Option<bool>,
    /// The application, as the platform names it: its executable on Windows
    /// ("SearchHost.exe"), its bundle identifier on macOS. For application modules (`apps/`).
    pub app: Option<String>,
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
}

impl GroupEntry {
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            list: false,
            dialog: false,
        }
    }

    pub fn list(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            list: true,
            dialog: false,
        }
    }

    pub fn dialog(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            list: false,
            dialog: true,
        }
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
            in_menu_bar: false,
            groups: Vec::new(),
            columns: Vec::new(),
            in_document: false,
            dom_id: None,
            web_application: None,
            tree_level: None,
            child_count: None,
            text: None,
            dialog_text: None,
            dialog_page: None,
            dialog_buttons: Vec::new(),
            dialog_lines: Vec::new(),
            window_items: Vec::new(),
            in_dialog_box: false,
            window_title: None,
            folder_name: None,
            help_text: None,
            access_key: None,
            read_only: None,
            app: None,
            platform: PlatformIds::default(),
        }
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
    /// Page elements asked for by DOM id, each with its subtree, as they are now; empty when
    /// none was found ([`crate::actions::ApplicationActions::read_page_elements`]).
    PageElements(Vec<crate::aom::AomNode>),
    DocumentLoaded {
        title: String,
        root: crate::aom::AomNode,
    },
}
