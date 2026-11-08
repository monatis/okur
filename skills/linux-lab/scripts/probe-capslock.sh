#!/usr/bin/env bash
# CapsLock as the Okur key on X11 (the code review of 2026-11-08, finding 14): with Okur running
# in the laptop layout, one press of CapsLock leaves Caps Lock off, two quick presses turn it on,
# two more turn it off, and once Okur stops the modifier map has Lock's keys again. Run in the
# lab, inside the accessibility stack:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-capslock.sh
# Prints CAPSLOCK PASS or CAPSLOCK FAIL as its last line.
set -uo pipefail
cd ~/okur || exit 1
cargo build -q -p okur --bin okur || exit 1
caps() { xset q | grep -o 'Caps Lock: *[a-z]*' | awk '{print $3}'; }
lock_keys() { xmodmap -pm | awk '$1 == "lock" { $1 = ""; print }' | xargs; }
status=0
expect() {
    if [ "$2" = "$3" ]; then
        echo "$1: $3"
    else
        echo "$1: $3, expected $2"
        status=1
    fi
}
out=$(mktemp -d)
before_keys=$(lock_keys)
echo "lock keys before Okur: $before_keys"
expect "Caps Lock before Okur" off "$(caps)"
RUST_LOG=okur_linux=debug ./target/debug/okur run --engine no-speech --keyboard-layout laptop \
    --settings "$out/settings" --speech-log "$out/speech.jsonl" > "$out/okur" 2>&1 &
okur=$!
sleep 4
expect "lock keys while Okur runs" "" "$(lock_keys)"
xdotool key Caps_Lock
sleep 1
expect "one press" off "$(caps)"
xdotool key Caps_Lock && xdotool key Caps_Lock
sleep 1
expect "two quick presses" on "$(caps)"
sleep 1
xdotool key Caps_Lock && xdotool key Caps_Lock
sleep 1
expect "two more" off "$(caps)"
kill -TERM "$okur"
wait "$okur"
sleep 1
expect "lock keys after Okur" "$before_keys" "$(lock_keys)"
echo "Okur said:"
grep -o '"text":"[^"]*"' "$out/speech.jsonl" | tail -5 | sed 's/^/  /'
if [ "$status" -ne 0 ]; then
    tail -20 "$out/okur" | sed 's/^/  okur: /'
    echo "CAPSLOCK FAIL"
else
    echo "CAPSLOCK PASS"
fi
exit $status
