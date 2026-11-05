#!/usr/bin/env bash
# Function keys through Okur's X11 hook: x11_hook beside a window in front, F5, F6, F1, a and Down
# pressed by XTest by keycode, as run-scenario-linux.py presses them. Passes when the hook was
# told of each key's press (with xdotool, F5 came as Alt pressed and let go, then F5's release
# alone: its press went to the window manager's Alt grab; the lab's live run of 712). Run in the
# lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-fkeys.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example x11_hook || exit 1
out=$(mktemp -d)
profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    "file://$HOME/okur/corpus/fixtures/doc-reading.html" > /dev/null 2>&1 &
sleep 8
./target/debug/examples/x11_hook 14 > "$out/hook" 2> "$out/trace" &
hook=$!
# Grabbing every key on every window of Chrome's takes the hook some seconds.
sleep 6
python3 - <<'PY'
import time
from Xlib import X, XK, display
from Xlib.ext import xtest
d = display.Display()
for name in ["F5", "F6", "F1", "a", "Down"]:
    code = d.keysym_to_keycode(XK.string_to_keysym(name))
    xtest.fake_input(d, X.KeyPress, code)
    d.sync()
    xtest.fake_input(d, X.KeyRelease, code)
    d.sync()
    time.sleep(0.4)
PY
wait "$hook"
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile"
echo "--- the hook's X events: $(grep -c "X event" "$out/trace"), its last lines:"
grep -v "request failed" "$out/trace" | tail -5 | cut -c1-200
echo "--- the hook was told of"
cat "$out/hook"
missing=
for key in "F(5)" "F(6)" "F(1)" "Alpha('A')" "DownArrow"; do
    grep -qF "down $key" "$out/hook" || missing="$missing $key"
done
if [ -z "$missing" ]; then
    echo "FKEYS PASS"
else
    echo "FKEYS FAIL: no press of$missing"
    exit 1
fi
