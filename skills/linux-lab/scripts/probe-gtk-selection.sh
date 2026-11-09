#!/usr/bin/env bash
# Whether GTK 4 marks a column view's rows SELECTABLE and SELECTED by its selection model
# (okur-linux-model's focus.rs says "Not Selected" for a focused row that is SELECTABLE and not
# SELECTED, as Okur's own PlaceMarker List's rows are until one is picked [obs, 704/s11]). The
# code review of 2026-11-10 asked whether a list with no selection (GtkNoSelection) marks its
# rows SELECTABLE too, which would say "Not Selected" on every row, and how a multiple
# selection's rows read. Three column views, single, none and multiple, each with three rows;
# Tab into each in turn, Down once, and the focused row's states read after each key.
# Run in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-gtk-selection.sh
set -uo pipefail
out=$(mktemp -d)
cat > "$out/app.py" <<'PY'
import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk

def view(kind):
    model = Gtk.StringList.new(["%s one" % kind, "%s two" % kind, "%s three" % kind])
    selection = {
        "single": Gtk.SingleSelection,
        "none": Gtk.NoSelection,
        "multiple": Gtk.MultiSelection,
    }[kind](model=model)
    if kind == "single":
        selection.set_autoselect(False)
        selection.set_can_unselect(True)
        selection.unselect_all()
    factory = Gtk.SignalListItemFactory()
    factory.connect("setup", lambda f, item: item.set_child(Gtk.Label(xalign=0)))
    factory.connect("bind", lambda f, item: item.get_child().set_label(item.get_item().get_string()))
    column = Gtk.ColumnViewColumn(title=kind, factory=factory)
    column.set_expand(True)
    columns = Gtk.ColumnView(model=selection)
    columns.append_column(column)
    columns.update_property([Gtk.AccessibleProperty.LABEL], [kind])
    return columns

def activate(app):
    window = Gtk.ApplicationWindow(application=app, title="Selection Probe")
    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=8)
    for kind in ("single", "none", "multiple"):
        box.append(view(kind))
    window.set_child(box)
    window.present()

app = Gtk.Application(application_id="org.okur.SelectionProbe")
app.connect("activate", activate)
app.run(None)
PY
cat > "$out/read.py" <<'PY'
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def focused(node, depth=0):
    if node is None or depth > 30:
        return None
    try:
        if node.get_state_set().contains(Atspi.StateType.FOCUSED):
            return node
        for i in range(node.get_child_count()):
            found = focused(node.get_child_at_index(i), depth + 1)
            if found is not None:
                return found
    except Exception:
        return None
    return None

desktop = Atspi.get_desktop(0)
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app is None or "python" not in (app.get_name() or "").lower() and "selection" not in (app.get_name() or "").lower():
        continue
    node = focused(app)
    if node is None:
        continue
    states = node.get_state_set()
    marks = [s for s in ("SELECTABLE", "SELECTED", "FOCUSED") if states.contains(getattr(Atspi.StateType, s))]
    print(node.get_role_name(), repr(node.get_name()), marks)
PY
python3 "$out/app.py" > "$out/app.log" 2>&1 &
app=$!
sleep 6
xdotool search --sync --onlyvisible --name "Selection Probe" windowactivate --sync %1 2>/dev/null
for kind in single none multiple; do
    xdotool key Tab; sleep 0.8
    echo "--- $kind, Tab"; python3 "$out/read.py"
    xdotool key Down; sleep 0.8
    echo "--- $kind, Down"; python3 "$out/read.py"
done
kill "$app" 2>/dev/null
cat "$out/app.log" | head -5
rm -rf "$out"
