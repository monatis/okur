#!/usr/bin/env bash
# Probe 2 of docs/plans/2026-11-05-linux-first.md: focus events while Tab moves through a GTK 4
# application and through Chrome, inside the headless desktop. Two listeners are compared:
#   - the vendored xa11y's `events` (match rules on the bus for one process, no RegisterEvent);
#   - a listener registered with the AT-SPI registry (RegisterEvent, through libatspi), as Orca is.
# Each application is run twice: first with xa11y alone (does a toolkit send its events when nobody
# has registered for them?), then with the registered listener beside it. Passes when the
# registered listener hears one focus event per Tab in both applications. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-events.sh
set -uo pipefail
cd ~/okur
cargo build -q -p xa11y --features cli --bin xa11y || exit 1
X=$PWD/target/debug/xa11y
TABS=6
out=$(mktemp -d)

cat > "$out/listener.py" <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def heard(event):
    if event.type.startswith("object:state-changed:focused") and event.detail1 != 1:
        return
    try:
        source = event.source
        print(f"{event.type}\t{source.get_application().get_name()}\t{source.get_role_name()}\t{source.get_name()}", flush=True)
    except Exception as error:
        print(f"{event.type}\t?\t?\t{error}", flush=True)

listener = Atspi.EventListener.new(heard)
listener.register("object:state-changed:focused")
listener.register("focus:")
Atspi.event_main()
PY

# Tab through the application whose process is $2; $3 = 1 adds the registered listener.
phase() {
    local label=$1 pid=$2 registered=$3
    "$X" events --pid "$pid" > "$out/$label.xa11y" 2>&1 &
    local events=$!
    local listener=
    if [ "$registered" = 1 ]; then
        python3 "$out/listener.py" > "$out/$label.registered" 2>&1 &
        listener=$!
    fi
    sleep 2
    for _ in $(seq "$TABS"); do "$X" key Tab > /dev/null; sleep 0.8; done
    sleep 1
    kill "$events" ${listener:+"$listener"} 2>/dev/null
    wait "$events" ${listener:+"$listener"} 2>/dev/null
    local heard_xa11y heard_registered=-
    heard_xa11y=$(grep -c '^\[focus_changed\]' "$out/$label.xa11y")
    if [ "$registered" = 1 ]; then
        heard_registered=$(grep -c '^object:state-changed:focused' "$out/$label.registered")
    fi
    echo "RESULT $label: $TABS Tabs; xa11y focus_changed: $heard_xa11y; registered focused: $heard_registered"
    echo "--- $label, xa11y"; head -20 "$out/$label.xa11y"
    if [ "$registered" = 1 ]; then echo "--- $label, registered"; head -20 "$out/$label.registered"; fi
    if [ "$registered" = 1 ] && [ "$heard_registered" -lt "$TABS" ]; then failed=1; fi
}

failed=0
gtk4-widget-factory > "$out/gtk.log" 2>&1 &
gtk=$!
for _ in $(seq 30); do sleep 1; "$X" apps | grep -q "^$gtk" && break; done
echo "--- xa11y apps"; "$X" apps
phase gtk-alone "$gtk" 0
phase gtk-registered "$gtk" 1
kill "$gtk" 2>/dev/null; wait "$gtk" 2>/dev/null

profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/doc-reading.html" > "$out/chrome.log" 2>&1 &
chrome=
for _ in $(seq 30); do
    sleep 2
    # Chrome's accessibility lives in its browser process, the one xa11y lists.
    chrome=$("$X" apps | awk -F'	' 'tolower($2) ~ /chrome/ { print $1; exit }')
    [ -n "$chrome" ] && break
done
sleep 3
echo "chrome's process for xa11y: ${chrome:-none}"
if [ -n "$chrome" ]; then
    phase chrome-alone "$chrome" 0
    phase chrome-registered "$chrome" 1
else
    failed=1
fi
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile" "$out"
if [ "$failed" = 0 ]; then
    echo "PROBE 2 PASS: the registered listener heard one focus event per Tab in GTK 4 and Chrome"
else
    echo "PROBE 2 FAIL"
    exit 1
fi
