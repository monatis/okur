#!/usr/bin/env bash
# Whether Chrome on Linux takes AT-SPI's Selection.SelectChild on its app menu as a highlight, as
# Windows' Chrome highlights the first item when the menu opens from the keyboard. Chrome on Linux
# opens it with no item highlighted (full5's 220), so Okur's user hears nothing at Alt+F and the
# arrows land one item behind Windows'. The probe: Alt+F, SelectChild(0) on the menu, the items'
# selected states, then Down, the states again, then Escape; every selection and focus event
# heard meanwhile. Research, not a test: it prints, and says whether the first item was
# highlighted and Down went on to the second. Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-chrome-menu-select.sh
set -uo pipefail
fixture=${1:-news-article.html}
out=$(mktemp -d)
profile=$(mktemp -d)

cat > "$out/probe.py" <<'PY'
import sys, time, threading
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi, GLib
from Xlib import X, XK, display
from Xlib.ext import xtest

start = time.time()
def log(*parts):
    print(f"{time.time() - start:7.3f}", *parts, flush=True)

def heard(event):
    try:
        source = event.source
        if source.get_application() is None:
            return
        if not source.get_application().get_name().startswith(("Google Chrome", "Chrom")):
            return
        role = source.get_role_name()
        if role in ("menu item", "menu", "frame") or event.type.startswith("focus"):
            log("EVENT", event.type, event.detail1, role, repr(source.get_name()))
    except Exception as error:
        log("EVENT ?", error)

listener = Atspi.EventListener.new(heard)
for kind in ("object:state-changed:selected", "object:selection-changed", "focus:",
             "object:state-changed:focused", "window:activate", "window:deactivate"):
    listener.register(kind)

d = display.Display()
def press(*names):
    codes = [d.keysym_to_keycode(XK.string_to_keysym(n)) for n in names]
    for c in codes:
        xtest.fake_input(d, X.KeyPress, c); d.sync()
    for c in reversed(codes):
        xtest.fake_input(d, X.KeyRelease, c); d.sync()

def chrome():
    desktop = Atspi.get_desktop(0)
    for i in range(desktop.get_child_count()):
        app = desktop.get_child_at_index(i)
        if app and app.get_name().startswith(("Google Chrome", "Chrom")):
            return app
    return None

def find_menu(node, depth=0):
    if node is None or depth > 8:
        return None
    try:
        if node.get_role_name() == "menu" and node.get_child_count() > 3:
            return node
        for i in range(node.get_child_count()):
            found = find_menu(node.get_child_at_index(i), depth + 1)
            if found:
                return found
    except Exception:
        return None
    return None

def selected_items(menu):
    names = []
    for i in range(menu.get_child_count()):
        item = menu.get_child_at_index(i)
        if item and item.get_state_set().contains(Atspi.StateType.SELECTED):
            names.append(item.get_name())
    return names

def steps():
    time.sleep(2)
    log("KEY Alt+F"); press("Alt_L", "f"); time.sleep(1.5)
    app = chrome()
    menu = None
    if app:
        for i in range(app.get_child_count()):
            menu = find_menu(app.get_child_at_index(i))
            if menu:
                break
    if not menu:
        log("NO MENU FOUND")
        GLib.idle_add(Atspi.event_quit); return
    log("MENU", menu.get_child_count(), "items; selected:", selected_items(menu))
    selection = menu.get_selection_iface()
    if selection is None:
        log("NO SELECTION INTERFACE")
    else:
        try:
            log("SelectChild(0) ->", selection.select_child(0))
        except Exception as error:
            log("SelectChild failed:", error)
    time.sleep(1)
    first = selected_items(menu)
    log("AFTER SELECT selected:", first)
    log("KEY Down"); press("Down"); time.sleep(1)
    second = selected_items(menu)
    log("AFTER DOWN selected:", second)
    log("KEY Escape"); press("Escape"); time.sleep(1)
    ok = bool(first) and bool(second) and first != second
    log("CHROME MENU SELECT", "WORKS" if ok else "DOES NOT WORK")
    GLib.idle_add(Atspi.event_quit)

threading.Thread(target=steps, daemon=True).start()
Atspi.event_main()
PY

google-chrome --no-sandbox --no-first-run --disable-gpu --user-data-dir="$profile" \
    --force-renderer-accessibility "file://$HOME/okur/corpus/fixtures/$fixture" > /tmp/chrome.log 2>&1 &
chrome=$!
sleep 8
timeout 60 python3 "$out/probe.py"
kill "$chrome" 2>/dev/null
wait "$chrome" 2>/dev/null
rm -rf "$out" "$profile"
