//! Native AT-SPI2 event subscription (push-based, via D-Bus signals).
//!
//! This replaces the prior inert subscription stub. AT-SPI2 delivers
//! accessibility events as D-Bus signals on four interfaces:
//!
//! - `org.a11y.atspi.Event.Object` — StateChanged, ChildrenChanged,
//!   PropertyChange, TextChanged, SelectionChanged, ValueChanged,
//!   Announcement, BoundsChanged, VisibleDataChanged, etc.
//! - `org.a11y.atspi.Event.Window` — Create/Destroy/Activate/Deactivate/…
//! - `org.a11y.atspi.Event.Focus` — Focus
//! - `org.a11y.atspi.Event.Document` — LoadComplete
//!
//! Each signal body follows the AT-SPI2 standard signature `(siiva{sv})`:
//! `(detail_kind, detail1, detail2, any_data, properties)`. The source
//! element is identified by the D-Bus message header's `sender` (bus name)
//! and `path` (object path).
//!
//! Subscriptions are scoped to a single application: we resolve the app's
//! D-Bus unique name via the registry, then install match rules filtered
//! by `sender`. A dedicated background thread iterates the message stream
//! and fans received signals out to an mpsc channel.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use zbus::blocking::fdo::DBusProxy;
use zbus::blocking::{Connection, MessageIterator, Proxy};
use zbus::message::Type as MessageType;
use zbus::zvariant::OwnedValue;
use zbus::MatchRule;

use xa11y_core::{
    CancelHandle, ElementData, ElementParts, Error, Event, EventKind, EventParts, EventReceiver,
    Result, Role, StateFlag, StateParts, StateSet, Subscription, Toggled,
};

use crate::atspi::{map_atspi_role, map_atspi_role_number, AccessibleRef, LinuxProvider};

/// AT-SPI2 event-interface names we subscribe to via match rules.
///
/// Kept as a shared constant so registration and the interface filter in
/// `signal_to_kinds` iterate the same list.
pub(crate) const EVENT_INTERFACES: &[&str] = &[
    "org.a11y.atspi.Event.Object",
    "org.a11y.atspi.Event.Window",
    "org.a11y.atspi.Event.Focus",
    "org.a11y.atspi.Event.Document",
];

/// The event classes a subscription registers with the AT-SPI registry, one per
/// interface of [`EVENT_INTERFACES`]: a class with an empty name covers every
/// event of it.
///
/// Toolkits send events only while someone has registered for them: in Okur's
/// Linux lab (probe 2, 2026-11-05), six Tabs through GTK 4 and through Chrome
/// gave match rules alone no event at all, and one focus event per Tab once a
/// listener had registered with the registry, as libatspi's listeners do.
pub(crate) const REGISTERED_EVENTS: &[&str] = &["object:", "window:", "focus:", "document:"];

const REGISTRY: &str = "org.a11y.atspi.Registry";
const REGISTRY_PATH: &str = "/org/a11y/atspi/registry";

/// Registers [`REGISTERED_EVENTS`] for the application on `app_bus` (signature
/// `sass`: the event, no properties to cache, the application's bus name).
fn register_events(conn: &Connection, app_bus: &str) -> Result<()> {
    for event in REGISTERED_EVENTS {
        conn.call_method(
            Some(REGISTRY),
            REGISTRY_PATH,
            Some(REGISTRY),
            "RegisterEvent",
            &(*event, Vec::<&str>::new(), app_bus),
        )
        .map_err(|e| Error::Platform {
            code: -1,
            message: format!("RegisterEvent({event}): {e}"),
        })?;
    }
    Ok(())
}

/// Takes back what [`register_events`] registered (signature `ss`). The
/// registry also forgets a listener's events when its connection closes; this
/// says so at once.
fn deregister_events(conn: &Connection, app_bus: &str) {
    for event in REGISTERED_EVENTS {
        let _ = conn.call_method(
            Some(REGISTRY),
            REGISTRY_PATH,
            Some(REGISTRY),
            "DeregisterEvent",
            &(*event, app_bus),
        );
    }
}

// ── Entry point ─────────────────────────────────────────────────────────────

/// Subscribe to AT-SPI2 events for the application identified by `pid`.
///
/// Returns a `Subscription` whose receiver yields `Event`s for every signal
/// emitted on the four AT-SPI event interfaces (scoped by the app's D-Bus
/// sender name). Drops and re-subscribes are clean: the cancel closure
/// removes every match rule before the thread joins.
pub(crate) fn subscribe_for_pid(
    provider: &LinuxProvider,
    pid: u32,
    app_name: String,
) -> Result<Subscription> {
    let sender_buses: Vec<String> = provider
        .find_apps_by_pid(pid)?
        .into_iter()
        .map(|app| app.bus_name)
        .collect();
    if sender_buses.is_empty() {
        return Err(Error::Platform {
            code: -1,
            message: format!("No application found with PID {pid}"),
        });
    }
    subscribe(
        None,
        Sources::App {
            buses: sender_buses,
            name: app_name,
            pid,
        },
    )
}

/// Subscribe to every application's AT-SPI2 events: the whole desktop, as a
/// screen reader hears it. Each event names its application and process,
/// looked up when the application is first heard. The registration is for
/// every application, so toolkits started later send their events too.
pub(crate) fn subscribe_desktop(wanted: Option<KindFilter>) -> Result<Subscription> {
    subscribe(wanted, Sources::Desktop(Mutex::new(HashMap::new())))
}

/// The events a subscriber takes, by their kind and their object (its application's bus name
/// and its path): the others are dropped before their element is read. A screen reader takes a
/// page's children and text changes from its live regions' objects alone, which it knows by
/// their paths.
pub type KindFilter = Box<dyn Fn(&EventKind, &str, &str) -> bool + Send + Sync>;

fn subscribe(wanted: Option<KindFilter>, sources: Sources) -> Result<Subscription> {
    // The senders the match rules and the registration name: an
    // application's bus names, or none for the desktop (every sender; the
    // registry takes an empty application name as every application).
    let senders: Vec<Option<String>> = match &sources {
        Sources::App { buses, .. } => buses.iter().cloned().map(Some).collect(),
        Sources::Desktop(_) => vec![None],
    };

    // Dedicated Connection per subscription. Dropping it (after we've
    // removed every match rule) cleanly tears the subscription down and
    // forbids any stale signal from arriving at the previous owner.
    let conn = LinuxProvider::connect_a11y_bus()?;
    // The snapshots' calls (each event's element read) go on a connection of their own. On
    // `conn` each reply also queues in the iterator's stream behind the signals, and the thread
    // waiting for it is the one that drains that stream: once a burst filled its 64 places
    // (Chrome's signals as a page loads), zbus stopped reading the socket and the reply never
    // came. The Linux lab's second live run in a session heard nothing (2026-11-05).
    let calls = LinuxProvider::connect_a11y_bus()?;

    // Match-rule management goes through the DBusProxy — the blocking
    // Connection itself doesn't expose AddMatch/RemoveMatch.
    let dbus = DBusProxy::new(&conn).map_err(|e| Error::Platform {
        code: -1,
        message: format!("DBusProxy: {e}"),
    })?;

    let mut rules: Vec<MatchRule<'static>> =
        Vec::with_capacity(EVENT_INTERFACES.len() * senders.len());
    for sender in &senders {
        for iface in EVENT_INTERFACES {
            // Pass owned Strings so the produced MatchRule is `'static` and
            // can be moved into the cancel closure alongside other owned state.
            let builder = MatchRule::builder().msg_type(MessageType::Signal);
            let builder = match sender {
                Some(bus) => builder.sender(bus.clone()).map_err(|e| Error::Platform {
                    code: -1,
                    message: format!("sender match rule: {e}"),
                })?,
                None => builder,
            };
            let rule = builder
                .interface((*iface).to_string())
                .map_err(|e| Error::Platform {
                    code: -1,
                    message: format!("interface match rule: {e}"),
                })?
                .build();
            dbus.add_match_rule(rule.clone())
                .map_err(|e| Error::Platform {
                    code: -1,
                    message: format!("add_match_rule({iface}): {e}"),
                })?;
            rules.push(rule);
        }
    }

    let (tx, rx) = std::sync::mpsc::channel::<Event>();
    let registered: Vec<String> = senders
        .iter()
        .map(|sender| sender.clone().unwrap_or_default())
        .collect();
    let ctx = Arc::new(EventContext {
        sources,
        tx: Mutex::new(tx),
        conn: calls,
        wanted,
    });

    // Activate the broadcast receiver *before* spawning the worker. If we
    // waited until inside the spawned thread to construct the iterator,
    // there would be a window between `add_match_rule` returning above and
    // the thread scheduling its first statement during which the daemon
    // could deliver matching signals to our Connection — where they would
    // fan out to zero subscribers and be dropped. Creating the iterator
    // here makes the subscription atomic from the caller's point of view:
    // any signal that arrives after every `add_match_rule` call has
    // returned is guaranteed to land in our queue.
    let iter = MessageIterator::from(conn.clone());

    // Only now ask the toolkits for their events: the match rules and the
    // iterator are in place, so the first event after registration is heard.
    for sender_bus in &registered {
        if let Err(error) = register_events(&conn, sender_bus) {
            deregister_events(&conn, sender_bus);
            for rule in &rules {
                let _ = dbus.remove_match_rule(rule.clone());
            }
            return Err(error);
        }
    }

    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_thread = stop.clone();
    let ctx_for_thread = ctx.clone();
    let handle = thread::spawn(move || {
        for msg in iter {
            if stop_for_thread.load(Ordering::Relaxed) {
                break;
            }
            if let Ok(msg) = msg {
                ctx_for_thread.dispatch(&msg);
            }
        }
    });

    // `CancelHandle` captures the connection and rules. On drop:
    // 1. Flip the stop flag so the thread exits on its next wake-up.
    // 2. Remove every match rule so the D-Bus daemon stops routing
    //    matching signals to us.
    // 3. Issue a Ping to the daemon — its reply travels back through our
    //    MessageIterator, forcing `.next()` to return and letting the
    //    thread observe the stop flag.
    // 4. Join the worker thread.
    let cancel_conn = conn.clone();
    let registered_buses = registered;
    let cancel = CancelHandle::new(move || {
        stop.store(true, Ordering::Relaxed);
        for sender_bus in &registered_buses {
            deregister_events(&cancel_conn, sender_bus);
        }
        if let Ok(dbus) = DBusProxy::new(&cancel_conn) {
            for rule in &rules {
                let _ = dbus.remove_match_rule(rule.clone());
            }
        }
        // Wake the blocked MessageIterator by forcing a round-trip: the
        // daemon's Ping reply is broadcast through our connection's
        // message stream, so `.next()` returns and the thread observes
        // the stop flag on its next iteration.
        let _ = cancel_conn.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus.Peer"),
            "Ping",
            &(),
        );
        let _ = handle.join();
    });

    Ok(Subscription::new(EventReceiver::new(rx), cancel))
}

// ── Event context + dispatch ────────────────────────────────────────────────

/// Shared state between the subscription thread and the cancel path.
///
/// `tx` is wrapped in `Mutex` because `std::sync::mpsc::Sender` is `!Sync`;
/// contention is trivial (one lock per emitted event).
struct EventContext {
    sources: Sources,
    tx: Mutex<std::sync::mpsc::Sender<Event>>,
    /// The kinds the subscriber takes; every kind when none is given.
    wanted: Option<KindFilter>,
    /// Used for D-Bus property/method calls to build target snapshots: a
    /// connection of its own, not the one whose signals the thread reads
    /// (see `subscribe`). It stays alive until every `Arc<EventContext>`
    /// (including the one held by the spawned thread) is dropped.
    conn: Connection,
}

/// Whose events a subscription hears.
enum Sources {
    /// One application: its bus names, its name and its process.
    App {
        buses: Vec<String>,
        name: String,
        pid: u32,
    },
    /// Every application, each sender's name and process kept once looked up, with when: an
    /// empty name is asked again a second later.
    Desktop(Mutex<HashMap<String, (String, u32, std::time::Instant)>>),
}

impl EventContext {
    /// The application a sender is, with its process: the subscription's own
    /// for one application (none for another sender), or looked up once for
    /// the desktop (its root's name, and its connection's process).
    fn app_of(&self, sender: &str) -> Option<(String, u32)> {
        match &self.sources {
            Sources::App { buses, name, pid } => buses
                .iter()
                .any(|bus| bus == sender)
                .then(|| (name.clone(), *pid)),
            Sources::Desktop(known) => {
                // A name not yet given (an application's first events, before its root is up:
                // okur-ui's windows came with none in every Linux trace) is asked again a second
                // later, not with every signal (a page load sends thousands; the code review of
                // 2026-11-09, sixth pass, 7).
                let fresh = |(name, _, at): &(String, u32, std::time::Instant)| {
                    !name.is_empty() || at.elapsed() < std::time::Duration::from_secs(1)
                };
                if let Some((name, pid, _)) = known
                    .lock()
                    .ok()
                    .and_then(|k| k.get(sender).filter(|app| fresh(app)).cloned())
                {
                    return Some((name, pid));
                }
                let root = AccessibleRef {
                    bus_name: sender.to_string(),
                    path: "/org/a11y/atspi/accessible/root".to_string(),
                };
                let app = (
                    get_name(&self.conn, &root).unwrap_or_default(),
                    connection_pid(&self.conn, sender).unwrap_or(0),
                );
                if let Ok(mut known) = known.lock() {
                    known.insert(
                        sender.to_string(),
                        (app.0.clone(), app.1, std::time::Instant::now()),
                    );
                }
                Some(app)
            }
        }
    }

    fn emit(&self, kind: EventKind, target: Option<ElementData>, app: &(String, u32)) {
        let event: Event = EventParts {
            kind,
            target,
            app_name: app.0.clone(),
            app_pid: app.1,
            timestamp: std::time::Instant::now(),
        }
        .into();
        if let Ok(tx) = self.tx.lock() {
            let _ = tx.send(event);
        }
    }

    fn dispatch(&self, msg: &zbus::Message) {
        let header = msg.header();

        // Match rules already filter by sender, but we stay defensive: the
        // daemon's Ping reply (used to wake the iterator on cancel) arrives
        // here too and must be skipped.
        let Some(sender) = header.sender() else {
            return;
        };
        // Only the event interfaces' signals: the Ping reply has none, and
        // the desktop's match rules name no sender to filter by.
        let Some(iface) = header.interface() else {
            return;
        };
        if !EVENT_INTERFACES.contains(&iface.as_str()) {
            return;
        }
        let Some(member) = header.member() else {
            return;
        };
        let Some(path) = header.path() else {
            return;
        };
        // AT-SPI2 event body signature: (s, i, i, v, a{sv}).
        let body = msg.body();
        let (detail, detail1, _detail2, _any, _props): (
            String,
            i32,
            i32,
            OwnedValue,
            HashMap<String, OwnedValue>,
        ) = match body.deserialize() {
            Ok(v) => v,
            Err(_) => return,
        };

        // The kinds first, before the element's snapshot (five to seven calls
        // back into the application): a signal that maps to no kind, or to none
        // the subscriber takes, costs nothing more (a page loading sends
        // thousands; the code review of 2026-11-08). The role only adds a text
        // kind beside a value change, so it is read when a kind is wanted.
        let wanted = |kind: &EventKind| {
            self.wanted
                .as_ref()
                .is_none_or(|w| w(kind, sender.as_str(), path.as_str()))
        };
        let unread = signal_to_kinds(iface.as_str(), member.as_str(), &detail, detail1, None);
        if !unread.iter().any(wanted) {
            return;
        }
        // The application, once its signal is wanted: looking it up can ask the bus.
        let Some(app) = self.app_of(sender.as_str()) else {
            return;
        };

        let target_ref = AccessibleRef {
            bus_name: sender.as_str().to_string(),
            path: path.to_string(),
        };
        // Snapshot the source element synchronously so consumers receive a
        // durable `ElementData` even if the element is destroyed before
        // they read the event.
        let target = build_event_snapshot(&self.conn, &target_ref, Some(app.1));
        let target_role = target.as_ref().map(|t| t.role);

        let kinds = signal_to_kinds(
            iface.as_str(),
            member.as_str(),
            &detail,
            detail1,
            target_role,
        );

        for kind in kinds.into_iter().filter(wanted) {
            self.emit(kind, target.clone(), &app);
        }
    }
}

// ── Signal → EventKind mapping ──────────────────────────────────────────────

/// Map an AT-SPI2 signal tuple to zero-or-more xa11y `EventKind`s.
///
/// Returns `Vec` because some AT-SPI2 signals map to multiple cross-platform
/// kinds — e.g. `Object:StateChanged(focused, true)` emits both
/// `FocusChanged` and `StateChanged{Focused, true}` (matching the Windows
/// UIA behaviour where focus changes arrive as both a dedicated focus event
/// and a property change), and `Object:ValueChanged` on a text role emits
/// `ValueChanged` + `TextChanged` (matching macOS).
pub(crate) fn signal_to_kinds(
    iface: &str,
    member: &str,
    detail: &str,
    detail1: i32,
    target_role: Option<Role>,
) -> Vec<EventKind> {
    match (iface, member) {
        // Focus interface — keyboard focus moved.
        ("org.a11y.atspi.Event.Focus", "Focus") => vec![EventKind::FocusChanged],

        // Object.StateChanged — one signal per state bit. `detail` is the
        // state name, `detail1` is 0/1.
        ("org.a11y.atspi.Event.Object", "StateChanged") => {
            let value = detail1 != 0;
            let lower = detail.to_ascii_lowercase();
            match lower.as_str() {
                "focused" => {
                    // Some toolkits (notably GTK4) skip `Focus:Focus` and
                    // only emit StateChanged(focused). Raise both so
                    // consumers can filter on either.
                    if value {
                        vec![
                            EventKind::FocusChanged,
                            EventKind::StateChanged {
                                flag: StateFlag::Focused,
                                value,
                            },
                        ]
                    } else {
                        vec![EventKind::StateChanged {
                            flag: StateFlag::Focused,
                            value,
                        }]
                    }
                }
                // A toggle button's pressed state is its checked one (Chrome's aria-pressed).
                "checked" | "pressed" => vec![EventKind::StateChanged {
                    flag: StateFlag::Checked,
                    value,
                }],
                // AT-SPI exposes both `enabled` and `sensitive`; xa11y
                // collapses them onto `Enabled` so consumers don't have to
                // deal with the (historical) distinction.
                "enabled" | "sensitive" => vec![EventKind::StateChanged {
                    flag: StateFlag::Enabled,
                    value,
                }],
                "visible" | "showing" => vec![EventKind::StateChanged {
                    flag: StateFlag::Visible,
                    value,
                }],
                "expanded" => vec![EventKind::StateChanged {
                    flag: StateFlag::Expanded,
                    value,
                }],
                "selected" => vec![EventKind::StateChanged {
                    flag: StateFlag::Selected,
                    value,
                }],
                "busy" => vec![EventKind::StateChanged {
                    flag: StateFlag::Busy,
                    value,
                }],
                "editable" => vec![EventKind::StateChanged {
                    flag: StateFlag::Editable,
                    value,
                }],
                "focusable" => vec![EventKind::StateChanged {
                    flag: StateFlag::Focusable,
                    value,
                }],
                "modal" => vec![EventKind::StateChanged {
                    flag: StateFlag::Modal,
                    value,
                }],
                "required" => vec![EventKind::StateChanged {
                    flag: StateFlag::Required,
                    value,
                }],
                // The AT-SPI state for "this window was iconified/restored".
                // Not emitted by every toolkit, but wiring it here is the only
                // way a subscriber learns of a window minimization — without
                // it the public StateFlag::Minimized is dead on Linux.
                "iconified" => vec![EventKind::StateChanged {
                    flag: StateFlag::Minimized,
                    value,
                }],
                _ => vec![],
            }
        }

        // Object.PropertyChange — AccessKit's AT-SPI bridge publishes both
        // name changes and numeric value changes through this signal with
        // `detail` set to the AT-SPI property id (`accessible-name`,
        // `accessible-value`, `accessible-description`, `accessible-parent`,
        // `accessible-role`). The design doc maps:
        // - name  → NameChanged
        // - value → ValueChanged (+ TextChanged on text roles, mirroring macOS)
        // - description/parent/role → no cross-platform EventKind; drop.
        ("org.a11y.atspi.Event.Object", "PropertyChange") => {
            let d = detail.to_ascii_lowercase();
            if d == "accessible-name" || d == "name" {
                vec![EventKind::NameChanged]
            } else if d == "accessible-value" || d == "value" {
                let mut kinds = vec![EventKind::ValueChanged];
                if matches!(target_role, Some(Role::TextField | Role::TextArea)) {
                    kinds.push(EventKind::TextChanged);
                }
                kinds
            } else {
                vec![]
            }
        }

        ("org.a11y.atspi.Event.Object", "ChildrenChanged") => vec![EventKind::StructureChanged],

        // Text changes. AT-SPI2 publishes text-change events on the Object
        // event bus (both "TextChanged" in newer specs and historical
        // "TextChanged" via an application-specific signal). `detail` is
        // "insert" or "delete".
        ("org.a11y.atspi.Event.Object", "TextChanged") => vec![EventKind::TextChanged],
        ("org.a11y.atspi.Event.Object", "TextSelectionChanged") => {
            vec![EventKind::SelectionChanged]
        }
        ("org.a11y.atspi.Event.Object", "TextAttributesChanged") => vec![],

        ("org.a11y.atspi.Event.Object", "SelectionChanged") => vec![EventKind::SelectionChanged],
        ("org.a11y.atspi.Event.Object", "ActiveDescendantChanged") => {
            vec![EventKind::SelectionChanged]
        }

        // Object.ValueChanged: AccessKit's AT-SPI bridge publishes numeric
        // value changes here (the spec moved this to Value:ValueChanged
        // eventually, but both shapes appear in the wild). Treat as
        // ValueChanged, and also emit TextChanged when the role is a text
        // role so the macOS/Windows text-field pattern survives.
        ("org.a11y.atspi.Event.Object", "ValueChanged") => {
            let mut kinds = vec![EventKind::ValueChanged];
            if matches!(target_role, Some(Role::TextField | Role::TextArea)) {
                kinds.push(EventKind::TextChanged);
            }
            kinds
        }

        ("org.a11y.atspi.Event.Object", "Announcement") => vec![EventKind::Announcement],

        // Document.LoadComplete — a page loaded, its reloads too.
        ("org.a11y.atspi.Event.Document", "LoadComplete") => vec![EventKind::DocumentLoaded],

        // BoundsChanged / VisibleDataChanged / AttributesChanged — no cross
        // platform EventKind; skip.
        (
            "org.a11y.atspi.Event.Object",
            "BoundsChanged" | "VisibleDataChanged" | "AttributesChanged" | "ModelChanged"
            | "ColumnReordered" | "RowReordered" | "ColumnInserted" | "RowInserted"
            | "ColumnDeleted" | "RowDeleted",
        ) => vec![],

        // Window interface.
        ("org.a11y.atspi.Event.Window", "Create") => vec![EventKind::WindowOpened],
        ("org.a11y.atspi.Event.Window", "Destroy") => vec![EventKind::WindowClosed],
        ("org.a11y.atspi.Event.Window", "Activate") => vec![EventKind::WindowActivated],
        ("org.a11y.atspi.Event.Window", "Deactivate") => vec![EventKind::WindowDeactivated],
        ("org.a11y.atspi.Event.Window", "Minimize") => vec![EventKind::WindowDeactivated],
        ("org.a11y.atspi.Event.Window", "Restore") => vec![EventKind::WindowActivated],
        // Move/Resize/Maximize/Raise/Lower have no cross-platform kind.
        ("org.a11y.atspi.Event.Window", _) => vec![],

        // Document load events — not modelled cross-platform.
        ("org.a11y.atspi.Event.Document", _) => vec![],

        _ => vec![],
    }
}

// ── Snapshot helpers (free functions, independent of LinuxProvider) ─────────

/// Build a best-effort `ElementData` snapshot for an event's source element.
///
/// Returns `None` only on a catastrophic D-Bus failure — role/name/value
/// fetches are individually fallible and the snapshot survives their
/// absence (the element may have been destroyed just before we queried it).
pub(crate) fn build_event_snapshot(
    conn: &Connection,
    aref: &AccessibleRef,
    pid: Option<u32>,
) -> Option<ElementData> {
    let role_name = get_role_name(conn, aref).unwrap_or_default();
    let role_num = if role_name.is_empty() {
        get_role_number(conn, aref).unwrap_or(0)
    } else {
        0
    };
    let mut role = if !role_name.is_empty() {
        let r = map_atspi_role(&role_name);
        if r == Role::Unknown {
            let n = get_role_number(conn, aref).unwrap_or(0);
            map_atspi_role_number(n)
        } else {
            r
        }
    } else {
        map_atspi_role_number(role_num)
    };

    // State bits (used both for role refinement and for StateSet).
    // A failed GetState is `None` — preserve that as `state_read_ok = false`
    // rather than collapsing it into an empty bitset: an unread state is not
    // a state read as absent (same rule `LinuxProvider::parse_states_with_bits`
    // applies via `state_bits`), and `states_from_bits` needs it to leave
    // `minimized` unknown instead of reporting `Some(false)`.
    let (bits, state_read_ok) = match get_state(conn, aref) {
        Some(state_bits) => (bits_from_u32s(&state_bits), true),
        None => (0, false),
    };

    // Refine TextArea → TextField for single-line text widgets (mirrors
    // the role-resolution logic on the main query path).
    if role == Role::TextArea {
        const MULTI_LINE: u64 = 1 << 17;
        if (bits & MULTI_LINE) == 0 {
            role = Role::TextField;
        }
    }

    let name = get_name(conn, aref).filter(|s| !s.is_empty());
    let value = get_value(conn, aref, role);

    let (numeric_value, min_value, max_value) = if matches!(
        role,
        Role::Slider | Role::ProgressBar | Role::ScrollBar | Role::SpinButton
    ) {
        get_numeric(conn, aref)
    } else {
        (None, None, None)
    };

    let states = states_from_bits(bits, role, state_read_ok);

    let raw = {
        let raw_role = if role_name.is_empty() {
            format!("role_num:{}", role_num)
        } else {
            role_name
        };
        let mut raw = HashMap::new();
        raw.insert("atspi_role".into(), serde_json::Value::String(raw_role));
        raw.insert(
            "bus_name".into(),
            serde_json::Value::String(aref.bus_name.clone()),
        );
        raw.insert(
            "object_path".into(),
            serde_json::Value::String(aref.path.clone()),
        );
        raw
    };

    Some(
        ElementParts {
            role,
            name,
            value,
            description: None,
            bounds: None,
            actions: vec![],
            states,
            numeric_value,
            min_value,
            max_value,
            stable_id: Some(aref.path.clone()),
            pid,
            raw,
            // Snapshots are read-only targets, not live handles into the main
            // provider's cache. Consumers wanting to drive actions must
            // re-resolve through the regular locator path.
            handle: 0,
        }
        .into(),
    )
}

fn make_proxy<'a>(conn: &'a Connection, bus: &str, path: &str, iface: &str) -> Option<Proxy<'a>> {
    zbus::blocking::proxy::Builder::<Proxy>::new(conn)
        .destination(bus.to_owned())
        .ok()?
        .path(path.to_owned())
        .ok()?
        .interface(iface.to_owned())
        .ok()?
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .ok()
}

fn get_role_name(conn: &Connection, aref: &AccessibleRef) -> Option<String> {
    let proxy = make_proxy(
        conn,
        &aref.bus_name,
        &aref.path,
        "org.a11y.atspi.Accessible",
    )?;
    let reply = proxy.call_method("GetRoleName", &()).ok()?;
    reply.body().deserialize::<String>().ok()
}

fn get_role_number(conn: &Connection, aref: &AccessibleRef) -> Option<u32> {
    let proxy = make_proxy(
        conn,
        &aref.bus_name,
        &aref.path,
        "org.a11y.atspi.Accessible",
    )?;
    let reply = proxy.call_method("GetRole", &()).ok()?;
    reply.body().deserialize::<u32>().ok()
}

/// The process owning a connection on the bus.
fn connection_pid(conn: &Connection, bus_name: &str) -> Option<u32> {
    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetConnectionUnixProcessID",
            &(bus_name,),
        )
        .ok()?;
    reply
        .body()
        .deserialize::<u32>()
        .ok()
        .filter(|pid| *pid > 0)
}

impl LinuxProvider {
    /// Every application's accessibility events: the whole desktop, as a
    /// screen reader hears it ([`subscribe_desktop`]).
    pub fn subscribe_desktop(&self) -> Result<Subscription> {
        subscribe_desktop(None)
    }

    /// [`Self::subscribe_desktop`], for the kinds `wanted` takes alone: a signal of another
    /// kind is dropped before its element is read.
    pub fn subscribe_desktop_kinds(&self, wanted: KindFilter) -> Result<Subscription> {
        subscribe_desktop(Some(wanted))
    }
}

fn get_name(conn: &Connection, aref: &AccessibleRef) -> Option<String> {
    let proxy = make_proxy(
        conn,
        &aref.bus_name,
        &aref.path,
        "org.a11y.atspi.Accessible",
    )?;
    proxy.get_property::<String>("Name").ok()
}

fn get_state(conn: &Connection, aref: &AccessibleRef) -> Option<Vec<u32>> {
    let proxy = make_proxy(
        conn,
        &aref.bus_name,
        &aref.path,
        "org.a11y.atspi.Accessible",
    )?;
    let reply = proxy.call_method("GetState", &()).ok()?;
    reply.body().deserialize::<Vec<u32>>().ok()
}

fn get_value(conn: &Connection, aref: &AccessibleRef, role: Role) -> Option<String> {
    if matches!(
        role,
        Role::Application
            | Role::Window
            | Role::Dialog
            | Role::Group
            | Role::MenuBar
            | Role::Toolbar
            | Role::TabGroup
            | Role::SplitGroup
            | Role::Table
            | Role::TableRow
            | Role::Separator
    ) {
        return None;
    }

    // Prefer Text.GetText for roles that carry text (text fields, labels,
    // combo boxes) — Value.CurrentValue returns 0.0 for text elements on
    // some AT-SPI adapters.
    if let Some(proxy) = make_proxy(conn, &aref.bus_name, &aref.path, "org.a11y.atspi.Text") {
        if let Ok(char_count) = proxy.get_property::<i32>("CharacterCount") {
            if char_count > 0 {
                if let Ok(reply) = proxy.call_method("GetText", &(0i32, char_count)) {
                    if let Ok(text) = reply.body().deserialize::<String>() {
                        if !text.is_empty() {
                            return Some(text);
                        }
                    }
                }
            }
        }
    }

    if let Some(proxy) = make_proxy(conn, &aref.bus_name, &aref.path, "org.a11y.atspi.Value") {
        if let Ok(v) = proxy.get_property::<f64>("CurrentValue") {
            return Some(v.to_string());
        }
    }

    None
}

fn get_numeric(conn: &Connection, aref: &AccessibleRef) -> (Option<f64>, Option<f64>, Option<f64>) {
    let Some(proxy) = make_proxy(conn, &aref.bus_name, &aref.path, "org.a11y.atspi.Value") else {
        return (None, None, None);
    };
    (
        proxy.get_property::<f64>("CurrentValue").ok(),
        proxy.get_property::<f64>("MinimumValue").ok(),
        proxy.get_property::<f64>("MaximumValue").ok(),
    )
}

/// AT-SPI2 packs its state bitfield into a `Vec<u32>` of length 2 (low and
/// high halves). Collapse into a single u64 for easier bit-testing.
fn bits_from_u32s(state_bits: &[u32]) -> u64 {
    match state_bits.len() {
        0 => 0,
        1 => state_bits[0] as u64,
        _ => (state_bits[0] as u64) | ((state_bits[1] as u64) << 32),
    }
}

/// Convert an AT-SPI2 state bitfield into a [`StateSet`].
///
/// `state_read_ok` reports whether the bits came from a successful `GetState`
/// read. When it is `false` the read produced no evidence, and the window
/// flags must stay tri-state-unknown (`None`) rather than read-as-absent —
/// the same rule `LinuxProvider::parse_states_with_bits` applies through
/// `state_bits`, kept in sync here.
fn states_from_bits(bits: u64, role: Role, state_read_ok: bool) -> StateSet {
    // Same bit positions as `LinuxProvider::parse_states` — kept in sync.
    // Each constant is `1 << ordinal` of the `AtspiStateType` enum in
    // at-spi2-core's `atspi/atspi-constants.h` (e.g. ICONIFIED=15,
    // MODAL=16, VISIBLE=30); the state set arrives packed as one bit per
    // state ordinal.
    const ACTIVE: u64 = 1 << 1;
    const BUSY: u64 = 1 << 3;
    const CHECKED: u64 = 1 << 4;
    const PRESSED: u64 = 1 << 20;
    const EDITABLE: u64 = 1 << 7;
    const ENABLED: u64 = 1 << 8;
    const EXPANDABLE: u64 = 1 << 9;
    const EXPANDED: u64 = 1 << 10;
    const FOCUSABLE: u64 = 1 << 11;
    const FOCUSED: u64 = 1 << 12;
    const ICONIFIED: u64 = 1 << 15;
    const MODAL: u64 = 1 << 16;
    const SELECTED: u64 = 1 << 23;
    const SENSITIVE: u64 = 1 << 24;
    const SHOWING: u64 = 1 << 25;
    const VISIBLE: u64 = 1 << 30;
    const INDETERMINATE: u64 = 1 << 32;
    const REQUIRED: u64 = 1 << 33;

    let enabled = (bits & ENABLED) != 0 || (bits & SENSITIVE) != 0;
    let visible = (bits & VISIBLE) != 0 || (bits & SHOWING) != 0;

    // A toggle button (the Switch role, by AT-SPI's "toggle button") is on when pressed:
    // Chrome gives aria-pressed as PRESSED, GTK 4 its toggle button's state too.
    let on = match role {
        Role::Switch => CHECKED | PRESSED,
        _ => CHECKED,
    };
    let checked = match role {
        Role::CheckBox | Role::RadioButton | Role::MenuItem | Role::Switch => {
            if (bits & INDETERMINATE) != 0 {
                Some(Toggled::Mixed)
            } else if (bits & on) != 0 {
                Some(Toggled::On)
            } else {
                Some(Toggled::Off)
            }
        }
        _ => None,
    };

    let expanded = if (bits & EXPANDABLE) != 0 {
        Some((bits & EXPANDED) != 0)
    } else {
        None
    };

    StateParts {
        enabled,
        visible,
        focused: (bits & FOCUSED) != 0,
        // AT-SPI `ACTIVE` = foreground window/frame; kept in sync with
        // `LinuxProvider::parse_states`.
        active: (bits & ACTIVE) != 0,
        checked,
        selected: (bits & SELECTED) != 0,
        expanded,
        editable: (bits & EDITABLE) != 0,
        focusable: (bits & FOCUSABLE) != 0,
        modal: (bits & MODAL) != 0,
        required: (bits & REQUIRED) != 0,
        busy: (bits & BUSY) != 0,
        minimized: if matches!(role, Role::Window | Role::Dialog) {
            if state_read_ok {
                Some((bits & ICONIFIED) != 0)
            } else {
                // Could not read the state: say so, don't misreport the
                // window as not-minimized (mirrors `parse_states_with_bits`).
                None
            }
        } else {
            None
        },
        maximized: None,
        fullscreen: None,
    }
    .into()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_event_interface_heard_is_registered_with_the_registry() {
        let classes: Vec<String> = EVENT_INTERFACES
            .iter()
            .filter_map(|interface| interface.rsplit('.').next())
            .map(|class| format!("{}:", class.to_lowercase()))
            .collect();
        assert_eq!(classes, REGISTERED_EVENTS);
    }

    #[test]
    fn focus_signal_emits_focus_changed() {
        let kinds = signal_to_kinds("org.a11y.atspi.Event.Focus", "Focus", "", 0, None);
        assert_eq!(kinds, vec![EventKind::FocusChanged]);
    }

    #[test]
    fn state_changed_focused_true_emits_both_kinds() {
        // GTK4 skips Focus:Focus sometimes — `focused=true` StateChanged
        // must still raise FocusChanged so consumers don't miss it.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "StateChanged",
            "focused",
            1,
            None,
        );
        assert!(kinds.contains(&EventKind::FocusChanged));
        assert!(kinds.iter().any(|k| matches!(
            k,
            EventKind::StateChanged {
                flag: StateFlag::Focused,
                value: true
            }
        )));
    }

    #[test]
    fn state_changed_focused_false_is_state_only() {
        // Losing focus shouldn't fire FocusChanged — that's reserved for
        // the element that gained focus.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "StateChanged",
            "focused",
            0,
            None,
        );
        assert_eq!(
            kinds,
            vec![EventKind::StateChanged {
                flag: StateFlag::Focused,
                value: false
            }]
        );
    }

    #[test]
    fn state_changed_checked_maps_value() {
        let on = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "StateChanged",
            "checked",
            1,
            None,
        );
        assert_eq!(
            on,
            vec![EventKind::StateChanged {
                flag: StateFlag::Checked,
                value: true
            }]
        );
        let off = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "StateChanged",
            "checked",
            0,
            None,
        );
        assert_eq!(
            off,
            vec![EventKind::StateChanged {
                flag: StateFlag::Checked,
                value: false
            }]
        );
    }

    #[test]
    fn state_changed_enabled_and_sensitive_collapse() {
        // AT-SPI2 exposes both `enabled` and `sensitive`; xa11y's cross
        // platform model collapses them onto `Enabled`.
        for name in ["enabled", "sensitive"] {
            let kinds =
                signal_to_kinds("org.a11y.atspi.Event.Object", "StateChanged", name, 1, None);
            assert_eq!(
                kinds,
                vec![EventKind::StateChanged {
                    flag: StateFlag::Enabled,
                    value: true
                }],
                "state '{name}' should map to Enabled"
            );
        }
    }

    #[test]
    fn state_changed_iconified_maps_to_minimized() {
        // Wiring Object:StateChanged(iconified) is the only thing that makes
        // the public StateFlag::Minimized observable on Linux — without it
        // the flag would only ever surface as a snapshot, never as an event.
        for value in [0, 1] {
            let kinds = signal_to_kinds(
                "org.a11y.atspi.Event.Object",
                "StateChanged",
                "iconified",
                value,
                None,
            );
            assert_eq!(
                kinds,
                vec![EventKind::StateChanged {
                    flag: StateFlag::Minimized,
                    value: value != 0,
                }],
                "iconified={value}"
            );
        }
    }

    #[test]
    fn state_changed_unknown_state_is_dropped() {
        // Unknown AT-SPI state names must not synthesize events.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "StateChanged",
            "expandable",
            1,
            None,
        );
        assert!(kinds.is_empty());
    }

    #[test]
    fn children_changed_emits_structure_changed() {
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "ChildrenChanged",
            "add",
            0,
            None,
        );
        assert_eq!(kinds, vec![EventKind::StructureChanged]);
    }

    #[test]
    fn property_change_name_emits_name_changed() {
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "PropertyChange",
            "accessible-name",
            0,
            None,
        );
        assert_eq!(kinds, vec![EventKind::NameChanged]);
    }

    #[test]
    fn property_change_description_is_dropped() {
        // Description changes don't map to any cross-platform event kind.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "PropertyChange",
            "accessible-description",
            0,
            None,
        );
        assert!(kinds.is_empty());
    }

    #[test]
    fn property_change_value_emits_value_changed() {
        // AccessKit's AT-SPI bridge emits Object:PropertyChange with
        // detail="accessible-value" for slider/range value changes.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "PropertyChange",
            "accessible-value",
            0,
            Some(Role::Slider),
        );
        assert_eq!(kinds, vec![EventKind::ValueChanged]);
    }

    #[test]
    fn property_change_value_on_text_role_also_emits_text_changed() {
        // If a text-role element's value changes via PropertyChange, the
        // provider synthesizes TextChanged too so the macOS/Windows text
        // pattern carries over to Linux.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "PropertyChange",
            "accessible-value",
            0,
            Some(Role::TextField),
        );
        assert!(kinds.contains(&EventKind::ValueChanged));
        assert!(kinds.contains(&EventKind::TextChanged));
    }

    #[test]
    fn text_changed_maps_directly() {
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "TextChanged",
            "insert",
            0,
            None,
        );
        assert_eq!(kinds, vec![EventKind::TextChanged]);
    }

    #[test]
    fn value_changed_on_text_role_also_emits_text_changed() {
        // AccessKit's AT-SPI bridge publishes text mutations via
        // Object:ValueChanged; the events module must synthesize the
        // cross-platform TextChanged signal too so consumers match.
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "ValueChanged",
            "",
            0,
            Some(Role::TextField),
        );
        assert!(kinds.contains(&EventKind::ValueChanged));
        assert!(kinds.contains(&EventKind::TextChanged));

        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "ValueChanged",
            "",
            0,
            Some(Role::TextArea),
        );
        assert!(kinds.contains(&EventKind::TextChanged));
    }

    #[test]
    fn value_changed_on_slider_is_value_only() {
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "ValueChanged",
            "",
            0,
            Some(Role::Slider),
        );
        assert_eq!(kinds, vec![EventKind::ValueChanged]);
    }

    #[test]
    fn window_signals_map_to_window_kinds() {
        let cases = [
            ("Create", EventKind::WindowOpened),
            ("Destroy", EventKind::WindowClosed),
            ("Activate", EventKind::WindowActivated),
            ("Deactivate", EventKind::WindowDeactivated),
            ("Minimize", EventKind::WindowDeactivated),
            ("Restore", EventKind::WindowActivated),
        ];
        for (member, expected) in cases {
            let kinds = signal_to_kinds("org.a11y.atspi.Event.Window", member, "", 0, None);
            assert_eq!(
                kinds,
                vec![expected.clone()],
                "window member '{member}' should map to {expected:?}",
            );
        }
    }

    #[test]
    fn selection_changed_maps_to_selection() {
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "SelectionChanged",
            "",
            0,
            None,
        );
        assert_eq!(kinds, vec![EventKind::SelectionChanged]);

        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Object",
            "TextSelectionChanged",
            "",
            0,
            None,
        );
        assert_eq!(kinds, vec![EventKind::SelectionChanged]);
    }

    #[test]
    fn announcement_maps_to_announcement() {
        let kinds = signal_to_kinds("org.a11y.atspi.Event.Object", "Announcement", "", 0, None);
        assert_eq!(kinds, vec![EventKind::Announcement]);
    }

    #[test]
    fn a_documents_load_complete_is_document_loaded() {
        let kinds = signal_to_kinds(
            "org.a11y.atspi.Event.Document",
            "LoadComplete",
            "",
            0,
            Some(Role::WebArea),
        );
        assert_eq!(kinds, vec![EventKind::DocumentLoaded]);
    }

    #[test]
    fn unrecognised_signals_are_dropped() {
        // Interfaces/members that don't map to any EventKind must yield an
        // empty vec so we don't flood consumers with no-op events.
        assert!(signal_to_kinds("org.a11y.atspi.Event.Window", "Move", "", 0, None).is_empty());
        assert!(
            signal_to_kinds("org.a11y.atspi.Event.Object", "BoundsChanged", "", 0, None).is_empty()
        );
        assert!(signal_to_kinds("com.example.OtherBus", "Whatever", "", 0, None).is_empty());
    }

    #[test]
    fn event_interfaces_covers_design_doc() {
        // The design doc mandates that we subscribe to exactly these four
        // AT-SPI2 event interfaces — a future refactor that drops one
        // silently should fail this test.
        assert!(EVENT_INTERFACES.contains(&"org.a11y.atspi.Event.Object"));
        assert!(EVENT_INTERFACES.contains(&"org.a11y.atspi.Event.Window"));
        assert!(EVENT_INTERFACES.contains(&"org.a11y.atspi.Event.Focus"));
        assert!(EVENT_INTERFACES.contains(&"org.a11y.atspi.Event.Document"));
    }

    #[test]
    fn bits_from_u32s_handles_short_and_long_arrays() {
        assert_eq!(bits_from_u32s(&[]), 0);
        assert_eq!(bits_from_u32s(&[0x1234]), 0x1234);
        // The u32[1] is the high 32 bits.
        assert_eq!(bits_from_u32s(&[0x11, 0x22]), 0x22_0000_0011u64);
    }

    #[test]
    fn states_from_bits_checked_requires_toggleable_role() {
        // `Checked` only makes sense on checkbox-like roles — other roles
        // should leave it as None even when the bit is set.
        const CHECKED: u64 = 1 << 4;
        let s = states_from_bits(CHECKED, Role::Button, true);
        assert!(s.checked.is_none());
        let s = states_from_bits(CHECKED, Role::CheckBox, true);
        assert_eq!(s.checked, Some(Toggled::On));
    }

    /// A toggle button pressed (Chrome's aria-pressed="true") is on; a check box is not on by
    /// being pressed.
    #[test]
    fn states_from_bits_a_pressed_toggle_button_is_on() {
        const PRESSED: u64 = 1 << 20;
        assert_eq!(
            states_from_bits(PRESSED, Role::Switch, true).checked,
            Some(Toggled::On)
        );
        assert_eq!(
            states_from_bits(PRESSED, Role::CheckBox, true).checked,
            Some(Toggled::Off)
        );
    }

    #[test]
    fn states_from_bits_collapses_enabled_sensitive() {
        const ENABLED: u64 = 1 << 8;
        const SENSITIVE: u64 = 1 << 24;
        // Either bit on its own should flip the enabled flag.
        assert!(states_from_bits(ENABLED, Role::Button, true).enabled);
        assert!(states_from_bits(SENSITIVE, Role::Button, true).enabled);
        assert!(!states_from_bits(0, Role::Button, true).enabled);
    }

    #[test]
    fn states_from_bits_leaves_minimized_unknown_when_read_failed() {
        // A failed GetState is not evidence the window is not minimized:
        // `state_read_ok = false` must leave `minimized` as None (tri-state
        // contract, mirrored with `parse_states_with_bits`), not report
        // `Some(false)` from the defaulted zero bitset.
        const ICONIFIED: u64 = 1 << 15;
        let read_ok = states_from_bits(ICONIFIED, Role::Window, true);
        assert_eq!(read_ok.minimized, Some(true));
        let read_ok = states_from_bits(0, Role::Window, true);
        assert_eq!(read_ok.minimized, Some(false));
        let failed = states_from_bits(0, Role::Window, false);
        assert_eq!(failed.minimized, None, "unread state must stay unknown");
        // Role guard is unchanged: a non-window role stays None regardless.
        let failed = states_from_bits(0, Role::Group, false);
        assert_eq!(failed.minimized, None);
    }
}
