use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Semantic roles matching accessibility specifications and screen reader paradigms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    Document,
    Heading { level: u32 },
    Paragraph,
    Text,
    Link { href: Option<String> },
    Button,
    CheckBox,
    Edit,
    Table,
    TableRow,
    TableCell,
    TabList,
    Tab,
    TabPanel,
    Landmark { landmark_type: String },
    List,
    ListItem,
    Generic,
}

impl Role {
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
        )
    }
}

/// Node state flags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeStates {
    pub checked: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    pub focused: bool,
    pub disabled: bool,
    pub hidden: bool,
}

/// Accessibility Object Model (AOM) node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AomNode {
    pub id: usize,
    pub role: Role,
    pub name: Option<String>,
    pub value: Option<String>,
    pub text: Option<String>,
    pub children: Vec<AomNode>,
    pub states: NodeStates,
    pub attributes: HashMap<String, String>,
}

impl AomNode {
    pub fn new(id: usize, role: Role) -> Self {
        Self {
            id,
            role,
            name: None,
            value: None,
            text: None,
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
