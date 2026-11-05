#!/usr/bin/env bash
# Research for Okur's Linux virtual buffer (docs/plans/2026-11-05-linux-first.md §2.3): Chrome's
# AT-SPI tree of a fixture page, as libatspi reads it, each element with its role, name, states
# that matter to reading, its object attributes (a heading's level, the tag, a link's place), and
# its text with U+FFFC where a child sits (the hypertext model IAccessible2 also has). The page's
# document subtree is printed indented. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-atspi-tree.sh [fixture.html]
set -uo pipefail
fixture="${1:-doc-reading.html}"
out=$(mktemp -d)
profile=$(mktemp -d)
google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/$fixture" > "$out/chrome.log" 2>&1 &
sleep 15
python3 - <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def find_document(node, depth=0):
    if node is None or depth > 40:
        return None
    try:
        if node.get_role_name() == "document web":
            return node
        for i in range(node.get_child_count()):
            found = find_document(node.get_child_at_index(i), depth + 1)
            if found:
                return found
    except Exception:
        return None
    return None

def text_of(node):
    try:
        text = node.get_text_iface()
        if text is None:
            return None
        return Atspi.Text.get_text(text, 0, Atspi.Text.get_character_count(text))
    except Exception:
        return None

def dump(node, depth=0):
    if depth > 30:
        return
    try:
        role = node.get_role_name()
        name = node.get_name()
        attrs = node.get_attributes() or {}
        states = node.get_state_set()
        marks = [s for s in ("focusable", "focused", "visited", "checked", "expanded", "required")
                 if states.contains(getattr(Atspi.StateType, s.upper()))]
        keep = {k: v for k, v in attrs.items() if k in ("level", "tag", "xml-roles", "display", "id", "href", "src")}
        text = text_of(node)
        line = "  " * depth + f"{role} name={name!r}"
        if keep:
            line += f" attrs={keep}"
        if marks:
            line += f" states={marks}"
        if text is not None:
            line += f" text={text!r}"
        print(line)
        for i in range(node.get_child_count()):
            dump(node.get_child_at_index(i), depth + 1)
    except Exception as error:
        print("  " * depth + f"(error: {error})")

desktop = Atspi.get_desktop(0)
document = None
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app and "chrome" in (app.get_name() or "").lower():
        document = find_document(app)
if document is None:
    print("no document")
else:
    dump(document)
PY
pkill -f "user-data-dir=$profile" 2>/dev/null
rm -rf "$profile" "$out"
