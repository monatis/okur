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
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;
use okur_ui_model::{decode, encode, Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

/// Draws what Okur sends until its input ends; without a display, reads and drops it, so that
/// Okur never waits on a full pipe.
pub fn run() {
    if let Err(e) = gtk::init() {
        eprintln!("okur-ui: no display for GTK: {e}");
        for _ in std::io::stdin().lock().lines() {}
        return;
    }
    let (messages, received) = std::sync::mpsc::channel::<Option<ToUi>>();
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
                    }
                    Err(e) => eprintln!("okur-ui: unreadable message {line:?}: {e}"),
                }
            }
            let _ = messages.send(None);
        });
    if reader.is_err() {
        eprintln!("okur-ui: no thread to read Okur's messages");
        return;
    }
    let main_loop = glib::MainLoop::new(None, false);
    let ui = Rc::new(Ui::default());
    let quit = main_loop.clone();
    // Okur's messages, taken on GTK's thread.
    glib::timeout_add_local(Duration::from_millis(10), move || {
        for message in received.try_iter() {
            match message {
                Some(message) => Ui::handle(&ui, message),
                None => {
                    quit.quit();
                    return glib::ControlFlow::Break;
                }
            }
        }
        glib::ControlFlow::Continue
    });
    main_loop.run();
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
    List(gtk::ListBox),
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
            Shown::List(w) => w.clone().upcast(),
            Shown::Radios(buttons) => buttons.first()?.clone().upcast(),
            Shown::Text(w) => w.clone().upcast(),
            Shown::Note(w) => w.clone().upcast(),
            Shown::Other(w) => w.clone(),
        })
    }
}

/// A shown dialog: its window, its controls by their ids, the labels that name them, and
/// whether Okur is setting a value (not the user, so not told back).
struct ShownDialog {
    window: gtk::Window,
    controls: HashMap<String, Shown>,
    labels: HashMap<String, gtk::Label>,
    setting: Rc<Cell<bool>>,
}

/// The dialogs shown, by Okur's numbers.
#[derive(Default)]
struct Ui {
    dialogs: RefCell<HashMap<u32, ShownDialog>>,
}

impl Ui {
    fn handle(ui: &Rc<Ui>, message: ToUi) {
        match message {
            ToUi::DialogShow { dialog } => Ui::show(ui, dialog),
            ToUi::DialogClose { id } => {
                if let Some(dialog) = ui.dialogs.borrow_mut().remove(&id) {
                    dialog.window.destroy();
                }
            }
            ToUi::DialogSet { id, control, value } => ui.set(id, &control, &value),
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
            return;
        }
        let id = spec.id;
        let window = gtk::Window::new();
        window.set_title(Some(&spec.title));
        window.set_default_size(480, -1);
        let setting = Rc::new(Cell::new(false));
        let outer = gtk::Box::new(gtk::Orientation::Vertical, 8);
        outer.set_margin_top(12);
        outer.set_margin_bottom(12);
        outer.set_margin_start(12);
        outer.set_margin_end(12);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_halign(gtk::Align::End);
        let mut controls = HashMap::new();
        let mut labels = HashMap::new();
        // A group box holds the controls after it, up to the next group or the buttons.
        let mut place: gtk::Box = outer.clone();
        let mut first: Option<gtk::Widget> = None;
        for control in &spec.controls {
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
                continue;
            }
            let target = if matches!(control.kind, ControlKind::Button { .. }) {
                place = outer.clone();
                &buttons
            } else {
                &place
            };
            let (shown, label) = build(ui, id, control, target, &window, &setting);
            if let Some(widget) = shown.widget() {
                widget.set_sensitive(control.enabled);
                widget.set_visible(!control.hidden);
                if first.is_none()
                    && control.enabled
                    && !control.hidden
                    && !matches!(
                        control.kind,
                        ControlKind::Text { .. } | ControlKind::Message { .. }
                    )
                {
                    first = Some(widget);
                }
            }
            if let Some(label) = label {
                labels.insert(control.id.clone(), label);
            }
            controls.insert(control.id.clone(), shown);
        }
        outer.append(&buttons);
        window.set_child(Some(&outer));
        // Escape and the close box are the dialog's Cancel: Okur is told it was cancelled, and
        // it goes.
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
        {
            let ui = Rc::clone(ui);
            window.connect_close_request(move |_| {
                ui.cancelled(id);
                glib::Propagation::Stop
            });
        }
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
        if let Some(first) = first {
            first.grab_focus();
        }
    }

    /// The dialog cancelled by the user: Okur told, the window gone.
    fn cancelled(&self, id: u32) {
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
            (Shown::List(list), Value::Selected(i)) => select_row(list, *i),
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

    /// Replaces a list's items, or a drop-down's choices, and selects one.
    fn items(&self, id: u32, control: &str, items: &[String], selected: usize) {
        let dialogs = self.dialogs.borrow();
        let Some(dialog) = dialogs.get(&id) else {
            return;
        };
        dialog.setting.set(true);
        match dialog.controls.get(control) {
            Some(Shown::List(list)) => {
                fill_list(list, items);
                select_row(list, selected);
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

/// A list's items in place of those it had.
fn fill_list(list: &gtk::ListBox, items: &[String]) {
    while let Some(row) = list.first_child() {
        list.remove(&row);
    }
    for item in items {
        let label = gtk::Label::new(Some(item));
        label.set_xalign(0.0);
        list.append(&label);
    }
}

/// Selects a list's row `i`, scrolled to and focused where the list has the focus.
fn select_row(list: &gtk::ListBox, i: usize) {
    if let Some(row) = i32::try_from(i).ok().and_then(|i| list.row_at_index(i)) {
        list.select_row(Some(&row));
        if list.focus_child().is_some() {
            row.grab_focus();
        }
    }
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
            let list = gtk::ListBox::new();
            list.set_selection_mode(gtk::SelectionMode::Single);
            fill_list(&list, items);
            select_row(&list, *selected);
            // A list's label names it; a list box's is shown above it ("Running Applications
            // List box").
            let label = matches!(control.kind, ControlKind::ListBox { .. })
                .then(|| field_label(&control.label, &list));
            list.update_property(&[gtk::accessible::Property::Label(&plain(&control.label))]);
            let setting = Rc::clone(setting);
            list.connect_row_selected(move |_, row| {
                if let Some(i) = row.and_then(|r| usize::try_from(r.index()).ok()) {
                    changed(id, &name, &setting, Value::Selected(i));
                }
            });
            // Enter on an item presses the dialog's default button.
            {
                let window = window.clone();
                list.connect_row_activated(move |_, _| {
                    if let Some(default) = window.default_widget() {
                        default.activate();
                    }
                });
            }
            let scrolled = gtk::ScrolledWindow::new();
            scrolled.set_min_content_height(200);
            scrolled.set_child(Some(&list));
            if let Some(label) = &label {
                place.append(label);
            }
            place.append(&scrolled);
            (Shown::List(list), label)
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
