//! The Linux driver's events as the engine's (docs/plans/2026-11-08-structure-for-three-platforms.md,
//! step 1): what okur-linux's event loop decides, made from what it read for each AT-SPI event (an
//! [`AtspiRecord`]), with no call to Linux, so it is tested on every OS and a live run's records
//! can be replayed as Windows' traces are. The driver asks the [`Converter`] what to read
//! ([`Converter::is_twin`], [`Converter::wants_page`], [`Converter::wants_native`],
//! [`Converter::wants_reread`]), reads it, and hands it the record ([`Converter::convert_read`]).
//! A replay converts each record as [`EventModel::convert`]: the twin passed over, then the read.

use okur_core::event::{AccessibilityEvent, ChangeKind, FocusEvent, TextContext};
use okur_model::event_model::EventModel;
use serde::{Deserialize, Serialize};
use xa11y_core::{ElementData, EventKind, Role};

use crate::focus::{
    focus_event, in_active_window, is_menu_item_role, menu_in, menu_item_focus, native_focus,
};
use crate::focus_track::{is_control_change, is_inside_change, FocusTrack, Reread};
use crate::page::{to_aom_from, AtspiNode};
use okur_core::event::{MenuEvent, PopupKind};

/// A control outside a page as the driver read it: its subtree, and its ancestors' from its
/// parent up to its window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NativeRead {
    pub focus: AtspiNode,
    pub ancestors: Vec<AtspiNode>,
    /// How long the read took, in milliseconds: what a focus costs on the bus.
    #[serde(default)]
    pub read_ms: u64,
}

/// A page read for the engine: its document's key, the base its objects are numbered from, its
/// tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageRead {
    pub document: String,
    pub base: i32,
    pub root: AtspiNode,
    /// How long the read took, in milliseconds: what the event thread waited on.
    #[serde(default)]
    pub read_ms: u64,
}

/// A text field read at its caret, as the engine asked after a key (`read_at_caret`): the
/// field's object, by its key, and its text, caret and selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextRead {
    pub object: String,
    pub context: TextContext,
    /// How long the read took, in milliseconds, from the ask: the caret's wait included.
    #[serde(default)]
    pub read_ms: u64,
}

/// A live region read again as it changed: its page's document, by its key, and the region's
/// subtree, its objects with the ids they have in the page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiveRead {
    pub document: String,
    pub root: AtspiNode,
    /// The read was cut at its object limit: the region's end, its newest content, is missing,
    /// and the page is captured again instead (the code review of 2026-11-09, second pass, 5).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cut: bool,
    /// How long the read took, in milliseconds.
    #[serde(default)]
    pub read_ms: u64,
}

/// What the driver read for one AT-SPI event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtspiRecord {
    /// When the event came, in milliseconds since the Unix epoch (the lab runner's clock).
    pub at_ms: u64,
    pub kind: EventKind,
    /// The application, by its AT-SPI name.
    pub app: String,
    /// The event's object, as the event's snapshot has it.
    pub target: Option<ElementData>,
    /// The page's document the target is in, by its key, where it is in one.
    pub document: Option<String>,
    /// The pages read: the focus's, new to the engine, and a page that finished loading.
    pub pages: Vec<PageRead>,
    /// The target read with what surrounds it, a focus or a control's change outside a page.
    pub native: Option<NativeRead>,
    /// The focused control read again, as something within it changed.
    pub reread: Option<NativeRead>,
    /// A text field read at its caret, asked for by the engine: a record of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextRead>,
    /// A live region read again as it changed: a record of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live: Option<LiveRead>,
    /// When the engine was given the record, in milliseconds since the Unix epoch, as the trace
    /// stamps it: a record held for a page, and a page, come later than their events, and a
    /// replay gives them to the engine in this order, the keys among them (the code review of
    /// 2026-11-09, finding 6). None in a record not yet given, and in the traces before it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivered_ms: Option<u64>,
}

impl AtspiRecord {
    /// When the engine was given it: its stamp, else (a trace before the stamps) its event's
    /// time.
    pub fn handed_ms(&self) -> u64 {
        self.delivered_ms.unwrap_or(self.at_ms)
    }
}

impl AtspiRecord {
    /// A page read off the event thread, given to the engine when it is read: a record of its
    /// own, a load with no object (the code review of 2026-11-08, finding 6).
    pub fn page(at_ms: u64, page: PageRead) -> Self {
        Self {
            pages: vec![page],
            ..Self::bare(at_ms, EventKind::DocumentLoaded, String::new(), None)
        }
    }

    /// A text field read at its caret, given to the engine as it is read: a record of its own.
    pub fn text(at_ms: u64, read: TextRead) -> Self {
        Self {
            text: Some(read),
            ..Self::bare(at_ms, EventKind::TextChanged, String::new(), None)
        }
    }

    /// A live region read again as it changed, given to the engine in its turn: a record of its
    /// own.
    pub fn live(at_ms: u64, read: LiveRead) -> Self {
        Self {
            live: Some(read),
            ..Self::bare(at_ms, EventKind::StructureChanged, String::new(), None)
        }
    }

    /// A record of an event for which nothing was read.
    pub fn bare(at_ms: u64, kind: EventKind, app: String, target: Option<ElementData>) -> Self {
        Self {
            at_ms,
            kind,
            app,
            target,
            document: None,
            pages: Vec::new(),
            native: None,
            reread: None,
            text: None,
            live: None,
            delivered_ms: None,
        }
    }
}

/// An object's key, its bus name and path, as an element's raw data names them.
pub fn object_key(element: &ElementData) -> Option<String> {
    let raw = |key: &str| element.raw.get(key).and_then(|v| v.as_str());
    Some(format!("{}{}", raw("bus_name")?, raw("object_path")?))
}

/// A value as a number says it: whole without its fraction ("40"), else as it is ("86.0274").
/// Chromium keeps a range's values as single-precision floats, which AT-SPI widens: 86.0274 comes
/// as 86.02739715576172, said as the float it was (the code review of 2026-11-09, third pass,
/// finding 4). None for a value that is no number.
fn number(value: f64) -> Option<String> {
    if !value.is_finite() {
        return None;
    }
    let value = value as f32;
    // Not "-0".
    let value = if value == 0.0 { 0.0 } else { value };
    Some(if value.fract() == 0.0 && value.abs() < 1e7 {
        format!("{value:.0}")
    } else {
        value.to_string()
    })
}

/// Whether the event is a window without a name coming forward: Chrome's own menu's window.
fn is_unnamed_window(kind: &EventKind, element: &ElementData) -> bool {
    *kind == EventKind::WindowActivated && element.name.as_deref().is_none_or(str::is_empty)
}

/// Whether the event is a menu's item highlighted: Chrome on Linux selects its own menus'
/// items, focusing none.
fn is_menu_highlight(kind: &EventKind, element: &ElementData) -> bool {
    *kind
        == (EventKind::StateChanged {
            flag: xa11y_core::StateFlag::Selected,
            value: true,
        })
        && element
            .raw
            .get("atspi_role")
            .and_then(|r| r.as_str())
            .is_some_and(is_menu_item_role)
}

/// Whether the event's object has the focus.
fn focused(target: Option<&ElementData>) -> bool {
    target.is_some_and(|e| e.states.focused)
}

/// The event loop's state: the focus as the engine was last told of it, the page read last,
/// and the control outside a page that has the focus.
#[derive(Debug, Default)]
pub struct Converter {
    track: FocusTrack,
    /// The document of the page read last.
    last_page: Option<String>,
    /// The focused control outside a page, as its focus event's snapshot had it.
    native_element: Option<ElementData>,
    /// The active window's title, from its activation and its renaming: a page's focus is in
    /// it, "Structural Navigation Reference Fixture - Google Chrome", as Windows' focus events
    /// carry their top-level window's title (Say Title's "Title is ..." [obs, 625/s1]).
    window_title: Option<String>,
    /// A page's select with the focus, by its object, as its focus was given to the engine:
    /// its value changes come as its hidden menu's items selected.
    page_select: Option<(String, FocusEvent)>,
    /// The unique ids of the objects of the page given to the engine last, by their paths: a
    /// focus in it names its node by its id, found where a role and a name do not find it (a
    /// link, whose role holds its address [obs, 204/s9]).
    page_ids: std::collections::HashMap<String, i32>,
    /// The places of that page's list items, by their paths: an option's focus says its list
    /// and its place.
    page_places: std::collections::HashMap<String, crate::page::ItemPlace>,
    /// The application regions that page's objects are in, by their paths.
    page_applications: std::collections::HashMap<String, std::sync::Arc<str>>,
    /// The live regions' lines in the last read of each of the last pages, by page: a page
    /// read again says what its regions changed.
    live_lines: Vec<(String, std::collections::HashMap<String, Vec<String>>)>,
    /// An application's own menus open (Chrome's app menu, Alt+F), outermost first, each by
    /// its window's object: Chrome on Linux opens each in a window of its own and highlights
    /// its items (AT-SPI's `selected`) without focusing them, where Windows' Chrome starts
    /// menu mode and focuses its items [obs, 220/s1-s5].
    menus: Vec<OpenMenu>,
    /// The last of those menus closed: menu mode ends as the focus or a window comes back,
    /// unless another menu's window opens first.
    menus_closing: bool,
}

/// An application's own menu open: its window's object, and the menu as it was read when the
/// window came forward, whose items give each highlighted one its place without reading it
/// again (the code review of 2026-11-09, third pass, finding 1: each arrow read the menu).
#[derive(Debug, Clone)]
struct OpenMenu {
    window: String,
    menu: AtspiNode,
}

/// What an application's own menu did, for one record: the engine's events, and whether the
/// record is said by them alone (an item highlighted).
struct MenuStep {
    events: Vec<AccessibilityEvent>,
    alone: bool,
}

/// How many pages' live regions are kept for their next read.
const LIVE_PAGES_KEPT: usize = 4;

impl Converter {
    /// Whether the event is GTK's second focus event for one move, passed over before anything
    /// is read.
    pub fn is_twin(&mut self, kind: &EventKind, target: Option<&ElementData>, at_ms: u64) -> bool {
        *kind == EventKind::FocusChanged
            && self
                .track
                .is_twin(target.and_then(object_key).as_deref(), at_ms)
    }

    /// Whether the page whose document is `document` is read as the focus lands on `element`
    /// in it: on its document (a load, or Tab coming back to it), in a page other than the one
    /// read last, or on an object the page given last does not hold. That page changed since it
    /// was read (a dialog opened, its content hidden until then; a list grew), and is read again
    /// before the focus, as Windows reads a page again as it changes [obs, 2681: the focus on
    /// the opened dialog's heading].
    pub fn wants_page(&self, element: &ElementData, document: &str) -> bool {
        let on_document = is_document(element);
        let read_last = self.last_page.as_deref() == Some(document);
        let changed = read_last
            && !on_document
            && !self.page_ids.is_empty()
            && element
                .raw
                .get("object_path")
                .and_then(|path| path.as_str())
                .is_some_and(|path| !self.page_ids.contains_key(path));
        on_document || !read_last || changed
    }

    /// The page whose document is `document` asked of the page reader: a focus in it before it
    /// comes does not ask again.
    pub fn page_asked(&mut self, document: &str) {
        self.last_page = Some(document.to_owned());
    }

    /// The page of `document` given up before the engine had it (the focus moved on before it
    /// was read): a focus coming back to it asks again.
    pub fn page_dropped(&mut self, document: &str) {
        if self.last_page.as_deref() == Some(document) {
            self.last_page = None;
        }
    }

    /// Whether the event's object is read with what surrounds it: a focus, or the focused
    /// control's change, outside a page.
    /// In a page, a menu item focused, which may be a select's option: Chrome on Linux shows a
    /// select's open list as a menu in the combo box (276).
    /// Outside a page, a window coming forward without a name (Chrome's own menu's) is read
    /// for the menu it may hold; its items highlighted later are not read (their places come
    /// from that read).
    pub fn wants_native(
        &self,
        kind: &EventKind,
        target: Option<&ElementData>,
        in_page: bool,
    ) -> bool {
        let focus = *kind == EventKind::FocusChanged;
        match target {
            None => false,
            Some(element) if in_page => focus && element.role == Role::MenuItem,
            Some(element) => {
                focus
                    || is_control_change(kind, focused(target))
                    || is_unnamed_window(kind, element)
            }
        }
    }

    /// Whether the event's object is read alone, without its ancestors: a window coming
    /// forward.
    pub fn reads_alone(kind: &EventKind) -> bool {
        *kind == EventKind::WindowActivated
    }

    /// A page's progress bar or meter changing: its new value, the bar found in the page by
    /// its object's id, as Windows' converter gives Chrome's ("10", "40", "100" [obs, 793/s2];
    /// "86.0274" [obs, 3113/s1]). Chrome on Linux tells it by the bar's `accessible-value`.
    /// A bar outside the page given last, or another object's value, says nothing here.
    fn progress(&self, record: &AtspiRecord) -> Option<AccessibilityEvent> {
        let target = record.target.as_ref()?;
        if target.role != Role::ProgressBar {
            return None;
        }
        // The page given last's object: another application's paths look alike (an Electron
        // application's), its bus tells them apart.
        let bus = target.raw.get("bus_name").and_then(|b| b.as_str())?;
        let in_last_page = self
            .last_page
            .as_deref()
            .and_then(|document| document.strip_prefix(bus))
            .is_some_and(|rest| rest.starts_with('/'));
        if !in_last_page {
            return None;
        }
        let path = target.raw.get("object_path").and_then(|p| p.as_str())?;
        let id = self.page_ids.get(path)?;
        let value = target
            .value
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .or_else(|| target.numeric_value.and_then(number))?;
        let mut bar = FocusEvent::new(okur_core::aom::Role::ProgressBar);
        bar.name = target.name.clone().filter(|n| !n.trim().is_empty());
        bar.value = Some(value);
        bar.in_document = true;
        bar.unique_id = Some(id.to_string());
        Some(AccessibilityEvent::ProgressChanged {
            bar,
            at_ms: record.at_ms,
        })
    }

    /// An application's own menu (Chrome's app menu, Alt+F) as Windows gives it: menu mode
    /// and its popup started as its window comes forward, each item highlighted as its focus
    /// with its place, the popup ended as its window goes, and menu mode as the focus or
    /// another window comes back [obs, 220/s1-s5]. Keys go to the menu from its start, as the
    /// engine passes them through in menu mode.
    fn own_menu(&mut self, record: &AtspiRecord, object: Option<&str>, in_page: bool) -> MenuStep {
        let mut events = Vec::new();
        let mut alone = false;
        let app = Some(record.app.clone()).filter(|a| !a.is_empty());
        let target = record.target.as_ref();
        let highlight = !in_page
            && !self.menus.is_empty()
            && target.is_some_and(|t| is_menu_highlight(&record.kind, t));
        match &record.kind {
            EventKind::WindowActivated => {
                let read = record.native.as_ref().filter(|r| !r.focus.role.is_empty());
                let menu = read.and_then(|r| menu_in(&r.focus));
                let named = target
                    .and_then(|t| t.name.as_deref())
                    .is_some_and(|n| !n.trim().is_empty());
                match (menu, object) {
                    // A menu open already comes forward again (its submenu closed): the menus
                    // opened from it are gone.
                    (Some(_), Some(window)) if self.menus.iter().any(|m| m.window == window) => {
                        let at = self
                            .menus
                            .iter()
                            .position(|m| m.window == window)
                            .map_or(self.menus.len(), |at| at + 1);
                        for _ in at..self.menus.len() {
                            events.push(AccessibilityEvent::Menu(MenuEvent::PopupEnd));
                        }
                        self.menus.truncate(at);
                    }
                    (Some(menu), Some(window)) => {
                        if self.menus.is_empty() && !self.menus_closing {
                            events.push(AccessibilityEvent::Menu(MenuEvent::Start {
                                launcher: false,
                            }));
                        }
                        self.menus_closing = false;
                        self.menus.push(OpenMenu {
                            window: window.to_owned(),
                            menu: menu.clone(),
                        });
                        events.push(AccessibilityEvent::Menu(MenuEvent::PopupStart {
                            kind: PopupKind::Plain,
                        }));
                        // An item highlighted as it opened.
                        if let Some(item) = menu.children.iter().find(|c| {
                            is_menu_item_role(&c.role) && c.has(crate::page::state::SELECTED)
                        }) {
                            let mut focus = menu_item_focus(item, Some(menu));
                            focus.app = app;
                            focus.platform.window_id = Some(window.to_owned());
                            events.push(AccessibilityEvent::Focus(focus));
                        }
                    }
                    // Another window with a title comes forward: whatever menus were open have
                    // closed, a deactivation missed (a click elsewhere, Alt+Tab) or not (the
                    // code review of 2026-11-09, third pass, finding 2).
                    _ if named => self.end_menus(&mut events),
                    _ => {}
                }
            }
            EventKind::WindowDeactivated => {
                // The innermost menu going. One under a submenu still open stays: Chrome may
                // let its window go as the submenu's comes.
                if object.is_some() && self.menus.last().map(|m| m.window.as_str()) == object {
                    self.menus.pop();
                    events.push(AccessibilityEvent::Menu(MenuEvent::PopupEnd));
                    self.menus_closing = self.menus.is_empty();
                }
            }
            // A focus ends the menus: Chrome focuses none of its menus' items.
            EventKind::FocusChanged => self.end_menus(&mut events),
            _ if highlight => {
                alone = true;
                if let Some(target) = target {
                    events.push(AccessibilityEvent::Focus(self.highlighted(target, app)));
                }
            }
            _ => {}
        }
        MenuStep { events, alone }
    }

    /// A menu's item highlighted, as its focus: its place among the items of the open menu
    /// that holds it, as read when the menu came forward; its name and state as its event's
    /// own snapshot has them (Chrome names its items as its menu opens).
    fn highlighted(&self, target: &ElementData, app: Option<String>) -> FocusEvent {
        let path = target
            .raw
            .get("object_path")
            .and_then(|p| p.as_str())
            .filter(|p| !p.is_empty());
        let found = path.and_then(|path| {
            self.menus.iter().rev().find_map(|open| {
                open.menu
                    .children
                    .iter()
                    .find(|item| item.path == path)
                    .map(|item| (open, item))
            })
        });
        let mut focus = match found {
            Some((open, item)) => {
                let mut focus = menu_item_focus(item, Some(&open.menu));
                focus.platform.window_id = Some(open.window.clone());
                focus
            }
            // An item the read did not hold: as its snapshot alone has it.
            None => {
                let mut focus = FocusEvent::new(okur_core::aom::Role::MenuItem);
                focus.platform.window_id = self.menus.last().map(|m| m.window.clone());
                focus
            }
        };
        if let Some(name) = target
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            focus.name = Some(name.to_owned());
        }
        if let (Some(_), Some(checked)) = (focus.states.checked, target.states.checked) {
            focus.states.checked = Some(checked == xa11y_core::Toggled::On);
        }
        focus.app = app;
        focus
    }

    /// Every menu open ends: their popups, then menu mode.
    fn end_menus(&mut self, events: &mut Vec<AccessibilityEvent>) {
        for _ in &self.menus {
            events.push(AccessibilityEvent::Menu(MenuEvent::PopupEnd));
        }
        let open = !self.menus.is_empty();
        self.menus.clear();
        if std::mem::take(&mut self.menus_closing) || open {
            events.push(AccessibilityEvent::Menu(MenuEvent::End));
        }
    }

    /// The focused control to read again, as something within it changed in its application.
    pub fn wants_reread(
        &self,
        kind: &EventKind,
        target: Option<&ElementData>,
    ) -> Option<&ElementData> {
        let object = target.and_then(object_key);
        let inside = is_inside_change(kind, focused(target))
            && self.track.in_focused_application(object.as_deref());
        self.native_element.as_ref().filter(|_| inside)
    }

    /// What a live region read again changed, compared with its lines when the page or the
    /// region was read before, then the region spliced into the page the engine has, by its
    /// objects' ids, which stay from read to read: the page is not read again for it. A region
    /// of another page than the one given last says nothing (a background tab's message, the
    /// code review of 2026-11-09, second pass, 6); a region read cut short, or whose objects have
    /// no ids, is the page's change instead, which captures the page again.
    fn live_read(&mut self, read: &LiveRead) -> Vec<AccessibilityEvent> {
        if self.last_page.as_deref() != Some(read.document.as_str()) {
            return Vec::new();
        }
        if read.cut {
            return vec![AccessibilityEvent::PageChanged];
        }
        let regions = crate::live::regions(&read.root);
        let ids = crate::page::page_ids(&read.root, 0);
        let part = to_aom_from(&read.root, 0);
        let lines = crate::live::region_lines(&part, &regions, &ids);
        let at = self
            .live_lines
            .iter()
            .position(|(document, _)| *document == read.document);
        let before = at
            .and_then(|i| self.live_lines.get(i))
            .map(|(_, lines)| lines);
        let mut events: Vec<AccessibilityEvent> = crate::live::changes(&regions, before, &lines)
            .into_iter()
            .map(AccessibilityEvent::LiveRegion)
            .collect();
        match at.and_then(|i| self.live_lines.get_mut(i)) {
            Some((_, known)) => known.extend(lines),
            None => {
                self.live_lines.push((read.document.clone(), lines));
                if self.live_lines.len() > LIVE_PAGES_KEPT {
                    self.live_lines.remove(0);
                }
            }
        }
        // The region's root as the page has it: spliced where the engine's page has the same id.
        let in_page =
            read.root.id.is_some() && self.page_ids.get(&read.root.path).copied() == read.root.id;
        if in_page {
            self.page_ids.extend(ids);
            self.page_places
                .extend(crate::page::item_places(&read.root));
            events.push(AccessibilityEvent::PagePartsLoaded(vec![part]));
        } else {
            events.push(AccessibilityEvent::PageChanged);
        }
        events
    }

    /// The engine's events for what was read for one event, in order: the pages read, then the
    /// focus or the change.
    pub fn convert_read(&mut self, record: &AtspiRecord) -> Vec<AccessibilityEvent> {
        // A text field read at its caret: what the engine asked for, as read.
        if let Some(read) = &record.text {
            return vec![AccessibilityEvent::TextContext(read.context.clone())];
        }
        // A live region read again: what it changed, then the page changed, for the engine to
        // capture it again once its changes settle.
        if let Some(read) = &record.live {
            return self.live_read(read);
        }
        let mut events = Vec::new();
        for page in &record.pages {
            self.last_page = Some(page.document.clone());
            self.page_ids = crate::page::page_ids(&page.root, page.base);
            self.page_places = crate::page::item_places(&page.root);
            self.page_applications = crate::page::applications(&page.root);
            let root = to_aom_from(&page.root, page.base);
            // What its live regions changed since it was read last, said before the page is
            // refreshed [obs, 206/s2].
            let regions = crate::live::regions(&page.root);
            let lines = crate::live::region_lines(&root, &regions, &self.page_ids);
            let at = self
                .live_lines
                .iter()
                .position(|(document, _)| *document == page.document);
            let before = at
                .and_then(|i| self.live_lines.get(i))
                .map(|(_, lines)| lines);
            for live in crate::live::changes(&regions, before, &lines) {
                events.push(AccessibilityEvent::LiveRegion(live));
            }
            if let Some(i) = at {
                self.live_lines.remove(i);
            }
            if !regions.is_empty() {
                self.live_lines.push((page.document.clone(), lines));
                if self.live_lines.len() > LIVE_PAGES_KEPT {
                    self.live_lines.remove(0);
                }
            }
            let title = root.name.clone().unwrap_or_default();
            events.push(AccessibilityEvent::DocumentLoaded { title, root });
        }
        let target = record.target.as_ref();
        let object = target.and_then(object_key);
        let name = || {
            target
                .and_then(|t| t.name.clone())
                .filter(|n| !n.is_empty())
        };
        match record.kind {
            EventKind::WindowActivated => {
                self.track.window_activated(object.clone());
                self.window_title = name();
            }
            EventKind::WindowDeactivated => self.track.window_deactivated(),
            // The active window renamed: a browser's, as its page changes.
            EventKind::NameChanged
                if object.is_some() && object.as_deref() == self.track.active_window() =>
            {
                self.window_title = name();
            }
            _ => {}
        }
        let app = Some(record.app.clone()).filter(|a| !a.is_empty());
        let focus = record.kind == EventKind::FocusChanged;
        let changed = is_control_change(&record.kind, focused(target));
        let in_page = record.document.is_some();
        let menu = self.own_menu(record, object.as_deref(), in_page);
        events.extend(menu.events);
        if menu.alone {
            return events;
        }
        if record.kind == EventKind::ValueChanged {
            events.extend(self.progress(record));
            return events;
        }
        let outside_page = (focus || changed) && !in_page;
        // A focus outside a page, and its control's change, read with what surrounds it: the
        // change names the control as its focus did.
        let native = target
            .filter(|_| outside_page)
            // A read that failed (no role: the object gone, or the bus call timed out) is no
            // read: the event's snapshot says the control (the second review, finding 4).
            .and_then(
                |element| match record.native.as_ref().filter(|r| !r.focus.role.is_empty()) {
                    // The focus moved on before it was read (fast Tabs): its newer focus event
                    // follows, and this one is not said (the code review of 2026-11-08, 3c). The
                    // lab's traces at f0b3a66d: every focus read but each desktop fixture's first,
                    // its window's object before any key, has FOCUSED.
                    Some(read) if focus && !read.focus.has(crate::page::state::FOCUSED) => None,
                    Some(read) => native_control(element, read, app.clone()),
                    // Not read (its object unnamed): as its event's snapshot has it.
                    None => {
                        let mut control = focus_event(element);
                        control.app = app.clone();
                        Some(control)
                    }
                },
            )
            .map(|control| {
                if changed {
                    AccessibilityEvent::Change {
                        kind: ChangeKind::State,
                        control,
                    }
                } else {
                    AccessibilityEvent::Focus(control)
                }
            });
        if let Some(AccessibilityEvent::Focus(read)) = &native {
            if !self.track.native_focus(object.clone(), read) {
                // The toolkit's repeat of the focus: nothing more to say.
                return events;
            }
            self.native_element = record.target.clone();
        } else if focus && in_page {
            self.track.page_focus(object.clone());
            self.native_element = None;
        }
        // The focused control read again: said as a focus when it reads as another control
        // (GTK's notebook's next tab [obs, 303/s13]), as its value changing when only that
        // changed (GTK's drop-down's item [obs, 304/s9]).
        if let (Some(element), Some(read)) = (self.native_element.clone(), &record.reread) {
            let Some(control) = native_control(&element, read, app.clone()) else {
                return events;
            };
            match self.track.reread(&control) {
                Reread::Focus => events.push(AccessibilityEvent::Focus(control)),
                Reread::Value => events.push(AccessibilityEvent::Change {
                    kind: ChangeKind::Value,
                    control,
                }),
                Reread::Same => {}
            }
        }
        // A select's option, read with its menu and its combo box: the list box option the
        // engine takes it as, Windows' shape [obs, 276/s2].
        let option = target
            .filter(|_| focus && in_page)
            .zip(record.native.as_ref())
            .and_then(|(element, read)| select_option(element, read, app.clone()));
        // Outside a page the native reading alone: none is a focus in a window behind.
        let converted = if let Some(option) = option {
            Some(AccessibilityEvent::Focus(option))
        } else if outside_page && target.is_some() {
            native
        } else {
            native.or_else(|| {
                // A page's focus is in the active window, where its application has it.
                let window = self.window_title.clone().filter(|_| {
                    in_page && same_application(object.as_deref(), self.track.active_window())
                });
                let path = target
                    .filter(|_| in_page)
                    .and_then(|t| t.raw.get("object_path"))
                    .and_then(|path| path.as_str());
                let unique_id = path
                    .and_then(|path| self.page_ids.get(path))
                    .map(i32::to_string);
                let item_place = path.and_then(|path| self.page_places.get(path));
                let application = path
                    .and_then(|path| self.page_applications.get(path))
                    .map(|region| region.to_string());
                let window_id = self
                    .track
                    .active_window()
                    .filter(|_| {
                        in_page && same_application(object.as_deref(), self.track.active_window())
                    })
                    .map(str::to_owned);
                plain(record, in_page, app, window, unique_id).map(|mut event| {
                    // A page's focus is in its browser's window, by that window's object, as a
                    // native focus names its window: leaving the page for the browser's
                    // toolbar is no window change [obs, 204/s11].
                    if let AccessibilityEvent::Focus(focus)
                    | AccessibilityEvent::Change { control: focus, .. } = &mut event
                    {
                        focus.platform.window_id = window_id;
                        focus.web_application = application;
                        // An option's list and its place, as Windows' focus carries them.
                        if let Some(place) =
                            item_place.filter(|_| focus.role == okur_core::aom::Role::ListItem)
                        {
                            focus.container_name.clone_from(&place.list);
                            focus.container_is_list_box = place.list_box;
                            focus.item_index = Some(place.index);
                            focus.item_count = Some(place.count);
                        }
                    }
                    event
                })
            })
        };
        // A page's select focused: kept for its value's changes; any other focus ends it.
        if focus {
            self.page_select = match (&converted, object.clone()) {
                (Some(AccessibilityEvent::Focus(control)), Some(object))
                    if in_page && control.role == okur_core::aom::Role::ComboBox =>
                {
                    Some((object, control.clone()))
                }
                _ => None,
            };
        }
        // The select's value changed by the arrows while it is closed: Chrome on Linux selects
        // the item in the select's hidden menu, focus unmoved; Windows' Chrome changes the
        // select's value, which JAWS says, "Eastgate" [obs, 205/s9, s10].
        let selected = record.kind
            == EventKind::StateChanged {
                flag: xa11y_core::StateFlag::Selected,
                value: true,
            };
        let value_change = target
            .filter(|t| selected && t.role == Role::MenuItem && !t.states.focused)
            .and_then(|item| {
                let (select, control) = self.page_select.as_ref()?;
                same_application(object.as_deref(), Some(select)).then(|| {
                    let mut control = control.clone();
                    control.value = item.name.clone().filter(|n| !n.is_empty());
                    AccessibilityEvent::Change {
                        kind: ChangeKind::Value,
                        control,
                    }
                })
            });
        // The select opened: Chrome on Linux says it expanded as its list takes the focus, its
        // focused state gone, where Windows' Chrome says it of the focused select, which the
        // engine says as "expanded" with its list [obs, 276/s7].
        let expanded = record.kind
            == EventKind::StateChanged {
                flag: xa11y_core::StateFlag::Expanded,
                value: true,
            };
        let opened = self
            .page_select
            .as_ref()
            .filter(|(select, _)| expanded && object.as_deref() == Some(select.as_str()))
            .map(|(_, control)| {
                let mut control = control.clone();
                control.states.expanded = Some(true);
                AccessibilityEvent::Change {
                    kind: ChangeKind::State,
                    control,
                }
            });
        let converted = converted.filter(|_| opened.is_none());
        events.extend(converted);
        events.extend(value_change);
        events.extend(opened);
        events
    }
}

impl EventModel for Converter {
    type Record = AtspiRecord;

    /// A record as the driver converted it: GTK's twin focus passed over before anything was
    /// read, then what was read.
    fn convert(&mut self, record: &AtspiRecord) -> Vec<AccessibilityEvent> {
        if self.is_twin(&record.kind, record.target.as_ref(), record.at_ms) {
            return Vec::new();
        }
        self.convert_read(record)
    }
}

/// A control outside a page read with what surrounds it, as the engine's focus event; none in a
/// window behind the active one, which is not said.
fn native_control(
    element: &ElementData,
    read: &NativeRead,
    app: Option<String>,
) -> Option<FocusEvent> {
    if !in_active_window(&read.ancestors) {
        return None;
    }
    let mut control = native_focus(focus_event(element), &read.focus, &read.ancestors);
    control.app = app;
    // Its window by the window's object, its application's bus name and its path: two windows
    // of one title are two windows.
    let bus = element.raw.get("bus_name").and_then(|v| v.as_str());
    control.platform.window_id = read
        .ancestors
        .iter()
        .find(|a| crate::focus::is_window(&a.role))
        .filter(|window| !window.path.is_empty())
        .zip(bus)
        .map(|(window, bus)| format!("{bus}{}", window.path));
    Some(control)
}

/// A focus, or the focused control's change, as its event's snapshot has it: a page's (`in_page`,
/// where the virtual cursor reads and takes the keys, as the focus chain that holds a document
/// tells it on Windows).
/// A select's option as Chrome on Linux shows it, a menu item of a menu in a combo box: a list
/// box's option in the page, with its place, as Windows' select list gives it [obs, 276/s2:
/// "list | with 3 items | Northside"].
fn select_option(
    element: &ElementData,
    read: &NativeRead,
    app: Option<String>,
) -> Option<FocusEvent> {
    if element.role != Role::MenuItem {
        return None;
    }
    let menu = read.ancestors.first().filter(|m| m.role == "menu")?;
    read.ancestors.get(1).filter(|c| c.role == "combo box")?;
    let mut option = focus_event(element);
    option.role = okur_core::aom::Role::ListItem;
    option.in_document = true;
    option.app = app;
    option.container_role = Some(okur_core::aom::Role::List);
    option.container_is_list_box = true;
    match crate::focus::own_place(&read.focus) {
        Some((at, of)) => {
            option.item_index = Some(at);
            option.item_count = Some(of);
        }
        None => crate::focus::place(&mut option, &read.focus, menu),
    }
    Some(option)
}

/// Whether two objects' keys (bus name and path) are of one application, by their bus names.
fn same_application(a: Option<&str>, b: Option<&str>) -> bool {
    let bus = |key: &str| key.split('/').next().map(str::to_owned);
    match (a.and_then(bus), b.and_then(bus)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn plain(
    record: &AtspiRecord,
    in_page: bool,
    app: Option<String>,
    window_title: Option<String>,
    unique_id: Option<String>,
) -> Option<AccessibilityEvent> {
    let element = record.target.as_ref()?;
    let mut control = focus_event(element);
    control.in_document = in_page;
    control.app = app;
    control.window_title = window_title;
    control.unique_id = unique_id;
    match record.kind {
        EventKind::FocusChanged => Some(AccessibilityEvent::Focus(control)),
        _ if is_control_change(&record.kind, element.states.focused) => {
            Some(AccessibilityEvent::Change {
                kind: ChangeKind::State,
                control,
            })
        }
        _ => None,
    }
}

/// Whether `element` is a page's document (Chrome's web area).
pub fn is_document(element: &ElementData) -> bool {
    element.role == Role::WebArea
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page::state;
    use okur_core::aom::Role as OkurRole;
    use xa11y_core::{Role as ElementRole, StateFlag};

    fn element(role: ElementRole, name: &str, path: &str, focused: bool) -> ElementData {
        let mut e = ElementData::for_role(role);
        e.name = Some(name.to_owned());
        e.states.focused = focused;
        e.raw.insert(
            "bus_name".to_owned(),
            serde_json::Value::String(":1.5".to_owned()),
        );
        e.raw.insert(
            "object_path".to_owned(),
            serde_json::Value::String(path.to_owned()),
        );
        e
    }

    fn node(role: &str, name: &str, states: &[u32]) -> AtspiNode {
        let mut n = AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            ..AtspiNode::default()
        };
        for s in states {
            n.states |= 1u64 << s;
        }
        n
    }

    fn window() -> AtspiNode {
        node("frame", "SR Desktop Fixture", &[state::ACTIVE])
    }

    fn record(kind: EventKind, target: ElementData, native: Option<NativeRead>) -> AtspiRecord {
        AtspiRecord {
            at_ms: 0,
            kind,
            app: "desktop-forms.py".to_owned(),
            target: Some(target),
            document: None,
            pages: Vec::new(),
            native,
            reread: None,
            text: None,
            live: None,
            delivered_ms: None,
        }
    }

    fn read(focus: AtspiNode) -> NativeRead {
        NativeRead {
            focus,
            ancestors: vec![window()],
            read_ms: 0,
        }
    }

    /// A focus outside a page is said once, with its application; GTK's repeat is passed over;
    /// Space's check is said as the control's change [obs, 304/s1].
    #[test]
    fn a_native_focus_its_repeat_and_its_change() {
        let mut converter = Converter::default();
        let mut check = element(ElementRole::CheckBox, "Remember me", "/check", true);
        check.states.checked = Some(xa11y_core::Toggled::Off);
        let focus = record(
            EventKind::FocusChanged,
            check.clone(),
            Some(read(node("check box", "Remember me", &[state::FOCUSED]))),
        );
        let events = converter.convert_read(&focus);
        assert!(matches!(
            events.as_slice(),
            [AccessibilityEvent::Focus(f)] if f.role == OkurRole::CheckBox
                && f.app.as_deref() == Some("desktop-forms.py")
        ));
        assert!(converter.convert_read(&focus).is_empty());
        let checked = record(
            EventKind::StateChanged {
                flag: StateFlag::Checked,
                value: true,
            },
            check,
            Some(read(node(
                "check box",
                "Remember me",
                &[state::FOCUSED, state::CHECKED],
            ))),
        );
        assert!(matches!(
            converter.convert_read(&checked).as_slice(),
            [AccessibilityEvent::Change { kind: ChangeKind::State, control }]
                if control.states.checked == Some(true)
        ));
    }

    /// A focus read after the focus moved on, its fresh object no longer focused, is not said:
    /// the newer focus follows.
    #[test]
    fn a_focus_gone_before_its_read_is_not_said() {
        let mut converter = Converter::default();
        let button = element(ElementRole::Button, "OK", "/ok", true);
        let gone = record(
            EventKind::FocusChanged,
            button.clone(),
            Some(read(node("push button", "OK", &[]))),
        );
        assert!(converter.convert_read(&gone).is_empty());
        let here = record(
            EventKind::FocusChanged,
            button,
            Some(read(node("push button", "OK", &[state::FOCUSED]))),
        );
        assert!(matches!(
            converter.convert_read(&here).as_slice(),
            [AccessibilityEvent::Focus(_)]
        ));
    }

    fn menu_item(name: &str, path: &str, states: &[u32]) -> AtspiNode {
        let mut item = node("menu item", name, states);
        item.path = path.to_owned();
        item
    }

    fn chrome_menu(selected: Option<usize>) -> AtspiNode {
        let mut menu = node("menu", "", &[]);
        let names = [
            "New tab Ctrl+T",
            "New window Ctrl+N",
            "Your Chrome Not signed in",
        ];
        for (at, name) in names.iter().enumerate() {
            let states: &[u32] = if selected == Some(at) {
                &[state::SELECTED]
            } else {
                &[]
            };
            let mut item = menu_item(name, &format!("/item/{at}"), states);
            if at == 2 {
                item.attributes
                    .insert("haspopup".to_owned(), "menu".to_owned());
            }
            menu.children.push(item);
            if at == 0 {
                menu.children.push(node("separator", "", &[]));
            }
        }
        menu
    }

    fn chrome_record(
        kind: EventKind,
        target: ElementData,
        native: Option<NativeRead>,
    ) -> AtspiRecord {
        AtspiRecord {
            app: "Google Chrome".to_owned(),
            ..record(kind, target, native)
        }
    }

    fn menu_window() -> ElementData {
        let mut window = element(ElementRole::Window, "", "/menu-window", false);
        window.name = None;
        window
    }

    fn highlight(at: usize, name: &str) -> AtspiRecord {
        let mut item = element(ElementRole::MenuItem, name, &format!("/item/{at}"), false);
        item.raw.insert(
            "atspi_role".to_owned(),
            serde_json::Value::String("menu item".to_owned()),
        );
        chrome_record(
            EventKind::StateChanged {
                flag: StateFlag::Selected,
                value: true,
            },
            item,
            None,
        )
    }

    fn menu_opened(path: &str) -> AtspiRecord {
        let mut frame = window();
        frame.children.push(chrome_menu(None));
        let mut opened = menu_window();
        opened.raw.insert(
            "object_path".to_owned(),
            serde_json::Value::String(path.to_owned()),
        );
        chrome_record(
            EventKind::WindowActivated,
            opened,
            Some(NativeRead {
                focus: frame,
                ancestors: Vec::new(),
                read_ms: 0,
            }),
        )
    }

    fn window_gone(path: &str) -> AtspiRecord {
        let mut gone = menu_window();
        gone.role = ElementRole::Group;
        gone.raw.insert(
            "object_path".to_owned(),
            serde_json::Value::String(path.to_owned()),
        );
        chrome_record(EventKind::WindowDeactivated, gone, None)
    }

    fn page_window() -> AtspiRecord {
        let window = element(ElementRole::Window, "News - Google Chrome", "/1", false);
        chrome_record(EventKind::WindowActivated, window, None)
    }

    /// Chrome's own menu on Linux: its window comes forward holding it (menu mode and its popup
    /// start, its keys the menu's), each item highlighted is said as its focus with its place
    /// among the items read as the menu opened, a submenu's item as one, and the window going
    /// ends the popup, then menu mode as the page's window comes back [obs, 220/s1-s5]. A
    /// highlight is never read.
    #[test]
    fn chromes_own_menu_is_menu_mode_and_its_highlighted_items() {
        let mut converter = Converter::default();
        assert!(matches!(
            converter
                .convert_read(&menu_opened("/menu-window"))
                .as_slice(),
            [
                AccessibilityEvent::Menu(MenuEvent::Start { launcher: false }),
                AccessibilityEvent::Menu(MenuEvent::PopupStart {
                    kind: PopupKind::Plain
                }),
            ]
        ));
        let second = highlight(1, "New window Ctrl+N");
        assert!(!converter.wants_native(&second.kind, second.target.as_ref(), false));
        assert!(matches!(
            converter.convert_read(&second).as_slice(),
            [AccessibilityEvent::Focus(f)] if f.role == OkurRole::MenuItem
                && f.name.as_deref() == Some("New window Ctrl+N")
                && (f.item_index, f.item_count) == (Some(2), Some(3))
                && f.states.expanded.is_none()
                && f.platform.window_id.as_deref() == Some(":1.5/menu-window")
        ));
        assert!(matches!(
            converter.convert_read(&highlight(2, "Your Chrome Not signed in")).as_slice(),
            [AccessibilityEvent::Focus(f)] if f.states.expanded == Some(false)
                && (f.item_index, f.item_count) == (Some(3), Some(3))
        ));
        assert!(matches!(
            converter
                .convert_read(&window_gone("/menu-window"))
                .as_slice(),
            [AccessibilityEvent::Menu(MenuEvent::PopupEnd)]
        ));
        assert!(matches!(
            converter.convert_read(&page_window()).as_slice(),
            [AccessibilityEvent::Menu(MenuEvent::End)]
        ));
    }

    /// Menus whose window's going was missed end as another window with a title, or a focus,
    /// comes; a select's highlight after them is no menu's.
    #[test]
    fn menus_whose_end_was_missed_end_with_the_next_window_or_focus() {
        let mut converter = Converter::default();
        converter.convert_read(&menu_opened("/menu-window"));
        assert!(matches!(
            converter.convert_read(&page_window()).as_slice(),
            [
                AccessibilityEvent::Menu(MenuEvent::PopupEnd),
                AccessibilityEvent::Menu(MenuEvent::End),
            ]
        ));
        assert!(!converter
            .convert_read(&highlight(0, "Northside"))
            .iter()
            .any(|e| matches!(
                e,
                AccessibilityEvent::Menu(_) | AccessibilityEvent::Focus(_)
            )));
        converter.convert_read(&menu_opened("/menu-window"));
        let link = element(ElementRole::Link, "Home", "/link", true);
        let focus = chrome_record(EventKind::FocusChanged, link, None);
        let events = converter.convert_read(&focus);
        assert!(matches!(
            events.as_slice(),
            [
                AccessibilityEvent::Menu(MenuEvent::PopupEnd),
                AccessibilityEvent::Menu(MenuEvent::End),
                ..
            ]
        ));
    }

    /// A submenu opens over its menu, in either order of their windows' events: the menu's
    /// window going under the open submenu ends nothing; the submenu going ends its popup; the
    /// menu coming forward again ends what was above it; a window come forward twice is one
    /// menu.
    #[test]
    fn a_submenu_over_its_menu_in_either_order() {
        let mut converter = Converter::default();
        converter.convert_read(&menu_opened("/menu"));
        assert!(matches!(
            converter.convert_read(&menu_opened("/submenu")).as_slice(),
            [AccessibilityEvent::Menu(MenuEvent::PopupStart { .. })]
        ));
        assert!(converter.convert_read(&window_gone("/menu")).is_empty());
        assert!(matches!(
            converter.convert_read(&highlight(1, "New window Ctrl+N")).as_slice(),
            [AccessibilityEvent::Focus(f)] if f.platform.window_id.as_deref() == Some(":1.5/submenu")
        ));
        assert!(matches!(
            converter.convert_read(&menu_opened("/menu")).as_slice(),
            [AccessibilityEvent::Menu(MenuEvent::PopupEnd)]
        ));
        assert!(converter.convert_read(&menu_opened("/menu")).is_empty());
        assert!(matches!(
            converter.convert_read(&window_gone("/menu")).as_slice(),
            [AccessibilityEvent::Menu(MenuEvent::PopupEnd)]
        ));
        assert!(matches!(
            converter.convert_read(&page_window()).as_slice(),
            [AccessibilityEvent::Menu(MenuEvent::End)]
        ));
    }

    /// A page's progress bar changing says its new value, the bar named by its page id; a bar
    /// the page given last does not hold says nothing.
    #[test]
    fn a_page_progress_bars_value_change_is_its_progress() {
        let mut converter = Converter::default();
        converter.page_ids.insert("/bar".to_owned(), 41);
        converter.last_page = Some(":1.5/doc".to_owned());
        let mut bar = element(ElementRole::ProgressBar, "Upload", "/bar", false);
        bar.numeric_value = Some(40.0);
        let changed = chrome_record(EventKind::ValueChanged, bar.clone(), None);
        assert!(matches!(
            converter.convert_read(&changed).as_slice(),
            [AccessibilityEvent::ProgressChanged { bar, .. }]
                if bar.value.as_deref() == Some("40")
                    && bar.unique_id.as_deref() == Some("41")
                    && bar.name.as_deref() == Some("Upload")
        ));
        // As Chromium's single-precision value comes, widened.
        bar.numeric_value = Some(f64::from(86.0274_f32));
        let meter = chrome_record(EventKind::ValueChanged, bar.clone(), None);
        assert!(matches!(
            converter.convert_read(&meter).as_slice(),
            [AccessibilityEvent::ProgressChanged { bar, .. }] if bar.value.as_deref() == Some("86.0274")
        ));
        bar.numeric_value = Some(f64::NAN);
        let no_number = chrome_record(EventKind::ValueChanged, bar.clone(), None);
        assert!(converter.convert_read(&no_number).is_empty());
        // Another application's object of the same path.
        bar.numeric_value = Some(50.0);
        bar.raw.insert(
            "bus_name".to_owned(),
            serde_json::Value::String(":1.9".to_owned()),
        );
        let elsewhere_app = chrome_record(EventKind::ValueChanged, bar, None);
        assert!(converter.convert_read(&elsewhere_app).is_empty());
        let other = element(ElementRole::ProgressBar, "Elsewhere", "/other", false);
        let elsewhere = chrome_record(EventKind::ValueChanged, other, None);
        assert!(converter.convert_read(&elsewhere).is_empty());
    }

    /// A window without a menu starts no menu mode, and an item highlighted with no menu open
    /// is no menu's: a page's select highlights its hidden menu's items as its value changes.
    #[test]
    fn a_highlight_without_a_menu_open_is_no_menus() {
        let mut converter = Converter::default();
        let plain = chrome_record(
            EventKind::WindowActivated,
            menu_window(),
            Some(read(node("panel", "", &[]))),
        );
        assert!(converter.convert_read(&plain).is_empty());
        let mut item = element(ElementRole::MenuItem, "New tab Ctrl+T", "/item/0", false);
        item.raw.insert(
            "atspi_role".to_owned(),
            serde_json::Value::String("menu item".to_owned()),
        );
        let mut menu = chrome_menu(Some(0));
        menu.path = "/menu".to_owned();
        let highlighted = chrome_record(
            EventKind::StateChanged {
                flag: StateFlag::Selected,
                value: true,
            },
            item,
            Some(NativeRead {
                focus: menu_item("New tab Ctrl+T", "/item/0", &[state::SELECTED]),
                ancestors: vec![menu, window()],
                read_ms: 0,
            }),
        );
        assert!(!converter
            .convert_read(&highlighted)
            .iter()
            .any(|e| matches!(
                e,
                AccessibilityEvent::Menu(_) | AccessibilityEvent::Focus(_)
            )));
        assert!(!converter.wants_native(&highlighted.kind, highlighted.target.as_ref(), false));
    }

    /// Chrome on Linux shows a select's open list as a menu in its combo box: its focused item
    /// is the list box's option, with its place [obs, 276/s2].
    #[test]
    fn a_selects_menu_item_is_its_list_boxs_option() {
        let mut converter = Converter::default();
        let item = element(ElementRole::MenuItem, "Northside", "/n", true);
        let mut menu = node("menu", "", &[]);
        for (name, path) in [("Northside", "/n"), ("Eastgate", "/e"), ("Westfield", "/w")] {
            let mut option = node("menu item", name, &[]);
            option.path = path.to_owned();
            menu.children.push(option);
        }
        let mut focus = node("menu item", "Northside", &[state::FOCUSED]);
        focus.path = "/n".to_owned();
        let mut opened = record(
            EventKind::FocusChanged,
            item,
            Some(NativeRead {
                focus,
                ancestors: vec![menu, node("combo box", "Home branch", &[])],
                read_ms: 0,
            }),
        );
        opened.document = Some(":1.5/doc".to_owned());
        assert!(matches!(
            converter.convert_read(&opened).as_slice(),
            [AccessibilityEvent::Focus(f)] if f.role == OkurRole::ListItem
                && f.container_is_list_box
                && f.in_document
                && f.item_index == Some(1)
                && f.item_count == Some(3)
        ));
    }

    /// A page's select changed by the arrows while closed: its hidden menu's item selected, the
    /// select's value change [obs, 205/s9].
    #[test]
    fn a_selects_item_selected_is_its_value_change() {
        let mut converter = Converter::default();
        let select = element(ElementRole::ComboBox, "Home branch", "/select", true);
        let mut focus = record(EventKind::FocusChanged, select, None);
        focus.document = Some(":1.5/doc".to_owned());
        let _ = converter.convert_read(&focus);
        let item = element(ElementRole::MenuItem, "Eastgate", "/e", false);
        let chosen = record(
            EventKind::StateChanged {
                flag: StateFlag::Selected,
                value: true,
            },
            item,
            None,
        );
        assert!(matches!(
            converter.convert_read(&chosen).as_slice(),
            [AccessibilityEvent::Change { kind: ChangeKind::Value, control }]
                if control.value.as_deref() == Some("Eastgate")
                    && control.role == OkurRole::ComboBox
        ));
    }

    /// A focus in the page given last names its node by the unique id the page's read gave it.
    #[test]
    fn a_page_focus_names_its_node() {
        let mut converter = Converter::default();
        let mut root = node("document web", "Sign up", &[]);
        root.path = "/doc".to_owned();
        let mut link = node("link", "terms of use", &[]);
        link.path = "/link".to_owned();
        root.children.push(link);
        let page = AtspiRecord::page(
            0,
            PageRead {
                document: ":1.5/doc".to_owned(),
                base: 1 << 20,
                root,
                read_ms: 0,
            },
        );
        let _ = converter.convert_read(&page);
        let target = element(ElementRole::Link, "terms of use", "/link", true);
        let mut focus = record(EventKind::FocusChanged, target, None);
        focus.document = Some(":1.5/doc".to_owned());
        let expected = ((1 << 20) + 2).to_string();
        assert!(matches!(
            converter.convert_read(&focus).as_slice(),
            [AccessibilityEvent::Focus(f)] if f.unique_id.as_deref() == Some(expected.as_str())
        ));
    }

    /// An option of the page given last is focused with its list's name and its place, as
    /// Windows' focus has them: "Fruits list | with 3 items | Apple | 1 of 3" [obs, 286/s19].
    #[test]
    fn a_page_option_focus_has_its_list_and_place() {
        let mut converter = Converter::default();
        let mut root = node("document web", "Patterns", &[]);
        root.path = "/doc".to_owned();
        let mut list = node("list box", "Fruits", &[]);
        list.path = "/fruits".to_owned();
        for (name, path, at) in [("Apple", "/apple", "1"), ("Apricot", "/apricot", "2")] {
            let mut item = node("list item", name, &[]);
            item.path = path.to_owned();
            item.attributes.insert("posinset".to_owned(), at.to_owned());
            item.attributes.insert("setsize".to_owned(), "2".to_owned());
            list.children.push(item);
        }
        root.children.push(list);
        let page = AtspiRecord::page(
            0,
            PageRead {
                document: ":1.5/doc".to_owned(),
                base: 0,
                root,
                read_ms: 0,
            },
        );
        let _ = converter.convert_read(&page);
        let target = element(ElementRole::ListItem, "Apricot", "/apricot", true);
        let mut focus = record(EventKind::FocusChanged, target, None);
        focus.document = Some(":1.5/doc".to_owned());
        assert!(matches!(
            converter.convert_read(&focus).as_slice(),
            [AccessibilityEvent::Focus(f)]
                if f.container_name.as_deref() == Some("Fruits")
                    && f.container_is_list_box
                    && f.item_index == Some(2)
                    && f.item_count == Some(2)
        ));
    }

    /// A page read again whose log gained a message says the log's change before the page is
    /// refreshed, as Windows' live region event comes before its page [obs, 206/s2]; the first
    /// read, and a read with nothing changed, say no change.
    #[test]
    fn a_live_regions_change_comes_before_its_page() {
        let read = |messages: &[&str]| {
            // The driver's ids, kept from read to read.
            let mut root = node("document web", "Team Chat", &[]);
            root.path = "/doc".to_owned();
            root.id = Some(1);
            let mut log = node("log", "Messages", &[]);
            log.path = "/log".to_owned();
            log.id = Some(2);
            for (key, value) in [
                ("live", "polite"),
                ("container-live", "polite"),
                ("id", "log"),
            ] {
                log.attributes.insert(key.to_owned(), value.to_owned());
            }
            for (i, message) in messages.iter().enumerate() {
                let mut text = node("static", message, &[]);
                text.text = Some((*message).to_owned());
                text.path = format!("/message/{i}");
                text.id = i32::try_from(i).ok().map(|i| 10 + i);
                text.attributes
                    .insert("container-live".to_owned(), "polite".to_owned());
                log.children.push(text);
            }
            root.children.push(log);
            AtspiRecord::page(
                0,
                PageRead {
                    document: ":1.5/doc".to_owned(),
                    base: 0,
                    root,
                    read_ms: 0,
                },
            )
        };
        let mut converter = Converter::default();
        let first = converter.convert_read(&read(&["Ada: Good morning"]));
        // The log read alone as it changes: its change, then the page's.
        let region = |messages: &[&str]| {
            let page = read(messages);
            let root = page
                .pages
                .first()
                .and_then(|p| p.root.children.first().cloned())
                .unwrap_or_default();
            AtspiRecord::live(
                0,
                LiveRead {
                    document: ":1.5/doc".to_owned(),
                    root,
                    cut: false,
                    read_ms: 0,
                },
            )
        };
        assert!(matches!(
            first.as_slice(),
            [AccessibilityEvent::DocumentLoaded { .. }]
        ));
        let second = converter.convert_read(&read(&["Ada: Good morning", "You: Message 1"]));
        assert!(
            matches!(
                second.as_slice(),
                [AccessibilityEvent::LiveRegion(live), AccessibilityEvent::DocumentLoaded { .. }]
                    if live.text.ends_with("You: Message 1")
                        && live.container_name.as_deref() == Some("Messages")
                        && live.region_id.as_deref() == Some("log")
            ),
            "{second:?}"
        );
        let third = converter.convert_read(&read(&["Ada: Good morning", "You: Message 1"]));
        assert!(matches!(
            third.as_slice(),
            [AccessibilityEvent::DocumentLoaded { .. }]
        ));
        let fourth = converter.convert_read(&region(&[
            "Ada: Good morning",
            "You: Message 1",
            "You: Message 2",
        ]));
        assert!(
            matches!(
                fourth.as_slice(),
                [AccessibilityEvent::LiveRegion(live), AccessibilityEvent::PagePartsLoaded(parts)]
                    if live.text.ends_with("You: Message 2")
                        && parts.first().and_then(|p| p.attributes.get(okur_core::aom::UNIQUE_ID_ATTR))
                            == Some(&"2".to_owned())
            ),
            "{fourth:?}"
        );
        // A region of another page than the one given last says nothing, nor captures the
        // page (a background tab's message).
        let mut elsewhere = region(&["Ada: Good morning", "You: Message 3"]);
        if let Some(read) = elsewhere.live.as_mut() {
            read.document = ":1.9/doc".to_owned();
        }
        assert!(converter.convert_read(&elsewhere).is_empty());
        // The page read again after it: nothing new to say of the log.
        let fifth = converter.convert_read(&read(&[
            "Ada: Good morning",
            "You: Message 1",
            "You: Message 2",
        ]));
        assert!(matches!(
            fifth.as_slice(),
            [AccessibilityEvent::DocumentLoaded { .. }]
        ));
    }

    /// A text field read at its caret is the engine's text context, as read.
    #[test]
    fn a_caret_read_is_the_fields_text_context() {
        let context = TextContext {
            text: "sam@example.com".to_owned(),
            caret: 15,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        };
        let read = AtspiRecord::text(
            0,
            TextRead {
                object: ":1.5/email".to_owned(),
                context: context.clone(),
                read_ms: 4,
            },
        );
        let mut converter = Converter::default();
        assert_eq!(
            converter.convert_read(&read),
            vec![AccessibilityEvent::TextContext(context)]
        );
        // And as a trace line, read back the same.
        let line = serde_json::to_string(&read).unwrap_or_default();
        let back: Option<AtspiRecord> = serde_json::from_str(&line).ok();
        assert_eq!(back.and_then(|r| r.text), read.text);
    }

    /// A page's select opened, its expansion reported with its focus gone to its list: the
    /// focused select's change, expanded [obs, 276/s7].
    #[test]
    fn a_selects_expansion_is_its_change() {
        let mut converter = Converter::default();
        let select = element(ElementRole::ComboBox, "Home branch", "/select", true);
        let mut focus = record(EventKind::FocusChanged, select.clone(), None);
        focus.document = Some(":1.5/doc".to_owned());
        let _ = converter.convert_read(&focus);
        let mut unfocused = select;
        unfocused.states.focused = false;
        let opened = record(
            EventKind::StateChanged {
                flag: StateFlag::Expanded,
                value: true,
            },
            unfocused,
            None,
        );
        assert!(matches!(
            converter.convert_read(&opened).as_slice(),
            [AccessibilityEvent::Change { kind: ChangeKind::State, control }]
                if control.states.expanded == Some(true)
        ));
    }

    /// A focus in a window behind the active one is not said.
    #[test]
    fn a_focus_in_a_window_behind_is_not_said() {
        let mut converter = Converter::default();
        let button = element(ElementRole::Button, "OK", "/ok", true);
        let behind = NativeRead {
            focus: node("push button", "OK", &[]),
            ancestors: vec![node("frame", "Chat", &[])],
            read_ms: 0,
        };
        let events = converter.convert_read(&record(EventKind::FocusChanged, button, Some(behind)));
        assert!(events.is_empty());
    }

    /// A label renamed within the focused drop-down is read again and said as its value
    /// [obs, 304/s9: "Brazil"]; the converter asks for the read only in the focus's application.
    #[test]
    fn a_change_within_the_focused_control_is_read_again() {
        let mut converter = Converter::default();
        let country = element(ElementRole::ComboBox, "Country:", "/country", true);
        let shown = |item: &str| {
            let mut combo = node("combo box", "Country:", &[state::FOCUSED]);
            combo
                .attributes
                .insert("valuetext".to_owned(), item.to_owned());
            combo.text = Some(item.to_owned());
            combo
        };
        let _ = converter.convert_read(&record(
            EventKind::FocusChanged,
            country.clone(),
            Some(read(shown("Germany"))),
        ));
        let label = element(ElementRole::StaticText, "Brazil", "/label", false);
        assert!(converter
            .wants_reread(&EventKind::NameChanged, Some(&label))
            .is_some());
        let mut other = element(ElementRole::StaticText, "12:00", "/clock", false);
        other.raw.insert(
            "bus_name".to_owned(),
            serde_json::Value::String(":1.9".to_owned()),
        );
        assert!(converter
            .wants_reread(&EventKind::NameChanged, Some(&other))
            .is_none());
    }

    /// A page read off the event thread comes as a record of its own: the engine is given the
    /// page alone; asked for, it is not asked again for a focus in it.
    #[test]
    fn a_page_read_later_is_its_own_record() {
        let mut converter = Converter::default();
        converter.page_asked(":1.5/doc");
        let link = element(ElementRole::Link, "News", "/link", true);
        assert!(!converter.wants_page(&link, ":1.5/doc"));
        let page = AtspiRecord::page(
            5,
            PageRead {
                document: ":1.5/doc".to_owned(),
                base: 0,
                root: node("document web", "News", &[]),
                read_ms: 40,
            },
        );
        assert!(matches!(
            converter.convert_read(&page).as_slice(),
            [AccessibilityEvent::DocumentLoaded { title, .. }] if title == "News"
        ));
    }

    /// A page read is given to the engine first, numbered from its base; the page read last is
    /// not read again for a focus in it, its document's focus is.
    #[test]
    fn a_page_read_is_given_first_and_remembered() {
        let mut converter = Converter::default();
        let link = element(ElementRole::Link, "News", "/link", true);
        let mut page = record(EventKind::FocusChanged, link, None);
        page.document = Some(":1.5/doc".to_owned());
        page.pages = vec![PageRead {
            document: ":1.5/doc".to_owned(),
            base: 1 << 20,
            root: node("document web", "News", &[]),
            read_ms: 0,
        }];
        let events = converter.convert_read(&page);
        assert!(matches!(
            events.as_slice(),
            [AccessibilityEvent::DocumentLoaded { title, .. }, AccessibilityEvent::Focus(f)]
                if title == "News" && f.in_document
        ));
        let document = element(ElementRole::WebArea, "News", "/doc", true);
        // The page read holds no object but its document: a focus on another is in a page
        // changed since, read again.
        let mut held = node("document web", "News", &[]);
        held.path = "/doc".to_owned();
        let mut known = node("link", "News", &[]);
        known.path = "/link".to_owned();
        held.children.push(known);
        let again = AtspiRecord::page(
            1,
            PageRead {
                document: ":1.5/doc".to_owned(),
                base: 0,
                root: held,
                read_ms: 0,
            },
        );
        let _ = converter.convert_read(&again);
        let link = element(ElementRole::Link, "News", "/link", true);
        let opened = element(ElementRole::Heading, "Add Delivery Address", "/new", true);
        assert!(!converter.wants_page(&link, ":1.5/doc"));
        assert!(converter.wants_page(&document, ":1.5/doc"));
        assert!(converter.wants_page(&link, ":1.5/other"));
        assert!(converter.wants_page(&opened, ":1.5/doc"));
    }
}
