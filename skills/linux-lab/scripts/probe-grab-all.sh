#!/usr/bin/env bash
# The X11 keyboard design's probe (docs/plans/2026-11-05-linux-first.md, "The keyboard on X11"):
# every key but the modifiers grabbed synchronously through XInput2 (XIPassiveGrabDevice), under
# 64 modifier combinations (Shift, Control, Alt, Super, and Caps Lock and Num Lock on or off), on
# the focused application window (not the root: there the newest grab wins, and Okur would
# take a window manager's shortcuts). The hook keeps h (a quick key), holds w unanswered until a watchdog replays it
# 200 ms later, and replays every other key at once. Another client grabs Super+E before the hook
# and one Super+R after it, as a window manager grabs its shortcuts. Keys sent by XTest: a, h, w,
# Shift+c, Down, Control+x, Super+E, Super+R, b. Passes when the application got a w C Down x b
# in order (h never), the hook got all but the two shortcuts, and each shortcut went to the
# client that grabbed it. It reports the cost per replayed key, and how late the watchdog let w
# go. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-grab-all.sh
set -uo pipefail
out=$(mktemp -d)

cat > "$out/target.py" <<'PY'
import time
from Xlib import X, XK, display
NAMES = {getattr(XK, n): n[3:] for n in dir(XK) if n.startswith("XK_")}
d = display.Display()
screen = d.screen()
window = screen.root.create_window(20, 20, 400, 200, 0, screen.root_depth, X.InputOutput,
    X.CopyFromParent, event_mask=X.KeyPressMask)
window.set_wm_name("okur-probe-target")
window.map()
d.sync()
time.sleep(1)
window.set_input_focus(X.RevertToParent, X.CurrentTime)
d.sync()
modifier_codes = {code for codes in d.get_modifier_mapping() for code in codes if code}
print("ready", flush=True)
while True:
    event = d.next_event()
    if event.type == X.KeyPress and event.detail not in modifier_codes:
        index = 1 if event.state & X.ShiftMask else 0
        print(NAMES.get(d.keycode_to_keysym(event.detail, index), event.detail), time.monotonic() * 1000, flush=True)
PY

cat > "$out/hook.py" <<'PY'
import itertools
import select
import time
from Xlib import X, XK, display
from Xlib.ext import xinput
from Xlib.protocol import rq

NAMES = {getattr(XK, n): n[3:] for n in dir(XK) if n.startswith("XK_")}

class XIAllowEvents(rq.Request):
    # XInput 2.0's form of the request python-xlib lacks: thaw a synchronous grab, keeping the
    # event (AsyncDevice) or replaying it to the window under it (ReplayDevice).
    _request = rq.Struct(
        rq.Card8("opcode"),
        rq.Opcode(53),
        rq.RequestLength(),
        rq.Card32("time"),
        rq.Card16("deviceid"),
        rq.Card8("event_mode"),
        rq.Pad(1),
    )

d = display.Display()
major = d.query_extension("XInputExtension").major_opcode
root = d.screen().root

def allow(deviceid, at, mode):
    XIAllowEvents(display=d.display, opcode=major, time=at, deviceid=deviceid, event_mode=mode)
    d.flush()

# The grabs go on the focused application window, not the root: X looks for a passive grab from
# the root down, so a shortcut another client grabbed on the root (a window manager's) is found
# first, whenever it was grabbed; on the root, the newest grab wins and Okur's would take it.
focused = d.get_input_focus().focus
while not hasattr(focused, "id") or focused.id == root.id:
    time.sleep(0.1)
    focused = d.get_input_focus().focus
keyboard = next(dev.deviceid for dev in d.xinput_query_device(xinput.AllDevices).devices
                if dev.use == xinput.MasterKeyboard)
modifier_codes = {code for codes in d.get_modifier_mapping() for code in codes if code}
base = [X.ShiftMask, X.ControlMask, X.Mod1Mask, X.Mod4Mask]
locks = [0, X.LockMask, X.Mod2Mask, X.LockMask | X.Mod2Mask]
masks = sorted({sum(c) | lock for n in range(len(base) + 1) for c in itertools.combinations(base, n) for lock in locks})
taken = []
grabbed = 0
for code in range(8, 256):
    if code in modifier_codes:
        continue
    reply = focused.xinput_grab_keycode(keyboard, X.CurrentTime, code, xinput.GrabModeSync,
                                     xinput.GrabModeAsync, False,
                                     xinput.KeyPressMask | xinput.KeyReleaseMask, masks)
    failed = list(reply.modifiers or [])
    grabbed += 1
    for mods in failed:
        taken.append((NAMES.get(d.keycode_to_keysym(code, 0), code), mods))
d.sync()
print(f"grabbed {grabbed} keycodes x {len(masks)} modifier sets; taken by another client: {taken}", flush=True)

held = None  # (deviceid, time, since) of w, kept frozen until the watchdog lets it go
while True:
    timeout = None
    if held:
        timeout = max(0.0, 0.2 - (time.monotonic() - held[2]))
    if not d.pending_events():
        readable, _, _ = select.select([d.fileno()], [], [], timeout)
        if not readable and held:
            allow(held[0], held[1], xinput.ReplayDevice)
            print("watchdog w", round((time.monotonic() - held[2]) * 1000, 1), time.monotonic() * 1000, flush=True)
            held = None
            continue
    event = d.next_event()
    if getattr(event, "evtype", None) != xinput.KeyPress:
        continue
    data = event.data
    name = NAMES.get(d.keycode_to_keysym(data.detail, 0), data.detail)
    if name == "h":
        allow(data.deviceid, data.time, xinput.AsyncDevice)
    elif name == "w":
        held = (data.deviceid, data.time, time.monotonic())
    else:
        allow(data.deviceid, data.time, xinput.ReplayDevice)
    print(name, time.monotonic() * 1000, flush=True)
PY

# Another client's shortcut, Super and the key given, grabbed as a window manager grabs its
# own (fluxbox here holds Alt+Tab already: a client's grab of it gets BadAccess).
cat > "$out/other.py" <<'PY'
import sys
from Xlib import X, XK, display
d = display.Display()
root = d.screen().root
code = d.keysym_to_keycode(XK.string_to_keysym(sys.argv[1]))
for lock in (0, X.LockMask, X.Mod2Mask, X.LockMask | X.Mod2Mask):
    root.grab_key(code, X.Mod4Mask | lock, False, X.GrabModeAsync, X.GrabModeAsync)
d.sync()
print("grabbed Super+" + sys.argv[1], flush=True)
while True:
    event = d.next_event()
    if event.type == X.KeyPress:
        print("Super+" + sys.argv[1] if event.detail == code else event.detail, flush=True)
PY

# Super+E grabbed before the hook, Super+R after it.
python3 "$out/other.py" e > "$out/before" 2>&1 &
before=$!
sleep 1
python3 "$out/target.py" > "$out/target" 2>&1 &
target=$!
# The hook grabs on the window with the focus: the target's, once it has taken it.
for _ in $(seq 40); do grep -q '^ready' "$out/target" 2>/dev/null && break; sleep 0.25; done
python3 "$out/hook.py" > "$out/hook" 2>&1 &
hook=$!
for _ in $(seq 40); do grep -q '^grabbed' "$out/hook" 2>/dev/null && break; sleep 0.25; done
python3 "$out/other.py" r > "$out/after" 2>&1 &
after=$!
sleep 2
for key in a h w shift+c Down ctrl+x super+e super+r b; do xdotool key "$key"; sleep 0.4; done
sleep 1
kill "$target" "$hook" "$before" "$after" 2>/dev/null
wait 2>/dev/null

echo "--- hook"; cat "$out/hook"
echo "--- target"; cat "$out/target"
echo "--- grabbed before the hook"; cat "$out/before"
echo "--- grabbed after the hook"; cat "$out/after"
target_keys=$(grep -v '^ready' "$out/target" | awk '{ print $1 }' | paste -sd' ')
hook_keys=$(grep -v '^grabbed\|^watchdog' "$out/hook" | awk '{ print $1 }' | paste -sd' ')
before_keys=$(grep -v '^grabbed' "$out/before" | paste -sd' ')
after_keys=$(grep -v '^grabbed' "$out/after" | paste -sd' ')
echo "hook:   $hook_keys"
echo "target: $target_keys"
echo "Super+E, grabbed before the hook, went to: the other client [$before_keys], the hook [$(echo "$hook_keys" | grep -o '\be\b' | head -1)]"
echo "Super+R, grabbed after the hook, went to: the other client [$after_keys], the hook [$(echo "$hook_keys" | grep -o '\br\b' | head -1)]"
# Replayed keys: the hook's decision to the application's receipt, matched in order.
paste <(grep -v '^grabbed\|^watchdog\|^h \|^w \|^e \|^r ' "$out/hook" | awk '{ print $2 }') \
      <(grep -v '^ready\|^w \|^e \|^r ' "$out/target" | awk '{ print $2 }') \
    | awk '{ printf "%.2f\n", $2 - $1 }' | sort -n \
    | awk '{ v[NR] = $1 } END { if (NR) print "replayed keys, ms from the decision to the application: median " v[int((NR + 1) / 2)] ", max " v[NR] }'
grep '^grabbed' "$out/hook" | grep -q 'keycodes' || echo "the grabs failed"
rm -rf "$out"
if [ "$target_keys" = "a w C Down x b" ] && [ "$hook_keys" = "a h w c Down x b" ] \
    && [ "$before_keys" = "Super+e" ] && [ "$after_keys" = "Super+r" ]; then
    echo "GRAB ALL PASS: every key through the hook, h kept, w let go by the watchdog, the rest replayed in order; the shortcuts other clients grabbed on the root, before or after, stay theirs"
else
    echo "GRAB ALL FAIL"
    exit 1
fi
