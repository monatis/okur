#!/usr/bin/env python3
"""SR Desktop Fixture on Linux: desktop-forms.ps1's window rebuilt in GTK 4, the same controls,
labels, values, Tab order and first focus, so that Okur's runs on Linux can be compared with
JAWS's Windows evidence of the scenarios that use it (docs/plans/2026-11-05-linux-first.md §2.4:
a check box, a combo box, a dialog said as on Windows). A platform's rendering of the fixture,
never JAWS evidence. Run by run-scenario-linux.py in the lab's headless desktop.

    python3 desktop-forms.py [seconds before it quits, default 900]

Do not change it without running again every Linux run that uses it.
"""

import sys

import gi

gi.require_version("Gtk", "4.0")
from gi.repository import Gio, GLib, Gtk  # noqa: E402


def labelled(text, widget):
    """A mnemonic label naming `widget`, as WinForms' label before a field names it."""
    label = Gtk.Label.new_with_mnemonic(text)
    label.set_mnemonic_widget(widget)
    label.set_xalign(0)
    return label


def build(app):
    window = Gtk.ApplicationWindow(application=app, title="SR Desktop Fixture")
    window.set_default_size(560, 520)

    # The menu bar: File (New, Open..., Exit), Edit (Copy, Paste), Help (About).
    menu = Gio.Menu()
    for top, items in (("_File", ("_New", "_Open...", "E_xit")),
                       ("_Edit", ("_Copy", "_Paste")),
                       ("_Help", ("_About",))):
        section = Gio.Menu()
        for item in items:
            section.append(item, "app.nothing")
        menu.append_submenu(top, section)
    nothing = Gio.SimpleAction.new("nothing", None)
    app.add_action(nothing)

    box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=6)
    box.set_margin_start(12)
    box.set_margin_end(12)
    box.append(Gtk.PopoverMenuBar.new_from_model(menu))

    name = Gtk.Entry(text="Ada")
    box.append(labelled("_Name:", name))
    box.append(name)

    notes = Gtk.TextView()
    scrolled = Gtk.ScrolledWindow(child=notes, min_content_height=44)
    box.append(labelled("N_otes:", notes))
    box.append(scrolled)

    checks = Gtk.Box(spacing=12)
    subscribe = Gtk.CheckButton.new_with_mnemonic("_Subscribe to updates")
    subscribe.set_active(True)
    remember = Gtk.CheckButton.new_with_mnemonic("_Remember me")
    checks.append(subscribe)
    checks.append(remember)
    box.append(checks)

    # The plan: a group of three radio buttons, Pro chosen.
    plan = Gtk.Frame(label="Plan")
    radios = Gtk.Box(spacing=12)
    first = None
    for choice in ("Free", "Pro", "Team"):
        radio = Gtk.CheckButton(label=choice)
        if first is None:
            first = radio
        else:
            radio.set_group(first)
        radio.set_active(choice == "Pro")
        radios.append(radio)
    plan.set_child(radios)
    box.append(plan)

    country = Gtk.DropDown.new_from_strings(["Turkey", "Germany", "Brazil"])
    country.set_selected(1)
    box.append(labelled("_Country:", country))
    box.append(country)

    fruit = Gtk.ListBox()
    fruit.set_selection_mode(Gtk.SelectionMode.SINGLE)
    for item in ("Apples", "Bananas", "Cherries"):
        fruit.append(Gtk.Label(label=item, xalign=0))
    fruit.select_row(fruit.get_row_at_index(1))
    box.append(labelled("_Fruit:", fruit))
    box.append(fruit)

    # The folders' tree: Documents (Reports, Invoices) and Pictures, Documents selected.
    folders = Gtk.TreeStore(str)
    documents = folders.append(None, ["Documents"])
    folders.append(documents, ["Reports"])
    folders.append(documents, ["Invoices"])
    folders.append(None, ["Pictures"])
    tree = Gtk.TreeView(model=folders, headers_visible=False)
    tree.append_column(Gtk.TreeViewColumn("Folder", Gtk.CellRendererText(), text=0))
    tree.set_cursor(Gtk.TreePath.new_first(), None, False)
    box.append(tree)

    # The files' list in columns, Name and Size, its first row selected.
    files = Gtk.ListStore(str, str)
    for row in (("budget.xlsx", "24 KB"), ("notes.txt", "2 KB"), ("photo.jpg", "1.2 MB")):
        files.append(list(row))
    table = Gtk.TreeView(model=files)
    for index, title in enumerate(("Name", "Size")):
        table.append_column(Gtk.TreeViewColumn(title, Gtk.CellRendererText(), text=index))
    table.set_cursor(Gtk.TreePath.new_first(), None, False)
    box.append(table)

    tabs = Gtk.Notebook()
    for title in ("General", "Advanced"):
        tabs.append_page(Gtk.Label(label=f"{title} settings", xalign=0), Gtk.Label(label=title))
    box.append(tabs)

    buttons = Gtk.Box(spacing=8, halign=Gtk.Align.END)
    ok = Gtk.Button(label="OK")
    cancel = Gtk.Button(label="Cancel")
    apply_ = Gtk.Button.new_with_mnemonic("_Apply")
    apply_.set_sensitive(False)
    for button in (ok, cancel, apply_):
        buttons.append(button)
    box.append(buttons)
    window.set_default_widget(ok)

    box.append(Gtk.Label(label="Ready", xalign=0))
    window.set_child(box)
    window.present()
    name.grab_focus()


def main():
    seconds = int(sys.argv[1]) if len(sys.argv) > 1 else 900
    app = Gtk.Application(application_id="org.okur.SrDesktopFixture",
                          flags=Gio.ApplicationFlags.NON_UNIQUE)
    app.connect("activate", build)
    GLib.timeout_add_seconds(seconds, app.quit)
    app.run([sys.argv[0]])


if __name__ == "__main__":
    main()
