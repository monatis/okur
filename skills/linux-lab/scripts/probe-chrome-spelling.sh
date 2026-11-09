#!/usr/bin/env bash
# Whether Chrome in the lab marks misspelled words, as Okur reads them (AT-SPI's `invalid`
# "spelling" attribute runs, crates/okur-linux/src/actions.rs `misspelled`): JAWS says
# "Misspelled" in 567 and 284, and the lab's runs of them carried no mark. A fresh profile, as
# each run has, on the sign-up form: Tab into Full name, "teh quick brown fxo" typed, then the
# profile's spelling dictionaries listed, and the field's attribute runs read twice, 3 s and
# 15 s after the typing (a dictionary downloaded late would mark the words at the second).
# Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-chrome-spelling.sh
set -uo pipefail
out=$(mktemp -d)
profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/signup-form.html" \
    > "$out/chrome.log" 2>&1 &
sleep 15
xdotool key Tab
sleep 1
xdotool type --delay 30 "teh quick brown fxo"

read_runs() {
    python3 - <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def focused(node, depth=0):
    if node is None or depth > 40:
        return None
    try:
        if node.get_state_set().contains(Atspi.StateType.FOCUSED) and node.get_text_iface():
            return node
        for i in range(node.get_child_count()):
            found = focused(node.get_child_at_index(i), depth + 1)
            if found is not None:
                return found
    except Exception:
        return None
    return None

desktop = Atspi.get_desktop(0)
field = None
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app is not None and "chrom" in (app.get_name() or "").lower():
        field = focused(app)
        if field is not None:
            break
if field is None:
    print("no focused text field found")
else:
    text = field.get_text_iface()
    length = Atspi.Text.get_character_count(text)
    print("field:", field.get_role_name(), repr(field.get_name()), "text:",
          repr(Atspi.Text.get_text(text, 0, length)))
    offset = 0
    while offset < length:
        attributes, start, end = Atspi.Text.get_attribute_run(text, offset, False)
        print("run", start, end, dict(attributes))
        offset = max(end, offset + 1)
PY
}
sleep 3
echo "--- 3 s after the typing"; read_runs
sleep 12
echo "--- 15 s after the typing"; read_runs
echo "--- the profile's spelling dictionaries"
ls -la "$profile/Dictionaries" 2>&1
echo "--- the profile's spelling settings"
grep -o '"spellcheck":{[^}]*}' "$profile/Default/Preferences" 2>&1 | head -3
echo "--- Chrome's log, spelling lines"
grep -i "spell\|hunspell\|dictionar" "$out/chrome.log" | head -10
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile" "$out"
