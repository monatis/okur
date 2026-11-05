#!/usr/bin/env bash
# Week 2's keyboard probe (docs/handoff/2026-11-05-linux-lab-week-1.md, Next 1): on X11, can a
# screen reader decide key by key, as Windows' low-level hook lets it, by grabbing keys
# synchronously and then keeping each one or replaying it to the application? libatspi's X11
# device does this. The hook grabs h and Down synchronously; it keeps h (a quick key) and replays
# Down; a is not grabbed. Keys sent by XTest: h, Down, a, ten times. Passes when the target got
# Down and a every time and h never, in order; it reports how much later a replayed key reaches
# the application than a key no one grabbed (median over the ten). Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-sync-grab.sh
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
print("ready", flush=True)
while True:
    event = d.next_event()
    if event.type == X.KeyPress:
        print(NAMES.get(d.keycode_to_keysym(event.detail, 0), event.detail), time.monotonic() * 1000, flush=True)
PY

cat > "$out/hook.py" <<'PY'
import time
from Xlib import X, XK, display
NAMES = {getattr(XK, n): n[3:] for n in dir(XK) if n.startswith("XK_")}
d = display.Display()
root = d.screen().root
for keysym in (XK.XK_h, XK.XK_Down):
    # Pointer asynchronous, keyboard synchronous: the keyboard freezes until the hook decides.
    root.grab_key(d.keysym_to_keycode(keysym), X.AnyModifier, False, X.GrabModeAsync, X.GrabModeSync)
d.sync()
print("grabbed", flush=True)
while True:
    event = d.next_event()
    if event.type == X.KeyPress:
        name = NAMES.get(d.keycode_to_keysym(event.detail, 0), event.detail)
        if name == "h":
            d.allow_events(X.AsyncKeyboard, event.time)   # kept: the application never sees it
        else:
            d.allow_events(X.ReplayKeyboard, event.time)  # let go: as if no one had grabbed it
        d.flush()
        print(name, time.monotonic() * 1000, flush=True)
    elif event.type == X.KeyRelease:
        d.allow_events(X.AsyncKeyboard, event.time)
        d.flush()
PY

python3 "$out/target.py" > "$out/target" 2>&1 &
target=$!
python3 "$out/hook.py" > "$out/hook" 2>&1 &
hook=$!
sleep 3
for _ in $(seq 10); do
    xdotool key h; xdotool key Down; xdotool key a
    sleep 0.2
done
sleep 1
kill "$target" "$hook" 2>/dev/null
wait 2>/dev/null

target_keys=$(grep -v '^ready' "$out/target" | awk '{ print $1 }' | paste -sd' ')
hook_keys=$(grep -v '^grabbed' "$out/hook" | awk '{ print $1 }' | paste -sd' ')
expected=$(for _ in $(seq 10); do printf 'Down a '; done | sed 's/ $//')
echo "hook:   $hook_keys"
echo "target: $target_keys"
# Monotonic times (ms, one clock for both processes): from the hook's decision on each Down to
# the application's receiving it: what a replay adds after the decision.
paste <(grep '^Down' "$out/hook" | awk '{ print $2 }') <(grep '^Down' "$out/target" | awk '{ print $2 }') \
    | awk '{ printf "%.2f\n", $2 - $1 }' | sort -n \
    | awk '{ v[NR] = $1 } END { if (NR) print "replayed Down, ms from the decision to the application: median " v[int((NR + 1) / 2)] ", max " v[NR] }'
rm -rf "$out"
if [ "$target_keys" = "$expected" ] && [ "$(echo "$hook_keys" | wc -w)" -eq 20 ]; then
    echo "SYNC GRAB PASS: h kept, Down replayed, a untouched, in order, ten times"
else
    echo "SYNC GRAB FAIL"
    exit 1
fi
