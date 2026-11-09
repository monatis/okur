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
//!   without deciding them;
//! - Caps Lock is Okur's while it runs ([`CapsLock`]): X toggles it on its own as the press
//!   reaches it, whatever the engine decides, so Lock's keys are taken off the modifier map, and
//!   Okur toggles Caps Lock itself when the engine passes a press on;
//! - the keys Okur presses itself ([`X11KeySender`], through XTest: a reading command's arrow,
//!   a key held for one of its dialogs) come back through the grab and are read as plain keys,
//!   with only the modifiers they carry, as Windows reads the keys it marks: the Okur key still
//!   held for CapsLock+O does not make its Down an Okur command;
//! - while a key the engine kept is down, its grab lasts and X sends every key to Okur, where a
//!   key replayed goes nowhere: the keys Okur presses meanwhile (CapsLock+O's Down, sent while O
//!   is down) wait for that key's release.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use okur_core::input::{Key, KeyAction, KeyEvent};
use okur_linux_model::keys::{self, Held};
use okur_runtime::hook::{HookCallback, KeyboardHook};
use okur_runtime::RuntimeError;
use x11rb::connection::{Connection, RequestConnection as _};
use x11rb::protocol::ge::ConnectionExt as _;
use x11rb::protocol::xinput::{self, ConnectionExt as _};
use x11rb::protocol::xkb::{self, ConnectionExt as _};
use x11rb::protocol::xproto::{self, ConnectionExt as _};
use x11rb::protocol::xtest::ConnectionExt as _;
use x11rb::protocol::Event;
use x11rb::rust_connection::RustConnection;

/// Caps Lock's keysym (XK_Caps_Lock).
const CAPS_LOCK_KEYSYM: u32 = 0xffe5;

/// The modifier map's row of Lock (Shift, Lock, Control, Mod1 to Mod5).
const LOCK_ROW: usize = 1;

/// The evdev keycode of Caps Lock: where a run that ended without putting its keysym back
/// left it with none, it is given Caps_Lock again.
const EVDEV_CAPS_LOCK: u8 = 66;

/// Caps Lock made Okur's while it runs (the code review of 2026-11-08, finding 14). X toggles
/// Caps Lock as a press reaches it, whatever the engine decides, where JAWS's CapsLock, the
/// laptop layout's Okur key, toggles only when pressed twice quickly. Taking Lock's keys off the
/// modifier map was not enough: XKB's Caps_Lock action locks Lock by name (the lab's
/// probe-capslock.sh, 2026-11-09). So Caps Lock's keys lose their keysyms while Okur runs (no
/// interpretation, no action) and leave the modifier map's Lock row (no modifier while held);
/// the hook knows them by their keycodes, and Okur toggles Caps Lock itself through XKB when the
/// engine passes a press on (the second of two, or each one where CapsLock is no Okur key).
/// Both are put back as the hook stops.
struct CapsLock {
    /// The modifier map to put back.
    map: Vec<u8>,
    /// Caps Lock's keys, each with the keysyms to put back.
    keys: Vec<(u8, Vec<u32>)>,
    per_code: u8,
}

impl CapsLock {
    /// Caps Lock's keys made Okur's, where XKB and the mapping requests work; none otherwise,
    /// and Caps Lock stays X's (said in the log).
    fn take(conn: &RustConnection, keymap: &Keymap) -> Option<Self> {
        let usable = conn
            .xkb_use_extension(1, 0)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some_and(|reply| reply.supported);
        if !usable {
            tracing::warn!("keyboard: no XKB, Caps Lock stays the system's");
            return None;
        }
        let per_code = u8::try_from(keymap.per_code).ok()?;
        let mut keys: Vec<(u8, Vec<u32>)> = keymap
            .all_codes()
            .filter(|code| keymap.keysym(*code, 0) == CAPS_LOCK_KEYSYM)
            .map(|code| (code, keymap.keysyms_of(code)))
            .collect();
        let mut map = conn.get_modifier_mapping().ok()?.reply().ok()?.keycodes;
        let per_row = map.len() / 8;
        let lock = map.get_mut(per_row * LOCK_ROW..per_row * (LOCK_ROW + 1))?;
        if keys.is_empty() {
            if lock.iter().any(|code| *code != 0) {
                return None;
            }
            // A run before left Caps Lock without its keysym and its modifier: both given back.
            keys.push((EVDEV_CAPS_LOCK, vec![CAPS_LOCK_KEYSYM; keymap.per_code]));
        }
        if lock.iter().all(|code| *code == 0) {
            for (slot, (code, _)) in lock.iter_mut().zip(&keys) {
                *slot = *code;
            }
        }
        let mut without = map.clone();
        if let Some(row) = without.get_mut(per_row * LOCK_ROW..per_row * (LOCK_ROW + 1)) {
            row.fill(0);
        }
        let set = conn
            .set_modifier_mapping(&without)
            .ok()?
            .reply()
            .ok()
            .is_some_and(|reply| reply.status == xproto::MappingStatus::SUCCESS);
        if !set {
            tracing::warn!("keyboard: the modifier map not changed, Caps Lock stays the system's");
            return None;
        }
        let none = vec![0u32; keymap.per_code];
        for (code, _) in &keys {
            let _ = conn.change_keyboard_mapping(1, *code, per_code, &none);
        }
        let _ = conn.flush();
        Some(Self {
            map,
            keys,
            per_code,
        })
    }

    /// Caps Lock's keys, by their keycodes.
    fn codes(&self) -> Vec<u8> {
        self.keys.iter().map(|(code, _)| *code).collect()
    }

    /// Caps Lock on or off now.
    fn is_on(conn: &RustConnection) -> Option<bool> {
        let state = conn
            .xkb_get_state(xkb::ID::USE_CORE_KBD.into())
            .ok()?
            .reply()
            .ok()?;
        Some(state.locked_mods.contains(xproto::ModMask::LOCK))
    }

    /// Caps Lock toggled, as the key would have.
    fn toggle(conn: &RustConnection) {
        let on = Self::is_on(conn).unwrap_or(false);
        let locks = if on {
            xproto::ModMask::from(0u16)
        } else {
            xproto::ModMask::LOCK
        };
        let sent = conn.xkb_latch_lock_state(
            xkb::ID::USE_CORE_KBD.into(),
            xproto::ModMask::LOCK,
            locks,
            false,
            xkb::Group::M1,
            xproto::ModMask::from(0u16),
            false,
            0,
        );
        if sent.is_err() || conn.flush().is_err() {
            tracing::warn!("keyboard: Caps Lock not toggled");
        }
    }

    /// Caps Lock's keysyms and the modifier map as they were.
    fn give_back(&self, conn: &RustConnection) {
        for (code, keysyms) in &self.keys {
            let _ = conn.change_keyboard_mapping(1, *code, self.per_code, keysyms);
        }
        let back = conn
            .set_modifier_mapping(&self.map)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_some_and(|reply| reply.status == xproto::MappingStatus::SUCCESS);
        if !back {
            tracing::warn!("keyboard: the modifier map not put back: Caps Lock may not lock");
        }
    }
}

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

/// How long a key Okur pressed is awaited back through the grab, on Okur's clock as the hook
/// reads the key: a hook kept waiting by the engine reads it late.
const OWN_KEY_WAIT: Duration = Duration::from_secs(2);

/// A key event Okur sends through XTest: a keycode, pressed (true) or released.
type Stroke = (u8, bool);

/// What waits for a kept key's release: a chord's key events, or keys held for a dialog,
/// their modifiers taken as they are when they are sent.
enum Waiting {
    Strokes(Vec<Stroke>),
    Again(Vec<KeyEvent>),
}

/// What the hook and the key sender share: the connection and keymap to press keys with, the
/// keys Okur pressed awaited back, and the keys waiting for a kept key's release.
#[derive(Default)]
struct Keys {
    /// The hook's connection, when the server has XTEST.
    conn: Option<Arc<RustConnection>>,
    /// The hook's keymap, as it is refreshed.
    keymap: Option<Keymap>,
    /// XTEST's own keyboard, which the keys Okur presses come from.
    xtest_device: Option<u16>,
    /// Shift, Control, Alt and Super as held now, from the hook's raw events.
    modifiers_down: u32,
    /// The keys Okur pressed, by keycode, as they were sent.
    own: Vec<(u8, Instant)>,
    /// The grabbed key the engine kept, while its grab lasts: until its release.
    grab: Option<u8>,
    /// What waits for that release.
    deferred: Vec<Waiting>,
    /// The keys down, by keycode, from the hook's raw events: a grab whose key is not down is
    /// over, its release missed.
    down: Vec<u8>,
}

type SharedKeys = Arc<Mutex<Keys>>;

fn lock(keys: &SharedKeys) -> std::sync::MutexGuard<'_, Keys> {
    keys.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Keys {
    /// Sends `waiting` now, or after the release of the key whose grab lasts.
    fn send(&mut self, waiting: Waiting) -> bool {
        if self.conn.is_none() {
            return false;
        }
        if self.grab.is_some_and(|code| !self.down.contains(&code)) {
            tracing::debug!("keyboard: a kept key's release missed: its grab taken as over");
            self.grab_over();
        }
        if self.grab.is_some() {
            self.deferred.push(waiting);
            return true;
        }
        self.send_waiting(waiting)
    }

    /// The kept key's grab over: what waited for it sent, in order.
    fn grab_over(&mut self) -> bool {
        self.grab = None;
        let deferred = std::mem::take(&mut self.deferred);
        deferred
            .into_iter()
            .map(|waiting| self.send_waiting(waiting))
            .filter(|sent| !sent)
            .count()
            == 0
    }

    /// Sends a chord's key events, or the keys held for a dialog with the modifiers down now.
    fn send_waiting(&mut self, waiting: Waiting) -> bool {
        match waiting {
            Waiting::Strokes(strokes) => self.send_now(&strokes),
            Waiting::Again(events) => {
                let Some((strokes, missing)) = self
                    .keymap
                    .as_ref()
                    .map(|keymap| strokes_again(keymap, self.modifiers_down, &events))
                else {
                    return false;
                };
                self.send_now(&strokes) && missing == 0
            }
        }
    }

    /// Sends `strokes` through XTest, each key pressed noted first: the grab may read it back
    /// before this returns.
    fn send_now(&mut self, strokes: &[Stroke]) -> bool {
        let Some(conn) = self.conn.clone() else {
            return false;
        };
        let now = Instant::now();
        self.own.retain(|(_, at)| at.elapsed() < OWN_KEY_WAIT);
        let modifier = |code: u8| self.keymap.as_ref().is_some_and(|k| k.is_modifier(code));
        let pressed: Vec<(u8, Instant)> = strokes
            .iter()
            .filter(|(code, down)| *down && !modifier(*code))
            .map(|(code, _)| (*code, now))
            .collect();
        self.own.extend(pressed);
        let sent = strokes.iter().all(|&(code, down)| {
            let kind = if down {
                xproto::KEY_PRESS_EVENT
            } else {
                xproto::KEY_RELEASE_EVENT
            };
            conn.xtest_fake_input(kind, code, x11rb::CURRENT_TIME, x11rb::NONE, 0, 0, 0)
                .is_ok()
        });
        sent && conn.flush().is_ok()
    }
}

/// The keyboard as X11 maps it: each keycode's keysyms, and which keycodes are modifiers, with
/// the state bit each sets while held.
#[derive(Clone)]
struct Keymap {
    min: u8,
    per_code: usize,
    keysyms: Vec<u32>,
    modifiers: Vec<(u8, u32)>,
    /// Caps Lock's keys while Okur holds them without a keysym ([`CapsLock`]).
    caps_lock: Vec<u8>,
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
            caps_lock: Vec::new(),
        })
    }

    /// The keysym at `level` (0 without Shift, 1 with it), or none; Caps_Lock for Caps Lock's
    /// keys while Okur holds them without one.
    fn keysym(&self, code: u8, level: usize) -> u32 {
        if self.caps_lock.contains(&code) {
            return CAPS_LOCK_KEYSYM;
        }
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
        self.all_codes()
            .filter(|code| !self.is_modifier(*code) && !self.is_caps_lock(*code))
    }

    fn all_codes(&self) -> impl Iterator<Item = u8> + '_ {
        let count = self.keysyms.len() / self.per_code.max(1);
        (0..count)
            .filter_map(|i| u8::try_from(i).ok())
            .map(|i| self.min.saturating_add(i))
    }

    /// Whether `code` is Caps Lock's key: off the modifier map while Okur runs ([`CapsLock`]),
    /// it is a modifier no longer, and still never grabbed.
    fn is_caps_lock(&self, code: u8) -> bool {
        self.keysym(code, 0) == CAPS_LOCK_KEYSYM
    }

    /// Every keysym of `code`, as the keyboard mapping has them.
    fn keysyms_of(&self, code: u8) -> Vec<u32> {
        let start = usize::from(code.saturating_sub(self.min)) * self.per_code;
        self.keysyms
            .get(start..start + self.per_code)
            .map(<[u32]>::to_vec)
            .unwrap_or_default()
    }

    /// The keycode of `key` as the hook reads keycodes, the main block's before the keypad's
    /// (the keypad's 4 is Left only with Num Lock off).
    fn code_of(&self, key: Key) -> Option<u8> {
        let keypad = |code: u8| (0xff80..=0xffbd).contains(&self.keysym(code, 0));
        let mut codes = self.all_codes().filter(|code| {
            self.keysym(*code, 0) != 0 && keys::key(*code, self.keysym(*code, 0)) == key
        });
        let first = codes.next()?;
        if !keypad(first) {
            return Some(first);
        }
        Some(codes.find(|code| !keypad(*code)).unwrap_or(first))
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
            native: u32::from(code),
        }
    }
}

/// The keyboard hook for an X11 session (or XWayland's X clients).
#[derive(Default)]
pub struct X11KeyboardHook {
    running: Option<Running>,
    keys: SharedKeys,
}

struct Running {
    conn: Arc<RustConnection>,
    stop: Arc<AtomicBool>,
    wake: xproto::Window,
    threads: Vec<JoinHandle<()>>,
    caps_lock: Option<CapsLock>,
}

impl X11KeyboardHook {
    pub fn new() -> Self {
        Self::default()
    }

    /// What presses keys for Okur on this hook's display, its keys read back as Okur's own.
    pub fn key_sender(&self) -> X11KeySender {
        X11KeySender {
            keys: Arc::clone(&self.keys),
        }
    }
}

/// Presses keys for Okur through XTest, on the hook's connection, by the hook's keymap: X takes
/// them as typed, and the hook's grab reads them back as Okur's own.
pub struct X11KeySender {
    keys: SharedKeys,
}

impl X11KeySender {
    /// Whether keys can be pressed: the hook runs, on a server with XTEST.
    pub fn can_press(&self) -> bool {
        lock(&self.keys).conn.is_some()
    }

    /// Presses `chord` in order and releases it in reverse; false when a key has no keycode
    /// or keys cannot be pressed.
    pub fn press(&self, chord: &[Key]) -> bool {
        let mut keys = lock(&self.keys);
        let codes: Option<Vec<u8>> = keys
            .keymap
            .as_ref()
            .and_then(|keymap| chord.iter().map(|key| keymap.code_of(*key)).collect());
        let Some(codes) = codes else {
            tracing::debug!(?chord, "keyboard: a key to press has no keycode");
            return false;
        };
        let strokes = codes
            .iter()
            .map(|code| (*code, true))
            .chain(codes.iter().rev().map(|code| (*code, false)))
            .collect();
        keys.send(Waiting::Strokes(strokes))
    }

    /// Presses `events` again, in order, each by its keycode as it came: the Shift, Control,
    /// Alt and Super keys it was pressed with that are not down now pressed around it, those
    /// down now that it was pressed without released around it. False when one has no keycode.
    pub fn press_again(&self, events: &[KeyEvent]) -> bool {
        lock(&self.keys).send(Waiting::Again(events.to_vec()))
    }
}

/// The key events that press `events` again with the modifiers `down` now (X's state bits),
/// and how many of them have no keycode.
fn strokes_again(keymap: &Keymap, down: u32, events: &[KeyEvent]) -> (Vec<Stroke>, usize) {
    use keys::state::{CONTROL, MOD1, MOD4, SHIFT};
    let mut strokes = Vec::new();
    let mut missing = 0;
    for event in events {
        let code = u8::try_from(event.native)
            .ok()
            .filter(|code| *code != 0)
            .or_else(|| keymap.code_of(event.key));
        let Some(code) = code else {
            missing += 1;
            continue;
        };
        let m = &event.modifiers;
        let around: Vec<Stroke> = [
            (m.ctrl, CONTROL, Key::Control),
            (m.alt, MOD1, Key::Alt),
            (m.shift, SHIFT, Key::Shift),
            (m.win, MOD4, Key::Windows),
        ]
        .into_iter()
        .filter(|(wanted, bit, _)| *wanted && down & bit == 0)
        .filter_map(|(_, _, key)| keymap.code_of(key))
        .map(|code| (code, true))
        .collect();
        strokes.extend(around.iter().copied());
        strokes.extend([(code, true), (code, false)]);
        strokes.extend(around.iter().rev().map(|&(code, _)| (code, false)));
    }
    (strokes, missing)
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
        let mut keymap = Keymap::read(&conn)?;
        let caps_lock = CapsLock::take(&conn, &keymap);
        let caps_codes = caps_lock.as_ref().map(CapsLock::codes).unwrap_or_default();
        keymap.caps_lock = caps_codes.clone();

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
        let (mut refused, mut refusing) = (0, 0);
        for window in tops.into_iter().filter(|w| *w != wake) {
            let client = client_in(&conn, window, wm_state).unwrap_or(window);
            let kept = grab_window(&conn, &keymap, keyboard, client, &sets);
            refused += kept;
            refusing += usize::from(kept > 0);
            grabbed += 1;
        }
        warn_refused(refused, refusing);
        tracing::debug!(
            windows = grabbed,
            keys = keymap.codes().count(),
            keyboard,
            "keyboard: every key grabbed on the top-level windows"
        );
        conn.flush().map_err(|e| hook_error("flush", e))?;

        // Keys pressed for Okur through XTest, on this connection, by this keymap.
        {
            let xtest = conn
                .extension_information(x11rb::protocol::xtest::X11_EXTENSION_NAME)
                .ok()
                .flatten()
                .is_some();
            let xtest_device = conn
                .xinput_xi_query_device(xinput::Device::ALL)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .and_then(|reply| {
                    reply
                        .infos
                        .into_iter()
                        .find(|info| {
                            info.type_ == xinput::DeviceType::SLAVE_KEYBOARD
                                && info.attachment == keyboard
                                && info.name.windows(5).any(|w| w == b"XTEST")
                        })
                        .map(|info| info.deviceid)
                });
            if !xtest {
                tracing::warn!("keyboard: no XTEST: Okur cannot press keys itself");
            }
            let mut keys = lock(&self.keys);
            // Nothing kept from a run before: a key then down, never seen up, would hold
            // every key Okur presses.
            *keys = Keys::default();
            keys.conn = xtest.then(|| Arc::clone(&conn));
            keys.keymap = Some(keymap.clone());
            keys.xtest_device = xtest_device;
        }
        let stop = Arc::new(AtomicBool::new(false));
        let pending: Arc<Mutex<Option<Pending>>> = Arc::new(Mutex::new(None));
        let shared = Arc::clone(&self.keys);
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
                    caps_codes,
                    keymap,
                    keyboard,
                    sets,
                    root,
                    held: Held::default(),
                    modifiers_down: 0,
                    wake,
                    callback,
                    pending,
                    keys: shared,
                };
                run.events(&stop);
            })
        };
        self.running = Some(Running {
            conn,
            stop,
            wake,
            threads: vec![watchdog, events],
            caps_lock,
        });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), RuntimeError> {
        let Some(running) = self.running.take() else {
            return Ok(());
        };
        lock(&self.keys).conn = None;
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
        if let Some(caps_lock) = &running.caps_lock {
            caps_lock.give_back(&running.conn);
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

/// Every key but the modifiers on `window`, synchronously, under every modifier set: how many
/// keys another client's grab kept from Okur (a leftover Okur's, whose keys then never reach
/// this one: the code review of 2026-11-08, finding 15). A window gone before the grab (it
/// closed) is no error.
fn grab_window(
    conn: &RustConnection,
    keymap: &Keymap,
    keyboard: u16,
    window: xproto::Window,
    sets: &[u32],
) -> usize {
    let mut cookies = Vec::new();
    for code in keymap.codes() {
        let cookie = conn.xinput_xi_passive_grab_device(
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
        cookies.extend(cookie.ok());
    }
    // The replies come back together: a grab refused names the modifiers it failed under.
    cookies
        .into_iter()
        .filter_map(|cookie| cookie.reply().ok())
        .filter(|reply| !reply.modifiers.is_empty())
        .count()
}

/// Says that keys were kept from Okur on `windows` windows, where any were.
fn warn_refused(refused: usize, windows: usize) {
    if refused > 0 {
        tracing::warn!(
            refused,
            windows,
            "keyboard: keys grabbed by another program, which Okur does not hear on these \
             windows (another Okur still running?)"
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
    /// Caps Lock's keys while Okur holds them ([`CapsLock`]): Okur toggles Caps Lock.
    caps_codes: Vec<u8>,
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
    keys: SharedKeys,
}

impl Run<'_> {
    /// Whether `code`'s press from `source` is a key Okur pressed itself, taken off the keys
    /// awaited: XTEST's keyboard's, as it was sent.
    fn is_own(&self, code: u8, source: u16) -> bool {
        let mut keys = lock(&self.keys);
        if keys.xtest_device.is_some_and(|device| device != source) {
            return false;
        }
        keys.own.retain(|(_, at)| at.elapsed() < OWN_KEY_WAIT);
        let found = keys.own.iter().position(|(c, _)| *c == code);
        found.map(|i| keys.own.remove(i)).is_some()
    }

    /// The kept key's grab over (its release, or a key passed on): the keys waiting sent.
    fn grab_over(&self) {
        if !lock(&self.keys).grab_over() {
            tracing::warn!("keyboard: keys Okur pressed could not be sent");
        }
    }

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
                Ok(Event::CreateNotify(event)) if event.window != self.wake => {
                    let refused = grab_window(
                        self.conn,
                        &self.keymap,
                        self.keyboard,
                        event.window,
                        &self.sets,
                    );
                    warn_refused(refused, 1);
                    let _ = self.conn.flush();
                }
                Ok(Event::ReparentNotify(event)) => self.reparented(&event),
                Ok(Event::MappingNotify(_)) => {
                    if let Ok(mut keymap) = Keymap::read(self.conn) {
                        keymap.caps_lock = self.caps_codes.clone();
                        lock(&self.keys).keymap = Some(keymap.clone());
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
        tracing::debug!(
            window = event.window,
            parent = event.parent,
            "keyboard: a window put in a frame"
        );
        if event.parent != self.root {
            ungrab_window(
                self.conn,
                &self.keymap,
                self.keyboard,
                event.parent,
                &self.sets,
            );
        }
        let refused = grab_window(
            self.conn,
            &self.keymap,
            self.keyboard,
            event.window,
            &self.sets,
        );
        warn_refused(refused, 1);
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
        // A key Okur pressed itself is a plain key: the Okur key held for its command is not
        // held for it.
        let held = if self.is_own(code, event.sourceid) {
            Held::default()
        } else {
            self.held
        };
        let key_event = self.keymap.event(code, state, held, true, time);
        // This press starts a grab, unless one lasts (a key repeating under it): the keys Okur
        // presses while it is being decided wait for it.
        let started = {
            let mut keys = lock(&self.keys);
            if !keys.down.contains(&code) {
                keys.down.push(code);
            }
            if keys.grab.is_some_and(|kept| !keys.down.contains(&kept)) && !keys.grab_over() {
                tracing::warn!("keyboard: keys Okur pressed could not be sent");
            }
            let started = keys.grab.is_none();
            if started {
                keys.grab = Some(code);
            }
            started
        };
        let action = (self.callback)(key_event);
        tracing::debug!(
            code,
            window = event.event,
            child = event.child,
            ?action,
            "keyboard: a grabbed key decided"
        );
        // Answered in time: the key is the engine's to keep or pass.
        let ours = self.pending.lock().ok().and_then(|mut p| p.take());
        let let_go = ours.is_none();
        if let Some(key) = ours {
            let mode = match action {
                KeyAction::Consume => xinput::EventMode::ASYNC_DEVICE,
                KeyAction::PassThrough => xinput::EventMode::REPLAY_DEVICE,
            };
            allow(self.conn, key, mode);
        }
        // Kept, its grab lasts until its release; passed on (or let go by the watchdog), the
        // grab is over.
        if started && (action == KeyAction::PassThrough || let_go) {
            self.grab_over();
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
            lock(&self.keys).modifiers_down = self.modifiers_down;
        }
        // The keys down; the kept key's release ends its grab.
        let released = {
            let mut keys = lock(&self.keys);
            if is_down {
                if !keys.down.contains(&code) {
                    keys.down.push(code);
                }
            } else {
                keys.down.retain(|c| *c != code);
            }
            !is_down && keys.grab == Some(code)
        };
        if released {
            self.grab_over();
        }
        let caps_lock = self.caps_codes.contains(&code);
        if is_down && !self.keymap.is_modifier(code) && !caps_lock {
            return;
        }
        let mut key_event =
            self.keymap
                .event(code, self.modifiers_down, self.held, is_down, event.time);
        if caps_lock && is_down {
            // Its state as it is, for the engine to say it rightly (Caps Lock on before Okur).
            key_event.caps_lock_on = CapsLock::is_on(self.conn);
        }
        let action = (self.callback)(key_event);
        // The engine passed the press on: the key's own toggle, which the map no longer makes.
        if caps_lock && is_down && action == KeyAction::PassThrough {
            CapsLock::toggle(self.conn);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A keyboard of a few keys: the keypad's 4 (Left with Num Lock off) before the gray Left,
    /// A, and the left Shift as a modifier.
    fn keymap() -> Keymap {
        let min = 38;
        let mut keysyms = vec![0; usize::from(114 - min)];
        for (code, keysym) in [(38, 0x61), (50, 0xffe1), (83, 0xff96), (113, 0xff51)] {
            if let Some(slot) = keysyms.get_mut(usize::from(code - min)) {
                *slot = keysym;
            }
        }
        Keymap {
            min,
            per_code: 1,
            keysyms,
            modifiers: vec![(50, keys::state::SHIFT)],
            caps_lock: Vec::new(),
        }
    }

    /// A key held for a dialog is pressed again by its keycode as it came, with the Shift it
    /// was pressed with put around it while Shift is up now; the Shift the user holds now is
    /// never let go or pressed again (a key pressed without it is pressed with it); a key with
    /// no keycode is counted.
    #[test]
    fn held_keys_are_pressed_again_with_their_own_modifiers() {
        let keymap = keymap();
        let mut a = KeyEvent::new(Key::Alpha('A'), okur_core::input::Modifiers::none(), true);
        a.native = 38;
        let mut shifted = a.clone();
        shifted.modifiers.shift = true;
        assert_eq!(
            strokes_again(&keymap, 0, &[shifted.clone()]),
            (vec![(50, true), (38, true), (38, false), (50, false)], 0)
        );
        assert_eq!(
            strokes_again(&keymap, keys::state::SHIFT, std::slice::from_ref(&a)),
            (vec![(38, true), (38, false)], 0)
        );
        assert_eq!(
            strokes_again(&keymap, keys::state::SHIFT, &[shifted]),
            (vec![(38, true), (38, false)], 0)
        );
        let f1 = KeyEvent::new(Key::F(1), okur_core::input::Modifiers::none(), true);
        assert_eq!(
            strokes_again(&keymap, 0, &[f1, a]),
            (vec![(38, true), (38, false)], 1)
        );
    }

    /// The keys Okur presses are found by the keycodes the hook reads them from: the gray Left,
    /// not the keypad's 4, which types a digit with Num Lock on; a key the keyboard lacks has
    /// none.
    #[test]
    fn keys_okur_presses_have_the_keycodes_the_hook_reads() {
        let keymap = keymap();
        assert_eq!(keymap.code_of(Key::LeftArrow), Some(113));
        assert_eq!(keymap.code_of(Key::Alpha('A')), Some(38));
        assert_eq!(keymap.code_of(Key::Shift), Some(50));
        assert_eq!(keymap.code_of(Key::F(1)), None);
    }
}
