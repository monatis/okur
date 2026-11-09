//! Okur's own windows on Linux, drawn with GTK 4 (`helper-ui-architecture.md`; the Linux plan's
//! "Okur's own dialogs"): a dialog of [`DialogSpec`] as a GTK window of the same controls, read
//! by Okur through AT-SPI like any GTK application's dialog. What the user does goes back to
//! Okur as Windows' renderer tells it ([`FromUi`]).
//!
//! First increment: the dialogs of the element lists (a list or list box, radio buttons, the
//! buttons, a line of text) and the plain controls (check boxes, drop-downs, edits, sliders,
//! notes, messages, group boxes). Trees, tables, check lists, menus, the wizard, the main window,
//! the Virtual Viewer and the reading highlight come next; a dialog holding one goes on
//! without it.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use okur_ui_model::{
    decode, encode, Control, ControlKind, DialogKey, DialogSpec, FromUi, ToUi, TreeItem, Value,
    WizardButtons, WizardLabels, WizardSpec,
};

/// Draws what Okur sends until its input ends; without a display, reads and drops it, so that
/// Okur never waits on a full pipe.
pub fn run() {
    // Okur's dialogs are plain controls, drawn at once by GTK's cairo renderer, without the
    // GPU renderer's start: the lab's Links List came 5.4 s after Insert+F7 with OpenGL drawn
    // in software (gtk2, 2026-11-09). A renderer the user chose is kept. Set before GTK starts
    // and before any thread does, as the environment is read then.
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "cairo");
    }
    // Its name on the accessibility bus, which had none (gtk2's trace of 222): the engine
    // knows Okur's own windows by it, as by okur-ui's process on Windows. GTK names the
    // application by its program name where no other is set.
    glib::set_prgname(Some("okur-ui"));
    // Its application name too: the traces still had none (gtk4's 222 at 298f0c7c).
    glib::set_application_name("okur-ui");
    if let Err(e) = gtk::init() {
        eprintln!("okur-ui: no display for GTK: {e}");
        for _ in std::io::stdin().lock().lines() {}
        return;
    }
    let (messages, received) = std::sync::mpsc::channel::<Option<ToUi>>();
    let main_loop = glib::MainLoop::new(None, false);
    // GTK's side, before the reader can wake it for a message.
    GTK_SIDE.with(|side| {
        *side.borrow_mut() = Some(GtkSide {
            ui: Rc::new(Ui::default()),
            received,
            main_loop: main_loop.clone(),
        });
    });
    let context = glib::MainContext::default();
    let reader = std::thread::Builder::new()
        .name("okur-ui-input".to_owned())
        .spawn(move || {
            for line in std::io::stdin().lock().lines() {
                let Ok(line) = line else { break };
                if line.trim().is_empty() {
                    continue;
                }
                match decode::<ToUi>(&line) {
                    Ok(message) => {
                        if messages.send(Some(message)).is_err() {
                            return;
                        }
                        context.invoke_with_priority(glib::Priority::DEFAULT, take_messages);
                    }
                    Err(e) => eprintln!("okur-ui: unreadable message {line:?}: {e}"),
                }
            }
            let _ = messages.send(None);
            context.invoke_with_priority(glib::Priority::DEFAULT, take_messages);
        });
    if reader.is_err() {
        eprintln!("okur-ui: no thread to read Okur's messages");
        return;
    }
    main_loop.run();
}

/// What GTK's thread keeps for the messages the reader wakes it for: the windows, the messages,
/// and the loop to end when Okur's input ends.
struct GtkSide {
    ui: Rc<Ui>,
    received: std::sync::mpsc::Receiver<Option<ToUi>>,
    main_loop: glib::MainLoop,
}

thread_local! {
    static GTK_SIDE: RefCell<Option<GtkSide>> = const { RefCell::new(None) };
}

/// Okur's messages, taken on GTK's thread as the reader wakes it (no polling: okur-ui woke a
/// hundred times a second for them, the code review of 2026-11-09, fourth pass, 10).
fn take_messages() {
    GTK_SIDE.with(|side| {
        let side = side.borrow();
        let Some(side) = side.as_ref() else {
            return;
        };
        for message in side.received.try_iter() {
            match message {
                Some(message) => Ui::handle(&side.ui, message),
                None => side.main_loop.quit(),
            }
        }
    });
}

/// Tells Okur what the user did, one line of JSON.
fn send(message: &FromUi) {
    if let Ok(line) = encode(message) {
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
    }
}

/// A label's access key as GTK marks it: Okur's "&" before the letter (Windows' mark) is GTK's
/// "_"; "&&" is a plain "&", and a plain "_" is doubled.
fn mnemonic(label: &str) -> String {
    let mut out = String::with_capacity(label.len() + 2);
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
                out.push('&');
            }
            '&' => out.push('_'),
            '_' => out.push_str("__"),
            c => out.push(c),
        }
    }
    out
}

/// A label without its access key's mark, as a list's or a field's accessible name.
fn plain(label: &str) -> String {
    mnemonic(label)
        .replace("__", "\u{0}")
        .replace('_', "")
        .replace('\u{0}', "_")
}

/// One control of a shown dialog, as it is updated.
enum Shown {
    Check(gtk::CheckButton),
    Choice(gtk::DropDown),
    Entry(gtk::Entry),
    Scale(gtk::Scale),
    Button(gtk::Button),
    List(ShownList),
    Tree(ShownTree),
    Radios(Vec<gtk::CheckButton>),
    Text(gtk::Label),
    Note(gtk::TextView),
    /// A control this renderer does not draw yet.
    Other(gtk::Widget),
}

impl Shown {
    fn widget(&self) -> Option<gtk::Widget> {
        Some(match self {
            Shown::Check(w) => w.clone().upcast(),
            Shown::Choice(w) => w.clone().upcast(),
            Shown::Entry(w) => w.clone().upcast(),
            Shown::Scale(w) => w.clone().upcast(),
            Shown::Button(w) => w.clone().upcast(),
            Shown::List(list) => list.widget(),
            Shown::Tree(tree) => tree.view.upgrade()?.upcast(),
            Shown::Radios(buttons) => buttons.first()?.clone().upcast(),
            Shown::Text(w) => w.clone().upcast(),
            Shown::Note(w) => w.clone().upcast(),
            Shown::Other(w) => w.clone(),
        })
    }
}

/// A list of Okur's: its items, the one selected, and the view showing them: a column view for
/// a list view (one column, as the element lists' list view on Windows: "links List view"), a
/// list view for a list box. Tab leaves either after its item, as a Windows list is one stop;
/// GTK's list box moved Tab from row to row (224 in gtk2).
#[derive(Clone)]
struct ShownList {
    view: ListWidget,
    items: gtk::StringList,
    selection: gtk::SingleSelection,
}

#[derive(Clone)]
enum ListWidget {
    Columns(gtk::ColumnView),
    Rows(gtk::ListView),
}

impl ShownList {
    fn widget(&self) -> gtk::Widget {
        match &self.view {
            ListWidget::Columns(view) => view.clone().upcast(),
            ListWidget::Rows(view) => view.clone().upcast(),
        }
    }

    /// Its items in place of those it had.
    fn fill(&self, items: &[String]) {
        let texts: Vec<&str> = items.iter().map(String::as_str).collect();
        self.items.splice(0, self.items.n_items(), &texts);
    }

    /// A table's rows in place of those it had, each its cells joined ([`CELLS`]).
    fn fill_rows(&self, rows: &[Vec<String>]) {
        let joined: Vec<String> = rows.iter().map(|cells| cells.join(CELLS)).collect();
        let texts: Vec<&str> = joined.iter().map(String::as_str).collect();
        self.items.splice(0, self.items.n_items(), &texts);
    }

    /// The next item after the selected one whose text starts with `letter`, wrapping, selected
    /// and focused: a Windows list's own search by first letter ("h" in Virtual HTML Features
    /// is "Headings List | 12 of 22" [obs, 226/s2]), which GTK's lists lack. Whether one did.
    fn jump_to(&self, letter: char) -> bool {
        let count = self.items.n_items();
        let selected = self.selection.selected();
        let wanted: String = letter.to_lowercase().collect();
        for step in 1..=count {
            let at = if selected < count {
                (selected + step) % count
            } else {
                step - 1
            };
            let Some(text) = self.items.string(at) else {
                continue;
            };
            let first = text
                .split(CELLS)
                .next()
                .unwrap_or_default()
                .trim_start()
                .to_lowercase();
            if first.starts_with(&wanted) {
                self.select(usize::try_from(at).unwrap_or(usize::MAX), true);
                return true;
            }
        }
        false
    }

    /// Letters typed in it move to the next item that starts with them.
    fn search_by_letter(&self) {
        let keys = gtk::EventControllerKey::new();
        // The model and the selection alone: the view, which owns this handler, is the one it
        // is attached to (holding it would keep every closed dialog's list alive).
        let items = self.items.clone();
        let selection = self.selection.clone();
        keys.connect_key_pressed(move |keys, key, _, modifiers| {
            let chord = gtk::gdk::ModifierType::CONTROL_MASK
                | gtk::gdk::ModifierType::ALT_MASK
                | gtk::gdk::ModifierType::SUPER_MASK;
            if modifiers.intersects(chord) {
                return glib::Propagation::Proceed;
            }
            let Some(letter) = key.to_unicode().filter(|c| c.is_alphanumeric()) else {
                return glib::Propagation::Proceed;
            };
            let view = match keys.widget() {
                Some(widget) => match widget.downcast::<gtk::ColumnView>() {
                    Ok(view) => ListWidget::Columns(view),
                    Err(widget) => match widget.downcast::<gtk::ListView>() {
                        Ok(view) => ListWidget::Rows(view),
                        Err(_) => return glib::Propagation::Proceed,
                    },
                },
                None => return glib::Propagation::Proceed,
            };
            let list = ShownList {
                view,
                items: items.clone(),
                selection: selection.clone(),
            };
            if list.jump_to(letter) {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        self.widget().add_controller(keys);
    }

    /// The focus on its selected item, or on its first where none is selected (a table with
    /// none chosen yet: "Not Selected", JAWS's PlaceMarker List [obs, 702/s18]).
    fn take_focus(&self) {
        let count = self.items.n_items();
        if count == 0 {
            return;
        }
        let selected = self.selection.selected();
        let (at, flags) = if selected < count {
            (
                selected,
                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
            )
        } else {
            (0, gtk::ListScrollFlags::FOCUS)
        };
        match &self.view {
            ListWidget::Columns(view) => {
                view.scroll_to(at, None::<&gtk::ColumnViewColumn>, flags, None);
            }
            ListWidget::Rows(view) => view.scroll_to(at, flags, None),
        }
    }

    /// Item `i` selected, the focus on it where the list has the focus, or `focus`.
    fn select(&self, i: usize, focus: bool) {
        let Ok(at) = u32::try_from(i) else {
            return;
        };
        if at >= self.items.n_items() {
            return;
        }
        self.selection.set_selected(at);
        let widget = self.widget();
        if focus || widget.has_focus() || widget.focus_child().is_some() {
            let flags = gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT;
            match &self.view {
                ListWidget::Columns(view) => {
                    view.scroll_to(at, None::<&gtk::ColumnViewColumn>, flags, None);
                }
                ListWidget::Rows(view) => view.scroll_to(at, flags, None),
            }
        }
    }
}

/// A tree of Okur's: its items in their order, each with its level, and a list view of GTK's tree
/// list model over them, each row with its expander. An item is told by its index in that
/// order, as Windows' tree view tells it: chosen (`Value::Selected`), opened or closed
/// (`Value::ItemOpened`). Right opens an item, or goes to its first; Left closes it, or goes to
/// its parent, as a Windows tree view's arrows do; Space and Shift+Space change it.
#[derive(Clone)]
struct ShownTree {
    /// Held weakly: its handlers hold the tree, and the dialog's window holds the view.
    view: glib::WeakRef<gtk::ListView>,
    items: Rc<RefCell<Vec<TreeItem>>>,
    /// The top items' indexes, as the model's roots.
    roots: gtk::StringList,
    model: gtk::TreeListModel,
    selection: gtk::SingleSelection,
    /// Okur is filling or opening it: not the user's doing, not told.
    filling: Rc<Cell<bool>>,
}

impl ShownTree {
    /// The items in `items` that are `at`'s children, by index: those after it one level
    /// deeper, up to the next at its level or above.
    fn children(items: &[TreeItem], at: usize) -> Vec<usize> {
        let Some(level) = items.get(at).map(|item| item.level) else {
            return Vec::new();
        };
        items
            .iter()
            .enumerate()
            .skip(at + 1)
            .take_while(|(_, item)| item.level > level)
            .filter(|(_, item)| item.level == level + 1)
            .map(|(i, _)| i)
            .collect()
    }

    /// The top items, by index: those at the first item's level or above.
    fn tops(items: &[TreeItem]) -> Vec<usize> {
        let top = items.iter().map(|item| item.level).min().unwrap_or(0);
        items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.level == top)
            .map(|(i, _)| i)
            .collect()
    }

    /// The item a row of the model stands for, by its index.
    fn index_of(row: &gtk::TreeListRow) -> Option<usize> {
        row.item()
            .and_downcast::<gtk::StringObject>()
            .and_then(|text| text.string().parse().ok())
    }

    /// `items` in place of those it had, each opened as it says, and `selected` chosen.
    fn fill(&self, items: &[TreeItem], selected: usize) {
        self.filling.set(true);
        *self.items.borrow_mut() = items.to_vec();
        let tops: Vec<String> = Self::tops(items).iter().map(usize::to_string).collect();
        let tops: Vec<&str> = tops.iter().map(String::as_str).collect();
        self.roots.splice(0, self.roots.n_items(), &tops);
        // Open the opened ones, from the top: each opening adds its rows after it.
        let mut at = 0;
        while at < self.model.n_items() {
            if let Some(row) = self.model.row(at) {
                let open = Self::index_of(&row)
                    .and_then(|i| self.items.borrow().get(i).map(|item| item.open))
                    .unwrap_or(false);
                if open && row.is_expandable() {
                    row.set_expanded(true);
                }
            }
            at += 1;
        }
        self.select(selected, false);
        self.filling.set(false);
    }

    /// The model's position of item `index`, its parents opened for it.
    fn position_of(&self, index: usize) -> Option<u32> {
        // Its parents, from the top: each the nearest item before it one level up.
        let items = self.items.borrow().clone();
        let mut parents = Vec::new();
        let mut level = items.get(index)?.level;
        for (i, item) in items.iter().enumerate().take(index).rev() {
            if item.level < level {
                parents.push(i);
                level = item.level;
            }
        }
        for parent in parents.into_iter().rev() {
            if let Some(row) = self.row_of(parent) {
                if !row.is_expanded() {
                    row.set_expanded(true);
                }
            }
        }
        (0..self.model.n_items()).find(|at| {
            self.model
                .row(*at)
                .and_then(|row| Self::index_of(&row))
                .is_some_and(|i| i == index)
        })
    }

    fn row_of(&self, index: usize) -> Option<gtk::TreeListRow> {
        (0..self.model.n_items())
            .filter_map(|at| self.model.row(at))
            .find(|row| Self::index_of(row) == Some(index))
    }

    /// Item `index` chosen, the focus on it where `focus` or the tree has the focus.
    fn select(&self, index: usize, focus: bool) {
        let filling = self.filling.replace(true);
        if let Some(at) = self.position_of(index) {
            self.selection.set_selected(at);
            let Some(view) = self.view.upgrade() else {
                self.filling.set(filling);
                return;
            };
            if focus || view.has_focus() || view.focus_child().is_some() {
                view.scroll_to(
                    at,
                    gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                    None,
                );
            }
        }
        self.filling.set(filling);
    }

    /// The focus on its chosen item, as a list's first focus is.
    fn take_focus(&self) {
        let at = self.selection.selected();
        let Some(view) = self.view.upgrade() else {
            return;
        };
        if at < self.model.n_items() {
            view.scroll_to(
                at,
                gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                None,
            );
        }
    }

    /// The chosen row.
    fn chosen(&self) -> Option<gtk::TreeListRow> {
        self.model.row(self.selection.selected())
    }
}

/// Between a table row's cells in its model's string (a unit separator, in no cell's text).
const CELLS: &str = "\u{1f}";

/// A shown dialog: its window, its controls by their ids, the labels that name them, and
/// whether Okur is setting a value (not the user, so not told back).
struct ShownDialog {
    window: gtk::Window,
    controls: HashMap<String, Shown>,
    labels: HashMap<String, gtk::Label>,
    setting: Rc<Cell<bool>>,
}

/// A wizard's parts besides its page's controls (which are its dialog's): the page's box, its
/// buttons, and what they do now.
struct ShownWizard {
    page: gtk::Box,
    back: gtk::Button,
    next: gtk::Button,
    buttons: Rc<Cell<WizardButtons>>,
    labels: WizardLabels,
}

/// The dialogs shown, by Okur's numbers; a wizard's page is its dialog, by the wizard's.
#[derive(Default)]
struct Ui {
    dialogs: RefCell<HashMap<u32, ShownDialog>>,
    wizards: RefCell<HashMap<u32, ShownWizard>>,
}

/// The controls of a dialog, or of a wizard's page, built into a box.
struct Built {
    controls: HashMap<String, Shown>,
    labels: HashMap<String, gtk::Label>,
    /// The control the focus goes to first.
    first: Option<gtk::Widget>,
}

impl Ui {
    fn handle(ui: &Rc<Ui>, message: ToUi) {
        match message {
            ToUi::DialogShow { dialog } => Ui::show(ui, dialog),
            ToUi::WizardShow { wizard } => Ui::show_wizard(ui, wizard),
            ToUi::WizardPage { id, page, buttons } => Ui::turn_wizard(ui, id, &page, buttons),
            ToUi::WizardButtons { id, buttons } => {
                let window = ui.dialogs.borrow().get(&id).map(|d| d.window.clone());
                if let (Some(wizard), Some(window)) = (ui.wizards.borrow().get(&id), window) {
                    set_wizard_buttons(wizard, &window, buttons);
                }
            }
            ToUi::DialogClose { id } => {
                // Out of the map before its window goes: destroying it may call back into it.
                ui.wizards.borrow_mut().remove(&id);
                let dialog = ui.dialogs.borrow_mut().remove(&id);
                if let Some(dialog) = dialog {
                    dialog.window.destroy();
                }
            }
            ToUi::DialogRows {
                id,
                control,
                rows,
                selected,
                ..
            } => ui.rows(id, &control, &rows, selected),
            ToUi::DialogSet { id, control, value } => ui.set(id, &control, &value),
            ToUi::DialogTree {
                id,
                control,
                items,
                selected,
            } => {
                let dialogs = ui.dialogs.borrow();
                if let Some(Shown::Tree(tree)) = dialogs
                    .get(&id)
                    .and_then(|dialog| dialog.controls.get(&control))
                {
                    tree.fill(&items, selected);
                }
            }
            ToUi::DialogItems {
                id,
                control,
                items,
                selected,
            } => ui.items(id, &control, &items, selected),
            ToUi::DialogEnable {
                id,
                control,
                enabled,
            } => {
                if let Some(widget) = ui.widget(id, &control) {
                    widget.set_sensitive(enabled);
                }
            }
            ToUi::DialogFocus { id, control } => {
                if let Some(widget) = ui.widget(id, &control) {
                    widget.grab_focus();
                }
            }
            ToUi::DialogDefault { id, control } => {
                let dialogs = ui.dialogs.borrow();
                if let Some(dialog) = dialogs.get(&id) {
                    if let Some(Shown::Button(button)) = dialog.controls.get(&control) {
                        dialog.window.set_default_widget(Some(button));
                    }
                }
            }
            ToUi::DialogLabel { id, control, label } => {
                let dialogs = ui.dialogs.borrow();
                if let Some(dialog) = dialogs.get(&id) {
                    if let Some(shown) = dialog.labels.get(&control) {
                        shown.set_text_with_mnemonic(&mnemonic(&label));
                    }
                    if let Some(widget) = dialog.controls.get(&control).and_then(Shown::widget) {
                        widget.update_property(&[gtk::accessible::Property::Label(&plain(&label))]);
                    }
                }
            }
            // Not drawn on Linux yet: the dialog goes on without them.
            other => eprintln!("okur-ui: not drawn on Linux yet: {}", kind(&other)),
        }
    }

    fn widget(&self, id: u32, control: &str) -> Option<gtk::Widget> {
        self.dialogs
            .borrow()
            .get(&id)?
            .controls
            .get(control)?
            .widget()
    }

    /// Shows a dialog in front, the focus on its first control; one already shown comes to the
    /// front.
    fn show(ui: &Rc<Ui>, spec: DialogSpec) {
        if let Some(dialog) = ui.dialogs.borrow().get(&spec.id) {
            dialog.window.present();
            let id = spec.id;
            once_active(&dialog.window, move || send(&FromUi::DialogShown { id }));
            return;
        }
        let id = spec.id;
        // A dialog box to the accessibility bus, as Windows' dialog is: "Links List dialog".
        let window = gtk::Window::builder()
            .accessible_role(gtk::AccessibleRole::Dialog)
            .build();
        window.set_title(Some(&spec.title));
        window.set_default_size(480, -1);
        let setting = Rc::new(Cell::new(false));
        let outer = margined_box();
        let Built {
            controls,
            labels,
            first,
        } = build_controls(ui, id, &spec.controls, &outer, &window, &setting);
        window.set_child(Some(&outer));
        Ui::closing(ui, &window, id);
        ui.dialogs.borrow_mut().insert(
            id,
            ShownDialog {
                window: window.clone(),
                controls,
                labels,
                setting,
            },
        );
        window.present();
        focus_and_tell_shown(ui, &window, id, first);
    }

    /// Escape and the close box are the dialog's Cancel, or the wizard's: Okur is told it was
    /// cancelled, and it goes.
    fn closing(ui: &Rc<Ui>, window: &gtk::Window, id: u32) {
        let keys = gtk::EventControllerKey::new();
        {
            let ui = Rc::clone(ui);
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    ui.cancelled(id);
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            });
        }
        window.add_controller(keys);
        let ui = Rc::clone(ui);
        window.connect_close_request(move |_| {
            ui.cancelled(id);
            glib::Propagation::Stop
        });
    }

    /// The focus on dialog `id`'s first control, `first`. A list taking the focus as the
    /// dialog opens has it on its selected item, as Windows' list does: "links List view |
    /// Current Page Overview | 1 of 15". Once GTK has laid the dialog out (an idle comes after
    /// its layout): focused at once, the row's object was gone as Okur read it, and the list
    /// and place went unsaid (222/s1 in full5).
    fn focus_first(&self, id: u32, first: Option<gtk::Widget>) {
        let Some(first) = first else {
            return;
        };
        let (list, tree) = self
            .dialogs
            .borrow()
            .get(&id)
            .map(|dialog| {
                let list = dialog.controls.values().find_map(|shown| match shown {
                    Shown::List(list) if list.widget() == first => Some(list.clone()),
                    _ => None,
                });
                let tree = dialog.controls.values().find_map(|shown| match shown {
                    Shown::Tree(tree)
                        if tree
                            .view
                            .upgrade()
                            .is_some_and(|view| view.upcast::<gtk::Widget>() == first) =>
                    {
                        Some(tree.clone())
                    }
                    _ => None,
                });
                (list, tree)
            })
            .unwrap_or_default();
        match (list, tree) {
            (Some(list), _) => {
                let _ = glib::idle_add_local_once(move || list.take_focus());
            }
            (None, Some(tree)) => {
                let _ = glib::idle_add_local_once(move || tree.take_focus());
            }
            (None, None) => {
                first.grab_focus();
            }
        }
    }

    /// Shows a wizard in front, its first page under Back, Next (or Finish) and Cancel, named
    /// as a Windows property-sheet wizard names them ("< Back", "Next >", "Cancel" [obs,
    /// 705/s5, s10]); one already shown comes to the front. Okur decides where it goes: its
    /// buttons are told ("back", "next", "finish"), and Okur turns the page.
    fn show_wizard(ui: &Rc<Ui>, spec: WizardSpec) {
        let id = spec.id;
        if let Some(dialog) = ui.dialogs.borrow().get(&id) {
            dialog.window.present();
            return;
        }
        let window = gtk::Window::builder()
            .accessible_role(gtk::AccessibleRole::Dialog)
            .build();
        window.set_title(Some(&spec.page.title));
        window.set_default_size(520, -1);
        let setting = Rc::new(Cell::new(false));
        let outer = margined_box();
        let page = gtk::Box::new(gtk::Orientation::Vertical, 8);
        outer.append(&page);
        let Built {
            controls,
            labels,
            first,
        } = build_controls(ui, id, &spec.page.controls, &page, &window, &setting);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_halign(gtk::Align::End);
        let back = gtk::Button::with_mnemonic(&mnemonic(&spec.labels.back));
        let next = gtk::Button::with_mnemonic(&mnemonic(&spec.labels.next));
        let cancel = gtk::Button::with_mnemonic(&mnemonic(&spec.labels.cancel));
        for button in [&back, &next, &cancel] {
            row.append(button);
        }
        outer.append(&row);
        let buttons = Rc::new(Cell::new(spec.buttons));
        back.connect_clicked(move |_| {
            send(&FromUi::DialogPressed {
                id,
                control: "back".to_owned(),
            });
        });
        {
            let buttons = Rc::clone(&buttons);
            next.connect_clicked(move |_| {
                let control = if buttons.get().next { "next" } else { "finish" };
                send(&FromUi::DialogPressed {
                    id,
                    control: control.to_owned(),
                });
            });
        }
        {
            let ui = Rc::clone(ui);
            cancel.connect_clicked(move |_| ui.cancelled(id));
        }
        window.set_child(Some(&outer));
        Ui::closing(ui, &window, id);
        ui.dialogs.borrow_mut().insert(
            id,
            ShownDialog {
                window: window.clone(),
                controls,
                labels,
                setting,
            },
        );
        let shown = ShownWizard {
            page,
            back,
            next,
            buttons,
            labels: spec.labels,
        };
        set_wizard_buttons(&shown, &window, spec.buttons);
        ui.wizards.borrow_mut().insert(id, shown);
        window.present();
        let ui = Rc::clone(ui);
        once_active(&window, move || ui.focus_first(id, first));
    }

    /// Turns wizard `id` to `page`, with `buttons`, the focus on its first control: the page
    /// before it goes, its controls no longer Okur's.
    fn turn_wizard(ui: &Rc<Ui>, id: u32, page: &DialogSpec, buttons: WizardButtons) {
        let Some((window, setting)) = ui
            .dialogs
            .borrow()
            .get(&id)
            .map(|dialog| (dialog.window.clone(), Rc::clone(&dialog.setting)))
        else {
            return;
        };
        let Some(place) = ui.wizards.borrow().get(&id).map(|wizard| {
            set_wizard_buttons(wizard, &window, buttons);
            wizard.page.clone()
        }) else {
            return;
        };
        while let Some(child) = place.first_child() {
            place.remove(&child);
        }
        let built = build_controls(ui, id, &page.controls, &place, &window, &setting);
        window.set_title(Some(&page.title));
        if let Some(dialog) = ui.dialogs.borrow_mut().get_mut(&id) {
            dialog.controls = built.controls;
            dialog.labels = built.labels;
        }
        ui.focus_first(id, built.first);
    }

    /// The dialog cancelled by the user: Okur told, the window gone.
    fn cancelled(&self, id: u32) {
        self.wizards.borrow_mut().remove(&id);
        let dialog = self.dialogs.borrow_mut().remove(&id);
        if let Some(dialog) = dialog {
            send(&FromUi::DialogClosed { id });
            dialog.window.destroy();
        }
    }

    /// Sets a control's value, as Okur asks: not told back as the user's change.
    fn set(&self, id: u32, control: &str, value: &Value) {
        let dialogs = self.dialogs.borrow();
        let Some(dialog) = dialogs.get(&id) else {
            return;
        };
        let Some(shown) = dialog.controls.get(control) else {
            return;
        };
        dialog.setting.set(true);
        match (shown, value) {
            (Shown::Check(button), Value::Checked(checked)) => button.set_active(*checked),
            (Shown::Choice(choice), Value::Selected(i)) => {
                choice.set_selected(u32::try_from(*i).unwrap_or(u32::MAX));
            }
            (Shown::List(list), Value::Selected(i)) => list.select(*i, false),
            (Shown::Tree(tree), Value::Selected(i)) => tree.select(*i, false),
            // An item's text and check box changed in place: the tree filled again as it is.
            (Shown::Tree(tree), Value::TreeItem(i, item)) => {
                let mut items = tree.items.borrow().clone();
                if let Some(slot) = items.get_mut(*i) {
                    *slot = item.clone();
                }
                let chosen = tree
                    .chosen()
                    .and_then(|row| ShownTree::index_of(&row))
                    .unwrap_or(*i);
                tree.fill(&items, chosen);
            }
            (Shown::Radios(buttons), Value::Selected(i)) => {
                if let Some(button) = buttons.get(*i) {
                    button.set_active(true);
                }
            }
            (Shown::Entry(entry), Value::Text(text)) => entry.set_text(text),
            (Shown::Scale(scale), Value::Number(n)) => scale.set_value(f64::from(*n)),
            (Shown::Text(label), Value::Text(text)) => label.set_text(text),
            (Shown::Note(view), Value::Text(text)) => view.buffer().set_text(text),
            _ => {}
        }
        dialog.setting.set(false);
    }

    /// Replaces a table's rows and selects one (Manage Key Labels after a label changed).
    fn rows(&self, id: u32, control: &str, rows: &[Vec<String>], selected: usize) {
        let dialogs = self.dialogs.borrow();
        let Some(dialog) = dialogs.get(&id) else {
            return;
        };
        dialog.setting.set(true);
        if let Some(Shown::List(list)) = dialog.controls.get(control) {
            list.fill_rows(rows);
            list.select(selected, false);
        }
        dialog.setting.set(false);
    }

    /// Replaces a list's items, or a drop-down's choices, and selects one.
    fn items(&self, id: u32, control: &str, items: &[String], selected: usize) {
        let dialogs = self.dialogs.borrow();
        let Some(dialog) = dialogs.get(&id) else {
            return;
        };
        dialog.setting.set(true);
        match dialog.controls.get(control) {
            Some(Shown::List(list)) => {
                list.fill(items);
                list.select(selected, false);
            }
            Some(Shown::Choice(choice)) => {
                let texts: Vec<&str> = items.iter().map(String::as_str).collect();
                choice.set_model(Some(&gtk::StringList::new(&texts)));
                choice.set_selected(u32::try_from(selected).unwrap_or(u32::MAX));
            }
            _ => {}
        }
        dialog.setting.set(false);
    }
}

/// The message's kind, for the line saying it is not drawn yet.
fn kind(message: &ToUi) -> String {
    encode(message)
        .ok()
        .and_then(|line| {
            let value: serde_json::Value = serde_json::from_str(&line).ok()?;
            value.get("type")?.as_str().map(str::to_owned)
        })
        .unwrap_or_default()
}

/// What runs once, kept until it does.
type Once = Rc<Cell<Option<Box<dyn FnOnce()>>>>;

/// Runs `then` once `window` is active (the window manager gives it the keyboard after
/// `present`, in its own time) and a frame of it is laid out (the next frame's tick, then an
/// idle, which comes after the frame's layout and drawing). A dialog's first focus is placed
/// then: GTK reports no focus for a widget focused before its window is active, and the list's
/// first row went unsaid (gtk4's 223, 586 and 682 at 298f0c7c: the first focus event was the
/// second row's, after Down). And it says the dialog is shown after it: GTK put a list's focus
/// on its first row at the layout after a letter Okur held had chosen another (the lab's
/// research run of 1205).
fn once_active(window: &gtk::Window, then: impl FnOnce() + 'static) {
    let then: Once = Rc::new(Cell::new(Some(Box::new(then))));
    let go = move |window: &gtk::Window| {
        let Some(then) = then.take() else {
            return;
        };
        let then = Cell::new(Some(then));
        window.add_tick_callback(move |_, _| {
            if let Some(then) = then.take() {
                let _ = glib::idle_add_local_once(then);
            }
            glib::ControlFlow::Break
        });
    };
    if window.is_active() {
        go(window);
        return;
    }
    // Once: the handler goes with it, a dialog brought forward again adding its own.
    let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
    let id_kept = Rc::clone(&handler);
    let connected = window.connect_is_active_notify(move |window| {
        if window.is_active() {
            go(window);
            if let Some(connected) = id_kept.borrow_mut().take() {
                window.disconnect(connected);
            }
        }
    });
    *handler.borrow_mut() = Some(connected);
}

/// Dialog `id`'s first focus on `first`, then Okur told the dialog is shown with the keyboard
/// in it, once its window is active and laid out ([`once_active`]): the keys Okur held since it
/// asked for the dialog go to it then, not to the window before it.
fn focus_and_tell_shown(ui: &Rc<Ui>, window: &gtk::Window, id: u32, first: Option<gtk::Widget>) {
    let ui = Rc::clone(ui);
    once_active(window, move || {
        ui.focus_first(id, first);
        // After the first focus, which a list places in an idle of its own.
        let _ = glib::idle_add_local_once(move || send(&FromUi::DialogShown { id }));
    });
}

/// Presses the dialog's default button, as Enter on a list's item does in Windows' dialog: where
/// the button is available, at once (a button's activate waits for its key to be let go). The
/// window is held weakly: the list's handler holding it kept every closed dialog alive.
fn press_default(window: &glib::WeakRef<gtk::Window>) {
    let Some(window) = window.upgrade() else {
        return;
    };
    let Some(button) = window.default_widget().and_downcast::<gtk::Button>() else {
        return;
    };
    if button.is_sensitive() {
        button.emit_clicked();
    }
}

/// A list of strings' rows: each a label of its item.
fn text_rows() -> gtk::SignalListItemFactory {
    text_cells(0)
}

/// A table's column `column`: each row's label of its cell there (a list's one column is its
/// whole item).
fn text_cells(column: usize) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        item.set_child(Some(&label));
    });
    factory.connect_bind(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(label) = item.child().and_downcast::<gtk::Label>() else {
            return;
        };
        let Some(text) = item.item().and_downcast::<gtk::StringObject>() else {
            return;
        };
        let row = text.string();
        label.set_label(row.split(CELLS).nth(column).unwrap_or_default());
    });
    factory
}

/// A tree's rows: each its expander and its item's text (with its check box's state, for an item
/// that is one). A row's accessible name is the item's text.
fn tree_rows(items: &Rc<RefCell<Vec<TreeItem>>>) -> gtk::SignalListItemFactory {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let expander = gtk::TreeExpander::new();
        let label = gtk::Label::new(None);
        label.set_xalign(0.0);
        expander.set_child(Some(&label));
        item.set_child(Some(&expander));
    });
    let items = Rc::clone(items);
    factory.connect_bind(move |_, object| {
        let Some(item) = object.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let Some(expander) = item.child().and_downcast::<gtk::TreeExpander>() else {
            return;
        };
        let Some(row) = item.item().and_downcast::<gtk::TreeListRow>() else {
            return;
        };
        let text = ShownTree::index_of(&row)
            .and_then(|i| {
                items
                    .borrow()
                    .get(i)
                    .map(|tree_item| tree_item.text.clone())
            })
            .unwrap_or_default();
        if let Some(label) = expander.child().and_downcast::<gtk::Label>() {
            label.set_label(&text);
        }
        expander.set_list_row(Some(&row));
        item.set_accessible_label(&text);
    });
    factory
}

/// A label shown before a field, its access key moving to the field.
fn field_label(text: &str, field: &impl IsA<gtk::Widget>) -> gtk::Label {
    let label = gtk::Label::with_mnemonic(&mnemonic(text));
    label.set_xalign(0.0);
    label.set_mnemonic_widget(Some(field));
    label
}

/// The value a control's change tells Okur, unless Okur set it.
fn changed(id: u32, control: &str, setting: &Rc<Cell<bool>>, value: Value) {
    if !setting.get() {
        send(&FromUi::DialogChanged {
            id,
            control: control.to_owned(),
            value,
        });
    }
}

/// A vertical box with a dialog's margins.
fn margined_box() -> gtk::Box {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 8);
    outer.set_margin_top(12);
    outer.set_margin_bottom(12);
    outer.set_margin_start(12);
    outer.set_margin_end(12);
    outer
}

/// `controls` built into `outer`, in the spec's order, which is the Tab order. The buttons in
/// a row where they come, consecutive ones together (a list, its buttons, then its radio
/// groups: "Move To Link Button" after the list [obs, 222/s2]; one row of them at the end had
/// Tab reach the radios first); a group box holds the controls after it, up to the next group
/// or the buttons.
fn build_controls(
    ui: &Rc<Ui>,
    id: u32,
    controls: &[Control],
    outer: &gtk::Box,
    window: &gtk::Window,
    setting: &Rc<Cell<bool>>,
) -> Built {
    let mut button_row: Option<gtk::Box> = None;
    let mut built = Built {
        controls: HashMap::new(),
        labels: HashMap::new(),
        first: None,
    };
    let mut place: gtk::Box = outer.clone();
    for control in controls {
        if control.kind == ControlKind::Group {
            let frame = gtk::Frame::new(Some(&plain(&control.label)));
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 6);
            inner.set_margin_top(6);
            inner.set_margin_bottom(6);
            inner.set_margin_start(6);
            inner.set_margin_end(6);
            frame.set_child(Some(&inner));
            outer.append(&frame);
            place = inner;
            button_row = None;
            continue;
        }
        let target = if matches!(control.kind, ControlKind::Button { .. }) {
            // A group box ends at its buttons.
            place = outer.clone();
            button_row
                .get_or_insert_with(|| {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    row.set_halign(gtk::Align::End);
                    outer.append(&row);
                    row
                })
                .clone()
        } else {
            button_row = None;
            place.clone()
        };
        let (shown, label) = build(ui, id, control, &target, window, setting);
        if let Some(widget) = shown.widget() {
            widget.set_sensitive(control.enabled);
            widget.set_visible(!control.hidden);
            if built.first.is_none()
                && control.enabled
                && !control.hidden
                && !matches!(
                    control.kind,
                    ControlKind::Text { .. } | ControlKind::Message { .. }
                )
            {
                built.first = Some(widget);
            }
        }
        if let Some(label) = label {
            built.labels.insert(control.id.clone(), label);
        }
        built.controls.insert(control.id.clone(), shown);
    }
    built
}

/// A wizard's buttons as `buttons` has them: Back where it can go back; Next, or Finish in its
/// place without it, available where it can finish (JAWS's Save Temporary Customizations, until
/// the rule has a name [obs, 740]); Enter presses it.
fn set_wizard_buttons(wizard: &ShownWizard, window: &gtk::Window, buttons: WizardButtons) {
    wizard.buttons.set(buttons);
    wizard.back.set_sensitive(buttons.back);
    if buttons.next {
        wizard.next.set_label(&mnemonic(&wizard.labels.next));
        wizard.next.set_sensitive(true);
    } else {
        wizard.next.set_label(&mnemonic(&wizard.labels.finish));
        wizard.next.set_sensitive(buttons.finish);
    }
    wizard.next.set_use_underline(true);
    window.set_default_widget(Some(&wizard.next));
}

/// A control drawn into `place`, with the label shown for it.
fn build(
    _ui: &Rc<Ui>,
    id: u32,
    control: &Control,
    place: &gtk::Box,
    window: &gtk::Window,
    setting: &Rc<Cell<bool>>,
) -> (Shown, Option<gtk::Label>) {
    let name = control.id.clone();
    match &control.kind {
        ControlKind::CheckBox { checked } => {
            let button = gtk::CheckButton::with_mnemonic(&mnemonic(&control.label));
            button.set_active(*checked);
            let setting = Rc::clone(setting);
            button.connect_toggled(move |b| {
                changed(id, &name, &setting, Value::Checked(b.is_active()));
            });
            place.append(&button);
            (Shown::Check(button), None)
        }
        ControlKind::Choice { options, selected } => {
            let texts: Vec<&str> = options.iter().map(String::as_str).collect();
            let choice = gtk::DropDown::from_strings(&texts);
            choice.set_selected(u32::try_from(*selected).unwrap_or(u32::MAX));
            let label = field_label(&control.label, &choice);
            let setting = Rc::clone(setting);
            choice.connect_selected_notify(move |d| {
                let i = usize::try_from(d.selected()).unwrap_or(usize::MAX);
                changed(id, &name, &setting, Value::Selected(i));
            });
            place.append(&label);
            place.append(&choice);
            (Shown::Choice(choice), Some(label))
        }
        ControlKind::ComboEdit { text, .. }
        | ControlKind::Edit { text }
        | ControlKind::SearchBox { text } => {
            let entry = gtk::Entry::new();
            entry.set_text(text);
            // Enter in it presses the dialog's default button, as in Windows' dialog (Find,
            // Jump To Line), at once: GTK's own default activation clicks it only on a key's
            // release, which goes to the entry, or 250 ms later (the code review of 2026-11-09,
            // fifth pass, 5).
            {
                let window = window.downgrade();
                entry.connect_activate(move |_| press_default(&window));
            }
            let label = field_label(&control.label, &entry);
            // A search box's label is not shown: it names the field alone ("SearchBox Edit").
            let shown_label = !matches!(control.kind, ControlKind::SearchBox { .. });
            if !shown_label {
                entry.update_property(&[gtk::accessible::Property::Label(&plain(&control.label))]);
            }
            let setting = Rc::clone(setting);
            entry.connect_changed(move |e| {
                changed(id, &name, &setting, Value::Text(e.text().to_string()));
            });
            if shown_label {
                place.append(&label);
            }
            place.append(&entry);
            (Shown::Entry(entry), shown_label.then_some(label))
        }
        ControlKind::Slider { min, max, value } => {
            let scale = gtk::Scale::with_range(
                gtk::Orientation::Horizontal,
                f64::from(*min),
                f64::from(*max),
                1.0,
            );
            scale.set_value(f64::from(*value));
            let label = field_label(&control.label, &scale);
            let setting = Rc::clone(setting);
            scale.connect_value_changed(move |s| {
                // A slider's value is a whole number from its min to its max.
                let n = s.value().round();
                let n = if n.is_finite() { n as i32 } else { 0 };
                changed(id, &name, &setting, Value::Number(n));
            });
            place.append(&label);
            place.append(&scale);
            (Shown::Scale(scale), Some(label))
        }
        ControlKind::Button { default } => {
            let button = gtk::Button::with_mnemonic(&mnemonic(&control.label));
            if *default {
                window.set_default_widget(Some(&button));
            }
            button.connect_clicked(move |_| {
                send(&FromUi::DialogPressed {
                    id,
                    control: name.clone(),
                });
            });
            place.append(&button);
            (Shown::Button(button), None)
        }
        ControlKind::List { items, selected } | ControlKind::ListBox { items, selected } => {
            let model = gtk::StringList::new(&[]);
            let selection = gtk::SingleSelection::new(Some(model.clone()));
            selection.set_autoselect(true);
            selection.set_can_unselect(false);
            let list_view = matches!(control.kind, ControlKind::List { .. });
            let title = plain(&control.label);
            let view = if list_view {
                let columns = gtk::ColumnView::new(Some(selection.clone()));
                let column = gtk::ColumnViewColumn::new(Some(&title), Some(text_rows()));
                column.set_expand(true);
                columns.append_column(&column);
                columns.set_tab_behavior(gtk::ListTabBehavior::Item);
                ListWidget::Columns(columns)
            } else {
                let rows = gtk::ListView::new(Some(selection.clone()), Some(text_rows()));
                rows.set_tab_behavior(gtk::ListTabBehavior::Item);
                ListWidget::Rows(rows)
            };
            let list = ShownList {
                view,
                items: model,
                selection: selection.clone(),
            };
            list.fill(items);
            list.select(*selected, false);
            list.search_by_letter();
            let widget = list.widget();
            // A list's label names it; a list box's is shown above it ("Running Applications
            // List box").
            let label = (!list_view).then(|| field_label(&control.label, &widget));
            widget.update_property(&[gtk::accessible::Property::Label(&title)]);
            {
                let setting = Rc::clone(setting);
                selection.connect_selected_notify(move |selection| {
                    let at = selection.selected();
                    if at == gtk::INVALID_LIST_POSITION {
                        return;
                    }
                    if let Ok(i) = usize::try_from(at) {
                        changed(id, &name, &setting, Value::Selected(i));
                    }
                });
            }
            // Enter on an item presses the dialog's default button.
            let window = window.downgrade();
            match &list.view {
                ListWidget::Columns(view) => {
                    view.connect_activate(move |_, _| press_default(&window));
                }
                ListWidget::Rows(view) => {
                    view.connect_activate(move |_, _| press_default(&window));
                }
            }
            let scrolled = gtk::ScrolledWindow::new();
            scrolled.set_min_content_height(200);
            scrolled.set_child(Some(&widget));
            if let Some(label) = &label {
                place.append(label);
            }
            place.append(&scrolled);
            (Shown::List(list), label)
        }
        ControlKind::Tree { items, selected } => {
            let roots = gtk::StringList::new(&[]);
            let tree_items: Rc<RefCell<Vec<TreeItem>>> = Rc::default();
            let model = {
                let tree_items = Rc::clone(&tree_items);
                gtk::TreeListModel::new(roots.clone(), false, false, move |object| {
                    let at: usize = object
                        .downcast_ref::<gtk::StringObject>()?
                        .string()
                        .parse()
                        .ok()?;
                    let children = ShownTree::children(&tree_items.borrow(), at);
                    if children.is_empty() {
                        return None;
                    }
                    let texts: Vec<String> = children.iter().map(usize::to_string).collect();
                    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
                    Some(gtk::StringList::new(&texts).upcast())
                })
            };
            let selection = gtk::SingleSelection::new(Some(model.clone()));
            selection.set_autoselect(true);
            selection.set_can_unselect(false);
            let view = gtk::ListView::new(Some(selection.clone()), Some(tree_rows(&tree_items)));
            view.set_tab_behavior(gtk::ListTabBehavior::Item);
            let tree = ShownTree {
                view: view.downgrade(),
                items: tree_items,
                roots,
                model,
                selection: selection.clone(),
                filling: Rc::new(Cell::new(false)),
            };
            tree.fill(items, *selected);
            let title = plain(&control.label);
            view.update_property(&[gtk::accessible::Property::Label(&title)]);
            {
                let (setting, filling, name) =
                    (Rc::clone(setting), Rc::clone(&tree.filling), name.clone());
                selection.connect_selected_notify(move |selection| {
                    if filling.get() {
                        return;
                    }
                    let index = selection
                        .selected_item()
                        .and_downcast::<gtk::TreeListRow>()
                        .and_then(|row| ShownTree::index_of(&row));
                    if let Some(i) = index {
                        changed(id, &name, &setting, Value::Selected(i));
                    }
                });
            }
            // An item opened or closed: its rows come or go after it.
            {
                // The model's own handler holds no model: it is given it.
                let (setting, name) = (Rc::clone(setting), name.clone());
                let (filling, items) = (Rc::clone(&tree.filling), Rc::clone(&tree.items));
                tree.model
                    .connect_items_changed(move |model, at, removed, added| {
                        if filling.get() || at == 0 || (removed == 0 && added == 0) {
                            return;
                        }
                        let Some(row) = model.row(at - 1) else {
                            return;
                        };
                        let Some(i) = ShownTree::index_of(&row) else {
                            return;
                        };
                        let open = row.is_expanded();
                        if let Some(item) = items.borrow_mut().get_mut(i) {
                            item.open = open;
                        }
                        changed(id, &name, &setting, Value::ItemOpened(i, open));
                    });
            }
            // Right and Left as a Windows tree view's; Space changes the item.
            let keys = gtk::EventControllerKey::new();
            {
                let tree = tree.clone();
                let name = name.clone();
                keys.connect_key_pressed(move |_, key, _, modifiers| {
                    let Some(row) = tree.chosen() else {
                        return glib::Propagation::Proceed;
                    };
                    match key {
                        gtk::gdk::Key::Right if row.is_expandable() && !row.is_expanded() => {
                            row.set_expanded(true);
                        }
                        gtk::gdk::Key::Right if row.is_expanded() => {
                            let first = row
                                .child_row(0)
                                .and_then(|child| ShownTree::index_of(&child));
                            if let Some(first) = first {
                                tree.select(first, true);
                            }
                        }
                        gtk::gdk::Key::Left if row.is_expanded() => row.set_expanded(false),
                        gtk::gdk::Key::Left => {
                            let parent =
                                row.parent().and_then(|parent| ShownTree::index_of(&parent));
                            if let Some(parent) = parent {
                                tree.select(parent, true);
                            }
                        }
                        gtk::gdk::Key::space => {
                            let back = modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK);
                            send(&FromUi::DialogKey {
                                id,
                                control: name.clone(),
                                key: if back {
                                    DialogKey::ChangeBack
                                } else {
                                    DialogKey::Change
                                },
                            });
                        }
                        _ => return glib::Propagation::Proceed,
                    }
                    glib::Propagation::Stop
                });
            }
            view.add_controller(keys);
            // Enter on an item presses the dialog's default button.
            {
                let window = window.downgrade();
                view.connect_activate(move |_, _| press_default(&window));
            }
            let label = field_label(&control.label, &view);
            let scrolled = gtk::ScrolledWindow::new();
            scrolled.set_min_content_height(200);
            scrolled.set_child(Some(&view));
            place.append(&label);
            place.append(&scrolled);
            (Shown::Tree(tree), Some(label))
        }
        // A table: a column view of its columns, none selected until the user picks a row.
        ControlKind::Table {
            columns,
            rows,
            selected,
            ..
        } => {
            let model = gtk::StringList::new(&[]);
            let selection = gtk::SingleSelection::new(Some(model.clone()));
            selection.set_autoselect(false);
            selection.set_can_unselect(true);
            let view = gtk::ColumnView::new(Some(selection.clone()));
            for (n, title) in columns.iter().enumerate() {
                let column = gtk::ColumnViewColumn::new(Some(&plain(title)), Some(text_cells(n)));
                column.set_expand(true);
                view.append_column(&column);
            }
            view.set_tab_behavior(gtk::ListTabBehavior::Item);
            let list = ShownList {
                view: ListWidget::Columns(view),
                items: model,
                selection: selection.clone(),
            };
            list.fill_rows(rows);
            match selected {
                Some(i) => list.select(*i, false),
                None => selection.set_selected(gtk::INVALID_LIST_POSITION),
            }
            list.search_by_letter();
            let widget = list.widget();
            widget.update_property(&[gtk::accessible::Property::Label(&plain(&control.label))]);
            {
                let setting = Rc::clone(setting);
                selection.connect_selected_notify(move |selection| {
                    let at = selection.selected();
                    if at == gtk::INVALID_LIST_POSITION {
                        return;
                    }
                    if let Ok(i) = usize::try_from(at) {
                        changed(id, &name, &setting, Value::Selected(i));
                    }
                });
            }
            let window = window.downgrade();
            if let ListWidget::Columns(view) = &list.view {
                view.connect_activate(move |_, _| press_default(&window));
            }
            let scrolled = gtk::ScrolledWindow::new();
            scrolled.set_min_content_height(200);
            scrolled.set_child(Some(&widget));
            place.append(&scrolled);
            (Shown::List(list), None)
        }
        ControlKind::Radios { options, selected }
        | ControlKind::RadioButtons {
            options, selected, ..
        } => {
            let enabled = match &control.kind {
                ControlKind::RadioButtons { enabled, .. } => enabled.clone(),
                _ => Vec::new(),
            };
            // Radios are in a group box the label names; radio buttons stand alone.
            let (holder, frame): (gtk::Box, Option<gtk::Frame>) = match &control.kind {
                ControlKind::Radios { .. } => {
                    let frame = gtk::Frame::new(Some(&plain(&control.label)));
                    let inner = gtk::Box::new(gtk::Orientation::Vertical, 4);
                    frame.set_child(Some(&inner));
                    (inner, Some(frame))
                }
                _ => (place.clone(), None),
            };
            let mut buttons: Vec<gtk::CheckButton> = Vec::new();
            for (i, option) in options.iter().enumerate() {
                let button = gtk::CheckButton::with_mnemonic(&mnemonic(option));
                if let Some(first) = buttons.first() {
                    button.set_group(Some(first));
                }
                button.set_active(i == *selected);
                if let Some(on) = enabled.get(i) {
                    button.set_sensitive(*on);
                }
                let setting = Rc::clone(setting);
                let name = name.clone();
                button.connect_toggled(move |b| {
                    if b.is_active() {
                        changed(id, &name, &setting, Value::Selected(i));
                    }
                });
                holder.append(&button);
                buttons.push(button);
            }
            if let Some(frame) = frame {
                place.append(&frame);
            }
            (Shown::Radios(buttons), None)
        }
        ControlKind::Text { text } | ControlKind::Message { text } => {
            let label = gtk::Label::new(Some(text));
            label.set_xalign(0.0);
            label.set_wrap(true);
            label.set_selectable(false);
            place.append(&label);
            (Shown::Text(label), None)
        }
        ControlKind::Note { text } => {
            let view = gtk::TextView::new();
            view.set_editable(false);
            view.set_wrap_mode(gtk::WrapMode::Word);
            view.buffer().set_text(text);
            let label = field_label(&control.label, &view);
            place.append(&label);
            place.append(&view);
            (Shown::Note(view), Some(label))
        }
        // Not drawn yet (a tree, a table, a check list): the dialog goes on without it.
        other => {
            eprintln!("okur-ui: control not drawn on Linux yet: {other:?}");
            let empty = gtk::Box::new(gtk::Orientation::Vertical, 0);
            place.append(&empty);
            (Shown::Other(empty.upcast()), None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Okur's access keys ("&") are GTK's ("_"); a doubled "&" is one, a "_" is doubled.
    #[test]
    fn access_keys_are_marked_as_gtk_marks_them() {
        assert_eq!(mnemonic("&Move To"), "_Move To");
        assert_eq!(mnemonic("Tom && Jerry"), "Tom & Jerry");
        assert_eq!(mnemonic("snake_case"), "snake__case");
        assert_eq!(plain("&Move To"), "Move To");
        assert_eq!(plain("snake_case"), "snake_case");
    }
}
