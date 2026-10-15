//! Behavior for particular applications, as JAWS keeps it in application scripts: kept here,
//! by application, so the general focus and reading code never branches on one program's
//! windows.
//!
//! Each module implements [`AppModule`]. The engine keeps them in a registry ([`builtin`]):
//! - every focus event is offered to the modules in order, before the engine's own handling,
//!   as JAWS runs an application's script before the default one;
//! - the other hooks go to the module that owns the focus ([`AppModule::owns`]).
//!
//! A module may match what the platform calls its windows (`FocusEvent::platform`, `app`);
//! general code does not.

pub mod windows;

use crate::engine::AppMode;
use crate::event::FocusEvent;
use crate::input::KeyEvent;

/// What a module does with a focus event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusResponse {
    /// Not the module's: the next module, then the engine, handle it.
    Pass,
    /// The module's, and nothing is said.
    Silent,
    /// The module's: this is said.
    Speak(crate::announcement::Announcement),
}

/// The engine's state a module reads and changes while it handles a focus event.
pub struct AppContext<'a> {
    /// Who has the focus: the virtual cursor, a native control, or an application's mode.
    pub app_mode: &'a mut AppMode,
    /// The active window's title (Insert+T) and kind, which tell the next window apart.
    pub window_title: &'a mut Option<String>,
    pub window_kind: &'a mut Option<String>,
}

impl AppContext<'_> {
    /// Records the focus's top-level window as the active one.
    pub fn note_window(&mut self, focus: &FocusEvent) {
        if let Some(window) = &focus.window_title {
            *self.window_title = Some(window.clone());
            *self.window_kind = focus.platform.top_window_class.clone();
        }
    }
}

/// Behavior for one application. Every hook has a default that leaves the engine's own
/// behavior alone.
pub trait AppModule: Send {
    /// Whether `focus` is in this module's application: the hooks below that take a focus
    /// are asked of the module that owns it.
    fn owns(&self, _focus: &FocusEvent) -> bool {
        false
    }

    /// Handles a focus event before the engine does.
    fn focus(&mut self, _focus: &FocusEvent, _cx: &mut AppContext<'_>) -> FocusResponse {
        FocusResponse::Pass
    }

    /// Whether arriving in one of the application's windows announces the window's title.
    fn names_window(&self, _focus: &FocusEvent) -> bool {
        true
    }

    /// What the announcement of a new window calls it, instead of its title.
    fn window_name(&self, _focus: &FocusEvent) -> Option<String> {
        None
    }

    /// Whether the application reads what `key` does in its `field` itself, so the engine
    /// says nothing for the key.
    fn reads_key(&self, _field: &FocusEvent, _key: &KeyEvent) -> bool {
        false
    }

    /// Whether Virtualize Window shows the focus's text, read fresh (a console's buffer).
    fn virtualizes_text(&self) -> bool {
        false
    }

    /// The Virtual Viewer's lines for the focus's text, and the line to open on, when the
    /// viewer shows that text.
    fn viewer_lines(&self, _text: &str, _caret: usize) -> Option<(Vec<String>, usize)> {
        None
    }

    /// What closing that Virtual Viewer reads.
    fn line_after_viewer(&self, _text: &str, _caret: usize) -> Option<String> {
        None
    }
}

/// The application modules Okur has, in the order focus events are offered to them.
pub fn builtin() -> Vec<Box<dyn AppModule>> {
    windows::modules()
}
