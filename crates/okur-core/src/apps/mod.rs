//! Behavior for particular applications, as JAWS keeps it in application scripts: kept here,
//! by application, so the general focus and reading code never branches on one program's
//! windows.
//!
//! Each module implements [`AppModule`]. The engine keeps them in a registry ([`builtin`]):
//! - every focus event is offered to the modules in order, before the engine's own handling,
//!   as JAWS runs an application's script before the default one;
//! - the other hooks go to the module that owns the focus ([`AppModule::owns`]);
//! - a web application's hooks go to the module that owns the page in front, by its address
//!   ([`AppModule::owns_page`]).
//!
//! A module may match what the platform calls its windows (`FocusEvent::platform`, `app`)
//! or a web application's address and DOM ids; general code does not.

pub mod web;
pub mod windows;

use crate::aom::{AomNode, TextFormatting};
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
    /// The last key extended a selection (Shift with an arrow key, Home, End or a page key).
    pub selecting: bool,
    /// The Alt key is held down (the task switcher stays up while it is).
    pub alt_held: bool,
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

    /// The rows of `text`, the screen `focus`'s application writes to as it runs (a console's),
    /// for Screen Echo to find what it wrote since; `None` where its screen is not read so.
    fn screen_rows(&self, _focus: &FocusEvent, _text: &str) -> Option<Vec<String>> {
        None
    }

    /// Whether arriving in one of the application's windows announces the window's title.
    fn names_window(&self, _focus: &FocusEvent) -> bool {
        true
    }

    /// What the announcement of a new window calls it, instead of its title.
    fn window_name(&self, _focus: &FocusEvent) -> Option<String> {
        None
    }

    /// Whether the announcement of a new window says the focused control after its name.
    fn names_control_with_window(&self, _focus: &FocusEvent) -> bool {
        true
    }

    /// Whether the application reads what `key` does in its `field` itself, so the engine
    /// says nothing for the key.
    fn reads_key(&self, _field: &FocusEvent, _key: &KeyEvent) -> bool {
        false
    }

    /// Whether typing goes into `field` though it is not an edit (a console's text area), so
    /// that typed characters are echoed there as in any text field.
    fn types_into(&self, _field: &FocusEvent) -> bool {
        false
    }

    /// An edit key of this application's own, where the field's text changes for a key that
    /// changes no other field's (a console's Escape clears its input line).
    fn edit_key(&self, _field: &FocusEvent, _key: &KeyEvent) -> Option<crate::editing::EditKey> {
        None
    }

    /// Whether Virtualize Window shows the focus's text, read fresh (a console's buffer).
    fn virtualizes_text(&self) -> bool {
        false
    }

    /// The Virtual Viewer's lines for the focus's text, and the line to open on, when the
    /// viewer shows that text.
    fn viewer_lines(
        &self,
        _focus: &FocusEvent,
        _text: &str,
        _caret: usize,
    ) -> Option<(Vec<String>, usize)> {
        None
    }

    /// What closing that Virtual Viewer reads.
    fn line_after_viewer(&self, _text: &str, _caret: usize) -> Option<String> {
        None
    }

    /// Whether the page at `url` is this module's web application: the page hooks below are
    /// asked of the module that owns the page in front.
    fn owns_page(&self, _url: &str) -> bool {
        false
    }

    /// For Say Font in the page, focused on the element with DOM id `focus_id`: the page
    /// elements, by DOM id, that show the formatting at the caret. For a page that draws its
    /// text itself (Google Docs' canvas), whose focus has no text to take a font from; empty
    /// where the focus's own text has the font.
    fn font_elements(&self, _focus_id: &str) -> &'static [&'static str] {
        &[]
    }

    /// The formatting at the caret, from those elements as they are now.
    fn font_from_elements(&self, _elements: &[AomNode]) -> Option<TextFormatting> {
        None
    }
}

/// The application modules Okur has, in the order focus events are offered to them.
pub fn builtin() -> Vec<Box<dyn AppModule>> {
    let mut modules = windows::modules();
    modules.extend(web::modules());
    modules
}
