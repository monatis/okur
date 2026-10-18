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
use crate::messages::msg;
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
#[cfg(test)]
mod web_app_tests;

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
    /// The last page focus was said as its line, without the table it is in: the next focus
    /// says the table as newly entered.
    focus_table_unsaid: bool,
    /// The page node the browser starts Tab from: the focus, or where Tab was last sent from.
    tab_start: Option<usize>,
    /// The list or grid item Enter was passed to in forms mode, until the next key: a focus
    /// that then lands elsewhere was put there by what the item opened.
    activated_item: Option<usize>,
    /// The page was asked to be captured again for a focus the buffer did not hold: the
    /// virtual cursor goes to the focus when the capture arrives.
    cursor_to_focus: bool,
    /// The page changed while forms mode was on: it is captured again when forms mode ends.
    page_stale: bool,
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
    /// The last key was Tab or Shift+Tab: the focus it moves is the user's.
    last_key_tabs: bool,
    /// The next key, with the modifiers held with it, goes to the application as it is
    /// (Pass Key Through).
    pass_next_key: bool,
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
    /// Okur's own windows (`okur-ui`); None when headless.
    ui: Option<Box<dyn crate::ui::UiHost>>,
    /// Where changed settings are kept; None when headless.
    settings_store: Option<Box<dyn crate::settings::SettingsStore>>,
    /// The settings dialog, while it is open.
    settings_dialog: Option<crate::ui::settings::SettingsDialog>,
    /// The list dialog open (the Links List, the Heading List).
    list_dialog: Option<crate::ui::lists::ListDialog>,
    /// Until when the focus returning from one of Okur's dialogs is not announced.
    quiet_focus_until: Option<std::time::Instant>,
    /// The field Select a Form Field gave the focus to: its focus is said even while the
    /// focus returns quietly from the dialog, as JAWS says it [obs, 225/s10].
    dialog_focus_target: Option<usize>,
    /// Dialogs opened so far: each gets the next number.
    dialog_count: u32,
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
            focus_table_unsaid: false,
            tab_start: None,
            activated_item: None,
            cursor_to_focus: false,
            page_stale: false,
            option_list: None,
            web_application: None,
            page_focus_is_field: false,
            last_native_focus: None,
            last_selected: None,
            last_typed: None,
            edit_read_in_console: false,
            last_key_selects: false,
            last_key_tabs: false,
            pass_next_key: false,
            menus: crate::native_focus::Menus::default(),
            pending_edit: None,
            forms_mode_by_enter: false,
            pending_page: None,
            last_text: None,
            actions: None,
            clock: None,
            ui: None,
            settings_store: None,
            settings_dialog: None,
            list_dialog: None,
            quiet_focus_until: None,
            dialog_focus_target: None,
            dialog_count: 0,
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
        let action = self.handle_key(event);
        self.refresh_stale_page();
        action
    }

    fn handle_key(&mut self, event: &KeyEvent) -> KeyAction {
        if !event.is_down {
            return KeyAction::PassThrough;
        }
        // Which held keys act as the JAWS key depends on the keyboard settings.
        // Pass Key Through: the next key goes to the application as it is, with the modifiers
        // held for it.
        if self.pass_next_key
            && !matches!(
                event.key,
                Key::Control | Key::Alt | Key::Shift | Key::Windows | Key::Insert | Key::CapsLock
            )
        {
            self.pass_next_key = false;
            return KeyAction::PassThrough;
        }
        let resolved = self.settings.keyboard.resolve(event);
        let event = &resolved;
        self.last_typed = event.text;
        self.edit_read_in_console = false;
        self.last_key_tabs = event.key == Key::Tab && !event.modifiers.ctrl && !event.modifiers.alt;
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

        // A key ends the quiet return from Okur's dialogs: what it does is said.
        self.quiet_focus_until = None;
        self.dialog_focus_target = None;

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
                        self.mark_viewer_window();
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Character(text) => {
                    if !text.is_empty() {
                        self.speech.speak_character(&text, true);
                    }
                    self.mark_viewer_window();
                    return KeyAction::Consume;
                }
                ViewerAction::Copy(text) => {
                    if let Some(actions) = &self.actions {
                        actions.copy_text(&text);
                    }
                    // [obs, 311/s11]
                    self.speech
                        .speak(msg::clipboard_copied_selection().as_str(), true);
                    return KeyAction::Consume;
                }
                ViewerAction::Close => {
                    self.viewer = None;
                    self.close_viewer_window();
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

        // Who takes the keys, after the JAWS keys and the Virtual Viewer: an open menu, then
        // forms mode (a page's field or application region), then the focused control outside
        // a document, and last the virtual PC cursor in a page. JAWS reads a page with the
        // virtual cursor, and anything outside it (the browser's address bar, its menus) with
        // the PC cursor.
        //
        // A menu takes every key, Escape too, which closes it, whatever cursor was active
        // before it: a page's ARIA menu (Google Docs', in forms mode), a WinUI context menu,
        // or Chrome's app menu over a page, which opens with no item focused: the Down that
        // focuses its first item went to the virtual cursor, and the menu never had a focus
        // (dev box, 2026-10-17).
        // Without Auto Forms Mode a page's menu leaves the arrows to the virtual cursor, as
        // JAWS did on old-pc (218: Tab onto an ARIA menu bar, then Down read the bar's next
        // items as lines).
        let page_menu_in_browse = self.menus.is_page() && !self.settings.auto_forms_mode;
        if self.menus.in_menu_mode() && !page_menu_in_browse {
            return KeyAction::PassThrough;
        }

        // Enter on a list or grid item in forms mode may open something (a chat from its
        // list): the next focus elsewhere is then the page's, not the user's.
        self.activated_item = (self.forms_mode && event.key == Key::Enter)
            .then_some(self.last_page_focus)
            .flatten()
            .filter(|&id| self.in_widget_item(id));

        // 3. Handle Forms Mode
        if self.forms_mode {
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
                self.speech.speak(msg::cursor_virtual_pc().as_str(), true);
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
                self.speech.speak(msg::cursor_virtual_pc().as_str(), true);
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
        // The controls that can take the focus (a capture can say one cannot: WhatsApp's
        // message times are buttons nothing focuses), and the other elements that can
        // (WhatsApp's messages), as JAWS goes to them.
        let controls = buffer.tab_stops.iter().filter_map(|&start| {
            let node = buffer.node(buffer.line_at_offset(start)?.node_id)?;
            (node.states.focusable != Some(false)).then_some((start, node.id))
        });
        let others = buffer.focus_stops.iter().copied();
        let stop = if backwards {
            controls
                .filter(|&(s, _)| s < line.start_offset)
                .chain(others.filter(|&(s, _)| s < line.start_offset))
                .max_by_key(|&(s, _)| s)
        } else {
            controls
                .filter(|&(s, _)| s > line.start_offset)
                .chain(others.filter(|&(s, _)| s > line.start_offset))
                .min_by_key(|&(s, _)| s)
        };
        let Some(node) = stop.and_then(|(_, id)| buffer.node(id)) else {
            return false;
        };
        let focused = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.focus(node));
        tracing::debug!(
            backwards,
            from = line.node_id,
            to = node.id,
            role = ?node.role,
            name = node.name.as_deref().unwrap_or_default(),
            focused,
            "tab from the virtual cursor"
        );
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
            AccessibilityEvent::LiveRegion(_)
                | AccessibilityEvent::DocumentLoaded { .. }
                | AccessibilityEvent::PageChanged
                | AccessibilityEvent::PagePartLoaded(_)
        ) {
            self.apply_pending_page();
        }
        let spoken = match event {
            // Back from one of Okur's dialogs, the focus returns to where it was and is taken
            // in silently, as JAWS returns from its own Links List (dev box, 2026-10-17).
            AccessibilityEvent::Focus(focus)
                if self.quiet_return() && !self.reaches_dialog_focus_target(focus) =>
            {
                let speech = std::mem::replace(
                    &mut self.speech,
                    Box::new(crate::speech::MockSpeechOutput::new()),
                );
                self.handle_focus_event(focus);
                self.speech = speech;
                None
            }
            AccessibilityEvent::Focus(focus) => self.handle_focus_event(focus),
            AccessibilityEvent::Menu(menu) => self.handle_menu_event(*menu),
            AccessibilityEvent::Change { kind, control } => self.handle_change(*kind, control),
            AccessibilityEvent::TextContext(context) => self.handle_text_context(context),
            AccessibilityEvent::Selected(item) => self.handle_selected(item),
            AccessibilityEvent::LiveRegion(live) => self.handle_live_region_event(live),
            AccessibilityEvent::ViewerText(lines) => self.open_screen_viewer(lines),
            AccessibilityEvent::PageElements(elements) => self.say_page_font(elements),
            AccessibilityEvent::Ui(message) => self.handle_ui_event(message),
            AccessibilityEvent::PageChanged => {
                self.page_changed();
                None
            }
            AccessibilityEvent::PagePartLoaded(part) => {
                self.page_part_loaded(part);
                None
            }
            AccessibilityEvent::WindowRenamed(title) => {
                self.window_renamed(title);
                None
            }
            AccessibilityEvent::DocumentLoaded { title, root } => {
                self.handle_document_loaded(title, VirtualBuffer::from_aom(root))
            }
        };
        self.refresh_stale_page();
        spoken
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

    /// Gives the engine its own windows (`okur-ui`).
    pub fn set_ui(&mut self, ui: Box<dyn crate::ui::UiHost>) {
        self.ui = Some(ui);
    }

    /// Gives the engine where changed settings are kept.
    pub fn set_settings_store(&mut self, store: Box<dyn crate::settings::SettingsStore>) {
        self.settings_store = Some(store);
    }

    /// Makes `settings` Okur's, now: the voice included, and kept in the settings store.
    pub fn apply_settings(&mut self, settings: crate::settings::Settings) {
        if settings.voice != self.settings.voice {
            self.speech.configure(&settings.voice);
        }
        if settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            self.close_viewer_window();
        }
        if let Some(store) = &self.settings_store {
            store.save(&settings);
        }
        self.settings = settings;
    }

    /// Opens the settings dialog, or brings it to the front. Okur speaks nothing itself: it
    /// reads the dialog when the dialog takes the focus, as any application's.
    pub(crate) fn open_settings(&mut self) -> Option<String> {
        let Some(ui) = &self.ui else {
            let speech = msg::settings_dialog_unavailable().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        };
        let dialog = match self.settings_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                crate::ui::settings::SettingsDialog::new(
                    self.dialog_count,
                    &self.settings,
                    self.speech.voices(),
                )
            }
        };
        ui.send(crate::ui::ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.settings_dialog = Some(dialog);
        None
    }

    /// What the user did to one of Okur's windows.
    fn handle_ui_event(&mut self, message: &crate::ui::FromUi) -> Option<String> {
        use crate::ui::settings::Outcome;
        match message {
            // The viewer's window took the focus or was closed: the viewer ends silently, and
            // the focus says where the user is (JAWS on the dev box, 2026-10-17: Alt+Tab to
            // its viewer's window ended the viewer, and the page took the focus back).
            crate::ui::FromUi::ViewerDismissed => {
                self.viewer = None;
                None
            }
            _ if self
                .list_dialog
                .as_ref()
                .is_some_and(|d| Some(d.id) == dialog_of(message)) =>
            {
                self.handle_list_event(message)
            }
            _ => {
                let outcome = self.settings_dialog.as_mut()?.handle(message);
                match outcome {
                    Outcome::Nothing => {}
                    Outcome::Send(messages) => {
                        if let Some(ui) = &self.ui {
                            messages.into_iter().for_each(|m| ui.send(m));
                        }
                    }
                    Outcome::Apply { settings, close } => {
                        self.apply_settings(settings);
                        if close {
                            if let Some(dialog) = self.settings_dialog.take() {
                                if let Some(ui) = &self.ui {
                                    ui.send(crate::ui::ToUi::DialogClose { id: dialog.id });
                                }
                            }
                            self.returning_from_dialog();
                        }
                    }
                    Outcome::Closed => {
                        self.settings_dialog = None;
                        self.returning_from_dialog();
                    }
                }
                None
            }
        }
    }

    /// Whether the focus is still returning from one of Okur's dialogs.
    fn quiet_return(&self) -> bool {
        self.quiet_focus_until
            .is_some_and(|until| std::time::Instant::now() < until)
    }

    /// Whether `focus` is the field a dialog gave the focus to. Its arrival ends the quiet
    /// return.
    fn reaches_dialog_focus_target(&mut self, focus: &crate::event::FocusEvent) -> bool {
        let reached = self.dialog_focus_target.is_some()
            && self.page_node_for(focus) == self.dialog_focus_target;
        if reached {
            self.dialog_focus_target = None;
            self.quiet_focus_until = None;
        }
        reached
    }

    /// The page as JAWS names it when the focus comes back to it from one of its dialogs:
    /// "Web App Patterns document" [obs, 221/s2, 227/s11].
    fn page_on_return(&self) -> Option<String> {
        self.cursor.buffer.document_title().map(|title| {
            format!(
                "{title}{}",
                crate::linearize::role_marker(msg::role_document().as_str())
            )
        })
    }

    /// Starts the quiet return after one of Okur's dialogs closed: the window it came from
    /// takes the focus back within this.
    fn returning_from_dialog(&mut self) {
        const RETURN: std::time::Duration = std::time::Duration::from_millis(800);
        self.quiet_focus_until = Some(std::time::Instant::now() + RETURN);
    }

    /// Opens one of JAWS's list dialogs on the page's elements of its kind; with none, says
    /// so, as JAWS does.
    pub(crate) fn open_list(&mut self, kind: crate::ui::lists::ListKind) -> Option<String> {
        use crate::ui::lists::{ListDialog, ListKind};
        let id = self.dialog_count + 1;
        let (dialog, none) = match kind {
            ListKind::Links => {
                let links = self.cursor.buffer.get_links_list();
                (
                    (!links.is_empty()).then(|| ListDialog::links(id, &links)),
                    msg::links_list_none().to_string(),
                )
            }
            ListKind::Headings => {
                let headings = self.cursor.buffer.get_headings_list();
                (
                    (!headings.is_empty()).then(|| ListDialog::headings(id, &headings)),
                    msg::quicknav_none_headings().to_string(),
                )
            }
            // With none, JAWS says so and then the page [obs, 221/s2].
            ListKind::FormFields => {
                let fields = self.cursor.buffer.get_form_fields_list();
                let page = self.page_on_return().map(|page| format!("\n{page}"));
                (
                    (!fields.is_empty()).then(|| ListDialog::form_fields(id, &fields)),
                    format!(
                        "{}{}",
                        msg::form_fields_list_none(),
                        page.unwrap_or_default()
                    ),
                )
            }
        };
        let Some(dialog) = dialog else {
            self.speech.speak(&none, true);
            return Some(none);
        };
        let ui = self.ui.as_ref()?;
        self.dialog_count = id;
        ui.send(crate::ui::ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.list_dialog = Some(dialog);
        None
    }

    /// What the user did to the open list dialog.
    fn handle_list_event(&mut self, message: &crate::ui::FromUi) -> Option<String> {
        use crate::ui::lists::{ListKind, Outcome};
        let kind = self.list_dialog.as_ref()?.kind();
        let outcome = self.list_dialog.as_mut()?.handle(message);
        let close = |engine: &mut Self| {
            if let (Some(dialog), Some(ui)) = (engine.list_dialog.take(), &engine.ui) {
                ui.send(crate::ui::ToUi::DialogClose { id: dialog.id });
            }
            engine.returning_from_dialog();
        };
        match outcome {
            Outcome::Nothing => None,
            Outcome::Send(messages) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
                None
            }
            // JAWS 2025 after Move To Link (dev box, 2026-10-17): "same page link Reputation
            // for Quality Service | To activate press Enter."
            // A heading's line is read as the virtual cursor reads it, not as H says the
            // heading: "heading level 1 Web App Patterns" [obs, 224/s10].
            Outcome::MoveTo(entry) => {
                close(self);
                let said = self.cursor.jump_to_element_offset(entry.offset);
                let speech = match kind {
                    ListKind::Links => format!("{said}\n{}", msg::hint_activate_enter()),
                    ListKind::Headings | ListKind::FormFields => self.cursor.current_line(),
                };
                self.speech.speak(&speech, true);
                Some(speech)
            }
            Outcome::Activate(link) => {
                close(self);
                self.cursor.jump_to_element_offset(link.offset);
                let activated = self.execute(crate::command::Command::Enter);
                if !link.same_page {
                    return activated;
                }
                // A link within the page keeps the focus, which coming back from the dialog
                // is taken quietly: JAWS says its usage hint alone [obs, 222/s10].
                let hint = msg::hint_activate_enter().to_string();
                self.speech.speak(&hint, false);
                Some(match activated.filter(|s| !s.trim().is_empty()) {
                    Some(said) => format!("{said}\n{hint}"),
                    None => hint,
                })
            }
            // OK in Select a Form Field: JAWS names the page, reads the field as the virtual
            // cursor does, and says it again as it takes the focus ("Web App Patterns | Outline
            // button menu | Outline button menu"); an edit then takes typing [obs, 225/s10,
            // 228/s9, s10].
            Outcome::Focus(field) => {
                close(self);
                self.cursor.jump_to_element_offset(field.offset);
                let speech = [
                    self.cursor.buffer.document_title().map(str::to_owned),
                    Some(self.cursor.current_line()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
                let focused = match (self.cursor.buffer.node(field.node_id), &self.actions) {
                    (Some(node), Some(actions)) => actions.focus(node),
                    _ => false,
                };
                if focused {
                    self.dialog_focus_target = Some(field.node_id);
                }
                self.speech.speak(&speech, true);
                Some(speech)
            }
            // Select a Form Field gives the page back named; the Links List quietly [obs,
            // 227/s11, 223/s3].
            Outcome::Closed => {
                self.list_dialog = None;
                self.returning_from_dialog();
                let page = (kind == ListKind::FormFields)
                    .then(|| self.page_on_return())
                    .flatten()?;
                self.speech.speak(&page, true);
                Some(page)
            }
        }
    }

    /// Shows the open Virtual Viewer in its window, when the settings show it on screen.
    pub(crate) fn show_viewer_window(&self) {
        if self.settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            return;
        }
        if let (Some(ui), Some(viewer)) = (&self.ui, &self.viewer) {
            ui.send(crate::ui::ToUi::ViewerShow {
                title: msg::viewer_title().to_string(),
                lines: viewer.lines(),
                mark: viewer.mark(),
            });
        }
    }

    /// Moves the viewer window's mark to what the viewer read last.
    fn mark_viewer_window(&self) {
        if self.settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            return;
        }
        if let (Some(ui), Some(viewer)) = (&self.ui, &self.viewer) {
            ui.send(crate::ui::ToUi::ViewerMark {
                mark: viewer.mark(),
            });
        }
    }

    /// Closes the viewer's window, if one is showing.
    fn close_viewer_window(&self) {
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::ViewerClose);
        }
    }
}

/// The dialog an event from one of Okur's windows is about.
fn dialog_of(message: &crate::ui::FromUi) -> Option<u32> {
    use crate::ui::FromUi;
    match message {
        FromUi::DialogChanged { id, .. }
        | FromUi::DialogPressed { id, .. }
        | FromUi::DialogClosed { id } => Some(*id),
        FromUi::ViewerDismissed => None,
    }
}
