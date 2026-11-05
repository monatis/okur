#!/usr/bin/env bash
# Okur's speech on Linux (crates/okur-linux/src/speech.rs), live: its example `ssip_speak` gives
# speech-dispatcher three marked parts, as Say All does, with its audio into a null sink. Passes
# when the marks are reached in order (1, 2, 3), speaking has ended after them, and the sink heard
# sound. Run in the lab, on a session bus of its own:
#   dbus-run-session -- bash /mnt/d/okur-linux/probe-ssip.sh
set -uo pipefail
cd ~/okur
cargo build -q -p okur-linux --example ssip_speak || exit 1
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/okur-runtime}"
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
out=$(mktemp -d)
pulseaudio --daemonize=yes --exit-idle-time=-1 -n --load="module-null-sink sink_name=okur_null" \
    --load=module-native-protocol-unix 2> "$out/pulse.log"
sleep 2
pactl set-default-sink okur_null
parec --device=okur_null.monitor --format=s16le --rate=16000 --channels=1 --raw > "$out/monitor.raw" &
recorder=$!
./target/debug/examples/ssip_speak > "$out/speak" 2>&1
sleep 1
kill "$recorder" 2>/dev/null
peak=$(python3 -c "
import array, sys
samples = array.array('h', open(sys.argv[1], 'rb').read())
print(max((abs(s) for s in samples), default=0))" "$out/monitor.raw")
cat "$out/speak"
echo "monitor peak $peak"
marks=$(grep '^reached' "$out/speak" | awk '{ print $2 }' | paste -sd' ')
ended=$(grep -c 'speaking after the last: false' "$out/speak")
pulseaudio --kill 2>/dev/null
pkill -f speech-dispatcher 2>/dev/null
rm -rf "$out"
if [ "$marks" = "1 2 3" ] && [ "$ended" = 1 ] && [ "$peak" -gt 1000 ]; then
    echo "SSIP PASS: three marked parts reached in order through speech-dispatcher, with sound"
else
    echo "SSIP FAIL (marks: $marks)"
    exit 1
fi
