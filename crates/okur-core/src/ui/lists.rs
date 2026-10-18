//! JAWS's list dialogs as one kind of window: a list of the page's elements of one kind,
//! buttons that act on the selected one, and radio groups that sort and filter the list.
//! Each list is a [`ListKind`], whose labels, options and items are JAWS 2025's:
//! - the Links List (Insert+F7): list "links", Move To Link, Activate Link (the default),
//!   Cancel, Sort Links and Display (all, visited, unvisited), the link's address
//!   [obs, 222];
//! - the Heading List (Insert+F6): list "headings" of "text : level", Move To Heading (the
//!   default), Cancel, Sort Headings and Display (all, or one level of 1 to 6) [obs, 224];
//! - Select a Form Field (Insert+F5): list "List1" of "name type value" entries, OK (the
//!   default) and Cancel [obs, 225, 227];
//! - Virtual HTML Features (Insert+F3): list "List1" of JAWS's 22 element lists, OK (the
//!   default) and Cancel; OK opens the chosen list [obs, 226, 230];
//! - JAWS's element lists (Insert+Ctrl and a letter: "Select a Button", ...): list "List1",
//!   OK (the default) and Cancel; their items are `crate::element_lists` [obs, 231-259];
//! - Document Regions (Insert+Ctrl+R): tree "Regions" of the page's regions, Move To region
//!   start (the default) and Cancel [obs, 241, 256, 274].
//!
//! See `docs/references/jaws-dialogs-and-visual-features.md` for the dialogs' layout.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, TreeItem, Value};

use crate::buffer::{FormFieldListItem, HeadingListItem, LinkListItem};
use crate::element_lists::{ElementItem, ElementList, RegionItem};
use crate::messages::{msg, StaticMessage};

const LINK_SORTS: [fn() -> StaticMessage; 2] = [
    msg::links_list_sort_tab_order,
    msg::links_list_sort_alphabetical,
];
const LINK_DISPLAYS: [fn() -> StaticMessage; 3] = [
    msg::links_list_display_all,
    msg::links_list_display_visited,
    msg::links_list_display_unvisited,
];
const HEADING_SORTS: [fn() -> StaticMessage; 2] = [
    msg::heading_list_sort_tab_order,
    msg::heading_list_sort_alphabetical,
];
/// Heading levels the Display group offers after "All Headings".
const HEADING_LEVELS: u32 = 6;

/// Which of JAWS's list dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Links,
    Headings,
    FormFields,
    /// Virtual HTML Features: the lists themselves.
    Features,
    /// One of JAWS's element lists.
    Elements(ElementList),
    /// Document Regions: a tree.
    Regions,
}

/// The element lists Virtual HTML Features offers, in JAWS's order [obs, 230/s1-s22].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HtmlFeature {
    Anchors,
    Articles,
    BlockQuotes,
    Buttons,
    CheckBoxes,
    Controls,
    Divisions,
    EditBoxes,
    FormFields,
    Frames,
    Graphics,
    Headings,
    Links,
    ListItems,
    Lists,
    Paragraphs,
    PlaceMarkers,
    RadioButtons,
    Regions,
    SmartGlance,
    Tables,
    Tabs,
}

impl HtmlFeature {
    pub const ALL: [Self; 22] = [
        Self::Anchors,
        Self::Articles,
        Self::BlockQuotes,
        Self::Buttons,
        Self::CheckBoxes,
        Self::Controls,
        Self::Divisions,
        Self::EditBoxes,
        Self::FormFields,
        Self::Frames,
        Self::Graphics,
        Self::Headings,
        Self::Links,
        Self::ListItems,
        Self::Lists,
        Self::Paragraphs,
        Self::PlaceMarkers,
        Self::RadioButtons,
        Self::Regions,
        Self::SmartGlance,
        Self::Tables,
        Self::Tabs,
    ];

    /// The list's name, as Virtual HTML Features shows it.
    pub fn name(self) -> StaticMessage {
        match self {
            Self::Anchors => msg::html_feature_anchors(),
            Self::Articles => msg::html_feature_articles(),
            Self::BlockQuotes => msg::html_feature_block_quotes(),
            Self::Buttons => msg::html_feature_buttons(),
            Self::CheckBoxes => msg::html_feature_check_boxes(),
            Self::Controls => msg::html_feature_controls(),
            Self::Divisions => msg::html_feature_divisions(),
            Self::EditBoxes => msg::html_feature_edit_boxes(),
            Self::FormFields => msg::html_feature_form_fields(),
            Self::Frames => msg::html_feature_frames(),
            Self::Graphics => msg::html_feature_graphics(),
            Self::Headings => msg::html_feature_headings(),
            Self::Links => msg::html_feature_links(),
            Self::ListItems => msg::html_feature_list_items(),
            Self::Lists => msg::html_feature_lists(),
            Self::Paragraphs => msg::html_feature_paragraphs(),
            Self::PlaceMarkers => msg::html_feature_placemarkers(),
            Self::RadioButtons => msg::html_feature_radio_buttons(),
            Self::Regions => msg::html_feature_regions(),
            Self::SmartGlance => msg::html_feature_smart_glance(),
            Self::Tables => msg::html_feature_tables(),
            Self::Tabs => msg::html_feature_tabs(),
        }
    }

    /// The list dialog Okur opens for it; the others are not built yet.
    pub fn list(self) -> Option<ListKind> {
        match self {
            Self::Links => Some(ListKind::Links),
            Self::Headings => Some(ListKind::Headings),
            Self::FormFields => Some(ListKind::FormFields),
            Self::RadioButtons => Some(ListKind::Elements(ElementList::RadioButtons)),
            Self::Buttons => Some(ListKind::Elements(ElementList::Buttons)),
            Self::Controls => Some(ListKind::Elements(ElementList::Controls)),
            Self::EditBoxes => Some(ListKind::Elements(ElementList::Edits)),
            Self::CheckBoxes => Some(ListKind::Elements(ElementList::CheckBoxes)),
            Self::Graphics => Some(ListKind::Elements(ElementList::Graphics)),
            Self::Lists => Some(ListKind::Elements(ElementList::Lists)),
            Self::Articles => Some(ListKind::Elements(ElementList::Articles)),
            Self::Paragraphs => Some(ListKind::Elements(ElementList::Paragraphs)),
            Self::BlockQuotes => Some(ListKind::Elements(ElementList::BlockQuotes)),
            Self::Tables => Some(ListKind::Elements(ElementList::Tables)),
            Self::Divisions => Some(ListKind::Elements(ElementList::Divisions)),
            Self::Tabs => Some(ListKind::Elements(ElementList::Tabs)),
            Self::Regions => Some(ListKind::Regions),
            _ => None,
        }
    }
}

/// One of the page's elements, as the list shows it and acts on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// What the list shows.
    pub text: String,
    /// What sorting alphabetically goes by.
    pub sort_key: String,
    /// A heading's level, which the Heading List's Display options filter by.
    pub level: Option<u32>,
    /// A link's address, which the Links List shows for the selected link.
    pub address: Option<String>,
    /// A link within the page.
    pub same_page: bool,
    /// The element's buffer node.
    pub node_id: usize,
    /// Where the element is in the page's buffer.
    pub offset: usize,
    /// In Virtual HTML Features, the list the entry opens.
    pub feature: Option<HtmlFeature>,
}

impl Entry {
    /// A link, after its current state when it has one: "Current Page Overview" [obs,
    /// 222/s1, s9].
    pub fn link(link: &LinkListItem) -> Self {
        Self {
            text: match link.current {
                Some(current) => format!("{current} {}", link.text),
                None => link.text.clone(),
            },
            sort_key: link.text.to_lowercase(),
            level: None,
            address: link.href.clone(),
            same_page: link.is_same_page,
            node_id: link.node_id,
            offset: link.offset,
            feature: None,
        }
    }

    /// A heading, with its level after a colon: "Web App Patterns : 1" [obs, 224/s1].
    pub fn heading(heading: &HeadingListItem) -> Self {
        Self {
            text: format!("{} : {}", heading.text, heading.level),
            sort_key: heading.text.to_lowercase(),
            level: Some(heading.level),
            address: None,
            same_page: false,
            node_id: heading.node_id,
            offset: heading.offset,
            feature: None,
        }
    }

    /// A form field: its name, type, then value or state ("Wrap check box not checked")
    /// [obs, 227].
    pub fn form_field(field: &FormFieldListItem) -> Self {
        Self {
            text: crate::speech::SpeechFormatter::form_field_list_entry(field),
            sort_key: field.name.to_lowercase(),
            level: None,
            address: None,
            same_page: false,
            node_id: field.node_id,
            offset: field.offset,
            feature: None,
        }
    }

    /// An element of one of JAWS's element lists.
    pub fn element(item: &ElementItem) -> Self {
        Self {
            sort_key: item.text.to_lowercase(),
            text: item.text.clone(),
            level: None,
            address: None,
            same_page: false,
            node_id: item.node_id,
            offset: item.offset,
            feature: None,
        }
    }

    /// A region of Document Regions, at its depth.
    pub fn region(item: &RegionItem) -> Self {
        Self {
            sort_key: item.text.to_lowercase(),
            text: item.text.clone(),
            level: u32::try_from(item.level).ok(),
            address: None,
            same_page: false,
            node_id: item.node_id,
            offset: item.offset,
            feature: None,
        }
    }

    /// One of Virtual HTML Features' lists, by its name.
    pub fn feature(feature: HtmlFeature) -> Self {
        let name = feature.name().to_string();
        Self {
            sort_key: name.to_lowercase(),
            text: name,
            level: None,
            address: None,
            same_page: false,
            node_id: 0,
            offset: 0,
            feature: Some(feature),
        }
    }
}

/// What the engine does after the dialog handled an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Send these to the dialog.
    Send(Vec<ToUi>),
    /// Close the dialog and move the virtual cursor to the element (Move To Link, Move To
    /// Heading).
    MoveTo(Entry),
    /// Close the dialog, move to the link and activate it (Activate Link).
    Activate(Entry),
    /// Close the dialog, move to the field and give it the focus (OK in Select a Form Field).
    Focus(Entry),
    /// Open the chosen list in place of Virtual HTML Features (OK there).
    Open(HtmlFeature),
    /// The dialog is gone; the cursor stays.
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListDialog {
    pub id: u32,
    kind: ListKind,
    entries: Vec<Entry>,
    /// 0: in tab order, 1: alphabetically.
    sort: usize,
    /// The Display group's option: 0 shows everything; for links 1 is visited only and 2
    /// unvisited only, for headings 1 to 6 one level.
    display: usize,
    /// The entries shown, as indices into `entries`, in the list's order.
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

fn texts_of(names: &[fn() -> StaticMessage]) -> Vec<String> {
    names.iter().map(|n| n().to_string()).collect()
}

impl ListDialog {
    /// The Links List over the page's `links`, in tab order, the first selected.
    pub fn links(id: u32, links: &[LinkListItem]) -> Self {
        Self::new(id, ListKind::Links, links.iter().map(Entry::link).collect())
    }

    /// The Heading List over the page's `headings`, in tab order, the first selected.
    pub fn headings(id: u32, headings: &[HeadingListItem]) -> Self {
        Self::new(
            id,
            ListKind::Headings,
            headings.iter().map(Entry::heading).collect(),
        )
    }

    /// Select a Form Field over the page's `fields`, in their order, the first selected.
    pub fn form_fields(id: u32, fields: &[FormFieldListItem]) -> Self {
        Self::new(
            id,
            ListKind::FormFields,
            fields.iter().map(Entry::form_field).collect(),
        )
    }

    /// One of JAWS's element lists over `items`, in page order, the first selected.
    pub fn elements(id: u32, list: ElementList, items: &[ElementItem]) -> Self {
        Self::new(
            id,
            ListKind::Elements(list),
            items.iter().map(Entry::element).collect(),
        )
    }

    /// Document Regions over the page's `regions`, the first selected.
    pub fn regions(id: u32, regions: &[RegionItem]) -> Self {
        Self::new(
            id,
            ListKind::Regions,
            regions.iter().map(Entry::region).collect(),
        )
    }

    /// Virtual HTML Features: JAWS's element lists, the first selected.
    pub fn features(id: u32) -> Self {
        Self::new(
            id,
            ListKind::Features,
            HtmlFeature::ALL.into_iter().map(Entry::feature).collect(),
        )
    }

    fn new(id: u32, kind: ListKind, entries: Vec<Entry>) -> Self {
        let mut dialog = Self {
            id,
            kind,
            entries,
            sort: 0,
            display: 0,
            shown: Vec::new(),
            selected: 0,
        };
        dialog.shown = dialog.order();
        dialog
    }

    pub fn kind(&self) -> ListKind {
        self.kind
    }

    /// The list control's id: the list's name in JAWS's dialog.
    fn list_id(&self) -> &'static str {
        match self.kind {
            ListKind::Links => "links",
            ListKind::Headings => "headings",
            ListKind::FormFields => "fields",
            ListKind::Features => "lists",
            ListKind::Elements(_) => "elements",
            ListKind::Regions => "regions",
        }
    }

    /// The Display group's options.
    fn displays(&self) -> Vec<String> {
        match self.kind {
            ListKind::Links => texts_of(&LINK_DISPLAYS),
            ListKind::Headings => std::iter::once(msg::heading_list_display_all().to_string())
                .chain(
                    (1..=HEADING_LEVELS)
                        .map(|level| msg::heading_list_display_level(level).to_string()),
                )
                .collect(),
            ListKind::FormFields
            | ListKind::Features
            | ListKind::Elements(_)
            | ListKind::Regions => Vec::new(),
        }
    }

    /// The entries the options show, in their order. Okur does not know yet which links were
    /// visited: none are.
    fn order(&self) -> Vec<usize> {
        let display = self.display;
        let kind = self.kind;
        let keep = |entry: &Entry| match (kind, display) {
            (_, 0) => true,
            (ListKind::Links, 1) => false,
            (ListKind::Links, _) => true,
            (ListKind::Headings, level) => entry.level == u32::try_from(level).ok(),
            // Select a Form Field has no Display options.
            (
                ListKind::FormFields
                | ListKind::Features
                | ListKind::Elements(_)
                | ListKind::Regions,
                _,
            ) => true,
        };
        let mut shown: Vec<usize> = (0..self.entries.len())
            .filter(|&i| self.entries.get(i).is_some_and(keep))
            .collect();
        if self.sort == 1 {
            shown.sort_by_cached_key(|&i| {
                self.entries
                    .get(i)
                    .map(|e| e.sort_key.clone())
                    .unwrap_or_default()
            });
        }
        shown
    }

    /// What the list shows, in its order.
    pub fn texts(&self) -> Vec<String> {
        self.shown
            .iter()
            .filter_map(|&i| self.entries.get(i))
            .map(|e| e.text.clone())
            .collect()
    }

    fn selected_entry(&self) -> Option<&Entry> {
        self.shown
            .get(self.selected)
            .and_then(|&i| self.entries.get(i))
    }

    fn address(&self) -> String {
        self.selected_entry()
            .and_then(|e| e.address.clone())
            .unwrap_or_default()
    }

    pub fn spec(&self) -> DialogSpec {
        let list = |label: StaticMessage| {
            control(
                self.list_id(),
                label.as_str(),
                ControlKind::List {
                    items: self.texts(),
                    selected: self.selected,
                },
            )
        };
        let button = |id: &str, label: StaticMessage, default: bool| {
            control(id, label.as_str(), ControlKind::Button { default })
        };
        let radios = |id: &str, label: StaticMessage, options: Vec<String>, selected: usize| {
            control(
                id,
                label.as_str(),
                ControlKind::Radios { options, selected },
            )
        };
        let (title, controls) = match self.kind {
            ListKind::Links => (
                msg::list_dialog_links(),
                vec![
                    list(msg::list_dialog_links_list()),
                    button("move", msg::links_list_move(), false),
                    button("activate", msg::links_list_activate(), true),
                    button("cancel", msg::links_list_cancel(), false),
                    radios(
                        "sort",
                        msg::links_list_sort(),
                        texts_of(&LINK_SORTS),
                        self.sort,
                    ),
                    radios(
                        "display",
                        msg::links_list_display(),
                        self.displays(),
                        self.display,
                    ),
                    control(
                        "address",
                        "",
                        ControlKind::Text {
                            text: self.address(),
                        },
                    ),
                ],
            ),
            ListKind::Headings => (
                msg::list_dialog_headings(),
                vec![
                    list(msg::list_dialog_headings_list()),
                    button("move", msg::heading_list_move(), true),
                    button("cancel", msg::heading_list_cancel(), false),
                    radios(
                        "sort",
                        msg::heading_list_sort(),
                        texts_of(&HEADING_SORTS),
                        self.sort,
                    ),
                    radios(
                        "display",
                        msg::heading_list_display(),
                        self.displays(),
                        self.display,
                    ),
                ],
            ),
            ListKind::FormFields => (
                msg::list_dialog_form_fields(),
                vec![
                    list(msg::list_dialog_form_fields_list()),
                    button("ok", msg::form_field_list_ok(), true),
                    button("cancel", msg::form_field_list_cancel(), false),
                ],
            ),
            ListKind::Features => (
                msg::list_dialog_features(),
                vec![
                    list(msg::list_dialog_features_list()),
                    button("ok", msg::features_ok(), true),
                    button("cancel", msg::features_cancel(), false),
                ],
            ),
            ListKind::Elements(elements) => (
                elements.title(),
                vec![
                    list(msg::list_dialog_form_fields_list()),
                    button("ok", msg::features_ok(), true),
                    button("cancel", msg::features_cancel(), false),
                ],
            ),
            ListKind::Regions => (
                msg::list_dialog_regions(),
                vec![
                    control(
                        self.list_id(),
                        msg::list_dialog_regions_tree().as_str(),
                        ControlKind::Tree {
                            items: self
                                .shown
                                .iter()
                                .filter_map(|&i| self.entries.get(i))
                                .map(|e| TreeItem {
                                    text: e.text.clone(),
                                    level: e
                                        .level
                                        .and_then(|l| usize::try_from(l).ok())
                                        .unwrap_or(0),
                                })
                                .collect(),
                            selected: self.selected,
                        },
                    ),
                    button("move", msg::regions_move(), true),
                    button("cancel", msg::regions_cancel(), false),
                ],
            ),
        };
        DialogSpec {
            id: self.id,
            title: title.to_string(),
            controls,
        }
    }

    /// The Links List's address field, after the selection changed; nothing for other lists.
    fn address_update(&self) -> Vec<ToUi> {
        match self.kind {
            ListKind::Links => vec![ToUi::DialogSet {
                id: self.id,
                control: "address".to_owned(),
                value: Value::Text(self.address()),
            }],
            ListKind::Headings
            | ListKind::FormFields
            | ListKind::Features
            | ListKind::Elements(_)
            | ListKind::Regions => Vec::new(),
        }
    }

    /// Shows the entries the options now choose, keeping the selected one where it stays.
    fn reorder(&mut self) -> Outcome {
        let current = self.shown.get(self.selected).copied();
        self.shown = self.order();
        self.selected = current
            .and_then(|c| self.shown.iter().position(|&i| i == c))
            .unwrap_or(0);
        let mut messages = vec![ToUi::DialogItems {
            id: self.id,
            control: self.list_id().to_owned(),
            items: self.texts(),
            selected: self.selected,
        }];
        messages.extend(self.address_update());
        Outcome::Send(messages)
    }

    /// Handles what the user did to the dialog; other dialogs' events change nothing.
    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                let displays = self.displays().len();
                match (control.as_str(), value) {
                    (list, Value::Selected(i)) if list == self.list_id() => {
                        self.selected = *i;
                        match self.address_update() {
                            update if update.is_empty() => Outcome::Nothing,
                            update => Outcome::Send(update),
                        }
                    }
                    ("sort", Value::Selected(i)) => {
                        self.sort = (*i).min(1);
                        self.reorder()
                    }
                    ("display", Value::Selected(i)) => {
                        self.display = (*i).min(displays.saturating_sub(1));
                        self.reorder()
                    }
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                match (control.as_str(), self.selected_entry(), self.kind) {
                    ("move", Some(entry), _) => Outcome::MoveTo(entry.clone()),
                    ("activate", Some(entry), ListKind::Links) => Outcome::Activate(entry.clone()),
                    ("ok", Some(entry), ListKind::FormFields) => Outcome::Focus(entry.clone()),
                    ("ok", Some(entry), ListKind::Elements(list)) if list.focuses() => {
                        Outcome::Focus(entry.clone())
                    }
                    ("ok", Some(entry), ListKind::Elements(_)) => Outcome::MoveTo(entry.clone()),
                    ("ok", Some(entry), ListKind::Features) => {
                        entry.feature.map_or(Outcome::Nothing, Outcome::Open)
                    }
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
            current: None,
        }
    }

    fn heading(text: &str, level: u32, offset: usize) -> HeadingListItem {
        HeadingListItem {
            node_id: offset,
            text: text.to_owned(),
            level,
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

    fn pressed(control: &str) -> FromUi {
        FromUi::DialogPressed {
            id: 2,
            control: control.into(),
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
        let spec = ListDialog::links(2, &links()).spec();
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
        let mut dialog = ListDialog::links(2, &links());
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
        let menu = Entry::link(&link("complete menu", 20));
        assert_eq!(
            dialog.handle(&pressed("move")),
            Outcome::MoveTo(menu.clone())
        );
        assert_eq!(dialog.handle(&pressed("activate")), Outcome::Activate(menu));
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 2 }),
            Outcome::Closed
        );
    }

    /// A link's current state comes before its text in the list, and sorting still goes by the
    /// text [obs, 222/s1, s9].
    #[test]
    fn a_current_link_says_its_state_first() {
        let mut overview = link("Overview", 1);
        overview.current = Some("Current Page");
        let dialog = ListDialog::links(2, &[overview, link("Settings", 2)]);
        assert_eq!(dialog.texts(), vec!["Current Page Overview", "Settings"]);
    }

    /// Okur knows no visited links yet: "Visited Links Only" shows none.
    #[test]
    fn visited_links_only_is_empty_until_okur_knows_visits() {
        let mut dialog = ListDialog::links(2, &links());
        let outcome = dialog.handle(&changed("display", Value::Selected(1)));
        assert!(
            matches!(&outcome, Outcome::Send(sent)
                if matches!(sent.first(), Some(ToUi::DialogItems { items, .. }) if items.is_empty())),
            "{outcome:?}"
        );
        assert_eq!(dialog.handle(&pressed("activate")), Outcome::Nothing);
    }

    /// JAWS's Heading List: "headings" of "text : level", Move To Heading as the default, and
    /// a Display group of all headings and levels 1 to 6 [obs, 224/s1-s6].
    #[test]
    fn the_dialog_is_jaws_heading_list() {
        let headings = [
            heading("Web App Patterns", 1, 0),
            heading("Current links", 2, 10),
            heading("Beef", 3, 20),
        ];
        let spec = ListDialog::headings(2, &headings).spec();
        assert_eq!(spec.title, "Heading List");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "headings",
                "&Move To Heading",
                "&Cancel",
                "Sort Headings",
                "Display"
            ]
        );
        assert_eq!(
            spec.controls.first().map(|c| c.kind.clone()),
            Some(ControlKind::List {
                items: vec![
                    "Web App Patterns : 1".into(),
                    "Current links : 2".into(),
                    "Beef : 3".into()
                ],
                selected: 0
            })
        );
        assert!(spec
            .controls
            .iter()
            .any(|c| c.kind == ControlKind::Button { default: true } && c.id == "move"));
        let display = spec.controls.last().map(|c| c.kind.clone());
        assert!(
            matches!(&display, Some(ControlKind::Radios { options, selected: 0 })
                if options.len() == 7
                    && options.first().map(String::as_str) == Some("A&ll Headings")
                    && options.get(2).map(String::as_str) == Some("Level &2 Headings")),
            "{display:?}"
        );
    }

    /// A level shows only its headings; Move To Heading goes to the selected one.
    #[test]
    fn a_level_shows_its_headings_only() {
        let headings = [
            heading("Web App Patterns", 1, 0),
            heading("Current links", 2, 10),
            heading("Described controls", 2, 20),
        ];
        let mut dialog = ListDialog::headings(2, &headings);
        let outcome = dialog.handle(&changed("display", Value::Selected(2)));
        assert_eq!(
            outcome,
            Outcome::Send(vec![ToUi::DialogItems {
                id: 2,
                control: "headings".into(),
                items: vec!["Current links : 2".into(), "Described controls : 2".into()],
                selected: 0,
            }])
        );
        assert_eq!(
            dialog.handle(&changed("headings", Value::Selected(1))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&pressed("move")),
            Outcome::MoveTo(Entry::heading(&heading("Described controls", 2, 20)))
        );
        assert_eq!(dialog.handle(&pressed("activate")), Outcome::Nothing);
    }

    /// Virtual HTML Features lists JAWS's 22 element lists in its order, with OK (the
    /// default) and Cancel; OK opens the selected list [obs, 230].
    #[test]
    fn the_dialog_is_jaws_virtual_html_features() {
        let mut dialog = ListDialog::features(2);
        let spec = dialog.spec();
        assert_eq!(spec.title, "Virtual HTML Features");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["List1", "OK", "Cancel"]);
        let texts = dialog.texts();
        assert_eq!(texts.len(), 22);
        assert_eq!(texts.first().map(String::as_str), Some("Anchors List"));
        assert_eq!(texts.get(11).map(String::as_str), Some("Headings List"));
        assert_eq!(texts.get(16).map(String::as_str), Some("PlaceMarkers"));
        assert_eq!(texts.last().map(String::as_str), Some("Tabs List"));
        assert_eq!(
            dialog.handle(&changed("lists", Value::Selected(11))),
            Outcome::Nothing
        );
        assert_eq!(
            dialog.handle(&pressed("ok")),
            Outcome::Open(HtmlFeature::Headings)
        );
    }
}
