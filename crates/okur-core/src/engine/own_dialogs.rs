//! Okur's own dialogs as the engine asked okur-ui to show them: what the platform may not say
//! of them, told from the dialog's spec where it does not. Windows' dialogs say it all; GTK 4's
//! do not:
//! - no button is the default (the Linux lab's 222: no button had AT-SPI's IS_DEFAULT), so a
//!   focus in Okur's Links List was in no dialog box, and Insert+E said "Not in a dialog box";
//! - a group box names no control in it: JAWS's "Use Rule Type: Read First Sentence of Every
//!   Paragraph radio button checked" was "Read First Sentence..." alone (812/s2 in full5);
//! - a drop-down gives no place: "Rules History: Combo box <New> 1 of 8" lost its "1 of 8"
//!   (812/s5);
//! - a tree's row is its expander's button, with its place in the rows shown and a level that
//!   is not its depth: JAWS's "level 1 | Primary Navigation | 1 of 1 | Banner" was "Primary
//!   Navigation 2 of 5" (241/s2 in gtk7).
//!
//! A focus in one of Okur's windows whose title is a dialog it showed is in a dialog box, its
//! default button the spec's default; its control, found in the spec by its label, has its
//! group box and a drop-down its choice's place, each where the platform says none.

use std::sync::{Arc, Mutex};

use super::held_keys::AskedDialog;
use super::{ScreenReaderEngine, OKUR_WINDOWS};
use crate::aom::Role;
use crate::event::FocusEvent;
use crate::ui::{
    Control, ControlKind, DialogSpec, FromUi, ToUi, TreeItem, UiHost, Value, WizardButtons,
    WizardLabels,
};

/// The dialogs okur-ui shows, as the engine last told it of them.
pub(super) type OpenDialogs = Arc<Mutex<Vec<DialogSpec>>>;

/// Okur's windows as the engine gives them messages: each dialog asked to show noted for the
/// keys held for it ([`super::held_keys`]), and the dialogs kept as they are shown.
pub(super) struct DialogWatch {
    pub(super) inner: Box<dyn UiHost>,
    pub(super) asked: Arc<Mutex<Option<AskedDialog>>>,
    pub(super) open: OpenDialogs,
}

impl UiHost for DialogWatch {
    fn send(&self, message: ToUi) {
        if let ToUi::DialogShow { dialog } = &message {
            if let Ok(mut asked) = self.asked.lock() {
                *asked = Some(AskedDialog { id: dialog.id });
            }
        }
        if let Ok(mut open) = self.open.lock() {
            kept(&mut open, &message);
        }
        self.inner.send(message);
    }
}

/// `open` with what `message` changes in it: a dialog shown, closed, its default or a label.
fn kept(open: &mut Vec<DialogSpec>, message: &ToUi) {
    match message {
        ToUi::DialogShow { dialog } => {
            open.retain(|d| d.id != dialog.id);
            open.push(dialog.clone());
        }
        ToUi::DialogClose { id } => open.retain(|d| d.id != *id),
        // A wizard, kept as a dialog of its page's controls and its Next and Finish, the one
        // Enter presses the default: on Linux, Insert+E in Flexible Web said "Not in a dialog
        // box", as no wizard was kept.
        ToUi::WizardShow { wizard } => {
            open.retain(|d| d.id != wizard.id);
            open.push(wizard_page(
                wizard.id,
                &wizard.page,
                &wizard.labels,
                wizard.buttons,
            ));
        }
        ToUi::WizardPage { id, page, buttons } => {
            if let Some(kept) = open.iter_mut().find(|d| d.id == *id) {
                let labels = wizard_labels_of(kept);
                *kept = wizard_page(*id, page, &labels, *buttons);
            }
        }
        ToUi::WizardButtons { id, buttons } => {
            if let Some(kept) = open.iter_mut().find(|d| d.id == *id) {
                set_wizard_default(&mut kept.controls, *buttons);
            }
        }
        ToUi::DialogDefault { id, control } => {
            for dialog in open.iter_mut().filter(|d| d.id == *id) {
                for c in &mut dialog.controls {
                    if let ControlKind::Button { default } = &mut c.kind {
                        *default = c.id == *control;
                    }
                }
            }
        }
        ToUi::DialogTree {
            id,
            control,
            items,
            selected,
        } => {
            if let Some((kept, chosen)) = tree_of(open, *id, control) {
                kept.clone_from(items);
                *chosen = *selected;
            }
        }
        ToUi::DialogRows {
            id,
            control,
            rows,
            checked,
            selected,
        } => {
            let controls = open
                .iter_mut()
                .filter(|d| d.id == *id)
                .flat_map(|d| d.controls.iter_mut());
            for c in controls.filter(|c| c.id == *control) {
                if let ControlKind::CheckList {
                    rows: kept_rows,
                    checked: kept_checked,
                    selected: chosen,
                    ..
                } = &mut c.kind
                {
                    kept_rows.clone_from(rows);
                    kept_checked.clone_from(checked);
                    *chosen = *selected;
                }
            }
        }
        ToUi::DialogSet {
            id,
            control,
            value: value @ (Value::Selected(_) | Value::ItemChecked(..)),
        } if check_list_of(open, *id, control).is_some() => {
            if let Some((checked, chosen)) = check_list_of(open, *id, control) {
                match value {
                    Value::Selected(i) => *chosen = *i,
                    Value::ItemChecked(i, now) => {
                        if let Some(check) = checked.get_mut(*i) {
                            *check = *now;
                        }
                    }
                    _ => {}
                }
            }
        }
        ToUi::DialogSet { id, control, value } => {
            if let Some((items, chosen)) = tree_of(open, *id, control) {
                match value {
                    Value::Selected(i) => *chosen = *i,
                    Value::TreeItem(i, item) => {
                        if let Some(slot) = items.get_mut(*i) {
                            *slot = item.clone();
                        }
                    }
                    _ => {}
                }
            }
        }
        // A drop-down's choices replaced (Voice Adjustment's profiles): its place counted
        // among them (the code review of 2026-11-09, sixth pass, 8).
        ToUi::DialogItems {
            id,
            control,
            items,
            selected,
        } => {
            let controls = open
                .iter_mut()
                .filter(|d| d.id == *id)
                .flat_map(|d| d.controls.iter_mut());
            for c in controls.filter(|c| c.id == *control) {
                if let ControlKind::Choice {
                    options,
                    selected: chosen,
                } = &mut c.kind
                {
                    options.clone_from(items);
                    *chosen = *selected;
                }
            }
        }
        ToUi::DialogLabel { id, control, label } => {
            let controls = open
                .iter_mut()
                .filter(|d| d.id == *id)
                .flat_map(|d| d.controls.iter_mut());
            for c in controls.filter(|c| c.id == *control) {
                c.label.clone_from(label);
            }
        }
        _ => {}
    }
}

/// The ids of a wizard's Next and Finish, kept with its page's controls.
const WIZARD_NEXT: &str = "next";
const WIZARD_FINISH: &str = "finish";

/// A wizard's page kept as a dialog: titled by the page, its controls, then its Next and Finish
/// as hidden buttons, the one Enter presses the default.
fn wizard_page(
    id: u32,
    page: &DialogSpec,
    labels: &WizardLabels,
    buttons: WizardButtons,
) -> DialogSpec {
    let button = |id: &str, label: &str| Control {
        id: id.to_owned(),
        label: label.to_owned(),
        kind: ControlKind::Button { default: false },
        enabled: true,
        hidden: true,
    };
    let mut controls = page.controls.clone();
    controls.push(button(WIZARD_NEXT, &labels.next));
    controls.push(button(WIZARD_FINISH, &labels.finish));
    set_wizard_default(&mut controls, buttons);
    DialogSpec {
        id,
        title: page.title.clone(),
        controls,
        keys: Vec::new(),
    }
}

/// Next the default where the wizard can go on, else Finish.
fn set_wizard_default(controls: &mut [Control], buttons: WizardButtons) {
    let default = if buttons.next {
        WIZARD_NEXT
    } else {
        WIZARD_FINISH
    };
    for c in controls.iter_mut() {
        if let ControlKind::Button { default: on } = &mut c.kind {
            *on = c.id == default;
        }
    }
}

/// A kept wizard's Next and Finish labels.
fn wizard_labels_of(kept: &DialogSpec) -> WizardLabels {
    let label = |id: &str| {
        kept.controls
            .iter()
            .find(|c| c.id == id && c.hidden)
            .map(|c| c.label.clone())
            .unwrap_or_default()
    };
    WizardLabels {
        next: label(WIZARD_NEXT),
        finish: label(WIZARD_FINISH),
        ..WizardLabels::default()
    }
}

/// The items and the choice of dialog `id`'s tree `control`, as kept.
fn tree_of<'a>(
    open: &'a mut [DialogSpec],
    id: u32,
    control: &str,
) -> Option<(&'a mut Vec<TreeItem>, &'a mut usize)> {
    open.iter_mut()
        .filter(|d| d.id == id)
        .flat_map(|d| d.controls.iter_mut())
        .find(|c| c.id == control)
        .and_then(|c| match &mut c.kind {
            ControlKind::Tree { items, selected } => {
                // A tuple of the two fields' borrows.
                Some((items, selected))
            }
            _ => None,
        })
}

/// What the user did in one of Okur's trees, as okur-ui tells it, kept: the item chosen, an
/// item opened or closed.
fn heard(open: &mut [DialogSpec], message: &FromUi) {
    let FromUi::DialogChanged { id, control, value } = message else {
        return;
    };
    if let Some((checked, chosen)) = check_list_of(open, *id, control) {
        match value {
            Value::Selected(i) => *chosen = *i,
            Value::ItemChecked(i, now) => {
                if let Some(check) = checked.get_mut(*i) {
                    *check = *now;
                }
            }
            _ => {}
        }
        return;
    }
    let Some((items, chosen)) = tree_of(open, *id, control) else {
        return;
    };
    match value {
        Value::Selected(i) => *chosen = *i,
        Value::ItemOpened(i, opened) => {
            if let Some(item) = items.get_mut(*i) {
                item.open = *opened;
            }
        }
        _ => {}
    }
}

/// The checks and the choice of dialog `id`'s check list `control`, as kept.
fn check_list_of<'a>(
    open: &'a mut [DialogSpec],
    id: u32,
    control: &str,
) -> Option<(&'a mut Vec<bool>, &'a mut usize)> {
    open.iter_mut()
        .filter(|d| d.id == id)
        .flat_map(|d| d.controls.iter_mut())
        .find(|c| c.id == control)
        .and_then(|c| match &mut c.kind {
            ControlKind::CheckList {
                checked, selected, ..
            } => Some((checked, selected)),
            _ => None,
        })
}

/// The check a row of a check list of Okur's has (`rows`, `checked` and `chosen` as kept): the
/// row its name gives (its first cell, or its cells), else the chosen one.
fn check_of(name: &str, rows: &[Vec<String>], checked: &[bool], chosen: usize) -> Option<bool> {
    let name = name.trim();
    let row = rows
        .iter()
        .position(|cells| {
            cells.first().is_some_and(|first| first.trim() == name)
                || cells.join(" ").trim() == name
        })
        .unwrap_or(chosen);
    checked.get(row).copied()
}

/// `focus` as an item of a tree of Okur's (`label` its tree's, `items` and `chosen` as kept):
/// the item its name gives, or the chosen one for a row that gives none, with its depth, its
/// place among its siblings, its parent, whether it is open and how many items it holds, as
/// Windows' tree view gives them.
fn as_tree_item(
    focus: &FocusEvent,
    label: &str,
    items: &[TreeItem],
    chosen: usize,
) -> Option<FocusEvent> {
    let name = focus.name.as_deref().map(str::trim).unwrap_or_default();
    // The chosen item where the row names none or names it (two items of one text: the one
    // chosen), else the first the name gives.
    let names_chosen = items
        .get(chosen)
        .is_some_and(|item| item.text.trim() == name);
    let at = if name.is_empty() || names_chosen {
        chosen
    } else {
        items.iter().position(|item| item.text.trim() == name)?
    };
    let item = items.get(at)?;
    let top = items.iter().map(|item| item.level).min().unwrap_or(0);
    // Its parent: the nearest item before it one level up.
    let parent = items
        .get(..at)?
        .iter()
        .rposition(|before| before.level < item.level);
    let first_sibling = parent.map_or(0, |p| p + 1);
    // Its siblings: from after its parent up to the first item above their level.
    let end = items
        .iter()
        .enumerate()
        .skip(first_sibling)
        .find(|(_, other)| other.level < item.level)
        .map_or(items.len(), |(i, _)| i);
    let siblings: Vec<usize> = (first_sibling..end)
        .filter(|i| items.get(*i).is_some_and(|other| other.level == item.level))
        .collect();
    let children = items
        .iter()
        .skip(at + 1)
        .take_while(|after| after.level > item.level)
        .filter(|after| after.level == item.level + 1)
        .count();
    let mut told = focus.clone();
    told.role = Role::TreeItem;
    told.name = Some(item.text.clone());
    told.tree_level = Some(item.level.saturating_sub(top));
    told.item_index = siblings.iter().position(|i| *i == at).map(|i| i + 1);
    told.item_count = Some(siblings.len());
    told.item_parent = parent.and_then(|p| items.get(p)).map(|p| p.text.clone());
    told.states.expanded = (children > 0).then_some(item.open);
    told.child_count = (children > 0 && item.open).then_some(children);
    told.states.checked = item.checked;
    told.container_role = Some(Role::Tree);
    told.container_name = Some(shown(label));
    Some(told)
}

/// The control of `controls` the focus is on, by its index: the one its label names, of a kind
/// its role can be; a radio button by its option among a group's.
fn control_of(controls: &[Control], focus: &FocusEvent) -> Option<usize> {
    let name = focus.name.as_deref()?.trim();
    controls.iter().position(|c| match (&c.kind, &focus.role) {
        (ControlKind::Radios { options, .. }, Role::RadioButton) => {
            options.iter().any(|o| shown(o).trim() == name)
        }
        (ControlKind::Choice { .. } | ControlKind::ComboEdit { .. }, Role::ComboBox)
        | (ControlKind::CheckBox { .. }, Role::CheckBox)
        | (ControlKind::Edit { .. } | ControlKind::SearchBox { .. }, Role::Edit)
        | (ControlKind::Slider { .. }, Role::Slider)
        | (ControlKind::Button { .. }, Role::Button) => shown(&c.label).trim() == name,
        _ => false,
    })
}

/// The group box the control at `at` is in, as okur-ui draws it: a radio group's own label
/// (its options are in a box it names); else the group before it, up to the buttons, which end
/// a group box.
fn group_box_of(controls: &[Control], at: usize) -> Option<String> {
    let control = controls.get(at)?;
    if let ControlKind::Radios { .. } = control.kind {
        return Some(shown(&control.label)).filter(|label| !label.trim().is_empty());
    }
    if matches!(control.kind, ControlKind::Button { .. }) {
        return None;
    }
    controls
        .get(..at)?
        .iter()
        .rev()
        .take_while(|c| !matches!(c.kind, ControlKind::Button { .. }))
        .find(|c| c.kind == ControlKind::Group)
        .map(|group| shown(&group.label))
}

/// A label as it is shown: its access key's mark gone, "&&" a plain "&".
fn shown(label: &str) -> String {
    let mut out = String::new();
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' if chars.peek() == Some(&'&') => {
                chars.next();
                out.push('&');
            }
            '&' => {}
            c => out.push(c),
        }
    }
    out
}

impl ScreenReaderEngine {
    /// okur-ui closed a dialog itself (its close box, Escape).
    pub(super) fn own_dialog_closed(&mut self, id: u32) {
        if let Ok(mut open) = self.open_dialogs.lock() {
            open.retain(|d| d.id != id);
        }
    }

    /// What the user did in one of Okur's dialogs, as okur-ui tells it: kept where the spec
    /// follows it (a tree's choice and opened items).
    pub(super) fn own_dialog_heard(&mut self, message: &FromUi) {
        if let Ok(mut open) = self.open_dialogs.lock() {
            heard(&mut open, message);
        }
    }

    /// `focus` in one of Okur's dialogs, told what its spec says and the platform did not: in
    /// a dialog box, its default button, its control's group box, a drop-down's place; none
    /// when nothing is to be told.
    pub(super) fn as_own_dialog_focus(&self, focus: &FocusEvent) -> Option<FocusEvent> {
        let okur_windows = focus
            .app
            .as_deref()
            .map(crate::settings_layers::application_id)
            .is_some_and(|app| app == OKUR_WINDOWS);
        if !okur_windows {
            return None;
        }
        let title = focus.window_title.as_deref()?;
        let open = self.open_dialogs.lock().ok()?;
        let dialog = open.iter().rev().find(|d| d.title == title)?;
        let default = dialog.controls.iter().find_map(|c| match c.kind {
            ControlKind::Button { default: true } => Some(shown(&c.label)),
            _ => None,
        });
        let mut told = focus.clone();
        told.in_dialog_box = true;
        if told.default_button.is_none() {
            told.default_button = default;
        }
        // A row of a check list of Okur's: its check, as Windows' list view with check boxes
        // gives it ("Hide ComplementaryRegion 1 from the top ... checked | 1 of 1" [obs,
        // 742/s18]).
        let check_list = dialog.controls.iter().find_map(|c| match &c.kind {
            ControlKind::CheckList {
                rows,
                checked,
                selected,
                ..
            } => Some((rows, checked, *selected)),
            _ => None,
        });
        if let (Some((rows, checked, chosen)), false, None) =
            (check_list, focus.dialog_box_read, told.states.checked)
        {
            if matches!(focus.role, Role::ListItem | Role::TableRow) {
                let name = focus.name.as_deref().unwrap_or_default();
                told.states.checked = check_of(name, rows, checked, chosen);
            }
        }
        // A wizard's page: its question, its first message, is the dialog's text, said as the
        // focus comes into the page, as Windows' wizard page is ("Flexible Web dialog | What
        // would you like to do?" [obs, 705/s3; 742/s6]).
        let wizard = dialog
            .controls
            .iter()
            .any(|c| c.id == WIZARD_NEXT && c.hidden);
        if wizard && !focus.dialog_box_read && told.dialog_text.as_deref().is_none_or(str::is_empty)
        {
            told.dialog_text = dialog.controls.iter().find_map(|c| match &c.kind {
                ControlKind::Message { text } => Some(text.clone()),
                _ => None,
            });
        }
        // Its page, by the title the wizard takes from it, as Windows' wizard gives it: the
        // next page is said as the page turned ("How would you like to customize the page? |
        // Choose an Action dialog | How would you like ..." [obs, 743/s7]).
        if wizard && !focus.dialog_box_read && told.wizard_page.is_none() {
            told.wizard_page = Some(dialog.title.clone());
        }
        // A row of a tree of Okur's: its item as the spec has it.
        let tree = dialog.controls.iter().find_map(|c| match &c.kind {
            ControlKind::Tree { items, selected } => Some((c.label.as_str(), items, *selected)),
            _ => None,
        });
        if let (Some((label, items, chosen)), None, false) = (
            tree,
            control_of(&dialog.controls, focus),
            focus.dialog_box_read,
        ) {
            if matches!(focus.role, Role::Button | Role::ListItem | Role::TreeItem) {
                if let Some(item) = as_tree_item(&told, label, items, chosen) {
                    told = item;
                }
            }
        }
        if let Some(at) = control_of(&dialog.controls, focus) {
            if told.group_box.is_none() && !focus.dialog_box_read {
                told.group_box = group_box_of(&dialog.controls, at);
                // GTK's frame names the group as the focus's container and a group entered
                // too: said once, as the box ("Use Rule Type: Use Rule Type: Read First Line",
                // 769/s1 in night2), and, no container shared, on each arrow as Windows' group
                // box is ("Use Rule Type: Read First Sentence ... 2 of 4" [obs, 812/s2]).
                if let Some(group) = told.group_box.clone() {
                    let group = group.trim();
                    told.groups.retain(|entry| entry.name.trim() != group);
                    if told.container_name.as_deref().map(str::trim) == Some(group) {
                        told.container_name = None;
                        told.container_role = None;
                    }
                }
            }
            let choice = dialog.controls.get(at).and_then(|c| match &c.kind {
                ControlKind::Choice { options, .. } if focus.role == Role::ComboBox => {
                    Some(options)
                }
                _ => None,
            });
            if let (Some(options), None, false) = (choice, told.item_index, focus.dialog_box_read) {
                let value = focus.value.as_deref().map(str::trim);
                if let Some(i) = options.iter().position(|o| Some(shown(o).trim()) == value) {
                    told.item_index = Some(i + 1);
                    told.item_count = Some(options.len());
                }
            }
        }
        (told != *focus).then_some(told)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::Control;

    fn button(id: &str, label: &str, default: bool) -> Control {
        Control {
            id: id.to_owned(),
            label: label.to_owned(),
            kind: ControlKind::Button { default },
            enabled: true,
            hidden: false,
        }
    }

    /// A row of Okur's Regions tree as GTK gives it (its expander's button, or a row with no
    /// name for the chosen item): its item with its depth, its place among its siblings, its
    /// parent, and an open item's count [obs, 241/s1-s3: "Banner Opened | 1 item", "level 1 |
    /// Primary Navigation | 1 of 1 | Banner", "level 0 | Main | 2 of 4"].
    #[test]
    fn a_trees_row_is_its_item_as_the_spec_has_it() {
        let item = |text: &str, level: usize, open: bool| TreeItem {
            text: text.to_owned(),
            level,
            open,
            checked: None,
        };
        let items = vec![
            item("Banner", 0, true),
            item("Primary Navigation", 1, true),
            item("Main", 0, true),
            item("Complementary", 0, true),
            item("Content Information", 0, true),
        ];
        let nested = FocusEvent::new(Role::Button).with_name("Primary Navigation");
        let told = as_tree_item(&nested, "&Regions", &items, 1);
        let place = told.as_ref().map(|f| {
            (
                f.role.clone(),
                f.tree_level,
                f.item_index,
                f.item_count,
                f.item_parent.clone(),
                f.states.expanded,
                f.container_name.clone(),
            )
        });
        assert_eq!(
            place,
            Some((
                Role::TreeItem,
                Some(1),
                Some(1),
                Some(1),
                Some("Banner".to_owned()),
                None,
                Some("Regions".to_owned())
            ))
        );
        let main = as_tree_item(
            &FocusEvent::new(Role::Button).with_name("Main"),
            "Regions",
            &items,
            2,
        );
        assert_eq!(
            main.as_ref()
                .map(|f| (f.tree_level, f.item_index, f.item_count)),
            Some((Some(0), Some(2), Some(4)))
        );
        // A row with no name: the chosen item, open with its count.
        let banner = as_tree_item(&FocusEvent::new(Role::ListItem), "Regions", &items, 0);
        assert_eq!(
            banner
                .as_ref()
                .map(|f| (f.name.clone(), f.states.expanded, f.child_count)),
            Some((Some("Banner".to_owned()), Some(true), Some(1)))
        );
    }

    /// A control of Okur's own dialog, found by its label: its group box (the group before it,
    /// or a radio group's own label), and a drop-down's choice's place.
    #[test]
    fn a_control_has_its_group_box_and_a_drop_downs_place() {
        let controls = vec![
            Control {
                id: "rule".to_owned(),
                label: "Use Rule &Type:".to_owned(),
                kind: ControlKind::Radios {
                    options: vec![
                        "Read &First Line".to_owned(),
                        "Read First &Sentence".to_owned(),
                    ],
                    selected: 0,
                },
                enabled: true,
                hidden: false,
            },
            Control {
                id: "colors".to_owned(),
                label: "Color or Attribute Rules".to_owned(),
                kind: ControlKind::Group,
                enabled: true,
                hidden: false,
            },
            Control {
                id: "text_color".to_owned(),
                label: "&Text Color:".to_owned(),
                kind: ControlKind::Choice {
                    options: vec!["*".to_owned(), "Red".to_owned(), "Blue".to_owned()],
                    selected: 0,
                },
                enabled: true,
                hidden: false,
            },
            button("ok", "OK", true),
            Control {
                id: "after".to_owned(),
                label: "&Bold".to_owned(),
                kind: ControlKind::CheckBox { checked: false },
                enabled: true,
                hidden: false,
            },
        ];
        let mut radio = FocusEvent::new(Role::RadioButton).with_name("Read First Sentence");
        radio.states.checked = Some(true);
        let at = control_of(&controls, &radio);
        assert_eq!(at, Some(0));
        assert_eq!(
            at.and_then(|at| group_box_of(&controls, at)).as_deref(),
            Some("Use Rule Type:")
        );
        let mut combo = FocusEvent::new(Role::ComboBox).with_name("Text Color:");
        combo.value = Some("Red".to_owned());
        let at = control_of(&controls, &combo);
        assert_eq!(at, Some(2));
        assert_eq!(
            at.and_then(|at| group_box_of(&controls, at)).as_deref(),
            Some("Color or Attribute Rules")
        );
        // After the buttons, no group box.
        let bold = FocusEvent::new(Role::CheckBox).with_name("Bold");
        assert_eq!(
            control_of(&controls, &bold).and_then(|at| group_box_of(&controls, at)),
            None
        );
    }

    /// A wizard is kept as a dialog of its page, its default Next where it can go on, Finish
    /// once it cannot, through its pages and its buttons' changes.
    #[test]
    fn a_wizard_is_kept_with_next_or_finish_its_default() {
        use crate::ui::WizardSpec;
        let mut open = Vec::new();
        let page = |title: &str| DialogSpec {
            id: 9,
            title: title.to_owned(),
            controls: Vec::new(),
            keys: Vec::new(),
        };
        let labels = WizardLabels {
            back: "< &Back".to_owned(),
            next: "&Next >".to_owned(),
            finish: "Finish".to_owned(),
            cancel: "Cancel".to_owned(),
        };
        let default = |open: &[DialogSpec]| {
            open.first().and_then(|d| {
                d.controls.iter().find_map(|c| match c.kind {
                    ControlKind::Button { default: true } => Some(shown(&c.label)),
                    _ => None,
                })
            })
        };
        kept(
            &mut open,
            &ToUi::WizardShow {
                wizard: WizardSpec {
                    id: 9,
                    page: page("Flexible Web"),
                    buttons: WizardButtons {
                        back: false,
                        next: true,
                        finish: false,
                    },
                    labels,
                },
            },
        );
        assert_eq!(default(&open).as_deref(), Some("Next >"));
        kept(
            &mut open,
            &ToUi::WizardPage {
                id: 9,
                page: page("Choose an element"),
                buttons: WizardButtons {
                    back: true,
                    next: false,
                    finish: true,
                },
            },
        );
        assert_eq!(
            open.first().map(|d| d.title.as_str()),
            Some("Choose an element")
        );
        assert_eq!(default(&open).as_deref(), Some("Finish"));
        kept(
            &mut open,
            &ToUi::WizardButtons {
                id: 9,
                buttons: WizardButtons {
                    back: true,
                    next: true,
                    finish: false,
                },
            },
        );
        assert_eq!(default(&open).as_deref(), Some("Next >"));
    }

    /// Shown, its default moved and a label changed, then closed: the dialogs kept follow.
    #[test]
    fn the_dialogs_kept_follow_what_okur_ui_is_told() {
        let mut open = Vec::new();
        let dialog = DialogSpec {
            id: 7,
            title: "PlaceMarker List".to_owned(),
            controls: vec![
                button("add", "&Add", true),
                button("move", "&Move To", false),
            ],
            keys: Vec::new(),
        };
        kept(&mut open, &ToUi::DialogShow { dialog });
        kept(
            &mut open,
            &ToUi::DialogDefault {
                id: 7,
                control: "move".to_owned(),
            },
        );
        kept(
            &mut open,
            &ToUi::DialogLabel {
                id: 7,
                control: "move".to_owned(),
                label: "Move &To && Close".to_owned(),
            },
        );
        let default = open.first().and_then(|d| {
            d.controls.iter().find_map(|c| match c.kind {
                ControlKind::Button { default: true } => Some(shown(&c.label)),
                _ => None,
            })
        });
        assert_eq!(default.as_deref(), Some("Move To & Close"));
        kept(&mut open, &ToUi::DialogClose { id: 7 });
        assert!(open.is_empty());
    }
}
