//! The desktop's accessibility events on Linux: every application's AT-SPI events, through the
//! vendored xa11y's desktop subscription (registered with the AT-SPI registry, without which
//! GTK 4 and Chrome send nothing: the Linux lab's probe 2), as the engine takes them. First the
//! focus: an element taking the keyboard focus is the engine's focus event
//! (`okur_linux_model::focus`); the other kinds follow as the engine's Linux path grows.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use okur_linux_model::convert::{AtspiRecord, Converter, NativeRead, PageRead};
use okur_linux_model::delivery::Delivery;
use okur_linux_model::focus::{ancestor_limit, is_window, CONTROL_LIMIT, WINDOW_DEPTH};
use okur_runtime::events::{EventSink, EventSource};
use okur_runtime::RuntimeError;
use xa11y_core::{ElementData, EventKind, RecvStatus, Role};
use xa11y_linux::{A11yStatus, AtspiObject, AtspiTree, LinuxProvider};

/// The objects of the last pages read, each its application's bus name and its path, by the id
/// the page's node was numbered with ([`okur_linux_model::page::atspi_to_web_from`], depth first
/// from the page's base): what acting on a page's node finds its object by
/// ([`crate::actions::LinuxActions`]).
pub type PageObjects = Arc<Mutex<PageIds>>;

/// The ids a page read gives its objects, and the objects of the last pages by them.
#[derive(Debug, Default)]
pub struct PageIds {
    /// The base the next page read numbers from.
    next_base: i32,
    /// The bases of the pages whose objects are kept, oldest first.
    kept: std::collections::VecDeque<i32>,
    pub by_id: std::collections::BTreeMap<i32, (String, String)>,
}

/// How far apart pages' ids start: more than a page read keeps ([`PAGE_LIMIT`]).
const PAGE_ID_STRIDE: i32 = 1 << 20;

/// How many pages' objects are kept.
const PAGES_KEPT: i32 = 4;

impl PageIds {
    /// The base a new page read numbers from; the objects of pages before the last few go.
    fn next_page(&mut self) -> i32 {
        let base = self.next_base;
        // Wrapping before i32's end: ids start again at 0 after 2000 pages.
        self.next_base = if base >= 2000 * PAGE_ID_STRIDE {
            0
        } else {
            base + PAGE_ID_STRIDE
        };
        let range = |start: i32| start..=start.saturating_add(PAGE_ID_STRIDE);
        // The page this one's ids were last given to, after the ids wrapped, and the oldest
        // page past the ones kept.
        self.by_id.retain(|id, _| !range(base).contains(id));
        self.kept.retain(|kept| *kept != base);
        self.kept.push_back(base);
        while self.kept.len() > usize::try_from(PAGES_KEPT).unwrap_or(4) {
            if let Some(oldest) = self.kept.pop_front() {
                self.by_id.retain(|id, _| !range(oldest).contains(id));
            }
        }
        base
    }
}

/// `tree`'s objects, depth first, as the page's nodes are numbered.
fn objects_of(
    tree: &AtspiTree,
    next: &mut i32,
    out: &mut std::collections::BTreeMap<i32, (String, String)>,
) {
    *next += 1;
    out.insert(*next, (tree.bus_name.clone(), tree.path.clone()));
    for child in &tree.children {
        objects_of(child, next, out);
    }
}

/// The most objects a page's capture reads: a large page is cut there, its first part read.
const PAGE_LIMIT: usize = 20_000;

/// xa11y-linux's capture as the model's snapshot node.
fn snapshot(tree: AtspiTree) -> okur_linux_model::page::AtspiNode {
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
        next = if is_window(&parent.role) {
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
) -> Option<NativeRead> {
    let Some(object) = AtspiObject::of(element) else {
        tracing::debug!("a native focus without its object's names");
        return None;
    };
    let started = std::time::Instant::now();
    let focus = snapshot(provider.atspi_tree(&object, CONTROL_LIMIT));
    let walked = match walked {
        Some(walked) => walked.to_vec(),
        None => walk_to_window(provider, &object),
    };
    let mut ancestors = Vec::new();
    for parent in walked {
        if ancestors.len() >= WINDOW_DEPTH {
            break;
        }
        let limit = ancestor_limit(ancestors.len(), &parent.role, &focus);
        let node = snapshot(provider.atspi_tree(&parent, limit));
        let window = is_window(&node.role);
        ancestors.push(node);
        // The window is the last read: its own parent, the application, is not read.
        if window {
            break;
        }
    }
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
    })
}

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
    // A lock a panicking thread left still holds the numbering (the second review): base 0
    // and no objects would have sent actions to other pages' objects.
    let base = {
        let mut objects = objects
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let base = objects.next_page();
        let mut next = base;
        objects_of(&tree, &mut next, &mut objects.by_id);
        base
    };
    let root = snapshot(tree);
    let read_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::debug!(ms = read_ms, objects = count(&root), "page read");
    PageRead {
        document: key_of(document),
        base,
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
    write: &mut dyn FnMut(&AtspiRecord),
    sink: &EventSink,
) -> bool {
    for record in delivery.take_ready() {
        write(&record);
        for said in converter.convert_read(&record) {
            if !sink.send(said) {
                return false;
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
fn unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
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
    let provider = LinuxProvider::new().map_err(|e| source_error("the bus", e))?;
    let subscription = provider
        .subscribe_desktop_kinds(Box::new(okur_linux_model::focus_track::is_heard))
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
        let subscription = provider
            .subscribe_desktop_kinds(Box::new(okur_linux_model::focus_track::is_heard))
            .map_err(|e| source_error("the desktop's events", e))?;
        tracing::debug!("AT-SPI: the desktop's events subscribed");
        let mut trace = match &self.trace {
            Some(path) => Some(std::io::BufWriter::new(
                std::fs::File::create(path).map_err(|e| source_error("the trace", e))?,
            )),
            None => None,
        };
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
                let mut write = |record: &AtspiRecord| {
                    if let Some(out) = trace.as_mut() {
                        if let Err(e) = write_record(out, record) {
                            tracing::warn!("the trace not written, no longer kept: {e}");
                            trace = None;
                        }
                    }
                };
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
                    if !deliver(&mut delivery, &mut converter, &mut write, &sink) {
                        return;
                    }
                    // Short while a page is awaited, so that it comes at once when the bus is
                    // quiet; else the thread wakes ten times a second, not fifty (the second
                    // review, finding 8).
                    let wait = if delivery.is_holding() { 20 } else { 100 };
                    match subscription.recv_status(Duration::from_millis(wait)) {
                        RecvStatus::Event(event) => {
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
                                if !deliver(&mut delivery, &mut converter, &mut write, &sink) {
                                    return;
                                }
                                continue;
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
                            if let (true, Some(element), Some(document)) =
                                (focus, target, document.as_ref())
                            {
                                let on_document = okur_linux_model::convert::is_document(element);
                                if converter.wants_page(on_document, &key_of(document)) {
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
                            let native = target
                                .filter(|_| {
                                    Converter::wants_native(&event.kind, target, document.is_some())
                                })
                                .and_then(|e| read_native(&provider, e, Some(&walked)));
                            let reread = converter
                                .wants_reread(&event.kind, target)
                                .cloned()
                                .and_then(|e| read_native(&provider, &e, None));
                            let record = AtspiRecord {
                                at_ms,
                                kind: event.kind.clone(),
                                app: event.app_name.clone(),
                                target: event.target.clone(),
                                document: document.as_ref().map(key_of),
                                // Pages come as records of their own, before this one.
                                pages: Vec::new(),
                                native,
                                reread,
                            };
                            let cancelled = delivery.push(record, true);
                            drop_pages(cancelled, &reader, &mut converter);
                            if !deliver(&mut delivery, &mut converter, &mut write, &sink) {
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
mod page_id_tests {
    use super::*;

    /// Each page read numbers its objects from a base of its own; the last pages' objects are
    /// kept, an older page's go.
    #[test]
    fn a_pages_objects_keep_their_ids_after_another_page_is_read() {
        let mut ids = PageIds::default();
        let mut bases = Vec::new();
        for page in 0..6 {
            let base = ids.next_page();
            ids.by_id
                .insert(base + 1, (":1.5".to_owned(), format!("/page/{page}")));
            bases.push(base);
        }
        assert_eq!(
            bases
                .windows(2)
                .filter(|w| matches!(w, [a, b] if b - a == PAGE_ID_STRIDE))
                .count(),
            5
        );
        let kept: Vec<&String> = ids.by_id.values().map(|(_, path)| path).collect();
        assert_eq!(kept, ["/page/2", "/page/3", "/page/4", "/page/5"]);
    }
}
