//! The Links List (Insert+F7) as a real dialog, as JAWS's: a list view named "links", the
//! buttons Move To Link, Activate Link (the default) and Cancel, the radio groups Sort Links
//! and Display, and the selected link's address in a read-only field. Labels, access keys
//! and tab order are JAWS 2025's (dev box, 2026-10-17; see
//! `docs/references/jaws-dialogs-and-visual-features.md`).

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::buffer::LinkListItem;

pub const TITLE: &str = "Links List";

const SORTS: [&str; 2] = ["In &Tab Order", "Al&phabetically"];
const DISPLAYS: [&str; 3] = ["A&ll Links", "&Visited Links Only", "&Unvisited Links"];

/// What the engine does after the dialog handled an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Send these to the dialog.
    Send(Vec<ToUi>),
    /// Close the dialog and move the virtual cursor to the link (Move To Link).
    MoveTo(LinkListItem),
    /// Close the dialog, move to the link and activate it (Activate Link, Enter).
    Activate(LinkListItem),
    /// The dialog is gone; the cursor stays.
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinksListDialog {
    pub id: u32,
    links: Vec<LinkListItem>,
    /// 0: in tab order, 1: alphabetically.
    sort: usize,
    /// 0: all links, 1: visited only, 2: unvisited only.
    display: usize,
    /// The links shown, as indices into `links`, in the list's order.
    shown: Vec<usize>,
    /// The selected entry of `shown`.
    selected: usize,
}

fn control(id: &str, label: &str, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label: label.to_owned(),
        kind,
        enabled: true,
    }
}

impl LinksListDialog {
    /// The dialog over the page's `links`, in tab order, the first selected.
    pub fn new(id: u32, links: Vec<LinkListItem>) -> Self {
        let mut dialog = Self {
            id,
            links,
            sort: 0,
            display: 0,
            shown: Vec::new(),
            selected: 0,
        };
        dialog.shown = dialog.order();
        dialog
    }

    /// The links the options show, in their order. Okur does not know yet which links were
    /// visited: none are.
    fn order(&self) -> Vec<usize> {
        let visited = |_: &LinkListItem| false;
        let mut shown: Vec<usize> = (0..self.links.len())
            .filter(|&i| {
                self.links.get(i).is_some_and(|link| match self.display {
                    1 => visited(link),
                    2 => !visited(link),
                    _ => true,
                })
            })
            .collect();
        if self.sort == 1 {
            shown.sort_by_cached_key(|&i| {
                self.links
                    .get(i)
                    .map(|l| l.text.to_lowercase())
                    .unwrap_or_default()
            });
        }
        shown
    }

    fn texts(&self) -> Vec<String> {
        self.shown
            .iter()
            .filter_map(|&i| self.links.get(i))
            .map(|l| l.text.clone())
            .collect()
    }

    fn selected_link(&self) -> Option<&LinkListItem> {
        self.shown
            .get(self.selected)
            .and_then(|&i| self.links.get(i))
    }

    fn address(&self) -> String {
        self.selected_link()
            .and_then(|l| l.href.clone())
            .unwrap_or_default()
    }

    pub fn spec(&self) -> DialogSpec {
        let options = |names: &[&str]| names.iter().map(|n| (*n).to_owned()).collect();
        DialogSpec {
            id: self.id,
            title: TITLE.to_owned(),
            controls: vec![
                control(
                    "links",
                    "links",
                    ControlKind::List {
                        items: self.texts(),
                        selected: self.selected,
                    },
                ),
                control(
                    "move",
                    "&Move To Link",
                    ControlKind::Button { default: false },
                ),
                control(
                    "activate",
                    "&Activate Link",
                    ControlKind::Button { default: true },
                ),
                control("cancel", "&Cancel", ControlKind::Button { default: false }),
                control(
                    "sort",
                    "Sort Links",
                    ControlKind::Radios {
                        options: options(&SORTS),
                        selected: self.sort,
                    },
                ),
                control(
                    "display",
                    "Display",
                    ControlKind::Radios {
                        options: options(&DISPLAYS),
                        selected: self.display,
                    },
                ),
                control(
                    "address",
                    "",
                    ControlKind::Text {
                        text: self.address(),
                    },
                ),
            ],
        }
    }

    fn address_update(&self) -> ToUi {
        ToUi::DialogSet {
            id: self.id,
            control: "address".to_owned(),
            value: Value::Text(self.address()),
        }
    }

    /// Shows the links the options now choose, keeping the selected one where it stays.
    fn reorder(&mut self) -> Outcome {
        let current = self.shown.get(self.selected).copied();
        self.shown = self.order();
        self.selected = current
            .and_then(|c| self.shown.iter().position(|&i| i == c))
            .unwrap_or(0);
        Outcome::Send(vec![
            ToUi::DialogItems {
                id: self.id,
                control: "links".to_owned(),
                items: self.texts(),
                selected: self.selected,
            },
            self.address_update(),
        ])
    }

    /// Handles what the user did to the dialog; other dialogs' events change nothing.
    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("links", Value::Selected(i)) => {
                        self.selected = *i;
                        Outcome::Send(vec![self.address_update()])
                    }
                    ("sort", Value::Selected(i)) => {
                        self.sort = (*i).min(SORTS.len() - 1);
                        self.reorder()
                    }
                    ("display", Value::Selected(i)) => {
                        self.display = (*i).min(DISPLAYS.len() - 1);
                        self.reorder()
                    }
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                match (control.as_str(), self.selected_link()) {
                    ("move", Some(link)) => Outcome::MoveTo(link.clone()),
                    ("activate", Some(link)) => Outcome::Activate(link.clone()),
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link(text: &str, offset: usize) -> LinkListItem {
        LinkListItem {
            node_id: offset,
            text: text.to_owned(),
            href: Some(format!("https://example.com/{offset}")),
            is_same_page: false,
            offset,
        }
    }

    fn changed(control: &str, value: Value) -> FromUi {
        FromUi::DialogChanged {
            id: 2,
            control: control.to_owned(),
            value,
        }
    }

    fn links() -> Vec<LinkListItem> {
        vec![
            link("Driving Directions", 10),
            link("complete menu", 20),
            link("Beef", 30),
        ]
    }

    #[test]
    fn the_dialog_is_jaws_links_list() {
        let spec = LinksListDialog::new(2, links()).spec();
        assert_eq!(spec.title, "Links List");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "links",
                "&Move To Link",
                "&Activate Link",
                "&Cancel",
                "Sort Links",
                "Display",
                ""
            ]
        );
        assert_eq!(
            spec.controls.first().map(|c| c.kind.clone()),
            Some(ControlKind::List {
                items: vec![
                    "Driving Directions".into(),
                    "complete menu".into(),
                    "Beef".into()
                ],
                selected: 0
            })
        );
        assert_eq!(
            spec.controls.last().map(|c| c.kind.clone()),
            Some(ControlKind::Text {
                text: "https://example.com/10".into()
            })
        );
    }

    #[test]
    fn sorting_keeps_the_selected_link_and_the_buttons_act_on_it() {
        let mut dialog = LinksListDialog::new(2, links());
        assert_eq!(
            dialog.handle(&changed("links", Value::Selected(1))),
            Outcome::Send(vec![ToUi::DialogSet {
                id: 2,
                control: "address".into(),
                value: Value::Text("https://example.com/20".into()),
            }])
        );
        // Alphabetically, ignoring case: Beef, complete menu, Driving Directions.
        assert_eq!(
            dialog.handle(&changed("sort", Value::Selected(1))),
            Outcome::Send(vec![
                ToUi::DialogItems {
                    id: 2,
                    control: "links".into(),
                    items: vec![
                        "Beef".into(),
                        "complete menu".into(),
                        "Driving Directions".into()
                    ],
                    selected: 1,
                },
                ToUi::DialogSet {
                    id: 2,
                    control: "address".into(),
                    value: Value::Text("https://example.com/20".into()),
                },
            ])
        );
        let pressed = |control: &str| FromUi::DialogPressed {
            id: 2,
            control: control.into(),
        };
        assert_eq!(
            dialog.handle(&pressed("move")),
            Outcome::MoveTo(link("complete menu", 20))
        );
        assert_eq!(
            dialog.handle(&pressed("activate")),
            Outcome::Activate(link("complete menu", 20))
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 2 }),
            Outcome::Closed
        );
    }

    /// Okur knows no visited links yet: "Visited Links Only" shows none.
    #[test]
    fn visited_links_only_is_empty_until_okur_knows_visits() {
        let mut dialog = LinksListDialog::new(2, links());
        let outcome = dialog.handle(&changed("display", Value::Selected(1)));
        assert!(
            matches!(&outcome, Outcome::Send(sent)
                if matches!(sent.first(), Some(ToUi::DialogItems { items, .. }) if items.is_empty())),
            "{outcome:?}"
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 2,
                control: "activate".into()
            }),
            Outcome::Nothing
        );
    }
}
