//! Navigation state the engine keeps: the list dialogs, the desktop, the taskbar and File
//! Explorer.

use crate::aom::Role;

/// Active element list dialog state.
#[derive(Debug, Clone)]
pub enum ActiveElementList {
    Links {
        items: Vec<crate::buffer::LinkListItem>,
        selected_index: usize,
    },
    Headings {
        items: Vec<crate::buffer::HeadingListItem>,
        selected_index: usize,
    },
    FormFields {
        items: Vec<crate::buffer::FormFieldListItem>,
        selected_index: usize,
    },
}

impl ActiveElementList {
    pub fn move_next(&mut self) -> String {
        match self {
            ActiveElementList::Links {
                items,
                selected_index,
            } => {
                if *selected_index + 1 < items.len() {
                    *selected_index += 1;
                }
                let item = match items.get(*selected_index) {
                    Some(it) => it,
                    None => return String::new(),
                };
                format!(
                    "{}\n{} of {}\nTo move to items use the Arrow keys.",
                    item.text,
                    *selected_index + 1,
                    items.len()
                )
            }
            ActiveElementList::Headings {
                items,
                selected_index,
            } => {
                if *selected_index + 1 < items.len() {
                    *selected_index += 1;
                }
                let item = match items.get(*selected_index) {
                    Some(it) => it,
                    None => return String::new(),
                };
                format!(
                    "{} : {}\n{} of {}\nTo move to items use the Arrow keys.",
                    item.text,
                    item.level,
                    *selected_index + 1,
                    items.len()
                )
            }
            ActiveElementList::FormFields {
                items,
                selected_index,
            } => {
                if *selected_index + 1 < items.len() {
                    *selected_index += 1;
                }
                let item = match items.get(*selected_index) {
                    Some(it) => it,
                    None => return String::new(),
                };
                let role_label = match item.role {
                    Role::Button => "Button",
                    Role::Edit => "Edit",
                    Role::CheckBox => "check box",
                    Role::ComboBox => "combo box",
                    Role::Slider => "slider",
                    _ => "Field",
                };
                format!(
                    "{} {}\n{} of {}\nTo move to items use the Arrow keys.",
                    item.name,
                    role_label,
                    *selected_index + 1,
                    items.len()
                )
            }
        }
    }

    pub fn move_prior(&mut self) -> String {
        match self {
            ActiveElementList::Links {
                items,
                selected_index,
            } => {
                if *selected_index > 0 {
                    *selected_index -= 1;
                }
                let item = match items.get(*selected_index) {
                    Some(it) => it,
                    None => return String::new(),
                };
                format!(
                    "{}\n{} of {}\nTo move to items use the Arrow keys.",
                    item.text,
                    *selected_index + 1,
                    items.len()
                )
            }
            ActiveElementList::Headings {
                items,
                selected_index,
            } => {
                if *selected_index > 0 {
                    *selected_index -= 1;
                }
                let item = match items.get(*selected_index) {
                    Some(it) => it,
                    None => return String::new(),
                };
                format!(
                    "{} : {}\n{} of {}\nTo move to items use the Arrow keys.",
                    item.text,
                    item.level,
                    *selected_index + 1,
                    items.len()
                )
            }
            ActiveElementList::FormFields {
                items,
                selected_index,
            } => {
                if *selected_index > 0 {
                    *selected_index -= 1;
                }
                let item = match items.get(*selected_index) {
                    Some(it) => it,
                    None => return String::new(),
                };
                let role_label = match item.role {
                    Role::Button => "Button",
                    Role::Edit => "Edit",
                    Role::CheckBox => "check box",
                    Role::ComboBox => "combo box",
                    Role::Slider => "slider",
                    _ => "Field",
                };
                format!(
                    "{} {}\n{} of {}\nTo move to items use the Arrow keys.",
                    item.name,
                    role_label,
                    *selected_index + 1,
                    items.len()
                )
            }
        }
    }

    pub fn selected_offset(&self) -> usize {
        match self {
            ActiveElementList::Links {
                items,
                selected_index,
            } => items.get(*selected_index).map(|i| i.offset).unwrap_or(0),
            ActiveElementList::Headings {
                items,
                selected_index,
            } => items.get(*selected_index).map(|i| i.offset).unwrap_or(0),
            ActiveElementList::FormFields {
                items,
                selected_index,
            } => items.get(*selected_index).map(|i| i.offset).unwrap_or(0),
        }
    }
}

/// Operating mode of the screen reader based on foreground application/container context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    VirtualBuffer,
    NativeDesktop,
    NativeShell,
    NativeExplorer,
    NativeControl,
}

/// State tracking for File Explorer list view navigation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeExplorerState {
    pub window_title: Option<String>,
    pub folder_name: Option<String>,
    pub entered: bool,
    pub last_index: Option<usize>,
    pub last_name: Option<String>,
}

/// Represents an item on the native Windows desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopItem {
    pub node_id: usize,
    pub name: String,
    pub index: usize,
    pub total: usize,
}

/// State tracking for native Windows desktop folder view navigation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDesktopState {
    pub container_name: String,
    pub items: Vec<DesktopItem>,
    pub current_index: usize,
    pub entered: bool,
}

impl NativeDesktopState {
    pub fn current_item(&self) -> Option<&DesktopItem> {
        self.items.get(self.current_index)
    }
}
