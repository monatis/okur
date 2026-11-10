//! The desktop's accessibility events on Linux: every application's AT-SPI events, through the
//! vendored xa11y's desktop subscription (registered with the AT-SPI registry, without which
//! GTK 4 and Chrome send nothing: the Linux lab's probe 2), as the engine takes them. First the
//! focus: an element taking the keyboard focus is the engine's focus event
//! (`okur_linux_model::focus`); the other kinds follow as the engine's Linux path grows.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use okur_core::event::AccessibilityEvent;
use okur_linux_model::convert::{AtspiRecord, Converter, NativeRead, PageRead};
use okur_linux_model::delivery::Delivery;
use okur_linux_model::focus::{
    ancestor_limit, is_window, window_index, CONTROL_LIMIT, WINDOW_DEPTH,
};
use okur_runtime::events::{EventSink, EventSource};
use okur_runtime::RuntimeError;
use xa11y_core::{ElementData, EventKind, RecvStatus, Role};
use xa11y_linux::{A11yStatus, AtspiObject, AtspiTree, LinuxProvider};

/// The objects of the last pages read, each its application's bus name and its path, by the id
/// the page's node was numbered with ([`okur_linux_model::page::atspi_to_web_from`], depth first
/// from the page's base): what acting on a page's node finds its object by
/// ([`crate::actions::LinuxActions`]).
pub type PageObjects = Arc<Mutex<PageIds>>;

/// Where the records are written, shared: the event thread writes each event's, the actions'
/// worker each text field read at its caret. Each writer holds it while it writes a record and
/// gives the engine what the record says, so the trace has the records in the order the engine
/// took them.
pub type TraceOut = Arc<Mutex<Option<std::io::BufWriter<std::fs::File>>>>;

/// The object with the keyboard focus, as its last focus event named it: the field a caret read
/// reads.
pub type FocusedObject = Arc<Mutex<Option<AtspiObject>>>;

/// The objects of the live regions of the last pages read, by page: their children and text
/// changing tells a region's change, which the region read again says (okur_linux_model::live).
/// Written by the event thread as it gives the engine a page, read by the subscription's filter
/// for every children or text signal.
type LiveObjects = Arc<std::sync::RwLock<Vec<LivePage>>>;

/// A page's live objects: its document's bus name and path, and their paths, each with its
/// region's.
struct LivePage {
    bus: String,
    document: String,
    paths: std::collections::HashMap<String, String>,
    /// Its progress bars' objects, whose value changes are heard.
    bars: std::collections::HashSet<String>,
    /// All its objects: the first of them whose children change is heard, and the page is
    /// captured again once its changes settle, as Windows' page is.
    objects: std::collections::HashSet<String>,
    /// Whether a change among its objects is still to be heard: one is, until the page is read
    /// again (a page changing all the time costs one read of an object per capture).
    armed: std::sync::atomic::AtomicBool,
}

/// A live region heard changing, read once its signals pause: the page's document, when, and
/// the most objects it is read with.
struct DirtyRegion {
    document: AtspiObject,
    due: std::time::Instant,
    limit: usize,
}

/// How long after a live region's first signal it is read: its change's signals (a message's
/// children, then its text) come together, and the region is read once for them.
const LIVE_SETTLE: Duration = Duration::from_millis(40);

/// The most objects a live region is read with.
const LIVE_LIMIT: usize = 2000;

/// The most objects a page's control is read with when its state changed with the focus
/// elsewhere: its own state is what changed, and a menu or a combo box's wrapper read whole for
/// each flip cost a subtree's reads over D-Bus (the code review of 2026-11-10, third pass, 6).
/// A control bigger than this is the page's change instead, which captures the page again.
const CONTROL_STATE_LIMIT: usize = 64;

/// The page in front captured again after it changed, as the engine asks
/// (`ApplicationActions::page_changed`): when first and last asked, and whether at once.
#[derive(Debug, Clone, Copy)]
pub struct PageWanted {
    first: std::time::Instant,
    last: std::time::Instant,
    now: bool,
}

impl PageWanted {
    /// Asked (again): at once, or once the changes settle.
    pub fn ask(wanted: &mut Option<PageWanted>, now: bool) {
        let at = std::time::Instant::now();
        match wanted {
            Some(w) => {
                w.last = at;
                w.now |= now;
            }
            None => {
                *wanted = Some(PageWanted {
                    first: at,
                    last: at,
                    now,
                })
            }
        }
    }

    /// Due: asked at once, or quiet for [`REFRESH_QUIET`], or waiting [`REFRESH_MAX_DELAY`]
    /// (Windows' times: a reply streamed for a minute still refreshes).
    fn is_due(&self) -> bool {
        self.now
            || self.last.elapsed() >= REFRESH_QUIET
            || self.first.elapsed() >= REFRESH_MAX_DELAY
    }
}

/// How long a changed page must be quiet before it is captured again.
const REFRESH_QUIET: Duration = Duration::from_millis(700);

/// The longest a changed page waits for quiet.
const REFRESH_MAX_DELAY: Duration = Duration::from_secs(3);

/// Whether the object at `bus` and `path` is the one the engine has the focus on.
fn is_focused(focused: &FocusedObject, bus: &str, path: &str) -> bool {
    focused
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .is_some_and(|object| object.bus_name == bus && object.path == path)
}

/// Whether `kind` on the object at `bus` and `path` is heard: what the converter takes, a live
/// region's children or text changing, and the focused field's text changing (typed in it).
fn heard(
    live: &LiveObjects,
    focused: &FocusedObject,
    kind: &EventKind,
    bus: &str,
    path: &str,
) -> bool {
    // The focused field's text, changed by typing: read again, as Windows' text events keep
    // the engine's copy current. Unheard, the field was read only at the next command, which
    // spoke the read before it ("t" for "teh quick brown fxo" at Insert+Up, each Ctrl+Left the
    // word before: 567 in gtk7).
    if *kind == EventKind::TextChanged && is_focused(focused, bus, path) {
        return true;
    }
    // A value changing is heard from a read page's progress bars, and from the focused control
    // (a slider moved by its arrows, "129" [obs, 2751/s1]) alone: each heard event costs its
    // object's read, and sliders and scroll bars change anywhere (the code review of 2026-11-09,
    // third pass, 8).
    if *kind == EventKind::ValueChanged {
        if is_focused(focused, bus, path) {
            return true;
        }
        let pages = live
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        return pages
            .iter()
            .any(|page| page.bus == bus && page.bars.contains(path));
    }
    if okur_linux_model::focus_track::is_heard(kind) {
        return true;
    }
    if !matches!(kind, EventKind::StructureChanged | EventKind::TextChanged) {
        return false;
    }
    if live_page_of(live, bus, path).is_some() {
        return true;
    }
    // A read page's content changed outside its live regions (a section opened, results come):
    // its first change heard, and none more until it is read again. Text alone is a field's
    // typing, not the page's.
    *kind == EventKind::StructureChanged && {
        let pages = live
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        pages
            .iter()
            .find(|page| page.bus == bus && page.objects.contains(path))
            .is_some_and(|page| page.armed.swap(false, std::sync::atomic::Ordering::SeqCst))
    }
}

/// The page of `document`'s key heard again from its next change.
fn rearm(live: &LiveObjects, document: &str) {
    let pages = live
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for page in pages
        .iter()
        .filter(|page| format!("{}{}", page.bus, page.document) == document)
    {
        page.armed.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

/// The document key of the read page holding the object at `bus` and `path`.
fn page_of_object(live: &LiveObjects, bus: &str, path: &str) -> Option<String> {
    let pages = live
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pages
        .iter()
        .find(|page| page.bus == bus && page.objects.contains(path))
        .map(|page| format!("{}{}", page.bus, page.document))
}

/// The document of the page whose live region holds the object at `bus` and `path`, and that
/// region's object.
fn live_page_of(live: &LiveObjects, bus: &str, path: &str) -> Option<(AtspiObject, AtspiObject)> {
    let pages = live
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    pages.iter().find_map(|page| {
        let region = page.paths.get(path).filter(|_| page.bus == bus)?;
        Some((
            AtspiObject {
                bus_name: page.bus.clone(),
                path: page.document.clone(),
                role: "document web".to_owned(),
            },
            AtspiObject {
                bus_name: page.bus.clone(),
                path: region.clone(),
                role: String::new(),
            },
        ))
    })
}

/// The object a document's key names (its bus name, then its path).
fn object_of_key(key: &str) -> Option<AtspiObject> {
    let at = key.find('/')?;
    let (bus, path) = key.split_at(at);
    Some(AtspiObject {
        bus_name: bus.to_owned(),
        path: path.to_owned(),
        role: "document web".to_owned(),
    })
}

/// A live region read again as it changed, as a record for the engine: its objects with the
/// ids they have in the page (new ones new), for the engine to splice it into the page and to
/// act on its new objects. Read in parallel, as a page is.
fn read_region(
    provider: &LinuxProvider,
    document: &str,
    region: &AtspiObject,
    objects: &PageObjects,
    limit: usize,
) -> AtspiRecord {
    let started = std::time::Instant::now();
    let tree = provider.atspi_tree_parallel(region, limit);
    let ids = {
        let mut objects = objects
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut ids = Vec::new();
        objects.ids_of(&tree, &mut ids);
        objects.part_read(document, &ids);
        ids
    };
    let mut root = snapshot(tree);
    give_ids(&mut root, &mut ids.iter());
    let objects = count(&root);
    let read_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::debug!(region = %region.path, ms = read_ms, objects, "a live region read again");
    AtspiRecord::live(
        unix_ms(),
        okur_linux_model::convert::LiveRead {
            document: document.to_owned(),
            root,
            cut: objects >= limit,
            read_ms,
        },
    )
}

/// The live regions read off the event thread, as their signals pause: a big region read on
/// it held every focus behind it (the code review of 2026-11-09, second pass, finding 1).
struct RegionReader {
    asks: std::sync::mpsc::Sender<(String, AtspiObject, usize)>,
    reads: std::sync::mpsc::Receiver<AtspiRecord>,
}

impl RegionReader {
    /// The reader, on a thread of its own, which ends once its asker is gone. A region asked
    /// for again before its read began is read once.
    fn start(objects: PageObjects) -> Result<Self, RuntimeError> {
        let provider =
            LinuxProvider::new().map_err(|e| source_error("the region reader's bus", e))?;
        let (asks, asked) = std::sync::mpsc::channel::<(String, AtspiObject, usize)>();
        let (read, reads) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            while let Ok(first) = asked.recv() {
                let mut waiting = vec![first];
                for more in asked.try_iter() {
                    if !waiting.contains(&more) {
                        waiting.push(more);
                    }
                }
                for (document, region, limit) in waiting {
                    if read
                        .send(read_region(&provider, &document, &region, &objects, limit))
                        .is_err()
                    {
                        return;
                    }
                }
            }
        });
        Ok(Self { asks, reads })
    }
}

/// The live objects of the pages `record` gives the engine, in place of those pages' before;
/// of a region read again, added to its page's: a message's own objects, new in it, are heard as
/// they change (a reply streamed into it; the code review of 2026-11-09, second pass, 3).
fn note_live_pages(live: &LiveObjects, record: &AtspiRecord) {
    if let Some(read) = &record.live {
        let objects = okur_linux_model::live::live_objects(&read.root);
        let Some(at) = read.document.find('/') else {
            return;
        };
        let (bus, document) = read.document.split_at(at);
        let mut pages = live
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(page) = pages
            .iter_mut()
            .find(|p| p.bus == bus && p.document == document)
        {
            page.paths.extend(objects);
        }
        return;
    }
    if record.pages.is_empty() {
        return;
    }
    // Walked before the lock is taken: the subscription's filter waits on it (the code review
    // of 2026-11-09).
    let found: Vec<_> = record
        .pages
        .iter()
        .map(|page| {
            (
                page,
                okur_linux_model::live::live_objects(&page.root),
                okur_linux_model::page::progress_bars(&page.root),
                okur_linux_model::page::object_paths(&page.root),
            )
        })
        .collect();
    let mut pages = live
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (page, paths, bars, objects) in found {
        let Some(at) = page.document.find('/') else {
            continue;
        };
        let (bus, document) = page.document.split_at(at);
        pages.retain(|p| !(p.bus == bus && p.document == document));
        pages.push(LivePage {
            bus: bus.to_owned(),
            document: document.to_owned(),
            paths,
            bars,
            objects,
            armed: std::sync::atomic::AtomicBool::new(true),
        });
        if pages.len() > PAGES_KEPT {
            pages.remove(0);
        }
    }
}

/// The element Okur last activated (Enter or Space on a page's check box), and when.
pub type Activated = Arc<Mutex<Option<(AtspiObject, std::time::Instant)>>>;

/// What the event source and the actions share ([`crate::actions::LinuxActions`]): the focused
/// object and the trace, for reading a text field at its caret; the element last activated, for
/// its focus to wait for its state; the page in front wanted again after it changed.
#[derive(Clone, Default)]
pub struct DriverShared {
    pub trace: TraceOut,
    pub focused: FocusedObject,
    pub activated: Activated,
    pub page_wanted: Arc<Mutex<Option<PageWanted>>>,
    /// The actions' worker, which reads a text field at its caret as its focus is delivered.
    pub(crate) field_reads: FieldReads,
}

/// Where the event loop asks the actions' worker for a field's read.
pub(crate) type FieldReads = Arc<Mutex<Option<std::sync::mpsc::Sender<crate::actions::Request>>>>;

/// How long after Okur activated it a toggle's focus is taken as the activation's.
const ACTIVATION_FRESH: Duration = Duration::from_millis(500);

/// How long a toggle's focus waits for the activation to change its state.
const TOGGLE_WAIT: Duration = Duration::from_millis(80);

/// A page's check box, radio button or toggle button focused as Okur activated it: Chrome
/// focuses it before it toggles it (the focus 7 ms after Space, its checked state 15 ms
/// [obs, 229/s8]), and its focus is read with its state once toggled, as Windows reads its focus
/// after the event: "Subscribe check box checked". Polled for at most [`TOGGLE_WAIT`]; an
/// activation that changes nothing (a radio button already checked) leaves it as it was.
fn settle_toggle(provider: &LinuxProvider, activated: &Activated, target: &mut ElementData) {
    use okur_linux_model::page::state;
    let switch = target.role == Role::Switch;
    if !matches!(target.role, Role::CheckBox | Role::RadioButton) && !switch {
        return;
    }
    let Some(object) = AtspiObject::of(target) else {
        return;
    };
    let fresh = {
        let mut last = activated
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let fresh = last.as_ref().is_some_and(|(o, at)| {
            o.bus_name == object.bus_name
                && o.path == object.path
                && at.elapsed() < ACTIVATION_FRESH
        });
        if fresh {
            *last = None;
        }
        fresh
    };
    if !fresh {
        return;
    }
    let bit = |n: u32| 1u64.checked_shl(n).unwrap_or(0);
    let on_bits = if switch {
        bit(state::CHECKED) | bit(state::PRESSED)
    } else {
        bit(state::CHECKED)
    };
    let before = target.states.checked;
    let started = std::time::Instant::now();
    while started.elapsed() < TOGGLE_WAIT {
        let Ok(bits) = provider.atspi_state_bits(&object.bus_name, &object.path) else {
            return;
        };
        // Mixed as xa11y-linux reads it, before checked (the code review of 2026-11-09,
        // second pass, finding 4).
        let now = Some(if bits & bit(state::INDETERMINATE) != 0 {
            xa11y_core::Toggled::Mixed
        } else if bits & on_bits != 0 {
            xa11y_core::Toggled::On
        } else {
            xa11y_core::Toggled::Off
        });
        if now != before {
            tracing::debug!(
                ms = started.elapsed().as_millis(),
                "an activated toggle's focus: its new state"
            );
            target.states.checked = now;
            return;
        }
        std::thread::sleep(Duration::from_millis(4));
    }
}

impl DriverShared {
    /// Writes `record` to the trace, where there is one, and gives the engine `said`: false
    /// once the engine has gone.
    pub fn hand(
        &self,
        record: AtspiRecord,
        said: Vec<AccessibilityEvent>,
        sink: &EventSink,
    ) -> bool {
        hand(&self.trace, record, said, sink)
    }
}

/// Writes `record` to `trace`, where there is one, stamped as given now: what another part of
/// Okur gives the engine itself (okur-ui's messages, which the UI host hands it).
pub(crate) fn trace_only(trace: &TraceOut, mut record: AtspiRecord) {
    let mut out = trace
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(writer) = out.as_mut() {
        record.delivered_ms = Some(unix_ms());
        if let Err(e) = write_record(writer, &record) {
            tracing::warn!("the trace not written, no longer kept: {e}");
            *out = None;
        }
    }
}

/// Writes `record` to `trace`, where there is one, and gives the engine `said`, the trace held
/// between the two, so that its records are in the order the engine took them: false once the
/// engine has gone.
fn hand(
    trace: &TraceOut,
    mut record: AtspiRecord,
    said: Vec<AccessibilityEvent>,
    sink: &EventSink,
) -> bool {
    let mut out = trace
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(writer) = out.as_mut() {
        // When the engine is given it, under the trace's lock: the trace's stamps go forward.
        record.delivered_ms = Some(unix_ms());
        if let Err(e) = write_record(writer, &record) {
            tracing::warn!("the trace not written, no longer kept: {e}");
            *out = None;
        }
    }
    said.into_iter().all(|event| sink.send(event))
}

/// Each object's id, by its application's bus name and its path, kept while its page is one of
/// the last read: a page read again gives an object the id it had, as IA2's unique ids stay on
/// Windows, so a part read again splices into the page the engine has, whichever read that page
/// came from (the code review of 2026-11-09, second pass, findings 2 and 7). The objects of the
/// pages before the last few go, and those that no page kept has any longer.
#[derive(Debug, Default)]
pub struct PageIds {
    /// The id given last.
    last: i32,
    ids: std::collections::HashMap<(String, String), i32>,
    pub by_id: std::collections::BTreeMap<i32, (String, String)>,
    /// The objects of the last pages read, by their documents' keys, the oldest first.
    pages: std::collections::VecDeque<(String, std::collections::HashSet<i32>)>,
}

/// How many pages' objects are kept.
const PAGES_KEPT: usize = 4;

impl PageIds {
    /// The id the object at `bus` and `path` has, if it has one.
    fn known_id(&self, bus: &str, path: &str) -> Option<i32> {
        self.ids.get(&(bus.to_owned(), path.to_owned())).copied()
    }

    /// The id of the object at `bus` and `path`: the one it has, else a new one.
    fn id_of(&mut self, bus: &str, path: &str) -> i32 {
        let key = (bus.to_owned(), path.to_owned());
        if let Some(&id) = self.ids.get(&key) {
            return id;
        }
        // Past i32's end the ids start again at 1, skipping any still kept.
        loop {
            self.last = if self.last >= i32::MAX - 1 {
                1
            } else {
                self.last + 1
            };
            if !self.by_id.contains_key(&self.last) {
                break;
            }
        }
        self.by_id.insert(self.last, key.clone());
        self.ids.insert(key, self.last);
        self.last
    }

    /// `tree`'s objects' ids, depth first, as its snapshot's nodes are ordered.
    fn ids_of(&mut self, tree: &AtspiTree, out: &mut Vec<i32>) {
        out.push(self.id_of(&tree.bus_name, &tree.path));
        for child in &tree.children {
            self.ids_of(child, out);
        }
    }

    /// `document`'s page read whole, its objects `ids`: they are the page's now. The objects of
    /// the pages before the last few go, and those no page kept has.
    fn page_read(&mut self, document: &str, ids: &[i32]) {
        self.pages.retain(|(d, _)| d != document);
        self.pages
            .push_back((document.to_owned(), ids.iter().copied().collect()));
        while self.pages.len() > PAGES_KEPT {
            self.pages.pop_front();
        }
        let kept: std::collections::HashSet<i32> = self
            .pages
            .iter()
            .flat_map(|(_, ids)| ids.iter().copied())
            .collect();
        self.by_id.retain(|id, _| kept.contains(id));
        self.ids.retain(|_, id| kept.contains(id));
    }

    /// A part of `document`'s page read again, its objects `ids`: kept with the page's. A part
    /// of a page not kept keeps nothing; its objects go at the next page read.
    fn part_read(&mut self, document: &str, ids: &[i32]) {
        if let Some((_, page)) = self.pages.iter_mut().find(|(d, _)| d == document) {
            page.extend(ids.iter().copied());
        }
    }
}

/// Gives `node` and its descendants, depth first, the ids `ids` gives in that order: those
/// [`PageIds::ids_of`] gave the tree it is the snapshot of.
fn give_ids(node: &mut okur_linux_model::page::AtspiNode, ids: &mut std::slice::Iter<'_, i32>) {
    node.id = ids.next().copied();
    for child in &mut node.children {
        give_ids(child, ids);
    }
}

/// The most objects a page's capture reads: a large page is cut there, its first part read.
const PAGE_LIMIT: usize = 20_000;

/// xa11y-linux's capture as the model's snapshot node.
pub(crate) fn snapshot(tree: AtspiTree) -> okur_linux_model::page::AtspiNode {
    okur_linux_model::page::AtspiNode {
        role: tree.role,
        name: tree.name,
        attributes: tree.attributes.into_iter().collect(),
        states: tree.states,
        text: tree.text,
        text_attributes: tree.text_attributes.into_iter().collect(),
        path: tree.path,
        relations: tree.relations,
        description: tree.description,
        uri: tree.uri,
        default_action: tree.default_action,
        // Given by give_ids, from the page's ids.
        id: None,
        children: tree.children.into_iter().map(snapshot).collect(),
    }
}

/// How far up from a focused element a page's document is looked for.
const PAGE_DEPTH: usize = 40;

/// `object`'s ancestors from its parent up, to its window at most: the walk a native focus's
/// read takes when no walk was made for it.
fn walk_to_window(provider: &LinuxProvider, object: &AtspiObject) -> Vec<AtspiObject> {
    let mut walked = Vec::new();
    let mut next = provider.atspi_parent(object);
    while let Some(parent) = next {
        if walked.len() >= WINDOW_DEPTH {
            break;
        }
        // An alert may be a part of its window (GTK's info bar): the walk goes on above it, to
        // the window or the application.
        next = if is_window(&parent.role) && parent.role != "alert" {
            None
        } else {
            provider.atspi_parent(&parent)
        };
        walked.push(parent);
    }
    walked
}

/// A focus outside a page read with what surrounds it, for okur-linux-model's `native_focus`:
/// its own subtree, and its ancestors up to its window as the model's read plan has them
/// (`ancestor_limit`). A few calls on the bus per object read; none for an element whose
/// object is not named. `walked`: its ancestors
/// as the look for its page walked them, from its parent up, which the read takes instead of
/// walking them again (the code review of 2026-11-08, finding 16).
fn read_native(
    provider: &LinuxProvider,
    element: &ElementData,
    walked: Option<&[AtspiObject]>,
    dialog_known: &dyn Fn(&str) -> bool,
) -> Option<NativeRead> {
    let Some(object) = AtspiObject::of(element) else {
        tracing::debug!("a native focus without its object's names");
        return None;
    };
    let started = std::time::Instant::now();
    // Read with each object's children at once, as a page is: a GTK list's item cost 130 ms
    // read call by call, its list read whole beside it (37 objects; Okur's own Links List in
    // gtk2), each arrow up to three reads.
    let focus = snapshot(provider.atspi_tree_parallel(&object, CONTROL_LIMIT));
    let walked = match walked {
        Some(walked) => walked.to_vec(),
        None => walk_to_window(provider, &object),
    };
    let mut ancestors = Vec::new();
    let mut window_object = None;
    let window_at = window_index(&walked, |o| o.role.as_str());
    for (index, parent) in walked.into_iter().enumerate() {
        if ancestors.len() >= WINDOW_DEPTH {
            break;
        }
        let limit = ancestor_limit(ancestors.len(), &parent.role, &focus);
        let node = snapshot(provider.atspi_tree_parallel(&parent, limit));
        let window = window_at == Some(index);
        ancestors.push(node);
        // The window is the last read: its own parent, the application, is not read.
        if window {
            window_object = Some(parent);
            break;
        }
    }
    // A dialog box come forward is read whole once, for its buttons and text: not again for
    // each focus in it.
    let dialog = ancestors
        .last()
        .filter(|window| matches!(window.role.as_str(), "dialog" | "alert"))
        .and(window_object)
        .and_then(|window| {
            let key = format!("{}{}", window.bus_name, window.path);
            if dialog_known(&key) {
                return None;
            }
            Some(snapshot(provider.atspi_tree_parallel_within(
                &window,
                DIALOG_LIMIT,
                &okur_linux_model::focus::dialog_reads_within,
            )))
        });
    // What a focus costs, for the review's estimates (2026-11-08) to become numbers.
    let objects = count(&focus) + ancestors.iter().map(count).sum::<usize>();
    tracing::debug!(
        ms = started.elapsed().as_millis(),
        objects,
        focus = %focus.role,
        attributes = ?focus.attributes,
        states = focus.states,
        ancestors = ?ancestors.iter().map(|a| a.role.as_str()).collect::<Vec<_>>(),
        parent = ?ancestors.first().map(|a| &a.attributes),
        window_states = ?ancestors.last().map(|a| a.states),
        "native focus read"
    );
    let read_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    Some(NativeRead {
        focus,
        ancestors,
        read_ms,
        dialog,
    })
}

/// How long after a window comes forward its focus is looked for: GTK places a window's first
/// focus at its first layout (Okur's own dialogs, 1205's research runs).
const WINDOW_FOCUS_WAIT: Duration = Duration::from_millis(150);

/// The most objects of a window looked through for its focus.
const WINDOW_FOCUS_LIMIT: usize = 400;

/// A window come forward whose focus is to be looked for, and when.
struct WindowFocus {
    window: AtspiObject,
    app: String,
    pid: u32,
    due: std::time::Instant,
}

/// The focus within a window come forward, as the focus event GTK did not send: the desktop
/// fixtures' first control, Okur's own dialogs' (the plan's open item: a window's focus said as
/// it comes forward; 548/s1 in full5, Insert+Tab silent there).
fn focus_within(provider: &LinuxProvider, wanted: WindowFocus) -> Option<xa11y_core::Event> {
    let object = provider.atspi_focused_within(&wanted.window, WINDOW_FOCUS_LIMIT)?;
    let element = provider.atspi_element(&object)?;
    tracing::debug!(
        window = %wanted.window.path,
        focus = ?element.name,
        "a window's focus found as it came forward"
    );
    Some(
        xa11y_core::EventParts {
            kind: EventKind::FocusChanged,
            target: Some(element),
            app_name: wanted.app,
            app_pid: wanted.pid,
            timestamp: std::time::Instant::now(),
        }
        .into(),
    )
}

/// The most objects a dialog box's window is read with, once, for its buttons and text.
const DIALOG_LIMIT: usize = 300;

/// The objects in `node`'s tree, itself included.
fn count(node: &okur_linux_model::page::AtspiNode) -> usize {
    1 + node.children.iter().map(count).sum::<usize>()
}

/// The page's document the focused element is in, or is.
fn document_of(provider: &LinuxProvider, element: &ElementData) -> Option<AtspiObject> {
    look_for_page(provider, element).0
}

/// The page's document the element is in, or is, and the ancestors walked to look for it, from
/// its parent up: to the document where there is one, else as far as the walk went, a native
/// control's ancestors, which its read then takes.
fn look_for_page(
    provider: &LinuxProvider,
    element: &ElementData,
) -> (Option<AtspiObject>, Vec<AtspiObject>) {
    let Some(object) = AtspiObject::of(element) else {
        return (None, Vec::new());
    };
    if element.role == Role::WebArea {
        return (Some(object), Vec::new());
    }
    let mut walked: Vec<AtspiObject> = Vec::new();
    for _ in 0..=PAGE_DEPTH {
        let current = walked.last().unwrap_or(&object);
        if current.role == "document web" {
            return (Some(current.clone()), walked);
        }
        match provider.atspi_parent(current) {
            Some(parent) => walked.push(parent),
            None => break,
        }
    }
    (None, walked)
}

/// An AT-SPI object's key, its bus name and path.
fn key_of(object: &AtspiObject) -> String {
    format!("{}{}", object.bus_name, object.path)
}

/// Whether `document` is a page in view: a document shown, not a frame's inside another page
/// (each frame's document loads too), nor a hidden tab's.
fn shown_page(provider: &LinuxProvider, document: &ElementData) -> bool {
    // The parent by the document's object: an event's element is in no provider's handle cache,
    // so get_parent never found one and every frame's document passed (the code review of
    // 2026-11-08).
    document.role == Role::WebArea
        && document.states.visible
        && AtspiObject::of(document)
            .and_then(|object| provider.atspi_parent(&object))
            .and_then(|parent| provider.atspi_ancestor(&parent, "document web", PAGE_DEPTH))
            .is_none()
}

/// A page read whole, as the engine takes a page, its objects numbered apart from the pages
/// before it, so an action finds this page's object even after another page was read. Read in
/// parallel: 3 to 3.5 times as fast as in order, the same tree (probe-read-speed.sh in the lab
/// at ebc4fd11: color-grid's 433 objects in 419 to 524 ms against 1399 to 1597).
fn read_page(provider: &LinuxProvider, document: &AtspiObject, objects: &PageObjects) -> PageRead {
    read_page_until(
        provider,
        document,
        objects,
        &std::sync::atomic::AtomicBool::new(false),
    )
}

/// [`read_page`], cut short once `stop` is set.
fn read_page_until(
    provider: &LinuxProvider,
    document: &AtspiObject,
    objects: &PageObjects,
    stop: &AtomicBool,
) -> PageRead {
    let started = std::time::Instant::now();
    let tree = provider.atspi_tree_parallel_until(document, PAGE_LIMIT, stop);
    // A lock a panicking thread left still holds the ids (the second review). A read stopped
    // gives its new objects ids but does not make them the page's: they go at the next read.
    let ids = {
        let mut objects = objects
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut ids = Vec::new();
        objects.ids_of(&tree, &mut ids);
        if !stop.load(Ordering::SeqCst) {
            objects.page_read(&key_of(document), &ids);
        }
        ids
    };
    let mut root = snapshot(tree);
    give_ids(&mut root, &mut ids.iter());
    let read_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::debug!(ms = read_ms, objects = count(&root), "page read");
    // Its objects carry their ids: the base numbers nothing.
    PageRead {
        document: key_of(document),
        base: 0,
        read_ms,
        root,
    }
}

fn source_error(what: &str, error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::EventSource(format!("AT-SPI: {what}: {error}"))
}

/// Pages read off the event thread (the code review of 2026-11-08, finding 6). A page is read
/// whole at about 3 ms an object: on the event thread, its focus and every event after it
/// waited for the read. The reader takes the documents asked of it, the latest of those
/// waiting (the focus moved on from the others), on a bus connection of its own, and hands
/// each page back; the event thread gives the engine its focus at once and the page when it
/// comes (`okur-lab live-replay linux --pages-after-focus` said the same as the runs, 418 of
/// 418 steps, before this was built).
/// The page being read, by its document, and what stops its read.
type Reading = Arc<Mutex<Option<(String, Arc<AtomicBool>)>>>;

struct PageReader {
    asks: std::sync::mpsc::Sender<Ask>,
    pages: std::sync::mpsc::Receiver<PageRead>,
    /// The page being read, and what stops its read (the second review, finding 5).
    reading: Reading,
}

impl PageReader {
    /// The read of `document`'s page stopped, where it is the one being read: nothing more of
    /// it is read, and it is not handed back.
    fn stop_reading(&self, document: &str) {
        if let Ok(reading) = self.reading.lock() {
            if let Some((_, stop)) = reading.as_ref().filter(|(key, _)| key == document) {
                stop.store(true, Ordering::SeqCst);
            }
        }
    }
}

/// What the page reader is asked.
enum Ask {
    Read(AtspiObject),
    /// The page of this document no longer wanted (the focus moved on before it was read).
    Cancel(String),
}

impl PageReader {
    /// The reader, on a thread of its own, which ends once its asker is gone and the read in
    /// hand is done. It reads the pages asked of it in their order, each once, and skips those
    /// cancelled before their read began.
    fn start(objects: PageObjects) -> Result<Self, RuntimeError> {
        let provider =
            LinuxProvider::new().map_err(|e| source_error("the page reader's bus", e))?;
        let (asks, asked) = std::sync::mpsc::channel::<Ask>();
        let (read, pages) = std::sync::mpsc::channel();
        let reading: Reading = Arc::default();
        let now_reading = Arc::clone(&reading);
        std::thread::spawn(move || {
            let mut waiting: std::collections::VecDeque<AtspiObject> = Default::default();
            let take = |ask: Ask, waiting: &mut std::collections::VecDeque<AtspiObject>| match ask {
                Ask::Read(document) => {
                    let key = key_of(&document);
                    if !waiting.iter().any(|w| key_of(w) == key) {
                        waiting.push_back(document);
                    }
                }
                Ask::Cancel(key) => waiting.retain(|w| key_of(w) != key),
            };
            while let Ok(ask) = asked.recv() {
                take(ask, &mut waiting);
                loop {
                    while let Ok(more) = asked.try_recv() {
                        take(more, &mut waiting);
                    }
                    let Some(document) = waiting.pop_front() else {
                        break;
                    };
                    let stop = Arc::new(AtomicBool::new(false));
                    if let Ok(mut reading) = now_reading.lock() {
                        *reading = Some((key_of(&document), Arc::clone(&stop)));
                    }
                    let page = read_page_until(&provider, &document, &objects, &stop);
                    if let Ok(mut reading) = now_reading.lock() {
                        *reading = None;
                    }
                    if stop.load(Ordering::SeqCst) {
                        tracing::debug!(document = %page.document, "page read stopped");
                        continue;
                    }
                    if read.send(page).is_err() {
                        return;
                    }
                }
            }
        });
        Ok(Self {
            asks,
            pages,
            reading,
        })
    }
}

/// A page asked for, before the record that asks for it in `delivery`: of the page reader where
/// it runs (a reader gone is said, and pages are read here from then on), else read here at
/// once. Once per page while it is awaited.
#[allow(clippy::too_many_arguments)]
fn ask_page(
    document: &AtspiObject,
    at_ms: u64,
    reader: &mut Option<PageReader>,
    delivery: &mut Delivery,
    converter: &mut Converter,
    provider: &LinuxProvider,
    objects: &PageObjects,
) {
    let key = key_of(document);
    if delivery.is_awaited(&key) {
        return;
    }
    delivery.ask(key.clone(), at_ms);
    converter.page_asked(&key);
    let sent = reader
        .as_ref()
        .is_some_and(|r| r.asks.send(Ask::Read(document.clone())).is_ok());
    if !sent {
        if reader.take().is_some() {
            tracing::warn!("the page reader is gone: pages are read on the event thread");
        }
        delivery.page_read(read_page(provider, document, objects));
    }
}

/// The records the engine may take now, each traced and converted: false once the engine is
/// gone.
fn deliver(
    delivery: &mut Delivery,
    converter: &mut Converter,
    trace: &TraceOut,
    live: &LiveObjects,
    (focused, field_reads): (&FocusedObject, &FieldReads),
    sink: &EventSink,
) -> bool {
    for record in delivery.take_ready() {
        note_live_pages(live, &record);
        // The focused object as the engine is given its focus: a caret read reads the field
        // the engine knows has the focus, not one whose focus waits for its page (the code
        // review of 2026-11-09, finding 4).
        if let Some(object) = record
            .target
            .as_ref()
            .filter(|e| record.kind == EventKind::FocusChanged && e.states.focused)
            .and_then(AtspiObject::of)
        {
            *focused
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(object);
        }
        let said = converter.convert_read(&record);
        // An editable control focused: read at its caret once the engine has its focus, as
        // Windows' focus carries the field's text.
        let field = record.kind == EventKind::FocusChanged
            && record
                .target
                .as_ref()
                .is_some_and(|e| e.states.focused && e.states.editable);
        let in_page = record.document.is_some();
        if !hand(trace, record, said, sink) {
            return false;
        }
        if field {
            if let Some(reads) = field_reads
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .as_ref()
            {
                let _ = reads.send(crate::actions::Request::FieldFocused(in_page));
            }
        }
    }
    true
}

/// Pages given up (the focus left them, or they took too long): the reader skips them, and the
/// converter asks again when the focus comes back.
fn drop_pages(documents: Vec<String>, reader: &Option<PageReader>, converter: &mut Converter) {
    for document in documents {
        tracing::debug!(%document, "page given up before the engine had it");
        converter.page_dropped(&document);
        if let Some(reader) = reader {
            reader.stop_reading(&document);
            let _ = reader.asks.send(Ask::Cancel(document));
        }
    }
}

/// Milliseconds since the Unix epoch: a record's time, on the clock the lab's runner stamps its
/// steps with, so a replay finds each step's records.
pub(crate) fn unix_ms() -> u64 {
    // The wall clock read once, then a steady clock from it: a step of the system's clock (WSL
    // sets it as the machine wakes) would have put a record before the one before it, and the
    // replay refuses such a trace (the code review of 2026-11-09, second pass, finding 8).
    static ANCHOR: std::sync::OnceLock<(u64, std::time::Instant)> = std::sync::OnceLock::new();
    let (wall, at) = ANCHOR.get_or_init(|| {
        let wall = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0);
        (wall, std::time::Instant::now())
    });
    wall.saturating_add(u64::try_from(at.elapsed().as_millis()).unwrap_or(u64::MAX))
}

/// One record, a JSON line, written through at once: a run stopped by its runner keeps every
/// record before it.
fn write_record(
    out: &mut std::io::BufWriter<std::fs::File>,
    record: &AtspiRecord,
) -> std::io::Result<()> {
    use std::io::Write;
    serde_json::to_writer(&mut *out, record)?;
    out.write_all(b"\n")?;
    out.flush()
}

/// The page whose address ends with `address_end`, read whole as Okur reads a page, as Chrome
/// gives the focus to it, for the Linux lab's snapshots (`examples/atspi_snapshot.rs`, the
/// plan's §2.3), with the time reading it took; an error if no focus reached it in `wait`.
/// Start it before Chrome.
pub fn capture_page(
    address_end: &str,
    wait: Duration,
) -> Result<(okur_linux_model::page::AtspiNode, Duration), RuntimeError> {
    capture_page_with(address_end, wait, false)
}

/// [`capture_page`] once the page's ARIA-AT setup ran: its "Run Test Setup" button activated
/// through AT-SPI, as the Windows capture's keys press it (capture-snapshot.ps1 -Keys), and the
/// page read once its script settled (a menu opened, a box checked). The Linux snapshots of
/// ARIA-AT's pages held their state before it (2026-11-10: the menu button's menu closed where
/// IA2's snapshot has it open). A page without the button is read as it is.
pub fn capture_set_up_page(
    address_end: &str,
    wait: Duration,
) -> Result<(okur_linux_model::page::AtspiNode, Duration), RuntimeError> {
    capture_page_with(address_end, wait, true)
}

/// How long a page's setup script is given before the page is read.
const SETUP_SETTLE: Duration = Duration::from_millis(1500);

/// The page's "Run Test Setup" button, ARIA-AT's.
fn setup_button(tree: &AtspiTree) -> Option<&AtspiTree> {
    if tree.role == "push button" && tree.name.trim() == "Run Test Setup" {
        return Some(tree);
    }
    tree.children.iter().find_map(setup_button)
}

fn capture_page_with(
    address_end: &str,
    wait: Duration,
    setup: bool,
) -> Result<(okur_linux_model::page::AtspiNode, Duration), RuntimeError> {
    let provider = LinuxProvider::new().map_err(|e| source_error("the bus", e))?;
    let subscription = provider
        .subscribe_desktop_kinds(Box::new(|kind, _, _| {
            okur_linux_model::focus_track::is_heard(kind)
        }))
        .map_err(|e| source_error("the desktop's events", e))?;
    let started = std::time::Instant::now();
    while started.elapsed() < wait {
        let RecvStatus::Event(event) = subscription.recv_status(Duration::from_millis(200)) else {
            continue;
        };
        if !matches!(
            event.kind,
            EventKind::FocusChanged | EventKind::DocumentLoaded
        ) {
            continue;
        }
        let Some(document) = event
            .target
            .as_ref()
            .and_then(|e| document_of(&provider, e))
        else {
            continue;
        };
        let address = provider.atspi_tree(&document, 1).uri;
        if !address.is_some_and(|a| a.ends_with(address_end)) {
            continue;
        }
        if setup {
            let tree = provider.atspi_tree(&document, PAGE_LIMIT);
            if let Some(button) = setup_button(&tree) {
                if let Err(error) = provider.atspi_do_action(&button.bus_name, &button.path, 0) {
                    tracing::warn!("the page's setup not run: {error}");
                }
                std::thread::sleep(SETUP_SETTLE);
            }
        }
        let reading = std::time::Instant::now();
        let tree = provider.atspi_tree(&document, PAGE_LIMIT);
        return Ok((snapshot(tree), reading.elapsed()));
    }
    Err(source_error(
        "the page",
        format!("no focus reached a page at …{address_end} in {wait:?}"),
    ))
}

/// Every application's accessibility events, from the AT-SPI bus.
#[derive(Default)]
pub struct AtspiEventSource {
    running: Option<Running>,
    objects: PageObjects,
    /// The desktop's accessibility status before the source told it a screen reader runs.
    status_before: Option<A11yStatus>,
    /// Where each event's record is written, one JSON line each, for the run to be replayed.
    trace: Option<std::path::PathBuf>,
    /// The trace's writer and the focused object, shared with the actions' caret reads.
    shared: DriverShared,
}

struct Running {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<()>,
}

impl AtspiEventSource {
    pub fn new() -> Self {
        Self::default()
    }

    /// The source writing what it read for each event, an [`AtspiRecord`] a line, to `path`:
    /// the Linux lab's live runs keep it, and `okur-lab live-replay linux` replays it through
    /// the converter and the engine with no Linux.
    pub fn with_trace(path: std::path::PathBuf) -> Self {
        let mut source = Self::default();
        source.trace = Some(path);
        source
    }

    /// The objects of the page it read last, for acting on the page's nodes.
    pub fn objects(&self) -> PageObjects {
        Arc::clone(&self.objects)
    }

    /// The focused object and the trace, for reading a text field at its caret.
    pub fn shared(&self) -> DriverShared {
        self.shared.clone()
    }
}

impl EventSource for AtspiEventSource {
    fn start(&mut self, sink: EventSink) -> Result<(), RuntimeError> {
        if self.running.is_some() {
            return Ok(());
        }
        // The desktop told a screen reader runs, as Orca tells it: Qt, Firefox and Chromium
        // turn their AT-SPI bridges on by it. Put back as it was when the source stops.
        self.status_before = A11yStatus::read()
            .map_err(|e| tracing::warn!("the desktop's accessibility status not read: {e}"))
            .ok();
        if let Err(e) = (A11yStatus {
            screen_reader_enabled: true,
            is_enabled: true,
        })
        .write()
        {
            tracing::warn!("the desktop not told a screen reader runs: {e}");
        }
        let provider = LinuxProvider::new().map_err(|e| source_error("the bus", e))?;
        let live: LiveObjects = Arc::default();
        let subscription = {
            let live = Arc::clone(&live);
            let focused = Arc::clone(&self.shared.focused);
            provider
                .subscribe_desktop_kinds(Box::new(move |kind, bus, path| {
                    heard(&live, &focused, kind, bus, path)
                }))
                .map_err(|e| source_error("the desktop's events", e))?
        };
        tracing::debug!("AT-SPI: the desktop's events subscribed");
        if let Some(path) = &self.trace {
            let file = std::fs::File::create(path).map_err(|e| source_error("the trace", e))?;
            *self
                .shared
                .trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                Some(std::io::BufWriter::new(file));
        }
        let trace = Arc::clone(&self.shared.trace);
        let focused = Arc::clone(&self.shared.focused);
        let activated = Arc::clone(&self.shared.activated);
        let field_reads = Arc::clone(&self.shared.field_reads);
        let page_wanted = Arc::clone(&self.shared.page_wanted);
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            let objects = Arc::clone(&self.objects);
            std::thread::spawn(move || {
                // The subscription lives in this thread: dropped here, it deregisters its
                // events and ends its own reader. The converter decides what each event says,
                // from what is read for it here (okur_linux_model::convert).
                let mut converter = Converter::default();
                let objects = objects;
                // Pages read off this thread; read here, as before, when the reader has no bus.
                let mut reader = PageReader::start(Arc::clone(&objects))
                    .map_err(|e| tracing::warn!("pages read on the event thread: {e}"))
                    .ok();
                // The order the engine takes the records and pages in: a page before the
                // focus that asked for it, as when pages were read here.
                let mut delivery = Delivery::default();
                // Live regions read off this thread.
                let regions = RegionReader::start(Arc::clone(&objects))
                    .map_err(|e| tracing::warn!("live regions not read: {e}"))
                    .ok();
                // The page the focus is in, by its document's key: the live regions heard.
                let mut focus_page: Option<String> = None;
                // Live regions heard changing, read once their signals pause, by their objects.
                let mut dirty: std::collections::HashMap<AtspiObject, DirtyRegion> =
                    std::collections::HashMap::new();
                // The read pages that changed while the focus was away from them, by document.
                let mut changed_away: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                // Whether each application is Chromium's, by its bus name: its menus open in
                // windows of their own, read as they come forward.
                // A window come forward whose focus GTK did not report: looked for in it once
                // its first layout is done, unless a focus came meanwhile.
                let mut window_focus: Option<WindowFocus> = None;
                // A focus found so, handled as the event GTK did not send.
                let mut found_focus: Option<Box<xa11y_core::Event>> = None;
                let mut chromium: std::collections::HashMap<String, bool> =
                    std::collections::HashMap::new();
                while !stop.load(Ordering::SeqCst) {
                    // The pages the reader read, into their places; what waited for them, and
                    // for pages waited for too long, goes on.
                    for page in reader.iter().flat_map(|r| r.pages.try_iter()) {
                        let document = page.document.clone();
                        if !delivery.page_read(page) {
                            tracing::debug!(%document, "page read after it was given up");
                        }
                    }
                    if delivery.is_holding() {
                        drop_pages(delivery.expire(unix_ms()), &reader, &mut converter);
                    }
                    if !deliver(
                        &mut delivery,
                        &mut converter,
                        &trace,
                        &live,
                        (&focused, &field_reads),
                        &sink,
                    ) {
                        return;
                    }
                    // Short while a page is awaited, so that it comes at once when the bus is
                    // quiet; else the thread wakes ten times a second, not fifty (the second
                    // review, finding 8).
                    // The live regions whose signals paused, read; the page in front, captured
                    // again once its changes settled, as the engine asked.
                    // In the page's order: a chat's log, then the status below it saying the
                    // message went, as JAWS says them [obs, 206/s3, 960/s3, 961/s3]. Chrome
                    // signals the status's text first, though the page changed the log first
                    // (gtk3's runs, ordered by their first signals): the regions' objects' ids
                    // follow the page's order as it was read, then a region found later.
                    let mut due: Vec<(i32, std::time::Instant, AtspiObject)> = {
                        let ids = objects
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        dirty
                            .iter()
                            .filter(|(_, d)| d.due <= std::time::Instant::now())
                            .map(|(region, d)| {
                                let id = ids
                                    .known_id(&region.bus_name, &region.path)
                                    .unwrap_or(i32::MAX);
                                (id, d.due, region.clone())
                            })
                            .collect()
                    };
                    due.sort_by_key(|(id, at, _)| (*id, *at));
                    let due = due.into_iter().map(|(_, _, region)| region);
                    for region in due {
                        if let (Some(d), Some(regions)) = (dirty.remove(&region), regions.as_ref())
                        {
                            let _ = regions.asks.send((key_of(&d.document), region, d.limit));
                        }
                    }
                    // The regions read, given to the engine where the focus is still in their
                    // page (the code review of 2026-11-09, second pass, finding 6).
                    for record in regions.iter().flat_map(|r| r.reads.try_iter()) {
                        let in_front = record
                            .live
                            .as_ref()
                            .is_some_and(|read| focus_page.as_deref() == Some(&read.document));
                        if in_front {
                            let cancelled = delivery.push(record, false);
                            drop_pages(cancelled, &reader, &mut converter);
                        }
                    }
                    let wanted = {
                        let mut wanted = page_wanted
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        match *wanted {
                            Some(w) if w.is_due() => wanted.take(),
                            _ => None,
                        }
                    };
                    if let (Some(_), Some(document)) =
                        (wanted, focus_page.as_deref().and_then(object_of_key))
                    {
                        tracing::debug!(document = %key_of(&document), "the changed page read again");
                        // Its next change heard from now, not from when the read comes: a
                        // read dropped or cut short would have left it unheard.
                        rearm(&live, &key_of(&document));
                        ask_page(
                            &document,
                            unix_ms(),
                            &mut reader,
                            &mut delivery,
                            &mut converter,
                            &provider,
                            &objects,
                        );
                    }
                    if !deliver(
                        &mut delivery,
                        &mut converter,
                        &trace,
                        &live,
                        (&focused, &field_reads),
                        &sink,
                    ) {
                        return;
                    }
                    let waiting = !dirty.is_empty()
                        || page_wanted
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .is_some();
                    let wait = if delivery.is_holding() || waiting || window_focus.is_some() {
                        20
                    } else {
                        100
                    };
                    // A window's focus looked for, once due.
                    if window_focus
                        .as_ref()
                        .is_some_and(|w| w.due <= std::time::Instant::now())
                    {
                        found_focus = window_focus
                            .take()
                            .and_then(|w| focus_within(&provider, w))
                            .map(Box::new);
                    }
                    let status = match found_focus.take() {
                        Some(event) => RecvStatus::Event(event),
                        None => subscription.recv_status(Duration::from_millis(wait)),
                    };
                    match status {
                        RecvStatus::Event(event) => {
                            // A focus come: no window's focus is looked for.
                            if event.kind == EventKind::FocusChanged {
                                window_focus = None;
                            }
                            tracing::trace!(
                                kind = ?event.kind,
                                role = ?event.target.as_ref().map(|e| e.role),
                                name = ?event.target.as_ref().and_then(|e| e.name.as_deref()),
                                "AT-SPI event"
                            );
                            let target = event.target.as_ref();
                            let at_ms = unix_ms();
                            // GTK 4's second focus event for one move, passed over before
                            // anything is read (and traced bare, for the replay to pass it
                            // over too).
                            if converter.is_twin(&event.kind, target, at_ms) {
                                tracing::debug!("AT-SPI focus's twin");
                                let twin = AtspiRecord::bare(
                                    at_ms,
                                    event.kind.clone(),
                                    event.app_name.clone(),
                                    event.target.clone(),
                                );
                                // In its place, cancelling no page: it was not read.
                                let _ = delivery.push(twin, false);
                                if !deliver(
                                    &mut delivery,
                                    &mut converter,
                                    &trace,
                                    &live,
                                    (&focused, &field_reads),
                                    &sink,
                                ) {
                                    return;
                                }
                                continue;
                            }
                            // The focused field's text changed (typed in it): read again at its
                            // caret, silently, as its focus's read is; reads asked meanwhile
                            // give way to the newest.
                            let typed = event.kind == EventKind::TextChanged
                                && target.and_then(AtspiObject::of).is_some_and(|object| {
                                    is_focused(&focused, &object.bus_name, &object.path)
                                });
                            if typed {
                                if let Some(reads) = field_reads
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                                    .as_ref()
                                {
                                    let _ = reads.send(crate::actions::Request::FieldFocused(
                                        focus_page.is_some(),
                                    ));
                                }
                                continue;
                            }
                            // A live region's children or text changed: the region is read
                            // again once its signals pause, and the converter says what it
                            // changed (the code review of 2026-11-09, finding 2: the whole page
                            // was read for each change).
                            if matches!(
                                event.kind,
                                EventKind::StructureChanged | EventKind::TextChanged
                            ) {
                                // The page the focus is in alone: another page read again
                                // would be taken for a new load and read out over what the
                                // user is doing (the code review of 2026-11-09, finding 1).
                                let page = target
                                    .and_then(AtspiObject::of)
                                    .and_then(|object| {
                                        live_page_of(&live, &object.bus_name, &object.path)
                                    })
                                    .filter(|(document, _)| {
                                        focus_page.as_deref() == Some(key_of(document).as_str())
                                    });
                                // The page in front changed outside its live regions: it is
                                // captured again once its changes settle (it was not, and
                                // what a page added later stayed unread until the focus
                                // moved).
                                // A page changed while the focus is away from it (in Okur's
                                // dialog, the address bar, another program) is asked for as the
                                // focus comes back to it (the code review of 2026-11-09, fifth
                                // pass, 3: it was left unheard for good).
                                if page.is_none() {
                                    let changed =
                                        target.and_then(AtspiObject::of).and_then(|object| {
                                            page_of_object(&live, &object.bus_name, &object.path)
                                        });
                                    match changed {
                                        Some(document)
                                            if focus_page.as_deref() == Some(document.as_str()) =>
                                        {
                                            let mut wanted = page_wanted
                                                .lock()
                                                .unwrap_or_else(std::sync::PoisonError::into_inner);
                                            PageWanted::ask(&mut wanted, false);
                                        }
                                        Some(document) => {
                                            changed_away.insert(document);
                                        }
                                        None => {}
                                    }
                                }
                                if let Some((document, region)) = page {
                                    // One time for the page's regions changing together, so
                                    // they are read together in the page's order, however far
                                    // apart their first signals came (the code review of
                                    // 2026-11-09, fourth pass, 6).
                                    let due = dirty
                                        .values()
                                        .filter(|d| d.document == document)
                                        .map(|d| d.due)
                                        .min()
                                        .unwrap_or_else(|| std::time::Instant::now() + LIVE_SETTLE);
                                    dirty.entry(region).or_insert_with(|| DirtyRegion {
                                        document,
                                        due,
                                        limit: LIVE_LIMIT,
                                    });
                                }
                                continue;
                            }
                            // A page's control checked or expanded while the focus is
                            // elsewhere (a script's switch turned on, a section opened by
                            // another button): read again once its signals pause and spliced
                            // into the engine's page by its id, as a live region is. Nothing
                            // read it, and the virtual cursor said its old state (ARIA-AT's
                            // switch plans on Linux, 2278/s1: "Off" for JAWS's "On"). A
                            // control in a live region is the region's to read.
                            let page_state = matches!(
                                event.kind,
                                EventKind::StateChanged {
                                    flag: xa11y_core::StateFlag::Checked
                                        | xa11y_core::StateFlag::Expanded,
                                    ..
                                }
                            );
                            // One changed while the focus is away from its page (in the address
                            // bar, in Okur's dialog) is that page's change, read as the focus
                            // comes back to it (the code review of 2026-11-10, third pass, 6).
                            if let Some(object) = target
                                .filter(|e| page_state && !e.states.focused)
                                .and_then(AtspiObject::of)
                            {
                                let page = page_of_object(&live, &object.bus_name, &object.path);
                                let in_region =
                                    live_page_of(&live, &object.bus_name, &object.path).is_some();
                                match page {
                                    Some(key) if focus_page.as_deref() == Some(key.as_str()) => {
                                        if let (Some(document), false) =
                                            (object_of_key(&key), in_region)
                                        {
                                            let due = std::time::Instant::now() + LIVE_SETTLE;
                                            dirty.entry(object).or_insert_with(|| DirtyRegion {
                                                document,
                                                due,
                                                limit: CONTROL_STATE_LIMIT,
                                            });
                                        }
                                    }
                                    Some(key) => {
                                        changed_away.insert(key);
                                    }
                                    None => {}
                                }
                            }
                            let focus = event.kind == EventKind::FocusChanged;
                            let changed = okur_linux_model::focus_track::is_control_change(
                                &event.kind,
                                target.is_some_and(|e| e.states.focused),
                            );
                            // The focus's page, looked up once: read when it is new to the
                            // engine, and telling it the focus is in it; a focused control's
                            // change is told the same way, whether it is on a page.
                            let (document, walked) = match target.filter(|_| focus || changed) {
                                Some(element) => look_for_page(&provider, element),
                                None => (None, Vec::new()),
                            };
                            if focus && target.is_some_and(|e| e.states.focused) {
                                focus_page = document.as_ref().map(key_of);
                                // Back on a page that changed meanwhile: captured again.
                                if focus_page
                                    .as_ref()
                                    .is_some_and(|page| changed_away.remove(page))
                                {
                                    let mut wanted = page_wanted
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                                    PageWanted::ask(&mut wanted, false);
                                }
                            }
                            if let (true, Some(element), Some(document)) =
                                (focus, target, document.as_ref())
                            {
                                if converter.wants_page(element, &key_of(document)) {
                                    ask_page(
                                        document,
                                        at_ms,
                                        &mut reader,
                                        &mut delivery,
                                        &mut converter,
                                        &provider,
                                        &objects,
                                    );
                                }
                            }
                            // A page that finished loading, a reload's too, is read again: the
                            // engine takes the page it already has as a refresh, and as a new
                            // load after F5 or Ctrl+R (the lab's live run of 712).
                            if event.kind == EventKind::DocumentLoaded {
                                if let Some(document) = target
                                    .filter(|d| shown_page(&provider, d))
                                    .and_then(AtspiObject::of)
                                {
                                    ask_page(
                                        &document,
                                        at_ms,
                                        &mut reader,
                                        &mut delivery,
                                        &mut converter,
                                        &provider,
                                        &objects,
                                    );
                                }
                            }
                            // A page's toggle Okur activated: its focus with its state once
                            // toggled.
                            let mut snapshot = event.target.clone();
                            if let (true, true, Some(element)) =
                                (focus, document.is_some(), snapshot.as_mut())
                            {
                                settle_toggle(&provider, &activated, element);
                            }
                            // A focus and a control's change read with the ancestors walked
                            // for its page, a window alone, anything else (a menu's item
                            // highlighted) with its own up to its window.
                            let walk = if Converter::reads_alone(&event.kind) {
                                Some(&[][..])
                            } else if focus || changed {
                                Some(walked.as_slice())
                            } else {
                                None
                            };
                            // A window coming forward is read for a menu in Chromium's alone:
                            // GTK's dialog window cost 295 ms to read, its 66 objects (gtk2).
                            let menu_window = event.kind == EventKind::WindowActivated
                                && target
                                    .and_then(|e| e.raw.get("bus_name"))
                                    .and_then(|b| b.as_str())
                                    .is_some_and(|bus| {
                                        if let Some(&known) = chromium.get(bus) {
                                            return known;
                                        }
                                        // A lookup that failed (the application starting)
                                        // is asked again next time, not kept as no (the code
                                        // review of 2026-11-09, third pass, 5).
                                        let Some(toolkit) = provider.atspi_toolkit_name(bus) else {
                                            return false;
                                        };
                                        let known = toolkit == "Chromium";
                                        chromium.insert(bus.to_owned(), known);
                                        known
                                    });
                            // A named window come forward, not Chromium's (its focus comes as an
                            // event): its focus is looked for in it after its first layout.
                            // Okur's own windows place and report their first focus once laid
                            // out (okur-ui's once_active): not looked for twice.
                            if event.kind == EventKind::WindowActivated
                                && !menu_window
                                && event.app_name != "okur-ui"
                            {
                                window_focus = target
                                    .filter(|e| e.name.as_deref().is_some_and(|n| !n.is_empty()))
                                    .and_then(AtspiObject::of)
                                    .map(|window| WindowFocus {
                                        window,
                                        app: event.app_name.clone(),
                                        pid: event.app_pid,
                                        due: std::time::Instant::now() + WINDOW_FOCUS_WAIT,
                                    });
                            }
                            let native = target
                                .filter(|_| {
                                    converter.wants_native(&event.kind, target, document.is_some())
                                        && (event.kind != EventKind::WindowActivated || menu_window)
                                })
                                .and_then(|e| {
                                    read_native(&provider, e, walk, &|key| {
                                        converter.knows_dialog_box(key)
                                    })
                                });
                            let reread = converter
                                .wants_reread(&event.kind, target)
                                .cloned()
                                .and_then(|e| {
                                    read_native(&provider, &e, None, &|key| {
                                        converter.knows_dialog_box(key)
                                    })
                                });
                            let record = AtspiRecord {
                                at_ms,
                                kind: event.kind.clone(),
                                app: event.app_name.clone(),
                                target: snapshot,
                                document: document.as_ref().map(key_of),
                                // Pages come as records of their own, before this one.
                                pages: Vec::new(),
                                native,
                                reread,
                                text: None,
                                live: None,
                                ui: None,
                                delivered_ms: None,
                            };
                            let cancelled = delivery.push(record, true);
                            drop_pages(cancelled, &reader, &mut converter);
                            if !deliver(
                                &mut delivery,
                                &mut converter,
                                &trace,
                                &live,
                                (&focused, &field_reads),
                                &sink,
                            ) {
                                return;
                            }
                        }
                        RecvStatus::Timeout => {}
                        RecvStatus::Disconnected => {
                            // Said, not silent: Okur hears nothing more until it starts again
                            // (the code review of 2026-11-08).
                            tracing::error!("AT-SPI: the accessibility bus ended; no more events");
                            return;
                        }
                    }
                }
            })
        };
        self.running = Some(Running { stop, thread });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        if let Some(running) = self.running.take() {
            running.stop.store(true, Ordering::SeqCst);
            let _ = running.thread.join();
        }
        if let Some(before) = self.status_before.take() {
            if let Err(e) = before.write() {
                tracing::warn!("the desktop's accessibility status not put back: {e}");
            }
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.running.is_some()
    }
}

impl Drop for AtspiEventSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod heard_tests {
    use super::*;

    /// A read page's first change of its children is heard and none after it until the page
    /// is read again; its text changing is not (a field typed in); another page's object is not
    /// this page's.
    #[test]
    fn a_pages_first_change_is_heard_once() {
        let no_focus = FocusedObject::default();
        let live: LiveObjects = Arc::default();
        let page = |objects: &[&str]| LivePage {
            bus: ":1.5".to_owned(),
            document: "/doc".to_owned(),
            paths: std::collections::HashMap::new(),
            bars: std::collections::HashSet::new(),
            objects: objects.iter().map(|p| (*p).to_owned()).collect(),
            armed: std::sync::atomic::AtomicBool::new(true),
        };
        live.write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(page(&["/doc", "/section"]));
        assert!(!heard(
            &live,
            &no_focus,
            &EventKind::TextChanged,
            ":1.5",
            "/section"
        ));
        // The focused field's text changing (typed in it) is heard.
        let field: FocusedObject = Arc::new(Mutex::new(Some(AtspiObject {
            bus_name: ":1.5".to_owned(),
            path: "/section".to_owned(),
            role: "entry".to_owned(),
        })));
        assert!(heard(
            &live,
            &field,
            &EventKind::TextChanged,
            ":1.5",
            "/section"
        ));
        assert!(heard(
            &live,
            &no_focus,
            &EventKind::StructureChanged,
            ":1.5",
            "/section"
        ));
        assert!(!heard(
            &live,
            &no_focus,
            &EventKind::StructureChanged,
            ":1.5",
            "/doc"
        ));
        assert!(!heard(
            &live,
            &no_focus,
            &EventKind::StructureChanged,
            ":1.9",
            "/section"
        ));
        assert_eq!(
            page_of_object(&live, ":1.5", "/section").as_deref(),
            Some(":1.5/doc")
        );
        // Asked for again, it is heard again.
        rearm(&live, ":1.5/doc");
        assert!(heard(
            &live,
            &no_focus,
            &EventKind::StructureChanged,
            ":1.5",
            "/doc"
        ));
    }
}

#[cfg(test)]
mod page_id_tests {
    use super::*;

    fn tree(path: &str, children: Vec<AtspiTree>) -> AtspiTree {
        AtspiTree {
            bus_name: ":1.5".to_owned(),
            path: path.to_owned(),
            children,
            ..AtspiTree::default()
        }
    }

    fn page_read(ids: &mut PageIds, document: &str, page: &AtspiTree) -> Vec<i32> {
        let mut out = Vec::new();
        ids.ids_of(page, &mut out);
        ids.page_read(document, &out);
        out
    }

    /// A page read again gives its objects the ids they had; a new object a new id; an object
    /// gone from it goes. A page's ids are no other page's.
    #[test]
    fn a_page_read_again_keeps_its_objects_ids() {
        let mut ids = PageIds::default();
        let first = tree("/doc", vec![tree("/a", vec![]), tree("/b", vec![])]);
        let before = page_read(&mut ids, ":1.5/doc", &first);
        let again = tree("/doc", vec![tree("/b", vec![]), tree("/c", vec![])]);
        let after = page_read(&mut ids, ":1.5/doc", &again);
        assert_eq!(after.first(), before.first());
        assert_eq!(after.get(1), before.get(2));
        assert!(after.get(2).is_some_and(|c| !before.contains(c)));
        assert!(before.get(1).is_some_and(|a| !ids.by_id.contains_key(a)));
        // Another application's page, its objects on its own bus.
        let mut other_page = tree("/doc", vec![]);
        other_page.bus_name = ":1.9".to_owned();
        let other = page_read(&mut ids, ":1.9/doc", &other_page);
        assert!(other.iter().all(|id| !after.contains(id)));
    }

    /// A page's objects' ids follow its order as read, which orders its live regions read
    /// together (the log before the status below it); an object never read has none.
    #[test]
    fn a_pages_ids_follow_its_order() {
        let mut ids = PageIds::default();
        let page = tree("/doc", vec![tree("/log", vec![]), tree("/status", vec![])]);
        page_read(&mut ids, ":1.5/doc", &page);
        let log = ids.known_id(":1.5", "/log");
        let status = ids.known_id(":1.5", "/status");
        assert!(log.is_some() && status.is_some() && log < status);
        assert_eq!(ids.known_id(":1.5", "/never"), None);
    }

    /// The last few pages' objects are kept; an older page's go.
    #[test]
    fn the_last_pages_objects_are_kept() {
        let mut ids = PageIds::default();
        let mut pages = Vec::new();
        for page in 0..6 {
            let id = page_read(
                &mut ids,
                &format!(":1.5/page/{page}"),
                &tree(&format!("/page/{page}"), vec![]),
            );
            pages.push(id);
        }
        let kept: Vec<&String> = ids.by_id.values().map(|(_, path)| path).collect();
        assert_eq!(kept, ["/page/2", "/page/3", "/page/4", "/page/5"]);
    }

    /// A live region read again keeps its objects' ids, its new ones kept with its page's, for
    /// acting on them; a region of a page not kept keeps nothing.
    #[test]
    fn a_live_region_read_again_keeps_its_ids() {
        let mut ids = PageIds::default();
        let page = tree("/doc", vec![tree("/log", vec![tree("/message/1", vec![])])]);
        let read = page_read(&mut ids, ":1.5/doc", &page);
        let region = tree(
            "/log",
            vec![tree("/message/1", vec![]), tree("/message/2", vec![])],
        );
        let mut part = Vec::new();
        ids.ids_of(&region, &mut part);
        ids.part_read(":1.5/doc", &part);
        assert_eq!(part.first(), read.get(1));
        assert_eq!(part.get(1), read.get(2));
        let new = part.get(2).copied().unwrap_or_default();
        assert_eq!(
            ids.by_id.get(&new).map(|(_, path)| path.as_str()),
            Some("/message/2")
        );
        // Another page read keeps it, as its page is still kept.
        page_read(&mut ids, ":1.9/other", &tree("/other", vec![]));
        assert!(ids.by_id.contains_key(&new));
    }
}
