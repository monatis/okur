#!/usr/bin/env bash
# Okur on Linux, whole (crates/okur-linux/examples/okur_linux.rs): the runtime's event loop with
# Linux's platform, the X11 keyboard hook, the desktop's AT-SPI events and the silent synthesizer,
# printing what it says, while Tab moves four times through GTK 4's widget factory. Passes when
# Okur ran, said something for the focus after each Tab, and the Tabs still reached the
# application (its focus moved: the hook passed them). Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-okur.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example okur_linux || exit 1
out=$(mktemp -d)

gtk4-widget-factory > "$out/gtk.log" 2>&1 &
gtk=$!
sleep 6
RUST_LOG=okur_core=debug,okur_linux=debug ./target/debug/examples/okur_linux --no-speech 20 > "$out/okur" 2>&1 &
okur=$!
sleep 6
for _ in $(seq 4); do xdotool key Tab; sleep 1; done
wait "$okur"
kill "$gtk" 2>/dev/null

cat "$out/okur"
spoken=$(grep -c '^\[Event Spoken\]' "$out/okur")
stopped=$(grep -c '^Okur stopped' "$out/okur")
echo "focus speech lines: $spoken (four Tabs)"
rm -rf "$out"
if [ "$stopped" = 1 ] && [ "$spoken" -ge 4 ]; then
    echo "OKUR PASS: Okur ran on Linux and spoke the focus after each Tab"
else
    echo "OKUR FAIL"
    exit 1
fi
