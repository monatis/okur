//! What acts on a page's elements for the engine on Linux: Enter or Space on a link, a button,
//! a check box activates it (its AT-SPI Action interface's first action, Chrome's "click"), and
//! focusing an element gives it the keyboard focus (its Component interface's GrabFocus). A page
//! node finds its object by its unique id, its place in the tree the event source read and
//! numbered ([`crate::events::PageObjects`]). And the focused text field read at its caret after
//! a key, as the engine asks (`read_at_caret`): its Text interface's caret, selection and text,
//! the caret polled until the key moved it, given to the engine and written to the trace.
//!
//! The calls on the bus run on a thread of their own, in the order asked: the engine asks from
//! the keyboard hook's callback, with the key held by the X server until it answers, and a slow
//! application made the hook's watchdog replay the key, activating the element twice (the code
//! review of 2026-11-08). An action is taken when its object is known; a call that fails is
//! logged.

use std::collections::VecDeque;
use std::sync::mpsc::{channel, Sender};
use std::time::{Duration, Instant};

use okur_core::actions::ApplicationActions;
use okur_core::aom::{AomNode, UNIQUE_ID_ATTR};
use okur_core::event::{AccessibilityEvent, FontInfo, TextContext};
use okur_linux_model::convert::{AtspiRecord, TextRead};
use okur_runtime::events::EventSink;
use xa11y_linux::{AtspiObject, AtspiTextPosition, LinuxProvider};

use crate::events::{DriverShared, PageObjects};

/// A call on the bus for the worker: an object's bus name and path.
pub(crate) enum Request {
    Focus(String, String),
    Activate(String, String),
    /// The focused text field read at its caret, after the key the engine asked it for: what
    /// the engine knew of the field before the key, and whether the field is a page's.
    Text(Option<Box<TextContext>>, bool),
    /// The focused text field's caret put at a character.
    Caret(usize),
    /// The text field that just took the focus read at its caret at once, as Windows' focus
    /// carries its text: Say Color right after (676/s4) and the lines read before any key
    /// need it. Whether the field is a page's.
    FieldFocused(bool),
}

/// How long a caret read waits for the key to move the caret, or change the text or the
/// selection, before it reads the field as it is: Windows' wait for a page's field.
const SETTLE: Duration = Duration::from_millis(250);

/// How often the caret is read while it is waited for: a round trip on the bus is under a
/// millisecond.
const POLL: Duration = Duration::from_millis(3);

/// Where the focus asked for `object` goes: to it, or, where it cannot take the focus (a
/// graphic, a list), to its page's document, as Windows' Chrome gives it there: Route PC to
/// Virtual on a graphic focuses the page, and the virtual cursor's line is read [obs, 569/s8;
/// Windows' trace: the document's focus 83 ms after the key]. Chrome on Linux leaves the focus
/// where it was, and nothing was said (gtk1's 569).
fn focus_target(provider: &LinuxProvider, object: AtspiObject) -> (String, String) {
    use okur_linux_model::page::state;
    let focusable = provider
        .atspi_state_bits(&object.bus_name, &object.path)
        .is_ok_and(|bits| {
            1u64.checked_shl(state::FOCUSABLE)
                .is_some_and(|b| bits & b != 0)
        });
    if focusable {
        return (object.bus_name, object.path);
    }
    let mut next = provider.atspi_parent(&object);
    for _ in 0..PAGE_DEPTH {
        let Some(parent) = next else { break };
        if parent.role == "document web" {
            return (parent.bus_name, parent.path);
        }
        next = provider.atspi_parent(&parent);
    }
    (object.bus_name, object.path)
}

/// How far up from an element its page's document is looked for.
const PAGE_DEPTH: usize = 40;

/// A text field's read as asked: what the engine knew of it before the key, whether it is a
/// page's, and whether a key moved its caret (else it is read at once, as it took the focus).
struct TextAsk {
    before: Option<TextContext>,
    in_page: bool,
    after_key: bool,
}

/// How long after the focus was asked for an element a caret put is put in it.
const FOCUS_ASKED_FRESH: Duration = Duration::from_millis(1000);

/// The most attribute runs read for the misspelled words of the caret's line: each is a call
/// on the bus.
const RUN_LIMIT: usize = 64;

pub struct LinuxActions {
    objects: PageObjects,
    /// Shared with the event source: the page in front wanted again after it changed.
    shared: DriverShared,
    /// The worker's queue; none when its thread could not start.
    worker: Option<Sender<Request>>,
    /// Presses keys for Okur; none without the X keyboard.
    keys: Option<crate::keyboard::X11KeySender>,
}

impl LinuxActions {
    /// Actions on the objects of the pages `objects` holds, as the event source reads them;
    /// caret reads of the field `shared` names as focused, written to its trace and given to the
    /// engine through `sink`.
    pub fn new(objects: PageObjects, shared: DriverShared, sink: EventSink) -> Self {
        let (requests, received) = channel::<Request>();
        // The event loop asks for a field's read as its focus is delivered.
        *shared
            .field_reads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(requests.clone());
        let kept = shared.clone();
        let worker = std::thread::Builder::new()
            .name("okur-linux-actions".to_owned())
            .spawn(move || {
                let provider = match LinuxProvider::new() {
                    Ok(provider) => provider,
                    Err(e) => {
                        tracing::warn!("Linux actions: no accessibility bus: {e}");
                        return;
                    }
                };
                let mut pending = VecDeque::new();
                // The element last asked to take the focus, and when: a caret put right after
                // it is put in it, its focus event not yet come (Up onto a text area).
                let mut focus_asked: Option<(AtspiObject, Instant)> = None;
                loop {
                    let request = match pending.pop_front() {
                        Some(request) => request,
                        None => match received.recv() {
                            Ok(request) => request,
                            Err(_) => return,
                        },
                    };
                    // The asks queued meanwhile: a caret read is answered for the newest only,
                    // as keys held down outrun the reads.
                    pending.extend(received.try_iter());
                    match request {
                        Request::Focus(bus, path) => {
                            let object = AtspiObject {
                                bus_name: bus,
                                path,
                                role: String::new(),
                            };
                            let (bus, path) = focus_target(&provider, object);
                            if let Err(e) = provider.atspi_grab_focus(&bus, &path) {
                                tracing::debug!("focus not given: {e}");
                            }
                            focus_asked = Some((
                                AtspiObject {
                                    bus_name: bus,
                                    path,
                                    role: String::new(),
                                },
                                Instant::now(),
                            ));
                        }
                        Request::Caret(offset) => {
                            let asked = focus_asked
                                .as_ref()
                                .filter(|(_, at)| at.elapsed() < FOCUS_ASKED_FRESH)
                                .map(|(object, _)| object.clone());
                            let object = asked.or_else(|| {
                                shared
                                    .focused
                                    .lock()
                                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                                    .clone()
                            });
                            let (Some(object), Ok(offset)) = (object, i32::try_from(offset)) else {
                                continue;
                            };
                            match provider.atspi_set_caret(&object.bus_name, &object.path, offset) {
                                Ok(true) => {}
                                Ok(false) => tracing::debug!(offset, "caret not put"),
                                Err(e) => tracing::debug!("caret not put: {e}"),
                            }
                        }
                        Request::Activate(bus, path) => {
                            // Its focus, if Chrome focuses it as it toggles it, waits for its
                            // state (events.rs's settle_toggle).
                            *shared
                                .activated
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some((
                                AtspiObject {
                                    bus_name: bus.clone(),
                                    path: path.clone(),
                                    role: String::new(),
                                },
                                Instant::now(),
                            ));
                            if let Err(e) = provider.atspi_do_action(&bus, &path, 0) {
                                tracing::debug!("not activated: {e}");
                            }
                        }
                        // A newer read asked (a key held down, another field focused) is answered
                        // alone.
                        Request::Text(..) | Request::FieldFocused(..)
                            if pending.iter().any(|r| {
                                matches!(r, Request::Text(..) | Request::FieldFocused(..))
                            }) => {}
                        Request::Text(before, in_page) => {
                            let before = before.map(|b| *b);
                            let read = TextAsk {
                                before,
                                in_page,
                                after_key: true,
                            };
                            if !read_text(&provider, &shared, &sink, read, &mut || false) {
                                return;
                            }
                        }
                        // A read asked as the field took the focus, given up where a key's
                        // read came meanwhile: the engine takes the first read after a key as
                        // the key's answer (the code review of 2026-11-09, third pass, 6).
                        Request::FieldFocused(in_page) => {
                            let read = TextAsk {
                                before: None,
                                in_page,
                                after_key: false,
                            };
                            let mut superseded = || {
                                pending.extend(received.try_iter());
                                pending.iter().any(|r| {
                                    matches!(r, Request::Text(..) | Request::FieldFocused(..))
                                })
                            };
                            if !read_text(&provider, &shared, &sink, read, &mut superseded) {
                                return;
                            }
                        }
                    }
                }
            })
            .map_err(|e| tracing::warn!("Linux actions: no worker thread: {e}"))
            .ok()
            .map(|_| requests);
        Self {
            objects,
            shared: kept,
            worker,
            keys: None,
        }
    }

    /// The same actions, pressing keys through `keys`.
    pub fn with_keys(mut self, keys: crate::keyboard::X11KeySender) -> Self {
        self.keys = Some(keys);
        self
    }

    /// Asks the worker for `request` on `node`'s object: whether it was asked.
    fn ask(&self, node: &AomNode, request: fn(String, String) -> Request) -> bool {
        let (Some(worker), Some((bus, path))) = (self.worker.as_ref(), self.object(node)) else {
            return false;
        };
        worker.send(request(bus, path)).is_ok()
    }

    /// The object of `node`: its application's bus name and its path.
    fn object(&self, node: &AomNode) -> Option<(String, String)> {
        let id: i32 = node.attributes.get(UNIQUE_ID_ATTR)?.parse().ok()?;
        let objects = self.objects.lock().ok()?;
        objects.by_id.get(&id).cloned()
    }
}

/// The line around `caret` in `text`, as `(start, end)` in characters: from after the line
/// break before it to the line break after it.
fn line_around(text: &str, caret: usize) -> (usize, usize) {
    let mut start = 0;
    let mut end = None;
    for (i, c) in text.chars().enumerate() {
        if c == '\n' {
            if i < caret {
                start = i + 1;
            } else {
                end = Some(i);
                break;
            }
        }
    }
    (start, end.unwrap_or_else(|| text.chars().count()))
}

/// A caret read's place, as the engine's text knows it: the caret, the selection, the length.
type Place = (usize, Option<(usize, usize)>, usize);

fn place_read(position: &AtspiTextPosition) -> Option<Place> {
    let offset = |o: i32| usize::try_from(o).ok();
    let selection = match position.selection {
        Some((start, end)) => Some((offset(start)?, offset(end)?)),
        None => None,
    };
    Some((offset(position.caret)?, selection, offset(position.length)?))
}

fn place_known(context: &TextContext) -> Place {
    (
        context.caret,
        context.selection,
        context.text.chars().count(),
    )
}

/// The colors at `offset` in the object's text (its run's `fg-color` and `bg-color`, "r,g,b"
/// as Chrome gives them), for Say Color: "navy on yellow" [obs, 676/s4].
fn colors_at(provider: &LinuxProvider, object: &AtspiObject, offset: i32) -> Option<FontInfo> {
    let (attributes, _, _) = provider
        .atspi_text_attribute_run(&object.bus_name, &object.path, offset, true)
        .ok()?;
    let color = |name: &str| {
        let css = okur_linux_model::page::css_color(attributes.get(name)?)?;
        okur_core::color::Rgb::parse_css(&css)
    };
    let (color, background) = (color("fg-color"), color("bg-color"));
    (color.is_some() || background.is_some()).then(|| FontInfo {
        color,
        background,
        ..FontInfo::default()
    })
}

/// The words the field marks as misspelled in the caret's line (`from` to `to`, in
/// characters), as `(start, end)`: its attribute runs whose `invalid` is "spelling", as Chrome
/// marks them (JAWS says "Misspelled" with them [obs, 284/s6]). The caret's line alone: a whole
/// text area's runs were a call each on every key (the code review of 2026-11-09, finding 5).
fn misspelled(
    provider: &LinuxProvider,
    object: &AtspiObject,
    from: usize,
    to: usize,
) -> Vec<(usize, usize)> {
    let mut words = Vec::new();
    let mut offset = from;
    for _ in 0..RUN_LIMIT {
        if offset >= to {
            break;
        }
        let Ok(at) = i32::try_from(offset) else {
            break;
        };
        let Ok((attributes, start, end)) =
            provider.atspi_text_attribute_run(&object.bus_name, &object.path, at, false)
        else {
            break;
        };
        let (Ok(start), Ok(end)) = (usize::try_from(start), usize::try_from(end)) else {
            break;
        };
        if attributes.get("invalid").map(String::as_str) == Some("spelling") && start < end {
            words.push((start, end));
        }
        if end <= offset {
            break;
        }
        offset = end;
    }
    words
}

/// Reads the focused field at its caret, once the key moved it from where `before` had it (or
/// at once, nothing known), and gives the engine the field's text, caret and selection, its
/// misspelled words and the colors at its caret, and a page field's word (Chrome's word moves
/// stop at punctuation), the read written to the trace first. False once the engine has gone; a
/// field that cannot be read (no Text interface, the object gone) is no answer.
fn read_text(
    provider: &LinuxProvider,
    reads: &DriverShared,
    sink: &EventSink,
    ask: TextAsk,
    superseded: &mut dyn FnMut() -> bool,
) -> bool {
    let TextAsk {
        before,
        in_page,
        after_key,
    } = ask;
    let started = Instant::now();
    let focused = reads
        .focused
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let Some(object) = focused else {
        tracing::debug!("caret read: no focus known");
        return true;
    };
    // The caret and the length polled (two calls) until the key moved them from where the
    // engine knew them, or, nothing known, from where the first read finds them: the key is
    // still held by the X server as the engine asks (the code review of 2026-11-09, finding 3,
    // as Windows' page fields are read). Then the caret, the selection and the length once.
    let caret_and_length = || {
        provider
            .atspi_caret_and_length(&object.bus_name, &object.path)
            .map_err(|e| tracing::debug!("caret read: the caret not read: {e}"))
            .ok()
    };
    // A field read as it took the focus is read at once: no key moves it.
    if after_key {
        let known = match before.as_ref().map(place_known) {
            Some((caret, _, length)) => Some((caret, length)),
            None => caret_and_length().and_then(|(caret, length)| {
                Some((usize::try_from(caret).ok()?, usize::try_from(length).ok()?))
            }),
        };
        loop {
            let now = caret_and_length().and_then(|(caret, length)| {
                Some((usize::try_from(caret).ok()?, usize::try_from(length).ok()?))
            });
            if now.is_none() || now != known || started.elapsed() >= SETTLE {
                break;
            }
            std::thread::sleep(POLL);
        }
    }
    let place = provider
        .atspi_text_position(&object.bus_name, &object.path)
        .map_err(|e| tracing::debug!("caret read: the caret not read: {e}"))
        .ok()
        .and_then(|p| place_read(&p));
    let Some((caret, selection, _)) = place else {
        return true;
    };
    let text = match provider.atspi_text(&object.bus_name, &object.path) {
        Ok(text) => text,
        Err(e) => {
            tracing::debug!("caret read: the text not read: {e}");
            return true;
        }
    };
    // The colors where the caret is; at the text's end, where typing leaves it, the last
    // character's.
    let length = text.chars().count();
    let at = if caret >= length {
        caret.saturating_sub(1)
    } else {
        caret
    };
    let font = i32::try_from(at)
        .ok()
        .and_then(|offset| colors_at(provider, &object, offset));
    let (line_start, line_end) = line_around(&text, caret);
    let misspelled = misspelled(provider, &object, line_start, line_end);
    let word = in_page
        .then(|| okur_core::editing::web_word_at(&text, caret))
        .flatten();
    let read_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    tracing::debug!(
        chars = text.chars().count(),
        caret,
        ?selection,
        ms = read_ms,
        "text context read"
    );
    let context = TextContext {
        text,
        caret,
        selection,
        font,
        word,
        misspelled,
        structure: None,
    };
    // The focus moved on while the field was read (Tab during the wait): the read is the field
    // left, and the engine is not given it.
    if superseded() {
        tracing::debug!("caret read: a newer read asked; not given");
        return true;
    }
    let still_focused = reads
        .focused
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .is_some_and(|now| now.bus_name == object.bus_name && now.path == object.path);
    if !still_focused {
        tracing::debug!("caret read: the focus moved on; not given");
        return true;
    }
    let record = AtspiRecord::text(
        crate::events::unix_ms(),
        TextRead {
            object: format!("{}{}", object.bus_name, object.path),
            context: context.clone(),
            read_ms,
        },
    );
    reads.hand(record, vec![AccessibilityEvent::TextContext(context)], sink)
}

impl ApplicationActions for LinuxActions {
    fn focus(&self, node: &AomNode) -> bool {
        self.ask(node, Request::Focus)
    }

    fn activate(&self, node: &AomNode) -> bool {
        self.ask(node, Request::Activate)
    }

    fn presses_keys(&self) -> bool {
        self.keys
            .as_ref()
            .is_some_and(crate::keyboard::X11KeySender::can_press)
    }

    /// The keys held for one of Okur's dialogs, by their keycodes as they came.
    fn press_again(&self, events: &[okur_core::input::KeyEvent]) -> bool {
        self.keys
            .as_ref()
            .is_some_and(|keys| keys.press_again(events))
    }

    /// A reading command's key (CapsLock+O's Down in a text field), or a key held for one of
    /// Okur's dialogs, pressed through XTest and read back as Okur's own.
    fn press(&self, chord: &[okur_core::Key]) -> bool {
        self.keys.as_ref().is_some_and(|keys| keys.press(chord))
    }

    /// The page in front changed outside its live regions, or a region's change was said: the
    /// event source captures it again once its changes settle, or at once (`now`).
    fn page_changed(&self, now: bool) -> bool {
        let mut wanted = self
            .shared
            .page_wanted
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        crate::events::PageWanted::ask(&mut wanted, now);
        true
    }

    fn move_caret(&self, _context: &TextContext, offset: usize) -> bool {
        self.worker
            .as_ref()
            .is_some_and(|worker| worker.send(Request::Caret(offset)).is_ok())
    }

    fn read_at_caret(&self, before: Option<&TextContext>, in_page: bool) -> bool {
        self.worker.as_ref().is_some_and(|worker| {
            worker
                .send(Request::Text(before.cloned().map(Box::new), in_page))
                .is_ok()
        })
    }
}
