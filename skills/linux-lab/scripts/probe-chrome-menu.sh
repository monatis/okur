#!/usr/bin/env bash
# What Chrome sends over AT-SPI as its own menu opens and its items are moved through, with no
# screen reader taking the keys: Alt+F (the app menu), Down three times, Escape. Okur's runs
# heard a window activated and the items renamed, and no focus or selection of an item
# (220/s1); this lists every event a registered listener hears meanwhile, with the source's
# role, name and states, for the driver's menu reading to be built on. Research, not a test:
# it prints, and passes when Chrome's menu window was heard at all. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-chrome-menu.sh [fixture.html]
set -uo pipefail
fixture=${1:-news-article.html}
out=$(mktemp -d)
profile=$(mktemp -d)

cat > "$out/listener.py" <<'PY'
import time
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

start = time.monotonic()

def heard(event):
    try:
        source = event.source
        app = source.get_application().get_name() if source.get_application() else "?"
        if app != "Google Chrome" and not app.startswith("Chrome"):
            return
        states = [s.value_nick for s in source.get_state_set().get_states()]
        keep = [s for s in states if s in ("focused", "selected", "active", "checked", "expanded", "showing")]
        print(f"{time.monotonic() - start:7.3f}\t{event.type}\t{event.detail1}\t{source.get_role_name()}\t{source.get_name()!r}\t{','.join(keep)}", flush=True)
    except Exception as error:
        print(f"{time.monotonic() - start:7.3f}\t{event.type}\t?\t{error}", flush=True)

listener = Atspi.EventListener.new(heard)
for kind in ("focus:", "window:", "object:state-changed", "object:active-descendant-changed",
             "object:selection-changed", "object:children-changed", "object:property-change:accessible-name"):
    listener.register(kind)
Atspi.event_main()
PY

cat > "$out/keys.py" <<'PY'
import sys, time
from Xlib import X, XK, display
from Xlib.ext import xtest
d = display.Display()
def press(*names):
    codes = [d.keysym_to_keycode(XK.string_to_keysym(n)) for n in names]
    for c in codes:
        xtest.fake_input(d, X.KeyPress, c); d.sync()
    for c in reversed(codes):
        xtest.fake_input(d, X.KeyRelease, c); d.sync()
for step in sys.argv[1:]:
    print(f"KEY {step}", flush=True)
    press(*step.split("+"))
    time.sleep(1.5)
PY

google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/$fixture" > /tmp/chrome.log 2>&1 &
chrome=$!
sleep 8
python3 "$out/listener.py" > "$out/events" 2>&1 &
listener=$!
sleep 2
python3 "$out/keys.py" Alt_L+f Down Down Down Escape > "$out/keys" 2>&1
sleep 2
kill "$listener" "$chrome" 2>/dev/null
wait "$listener" "$chrome" 2>/dev/null
cat "$out/keys"
echo "--- events"
cat "$out/events"
if grep -q "window:" "$out/events"; then
    echo "CHROME MENU PROBE DONE"
else
    echo "CHROME MENU PROBE FAILED: no window event heard"
fi
rm -rf "$out" "$profile"
