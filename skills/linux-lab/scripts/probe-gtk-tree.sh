#!/usr/bin/env bash
# Research for the Linux driver's desktop controls (docs/plans/2026-11-05-linux-first.md §2.4):
# a Linux replica of a Windows fixture (corpus/fixtures/linux/<name>.py, GTK 4) as libatspi reads
# it, each element indented under its parent with its role, name, states that matter (focused,
# checked, selected, expanded, sensitive), its relations (labelled-by) and its index in its
# parent; then the focus as Tab moves through it, each focused element with its ancestors. Run
# in the lab:
#   bash /mnt/d/okur-linux/a11y-stack.sh bash /mnt/d/okur-linux/probe-gtk-tree.sh [desktop-forms]
set -uo pipefail
name="${1:-desktop-forms}"
python3 "$HOME/okur/corpus/fixtures/linux/$name.py" 120 > /tmp/replica.log 2>&1 &
replica=$!
sleep 6
python3 - <<'PY'
import time
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

STATES = [("focused", Atspi.StateType.FOCUSED), ("checked", Atspi.StateType.CHECKED),
          ("selected", Atspi.StateType.SELECTED), ("expanded", Atspi.StateType.EXPANDED),
          ("expandable", Atspi.StateType.EXPANDABLE), ("insensitive", None)]

def states(node):
    s = node.get_state_set()
    out = [n for n, t in STATES if t is not None and s.contains(t)]
    if not s.contains(Atspi.StateType.SENSITIVE):
        out.append("insensitive")
    return out

def relations(node):
    out = []
    try:
        for r in node.get_relation_set():
            kind = r.get_relation_type().value_nick
            targets = [r.get_target(i).get_name() for i in range(r.get_n_targets())]
            out.append(f"{kind}={targets}")
    except Exception:
        pass
    return out

def value(node):
    try:
        text = node.get_text_iface()
        if text is not None:
            return Atspi.Text.get_text(node, 0, -1)
    except Exception:
        pass
    return None

def dump(node, depth=0):
    if node is None or depth > 30:
        return
    try:
        line = (f"{'  ' * depth}[{node.get_index_in_parent()}] {node.get_role_name()} "
                f"{node.get_name()!r} {states(node)} {relations(node)}")
        v = value(node)
        if v:
            line += f" text={v!r}"
        print(line)
        for i in range(node.get_child_count()):
            dump(node.get_child_at_index(i), depth + 1)
    except Exception as error:
        print(f"{'  ' * depth}! {error}")

desktop = Atspi.get_desktop(0)
for i in range(desktop.get_child_count()):
    app = desktop.get_child_at_index(i)
    if app is not None:
        print(f"== application {app.get_name()!r}")
        dump(app)
PY
# The focus as Tab moves, each focused element with its ancestors.
python3 - <<'PY'
import subprocess
import time
import gi
gi.require_version("Atspi", "2.0")
from gi.repository import Atspi

def chain(node):
    out = []
    while node is not None and len(out) < 20:
        try:
            out.append(f"{node.get_role_name()}:{node.get_name()!r}[{node.get_index_in_parent()}]")
            node = node.get_parent()
        except Exception:
            break
    return " < ".join(out)

def focused(node, depth=0):
    if node is None or depth > 30:
        return None
    try:
        if node.get_state_set().contains(Atspi.StateType.FOCUSED):
            return node
        for i in range(node.get_child_count()):
            found = focused(node.get_child_at_index(i), depth + 1)
            if found:
                return found
    except Exception:
        return None
    return None

desktop = Atspi.get_desktop(0)
apps = [desktop.get_child_at_index(i) for i in range(desktop.get_child_count())]
apps = [a for a in apps if a is not None]
for step in range(14):
    for app in apps:
        node = focused(app)
        if node:
            print(f"tab {step}: {chain(node)}")
    subprocess.run(["xdotool", "key", "Tab"])
    time.sleep(0.7)
PY
kill "$replica" 2>/dev/null
tail -5 /tmp/replica.log
