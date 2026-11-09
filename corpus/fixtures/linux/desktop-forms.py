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
from gi.repository import Gdk, Gio, GLib, Gtk  # noqa: E402


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

    # Tab leaves the field, as WinForms' multi-line TextBox (AcceptsTab off) lets it.
    notes = Gtk.TextView(accepts_tab=False)
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

    # The plan: a group of three radio buttons, Pro chosen, one under another: GTK's arrows move
    # by where the radios are, and Down from a row of them left the group for the drop-down
    # below (548/s6 in night2), where the Windows fixture's Down goes on to Team as any Win32
    # radio group's does [obs, 548/s6].
    plan = Gtk.Frame(label="Plan")
    radios = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=4)
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

    def descendant(widget, kind):
        """The first widget of `kind` within `widget`, depth first."""
        child = widget.get_first_child()
        while child is not None:
            if isinstance(child, kind):
                return child
            found = descendant(child, kind)
            if found is not None:
                return found
            child = child.get_next_sibling()
        return None

    # WinForms' list opens on the chosen item, GTK's on its first: the open list is put on the
    # chosen one, as the Windows fixture's opens (304/s11, JAWS: "Open List Box Germany 2 of 3").
    def open_on_chosen():
        items = descendant(country, Gtk.ListView)
        if items is not None:
            items.scroll_to(country.get_selected(),
                            Gtk.ListScrollFlags.FOCUS | Gtk.ListScrollFlags.SELECT, None)
        return False

    # Closed, Down and Up choose the next and the previous country and Alt+Down opens the
    # list, as in WinForms' combo box (GTK's drop-down leaves the arrows to move the focus).
    def country_key(_controller, keyval, _keycode, state):
        count = country.get_model().get_n_items()
        chosen = country.get_selected()
        if state & Gdk.ModifierType.ALT_MASK:
            if keyval == Gdk.KEY_Down:
                country.activate()
                GLib.idle_add(open_on_chosen)
                return True
            return False
        if keyval == Gdk.KEY_Down and chosen + 1 < count:
            country.set_selected(chosen + 1)
            return True
        if keyval == Gdk.KEY_Up and chosen > 0:
            country.set_selected(chosen - 1)
            return True
        return keyval in (Gdk.KEY_Down, Gdk.KEY_Up)

    # On its button, which has the focus while it is closed: its open list is a child of the
    # drop-down too, where the arrows move through the list.
    button = country.get_first_child()
    while button is not None and not isinstance(button, Gtk.ToggleButton):
        button = button.get_next_sibling()
    if button is None:
        raise RuntimeError("GTK's drop-down has no toggle button")
    country_keys = Gtk.EventControllerKey(propagation_phase=Gtk.PropagationPhase.CAPTURE)
    country_keys.connect("key-pressed", country_key)
    button.add_controller(country_keys)
    box.append(labelled("_Country:", country))
    box.append(country)

    # The fruit list, Bananas selected; Tab leaves it, as WinForms' list box (GTK's list view's
    # tab behavior "item"; its list box moves from row to row).
    def label_setup(_factory, cell):
        cell.set_child(Gtk.Label(xalign=0))

    def label_bind(_factory, cell):
        cell.get_child().set_label(cell.get_item().get_string())

    labels = Gtk.SignalListItemFactory()
    labels.connect("setup", label_setup)
    labels.connect("bind", label_bind)
    fruit_model = Gtk.StringList()
    for item in ("Apples", "Bananas", "Cherries"):
        fruit_model.append(item)
    fruit = Gtk.ListView(model=Gtk.SingleSelection(model=fruit_model, selected=1),
                         factory=labels, tab_behavior=Gtk.ListTabBehavior.ITEM)
    box.append(labelled("_Fruit:", fruit))
    box.append(fruit)

    # The folders' tree: Documents (Reports, Invoices) and Pictures, Documents selected; GTK 4's
    # list view over a tree list model (its deprecated TreeView shows no rows to AT-SPI).
    children = {"Documents": ["Reports", "Invoices"]}

    def strings(names):
        model = Gtk.StringList()
        for item in names:
            model.append(item)
        return model

    def child_model(item):
        names = children.get(item.get_string())
        return strings(names) if names else None

    folders = Gtk.TreeListModel.new(strings(["Documents", "Pictures"]), False, False, child_model)

    def tree_setup(_factory, cell):
        expander = Gtk.TreeExpander()
        expander.set_child(Gtk.Label(xalign=0))
        cell.set_child(expander)

    def tree_bind(_factory, cell):
        row = cell.get_item()
        expander = cell.get_child()
        expander.set_list_row(row)
        expander.get_child().set_label(row.get_item().get_string())

    tree_factory = Gtk.SignalListItemFactory()
    tree_factory.connect("setup", tree_setup)
    tree_factory.connect("bind", tree_bind)
    tree = Gtk.ListView(model=Gtk.SingleSelection(model=folders), factory=tree_factory,
                        tab_behavior=Gtk.ListTabBehavior.ITEM)

    # Right opens the folder selected and Left closes it, as in WinForms' tree (GTK's expander
    # opens with plus and closes with minus).
    def tree_key(_controller, keyval, _keycode, _state):
        row = tree.get_model().get_selected_item()
        if row is None or not row.is_expandable():
            return False
        if keyval == Gdk.KEY_Right and not row.get_expanded():
            row.set_expanded(True)
            return True
        if keyval == Gdk.KEY_Left and row.get_expanded():
            row.set_expanded(False)
            return True
        return False

    tree_keys = Gtk.EventControllerKey(propagation_phase=Gtk.PropagationPhase.CAPTURE)
    tree_keys.connect("key-pressed", tree_key)
    tree.add_controller(tree_keys)
    box.append(tree)

    # The files' list in columns, Name and Size, its first row selected.
    rows = (("budget.xlsx", "24 KB"), ("notes.txt", "2 KB"), ("photo.jpg", "1.2 MB"))
    files = strings([name for name, _ in rows])
    sizes = dict(rows)
    table = Gtk.ColumnView(model=Gtk.SingleSelection(model=files),
                           tab_behavior=Gtk.ListTabBehavior.ITEM)

    def column(title, text_of):
        def setup(_factory, cell):
            cell.set_child(Gtk.Label(xalign=0))

        def bind(_factory, cell):
            cell.get_child().set_label(text_of(cell.get_item().get_string()))

        factory = Gtk.SignalListItemFactory()
        factory.connect("setup", setup)
        factory.connect("bind", bind)
        return Gtk.ColumnViewColumn(title=title, factory=factory)

    table.append_column(column("Name", lambda name: name))
    table.append_column(column("Size", lambda name: sizes[name]))
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
    # The fruit list's focus on its selected item, Bananas, where Tab enters it, as in WinForms'
    # list box (GTK's list view enters at the item it focused last, else the first).
    fruit.scroll_to(1, Gtk.ListScrollFlags.FOCUS, None)
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
