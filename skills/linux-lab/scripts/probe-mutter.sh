#!/usr/bin/env bash
# Probe 5 of docs/plans/2026-11-05-linux-first.md: GNOME's compositor, Mutter, headless, and its
# KeyboardMonitor (the D-Bus interface org.freedesktop.a11y.KeyboardMonitor that gives a screen
# reader keys on Wayland). A client owning Okur's own name (org.okur.Okur.KeyboardMonitor) asks
# WatchKeyboard twice: Mutter as it ships, then with its debug switch
# MUTTER_DEBUG_A11Y_MANAGER_WITHOUT_ACCESS_CONTROL=1. Passes when the first is refused and the
# second allowed. Then keys are injected through Mutter's RemoteDesktop API, and the probe reports
# whether they arrive as KeyEvent (open in the proposal; reported, not judged). Run in the lab:
#   dbus-run-session -- bash /mnt/d/okur-linux/probe-mutter.sh
set -uo pipefail
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/okur-runtime}"
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
out=$(mktemp -d)
mutter --version

cat > "$out/watch.py" <<'PY'
import sys
import time
from gi.repository import Gio, GLib

bus = Gio.bus_get_sync(Gio.BusType.SESSION)
# Okur's own name, as libatspi asks under the application's id.
bus.call_sync("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "RequestName",
              GLib.Variant("(su)", ("org.okur.Okur.KeyboardMonitor", 0)), None, 0, -1, None)
keys = []

def key_event(connection, sender, path, interface, name, parameters, data):
    keys.append(parameters.unpack())

bus.signal_subscribe("org.freedesktop.a11y.Manager", "org.freedesktop.a11y.KeyboardMonitor", "KeyEvent",
                     "/org/freedesktop/a11y/Manager", None, 0, key_event, None)
try:
    bus.call_sync("org.freedesktop.a11y.Manager", "/org/freedesktop/a11y/Manager",
                  "org.freedesktop.a11y.KeyboardMonitor", "WatchKeyboard", None, None, 0, 5000, None)
    print("WatchKeyboard: allowed", flush=True)
except GLib.Error as error:
    print("WatchKeyboard: refused:", error.message, flush=True)
    sys.exit(0)

remote = "org.gnome.Mutter.RemoteDesktop"
try:
    session = bus.call_sync(remote, "/org/gnome/Mutter/RemoteDesktop", remote, "CreateSession", None,
                            GLib.VariantType("(o)"), 0, 5000, None).unpack()[0]
    bus.call_sync(remote, session, remote + ".Session", "Start", None, None, 0, 5000, None)
    for keysym in (0x61, 0xFF54):  # a, Down
        for pressed in (True, False):
            bus.call_sync(remote, session, remote + ".Session", "NotifyKeyboardKeysym",
                          GLib.Variant("(ub)", (keysym, pressed)), None, 0, 5000, None)
except GLib.Error as error:
    print("RemoteDesktop:", error.message, flush=True)
context = GLib.MainContext.default()
deadline = time.time() + 2
while time.time() < deadline:
    context.iteration(False)
    time.sleep(0.01)
print("KeyEvents heard:", len(keys), keys, flush=True)
PY

# One Mutter run: $1 = 1 sets the debug switch.
run() {
    local debug=$1
    MUTTER_DEBUG_A11Y_MANAGER_WITHOUT_ACCESS_CONTROL=$debug \
        mutter --headless --virtual-monitor 1280x720 > "$out/mutter-$debug.log" 2>&1 &
    local mutter=$!
    for _ in $(seq 30); do
        sleep 1
        gdbus call --session --dest org.freedesktop.DBus --object-path /org/freedesktop/DBus \
            --method org.freedesktop.DBus.NameHasOwner org.freedesktop.a11y.Manager 2>/dev/null \
            | grep -q true && break
    done
    python3 "$out/watch.py" 2>&1
    kill "$mutter" 2>/dev/null
    wait "$mutter" 2>/dev/null
    echo "--- mutter (debug switch $debug)"; tail -5 "$out/mutter-$debug.log"
}
shipped=$(run 0)
echo "$shipped"
debug=$(run 1)
echo "$debug"
rm -rf "$out"
if echo "$shipped" | grep -q "WatchKeyboard: refused" && echo "$debug" | grep -q "WatchKeyboard: allowed"; then
    echo "PROBE 5 PASS: Okur's own name refused, allowed with the debug switch"
else
    echo "PROBE 5 FAIL"
    exit 1
fi
