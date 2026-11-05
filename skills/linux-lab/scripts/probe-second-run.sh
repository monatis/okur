#!/usr/bin/env bash
# Why a second live run in one desktop session heard nothing (the lab's live runs, 2026-11-05:
# the first scenario of a job worked, the later ones got no AT-SPI event): Okur and Chrome twice
# in the same session, as run-scenario-linux.py runs them, with the registry's registered events
# after each Okur subscribed and the focus events each Okur logged. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-second-run.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example okur_linux || exit 1
a11y=$(busctl --user call org.a11y.Bus /org/a11y/bus org.a11y.Bus GetAddress | sed 's/^s "//; s/"$//')
registered() {
    busctl --address="$a11y" call org.a11y.atspi.Registry /org/a11y/atspi/registry \
        org.a11y.atspi.Registry GetRegisteredEvents 2>&1 | cut -c1-400
}
echo "registered before any Okur: $(registered)"
for run in 1 2; do
    out=$(mktemp -d)
    RUST_LOG=okur_linux=debug ./target/debug/examples/okur_linux --no-speech 30 \
        --speech-log "$out/speech.jsonl" > "$out/okur" 2>&1 &
    okur=$!
    sleep 4
    echo "run $run, registered after Okur subscribed: $(registered)"
    # Every signal on the accessibility bus while Chrome starts and loads its page.
    timeout 20 dbus-monitor --address "$a11y" "type='signal'" > "$out/signals" 2>&1 &
    profile=$(mktemp -d)
    google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
        --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/doc-reading.html" \
        > "$out/chrome.log" 2>&1 &
    chrome=$!
    sleep 15
    echo "run $run, registered with Chrome up: $(registered)"
    echo "run $run, focus events heard: $(grep -c 'AT-SPI focus' "$out/okur")"
    echo "run $run, utterances: $(grep -c '"speak"' "$out/speech.jsonl" 2>/dev/null)"
    echo "run $run, the bus's signals by member (interface; member):"
    grep -o 'interface=[^;]*; member=[A-Za-z]*' "$out/signals" | sort | uniq -c | sort -rn | head -12
    echo "run $run, focus-related details:"
    grep -E -A1 'member=(StateChanged|Focus|LoadComplete|Activate)' "$out/signals" \
        | grep -o 'string "[a-z:-]*"' | sort | uniq -c | sort -rn | head -8
    kill "$chrome" 2>/dev/null
    pkill -f "user-data-dir=$profile" 2>/dev/null
    kill "$okur" 2>/dev/null
    wait "$okur" 2>/dev/null
    sleep 1
    echo "run $run, registered after both ended: $(registered)"
done
