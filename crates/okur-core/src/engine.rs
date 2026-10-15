//! The screen reader engine: keys and accessibility events in, speech out.
//!
//! This module holds the engine's state and its two entry points,
//! [`ScreenReaderEngine::handle_key_event`] and
//! [`ScreenReaderEngine::handle_accessibility_event`]. The handlers live in submodules:
//! `commands` (reading commands and JAWS-key chords), `focus` (focus and menu events),
//! `text_feedback` (what edit keys say), `page` (pages, changes, selections, live regions) and
//! `state` (list, desktop, taskbar and File Explorer navigation state).

use crate::aom::{AomNode, Role};
use crate::buffer::VirtualBuffer;
use crate::cursor::VirtualCursor;
use crate::event::{AccessibilityEvent, FocusEvent};
use crate::input::{Key, KeyAction, KeyEvent};
use crate::speech::SpeechOutput;

/// The longest gap between two presses of a JAWS command that counts as "twice quickly"
/// (the Windows default double-click time).
const DOUBLE_PRESS_MS: u64 = 500;

mod commands;
mod focus;
mod page;
mod state;
#[cfg(test)]
mod tests;
mod text_feedback;

pub use state::{
    ActiveElementList, AppMode, DesktopItem, NativeDesktopState, NativeExplorerState, ShellElement,
    ShellNavState,
};

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
    /// A Virtual Viewer waiting for the focused text to be read (a console): `Some(window)`
    /// for Virtualize Window, `Some(false)` for Virtualize Current Control.
    pending_viewer: Option<bool>,
    /// Alt+Tab's task switcher, read as JAWS reads it.
    task_switcher: crate::apps::task_switcher::TaskSwitcher,
    /// Each live region's lines when it last changed, by DOM id.
    last_live_text: std::collections::HashMap<String, Vec<String>>,
    /// Live regions being read as they grow, by DOM id (`LiveRegionAppend::Appended`).
    live_streams: std::collections::HashMap<String, page::LiveStream>,
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
            pending_viewer: None,
            task_switcher: Default::default(),
            last_live_text: std::collections::HashMap::new(),
            live_streams: std::collections::HashMap::new(),
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
            AccessibilityEvent::Selected(item) => self.handle_selected(item),
            AccessibilityEvent::LiveRegion(live) => self.handle_live_region_event(live),
            AccessibilityEvent::DocumentLoaded { title, root } => {
                self.handle_document_loaded(title, VirtualBuffer::from_aom(root))
            }
        }
    }

    /// Connects the engine to the real application, so that commands such as Enter on an
    /// edit act on the page and not only on the buffer.
    pub fn set_actions(&mut self, actions: Box<dyn crate::actions::ApplicationActions>) {
        self.actions = Some(actions);
    }

    /// Gives the engine the system clock and locale formats (Say System Time).
    pub fn set_clock(&mut self, clock: Box<dyn crate::clock::Clock>) {
        self.clock = Some(clock);
    }
}
