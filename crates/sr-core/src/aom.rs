use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic roles matching accessibility specifications and screen reader paradigms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Document,
    Heading {
        level: u32,
    },
    Paragraph,
    Text,
    Link {
        href: Option<String>,
    },
    Button,
    CheckBox,
    Edit,
    Table,
    TableRow,
    TableCell,
    TabList,
    Tab,
    TabPanel,
    Landmark {
        landmark_type: String,
    },
    List,
    ListItem,
    Dialog,
    AlertDialog,
    ComboBox,
    Tree,
    TreeItem,
    Slider,
    ProgressBar,
    IFrame,
    Math,
    Window,
    /// Structural container with no role of its own (UIA Group without landmark/dialog semantics).
    Group,
    RadioButton,
    Image,
    /// A quotation block (HTML `blockquote`), read with start and end lines.
    BlockQuote,
    /// An item of a menu bar or a popup menu.
    MenuItem,
    /// A button that opens a menu (MSAA `ROLE_SYSTEM_BUTTONMENU`).
    MenuButton,
    Generic,
}

impl Role {
    /// Controls whose default action Enter or Space performs in browse mode.
    pub fn is_interactive_control(&self) -> bool {
        matches!(
            self,
            Role::Button
                | Role::MenuButton
                | Role::CheckBox
                | Role::RadioButton
                | Role::Tab
                | Role::Link { .. }
        )
    }

    /// Determines whether the role represents a block-level element in the virtual buffer.
    pub fn is_block(&self) -> bool {
        matches!(
            self,
            Role::Document
                | Role::Heading { .. }
                | Role::Paragraph
                | Role::Table
                | Role::TableRow
                | Role::ListItem
                | Role::Landmark { .. }
                | Role::TabList
                | Role::TabPanel
                | Role::Dialog
                | Role::AlertDialog
                | Role::Tree
                | Role::TreeItem
                | Role::IFrame
                | Role::Math
                | Role::Window
                | Role::BlockQuote
        )
    }
}

/// Inline font and text formatting styling attributes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TextFormatting {
    #[serde(default)]
    pub font_family: Option<String>,
    #[serde(default)]
    pub font_size_pt: Option<f32>,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    #[serde(default)]
    pub alignment: Option<String>,
}

impl Eq for TextFormatting {}

impl TextFormatting {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_family(mut self, family: impl Into<String>) -> Self {
        self.font_family = Some(family.into());
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.font_size_pt = Some(size);
        self
    }

    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    pub fn with_alignment(mut self, alignment: impl Into<String>) -> Self {
        self.alignment = Some(alignment.into());
        self
    }

    /// Merges child formatting overrides on top of parent styles.
    pub fn merge(&self, child: &TextFormatting) -> TextFormatting {
        TextFormatting {
            font_family: child
                .font_family
                .clone()
                .or_else(|| self.font_family.clone()),
            font_size_pt: child.font_size_pt.or(self.font_size_pt),
            bold: child.bold || self.bold,
            italic: child.italic || self.italic,
            underline: child.underline || self.underline,
            alignment: child.alignment.clone().or_else(|| self.alignment.clone()),
        }
    }
}

/// Node state flags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStates {
    #[serde(default)]
    pub checked: Option<bool>,
    #[serde(default)]
    pub selected: Option<bool>,
    #[serde(default)]
    pub expanded: Option<bool>,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub modal: bool,
}

/// Accessibility Object Model (AOM) node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AomNode {
    pub id: usize,
    pub role: Role,
    pub name: Option<String>,
    pub value: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub formatting: Option<TextFormatting>,
    #[serde(default)]
    pub level: Option<u32>,
    #[serde(default)]
    pub value_min: Option<f64>,
    #[serde(default)]
    pub value_max: Option<f64>,
    #[serde(default)]
    pub value_now: Option<f64>,
    pub children: Vec<AomNode>,
    pub states: NodeStates,
    pub attributes: HashMap<String, String>,
}

impl Eq for AomNode {}

impl AomNode {
    pub fn new(id: usize, role: Role) -> Self {
        Self {
            id,
            role,
            name: None,
            value: None,
            text: None,
            formatting: None,
            level: None,
            value_min: None,
            value_max: None,
            value_now: None,
            children: Vec::new(),
            states: NodeStates::default(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_formatting(mut self, formatting: TextFormatting) -> Self {
        self.formatting = Some(formatting);
        self
    }

    pub fn with_level(mut self, level: u32) -> Self {
        self.level = Some(level);
        self
    }

    pub fn with_range(mut self, min: f64, max: f64, now: f64) -> Self {
        self.value_min = Some(min);
        self.value_max = Some(max);
        self.value_now = Some(now);
        self
    }

    pub fn with_value_now(mut self, now: f64) -> Self {
        self.value_now = Some(now);
        self
    }

    pub fn with_states(mut self, states: NodeStates) -> Self {
        self.states = states;
        self
    }

    pub fn with_children(mut self, children: Vec<AomNode>) -> Self {
        self.children = children;
        self
    }

    pub fn add_child(&mut self, child: AomNode) {
        self.children.push(child);
    }

    pub fn find_by_id(&self, target_id: usize) -> Option<&AomNode> {
        if self.id == target_id {
            return Some(self);
        }
        for child in &self.children {
            if let Some(found) = child.find_by_id(target_id) {
                return Some(found);
            }
        }
        None
    }

    /// Recursively collects text content from this node and its children.
    pub fn collect_text(&self) -> String {
        let mut result = String::new();
        if let Some(ref t) = self.text {
            result.push_str(t);
        }
        for child in &self.children {
            let child_text = child.collect_text();
            if !child_text.is_empty() {
                if !result.is_empty() && !result.ends_with(' ') && !child_text.starts_with(' ') {
                    result.push(' ');
                }
                result.push_str(&child_text);
            }
        }
        result
    }
}
