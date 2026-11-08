//! Okur's keyboard on X11: the engine sees every key and decides each one, as Windows' low-level
//! hook lets it (docs/plans/2026-11-05-linux-first.md, "The keyboard on X11"; each point below
//! proved in the Linux lab by `skills/linux-lab/scripts/probe-grab-all.sh`):
//! - every key but the modifiers is grabbed synchronously through XInput 2, under any modifiers,
//!   on every top-level window, not the root: X looks for a passive grab from the root down, so
//!   a window manager's shortcuts on the root stay its own whenever they were grabbed;
//! - on an application's own window, not the frame a window manager puts it in: X reports a grab
//!   on an ancestor of the window that has the focus as the focus leaving for it, and GTK
//!   deactivated its window around every key grabbed on its frame (the lab's 304, under
//!   fluxbox); a grab on the focus window itself moves no focus;
//! - a key the engine keeps is let go asynchronously, one it passes is replayed to the
//!   application (0.3 ms median from the decision to the application in the lab);
//! - a watchdog in its own thread replays a key the engine has not answered in
//!   [`WATCHDOG`]: X11 has no hook timeout, and a hook that stops answering freezes the
//!   keyboard (it did in the lab);
//! - the modifiers, and every key's release, come from XInput 2's raw events, which X sends
//!   whatever holds a grab: the Okur key and ScrollLock held, and the keys the engine is told of
//!   without deciding them.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use okur_core::input::{Key, KeyAction, KeyEvent};
use okur_linux_model::keys::{self, Held};
use okur_runtime::hook::{HookCallback, KeyboardHook};
use okur_runtime::RuntimeError;
use x11rb::connection::Connection;
use x11rb::protocol::ge::ConnectionExt as _;
use x11rb::protocol::xinput::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{self, ConnectionExt as _};
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

/// How long a key waits for the engine before the watchdog replays it to the application.
pub const WATCHDOG: Duration = Duration::from_millis(200);

/// The modifiers each key is grabbed under: XInput 2's "any modifier", one grab per key and
/// window. Each combination apart (Shift, Control, Alt and Super, with Caps Lock and Num Lock
/// on or off: 64) made 626,000 grabs over the lab's 42 top-level windows, and the X server, which
/// checks each new grab against a window's others, delivered no key for longer than the test
/// ran. On application windows the grab meets no window manager's shortcut: X finds those on
/// the root first, whatever the modifiers.
fn modifier_sets() -> Vec<u32> {
    vec![1 << 31]
}

fn hook_error(what: &str, error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::Hook(format!("X11 keyboard: {what}: {error}"))
}

/// A grabbed key the engine has not answered yet, which the watchdog may let go.
#[derive(Clone, Copy)]
struct Pending {
    device: u16,
    time: u32,
    since: Instant,
}

/// The keyboard as X11 maps it: each keycode's keysyms, and which keycodes are modifiers, with
/// the state bit each sets while held.
struct Keymap {
    min: u8,
    per_code: usize,
    keysyms: Vec<u32>,
    modifiers: Vec<(u8, u32)>,
}

impl Keymap {
    fn read(conn: &RustConnection) -> Result<Self, RuntimeError> {
        let setup = conn.setup();
        let (min, max) = (setup.min_keycode, setup.max_keycode);
        let mapping = conn
            .get_keyboard_mapping(min, max.saturating_sub(min).saturating_add(1))
            .map_err(|e| hook_error("keyboard mapping", e))?
            .reply()
            .map_err(|e| hook_error("keyboard mapping", e))?;
        // Eight rows, Shift, Lock, Control and Mod1 to Mod5, of the same number of keycodes.
        let rows = conn
            .get_modifier_mapping()
            .map_err(|e| hook_error("modifier mapping", e))?
            .reply()
            .map_err(|e| hook_error("modifier mapping", e))?
            .keycodes;
        let per_row = (rows.len() / 8).max(1);
        let modifiers = rows
            .iter()
            .enumerate()
            .filter(|(_, code)| **code != 0)
            .map(|(i, code)| (*code, 1u32 << (i / per_row)))
            .collect();
        Ok(Self {
            min,
            per_code: usize::from(mapping.keysyms_per_keycode),
            keysyms: mapping.keysyms,
            modifiers,
        })
    }

    /// The keysym at `level` (0 without Shift, 1 with it), or none.
    fn keysym(&self, code: u8, level: usize) -> u32 {
        let index = usize::from(code.saturating_sub(self.min)) * self.per_code + level;
        self.keysyms.get(index).copied().unwrap_or(0)
    }

    fn is_modifier(&self, code: u8) -> bool {
        self.modifier_bit(code).is_some()
    }

    /// The state bit a modifier key sets while held.
    fn modifier_bit(&self, code: u8) -> Option<u32> {
        self.modifiers
            .iter()
            .find(|(c, _)| *c == code)
            .map(|(_, bit)| *bit)
    }

    fn codes(&self) -> impl Iterator<Item = u8> + '_ {
        let count = self.keysyms.len() / self.per_code.max(1);
        (0..count)
            .filter_map(|i| u8::try_from(i).ok())
            .map(|i| self.min.saturating_add(i))
            .filter(|code| !self.is_modifier(*code))
    }

    fn event(&self, code: u8, state: u32, held: Held, is_down: bool, time: u32) -> KeyEvent {
        let shifted = state & keys::state::SHIFT != 0;
        let typed = self.keysym(code, usize::from(shifted));
        KeyEvent {
            key: keys::key(code, self.keysym(code, 0)),
            modifiers: keys::modifiers(state, held),
            is_down,
            is_repeat: false,
            text: keys::character(typed),
            time_ms: u64::from(time),
            caps_lock_on: Some(state & keys::state::LOCK != 0),
        }
    }
}

/// The keyboard hook for an X11 session (or XWayland's X clients).
#[derive(Default)]
pub struct X11KeyboardHook {
    running: Option<Running>,
}

struct Running {
    conn: Arc<RustConnection>,
    stop: Arc<AtomicBool>,
    wake: xproto::Window,
    threads: Vec<JoinHandle<()>>,
}

impl X11KeyboardHook {
    pub fn new() -> Self {
        Self::default()
    }
}

impl KeyboardHook for X11KeyboardHook {
    fn start(&mut self, callback: HookCallback) -> Result<(), RuntimeError> {
        if self.running.is_some() {
            return Ok(());
        }
        let (conn, screen) = x11rb::connect(None).map_err(|e| hook_error("connect", e))?;
        let conn = Arc::new(conn);
        let root = conn
            .setup()
            .roots
            .get(screen)
            .map(|s| s.root)
            .ok_or_else(|| hook_error("screen", "none"))?;
        // XInput 2's events are X's generic events, which the server sends only to a client
        // that has asked for the Generic Event extension's version: Xlib asks by itself, x11rb
        // does not (the lab's first run of this hook heard no key).
        conn.ge_query_version(1, 0)
            .map_err(|e| hook_error("Generic Event extension", e))?
            .reply()
            .map_err(|e| hook_error("Generic Event extension", e))?;
        conn.xinput_xi_query_version(2, 2)
            .map_err(|e| hook_error("XInput 2", e))?
            .reply()
            .map_err(|e| hook_error("XInput 2", e))?;
        let keyboard = conn
            .xinput_xi_query_device(xinput::Device::ALL_MASTER)
            .map_err(|e| hook_error("devices", e))?
            .reply()
            .map_err(|e| hook_error("devices", e))?
            .infos
            .into_iter()
            .find(|info| info.type_ == xinput::DeviceType::MASTER_KEYBOARD)
            .map(|info| info.deviceid)
            .ok_or_else(|| hook_error("devices", "no master keyboard"))?;
        let keymap = Keymap::read(&conn)?;

        // New top-level windows are grabbed as they come; every key's raw press and release.
        conn.change_window_attributes(
            root,
            &xproto::ChangeWindowAttributesAux::new()
                .event_mask(xproto::EventMask::SUBSTRUCTURE_NOTIFY),
        )
        .map_err(|e| hook_error("root events", e))?;
        conn.xinput_xi_select_events(
            root,
            &[xinput::EventMask {
                deviceid: xinput::Device::ALL_MASTER.into(),
                mask: vec![
                    xinput::XIEventMask::RAW_KEY_PRESS | xinput::XIEventMask::RAW_KEY_RELEASE,
                ],
            }],
        )
        .map_err(|e| hook_error("raw events", e))?;
        // A window of Okur's own, to wake the event loop when the hook stops.
        let wake = conn.generate_id().map_err(|e| hook_error("id", e))?;
        conn.create_window(
            0,
            wake,
            root,
            0,
            0,
            1,
            1,
            0,
            xproto::WindowClass::INPUT_ONLY,
            0,
            &xproto::CreateWindowAux::new(),
        )
        .map_err(|e| hook_error("window", e))?;
        let sets = modifier_sets();
        let wm_state = conn
            .intern_atom(false, b"WM_STATE")
            .map_err(|e| hook_error("WM_STATE", e))?
            .reply()
            .map_err(|e| hook_error("WM_STATE", e))?
            .atom;
        let tops = conn
            .query_tree(root)
            .map_err(|e| hook_error("windows", e))?
            .reply()
            .map_err(|e| hook_error("windows", e))?
            .children;
        let mut grabbed = 0;
        for window in tops.into_iter().filter(|w| *w != wake) {
            let client = client_in(&conn, window, wm_state).unwrap_or(window);
            grab_window(&conn, &keymap, keyboard, client, &sets);
            grabbed += 1;
        }
        tracing::debug!(
            windows = grabbed,
            keys = keymap.codes().count(),
            keyboard,
            "keyboard: every key grabbed on the top-level windows"
        );
        conn.flush().map_err(|e| hook_error("flush", e))?;

        let stop = Arc::new(AtomicBool::new(false));
        let pending: Arc<Mutex<Option<Pending>>> = Arc::new(Mutex::new(None));
        let watchdog = {
            let (conn, stop, pending) =
                (Arc::clone(&conn), Arc::clone(&stop), Arc::clone(&pending));
            std::thread::spawn(move || watch(&conn, &stop, &pending))
        };
        let events = {
            let (conn, stop, pending) =
                (Arc::clone(&conn), Arc::clone(&stop), Arc::clone(&pending));
            std::thread::spawn(move || {
                let mut run = Run {
                    conn: &conn,
                    keymap,
                    keyboard,
                    sets,
                    root,
                    held: Held::default(),
                    modifiers_down: 0,
                    wake,
                    callback,
                    pending,
                };
                run.events(&stop);
            })
        };
        self.running = Some(Running {
            conn,
            stop,
            wake,
            threads: vec![watchdog, events],
        });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        let Some(running) = self.running.take() else {
            return Ok(());
        };
        running.stop.store(true, Ordering::SeqCst);
        // Wake the event loop; the grabs end with the connection.
        let wake =
            xproto::ClientMessageEvent::new(32, running.wake, xproto::AtomEnum::NONE, [0u32; 5]);
        let _ = running
            .conn
            .send_event(false, running.wake, xproto::EventMask::NO_EVENT, wake);
        let _ = running.conn.flush();
        for thread in running.threads {
            let _ = thread.join();
        }
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.running.is_some()
    }
}

impl Drop for X11KeyboardHook {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Every key but the modifiers on `window`, synchronously, under every modifier set. A window
/// gone before the grab (it closed) is no error.
fn grab_window(
    conn: &RustConnection,
    keymap: &Keymap,
    keyboard: u16,
    window: xproto::Window,
    sets: &[u32],
) {
    for code in keymap.codes() {
        let _ = conn.xinput_xi_passive_grab_device(
            x11rb::CURRENT_TIME,
            window,
            x11rb::NONE,
            u32::from(code),
            keyboard,
            xinput::GrabType::KEYCODE,
            xinput::GrabMode22::SYNC,
            xproto::GrabMode::ASYNC,
            xinput::GrabOwner::NO_OWNER,
            &[u32::from(
                xinput::XIEventMask::KEY_PRESS | xinput::XIEventMask::KEY_RELEASE,
            )],
            sets,
        );
    }
}

/// `grab_window`'s grabs let go.
fn ungrab_window(
    conn: &RustConnection,
    keymap: &Keymap,
    keyboard: u16,
    window: xproto::Window,
    sets: &[u32],
) {
    for code in keymap.codes() {
        let _ = conn.xinput_xi_passive_ungrab_device(
            window,
            u32::from(code),
            keyboard,
            xinput::GrabType::KEYCODE,
            sets,
        );
    }
}

/// The application's own window at or within top-level `window`: the one holding WM_STATE,
/// which a window manager sets on each window it manages, `window` itself or, under a
/// reparenting window manager, a window in its frame. None for a window without (a popup the
/// window manager leaves alone).
fn client_in(
    conn: &RustConnection,
    window: xproto::Window,
    wm_state: xproto::Atom,
) -> Option<xproto::Window> {
    let managed = |w: xproto::Window| {
        conn.get_property(false, w, wm_state, xproto::AtomEnum::ANY, 0, 0)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some_and(|reply| reply.type_ != u32::from(xproto::AtomEnum::NONE))
    };
    if managed(window) {
        return Some(window);
    }
    let children = |w: xproto::Window| {
        conn.query_tree(w)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .map(|reply| reply.children)
            .unwrap_or_default()
    };
    // A frame holds its window as a child, or a child's child (a decoration's own window).
    children(window).into_iter().find_map(|child| {
        if managed(child) {
            Some(child)
        } else {
            children(child).into_iter().find(|w| managed(*w))
        }
    })
}

/// The watchdog: a key the engine has not answered in [`WATCHDOG`] is replayed.
fn watch(conn: &RustConnection, stop: &AtomicBool, pending: &Mutex<Option<Pending>>) {
    while !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(20));
        let overdue = pending
            .lock()
            .ok()
            .and_then(|mut p| p.take_if(|p| p.since.elapsed() >= WATCHDOG));
        if let Some(key) = overdue {
            allow(conn, key, xinput::EventMode::REPLAY_DEVICE);
            tracing::warn!(
                "keyboard: a key unanswered for {WATCHDOG:?} was let go to the application"
            );
        }
    }
}

fn allow(conn: &RustConnection, key: Pending, mode: xinput::EventMode) {
    let _ = conn.xinput_xi_allow_events(key.time, key.device, mode, 0, x11rb::NONE);
    let _ = conn.flush();
}

/// The event loop's state.
struct Run<'a> {
    conn: &'a RustConnection,
    keymap: Keymap,
    keyboard: u16,
    sets: Vec<u32>,
    root: xproto::Window,
    held: Held,
    /// Shift, Control, Alt and Super as held now, from their raw presses and releases.
    modifiers_down: u32,
    wake: xproto::Window,
    callback: HookCallback,
    pending: Arc<Mutex<Option<Pending>>>,
}

impl Run<'_> {
    fn events(&mut self, stop: &AtomicBool) {
        while !stop.load(Ordering::SeqCst) {
            let event = self.conn.wait_for_event();
            if let Ok(event) = &event {
                tracing::trace!(?event, "keyboard: X event");
            }
            match event {
                Ok(Event::XinputKeyPress(event)) => self.grabbed(&event),
                Ok(Event::XinputRawKeyPress(event)) => self.raw(&event, true),
                Ok(Event::XinputRawKeyRelease(event)) => self.raw(&event, false),
                Ok(Event::CreateNotify(event)) if event.parent != self.wake => {
                    grab_window(
                        self.conn,
                        &self.keymap,
                        self.keyboard,
                        event.window,
                        &self.sets,
                    );
                    let _ = self.conn.flush();
                }
                Ok(Event::ReparentNotify(event)) => self.reparented(&event),
                Ok(Event::MappingNotify(_)) => {
                    if let Ok(keymap) = Keymap::read(self.conn) {
                        self.keymap = keymap;
                    }
                }
                Ok(Event::Error(error)) => {
                    tracing::debug!(?error, "keyboard: an X request failed");
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::error!("keyboard: the X connection ended: {error}");
                    return;
                }
            }
        }
    }

    /// A window put in a frame by the window manager (or back on the root): the frame, grabbed as
    /// a new top-level window, lets its keys go, and the window holds them itself.
    fn reparented(&mut self, event: &xproto::ReparentNotifyEvent) {
        if event.parent != self.root {
            ungrab_window(
                self.conn,
                &self.keymap,
                self.keyboard,
                event.parent,
                &self.sets,
            );
        }
        grab_window(
            self.conn,
            &self.keymap,
            self.keyboard,
            event.window,
            &self.sets,
        );
        let _ = self.conn.flush();
    }

    /// A grabbed key's press: the engine decides it, unless the watchdog already let it go.
    fn grabbed(&mut self, event: &xinput::KeyPressEvent) {
        let (Ok(code), time) = (u8::try_from(event.detail), event.time) else {
            return;
        };
        let key = Pending {
            device: event.deviceid,
            time,
            since: Instant::now(),
        };
        if let Ok(mut pending) = self.pending.lock() {
            *pending = Some(key);
        }
        let state = event.mods.effective;
        let key_event = self.keymap.event(code, state, self.held, true, time);
        let action = (self.callback)(key_event);
        // Answered in time: the key is the engine's to keep or pass.
        let ours = self.pending.lock().ok().and_then(|mut p| p.take());
        if let Some(key) = ours {
            let mode = match action {
                KeyAction::Consume => xinput::EventMode::ASYNC_DEVICE,
                KeyAction::PassThrough => xinput::EventMode::REPLAY_DEVICE,
            };
            allow(self.conn, key, mode);
        }
    }

    /// A raw press or release: the keys Okur holds as its own modifiers, and the modifiers and
    /// releases the engine is told of without deciding them (grabbed presses come grabbed).
    fn raw(&mut self, event: &xinput::RawKeyPressEvent, is_down: bool) {
        let Ok(code) = u8::try_from(event.detail) else {
            return;
        };
        let keysym = self.keymap.keysym(code, 0);
        match keys::key(code, keysym) {
            Key::Insert => self.held.insert = is_down,
            Key::CapsLock => self.held.caps_lock = is_down,
            Key::ScrollLock => self.held.scroll_lock = is_down,
            _ => {}
        }
        // A raw event has no state: the modifiers held are kept from their own raw events
        // (Caps Lock and Num Lock, toggles, are not held modifiers).
        let held_bits =
            keys::state::SHIFT | keys::state::CONTROL | keys::state::MOD1 | keys::state::MOD4;
        if let Some(bit) = self
            .keymap
            .modifier_bit(code)
            .filter(|bit| held_bits & bit != 0)
        {
            if is_down {
                self.modifiers_down |= bit;
            } else {
                self.modifiers_down &= !bit;
            }
        }
        if is_down && !self.keymap.is_modifier(code) {
            return;
        }
        let key_event =
            self.keymap
                .event(code, self.modifiers_down, self.held, is_down, event.time);
        let _ = (self.callback)(key_event);
    }
}
