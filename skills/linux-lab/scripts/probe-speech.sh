#!/usr/bin/env bash
# Probe 4 of docs/plans/2026-11-05-linux-first.md: speech-dispatcher with espeak-ng, its audio into
# a null sink (nothing reaches old-pc's speakers). A marked text (SSML) is spoken through SSIP's
# Python client; passes when the notifications come as begin, mark m1, mark m2, end, in order, and
# the sink's monitor heard sound. Run in the lab, on a session bus of its own:
#   dbus-run-session -- bash /mnt/d/okur-linux/probe-speech.sh
set -uo pipefail
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/tmp/okur-runtime}"
mkdir -p "$XDG_RUNTIME_DIR" && chmod 700 "$XDG_RUNTIME_DIR"
out=$(mktemp -d)

# A sound server with one sink that goes nowhere; as root it warns, and runs.
pulseaudio --daemonize=yes --exit-idle-time=-1 -n --load="module-null-sink sink_name=okur_null" \
    --load=module-native-protocol-unix 2> "$out/pulse.log"
sleep 2
pactl set-default-sink okur_null
pactl info | grep -E 'Server Name|Default Sink'
parec --device=okur_null.monitor --format=s16le --rate=16000 --channels=1 --raw > "$out/monitor.raw" &
recorder=$!

cat > "$out/speak.py" <<'PY'
import threading
import speechd

heard = []
finished = threading.Event()

def notified(kind, index_mark=None):
    heard.append(index_mark if kind == speechd.CallbackType.INDEX_MARK else kind)
    if kind in (speechd.CallbackType.END, speechd.CallbackType.CANCEL):
        finished.set()

client = speechd.SSIPClient("okur-probe")
client.set_output_module("espeak-ng")
client.set_data_mode(speechd.DataMode.SSML)
client.speak('<speak>The first words <mark name="m1"/>then the second part <mark name="m2"/>and the end.</speak>',
             callback=notified,
             event_types=(speechd.CallbackType.BEGIN, speechd.CallbackType.END,
                          speechd.CallbackType.INDEX_MARK, speechd.CallbackType.CANCEL))
finished.wait(30)
client.close()
print("notifications:", " ".join(str(item) for item in heard))
PY
notifications=$(python3 "$out/speak.py" 2>&1)
echo "$notifications"
sleep 1
kill "$recorder" 2>/dev/null
peak=$(python3 -c "
import array, sys
samples = array.array('h', open(sys.argv[1], 'rb').read())
print(max((abs(s) for s in samples), default=0))" "$out/monitor.raw")
echo "monitor: $(stat -c %s "$out/monitor.raw") bytes, peak $peak"
echo "--- pulseaudio"; tail -3 "$out/pulse.log"
pulseaudio --kill 2>/dev/null
pkill -f speech-dispatcher 2>/dev/null
rm -rf "$out"
if echo "$notifications" | grep -q "^notifications: begin m1 m2 end$" && [ "$peak" -gt 1000 ]; then
    echo "PROBE 4 PASS: begin, m1, m2 and end in order, and sound on the null sink"
else
    echo "PROBE 4 FAIL"
    exit 1
fi
