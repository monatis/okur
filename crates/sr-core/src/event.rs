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

/// How much text around the caret a navigation key reads in an edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaretUnit {
    Character,
    Word,
    Line,
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
    /// Window class of the container (`SysListView32` makes a list a list view).
    pub container_class: Option<String>,
    /// Further columns of a list-view item, after its name.
    pub columns: Vec<String>,
    /// The focused element is inside a browser document (its focus chain has a document).
    pub in_document: bool,
    /// The element's DOM id, when it is in a document and has one.
    pub dom_id: Option<String>,
    /// For tree items: the nesting level, 0 for top-level items.
    pub tree_level: Option<usize>,
    /// Number of child items (an expanded tree item's children).
    pub child_count: Option<usize>,
    /// For a text field: its text, caret and selection when it got focus.
    pub text: Option<TextContext>,
    /// When the active window is a dialog box (Win32 `#32770`): its static text.
    pub dialog_text: Option<String>,
    /// When the active window is a dialog box: its buttons, in order (the Virtual Viewer's
    /// last line, "Yes No Cancel") [obs, 311/s14].
    pub dialog_buttons: Vec<String>,
    /// The active window's class, which tells windows with the same title apart (a message
    /// box titled like its main window).
    pub window_kind: Option<String>,
    pub window_class: Option<String>,
    pub window_title: Option<String>,
    pub folder_name: Option<String>,
    pub help_text: Option<String>,
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
            container_class: None,
            columns: Vec::new(),
            in_document: false,
            dom_id: None,
            tree_level: None,
            child_count: None,
            text: None,
            dialog_text: None,
            dialog_buttons: Vec::new(),
            window_kind: None,
            window_class: None,
            window_title: None,
            folder_name: None,
            help_text: None,
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
        self.window_class = Some(class.into());
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
}

impl LiveRegionEvent {
    pub fn new(text: impl Into<String>, politeness: Politeness) -> Self {
        Self {
            text: text.into(),
            politeness,
            container_name: None,
            region_id: None,
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
    LiveRegion(LiveRegionEvent),
    DocumentLoaded {
        title: String,
        root: crate::aom::AomNode,
    },
}
