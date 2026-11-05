#!/usr/bin/env bash
# Probe 3 of docs/plans/2026-11-05-linux-first.md: a screen reader's keyboard on X11, inside the
# headless desktop. Three X clients:
#   - a target window with the input focus, which logs the keys it is given (the application);
#   - the hook: a passive grab of Insert on the root window (Okur's key), so Insert and every key
#     pressed while it is held come to the hook instead of the application (the chord);
#   - a watcher of XInput2 raw key events on the root, which sees every key whatever holds a grab.
# Keys are sent by XTest (xdotool): a, Down, Insert held with Down, b. Passes when the watcher saw
# all five presses, the hook got Insert and Down, and the target got a, Down and b and nothing
# else. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-keys.sh
set -uo pipefail
out=$(mktemp -d)

cat > "$out/target.py" <<'PY'
import time
from Xlib import X, XK, display
NAMES = {getattr(XK, n): n[3:] for n in dir(XK) if n.startswith("XK_")}
d = display.Display()
screen = d.screen()
window = screen.root.create_window(20, 20, 400, 200, 0, screen.root_depth, X.InputOutput,
    X.CopyFromParent, event_mask=X.KeyPressMask | X.ExposureMask)
window.set_wm_name("okur-probe-target")
window.map()
d.sync()
time.sleep(1)
window.set_input_focus(X.RevertToParent, X.CurrentTime)
d.sync()
print("ready", flush=True)
while True:
    event = d.next_event()
    if event.type == X.KeyPress:
        print(NAMES.get(d.keycode_to_keysym(event.detail, 0), event.detail), flush=True)
PY

cat > "$out/hook.py" <<'PY'
from Xlib import X, XK, display
NAMES = {getattr(XK, n): n[3:] for n in dir(XK) if n.startswith("XK_")}
d = display.Display()
root = d.screen().root
insert = d.keysym_to_keycode(XK.XK_Insert)
root.grab_key(insert, X.AnyModifier, False, X.GrabModeAsync, X.GrabModeAsync)
d.sync()
print("grabbed", insert, flush=True)
while True:
    event = d.next_event()
    if event.type == X.KeyPress:
        print(NAMES.get(d.keycode_to_keysym(event.detail, 0), event.detail), flush=True)
PY

python3 "$out/target.py" > "$out/target" 2>&1 &
target=$!
python3 "$out/hook.py" > "$out/hook" 2>&1 &
hook=$!
xinput test-xi2 --root > "$out/raw" 2>&1 &
raw=$!
sleep 3
xdotool key a
xdotool key Down
xdotool keydown Insert
xdotool key Down
xdotool keyup Insert
xdotool key b
sleep 1
kill "$target" "$hook" "$raw" 2>/dev/null
wait 2>/dev/null

# The raw watcher's key codes, as key names.
raw_keys=$(awk '/\(RawKeyPress\)/ { press = 1; next } press && /detail:/ { print $2; press = 0 }' "$out/raw" \
    | while read -r code; do xmodmap -pke | awk -v c="$code" '$2 == c { print $4; exit }'; done | paste -sd' ')
target_keys=$(grep -v '^ready' "$out/target" | paste -sd' ')
hook_keys=$(grep -v '^grabbed' "$out/hook" | paste -sd' ')
echo "watcher (raw): $raw_keys"
echo "hook:          $hook_keys"
echo "target:        $target_keys"
echo "--- target's log"; cat "$out/target"
echo "--- hook's log"; cat "$out/hook"
rm -rf "$out"
if [ "$raw_keys" = "a Down Insert Down b" ] && [ "$hook_keys" = "Insert Down" ] \
    && [ "$target_keys" = "a Down b" ]; then
    echo "PROBE 3 PASS: every key seen, Insert+Down kept from the application"
else
    echo "PROBE 3 FAIL"
    exit 1
fi
