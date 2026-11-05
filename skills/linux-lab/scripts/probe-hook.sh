#!/usr/bin/env bash
# Okur's own X11 keyboard hook (crates/okur-linux/src/keyboard.rs), live: its example
# `x11_hook` keeps H, answers W only after half a second, and passes every other key, beside a
# window that logs the keys it is given, a client that grabbed Super+E before the hook and one
# that grabs Super+R after it. Keys sent by XTest: a, h, w, Shift+c, Down, Control+x, Super+E,
# Super+R, b. Passes when the window got a w C Down x b in order (H kept, W let go by the
# watchdog before the hook's late answer), the hook was asked about A H W C Down X B, and each
# shortcut reached the client that grabbed it. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-hook.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example x11_hook || exit 1
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
        print(NAMES.get(d.keycode_to_keysym(event.detail, index), event.detail), flush=True)
PY

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

python3 "$out/other.py" e > "$out/before" 2>&1 &
before=$!
python3 "$out/target.py" > "$out/target" 2>&1 &
target=$!
for _ in $(seq 40); do grep -q '^ready' "$out/target" 2>/dev/null && break; sleep 0.25; done
./target/debug/examples/x11_hook 15 > "$out/hook" 2>&1 &
hook=$!
for _ in $(seq 40); do grep -q '^hook started' "$out/hook" 2>/dev/null && break; sleep 0.25; done
python3 "$out/other.py" r > "$out/after" 2>&1 &
after=$!
sleep 2
for key in a h w shift+c Down ctrl+x super+e super+r b; do xdotool key "$key"; sleep 0.8; done
sleep 1
kill "$target" "$before" "$after" 2>/dev/null
wait "$hook" 2>/dev/null
wait 2>/dev/null

echo "--- hook"; cat "$out/hook"
echo "--- target"; cat "$out/target"
echo "--- grabbed before the hook"; cat "$out/before"
echo "--- grabbed after the hook"; cat "$out/after"
target_keys=$(grep -v '^ready' "$out/target" | paste -sd' ')
asked=$(grep '^down ' "$out/hook" | awk '{ print $2 }' | grep -v '^Shift$\|^Control$\|^Windows$\|^Alt$' | paste -sd' ')
before_keys=$(grep -v '^grabbed' "$out/before" | paste -sd' ')
after_keys=$(grep -v '^grabbed' "$out/after" | paste -sd' ')
echo "target: $target_keys"
echo "asked:  $asked"
rm -rf "$out"
if [ "$target_keys" = "a w C Down x b" ] \
    && [ "$asked" = "Alpha('A') Alpha('H') Alpha('W') Alpha('C') DownArrow Alpha('X') Alpha('B')" ] \
    && [ "$before_keys" = "Super+e" ] && [ "$after_keys" = "Super+r" ]; then
    echo "HOOK PASS: Okur's X11 hook asked about every key, kept H, the watchdog let W go, the rest reached the window in order, and the shortcuts stayed their clients'"
else
    echo "HOOK FAIL"
    exit 1
fi
