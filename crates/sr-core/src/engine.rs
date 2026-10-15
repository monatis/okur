use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::cursor::VirtualCursor;
use crate::event::{AccessibilityEvent, FocusEvent, LiveRegionEvent, Politeness};
use crate::input::{Key, KeyAction, KeyEvent};
use crate::speech::{SpeechFormatter, SpeechOutput};

/// The longest gap between two presses of a JAWS command that counts as "twice quickly"
/// (the Windows default double-click time).
const DOUBLE_PRESS_MS: u64 = 500;

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

    pub fn move_next(&mut self) -> Option<&DesktopItem> {
        if self.items.is_empty() {
            return None;
        }
        if self.current_index + 1 < self.items.len() {
            self.current_index += 1;
        }
        self.items.get(self.current_index)
    }

    pub fn move_prior(&mut self) -> Option<&DesktopItem> {
        if self.items.is_empty() {
            return None;
        }
        if self.current_index > 0 {
            self.current_index -= 1;
        }
        self.items.get(self.current_index)
    }

    pub fn match_initial(&mut self, initial: char) -> Option<&DesktopItem> {
        if self.items.is_empty() {
            return None;
        }
        let total = self.items.len();
        let target_char = initial.to_ascii_lowercase();

        // Search starting from next item, with wrap-around
        for offset in 1..=total {
            let idx = (self.current_index + offset) % total;
            if let Some(item) = self.items.get(idx) {
                if item.name.to_lowercase().starts_with(target_char) {
                    self.current_index = idx;
                    return self.items.get(self.current_index);
                }
            }
        }
        None
    }
}

/// Represents an element in the Windows shell bar cycle (Start, Search, Task View, Taskbar).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellElement {
    pub node_id: usize,
    pub name: String,
    pub role: Role,
    pub container: Option<String>,
    pub help: Option<String>,
}

/// State tracking for Tab/Shift+Tab navigation across Windows shell surfaces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellNavState {
    pub elements: Vec<ShellElement>,
    pub current_index: usize,
}

impl ShellNavState {
    pub fn current_element(&self) -> Option<&ShellElement> {
        self.elements.get(self.current_index)
    }
}

/// Primary runtime engine for screen reader execution, coordinating virtual cursor navigation,
/// speech output, and interactive input event handling.
pub struct ScreenReaderEngine {
    pub cursor: VirtualCursor,
    pub speech: Box<dyn SpeechOutput>,
    pub forms_mode: bool,
    pub active_element_list: Option<ActiveElementList>,
    pub app_mode: AppMode,
    pub desktop_state: Option<NativeDesktopState>,
    pub shell_nav_state: Option<ShellNavState>,
    pub explorer_state: Option<NativeExplorerState>,
    pub last_window_title: Option<String>,
    /// The active window's class, with `last_window_title` identifying the window.
    last_window_kind: Option<String>,
    /// Container of the last focused native control; entering a new one announces it.
    native_focus: crate::native_focus::NativeFocus,
    /// The page node that last had focus, to drop repeated focus events on it.
    last_page_focus: Option<usize>,
    /// The last native focus outside menus, re-announced when menus close.
    last_native_focus: Option<FocusEvent>,
    menus: crate::native_focus::Menus,
    /// The page re-read after a change, waiting for the change's events to be handled.
    pending_page: Option<VirtualBuffer>,
    /// Forms mode was just turned on by Enter (not by focus moving into a field).
    forms_mode_by_enter: bool,
    /// The edit key whose effect is spoken when the field next reports its text.
    pending_edit: Option<crate::editing::EditKey>,
    /// The focused text field's last known text, caret and selection.
    last_text: Option<crate::event::TextContext>,
    /// Actions on the real application (focus, default action); None when headless.
    actions: Option<Box<dyn crate::actions::ApplicationActions>>,
    /// The local time and locale formats for Say System Time; None when headless.
    clock: Option<Box<dyn crate::clock::Clock>>,
    /// The last JAWS-key command and when it was pressed, to tell a double press.
    last_press: Option<(String, u64)>,
    /// User settings (JAWS defaults unless configured).
    pub settings: crate::settings::Settings,
    /// Caps Lock's state as toggled through the JAWS key, when the platform does not report it.
    caps_lock_on: bool,
    /// The Virtual Viewer, while it is open (it takes the keys).
    viewer: Option<crate::viewer::VirtualViewer>,
    /// Alt+Tab's task switcher, read as JAWS reads it.
    task_switcher: crate::apps::task_switcher::TaskSwitcher,
    /// Each live region's lines when it last changed, by DOM id.
    last_live_text: std::collections::HashMap<String, Vec<String>>,
}

impl ScreenReaderEngine {
    pub fn new(cursor: VirtualCursor, speech: Box<dyn SpeechOutput>) -> Self {
        Self {
            cursor,
            speech,
            forms_mode: false,
            active_element_list: None,
            app_mode: AppMode::VirtualBuffer,
            desktop_state: None,
            shell_nav_state: None,
            explorer_state: None,
            last_window_title: None,
            last_window_kind: None,
            native_focus: crate::native_focus::NativeFocus::default(),
            last_page_focus: None,
            last_native_focus: None,
            menus: crate::native_focus::Menus::default(),
            pending_edit: None,
            forms_mode_by_enter: false,
            pending_page: None,
            last_text: None,
            actions: None,
            clock: None,
            last_press: None,
            settings: crate::settings::Settings::default(),
            caps_lock_on: false,
            viewer: None,
            task_switcher: Default::default(),
            last_live_text: std::collections::HashMap::new(),
        }
    }

    /// Replaces the active document buffer and resets the cursor.
    pub fn load_buffer(&mut self, buffer: VirtualBuffer) {
        self.cursor = VirtualCursor::new(buffer);
        self.cursor.move_to_start();
        self.forms_mode = false;
        self.active_element_list = None;
        self.app_mode = AppMode::VirtualBuffer;
    }

    /// Inspects an AOM tree and loads native desktop and shell navigation state if present.
    pub fn load_desktop_from_aom(&mut self, root: &AomNode) {
        fn find_desktop_list(node: &AomNode) -> Option<&AomNode> {
            if node.role == Role::List
                && (node.name.as_deref() == Some("FolderView")
                    || node.attributes.get("class").map(|s| s.as_str()) == Some("SysListView32")
                    || node.attributes.get("window_type").map(|s| s.as_str()) == Some("desktop"))
            {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_desktop_list(child) {
                    return Some(found);
                }
            }
            None
        }

        fn find_taskbar_buttons(node: &AomNode, out: &mut Vec<ShellElement>) {
            if node.role == Role::Button {
                out.push(ShellElement {
                    node_id: node.id,
                    name: node.name.clone().unwrap_or_default(),
                    role: node.role.clone(),
                    container: node.attributes.get("container").cloned(),
                    help: node.attributes.get("help").cloned(),
                });
            }
            for child in &node.children {
                find_taskbar_buttons(child, out);
            }
        }

        fn find_taskbar_window(node: &AomNode) -> Option<&AomNode> {
            if node.role == Role::Window
                && (node.name.as_deref() == Some("Taskbar")
                    || node.attributes.get("class").map(|s| s.as_str()) == Some("Shell_TrayWnd"))
            {
                return Some(node);
            }
            for child in &node.children {
                if let Some(found) = find_taskbar_window(child) {
                    return Some(found);
                }
            }
            None
        }

        if let Some(list_node) = find_desktop_list(root) {
            let container_name = list_node
                .name
                .clone()
                .unwrap_or_else(|| "FolderView".to_string());
            let mut items = Vec::new();
            let total_count = list_node.children.len();
            for (idx, child) in list_node.children.iter().enumerate() {
                if child.role == Role::ListItem {
                    let index = child
                        .attributes
                        .get("item_index")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(idx + 1);
                    let total = child
                        .attributes
                        .get("item_count")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(total_count);
                    items.push(DesktopItem {
                        node_id: child.id,
                        name: child.name.clone().unwrap_or_default(),
                        index,
                        total,
                    });
                }
            }

            if !items.is_empty() {
                self.app_mode = AppMode::NativeDesktop;
                self.desktop_state = Some(NativeDesktopState {
                    container_name,
                    items,
                    current_index: 0,
                    entered: false,
                });
            }
        }

        let mut shell_elements = Vec::new();
        if let Some(tb) = find_taskbar_window(root) {
            find_taskbar_buttons(tb, &mut shell_elements);
        }
        if !shell_elements.is_empty() {
            self.shell_nav_state = Some(ShellNavState {
                elements: shell_elements,
                current_index: 0,
            });
        }
    }

    /// Executes a screen reader navigation command string (matching JAWS script names and keystrokes).
    /// Speaks the resulting announcement and returns the emitted speech text.
    pub fn execute_command(&mut self, command: &str) -> Option<String> {
        if command == "SayCurrentFocus" {
            // Native controls; a page's focus is not read this way yet.
            let speech = self
                .last_native_focus
                .as_ref()
                .filter(|_| self.app_mode == AppMode::NativeControl)
                .map(crate::native_focus::say_focus)
                .filter(|s| !s.is_empty())?;
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        if command == "SayWindowTitle" {
            // [obs, 306/s12]: "Title is | SR Dialog Fixture". In a browser it is the browser
            // window's title, not the page's [obs, 209/s1: "... - Google Chrome - <profile>"].
            let title = self.last_window_title.clone().or_else(|| {
                (self.app_mode == AppMode::VirtualBuffer)
                    .then(|| self.cursor.buffer.document_title().map(str::to_owned))
                    .flatten()
            })?;
            let speech = format!("Title is\n{title}");
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        if let Some(speech) = self.clock_command(command) {
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        if command == "VirtualizeWindow" || command == "VirtualizeControl" {
            return self.virtualize(command == "VirtualizeWindow");
        }
        if command == "Insert+F" || command == "SayFontInViewer" {
            return self.say_font(command == "SayFontInViewer");
        }
        // Native desktop navigation mode handling
        if self.app_mode == AppMode::NativeDesktop {
            match command {
                "{DOWN}" | "{RIGHT}" | "DesktopNextItem" => {
                    if let Some(ref mut ds) = self.desktop_state {
                        if let Some(item) = ds.move_next() {
                            let speech = SpeechFormatter::format_desktop_item(
                                &item.name, item.index, item.total,
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                "{UP}" | "{LEFT}" | "DesktopPriorItem" => {
                    if let Some(ref mut ds) = self.desktop_state {
                        if let Some(item) = ds.move_prior() {
                            let speech = SpeechFormatter::format_desktop_item(
                                &item.name, item.index, item.total,
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                "{TAB}" | "ShellNextElement" => {
                    self.app_mode = AppMode::NativeShell;
                    if let Some(ref mut sn) = self.shell_nav_state {
                        sn.current_index = 0;
                        if let Some(el) = sn.current_element() {
                            let speech = SpeechFormatter::format_shell_element(
                                el.container.as_deref(),
                                &el.name,
                                "Button",
                                el.help.as_deref(),
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                "+{TAB}" | "ShellPriorElement" => {
                    self.app_mode = AppMode::NativeShell;
                    if let Some(ref mut sn) = self.shell_nav_state {
                        sn.current_index = sn.elements.len().saturating_sub(1);
                        if let Some(el) = sn.current_element() {
                            let speech = SpeechFormatter::format_shell_element(
                                el.container.as_deref(),
                                &el.name,
                                "Button",
                                el.help.as_deref(),
                            );
                            self.speech.speak(&speech, true);
                            return Some(speech);
                        }
                    }
                    return None;
                }
                _ => {
                    // Check if it's a single alphanumeric key for desktop first-letter navigation
                    if command.len() == 1 {
                        if let Some(c) = command.chars().next() {
                            if c.is_alphanumeric() {
                                if let Some(ref mut ds) = self.desktop_state {
                                    let item = ds.match_initial(c)?;
                                    let speech = SpeechFormatter::format_desktop_item(
                                        &item.name, item.index, item.total,
                                    );
                                    self.speech.speak(&speech, true);
                                    return Some(speech);
                                }
                            }
                        }
                    }
                }
            }
            return None;
        } else if self.app_mode == AppMode::NativeShell {
            match command {
                "{TAB}" | "ShellNextElement" => {
                    if let Some(ref mut sn) = self.shell_nav_state {
                        if sn.current_index + 1 >= sn.elements.len() {
                            // Cycle back to Desktop
                            self.app_mode = AppMode::NativeDesktop;
                            if let Some(ref ds) = self.desktop_state {
                                if let Some(item) = ds.current_item() {
                                    let speech = SpeechFormatter::format_desktop_entry(
                                        &ds.container_name,
                                        &item.name,
                                        item.index,
                                        item.total,
                                    );
                                    self.speech.speak(&speech, true);
                                    return Some(speech);
                                }
                            }
                        } else {
                            sn.current_index += 1;
                            if let Some(el) = sn.current_element() {
                                let speech = SpeechFormatter::format_shell_element(
                                    el.container.as_deref(),
                                    &el.name,
                                    "Button",
                                    el.help.as_deref(),
                                );
                                self.speech.speak(&speech, true);
                                return Some(speech);
                            }
                        }
                    }
                    return None;
                }
                "+{TAB}" | "ShellPriorElement" => {
                    if let Some(ref mut sn) = self.shell_nav_state {
                        if sn.current_index == 0 {
                            // Step back to Desktop
                            self.app_mode = AppMode::NativeDesktop;
                            if let Some(ref ds) = self.desktop_state {
                                if let Some(item) = ds.current_item() {
                                    let speech = SpeechFormatter::format_desktop_entry(
                                        &ds.container_name,
                                        &item.name,
                                        item.index,
                                        item.total,
                                    );
                                    self.speech.speak(&speech, true);
                                    return Some(speech);
                                }
                            }
                        } else {
                            sn.current_index -= 1;
                            if let Some(el) = sn.current_element() {
                                let speech = SpeechFormatter::format_shell_element(
                                    el.container.as_deref(),
                                    &el.name,
                                    "Button",
                                    el.help.as_deref(),
                                );
                                self.speech.speak(&speech, true);
                                return Some(speech);
                            }
                        }
                    }
                    return None;
                }
                _ => {}
            }
            return None;
        }

        // 0. Active element list navigation takes precedence for {DOWN}, {UP}, {ENTER}, {ESC}
        if let Some(ref mut list) = self.active_element_list {
            match command {
                "{DOWN}" => {
                    let speech = list.move_next();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                "{UP}" => {
                    let speech = list.move_prior();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                "{ENTER}" => {
                    let target_offset = list.selected_offset();
                    self.active_element_list = None;
                    let speech = self.cursor.jump_to_element_offset(target_offset);
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                "{ESC}" => {
                    self.active_element_list = None;
                    let speech = "virtual PC Cursor".to_string();
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                _ => {}
            }
        }

        // Parameterized search commands
        if let Some(query) = command
            .strip_prefix("Find:")
            .or_else(|| command.strip_prefix("find:"))
            .or_else(|| command.strip_prefix("VirtualFind:"))
            .or_else(|| command.strip_prefix("JAWSFind:"))
            .or_else(|| command.strip_prefix("^f:"))
        {
            let speech = self.cursor.find_text(query, false);
            if let Some(ref text) = speech {
                self.speech.speak(text, true);
            }
            return speech;
        }

        // Parameterized element list selection
        if let Some(target) = command
            .strip_prefix("SelectHeading:")
            .or_else(|| command.strip_prefix("JumpToHeading:"))
        {
            let list = self.cursor.buffer.get_headings_list();
            if let Some(item) = list
                .iter()
                .find(|h| h.text.eq_ignore_ascii_case(target) || h.text.contains(target))
            {
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
        }

        if let Some(target) = command
            .strip_prefix("SelectLink:")
            .or_else(|| command.strip_prefix("JumpToLink:"))
        {
            let list = self.cursor.buffer.get_links_list();
            if let Some(item) = list
                .iter()
                .find(|l| l.text.eq_ignore_ascii_case(target) || l.text.contains(target))
            {
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
        }

        if let Some(target) = command
            .strip_prefix("SelectFormField:")
            .or_else(|| command.strip_prefix("JumpToFormField:"))
        {
            let list = self.cursor.buffer.get_form_fields_list();
            if let Some(item) = list
                .iter()
                .find(|f| f.name.eq_ignore_ascii_case(target) || f.name.contains(target))
            {
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
        }

        let speech = match command {
            "{DOWN}" => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_next_option()
                } else {
                    self.cursor.move_next_line()
                }
            }
            "{UP}" => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_prior_option()
                } else {
                    self.cursor.move_prior_line()
                }
            }
            "^{RIGHT}" => self.cursor.move_next_word(),
            "^{LEFT}" => self.cursor.move_prior_word(),
            "{RIGHT}" => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_expand_or_next(),
                    Some(Role::Slider) => self.cursor.slider_increase(),
                    _ => self.cursor.move_next_char(),
                }
            }
            "{LEFT}" => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_collapse_or_parent(),
                    Some(Role::Slider) => self.cursor.slider_decrease(),
                    _ => self.cursor.move_prior_char(),
                }
            }
            "^{DOWN}" => self.cursor.move_next_paragraph(),
            "^{UP}" => self.cursor.move_prior_paragraph(),
            "^{HOME}" => Some(self.cursor.move_to_start()),
            "^{END}" => Some(self.cursor.move_to_end()),

            // Headings
            "H" | "h" | "MoveToNextHeading" => self.cursor.move_next_heading(),
            "+H" | "+h" | "MoveToPriorHeading" => self.cursor.move_prior_heading(),
            "1" | "MoveToNextHeadingLevelOne" => self.cursor.move_next_heading_level(1),
            "2" | "MoveToNextHeadingLevelTwo" => self.cursor.move_next_heading_level(2),
            "3" | "MoveToNextHeadingLevelThree" => self.cursor.move_next_heading_level(3),
            "4" | "MoveToNextHeadingLevelFour" => self.cursor.move_next_heading_level(4),
            "5" | "MoveToNextHeadingLevelFive" => self.cursor.move_next_heading_level(5),
            "6" | "MoveToNextHeadingLevelSix" => self.cursor.move_next_heading_level(6),
            "+1" | "MoveToPriorHeadingLevelOne" => self.cursor.move_prior_heading_level(1),
            "+2" | "MoveToPriorHeadingLevelTwo" => self.cursor.move_prior_heading_level(2),
            "+3" | "MoveToPriorHeadingLevelThree" => self.cursor.move_prior_heading_level(3),
            "+4" | "MoveToPriorHeadingLevelFour" => self.cursor.move_prior_heading_level(4),
            "+5" | "MoveToPriorHeadingLevelFive" => self.cursor.move_prior_heading_level(5),
            "+6" | "MoveToPriorHeadingLevelSix" => self.cursor.move_prior_heading_level(6),

            // Links
            "U" | "u" | "MoveToNextLink" | "MoveToNextUnvisitedLink" => {
                self.cursor.move_next_link()
            }
            "+U" | "+u" | "MoveToPriorLink" | "MoveToPriorUnvisitedLink" => {
                self.cursor.move_prior_link()
            }

            // Tables
            "T" | "t" | "MoveToNextTable" => self.cursor.move_next_table(),
            "+T" | "+t" | "MoveToPriorTable" => self.cursor.move_prior_table(),
            "^%{RIGHT}" | "NextCell" | "TableCellRight" => self.cursor.move_table_cell_right(),
            "^%{LEFT}" | "PriorCell" | "TableCellLeft" => self.cursor.move_table_cell_left(),
            "^%{DOWN}" | "NextRow" | "TableCellDown" => self.cursor.move_table_cell_down(),
            "^%{UP}" | "PriorRow" | "TableCellUp" => self.cursor.move_table_cell_up(),
            "^%{HOME}" | "FirstCell" | "TableFirstCell" => self.cursor.move_table_first_cell(),
            "^%{END}" | "LastCell" | "TableLastCell" => self.cursor.move_table_last_cell(),
            "^%+LEFT" | "^%+{LEFT}" | "MoveToStartOfRow" | "RowStart" => {
                self.cursor.move_table_row_start()
            }
            "^%+RIGHT" | "^%+{RIGHT}" | "MoveToEndOfRow" | "RowEnd" => {
                self.cursor.move_table_row_end()
            }
            "^%+UP" | "^%+{UP}" | "MoveToTopOfColumn" | "ColumnTop" => {
                self.cursor.move_table_col_top()
            }
            "^%+DOWN" | "^%+{DOWN}" | "MoveToBottomOfColumn" | "ColumnBottom" => {
                self.cursor.move_table_col_bottom()
            }
            "^%{NUMPAD5}" | "^%5" | "SayCurrentCell" | "CurrentCell" => {
                self.cursor.read_current_cell()
            }
            "^%+H" | "^%+h" | "SayColumnHeader" => self.cursor.read_current_col_header(),
            "^%+R" | "^%+r" | "SayRowHeader" => self.cursor.read_current_row_header(),

            // Bindings from JAWS's Quick Navigation Keys table
            // (docs/references/jaws-quick-navigation-keys.md).
            "O" | "o" | "MoveToNextArticle" => self.cursor.move_next_article(),
            "+O" | "+o" | "MoveToPriorArticle" => self.cursor.move_prior_article(),
            "Q" | "q" | "+Q" | "+q" | "MoveToMainRegion" => self.cursor.move_to_main_region(),
            ";" | "MoveToNextOnMouseOverElement" => self.cursor.move_next_on_mouse_over(),
            "+;" | "MoveToPriorOnMouseOverElement" => self.cursor.move_prior_on_mouse_over(),
            "'" | "MoveToNextTab" => self.cursor.move_next_tab(),
            "+'" | "MoveToPriorTab" => self.cursor.move_prior_tab(),
            // Place markers are not implemented; K must not fall through to another command.
            "K" | "k" | "+K" | "+k" | "MoveToNextPlaceMarker" | "MoveToPriorPlaceMarker" => None,

            // Graphics and radio buttons (spec §16, §17)
            "G" | "g" | "MoveToNextGraphic" => self.cursor.move_next_graphic(),
            "+G" | "+g" | "MoveToPriorGraphic" => self.cursor.move_prior_graphic(),
            "A" | "a" | "MoveToNextRadioButton" => self.cursor.move_next_radio_button(),
            "+A" | "+a" | "MoveToPriorRadioButton" => self.cursor.move_prior_radio_button(),

            // MathML
            "MoveToNextMath" => self.cursor.move_next_math(),
            "MoveToPriorMath" => self.cursor.move_prior_math(),
            "EnterMathViewer" | "ShowMathEditor" => self.cursor.enter_math_viewer(),

            // Form controls
            "F" | "f" | "FocusToNextField" => self.cursor.move_next_form_control(),
            "+F" | "+f" | "FocusToPriorField" => self.cursor.move_prior_form_control(),
            "B" | "b" | "MoveToNextButton" => self.cursor.move_next_button(),
            "+B" | "+b" | "MoveToPriorButton" => self.cursor.move_prior_button(),
            "E" | "e" | "MoveToNextEdit" => self.cursor.move_next_edit(),
            "+E" | "+e" | "MoveToPriorEdit" => self.cursor.move_prior_edit(),
            "X" | "x" | "MoveToNextCheckbox" => self.cursor.move_next_checkbox(),
            "+X" | "+x" | "MoveToPriorCheckbox" => self.cursor.move_prior_checkbox(),
            "C" | "c" | "MoveToNextCombo" | "MoveToNextComboBox" => {
                self.cursor.move_next_combobox()
            }
            "+C" | "+c" | "MoveToPriorCombo" | "MoveToPriorComboBox" => {
                self.cursor.move_prior_combobox()
            }
            "!{DOWN}" | "ExpandComboBox" => self.cursor.expand_combobox(),
            "!{UP}" | "CollapseComboBox" => self.cursor.collapse_combobox(),

            // Lists & Landmarks
            "L" | "l" | "MoveToNextList" => self.cursor.move_next_list(),
            "+L" | "+l" | "MoveToPriorList" => self.cursor.move_prior_list(),
            "I" | "i" | "MoveToNextListItem" => self.cursor.move_next_list_item(),
            "+I" | "+i" | "MoveToPriorListItem" => self.cursor.move_prior_list_item(),
            "R" | "r" | "MoveToNextLandmark" | "MoveToNextRegion" => {
                self.cursor.move_next_landmark()
            }
            "+R" | "+r" | "MoveToPriorLandmark" | "MoveToPriorRegion" => {
                self.cursor.move_prior_landmark()
            }

            // Frames
            "M" | "m" | "MoveToNextFrame" => self.cursor.move_next_frame(),
            "+M" | "+m" | "MoveToPriorFrame" => self.cursor.move_prior_frame(),

            // Interactive navigation & actions
            "{TAB}" => self.cursor.move_next_tab_stop(),
            "+{TAB}" | "+TAB" => self.cursor.move_prior_tab_stop(),
            "{SPACE}" => {
                if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
                    let is_link = matches!(node.role, Role::Link { .. });
                    if node.role.is_interactive_control() && !is_link && actions.activate(&node) {
                        // The page's own state change and focus events report the result.
                        return Some(String::new());
                    }
                }
                None
            }
            "{ENTER}" | "EnterFormsMode" | "ActivateLink" => {
                let node = self.node_under_cursor();
                let same_page_link = node
                    .as_ref()
                    .is_some_and(|n| n.attributes.get("same_page").is_some_and(|v| v == "true"));
                if let (Some(actions), Some(node)) = (&self.actions, node) {
                    let handled = match node.role {
                        // Forms mode on (JAWS plays a sound) and real focus in the field; the
                        // browser's focus event then speaks it.
                        Role::Edit | Role::ComboBox => {
                            let focused = actions.focus(&node);
                            if focused {
                                self.forms_mode = true;
                                self.cursor.forms_mode_active = true;
                                self.forms_mode_by_enter = true;
                            }
                            focused
                        }
                        Role::Link { .. } => !same_page_link && actions.activate(&node),
                        Role::Button
                        | Role::MenuButton
                        | Role::CheckBox
                        | Role::RadioButton
                        | Role::Tab => actions.activate(&node),
                        _ => false,
                    };
                    if handled {
                        return Some(String::new());
                    }
                }
                if let Some(link_speech) = self.cursor.activate_current_link() {
                    Some(link_speech)
                } else if let Some(math_speech) = self.cursor.enter_math_viewer() {
                    Some(math_speech)
                } else {
                    let res = self.cursor.enter_forms_mode();
                    if res.is_some() {
                        self.forms_mode = true;
                    }
                    res
                }
            }
            "{ESC}" | "ExitFormsMode" => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.collapse_combobox()
                } else if self.forms_mode {

                    let res = self.cursor.exit_forms_mode();
                    self.forms_mode = false;
                    res
                } else if self.cursor.is_in_modal() {
                    self.cursor.exit_modal()
                } else {
                    None
                }
            }

            // Text selection commands
            "+{RIGHT}" | "SelectNextCharacter" => self.cursor.select_next_char(),
            "+{LEFT}" | "SelectPriorCharacter" => self.cursor.select_prior_char(),
            "+{DOWN}" | "SelectNextLine" => self.cursor.select_next_line(),
            "+{UP}" | "SelectPriorLine" => self.cursor.select_prior_line(),
            "+!{DOWN}" | "SaySelection" | "SaySelectedText" => Some(self.cursor.say_selection()),

            // Font formatting query
            "Insert+F" | "WhatFont" | "SayFont" => Some(self.cursor.say_font_formatting()),

            // Modal dialog commands
            "EnterModal" => {
                let target_dlg = if let Some(node_id) = self.cursor.current_node_id {
                    self.cursor.buffer.dialog_for_node(node_id)
                } else {
                    None
                }
                .or_else(|| self.cursor.buffer.dialogs.first());

                if let Some(dlg) = target_dlg {
                    self.cursor.enter_modal(dlg.node_id)
                } else {
                    None
                }
            }
            "ExitModal" => self.cursor.exit_modal(),

            // JAWS Say Scripts
            "SayAll" => Some(self.cursor.say_all()),
            "SayLine" => Some(self.cursor.current_line()),
            "SayWord" => Some(self.cursor.current_word()),
            "SayCharacter" => Some(self.cursor.current_char()),

            // In-buffer find commands
            "^f" | "Ctrl+F" | "VirtualFind" | "JAWSFind" => {
                Some("JAWS Find  \u{001d} dialog \u{001e} \nFind what:  \u{001d} edit combo \u{001e} \nTo set the value use the Arrow keys or type the value.".to_string())
            }
            "{F3}" | "F3" | "JAWSFindNext" | "FindNext" => self.cursor.find_next(),
            "+{F3}" | "+F3" | "Shift+F3" | "JAWSFindPrior" | "FindPrior" => self.cursor.find_prior(),

            // Element list dialogs
            "SelectAHeading" | "HeadingList" | "HeadingsList" | "Insert+F6" | "JAWSKey+F6" => {
                let items = self.cursor.buffer.get_headings_list();
                if items.is_empty() {
                    Some("There are no headings on this page.".to_string())
                } else {
                    let speech = SpeechFormatter::format_headings_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::Headings {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            "SelectALink" | "LinkList" | "LinksList" | "Insert+F7" | "JAWSKey+F7" => {
                let items = self.cursor.buffer.get_links_list();
                if items.is_empty() {
                    Some("There are no links on this page.".to_string())
                } else {
                    let speech = SpeechFormatter::format_links_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::Links {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }
            "SelectAFormField"
            | "FormFieldList"
            | "FormFieldsList"
            | "Insert+F5"
            | "JAWSKey+F5"
            | "Insert+Ctrl+F"
            | "JAWSKey+Ctrl+F" => {
                let items = self.cursor.buffer.get_form_fields_list();
                if items.is_empty() {
                    Some("There are no form fields on this page.".to_string())
                } else {
                    let speech = SpeechFormatter::format_form_fields_list_dialog(&items);
                    self.active_element_list = Some(ActiveElementList::FormFields {
                        items,
                        selected_index: 0,
                    });
                    Some(speech)
                }
            }

            _ => None,
        };

        if let Some(ref text) = speech {
            // Left/Right read a character, which the synthesizer names ("space").
            let character = matches!(command, "{RIGHT}" | "{LEFT}") && text.chars().count() == 1;
            if character {
                self.speech.speak_character(text, true);
            } else if !text.is_empty() {
                self.speech.speak(text, true);
            }
        }

        speech
    }

    /// Handles a live keyboard event from the low-level hook.
    /// Intercepts navigation keys, manages forms mode transitions, silences ongoing speech,
    /// and returns whether the key was consumed or should pass through to the OS.
    pub fn handle_key_event(&mut self, event: &KeyEvent) -> KeyAction {
        if !event.is_down {
            return KeyAction::PassThrough;
        }
        // Which held keys act as the JAWS key depends on the keyboard settings.
        let resolved = self.settings.keyboard.resolve(event);
        let event = &resolved;
        self.apply_pending_page();

        // 1. Any key press interrupts speech immediately (NVDA & JAWS standard behavior)
        self.speech.stop();

        // 2. Standalone Control tap is speech silence key
        if event.key == Key::Control && event.modifiers.is_empty() {
            return KeyAction::PassThrough;
        }

        // The JAWS key (Insert, or CapsLock in the laptop layout) and every combination with
        // it are JAWS commands: they never reach the application, bound or not, in any mode.
        if self.settings.keyboard.is_jaws_key(event.key) {
            if event.key == Key::CapsLock && !event.is_repeat {
                return self.caps_lock_press(event);
            }
            return KeyAction::Consume;
        }
        // The Virtual Viewer takes the keys while it is open.
        if let Some(action) = self.viewer.as_mut().map(|v| v.key(event)) {
            use crate::viewer::ViewerAction;
            match action {
                ViewerAction::Speak(speech) => {
                    if let Some(speech) = speech {
                        self.speech.speak(&speech, true);
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Character(text) => {
                    if !text.is_empty() {
                        self.speech.speak_character(&text, true);
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Copy(text) => {
                    if let Some(actions) = &self.actions {
                        actions.copy_text(&text);
                    }
                    // [obs, 311/s11]
                    self.speech.speak("Copied selection to clipboard", true);
                    return KeyAction::Consume;
                }
                ViewerAction::Close => {
                    self.viewer = None;
                    self.speak_focus_again();
                    return KeyAction::Consume;
                }
                ViewerAction::Ignore => {}
            }
        }
        if event.modifiers.insert {
            self.jaws_key_command(event);
            return KeyAction::Consume;
        }

        // 3. Handle Forms Mode
        if self.forms_mode {
            if event.key == Key::Escape {
                self.forms_mode = false;
                let _ = self.cursor.exit_forms_mode();
                // [obs, 205/s11]
                self.speech.speak("virtual PC Cursor", true);
                return KeyAction::Consume;
            }
            // In forms mode, pass all typing and navigation through to the active control;
            // caret movement keys then read the text they moved over, typed keys are echoed.
            self.edit_key_feedback(event);
            return KeyAction::PassThrough;
        }

        // 4. Handle Escape when inside an active element list, expanded combobox or modal dialog
        if event.key == Key::Escape && event.modifiers.is_empty() {
            if self.active_element_list.is_some() {
                self.active_element_list = None;
                self.speech.speak("virtual PC Cursor", true);
                return KeyAction::Consume;
            }
            if self.cursor.is_combobox_expanded() {
                if let Some(msg) = self.cursor.collapse_combobox() {
                    self.speech.speak(&msg, true);
                }
                return KeyAction::Consume;
            }
            if self.cursor.is_in_modal() {
                if let Some(msg) = self.cursor.exit_modal() {
                    self.speech.speak(&msg, true);
                }
                return KeyAction::Consume;
            }
        }

        // 5. In non-VirtualBuffer modes (Desktop, Shell, native controls),
        // navigation and typing keys MUST pass through to the OS so Windows
        // handles arrow navigation, Tab cycling, and initial-letter jumps natively!
        if self.app_mode != AppMode::VirtualBuffer {
            if self.editing_text() {
                self.edit_key_feedback(event);
            }
            return KeyAction::PassThrough;
        }

        // 6. Tab and Shift+Tab belong to the page: the browser moves focus, and the focus
        //    event moves the virtual cursor and speaks [obs, 204].
        if event.key == Key::Tab && !event.modifiers.ctrl && !event.modifiers.alt {
            return KeyAction::PassThrough;
        }

        // 7. In Virtual PC Cursor mode, check if keystroke translates to a command
        if let Some(cmd) = event.to_command_string() {
            let res = self.execute_command(&cmd);
            if res.is_some() {
                return KeyAction::Consume;
            }
            // For navigation keys, consume even if boundary reached so host window doesn't scroll
            if matches!(
                event.key,
                Key::DownArrow
                    | Key::UpArrow
                    | Key::LeftArrow
                    | Key::RightArrow
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown
                    | Key::Tab
                    | Key::Alpha(_)
                    | Key::Digit(_)
            ) {
                return KeyAction::Consume;
            }
        }

        KeyAction::PassThrough
    }

    /// Handles client-side SPA routing transitions: announces new page title and retargets cursor.
    pub fn handle_route_transition(&mut self, new_title: &str, target_id: Option<&str>) -> String {
        let title_announcement = SpeechFormatter::format_page_title_change(new_title);
        let mut announcements = vec![title_announcement];

        if let Some(target) = target_id {
            let resolved_offset = if let Ok(num_id) = target.parse::<usize>() {
                self.cursor.move_to_node_id(num_id);
                Some(self.cursor.offset)
            } else if let Some(&offset) = self.cursor.buffer.target_anchors.get(target) {
                self.cursor.offset = offset;
                self.cursor.current_node_id =
                    self.cursor.buffer.node_at_offset(offset).map(|n| n.id);
                Some(offset)
            } else {
                None
            };

            if resolved_offset.is_some() {
                let line = self.cursor.current_line();
                if !line.is_empty() {
                    announcements.push(line);
                }
            }
        }

        let full_text = announcements.join("\n");
        self.speech.speak(&full_text, true);
        full_text
    }

    /// Handles an incoming accessibility event from the OS event listener.
    pub fn handle_accessibility_event(&mut self, event: &AccessibilityEvent) -> Option<String> {
        if !matches!(
            event,
            AccessibilityEvent::LiveRegion(_) | AccessibilityEvent::DocumentLoaded { .. }
        ) {
            self.apply_pending_page();
        }
        match event {
            AccessibilityEvent::Focus(focus) => self.handle_focus_event(focus),
            AccessibilityEvent::Menu(menu) => self.handle_menu_event(*menu),
            AccessibilityEvent::Change { kind, control } => self.handle_change(*kind, control),
            AccessibilityEvent::TextContext(context) => self.handle_text_context(context),
            AccessibilityEvent::LiveRegion(live) => self.handle_live_region_event(live),
            AccessibilityEvent::DocumentLoaded { title, root } => {
                let buffer = VirtualBuffer::from_aom(root);
                // The page already in the buffer, captured again (the first focus in it
                // after start-up): refresh it without announcing it or leaving forms mode.
                let same_page = self.app_mode == AppMode::VirtualBuffer
                    && self.cursor.buffer.document_title() == Some(title.as_str());
                if same_page {
                    // Applied once the events of this change are handled: several live regions
                    // can change at once, and each is compared with the page before the change
                    // [obs, 206/s2].
                    self.pending_page = Some(buffer);
                    return None;
                }
                self.pending_page = None;
                self.last_page_focus = None;
                self.last_live_text.clear();
                self.load_buffer(buffer);
                self.app_mode = AppMode::VirtualBuffer;
                self.last_window_title = Some(title.clone());
                let welcome = format!("{}  \u{001d} Virtual PC Cursor \u{001e}", title);
                self.speech.speak(&welcome, true);
                Some(welcome)
            }
        }
    }

    /// Records the focused control's top-level window as the active one, for Insert+T and
    /// for announcing the next window change.
    fn note_window(title: &mut Option<String>, kind: &mut Option<String>, focus: &FocusEvent) {
        if let Some(window) = &focus.window_title {
            *title = Some(window.clone());
            *kind = focus.window_kind.clone();
        }
    }

    /// Handles an OS-level focus changed event.
    /// Synchronizes cursor position, activates/deactivates forms mode,
    /// and emits canonical JAWS speech announcements.
    pub fn handle_focus_event(&mut self, focus: &FocusEvent) -> Option<String> {
        if focus.role != Role::MenuItem {
            self.last_text = focus.text.clone();
            self.pending_edit = None;
        }
        if focus.role == Role::MenuItem {
            let speech = self
                .menus
                .focus(focus.name.as_deref().unwrap_or_default())?;
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        // Ignore spontaneous background tile updates from Windows 11 Widgets applet
        if focus
            .name
            .as_deref()
            .is_some_and(|n| n.starts_with("Widgets"))
        {
            return None;
        }
        // Alt+Tab: the task switcher's items. Its window is remembered, so going back to an
        // application announces that application's window.
        if let Some(speech) = self.task_switcher.focus(focus) {
            Self::note_window(
                &mut self.last_window_title,
                &mut self.last_window_kind,
                focus,
            );
            if speech.is_empty() {
                return None;
            }
            self.speech.speak(&speech, true);
            return Some(speech);
        }

        let is_explorer_event = focus.window_class.as_deref() == Some("CabinetWClass")
            || focus.window_class.as_deref() == Some("ShellTabWindowClass")
            || focus.window_class.as_deref() == Some("UIItemsView")
            || focus.container_name.as_deref() == Some("Items View")
            || focus.container_name.as_deref() == Some("Items view")
            || focus.container_name.as_deref() == Some("Shell Folder View")
            || focus
                .window_title
                .as_deref()
                .is_some_and(|t| t.contains("File Explorer"));

        // 0. Check if focus landed on native desktop
        let is_desktop_event = !is_explorer_event
            && (focus.container_name.as_deref() == Some("FolderView")
                || focus.window_class.as_deref() == Some("Progman")
                || focus.window_class.as_deref() == Some("WorkerW")
                || (focus.window_class.as_deref() == Some("SysListView32")
                    && focus.container_name.as_deref() != Some("Items View")
                    && focus.container_name.as_deref() != Some("Items view"))
                // An item in the desktop's window, whatever window the event came from (on
                // Windows 11 the first of the desktop's focus events has none, seen live).
                || (focus.role == Role::ListItem
                    && focus
                        .window_kind
                        .as_deref()
                        .is_some_and(|k| matches!(k, "Progman" | "WorkerW")))
                // An item still on the desktop; not one in another window (the task
                // switcher's, after Win+M then Alt+Tab).
                || (focus.role == Role::ListItem
                    && self.app_mode == AppMode::NativeDesktop
                    && focus
                        .window_kind
                        .as_deref()
                        .is_none_or(|k| matches!(k, "Progman" | "WorkerW"))));

        if is_desktop_event {
            let prev_mode = self.app_mode;
            let ds = self
                .desktop_state
                .get_or_insert_with(|| NativeDesktopState {
                    container_name: "FolderView".to_string(),
                    items: Vec::new(),
                    current_index: 0,
                    entered: false,
                });

            let is_first_entry = prev_mode != AppMode::NativeDesktop || !ds.entered;
            ds.entered = true;
            self.app_mode = AppMode::NativeDesktop;
            // Insert+T on the desktop speaks the virtual desktop's name, not the window
            // focused before it (user report).
            Self::note_window(
                &mut self.last_window_title,
                &mut self.last_window_kind,
                focus,
            );

            if let Some(ref mut es) = self.explorer_state {
                es.entered = false;
            }

            let name = focus.name.as_deref().unwrap_or("Desktop");
            let index = focus
                .item_index
                .unwrap_or_else(|| ds.current_item().map(|it| it.index).unwrap_or(1));
            let total = focus
                .item_count
                .unwrap_or_else(|| ds.current_item().map(|it| it.total).unwrap_or(1));

            // Sync ds.items and check for deduplication
            if ds.items.is_empty() {
                ds.items.push(DesktopItem {
                    node_id: index,
                    name: name.to_string(),
                    index,
                    total,
                });
                ds.current_index = 0;
            } else if let Some(pos) = ds
                .items
                .iter()
                .position(|it| it.name.eq_ignore_ascii_case(name) || it.index == index)
            {
                if !is_first_entry && ds.current_index == pos {
                    return None;
                }
                ds.current_index = pos;
            } else {
                ds.items.push(DesktopItem {
                    node_id: index,
                    name: name.to_string(),
                    index,
                    total,
                });
                ds.current_index = ds.items.len().saturating_sub(1);
            }

            let speech = if is_first_entry {
                SpeechFormatter::format_desktop_entry("FolderView", name, index, total)
            } else {
                SpeechFormatter::format_desktop_item(name, index, total)
            };
            self.speech.speak(&speech, true);
            return Some(speech);
        } else if is_explorer_event && (focus.role == Role::ListItem || focus.role == Role::List) {
            let prev_mode = self.app_mode;
            let es = self
                .explorer_state
                .get_or_insert_with(NativeExplorerState::default);

            let is_first_entry = prev_mode != AppMode::NativeExplorer || !es.entered;
            es.entered = true;
            self.app_mode = AppMode::NativeExplorer;
            Self::note_window(
                &mut self.last_window_title,
                &mut self.last_window_kind,
                focus,
            );

            if let Some(ref mut ds) = self.desktop_state {
                ds.entered = false;
            }

            if let Some(ref title) = focus.window_title {
                es.window_title = Some(title.clone());
            }
            if let Some(ref folder) = focus.folder_name {
                es.folder_name = Some(folder.clone());
            }

            let name = focus.name.as_deref().unwrap_or("Item");
            let index = focus.item_index.unwrap_or(1);
            let total = focus.item_count.unwrap_or(1);
            let selected = focus.states.selected;

            if !is_first_entry
                && es.last_index == Some(index)
                && es.last_name.as_deref() == Some(name)
            {
                return None;
            }
            es.last_index = Some(index);
            es.last_name = Some(name.to_string());

            let speech = if is_first_entry {
                let window_title = focus
                    .window_title
                    .as_deref()
                    .or(es.window_title.as_deref())
                    .or(Some("File Explorer"));
                let folder_name = focus.folder_name.as_deref().or(es.folder_name.as_deref());
                SpeechFormatter::format_explorer_entry(
                    window_title,
                    folder_name,
                    selected,
                    name,
                    index,
                    total,
                )
            } else {
                SpeechFormatter::format_explorer_item(name, index, total)
            };
            self.speech.speak(&speech, true);
            return Some(speech);
        } else if focus.window_class.as_deref() == Some("Shell_TrayWnd")
            || focus.container_name.as_deref() == Some("Taskbar")
        {
            if let Some(ref mut es) = self.explorer_state {
                es.entered = false;
            }
            let prev_mode = self.app_mode;
            self.app_mode = AppMode::NativeShell;
            let name = focus.name.as_deref().unwrap_or("Start Button");
            let container = if prev_mode != AppMode::NativeShell {
                focus.container_name.as_deref().or(Some("Taskbar"))
            } else {
                None
            };
            let speech = SpeechFormatter::format_shell_element(
                container,
                name,
                "Button",
                focus.help_text.as_deref(),
            );
            self.speech.speak(&speech, true);
            return Some(speech);
        }

        if let Some(ref mut ds) = self.desktop_state {
            ds.entered = false;
        }
        if let Some(ref mut es) = self.explorer_state {
            es.entered = false;
        }

        let is_in_document = focus.in_document
            || focus
                .node_id
                .is_some_and(|id| self.cursor.buffer.node_map.contains_key(&id));

        if is_in_document {
            self.app_mode = AppMode::VirtualBuffer;
            // The page is in the same top-level window as anything focused next in the
            // browser (toolbar, address bar), so that focus does not announce a new window.
            if let Some(title) = &focus.window_title {
                self.last_window_title = Some(title.clone());
                self.last_window_kind = focus.window_kind.clone();
            }

            let target_id = self.page_node_for(focus);

            // If focus target is a dialog, enter modal confinement and announce Dialog <Title>
            if matches!(focus.role, Role::Dialog | Role::AlertDialog) {
                let title = focus.name.as_deref().unwrap_or("");
                let msg = if let Some(id) = target_id {
                    self.cursor.move_to_node_id(id);
                    self.cursor
                        .enter_modal(id)
                        .unwrap_or_else(|| SpeechFormatter::format_dialog_entry(title))
                } else {
                    SpeechFormatter::format_dialog_entry(title)
                };
                if !msg.is_empty() {
                    self.speech.speak(&msg, true);
                }
                return Some(msg);
            }

            // If target is inside a modal dialog, ensure modal confinement is active
            if let Some(id) = target_id {
                if let Some(dlg) = self.cursor.buffer.dialog_for_node(id) {
                    if dlg.is_modal && self.cursor.active_modal_root != Some(dlg.node_id) {
                        let _ = self.cursor.enter_modal(dlg.node_id);
                    }
                } else if self.cursor.is_in_modal() {
                    let _ = self.cursor.exit_modal();
                }
            }

            // Forms mode follows focus into edits silently: JAWS 2025 signals it with a
            // sound by default, and its speech history has no forms-mode text [obs, 204].
            let target_role = target_id
                .and_then(|id| self.cursor.buffer.node_map.get(&id))
                .map(|n| n.role.clone());
            if target_role == Some(Role::Document) {
                // The page itself has focus: the next control focus enters the page.
                self.last_page_focus = None;
                return None;
            }
            // Forms mode follows focus into edits; a combo box keeps the forms mode Enter
            // turned on [obs, 205/s8].
            let is_edit = target_role == Some(Role::Edit) || focus.role == Role::Edit;
            let in_forms_mode = is_edit || (target_role == Some(Role::ComboBox) && self.forms_mode);
            self.forms_mode = in_forms_mode;
            self.cursor.forms_mode_active = in_forms_mode;
            // Chrome can report focus on the same element again (while typing in it); JAWS
            // does not repeat the field.
            if target_id.is_some() && target_id == self.last_page_focus {
                return None;
            }
            // The first control focused after the page itself is preceded by the page title
            // [obs, 205/s2; 204/s1 second variant].
            let entering_page = self.last_page_focus.is_none();
            // Group entry is judged from where focus was, not the virtual cursor [obs, 205/s6].
            let previous_focus_offset = self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.lines.iter().find(|l| l.node_id == id))
                .map(|l| l.start_offset);
            self.last_page_focus = target_id;
            // The focus event carries the live state; the buffer's copy may be older (Space
            // just toggled the control) [obs, 205/s2, s6].
            if let Some(id) = target_id {
                let stale = self.cursor.buffer.node_map.get(&id).is_some_and(|n| {
                    focus.states.checked.is_some() && n.states.checked != focus.states.checked
                });
                if stale {
                    if let Some(rebuilt) = self
                        .cursor
                        .buffer
                        .with_node_changed(id, |n| n.states.checked = focus.states.checked)
                    {
                        self.cursor.buffer = rebuilt;
                    }
                }
            }
            if let Some(offset) = previous_focus_offset {
                self.cursor.offset = offset;
            }
            let mut speech = target_id
                .and_then(|id| self.cursor.focus_node(id))
                .or_else(|| focus.name.clone());
            if target_role == Some(Role::ComboBox) && in_forms_mode {
                // In forms mode a combo box gets its usage hint [obs, 205/s8].
                speech =
                    speech.map(|s| format!("{s}\nTo change the selection use the Arrow keys."));
            }
            // Forms mode turned on by Enter adds the regions the field is in and the edit's
            // hint [obs, 19/s2]; Tab's automatic forms mode does not (204/s1).
            let by_enter = std::mem::take(&mut self.forms_mode_by_enter) && is_edit;
            if by_enter {
                speech = speech.map(|s| format!("{s}\nType in text."));
            }
            if entering_page {
                let regions = if by_enter {
                    target_id
                        .map(|id| self.region_markers(id))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                if let Some(title) = self.cursor.buffer.document_title() {
                    speech = speech.map(|s| format!("{title}\n{regions}{s}"));
                }
            }
            if let Some(ref text) = speech {
                if !text.is_empty() {
                    self.speech.speak(text, true);
                }
            }

            return speech;
        }

        // Native control / application window handling
        self.last_page_focus = None;
        self.app_mode = AppMode::NativeControl;
        if self.forms_mode {
            self.forms_mode = false;
            self.cursor.forms_mode_active = false;
        }

        let window_changed = if let Some(ref wt) = focus.window_title {
            let changed = self.last_window_title.as_ref() != Some(wt)
                || (focus.window_kind.is_some() && self.last_window_kind != focus.window_kind);
            if changed {
                self.last_window_title = Some(wt.clone());
                self.last_window_kind = focus.window_kind.clone();
            }
            changed
        } else {
            false
        };

        if window_changed {
            self.native_focus.reset();
        }
        let control_speech = self.native_focus.focus(focus)?;
        self.last_native_focus = Some(focus.clone());

        let speech = if window_changed {
            // A new active window: its title, then the control. A dialog box adds its role
            // and its text [obs, 306/s1, s3, s10].
            // The desktop is announced as "desktop"; its title (Insert+T) is the virtual
            // desktop's name (JAWS on Windows 11: "desktop | FolderView List view").
            let win = if focus.window_kind.as_deref() == Some("Progman") {
                "desktop"
            } else {
                focus.window_title.as_deref().unwrap_or("")
            };
            let heading = match focus.dialog_text.as_deref() {
                Some(text) if !text.is_empty() => format!("{win}  \u{1d} dialog \u{1e} \n{text}"),
                Some(_) => format!("{win}  \u{1d} dialog \u{1e} "),
                None => win.to_owned(),
            };
            if control_speech.is_empty() || control_speech == win {
                heading
            } else {
                format!("{heading}\n{control_speech}")
            }
        } else {
            control_speech
        };

        if !speech.is_empty() {
            self.speech.speak(&speech, true);
            Some(speech)
        } else {
            None
        }
    }

    /// Handles menu mode transitions [obs, 302]. Leaving menus re-announces the control
    /// focused before them, as on a fresh focus: `Leaving menus` | `Name: ‹ Edit ›` | …
    pub fn handle_menu_event(&mut self, event: crate::event::MenuEvent) -> Option<String> {
        use crate::event::MenuEvent;
        match event {
            MenuEvent::Start => None,
            MenuEvent::PopupStart => {
                self.menus.popup_start();
                None
            }
            MenuEvent::PopupEnd => {
                self.menus.popup_end();
                None
            }
            MenuEvent::End => {
                if !self.menus.end() {
                    return None;
                }
                let mut speech = "Leaving menus".to_owned();
                if let Some(focus) = &self.last_native_focus {
                    speech.push('\n');
                    speech.push_str(&crate::native_focus::focus_speech(focus, None));
                }
                self.speech.speak(&speech, true);
                Some(speech)
            }
        }
    }

    /// Connects the engine to the real application, so that commands such as Enter on an
    /// edit act on the page and not only on the buffer.
    pub fn set_actions(&mut self, actions: Box<dyn crate::actions::ApplicationActions>) {
        self.actions = Some(actions);
    }

    /// Whether keys go to a text field: forms mode in a page, or a focused native edit or
    /// text document (Windows 11 Notepad, Word).
    fn editing_text(&self) -> bool {
        (self.forms_mode
            && self
                .node_under_cursor()
                .is_some_and(|n| n.role == Role::Edit))
            || (self.app_mode != AppMode::VirtualBuffer
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| matches!(f.role, Role::Edit | Role::Document)))
    }

    /// A JAWS key command (Insert+…). In a text field the reading commands read the field;
    /// elsewhere they run as virtual cursor or application commands.
    fn jaws_key_command(&mut self, event: &KeyEvent) {
        if self.editing_text() && crate::editing::EditKey::of(event).is_some() {
            // Say Line does not change the field, so no report follows: speak the field's
            // last known text now, or read it when none is known [obs, 305/s9, s15].
            let key = crate::editing::EditKey::SayLine;
            match self.last_text.clone() {
                Some(context) => {
                    let field = self.text_field_line();
                    if let Some(speech) = crate::editing::speech(key, None, &context, &field) {
                        self.speech.speak(&speech, true);
                    }
                }
                None => self.expect_text_change(key),
            }
            return;
        }
        if let Some(cmd) = event.to_command_string() {
            let cmd = self.repeated_command(cmd, event.time_ms);
            let _ = self.execute_command(&cmd);
        }
    }

    /// The command a JAWS-key command becomes when pressed twice quickly, e.g. Say System
    /// Time's second press speaks the date [obs, 308/s2, s4].
    fn repeated_command(&mut self, cmd: String, time_ms: u64) -> String {
        let twice = matches!(&self.last_press, Some((last, at))
            if *last == cmd && time_ms != 0 && time_ms.saturating_sub(*at) <= DOUBLE_PRESS_MS);
        self.last_press = (!twice).then(|| (cmd.clone(), time_ms));
        if !twice {
            return cmd;
        }
        match cmd.as_str() {
            "SaySystemTime" => "SaySystemDate".to_owned(),
            "SaySystemTimeWithSeconds" => "SayWeekNumber".to_owned(),
            "CapsLock" => "ToggleCapsLock".to_owned(),
            "Insert+F" => "SayFontInViewer".to_owned(),
            _ => cmd,
        }
    }

    /// CapsLock as the JAWS key: a single press is the JAWS key; pressed twice quickly it
    /// toggles Caps Lock (the second press reaches the system), which JAWS announces
    /// [obs, 310: "Caps lock on", then "Caps lock off"].
    fn caps_lock_press(&mut self, event: &KeyEvent) -> KeyAction {
        if self.repeated_command("CapsLock".to_owned(), event.time_ms) != "ToggleCapsLock" {
            return KeyAction::Consume;
        }
        let was_on = event.caps_lock_on.unwrap_or(self.caps_lock_on);
        self.caps_lock_on = !was_on;
        let speech = if was_on {
            "Caps lock off"
        } else {
            "Caps lock on"
        };
        self.speech.speak(speech, true);
        KeyAction::PassThrough
    }

    /// Say System Time and its variants, from the clock in the user's locale formats.
    fn clock_command(&self, command: &str) -> Option<String> {
        let clock = self.clock.as_deref();
        match command {
            "SaySystemTime" => Some(crate::clock::say_time(clock?, false)),
            "SaySystemTimeWithSeconds" => Some(crate::clock::say_time(clock?, true)),
            "SaySystemDate" => Some(crate::clock::say_date(clock?, false)),
            "SayWeekNumber" => Some(crate::clock::say_date(clock?, true)),
            _ => None,
        }
    }

    /// Say Font (`Insert+F`): the font at the cursor. Pressed twice quickly, JAWS shows it in
    /// the Virtual Viewer and reads the message [obs, 209/s7, 314/s2]; with no font it just
    /// says "No font" again (313/s2). Native controls report no font yet: JAWS reads
    /// Notepad's from the screen ("Consolas 22 point" for a 24-point LOGFONT, 314/s1), a
    /// conversion not yet known.
    fn say_font(&mut self, in_viewer: bool) -> Option<String> {
        let formatting = if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
            self.cursor.buffer.formatting_at_offset(self.cursor.offset)
        } else {
            // A text field reports the font at its caret.
            self.last_text
                .as_ref()
                .and_then(|t| t.font.as_ref())
                .map(crate::event::FontInfo::formatting)
        };
        let description = formatting
            .as_ref()
            .and_then(SpeechFormatter::font_description);
        let speech = match (in_viewer, description) {
            (true, Some(font)) if self.viewer.is_none() => {
                let viewer = crate::viewer::VirtualViewer::message(vec![
                    "Font information:".to_owned(),
                    font,
                    String::new(),
                    String::new(),
                    "\u{1d} Link \u{1e} List JAWS Hot Keys".to_owned(),
                    "Press ESCAPE to close this message.".to_owned(),
                ]);
                let speech = viewer.message_speech();
                self.viewer = Some(viewer);
                speech
            }
            _ => SpeechFormatter::format_font(formatting.as_ref()),
        };
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Virtualize Window or Virtualize Current Control: opens the Virtual Viewer on the
    /// window's or the control's text and reads its first line [obs, 311/s1, s14; 312/s1].
    fn virtualize(&mut self, window: bool) -> Option<String> {
        // A page's virtual cursor is a virtual view already: JAWS virtualizes neither the
        // window nor the control (observed on the dev box with JAWS 2025 in Chrome,
        // 2026-10-15; scenario 318 records it on old-pc).
        let in_page = self.app_mode == AppMode::VirtualBuffer && !self.forms_mode;
        if self.viewer.is_some() || in_page {
            // [obs, 311/s32]
            let speech = "The current location is already in a virtual view".to_owned();
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        let lines = self.virtualized_lines(window)?;
        let viewer = crate::viewer::VirtualViewer::new(lines);
        let what = if window { "window" } else { "control" };
        let speech = format!("Virtualizing current {what}\n{}", viewer.line_text());
        self.viewer = Some(viewer);
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// The Virtual Viewer's lines for the focused native control or its window:
    /// - a dialog box: its title, its text, and its buttons on one line [obs, 311/s14–s22];
    /// - a text field: its lines, after the window title for the whole window [obs, 312];
    /// - another control: its name and type, then a blank line [obs, 311/s1–s11].
    fn virtualized_lines(&self, window: bool) -> Option<Vec<String>> {
        let focus = self.last_native_focus.as_ref()?;
        let title = focus.window_title.clone().unwrap_or_default();
        if window && focus.window_kind.as_deref() == Some("#32770") {
            let mut lines = vec![title];
            // As laid out on screen: wrapped text by its rows, a label beside its field
            // (JAWS on the Run dialog: "Open: C:", "OK Cancel Browse...").
            if !focus.dialog_lines.is_empty() {
                lines.extend(focus.dialog_lines.iter().cloned());
                return Some(lines);
            }
            lines.extend(
                focus
                    .dialog_text
                    .iter()
                    .flat_map(|t| t.lines())
                    .map(str::to_owned),
            );
            if !focus.dialog_buttons.is_empty() {
                lines.push(focus.dialog_buttons.join(" "));
            }
            return Some(lines);
        }
        if self.editing_text() {
            if let Some(text) = &self.last_text {
                let mut lines: Vec<String> = Vec::new();
                if window {
                    lines.push(title);
                }
                lines.extend(crate::editing::text_lines(&text.text));
                return Some(lines);
            }
        }
        let name = focus.name.clone().unwrap_or_default();
        let control = match crate::native_focus::role_word(&focus.role) {
            Some(role) => format!("{name} {role}"),
            None => name,
        };
        Some(vec![control, " ".to_owned()])
    }

    /// What JAWS reads when the Virtual Viewer closes: the text field's line, or the focused
    /// control's name and type [obs, 311/s12, s23; 312/s11].
    fn speak_focus_again(&mut self) {
        let speech = if self.editing_text() {
            let line = self.last_text.as_ref().map(crate::editing::line_at);
            // A combo box's field adds its list position (JAWS on the Run dialog, dev box:
            // "C: | 1 of 6").
            let position = self
                .last_native_focus
                .as_ref()
                .filter(|f| f.container_role == Some(Role::ComboBox))
                .and_then(|f| Some(format!("{} of {}", f.item_index?, f.item_count?)));
            match (line, position) {
                (Some(line), Some(position)) => Some(format!("{line}\n{position}")),
                (line, _) => line,
            }
        } else if self.app_mode == AppMode::VirtualBuffer {
            // In a page, the virtual cursor's line [obs, 209/s11].
            Some(self.cursor.current_line())
        } else {
            self.last_native_focus
                .as_ref()
                .map(crate::native_focus::focus_line)
        };
        if let Some(speech) = speech.filter(|s| !s.is_empty()) {
            self.speech.speak(&speech, true);
        }
    }

    /// Gives the engine the system clock and locale formats (Say System Time).
    pub fn set_clock(&mut self, clock: Box<dyn crate::clock::Clock>) {
        self.clock = Some(clock);
    }

    /// Feedback for a key passed to a text field: keys that move the caret, select or
    /// delete are spoken once the application has handled them (from the field's text
    /// before and after); typed characters are echoed, as JAWS does with its default typing
    /// echo (characters).
    fn edit_key_feedback(&mut self, event: &KeyEvent) {
        if let Some(key) = crate::editing::EditKey::of(event) {
            self.expect_text_change(key);
        } else if let Some(c) = event.text {
            let echo = if c == ' ' {
                "space".to_owned()
            } else {
                SpeechFormatter::format_char(c)
            };
            tracing::debug!(%echo, "typing echo");
            self.speech.speak_echo(&echo);
            // Keep the field's known text current (no speech follows), so the next edit key
            // (Backspace) is spoken from the text as typed.
            if let Some(actions) = &self.actions {
                actions.read_at_caret(crate::event::CaretUnit::Character);
            }
        }
    }

    /// Remembers the edit key until the field reports its new text, and asks the platform
    /// to read it (applications do not all report caret moves).
    fn expect_text_change(&mut self, key: crate::editing::EditKey) {
        tracing::debug!(?key, "edit key; reading the field");
        self.pending_edit = Some(key);
        if let Some(actions) = &self.actions {
            actions.read_at_caret(crate::event::CaretUnit::Character);
        }
    }

    /// The focused text field's text changed: speaks the pending edit key's effect.
    fn handle_text_context(&mut self, context: &crate::event::TextContext) -> Option<String> {
        let before = self.last_text.replace(context.clone());
        let key = self.pending_edit.take()?;
        let field = self.text_field_line();
        let speech = crate::editing::speech(key, before.as_ref(), context, &field)?;
        tracing::debug!(?key, %speech, "edit speech");
        use crate::editing::EditKey;
        let character = matches!(
            key,
            EditKey::Character | EditKey::DeleteForward | EditKey::DeleteBackward
        ) && speech.chars().count() == 1;
        if character {
            self.speech.speak_character(&speech, true);
        } else {
            self.speech.speak(&speech, true);
        }
        Some(speech)
    }

    /// The focused text field's name and role, as Say Line speaks them.
    fn text_field_line(&self) -> String {
        let name = if self.forms_mode {
            self.node_under_cursor().and_then(|n| n.name)
        } else {
            self.last_native_focus.as_ref().and_then(|f| f.name.clone())
        };
        format!(
            "{}  \u{1d} Edit \u{1e} ",
            name.as_deref().unwrap_or_default().trim()
        )
    }
    /// The node of the line under the virtual cursor.
    fn node_under_cursor(&self) -> Option<AomNode> {
        let line = self.cursor.buffer.line_at_offset(self.cursor.offset)?;
        self.cursor.buffer.node_map.get(&line.node_id).cloned()
    }

    /// A page control changed (Space toggled a check box, a combo box took a new value): the
    /// buffer is rebuilt with the new state, the virtual cursor stays on the control, and the
    /// change is spoken.
    fn page_change(&mut self, control: &FocusEvent) -> Option<String> {
        let id = self.page_node_for(control)?;
        let before = self.cursor.buffer.node_map.get(&id)?.clone();
        let speech = match control.role {
            Role::CheckBox | Role::RadioButton
                if before.states.checked != control.states.checked =>
            {
                let state = if control.states.checked == Some(true) {
                    "checked"
                } else {
                    "not checked"
                };
                crate::linearize::state_marker(state)
            }
            Role::ComboBox if before.value != control.value => control.value.clone()?,
            // Space on a tab selects it [obs, 29/s4].
            Role::Tab
                if before.states.selected != control.states.selected
                    && control.states.selected == Some(true) =>
            {
                crate::linearize::state_marker("selected")
            }
            _ => return None,
        };
        let rebuilt = self.cursor.buffer.with_node_changed(id, |node| {
            node.states.checked = control.states.checked;
            node.states.selected = control.states.selected;
            if control.value.is_some() {
                node.value = control.value.clone();
            }
        })?;
        self.cursor.buffer = rebuilt;
        self.cursor.move_to_node_id(id);
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// The region markers for the landmarks around a page node, outermost first: `main`
    /// and forms (named or not) [obs, 19/s2; 204/s1].
    fn region_markers(&self, node_id: usize) -> String {
        fn path<'a>(node: &'a AomNode, id: usize, out: &mut Vec<&'a AomNode>) -> bool {
            if node.id == id {
                return true;
            }
            out.push(node);
            if node.children.iter().any(|c| path(c, id, out)) {
                return true;
            }
            out.pop();
            false
        }
        let Some(root) = self
            .cursor
            .buffer
            .root_id
            .and_then(|r| self.cursor.buffer.node_map.get(&r))
        else {
            return String::new();
        };
        let mut ancestors = Vec::new();
        path(root, node_id, &mut ancestors);
        ancestors
            .iter()
            .filter_map(|n| {
                let aria = n.attributes.get("aria_role").map(String::as_str);
                match &n.role {
                    Role::Landmark { landmark_type } if landmark_type == "main" => {
                        Some("MainRegion")
                    }
                    Role::Landmark { landmark_type } if landmark_type == "form" => {
                        Some("FormRegion")
                    }
                    _ if aria == Some("form") => Some("FormRegion"),
                    _ => None,
                }
            })
            .map(|r| format!(" \u{1d} {r} \u{1e} \n"))
            .collect()
    }

    /// The buffer node a page event concerns: by node id, then by DOM id, then by role and
    /// name. Candidates are taken in document order, so repeated names resolve to the first.
    fn page_node_for(&self, focus: &FocusEvent) -> Option<usize> {
        let map = &self.cursor.buffer.node_map;
        if let Some(id) = focus.node_id.filter(|id| map.contains_key(id)) {
            return Some(id);
        }
        let mut ids: Vec<usize> = map.keys().copied().collect();
        ids.sort_unstable();
        let node = |id: &usize| map.get(id);
        if let Some(dom_id) = focus.dom_id.as_deref() {
            if let Some(id) = ids.iter().find(|id| {
                node(id).is_some_and(|n| n.attributes.get("id").map(String::as_str) == Some(dom_id))
            }) {
                return Some(*id);
            }
        }
        let name = focus.name.as_deref()?;
        ids.into_iter().find(|id| {
            node(id).is_some_and(|n| {
                n.role == focus.role
                    && n.name
                        .as_deref()
                        .or(n.text.as_deref())
                        .is_some_and(|nn| nn.eq_ignore_ascii_case(name))
            })
        })
    }

    /// The focused control changed without focus moving (Space on a check box, a tree item
    /// expanded, a combo box value). Speaks what changed, as JAWS does.
    pub fn handle_change(
        &mut self,
        kind: crate::event::ChangeKind,
        control: &FocusEvent,
    ) -> Option<String> {
        tracing::debug!(?kind, role = ?control.role, name = ?control.name, "control changed");
        if control.in_document {
            return self.page_change(control);
        }
        let before = self.last_native_focus.replace(control.clone())?;
        let speech = crate::native_focus::change_speech(&before, control)?;
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Replaces the buffer with the page refreshed after a change, keeping the virtual
    /// cursor's place.
    fn apply_pending_page(&mut self) {
        if let Some(buffer) = self.pending_page.take() {
            let offset = self.cursor.offset;
            self.cursor.buffer = buffer;
            self.cursor.offset = offset.min(self.cursor.buffer.len());
            self.last_page_focus = None;
        }
    }

    /// Completes an edit key the application did not act on (Down on the last line, Ctrl+C):
    /// the field's text is unchanged, so no change event follows, but JAWS still speaks
    /// (313/s5–s8). Live, the platform reads the field after every edit key and reports it
    /// changed or not; replay, which has only the recorded change events, calls this after a
    /// step's events.
    pub fn settle_pending_edit(&mut self) -> Option<String> {
        self.pending_edit?;
        let unchanged = self.last_text.clone()?;
        self.handle_accessibility_event(&AccessibilityEvent::TextContext(unchanged))
    }

    /// Called when the OS event stream goes quiet: speech JAWS defers until no further
    /// event supersedes it (a closed popup menu, unless focus moved on).
    pub fn on_idle(&mut self) -> Option<String> {
        self.apply_pending_page();
        let speech = self.menus.idle()?;
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Handles an ARIA live region announcement event with politeness consideration.
    pub fn handle_live_region_event(&mut self, live: &LiveRegionEvent) -> Option<String> {
        // JAWS speaks what was added to the region, after the region's name: "Messages |
        // You: Message 1" [obs, 206/s2-s3]. The region's previous text is in the buffer.
        let added = self.live_region_additions(live);
        if added.trim().is_empty() {
            return None;
        }
        let text = match live.container_name.as_deref() {
            Some(name) => format!("{name}\n{added}"),
            None => added,
        };
        match live.politeness {
            Politeness::Assertive => {
                self.speech.stop();
                self.speech.speak(&text, true);
                Some(text)
            }
            Politeness::Polite => {
                self.speech.speak(&text, false);
                Some(text)
            }
            Politeness::Off => None,
        }
    }

    /// The lines of a live region's text that its previous text in the buffer lacked.
    fn live_region_additions(&mut self, live: &LiveRegionEvent) -> String {
        fn texts(node: &AomNode, out: &mut Vec<String>) {
            if let Some(t) = node
                .text
                .as_deref()
                .map(str::trim)
                .filter(|t| !t.is_empty())
            {
                out.push(t.to_owned());
            }
            for child in &node.children {
                texts(child, out);
            }
        }
        let map = &self.cursor.buffer.node_map;
        let mut ids: Vec<usize> = map.keys().copied().collect();
        ids.sort_unstable();
        let region = live.region_id.as_deref().and_then(|dom_id| {
            ids.iter().find_map(|id| {
                map.get(id)
                    .filter(|n| n.attributes.get("id").map(String::as_str) == Some(dom_id))
            })
        });
        let mut old = Vec::new();
        if let Some(region) = region {
            texts(region, &mut old);
        }
        // The region's text when it last changed, if newer than the page: the page is
        // refreshed after the change, so two quick changes (two chat messages) would otherwise
        // compare the second with a page that lacks the first.
        if let Some(last) = live
            .region_id
            .as_ref()
            .and_then(|id| self.last_live_text.get(id))
        {
            old = last.clone();
        }
        if let Some(id) = &live.region_id {
            self.last_live_text
                .insert(id.clone(), live.text.lines().map(str::to_owned).collect());
        }
        let new: Vec<&str> = live.text.lines().collect();
        let added = if old.len() <= new.len() && old.iter().zip(&new).all(|(o, n)| o.as_str() == *n)
        {
            new.get(old.len()..).unwrap_or_default().to_vec()
        } else {
            new
        };
        added.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::{AomNode, Role};
    use crate::input::Modifiers;
    use crate::speech::MockSpeechOutput;

    fn create_test_buffer() -> VirtualBuffer {
        let aom = AomNode::new(0, Role::Document)
            .with_name("Interactive Test Document")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 }).with_text("Main Title"),
                AomNode::new(2, Role::Paragraph).with_text("This is interactive paragraph."),
                AomNode::new(3, Role::Edit).with_name("Search Input"),
                AomNode::new(4, Role::Button).with_name("Submit Button"),
                {
                    let mut cb = AomNode::new(5, Role::CheckBox).with_name("Subscribe Checkbox");
                    cb.states.checked = Some(true);
                    cb
                },
            ]);
        VirtualBuffer::from_aom(&aom)
    }

    #[test]
    fn test_engine_navigation_and_speech() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Down arrow navigation
        let speech = engine.execute_command("{DOWN}");
        assert_eq!(
            speech,
            Some(" \u{001d} heading level 1 \u{001e} Main Title".to_string())
        );

        // Heading navigation
        let h_speech = engine.execute_command("H");
        assert_eq!(
            h_speech,
            Some("wrapping to top\nMain Title\n \u{001d} heading level 1 \u{001e}".to_string())
        );
    }

    #[test]
    fn test_engine_key_event_handling_and_forms_mode() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Move to edit field
        engine.execute_command("E");

        // Press Enter to enter forms mode
        let enter_event = KeyEvent::new(Key::Enter, Modifiers::none(), true);
        let action = engine.handle_key_event(&enter_event);
        assert_eq!(action, KeyAction::Consume);
        assert!(engine.forms_mode);

        // In forms mode, regular keys pass through
        let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);

        // Press Escape to exit forms mode
        let esc_event = KeyEvent::new(Key::Escape, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&esc_event), KeyAction::Consume);
        assert!(!engine.forms_mode);

        // Back in Virtual PC Cursor mode, Down arrow is consumed
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::Consume);
    }

    #[test]
    fn test_engine_auto_forms_mode_on_focus_edit() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        assert!(!engine.forms_mode);

        // Focus event moving into Search Input
        let focus_ev = FocusEvent::new(Role::Edit)
            .with_node_id(3)
            .with_name("Search Input");
        let speech = engine.handle_focus_event(&focus_ev).unwrap_or_default();

        // Forms mode follows focus into the edit silently; a page edit has no typing hint
        // [obs, 204/s1-s6].
        assert!(engine.forms_mode);
        assert!(engine.cursor.forms_mode_active);
        assert_eq!(engine.cursor.current_node_id, Some(3));
        assert!(speech.contains("Search Input"));
        assert!(!speech.contains("Type in text."));

        // Focus transitions to Submit Button -> should auto exit forms mode
        let btn_focus = FocusEvent::new(Role::Button)
            .with_node_id(4)
            .with_name("Submit Button");
        let btn_speech = engine.handle_focus_event(&btn_focus).unwrap_or_default();

        assert!(!engine.forms_mode);
        assert!(!engine.cursor.forms_mode_active);
        assert_eq!(engine.cursor.current_node_id, Some(4));
        assert!(btn_speech.contains("Submit Button"));
        assert!(btn_speech.contains("To activate press Enter."));
        assert!(!btn_speech.contains("virtual PC Cursor"));
    }

    #[test]
    fn test_engine_live_region_announcements() {
        let buffer = create_test_buffer();
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Assertive live region
        let alert = LiveRegionEvent::new("Security update installed.", Politeness::Assertive);
        let alert_speech = engine.handle_live_region_event(&alert).unwrap_or_default();
        assert_eq!(alert_speech, "Security update installed.");

        // Polite live region
        let status = LiveRegionEvent::new("Download finished.", Politeness::Polite);
        let status_speech = engine.handle_live_region_event(&status).unwrap_or_default();
        assert_eq!(status_speech, "Download finished.");

        // Politeness::Off
        let off = LiveRegionEvent::new("Ignored mutation.", Politeness::Off);
        assert_eq!(engine.handle_live_region_event(&off), None);
    }

    #[test]
    fn a_page_is_already_a_virtual_view() {
        let aom =
            AomNode::new(0, Role::Document)
                .with_children(vec![AomNode::new(1, Role::Paragraph).with_text("Hello")]);
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&aom));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        engine.load_buffer(VirtualBuffer::from_aom(&aom));
        for command in ["VirtualizeWindow", "VirtualizeControl"] {
            assert_eq!(
                engine.execute_command(command).as_deref(),
                Some("The current location is already in a virtual view")
            );
        }
        assert!(engine.viewer.is_none());
    }

    #[test]
    fn test_engine_text_selection_and_font_query() {
        let aom =
            AomNode::new(0, Role::Document).with_children(vec![AomNode::new(1, Role::Paragraph)
                .with_text("Hello World")
                .with_formatting(
                    crate::aom::TextFormatting::new()
                        .with_size(16.0)
                        .with_bold(true)
                        .with_family("Arial")
                        .with_alignment("left"),
                )]);
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Font query, in JAWS's phrasing [obs, 209/s3]
        let font_speech = engine.execute_command("Insert+F");
        assert_eq!(
            font_speech,
            Some("Font is bold  Arial 16 point".to_string())
        );

        // Select next character
        let sel1 = engine.execute_command("+{RIGHT}");
        assert_eq!(sel1, Some("selected H".to_string()));

        let sel2 = engine.execute_command("+{RIGHT}");
        assert_eq!(sel2, Some("selected e".to_string()));

        // Unselect prior character
        let unsel = engine.execute_command("+{LEFT}");
        assert_eq!(unsel, Some("unselected e".to_string()));

        // Query active selection
        let say_sel = engine.execute_command("SaySelection");
        assert_eq!(say_sel, Some("H".to_string()));

        // Moving cursor clears selection
        engine.execute_command("{RIGHT}");
        let no_sel = engine.execute_command("SaySelection");
        assert_eq!(no_sel, Some("No selection".to_string()));
    }

    #[test]
    fn test_engine_modal_dialog_confinement_and_escape() {
        let mut dlg = AomNode::new(10, Role::Dialog)
            .with_name("Confirm Deletion")
            .with_children(vec![
                AomNode::new(11, Role::Heading { level: 2 }).with_text("Are you sure?"),
                AomNode::new(12, Role::Button).with_name("Confirm"),
                AomNode::new(13, Role::Button).with_name("Cancel"),
            ]);
        dlg.states.modal = true;

        let aom = AomNode::new(0, Role::Document).with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Dashboard"),
            dlg,
            AomNode::new(20, Role::Paragraph).with_text("Backdrop footer text"),
        ]);

        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Focus event moving into dialog
        let focus_dlg = FocusEvent::new(Role::Dialog)
            .with_node_id(10)
            .with_name("Confirm Deletion");
        let speech = engine.handle_focus_event(&focus_dlg).unwrap_or_default();
        assert_eq!(speech, "Dialog Confirm Deletion");
        assert!(engine.cursor.is_in_modal());

        // Quick navigation 'H' inside modal wraps within modal and finds level 2 heading
        let h_speech = engine.execute_command("H").unwrap_or_default();
        assert!(h_speech.contains("Are you sure?"));

        // Tab moves to Confirm button
        let tab1 = engine.execute_command("{TAB}").unwrap_or_default();
        assert!(tab1.contains("Confirm"));

        // Tab moves to Cancel button
        let tab2 = engine.execute_command("{TAB}").unwrap_or_default();
        assert!(tab2.contains("Cancel"));

        // Tab wraps back to Confirm button inside dialog (cannot escape to backdrop!)
        let tab3 = engine.execute_command("{TAB}").unwrap_or_default();
        assert!(tab3.contains("Confirm"));

        // Escape exits modal dialog
        let esc_event = KeyEvent::new(Key::Escape, Modifiers::none(), true);
        let action = engine.handle_key_event(&esc_event);
        assert_eq!(action, KeyAction::Consume);
        assert!(!engine.cursor.is_in_modal());
    }

    #[test]
    fn test_engine_desktop_entry_vs_subsequent_item_navigation() {
        let aom = AomNode::new(0, Role::Document);
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // First entry to Desktop (e.g. Win+M / Win+D)
        let focus_entry = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("Progman")
            .with_name("Dropbox")
            .with_item_index(7)
            .with_item_count(11);
        let s1 = engine.handle_focus_event(&focus_entry).unwrap_or_default();
        assert!(s1.starts_with("desktop\nFolderView    List view"));
        assert!(s1.contains("Dropbox"));
        assert!(s1.contains("7 of 11"));
        assert!(s1.contains("To move to items use the Arrow keys."));

        // Subsequent item navigation (e.g. Down Arrow to Microsoft Edge)
        let focus_down = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("Progman")
            .with_name("Microsoft Edge")
            .with_item_index(9)
            .with_item_count(11);
        let s2 = engine.handle_focus_event(&focus_down).unwrap_or_default();
        assert_eq!(s2, "Microsoft Edge\n9 of 11");

        // Initial letter jump (e.g. 'j' to JAWS 2025)
        let focus_letter = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("Progman")
            .with_name("JAWS 2025")
            .with_item_index(4)
            .with_item_count(11);
        let s3 = engine.handle_focus_event(&focus_letter).unwrap_or_default();
        assert_eq!(s3, "JAWS 2025\n4 of 11");

        // Moving to another item directly on desktop speaks item-only
        let focus_recycle = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("SysListView32")
            .with_name("Recycle Bin")
            .with_item_index(1)
            .with_item_count(11);
        let s4 = engine
            .handle_focus_event(&focus_recycle)
            .unwrap_or_default();
        assert_eq!(s4, "Recycle Bin\n1 of 11");
    }

    #[test]
    fn window_title_on_the_desktop_is_the_virtual_desktop() {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let edit = FocusEvent::new(Role::Edit)
            .with_window_title("Untitled - Notepad")
            .with_window_class("Edit");
        engine.handle_focus_event(&edit);
        assert_eq!(
            engine.execute_command("SayWindowTitle").as_deref(),
            Some("Title is\nUntitled - Notepad")
        );
        // Win+M: the desktop's list takes focus; the tracer names the window after the
        // virtual desktop.
        let desktop = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("SysListView32")
            .with_window_title("Desktop 1")
            .with_name("Recycle Bin")
            .with_item_index(1)
            .with_item_count(3);
        let entry = engine.handle_focus_event(&desktop).unwrap_or_default();
        assert!(entry.starts_with("desktop\n"), "{entry:?}");
        assert_eq!(
            engine.execute_command("SayWindowTitle").as_deref(),
            Some("Title is\nDesktop 1")
        );
        // Back in Notepad, its window is announced again.
        let back = engine.handle_focus_event(&edit).unwrap_or_default();
        assert!(back.starts_with("Untitled - Notepad"), "{back:?}");
    }

    /// Windows 11 sends the desktop's focus several times, the first from no window class
    /// and with the list named "Desktop": it is announced once, as the desktop.
    #[test]
    fn the_desktop_is_announced_once() {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let item = |class: Option<&str>, container: &str| {
            let mut focus = FocusEvent::new(Role::ListItem)
                .with_container_name(container)
                .with_window_title("Desktop 1")
                .with_name("Microsoft Edge")
                .with_item_index(9)
                .with_item_count(11);
            focus.window_class = class.map(str::to_owned);
            focus.window_kind = Some("Progman".to_owned());
            focus
        };
        let first = engine
            .handle_focus_event(&item(None, "Desktop"))
            .unwrap_or_default();
        assert!(
            first.starts_with("desktop\nFolderView    List view"),
            "{first:?}"
        );
        assert_eq!(
            engine.handle_focus_event(&item(Some("Progman"), "FolderView")),
            None
        );
        assert_eq!(
            engine.handle_focus_event(&item(Some("SysListView32"), "FolderView")),
            None
        );
    }

    #[test]
    fn task_switcher_items_after_the_desktop_are_not_desktop_items() {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let desktop = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("SysListView32")
            .with_window_title("Desktop 1")
            .with_name("Recycle Bin")
            .with_item_index(1)
            .with_item_count(3);
        engine.handle_focus_event(&desktop);
        // Alt+Tab: the switcher's item takes focus through UIA.
        let mut item = FocusEvent::new(Role::ListItem)
            .with_window_class("ForegroundStaging")
            .with_window_title("Task Switching")
            .with_name("Untitled - Notepad");
        item.window_kind = Some("MultitaskingViewFrame".to_owned());
        // The switcher's item, not a desktop item ("Untitled - Notepad | 1 of 3").
        assert_eq!(
            engine.handle_focus_event(&item).as_deref(),
            Some("Untitled - Notepad\nTo move to an item press the Arrow keys.")
        );
    }

    #[test]
    fn test_engine_file_explorer_entry_and_item_navigation() {
        let aom = AomNode::new(0, Role::Document);
        let buffer = VirtualBuffer::from_aom(&aom);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Desktop item
        let desktop_focus = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("Progman")
            .with_name("Dropbox")
            .with_item_index(7)
            .with_item_count(11);
        let _ = engine.handle_focus_event(&desktop_focus);

        // User presses Enter on Dropbox -> Opens File Explorer window
        let mut explorer_entry = FocusEvent::new(Role::ListItem)
            .with_window_title("Dropbox - File Explorer")
            .with_folder_name("Dropbox")
            .with_container_name("Items View")
            .with_window_class("CabinetWClass")
            .with_name(".cache")
            .with_item_index(1)
            .with_item_count(143);
        explorer_entry.states.selected = Some(false);

        let s1 = engine
            .handle_focus_event(&explorer_entry)
            .unwrap_or_default();
        assert_eq!(
            s1,
            "Dropbox - File Explorer\nDropbox\nItems view multi select box\nNot selected\n.cache\n1 of 143"
        );

        // Press 'a' inside File Explorer -> ailabs
        let explorer_item = FocusEvent::new(Role::ListItem)
            .with_window_title("Dropbox - File Explorer")
            .with_folder_name("Dropbox")
            .with_container_name("Items View")
            .with_window_class("CabinetWClass")
            .with_name("ailabs")
            .with_item_index(3)
            .with_item_count(143);
        let s2 = engine
            .handle_focus_event(&explorer_item)
            .unwrap_or_default();
        assert_eq!(s2, "ailabs\n3 of 143");

        // Focus shifts back to Desktop -> Full entry announcement re-triggers
        let back_to_desktop = FocusEvent::new(Role::ListItem)
            .with_container_name("FolderView")
            .with_window_class("Progman")
            .with_name("Dropbox")
            .with_item_index(7)
            .with_item_count(11);
        let s3 = engine
            .handle_focus_event(&back_to_desktop)
            .unwrap_or_default();
        assert!(s3.contains("FolderView    List view"));
        assert!(s3.contains("7 of 11"));
    }

    #[test]
    fn test_engine_native_control_key_passthrough_and_focus_announcement() {
        let empty_doc = AomNode::new(0, Role::Document);
        let buffer = VirtualBuffer::from_aom(&empty_doc);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);
        engine.app_mode = AppMode::NativeControl;

        // In NativeControl mode, arrow keys, Tab, Enter, typing MUST pass through!
        let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);

        let up_event = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&up_event), KeyAction::PassThrough);

        let tab_event = KeyEvent::new(Key::Tab, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&tab_event), KeyAction::PassThrough);

        let t_key = KeyEvent::new(Key::Alpha('t'), Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&t_key), KeyAction::PassThrough);
    }

    #[test]
    fn test_jaws_key_combinations_are_consumed_in_native_apps() {
        let empty_doc = AomNode::new(0, Role::Document);
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&empty_doc));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        engine.app_mode = AppMode::NativeControl;

        let mut ins_t = KeyEvent::new(Key::Alpha('t'), Modifiers::none(), true);
        ins_t.modifiers.insert = true;
        assert_eq!(engine.handle_key_event(&ins_t), KeyAction::Consume);
    }

    /// Speech shared with the test, for commands spoken from key events.
    #[derive(Clone, Default)]
    struct SharedSpeech(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl SpeechOutput for SharedSpeech {
        fn speak(&mut self, text: &str, _interrupt: bool) {
            if let Ok(mut said) = self.0.lock() {
                said.push(text.to_owned());
            }
        }
        fn stop(&mut self) {}
    }

    impl SharedSpeech {
        fn take(&self) -> Vec<String> {
            self.0
                .lock()
                .map(|mut s| s.drain(..).collect())
                .unwrap_or_default()
        }
    }

    /// In the laptop layout CapsLock is the JAWS key and Insert works as well; CapsLock
    /// pressed twice quickly toggles Caps Lock [obs, 309, 310].
    #[test]
    fn test_caps_lock_as_the_jaws_key() {
        let empty_doc = AomNode::new(0, Role::Document);
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&empty_doc));
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
        engine.app_mode = AppMode::NativeControl;
        engine.last_window_title = Some("SR Editor Fixture".to_owned());

        let caps = |time_ms: u64, repeat: bool| {
            let mut key = KeyEvent::new(Key::CapsLock, Modifiers::none(), true);
            key.modifiers.caps_lock = true;
            key.time_ms = time_ms;
            key.is_repeat = repeat;
            key
        };
        let mut caps_t = KeyEvent::new(Key::Alpha('T'), Modifiers::none(), true);
        caps_t.modifiers.caps_lock = true;

        // Desktop layout (the default): CapsLock is an ordinary key.
        assert_eq!(
            engine.handle_key_event(&caps(1_000, false)),
            KeyAction::PassThrough
        );
        assert_ne!(engine.handle_key_event(&caps_t), KeyAction::Consume);
        assert!(speech.take().is_empty());

        engine.settings = crate::settings::Settings::reference_machine();
        assert_eq!(
            engine.handle_key_event(&caps(5_000, false)),
            KeyAction::Consume
        );
        // Held down, the key repeats: that is not a second press.
        assert_eq!(
            engine.handle_key_event(&caps(5_100, true)),
            KeyAction::Consume
        );
        assert_eq!(engine.handle_key_event(&caps_t), KeyAction::Consume);
        assert_eq!(speech.take(), ["Title is\nSR Editor Fixture"]);

        assert_eq!(
            engine.handle_key_event(&caps(9_000, false)),
            KeyAction::Consume
        );
        assert_eq!(
            engine.handle_key_event(&caps(9_300, false)),
            KeyAction::PassThrough
        );
        assert_eq!(speech.take(), ["Caps lock on"]);
        assert_eq!(
            engine.handle_key_event(&caps(12_000, false)),
            KeyAction::Consume
        );
        assert_eq!(
            engine.handle_key_event(&caps(12_300, false)),
            KeyAction::PassThrough
        );
        assert_eq!(speech.take(), ["Caps lock off"]);
    }

    /// Insert+F12 speaks the time; pressed twice quickly, the time and then the date. With
    /// Ctrl, seconds, then the week [obs, 308/s1–s4].
    #[test]
    fn test_say_system_time_and_its_double_press() {
        use crate::clock::{FixedClock, LocalDateTime, LocaleFormats};
        let empty_doc = AomNode::new(0, Role::Document);
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&empty_doc));
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
        engine.app_mode = AppMode::NativeControl;
        let recorded = LocalDateTime::from_unix_ms(1_792_005_107_069, 180);
        engine.set_clock(Box::new(FixedClock::new(recorded, LocaleFormats::en_us())));

        let press = |engine: &mut ScreenReaderEngine, ctrl: bool, time_ms: u64| {
            let mut key = KeyEvent::new(Key::F(12), Modifiers::none(), true);
            key.modifiers.insert = true;
            key.modifiers.ctrl = ctrl;
            key.time_ms = time_ms;
            assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
        };
        press(&mut engine, false, 1_000);
        press(&mut engine, false, 5_000);
        assert_eq!(speech.take(), ["10:11 PM", "10:11 PM"], "separate presses");
        press(&mut engine, false, 9_000);
        press(&mut engine, false, 9_300);
        assert_eq!(
            speech.take(),
            ["10:11 PM", "Wednesday, October 14, 2026"],
            "twice quickly"
        );
        press(&mut engine, true, 20_000);
        press(&mut engine, true, 20_300);
        assert_eq!(speech.take(), ["10:11:47 PM", "Week 42"]);
        // A third quick press starts over.
        press(&mut engine, true, 20_600);
        assert_eq!(speech.take(), ["10:11:47 PM"]);
    }

    /// In a text field, the JAWS key is consumed too, and Say Line (Insert+Up) reads the
    /// caret's line instead of reaching the field.
    #[test]
    fn test_jaws_key_in_forms_mode_reads_the_caret_line() {
        let mut cursor = VirtualCursor::new(create_test_buffer());
        assert!(cursor.move_to_node_id(3), "the test page has an edit");
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        let actions = crate::actions::RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));
        engine.forms_mode = true;

        let insert = KeyEvent::new(Key::Insert, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&insert), KeyAction::Consume);
        let mut say_line = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
        say_line.modifiers.insert = true;
        assert_eq!(engine.handle_key_event(&say_line), KeyAction::Consume);
        assert_eq!(actions.requests().len(), 1, "one read of the field");
        let spoken = engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
            crate::event::TextContext {
                text: "weather today".to_owned(),
                caret: 3,
                selection: None,
                font: None,
            },
        ));
        assert_eq!(
            spoken.as_deref(),
            Some("Search Input  \u{1d} Edit \u{1e} \nweather today")
        );
    }

    #[test]
    fn test_engine_dynamic_document_loaded_and_virtual_pc_cursor_navigation() {
        let empty_doc = AomNode::new(0, Role::Document);
        let buffer = VirtualBuffer::from_aom(&empty_doc);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);
        engine.app_mode = AppMode::NativeControl;

        // User switches to Chrome: DocumentLoaded event received
        let web_aom = AomNode::new(0, Role::Document)
            .with_name("ALTAI - Model and Agent Platform")
            .with_children(vec![
                AomNode::new(1, Role::Heading { level: 1 }).with_text("Main Heading"),
                AomNode::new(2, Role::Link { href: None }).with_name("Documentation Link"),
                AomNode::new(3, Role::Button).with_name("Get Started Button"),
            ]);

        let load_event = AccessibilityEvent::DocumentLoaded {
            title: "ALTAI - Model and Agent Platform".to_string(),
            root: web_aom,
        };

        let announcement = engine
            .handle_accessibility_event(&load_event)
            .unwrap_or_default();
        assert!(announcement.contains("ALTAI - Model and Agent Platform"));
        assert!(announcement.contains("Virtual PC Cursor"));
        assert_eq!(engine.app_mode, AppMode::VirtualBuffer);

        // Now in VirtualBuffer mode: Down Arrow navigates virtual lines and consumes
        let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::Consume);

        // H key moves to heading and consumes
        let h_key = KeyEvent::new(Key::Alpha('h'), Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&h_key), KeyAction::Consume);

        // User switches to Windows Terminal (native control)
        let terminal_focus = FocusEvent::new(Role::Window)
            .with_window_title("Command Prompt")
            .with_name("Command Prompt");
        let term_speech = engine
            .handle_focus_event(&terminal_focus)
            .unwrap_or_default();
        assert!(term_speech.contains("Command Prompt"));
        assert_eq!(engine.app_mode, AppMode::NativeControl);

        // In terminal: Down Arrow immediately passes through!
        assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);
        // H key immediately passes through!
        assert_eq!(engine.handle_key_event(&h_key), KeyAction::PassThrough);
    }

    #[test]
    fn test_engine_window_change_announcement_in_native_control() {
        let empty_doc = AomNode::new(0, Role::Document);
        let buffer = VirtualBuffer::from_aom(&empty_doc);
        let cursor = VirtualCursor::new(buffer);
        let mock_speech = Box::new(MockSpeechOutput::new());
        let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

        // Focus enters Notepad
        let notepad_focus = FocusEvent::new(Role::Edit)
            .with_window_title("Untitled - Notepad")
            .with_name("Text Editor");
        let s1 = engine
            .handle_focus_event(&notepad_focus)
            .unwrap_or_default();
        assert!(s1.contains("Untitled - Notepad"));
        assert!(s1.contains("Text Editor"));

        // Focus moves to another control within same Notepad window -> Does NOT repeat window title
        let notepad_btn = FocusEvent::new(Role::Button)
            .with_window_title("Untitled - Notepad")
            .with_name("File");
        let s2 = engine.handle_focus_event(&notepad_btn).unwrap_or_default();
        assert!(!s2.contains("Untitled - Notepad"));
        assert!(s2.contains("File"));

        // Focus switches to Command Prompt -> Announces new window title
        let cmd_focus = FocusEvent::new(Role::Window)
            .with_window_title("Command Prompt")
            .with_name("Command Prompt");
        let s3 = engine.handle_focus_event(&cmd_focus).unwrap_or_default();
        assert!(s3.contains("Command Prompt"));
    }
}
