//! The screen reader engine: keys and accessibility events in, speech out.
//!
//! This module holds the engine's state and its two entry points,
//! [`ScreenReaderEngine::handle_key_event`] and
//! [`ScreenReaderEngine::handle_accessibility_event`]. The handlers live in submodules:
//! `commands` (reading commands and JAWS-key chords), `focus` (focus and menu events),
//! `text_feedback` (what edit keys say), `page` (pages, changes, selections, live regions) and
//! `state` (list, desktop, taskbar and File Explorer navigation state).

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

pub use state::{ActiveElementList, AppMode};

/// A key press counted for "pressed twice quickly": a command, or CapsLock alone (which the
/// laptop layout's JAWS key also is).
#[derive(Debug, Clone, PartialEq, Eq)]
enum Press {
    Command(crate::command::Command),
    CapsLock,
}

/// A Virtual Viewer that opens once its text is read; `window` is Virtualize Window, not
/// Virtualize Current Control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingViewer {
    /// A console's buffer, arriving as a text context.
    Console { window: bool },
    /// What the window or control shows, arriving as viewer text.
    Screen { window: bool },
}

/// Primary runtime engine for screen reader execution, coordinating virtual cursor navigation,
/// speech output, and interactive input event handling.
pub struct ScreenReaderEngine {
    pub cursor: VirtualCursor,
    pub speech: Box<dyn SpeechOutput>,
    pub forms_mode: bool,
    pub active_element_list: Option<ActiveElementList>,
    pub app_mode: AppMode,
    pub last_window_title: Option<String>,
    /// The active window's class, with `last_window_title` identifying the window.
    last_window_kind: Option<String>,
    /// Container of the last focused native control; entering a new one announces it.
    native_focus: crate::native_focus::NativeFocus,
    /// The page node that last had focus, to drop repeated focus events on it.
    last_page_focus: Option<usize>,
    /// The page node the browser starts Tab from: the focus, or where Tab was last sent from.
    tab_start: Option<usize>,
    /// The list whose option the focused field points at (aria-activedescendant), by name.
    option_list: Option<String>,
    /// The page's application region (ARIA role "application") the focus is in, by name:
    /// every key goes to the page there, Escape too.
    web_application: Option<String>,
    /// The page's focus is a text field or a combo box (one is typed in, its caret read).
    page_focus_is_field: bool,
    /// The last native focus outside menus, re-announced when menus close.
    last_native_focus: Option<FocusEvent>,
    /// The item a field's list last selected, since the focus last moved: the same one again
    /// says its name alone.
    last_selected: Option<String>,
    /// The character the last key typed, if it typed one: a console's notification of just
    /// that character is its echo of the key.
    last_typed: Option<char>,
    /// An edit key's result was read in a field its application also announces (a console):
    /// its announcements of the same change, until the next key, are not said.
    edit_read_in_console: bool,
    /// The last key extended a selection: Shift with an arrow key, Home, End or a page key.
    last_key_selects: bool,
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
    last_press: Option<(Press, u64)>,
    /// User settings (JAWS defaults unless configured).
    pub settings: crate::settings::Settings,
    /// Caps Lock's state as toggled through the JAWS key, when the platform does not report it.
    caps_lock_on: bool,
    /// The Virtual Viewer, while it is open (it takes the keys).
    viewer: Option<crate::viewer::VirtualViewer>,
    /// A Virtual Viewer waiting for what it will show, read off the keyboard hook.
    pending_viewer: Option<PendingViewer>,
    /// Say Font waiting for the page elements that show the font (`true`: pressed twice,
    /// shown in the Virtual Viewer).
    pending_font: Option<bool>,
    /// Behavior for particular applications, offered every focus first ([`crate::apps`]).
    apps: Vec<Box<dyn crate::apps::AppModule>>,
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
            last_window_title: None,
            last_window_kind: None,
            native_focus: crate::native_focus::NativeFocus::default(),
            last_page_focus: None,
            tab_start: None,
            option_list: None,
            web_application: None,
            page_focus_is_field: false,
            last_native_focus: None,
            last_selected: None,
            last_typed: None,
            edit_read_in_console: false,
            last_key_selects: false,
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
            pending_font: None,
            apps: crate::apps::builtin(),
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

    /// The application module that owns `focus`, whose hooks apply to it.
    fn app_for(&self, focus: &FocusEvent) -> Option<&dyn crate::apps::AppModule> {
        self.apps
            .iter()
            .find(|app| app.owns(focus))
            .map(|app| app.as_ref())
    }

    /// The application module that owns the page in front, by the page's address.
    fn page_app(&self) -> Option<&dyn crate::apps::AppModule> {
        let url = self.cursor.buffer.document_url()?;
        self.apps
            .iter()
            .find(|app| app.owns_page(url))
            .map(|app| app.as_ref())
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
        self.last_typed = event.text;
        self.edit_read_in_console = false;
        self.last_key_selects = event.modifiers.shift
            && !event.modifiers.alt
            && matches!(
                event.key,
                Key::UpArrow
                    | Key::DownArrow
                    | Key::LeftArrow
                    | Key::RightArrow
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown
            );
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
            // A page's menu takes every key, Escape too, which closes it (JAWS in Google Docs'
            // menus, dev box, 2026-10-16); the menu's items are focus changes, not a field's
            // text.
            if self.menus.is_active() && self.app_mode == AppMode::VirtualBuffer {
                return KeyAction::PassThrough;
            }
            // In a page's application region every key is the page's (JAWS on the dev box:
            // Escape closes VS Code's command palette).
            if self.web_application.is_some() {
                // Typing and caret keys are read in its text fields only: on its buttons and
                // lists the page answers keys itself, and JAWS says nothing for them.
                if self.page_focus_is_field {
                    self.edit_key_feedback(event);
                }
                return KeyAction::PassThrough;
            }
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
            } else {
                // A field typing goes into though it is not an edit (Windows Terminal's text
                // area): typed characters are echoed as in any field, and the application's
                // own edit keys are read (its output arrives as notifications, but Escape's
                // cleared line does not).
                let (types_into, own_key) = self
                    .last_native_focus
                    .as_ref()
                    .and_then(|field| {
                        self.app_for(field)
                            .map(|app| (app.types_into(field), app.edit_key(field, event)))
                    })
                    .unwrap_or((false, None));
                if types_into {
                    use crate::editing::EditKey;
                    // Left, Right and Backspace read as in a field (JAWS: "c", "b"; the
                    // deleted "b"); Up and Down recall history, which the application
                    // announces itself. Ctrl+Left and Right are left out: the word read at
                    // the caret came out "blank" where JAWS said "ac" (dev box, 2026-10-17).
                    let caret_key = EditKey::of(event).filter(|k| {
                        matches!(
                            k,
                            EditKey::Character | EditKey::DeleteBackward | EditKey::DeleteForward
                        )
                    });
                    if let Some(key) = own_key.or(caret_key) {
                        self.expect_text_change(key);
                    } else {
                        if let Some(c) = event.text {
                            self.echo_typed(c);
                        }
                        // The field's known text kept current with no speech, so that the
                        // next key read finds its own change (Escape after Ctrl+Left read the
                        // line before it was cleared).
                        if let Some(actions) = &self.actions {
                            actions.read_at_caret(self.last_text.as_ref(), false);
                        }
                    }
                }
            }
            return KeyAction::PassThrough;
        }

        // 6. Tab and Shift+Tab belong to the page: the browser moves focus, and the focus
        //    event moves the virtual cursor and speaks [obs, 204]. Away from the focus, the
        //    virtual cursor is where the browser moves on from, as with JAWS (dev box).
        if event.key == Key::Tab && !event.modifiers.ctrl && !event.modifiers.alt {
            if self.tab_from_virtual_cursor(event.modifiers.shift) {
                return KeyAction::Consume;
            }
            return KeyAction::PassThrough;
        }

        // 7. In Virtual PC Cursor mode, check if keystroke translates to a command
        if let Some(command) = crate::command::command_for(event) {
            let res = self.execute(command);
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

    /// Tab (or Shift+Tab) with the virtual cursor outside the element the browser would start
    /// from: the page's next (or prior) tab stop after the cursor's line is focused, as NVDA's
    /// browse mode does. JAWS on the dev box (2026-10-16): with focus on a button and the
    /// virtual cursor moved past a link to a group's end, Tab went to the field after the
    /// group. A browser starts Tab from its focus whatever the page's caret, so the key alone
    /// cannot do it. In a dialog, or past the page's last stop, the key goes to the browser.
    fn tab_from_virtual_cursor(&mut self, backwards: bool) -> bool {
        if self.cursor.is_in_modal() {
            return false;
        }
        let buffer = &self.cursor.buffer;
        let Some(line) = buffer.line_at_offset(self.cursor.offset) else {
            return false;
        };
        let within_start = self
            .tab_start
            .and_then(|id| buffer.node(id))
            .is_some_and(|start| start.find_by_id(line.node_id).is_some());
        if within_start {
            return false;
        }
        let stop = if backwards {
            buffer.tab_stops.iter().rfind(|&&s| s < line.start_offset)
        } else {
            buffer.tab_stops.iter().find(|&&s| s > line.start_offset)
        };
        let Some(node) = stop
            .and_then(|&s| buffer.line_at_offset(s))
            .and_then(|l| buffer.node(l.node_id))
        else {
            return false;
        };
        let focused = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.focus(node));
        if !focused {
            return false;
        }
        let (id, is_edit) = (node.id, node.role == crate::aom::Role::Edit);
        self.tab_start = Some(id);
        // Focus already there brings no focus event: the element is said here, as JAWS
        // says it (GitHub's current README tab, dev box).
        if self.last_page_focus == Some(id) {
            if is_edit {
                self.forms_mode = true;
                self.cursor.forms_mode_active = true;
            }
            let from = self
                .cursor
                .buffer
                .line_at_offset(self.cursor.offset)
                .map(|l| l.node_id);
            let entered = from
                .map(|from| self.entered_containers(from, id))
                .unwrap_or_default();
            if let Some(speech) = self.cursor.focus_node(id) {
                self.speech.speak(&format!("{entered}{speech}"), true);
            }
        }
        true
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
            AccessibilityEvent::ViewerText(lines) => self.open_screen_viewer(lines),
            AccessibilityEvent::PageElements(elements) => self.say_page_font(elements),
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
