//! Navigation state the engine keeps: the list dialogs, the desktop, the taskbar and File
//! Explorer.

use crate::messages::msg;

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
                    "{}\n{}\n{}",
                    item.text,
                    msg::position(*selected_index + 1, items.len()),
                    msg::hint_move_to_items_arrows()
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
                    "{} : {}\n{}\n{}",
                    item.text,
                    item.level,
                    msg::position(*selected_index + 1, items.len()),
                    msg::hint_move_to_items_arrows()
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
                format!(
                    "{}\n{}\n{}",
                    crate::speech::SpeechFormatter::form_field_list_entry(item),
                    msg::position(*selected_index + 1, items.len()),
                    msg::hint_move_to_items_arrows()
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
                    "{}\n{}\n{}",
                    item.text,
                    msg::position(*selected_index + 1, items.len()),
                    msg::hint_move_to_items_arrows()
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
                    "{} : {}\n{}\n{}",
                    item.text,
                    item.level,
                    msg::position(*selected_index + 1, items.len()),
                    msg::hint_move_to_items_arrows()
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
                format!(
                    "{}\n{}\n{}",
                    crate::speech::SpeechFormatter::form_field_list_entry(item),
                    msg::position(*selected_index + 1, items.len()),
                    msg::hint_move_to_items_arrows()
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
