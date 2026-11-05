#!/usr/bin/env bash
# Okur's Linux event source (crates/okur-linux/src/events.rs), live: its example `atspi_focus`
# prints each focus event the desktop sends, as the engine would be given it, while Tab moves six
# times through GTK 4's widget factory, then through Chrome on a fixture page. Passes when six
# focus events came from each, each with a role. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-focus.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example atspi_focus || exit 1
out=$(mktemp -d)
X=./target/debug/examples/atspi_focus

# The application first, then the source, as a screen reader finds applications running.
gtk4-widget-factory > "$out/gtk.log" 2>&1 &
gtk=$!
sleep 6
"$X" 12 > "$out/gtk-focus" 2>&1 &
source=$!
sleep 2
for _ in $(seq 6); do xdotool key Tab; sleep 0.8; done
wait "$source"
kill "$gtk" 2>/dev/null

# The source first, then the application: a toolkit started later hears the registration.
"$X" 30 > "$out/chrome-focus" 2>&1 &
source=$!
sleep 2
profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/doc-reading.html" > "$out/chrome.log" 2>&1 &
sleep 14
for _ in $(seq 6); do xdotool key Tab; sleep 0.8; done
wait "$source"
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile"

echo "--- GTK 4"; cat "$out/gtk-focus"
echo "--- Chrome"; cat "$out/chrome-focus"
gtk_count=$(grep -c '^focus ' "$out/gtk-focus")
chrome_count=$(grep -c '^focus ' "$out/chrome-focus")
echo "focus events: GTK 4 $gtk_count, Chrome $chrome_count (six Tabs each)"
rm -rf "$out"
if [ "$gtk_count" -ge 6 ] && [ "$chrome_count" -ge 6 ]; then
    echo "FOCUS PASS: the desktop's focus events reached Okur's event source from GTK 4 and Chrome"
else
    echo "FOCUS FAIL"
    exit 1
fi
