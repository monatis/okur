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
mod cursors;
mod find;
mod focus;
mod help;
mod information;
mod jump;
mod layer;
mod page;
mod quit;
mod reading;
mod routing;
mod say_all;
mod state;
#[cfg(test)]
mod tests;
mod text_feedback;
mod toggles;
mod tray;
#[cfg(test)]
mod web_app_tests;
mod window_list;

pub use state::AppMode;

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
    pub app_mode: AppMode,
    pub last_window_title: Option<String>,
    /// The active window's class, with `last_window_title` identifying the window.
    last_window_kind: Option<String>,
    /// Container of the last focused native control; entering a new one announces it.
    native_focus: crate::native_focus::NativeFocus,
    /// The page node that last had focus, to drop repeated focus events on it.
    last_page_focus: Option<usize>,
    /// The page node that had the focus as the last key went down.
    focus_at_key: Option<usize>,
    /// The last key routed the focus to the virtual cursor (Ctrl+Insert+Delete).
    routed: bool,
    /// The select box with the focus opened its list with the last key: its list is said as
    /// "expanded" [obs, 276/s7].
    select_opened: bool,
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
    /// The option of that list said last, which a repeated focus does not say again.
    option_said: Option<String>,
    /// The field an arrow put the focus on (Auto Forms Mode), already said: its focus event
    /// is not said again.
    arrow_focus: Option<usize>,
    /// The text last known in a page's fields, by node, kept as the focus leaves them: the
    /// buffer keeps the values the page had when it was captured, and an arrow back onto a
    /// field says what was typed in it [obs, 284/s6].
    field_texts: std::collections::HashMap<usize, String>,
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
    /// Items an edit combo's lists selected since the last key, with their positions: the one
    /// the field's new text names is its own [obs, 541/s3].
    combo_items: Vec<(String, usize)>,
    /// The character the last key typed, if it typed one: a console's notification of just
    /// that character is its echo of the key.
    last_typed: Option<char>,
    /// The word typed so far in a text field, for word echo.
    typed_word: String,
    /// An edit key's result was read in a field its application also announces (a console):
    /// its announcements of the same change, until the next key, are not said.
    edit_read_in_console: bool,
    /// The last key extended a selection: Shift with an arrow key, Home, End or a page key.
    last_key_selects: bool,
    /// The last key was Tab or Shift+Tab: the focus it moves is the user's.
    last_key_tabs: bool,
    /// The Alt key is down: pressed on its own and not yet released (Alt held through
    /// Tabs in the task switcher).
    alt_held: bool,
    /// The next key, with the modifiers held with it, goes to the application as it is
    /// (Pass Key Through).
    pass_next_key: bool,
    menus: crate::native_focus::Menus,
    /// The page re-read after a change, waiting for the change's events to be handled.
    pending_page: Option<VirtualBuffer>,
    /// Forms mode was just turned on by Enter (not by focus moving into a field).
    forms_mode_by_enter: bool,
    /// The focused control's access key, whatever the Access Key output: Say Focus says it.
    focus_access_key: Option<String>,
    /// The focused application's product name and version, as the last focus gave them
    /// (Ctrl+Insert+V).
    focus_app_version: Option<(String, String)>,
    /// The focused element's place on the screen: the PC cursor's.
    focus_bounds: Option<[i32; 4]>,
    /// Where the mouse pointer is, as the platform last reported it: the JAWS cursor's place.
    mouse: Option<(i32, i32)>,
    /// The cursor the reading keys act on.
    active_cursor: cursors::ActiveCursor,
    /// Forms mode was just turned on by one of JAWS's element lists giving a field the focus:
    /// the field's focus is followed by its line, as Say Line reads it [obs, 249/s3].
    forms_mode_by_list: bool,
    /// The control Select a Form Field gave the focus to, after saying the page itself as it
    /// closed: the control's focus comes within the page, without the title [obs, 225/s10,
    /// 228/s9].
    page_said_for: Option<usize>,
    /// The edit key whose effect is spoken when the field next reports its text.
    pending_edit: Option<crate::editing::EditKey>,
    /// Ctrl+Home (true) or Ctrl+End (false) pressed in an application's grid: the cell it
    /// reaches is followed by "top of file" or "Bottom of file" and the cell again.
    pending_file_edge: Option<bool>,
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
    /// The settings in their layers; `settings` is what they give where the focus is.
    settings_layers: crate::settings_layers::SettingsLayers,
    /// The application and the site `settings` were looked up for.
    settings_place: (Option<String>, Option<String>),
    /// The settings dialog, while it is open.
    settings_dialog: Option<crate::ui::settings::SettingsDialog>,
    /// The list dialog open (the Links List, the Heading List).
    list_dialog: Option<crate::ui::lists::ListDialog>,
    /// JAWS Find, while it is open.
    find_dialog: Option<crate::ui::find::FindDialog>,
    /// The Window List while it is open, and whether its windows were asked for and not come.
    window_list: Option<crate::ui::windows::WindowListDialog>,
    window_list_asked: bool,
    /// The system tray list while it is open, and whether its icons were asked for.
    tray_dialog: Option<crate::ui::tray::TrayDialog>,
    tray_asked: bool,
    /// Jump To Line while it is open, and where the last jump was made from (Shift+J).
    jump_dialog: Option<crate::ui::jump::JumpDialog>,
    jump_return: Option<usize>,
    /// Ctrl+R or F5 was pressed in a page: its next load is a new one, announced, not a
    /// capture of the page again [obs, 507/s1].
    reload_asked: bool,
    /// JAWS Find's check boxes and direction, kept between openings.
    find_options: crate::ui::find::FindOptions,
    /// JAWS Find's searches, the latest first, by site ("" for every site).
    find_histories: std::collections::HashMap<String, Vec<String>>,
    /// Until when the focus returning from one of Okur's dialogs is not announced.
    quiet_focus_until: Option<std::time::Instant>,
    /// The field Select a Form Field gave the focus to: its focus is said even while the
    /// focus returns quietly from the dialog, as JAWS says it [obs, 225/s10].
    dialog_focus_target: Option<usize>,
    /// Dialogs opened so far: each gets the next number.
    dialog_count: u32,
    /// The last JAWS-key command, when it was pressed and how many times quickly in a row, to
    /// tell a double or triple press.
    last_press: Option<(Press, u64, u32)>,
    /// A Say All in progress, whose cursor follows the synthesizer.
    say_all: Option<say_all::SayAll>,
    /// The mark the next part of a Say All is given.
    next_mark: u64,
    /// What the quick keys do (Insert+N), and whether they are suspended (Insert+Shift+Z).
    quick_keys: toggles::QuickKeys,
    quick_keys_suspended: bool,
    /// The virtual cursor turned off (Insert+Z): the page's keys are the browser's.
    virtual_cursor_off: bool,
    /// The voice rate's change for now (Ctrl+Alt+PageUp and PageDown), until Insert+Escape.
    rate_change: i32,
    /// Insert+Space was pressed: the next key is the layer's.
    layer_open: bool,
    /// The table layer (Insert+Space, T): the arrows move by cell until Escape.
    table_layer: bool,
    /// The page element Numpad Slash clicked at the virtual cursor: its focus is said as
    /// Enter's is, with the regions it is in [obs, 534/s2].
    clicked: Option<usize>,
    /// The radio button that Space or Numpad Slash checked while another of its group was
    /// checked: its focus is said without its position [obs, 205/s6, 534/s9].
    radio_switched: Option<usize>,
    /// The named groups the virtual cursor entered as an element list's Enter moved it to
    /// a control: said before the page as the focus comes back [obs, 260/s3, 246/s3].
    list_entered_groups: Vec<String>,
    /// Speech turned off with Insert+Space, S.
    speech_muted: bool,
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
    /// The question before Okur exits, while it is open (Insert+F4).
    quit_dialog: Option<crate::ui::quit::QuitDialog>,
    /// How the engine asks its host to end Okur; None when headless.
    exit: Option<quit::ExitRequest>,
}

impl ScreenReaderEngine {
    pub fn new(cursor: VirtualCursor, speech: Box<dyn SpeechOutput>) -> Self {
        Self {
            cursor,
            speech,
            forms_mode: false,
            app_mode: AppMode::VirtualBuffer,
            last_window_title: None,
            last_window_kind: None,
            native_focus: crate::native_focus::NativeFocus::default(),
            last_page_focus: None,
            focus_at_key: None,
            routed: false,
            select_opened: false,
            focus_table_unsaid: false,
            tab_start: None,
            activated_item: None,
            cursor_to_focus: false,
            page_stale: false,
            option_list: None,
            option_said: None,
            arrow_focus: None,
            field_texts: std::collections::HashMap::new(),
            web_application: None,
            page_focus_is_field: false,
            last_native_focus: None,
            last_selected: None,
            combo_items: Vec::new(),
            last_typed: None,
            typed_word: String::new(),
            edit_read_in_console: false,
            last_key_selects: false,
            last_key_tabs: false,
            alt_held: false,
            pass_next_key: false,
            menus: crate::native_focus::Menus::default(),
            pending_edit: None,
            pending_file_edge: None,
            forms_mode_by_enter: false,
            focus_access_key: None,
            focus_app_version: None,
            focus_bounds: None,
            mouse: None,
            active_cursor: cursors::ActiveCursor::Pc,
            forms_mode_by_list: false,
            page_said_for: None,
            pending_page: None,
            last_text: None,
            actions: None,
            clock: None,
            ui: None,
            settings_store: None,
            settings_layers: crate::settings_layers::SettingsLayers::default(),
            settings_place: (None, None),
            settings_dialog: None,
            list_dialog: None,
            find_dialog: None,
            window_list: None,
            window_list_asked: false,
            tray_dialog: None,
            tray_asked: false,
            jump_dialog: None,
            jump_return: None,
            reload_asked: false,
            find_options: crate::ui::find::FindOptions::default(),
            find_histories: std::collections::HashMap::new(),
            quiet_focus_until: None,
            dialog_focus_target: None,
            dialog_count: 0,
            last_press: None,
            say_all: None,
            next_mark: 0,
            quick_keys: toggles::QuickKeys::On,
            quick_keys_suspended: false,
            virtual_cursor_off: false,
            rate_change: 0,
            layer_open: false,
            table_layer: false,
            clicked: None,
            radio_switched: None,
            list_entered_groups: Vec::new(),
            speech_muted: false,
            settings: crate::settings::Settings::default(),
            caps_lock_on: false,
            viewer: None,
            pending_viewer: None,
            pending_font: None,
            apps: crate::apps::builtin(),
            last_live_text: std::collections::HashMap::new(),
            live_streams: std::collections::HashMap::new(),
            quit_dialog: None,
            exit: None,
        }
    }

    /// Replaces the active document buffer and resets the cursor.
    pub fn load_buffer(&mut self, buffer: VirtualBuffer) {
        crate::retire::retire(std::mem::replace(
            &mut self.cursor,
            VirtualCursor::new(buffer),
        ));
        self.cursor.move_to_start();
        self.cursor.verbosity = self.settings.verbosity.clone();
        self.forms_mode = false;
        self.app_mode = AppMode::VirtualBuffer;
        self.say_all = None;
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
        if event.key == Key::Alt {
            self.alt_held = event.is_down;
        }
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
        if self.app_mode == AppMode::VirtualBuffer {
            let m = &event.modifiers;
            let plain = !m.alt && !m.shift && !m.insert && !m.win;
            if plain
                && ((m.ctrl && event.key == Key::Alpha('R')) || (!m.ctrl && event.key == Key::F(5)))
            {
                self.reload_asked = true;
            }
        }
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
        // The hook waits for this: a slow key leaks to the application (Space on an edit held
        // 758 ms and toggled the focused check box, old-pc 2026-10-18), so its stages are
        // timed.
        let started = std::time::Instant::now();
        self.apply_pending_page();
        let page_applied = started.elapsed();

        // A key ends the quiet return from Okur's dialogs: what it does is said.
        self.quiet_focus_until = None;
        self.dialog_focus_target = None;
        self.page_said_for = None;
        self.focus_at_key = self.last_page_focus;
        self.routed = false;
        self.select_opened = false;
        self.combo_items.clear();

        // A Say All key steers the reading instead of ending it.
        if let Some(action) = self.say_all_key(event) {
            return action;
        }

        // 1. Any key press interrupts speech immediately (NVDA & JAWS standard behavior)
        self.speech.stop();
        // It ends a Say All, whose cursor stays on the line last heard.
        self.end_say_all();
        let stopped = started.elapsed();
        if stopped > std::time::Duration::from_millis(50) {
            tracing::warn!(
                key = ?event.key,
                page_ms = page_applied.as_millis(),
                stop_ms = (stopped - page_applied).as_millis(),
                "slow start of a key"
            );
        }

        // The key after Insert+Space is the layer's.
        if let Some(action) = self.layer_key(event) {
            return action;
        }
        if let Some(action) = self.table_layer_key(event) {
            return action;
        }

        // 2. Standalone Control tap is speech silence key
        if event.key == Key::Control && event.modifiers.is_empty() {
            return KeyAction::PassThrough;
        }
        // The sentence and paragraph keys and the hot key in an application or a page's field,
        // where the keys would otherwise go to it [obs, 504/s14, s15; 537/s1].
        if self.app_mode != AppMode::VirtualBuffer || self.forms_mode {
            use crate::command::Command;
            if let Some(
                command @ (Command::SaySentence
                | Command::SayNextSentence
                | Command::SayPriorSentence
                | Command::SayParagraph
                | Command::SayHotKey),
            ) = crate::command::command_for(event)
            {
                let _ = self.execute(command);
                return KeyAction::Consume;
            }
        }
        // The cursors, in any application [obs, 516]. Numpad Plus is the PC cursor from the
        // JAWS cursor or in an application; in a page it is the virtual PC cursor's key.
        {
            use crate::command::Command;
            use cursors::ActiveCursor;
            if let Some(
                command @ (Command::OkurCursor
                | Command::PcCursor
                | Command::RouteOkurToPc
                | Command::SayActiveCursor),
            ) = crate::command::command_for(event)
            {
                let in_page = self.app_mode == AppMode::VirtualBuffer;
                if command != Command::PcCursor
                    || self.active_cursor == ActiveCursor::Okur
                    || !in_page
                {
                    let _ = self.execute(command);
                    return KeyAction::Consume;
                }
            }
            // With the JAWS cursor active the arrows move it, not the focus: they stay out of
            // the application [obs, 516/s5].
            if self.active_cursor == ActiveCursor::Okur
                && self.app_mode != AppMode::VirtualBuffer
                && event.modifiers.is_empty()
                && matches!(
                    event.key,
                    Key::UpArrow | Key::DownArrow | Key::LeftArrow | Key::RightArrow
                )
            {
                return KeyAction::Consume;
            }
        }
        // The voice rate for now, in any application [obs, 512].
        if let Some(
            command @ (crate::command::Command::RateFaster | crate::command::Command::RateSlower),
        ) = crate::command::command_for(event)
        {
            let _ = self.execute(command);
            return KeyAction::Consume;
        }

        // The JAWS key (Insert, or CapsLock in the laptop layout) and every combination with
        // it are JAWS commands: they never reach the application, bound or not, in any mode.
        if self.settings.keyboard.is_okur_key(event.key) {
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
                    // [obs, 311/s11], worded for the level as the other copies are.
                    if let Some(copied) = self.copied_message() {
                        self.speech.speak(&copied, true);
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Close => {
                    self.viewer = None;
                    self.close_viewer_window();
                    self.speak_focus_again();
                    return KeyAction::Consume;
                }
                ViewerAction::Run(command) => {
                    self.viewer = None;
                    self.close_viewer_window();
                    let _ = self.execute(command);
                    return KeyAction::Consume;
                }
                ViewerAction::Ignore => {}
            }
        }
        if event.modifiers.insert {
            self.okur_key_command(event);
            return KeyAction::Consume;
        }
        // Numpad 5 is Say Character wherever the focus is, in an application's text field
        // too: JAWS keeps it from the application [obs, 401/s1-s3].
        if event.key == Key::Numpad5 && event.modifiers.is_empty() {
            self.execute_pressed(crate::command::Command::SayCharacter, event.time_ms);
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
        // Alt and a letter outside a page may open a menu at once [obs, 541/s1, 544/s1].
        if self.app_mode != AppMode::VirtualBuffer {
            let m = event.modifiers;
            self.menus.access_key(
                m.alt && !m.ctrl && !m.shift && !m.insert && matches!(event.key, Key::Alpha(_)),
            );
        }
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
            if let Some(action) = self.arrow_out_of_field(event) {
                return action;
            }
            // Numpad Plus leaves forms mode as Escape does (JAWS 2025 Settings Center help, Forms
            // Mode Options; Surf's Up, Forms).
            let numpad_plus = event.key == Key::NumpadPlus && event.modifiers.is_empty();
            if event.key == Key::Escape || numpad_plus {
                self.forms_mode = false;
                let _ = self.cursor.exit_forms_mode();
                // [obs, 205/s11]
                let said = self.virtual_pc_message();
                self.speech.speak(&said, true);
                return KeyAction::Consume;
            }
            // In forms mode, pass all typing and navigation through to the active control;
            // caret movement keys then read the text they moved over, typed keys are echoed.
            // A select (a combo box one does not type in) answers the arrows itself, and its
            // value change says the new choice; reading its text would answer first, with
            // nothing [obs, 205/s9, s10].
            if !self.page_focus_is_select() {
                self.edit_key_feedback(event);
            }
            return KeyAction::PassThrough;
        }

        // 4. Handle Escape when inside an expanded combobox or modal dialog
        if event.key == Key::Escape && event.modifiers.is_empty() {
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
            // Ctrl+Home and Ctrl+End in an application's grid are its top and bottom of file
            // [obs, 545/s6, s7].
            let file_key = event.modifiers.ctrl
                && !event.modifiers.alt
                && !event.modifiers.shift
                && matches!(event.key, Key::Home | Key::End);
            self.pending_file_edge = (file_key
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| f.role == crate::aom::Role::TableCell))
            .then_some(event.key == Key::Home);
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

        // With the virtual cursor off the page's keys are the browser's, and so are the quick
        // keys while they are off [obs, 298].
        if self.virtual_cursor_off || self.quick_key_passes(event) {
            return KeyAction::PassThrough;
        }
        // 7. In Virtual PC Cursor mode, check if keystroke translates to a command
        if let Some(command) = crate::command::command_for(event) {
            let started = std::time::Instant::now();
            let res = self.execute(command.clone());
            if started.elapsed() > std::time::Duration::from_millis(50) {
                tracing::warn!(?command, ms = started.elapsed().as_millis(), "slow command");
            }
            if res.is_some() {
                return KeyAction::Consume;
            }
            // Ctrl+C without a selection made with the virtual cursor is the browser's copy.
            if command == crate::command::Command::CopySelection {
                return KeyAction::PassThrough;
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
        // Both are in document order (a buffer invariant the corpus checks), so the nearest
        // one each way is found from the cursor's line on, not by walking every stop: that
        // took 35-48 ms on a page with 5,000 of them, in the keyboard hook (responsiveness
        // review, 2026-10-19). A control and another element at one place: the other element
        // going back, the control going forward, as before.
        let focusable = |start: usize| {
            let node = buffer.node(buffer.line_at_offset(start)?.node_id)?;
            (node.states.focusable != Some(false)).then_some((start, node.id))
        };
        let from = line.start_offset;
        let (controls, others) = (&buffer.tab_stops, &buffer.focus_stops);
        let stop = if backwards {
            let before = controls.partition_point(|&s| s < from);
            let control = controls
                .get(..before)
                .unwrap_or_default()
                .iter()
                .rev()
                .find_map(|&s| focusable(s));
            let other = others
                .get(..others.partition_point(|&(s, _)| s < from))
                .and_then(<[_]>::last)
                .copied();
            [control, other]
                .into_iter()
                .flatten()
                .max_by_key(|&(s, _)| s)
        } else {
            let after = controls.partition_point(|&s| s <= from);
            let control = controls
                .get(after..)
                .unwrap_or_default()
                .iter()
                .find_map(|&s| focusable(s));
            let other = others
                .get(others.partition_point(|&(s, _)| s <= from)..)
                .and_then(<[_]>::first)
                .copied();
            [control, other]
                .into_iter()
                .flatten()
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
                if self.settings.auto_forms_mode {
                    self.cursor.move_to_node_id(id);
                    // A field an arrow had left, with Auto Forms Mode: Shift+Tab back onto it
                    // turned forms mode on and JAWS said nothing [obs, 284/s11]; Tab onto it
                    // said it as a focus, with the text typed in it, "Full name Edit Required
                    // | ada" [obs, 284/s8]. One observation each.
                    if backwards {
                        return true;
                    }
                    let mut field = self.cursor.buffer.node(id).cloned();
                    let typed = self
                        .last_text
                        .as_ref()
                        .map(|c| c.text.clone())
                        .or_else(|| self.field_texts.get(&id).cloned());
                    if let (Some(field), Some(typed)) = (field.as_mut(), typed) {
                        field.value = Some(typed);
                    }
                    if let Some(field) = field {
                        let said = crate::speech::SpeechFormatter::control_focus(&field, |_| false)
                            .render();
                        self.speech.speak(&said, true);
                    }
                    return true;
                }
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
                | AccessibilityEvent::PagePartsLoaded(_)
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
            AccessibilityEvent::TopWindows(windows) => self.open_window_list(windows),
            AccessibilityEvent::TrayIcons(icons) => self.open_tray_list(icons),
            AccessibilityEvent::PageElements(elements) => self.say_page_font(elements),
            AccessibilityEvent::Ui(message) => self.handle_ui_event(message),
            AccessibilityEvent::PageChanged => {
                self.page_changed();
                None
            }
            AccessibilityEvent::PagePartsLoaded(parts) => {
                self.page_parts_loaded(parts);
                None
            }
            AccessibilityEvent::MouseMoved { x, y } => {
                self.mouse = Some((*x, *y));
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

    /// Makes `settings` Okur's for all applications, with no layer of their own: `okur run`'s
    /// settings, replay's reference machine.
    pub fn set_settings(&mut self, settings: crate::settings::Settings) {
        self.set_settings_layers(crate::settings_layers::SettingsLayers::with_global(
            &settings,
        ));
    }

    /// Gives the engine its settings in their layers (the user's saved files).
    pub fn set_settings_layers(&mut self, layers: crate::settings_layers::SettingsLayers) {
        self.settings_layers = layers;
        self.resolve_settings();
    }

    /// The settings in force from the layers, for the application and site the focus is in.
    fn resolve_settings(&mut self) {
        let (application, site) = &self.settings_place;
        let settings = self
            .settings_layers
            .resolve(application.as_deref(), site.as_deref());
        let voice_changed = settings.voice != self.settings.voice;
        // The cursor says a page control's focus itself, at the verbosity in force.
        self.cursor.verbosity = settings.verbosity.clone();
        self.settings = settings;
        if voice_changed {
            self.configure_voice();
        }
    }

    /// The focus is in `application` (as the platform names it), on a page of `site`: its
    /// settings are looked up anew when either changed.
    pub(crate) fn settings_place(&mut self, application: Option<&str>, site: Option<String>) {
        let place = (
            application.map(crate::settings_layers::application_id),
            site,
        );
        if place != self.settings_place {
            tracing::debug!(?place, "settings looked up for another place");
            self.settings_place = place;
            self.resolve_settings();
        }
    }

    /// Makes the settings dialog's changes Okur's, now, for all applications: the voice
    /// included, and kept in the settings store. Only what the dialog changed is kept: a level
    /// set for the session is not saved with it.
    pub fn apply_settings(&mut self, settings: crate::settings::Settings) {
        if settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            self.close_viewer_window();
        }
        let application = self.settings_place.0.clone();
        self.settings_layers.change(
            &crate::settings_layers::Scope::Global,
            &self.settings,
            &settings,
            application.as_deref(),
        );
        if let Some(store) = &self.settings_store {
            store.save(&self.settings_layers);
        }
        self.resolve_settings();
    }

    /// Set Punctuation Level (Insert+Shift+2): the next level, said by its name, until Okur
    /// exits. JAWS keeps it in neither its voice profile nor its JCF [obs, 400 exploration,
    /// 2026-10-19], so it goes in the session's layer and the settings store is not written.
    pub(crate) fn next_punctuation_level(&mut self) -> Option<String> {
        let level = self.settings.voice.punctuation.next();
        self.settings_layers.set_for_session(
            &["voice", "punctuation"],
            serde_json::to_value(level).unwrap_or_default(),
        );
        self.resolve_settings();
        let name = level.name();
        self.speech.speak(&name, true);
        Some(name)
    }

    /// Typing Echo (Insert+2): the next echo, for this session only, as Set Punctuation Level.
    pub(crate) fn next_typing_echo(&mut self) -> Option<String> {
        let echo = self.settings.typing_echo.next();
        self.settings_layers.set_for_session(
            &["typing_echo"],
            serde_json::to_value(echo).unwrap_or_default(),
        );
        self.resolve_settings();
        let name = echo.name();
        self.speech.speak(&name, true);
        Some(name)
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
            _ if self.is_find_event(message) => self.handle_find_event(message),
            _ if self.is_window_list_event(message) => self.handle_window_list_event(message),
            _ if self.is_tray_event(message) => self.handle_tray_event(message),
            _ if self.is_jump_event(message) => self.handle_jump_event(message),
            _ if self.is_quit_event(message) => self.handle_quit_event(message),
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

    /// Whether the page's focus is a select: a combo box one does not type in.
    fn page_focus_is_select(&self) -> bool {
        self.last_page_focus
            .and_then(|id| self.cursor.buffer.node(id))
            .is_some_and(|n| {
                n.role == crate::aom::Role::ComboBox
                    && n.attributes.get("editable").is_none_or(|e| e != "true")
            })
    }

    /// Whether the focus is still returning from one of Okur's dialogs.
    fn quiet_return(&self) -> bool {
        self.quiet_focus_until
            .is_some_and(|until| std::time::Instant::now() < until)
    }

    /// Whether `focus` is the field a dialog gave the focus to: said once, while the rest of
    /// the return (the page's own focus, Chrome's repeats of the field's) stays quiet.
    fn reaches_dialog_focus_target(&mut self, focus: &crate::event::FocusEvent) -> bool {
        let Some(target) = self.dialog_focus_target else {
            return false;
        };
        let node = self.page_node_for(focus);
        let reached = node == Some(target);
        tracing::debug!(
            target,
            ?node,
            reached,
            "focus while returning from a dialog"
        );
        if reached {
            self.dialog_focus_target = None;
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
            ListKind::Features => (Some(ListDialog::features(id)), String::new()),
            ListKind::Regions => {
                let regions = crate::element_lists::regions(&self.cursor.buffer);
                (
                    (!regions.is_empty()).then(|| ListDialog::regions(id, &regions)),
                    msg::regions_list_none().to_string(),
                )
            }
            ListKind::Elements(elements) => {
                let items = elements.items(&self.cursor.buffer);
                (
                    (!items.is_empty()).then(|| ListDialog::elements(id, elements, &items)),
                    elements.none().to_string(),
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
        // The keys are the dialog's now, before its window reports the focus: a Down the
        // window has yet to take must not move the virtual cursor [obs, 242/s2].
        self.app_mode = AppMode::NativeControl;
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
            engine.app_mode = AppMode::VirtualBuffer;
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
                    // Select a Table enters the table, as T does [obs, 257/s3].
                    ListKind::Elements(crate::element_lists::ElementList::Tables) => self
                        .cursor
                        .enter_table_at(entry.offset)
                        .unwrap_or_else(|| self.cursor.current_line()),
                    // Move To region start is silent; Say Line then reads the region's start
                    // [obs, 256/s3, s4].
                    ListKind::Regions => {
                        self.cursor.jump_to_element_offset(entry.offset);
                        return None;
                    }
                    // The controls list moves to a combo box as C does, its label said with it:
                    // "Home branch Combo box collapsed Northside" [obs, 248/s3].
                    ListKind::Elements(crate::element_lists::ElementList::Controls) => {
                        self.cursor.control_speech_here()
                    }
                    ListKind::Headings
                    | ListKind::FormFields
                    | ListKind::Features
                    | ListKind::Elements(_) => self.cursor.current_line(),
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
                // The named groups around the control that the virtual cursor was outside of.
                let from = self.cursor.offset;
                self.list_entered_groups = self
                    .cursor
                    .buffer
                    .ancestors(field.node_id)
                    .into_iter()
                    .rev()
                    .filter(|a| {
                        matches!(a.role, crate::aom::Role::Group | crate::aom::Role::TabList)
                    })
                    .filter_map(|a| {
                        let name = a.name.as_deref().map(str::trim).filter(|n| !n.is_empty())?;
                        let span = self.cursor.buffer.bounded.iter().find(|&&(s, e)| {
                            self.cursor
                                .buffer
                                .line_at_offset(s)
                                .is_some_and(|l| l.node_id == a.id)
                                && e >= s
                        })?;
                        (from < span.0 || from > span.1).then(|| name.to_owned())
                    })
                    .collect();
                self.cursor.jump_to_element_offset(field.offset);
                // JAWS's element lists leave the speech to the focus: the page and its regions
                // as the focus enters them, then the control [obs, 247/s3, 250/s3].
                let element_list = matches!(kind, ListKind::Elements(_));
                let node = self.cursor.buffer.node(field.node_id);
                let typed_in = node.is_some_and(|n| {
                    matches!(n.role, crate::aom::Role::Edit | crate::aom::Role::ComboBox)
                });
                let edit = node.is_some_and(|n| n.role == crate::aom::Role::Edit);
                let focused = match (node, &self.actions) {
                    (Some(node), Some(actions)) => actions.focus(node),
                    _ => false,
                };
                tracing::debug!(node = field.node_id, focused, "form field list: focus");
                if focused {
                    self.dialog_focus_target = Some(field.node_id);
                    // A field one types in is then in forms mode, as after Enter on it, whatever
                    // Auto Forms Mode says [obs, 228/s10]; JAWS's documentation: "Forms mode is
                    // automatically enabled" (Surf's Up, Using Forms with JAWS). Without Enter's
                    // "Type in text." [obs, 249/s3, 228/s9].
                    if typed_in {
                        self.forms_mode = true;
                        self.cursor.forms_mode_active = true;
                        self.forms_mode_by_list = element_list;
                    }
                    if !element_list {
                        self.page_said_for = Some(field.node_id);
                    }
                }
                if element_list {
                    // The page's own focus on the way to the control is not the user's: the
                    // control's focus enters the page.
                    self.last_page_focus = None;
                    return None;
                }
                // Select a Form Field says the page and the line it left the virtual cursor on,
                // then an edit's line in forms mode, before the field's own focus [obs, 228/s9].
                let speech = [
                    self.cursor.buffer.document_title().map(str::to_owned),
                    Some(self.cursor.current_line()),
                    (focused && edit)
                        .then(|| self.field_forms_line(field.node_id))
                        .flatten(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
                self.speech.speak(&speech, true);
                Some(speech)
            }
            // JAWS opens the chosen list in place of Virtual HTML Features [obs, 226/s3]: the
            // new dialog is shown before this one goes, so the focus moves from one to the
            // other. A list Okur has not built yet is said so.
            Outcome::Open(feature) => {
                let features = self.list_dialog.take();
                let opened = match feature.list() {
                    Some(kind) => self.open_list(kind),
                    None => {
                        let speech = msg::html_feature_not_yet(feature.name().as_str()).to_string();
                        self.speech.speak(&speech, true);
                        Some(speech)
                    }
                };
                if let (Some(features), Some(ui)) = (features, &self.ui) {
                    ui.send(crate::ui::ToUi::DialogClose { id: features.id });
                }
                if self.list_dialog.is_none() {
                    self.returning_from_dialog();
                }
                opened
            }
            // Select a Form Field gives the page back named; the Links List quietly [obs,
            // 227/s11, 223/s3].
            Outcome::Closed => {
                self.list_dialog = None;
                self.app_mode = AppMode::VirtualBuffer;
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
