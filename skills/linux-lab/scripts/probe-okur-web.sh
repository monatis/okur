#!/usr/bin/env bash
# Okur on Linux reading a page (crates/okur-linux/examples/okur_linux.rs, the plan's §2.3): Chrome
# on doc-reading.html with Okur running (the silent synthesizer, its speech log kept), then Down
# twice and H in the virtual cursor. Passes when Okur said the page's heading and a paragraph of
# it: the page reached the engine as its AOM, and the keys moved through it. The speech log is
# printed. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-okur-web.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example okur_linux || exit 1
out=$(mktemp -d)
RUST_LOG=okur_linux=debug ./target/debug/examples/okur_linux --no-speech 45 \
    --speech-log "$out/speech.jsonl" > "$out/okur" 2>&1 &
okur=$!
sleep 4
profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/doc-reading.html" > "$out/chrome.log" 2>&1 &
sleep 20
for key in Down Down h; do xdotool key "$key"; sleep 2; done
wait "$okur"
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile"
grep -v '^  - \|Navigation shortcuts\|^Press\|^====\|Okur, an open\|TTS Driver' "$out/okur" \
    | sed 's/\x1b\[[0-9;]*m//g' | tail -15
echo "--- speech log"
python3 -c "
import json, sys
for line in open(sys.argv[1]):
    entry = json.loads(line)
    print(entry.get('kind'), '|', entry.get('text'))" "$out/speech.jsonl" 2>/dev/null | head -30
heading=$(grep -c 'The Accessibility Engine Specification' "$out/speech.jsonl" 2>/dev/null)
paragraph=$(grep -c 'virtual buffer\|Engine components\|Rust provides' "$out/speech.jsonl" 2>/dev/null)
rm -rf "$out"
echo "the heading said ${heading:-0} time(s), a paragraph ${paragraph:-0} time(s)"
if [ "${heading:-0}" -ge 1 ] && [ "${paragraph:-0}" -ge 1 ]; then
    echo "WEB PASS: Okur on Linux read the page's heading and a paragraph"
else
    echo "WEB FAIL"
    exit 1
fi
