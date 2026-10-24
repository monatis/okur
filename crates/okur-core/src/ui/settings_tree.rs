//! Settings Center (Insert+6) and Quick Settings (Insert+V), as JAWS's: a search box over a
//! tree of groups and settings, each setting named with its value ("Typing Echo Characters"),
//! Space changing it and Shift+Space back, the selected item's description below the tree, then
//! OK, Cancel, Apply once something changed, and in Settings Center the place the settings are
//! kept (`docs/references/okur-settings-redesign.md` §3.1). Generated from the settings table
//! ([`crate::settings::registry`]): a setting added there with a label is here too.
//!
//! The dialog keeps a draft of the settings, as [`super::settings::SettingsDialog`] does; Apply
//! and OK make it Okur's settings, Cancel drops what was not applied.

use okur_ui_model::{Control, ControlKind, DialogKey, DialogSpec, FromUi, ToUi, TreeItem, Value};

use super::settings::{Outcome, Place};
use crate::messages::{msg, StaticMessage};
use crate::settings::registry::{self, Kind, SettingSpec, SETTINGS};
use crate::settings::Settings;
use crate::settings_layers::Scope;

/// A setting's value, as the settings file writes it.
type Json = serde_json::Value;

/// The tree's control id, and the others'.
const TREE: &str = "tree";
const SEARCH: &str = "search";
const DESCRIPTION: &str = "description";
const PLACE: &str = "application";

/// Which of JAWS's two dialogs this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    /// Every setting, for the place its Application box chooses (Insert+6).
    Center,
    /// The settings JAWS's Quick Settings offers, for the application in front (Insert+V).
    Quick,
}

/// A group of the tree, by JAWS's name: Settings Center's groups and subgroups that hold
/// Okur's settings, and Quick Settings' own groups (old-pc's walks, the redesign's §1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeGroup {
    User,
    Web,
    WebReading,
    FormsMode,
    TextProcessing,
    TextGeneral,
    SpeechVerbosity,
    SayAll,
    Synthesizer,
    Punctuation,
    Keyboard,
    KeyboardGeneral,
    MouseEcho,
    Miscellaneous,
    GeneralOptions,
    SayAllOptions,
    EditingOptions,
}

impl TreeGroup {
    /// Settings Center's, in JAWS's order, each group right before its subgroups.
    const CENTER: [TreeGroup; 14] = [
        Self::User,
        Self::Web,
        Self::WebReading,
        Self::FormsMode,
        Self::TextProcessing,
        Self::TextGeneral,
        Self::SpeechVerbosity,
        Self::SayAll,
        Self::Synthesizer,
        Self::Punctuation,
        Self::Keyboard,
        Self::KeyboardGeneral,
        Self::MouseEcho,
        Self::Miscellaneous,
    ];
    /// Quick Settings', in its order in Notepad.
    const QUICK: [TreeGroup; 3] = [
        Self::GeneralOptions,
        Self::SayAllOptions,
        Self::EditingOptions,
    ];

    /// The group a subgroup is in.
    fn parent(self) -> Option<TreeGroup> {
        match self {
            Self::WebReading => Some(Self::Web),
            Self::TextGeneral => Some(Self::TextProcessing),
            Self::KeyboardGeneral => Some(Self::Keyboard),
            _ => None,
        }
    }

    /// Its level in the tree: 0 at the top.
    fn depth(self) -> usize {
        self.parent().map_or(0, |parent| parent.depth() + 1)
    }

    /// Whether it is `group` or one of its subgroups.
    fn within(self, group: TreeGroup) -> bool {
        self == group || self.parent().is_some_and(|parent| parent.within(group))
    }

    fn label(self) -> StaticMessage {
        match self {
            Self::User => msg::settings_tree_user(),
            Self::Web => msg::settings_tree_web(),
            Self::WebReading => msg::settings_tree_reading(),
            Self::FormsMode => msg::settings_tree_forms_mode(),
            Self::TextProcessing => msg::settings_tree_text_processing(),
            Self::TextGeneral | Self::KeyboardGeneral => msg::settings_tree_general(),
            Self::SpeechVerbosity => msg::settings_tree_speech_verbosity(),
            Self::SayAll => msg::settings_tree_say_all(),
            Self::Synthesizer => msg::settings_tree_synthesizer(),
            Self::Punctuation => msg::settings_tree_punctuation(),
            Self::Keyboard => msg::settings_tree_keyboard(),
            Self::MouseEcho => msg::settings_tree_mouse_echo(),
            Self::Miscellaneous => msg::settings_tree_miscellaneous(),
            Self::GeneralOptions => msg::settings_tree_general_options(),
            Self::SayAllOptions => msg::settings_tree_say_all_options(),
            Self::EditingOptions => msg::settings_tree_editing_options(),
        }
    }
}

/// Where Settings Center keeps a setting: JAWS's place for it, as old-pc's walk found it
/// ("Typing Echo" in User, "Document and Web Pages Automatically Read When Loaded" in Web /
/// HTML / PDFs, Reading). Okur's voice settings are in Synthesizer until Voice Adjustment has
/// them. `None`: not shown.
fn center_group(id: &str) -> Option<TreeGroup> {
    Some(match id {
        "typing_echo" | "viewer_display" => TreeGroup::User,
        "read_page_on_load" => TreeGroup::WebReading,
        "forms_mode" => TreeGroup::FormsMode,
        "voice.indicate_caps" | "live_region_append" => TreeGroup::TextGeneral,
        "verbosity.level" => TreeGroup::SpeechVerbosity,
        "say_all_unit" => TreeGroup::SayAll,
        "voice.synthesizer" | "voice.voice" | "voice.rate" | "voice.volume" => {
            TreeGroup::Synthesizer
        }
        "voice.punctuation" => TreeGroup::Punctuation,
        "keyboard.layout" | "keyboard.okur_key" => TreeGroup::KeyboardGeneral,
        "mouse_echo.enabled" | "mouse_echo.instant" | "mouse_echo.unit" => TreeGroup::MouseEcho,
        "confirm_exit" => TreeGroup::Miscellaneous,
        _ => return None,
    })
}

/// Where Quick Settings offers a setting, as it did in Notepad ("User Verbosity" in General
/// Options, "Typing Echo" in Editing Options). `None`: not offered.
fn quick_group(id: &str) -> Option<TreeGroup> {
    Some(match id {
        "verbosity.level" => TreeGroup::GeneralOptions,
        "say_all_unit" => TreeGroup::SayAllOptions,
        "typing_echo" | "voice.indicate_caps" | "voice.punctuation" => TreeGroup::EditingOptions,
        _ => return None,
    })
}

/// A row of the tree: a group, a setting in the group above it, or an item that opens a
/// dialog of its own.
#[derive(Debug, Clone, Copy)]
enum Row {
    Group(TreeGroup),
    Setting(&'static SettingSpec),
    /// Speech Verbosity's "Configure Verbosity Levels": Space opens its dialog.
    VerbosityLevels,
}

impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Group(a), Self::Group(b)) => a == b,
            (Self::Setting(a), Self::Setting(b)) => a.id == b.id,
            (Self::VerbosityLevels, Self::VerbosityLevels) => true,
            _ => false,
        }
    }
}

impl Eq for Row {}

/// A label without its access key's "&" and its colon: "Typing &Echo:" is "Typing Echo".
fn plain(label: StaticMessage) -> String {
    label
        .as_str()
        .replace('&', "")
        .trim_end()
        .trim_end_matches(':')
        .to_owned()
}

/// The settings tree's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTree {
    pub id: u32,
    flavor: Flavor,
    places: Vec<Place>,
    place: usize,
    draft: Settings,
    voices: Vec<String>,
    /// The words searched for; empty, every group.
    search: String,
    /// The groups open.
    open: Vec<TreeGroup>,
    rows: Vec<Row>,
    selected: usize,
    /// Something changed since the dialog opened or was applied: Apply is available.
    changed: bool,
}

impl SettingsTree {
    /// The dialog over `places`, on `place` among them, offering `voices` (the synthesizer's,
    /// by name). Every group starts closed, as JAWS's.
    pub fn new(
        id: u32,
        flavor: Flavor,
        places: Vec<Place>,
        place: usize,
        voices: Vec<String>,
    ) -> Self {
        let place = place.min(places.len().saturating_sub(1));
        let draft = places
            .get(place)
            .map(|p| p.settings.clone())
            .unwrap_or_default();
        let mut tree = Self {
            id,
            flavor,
            places,
            place,
            draft,
            voices,
            search: String::new(),
            open: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            changed: false,
        };
        tree.rows = tree.rows_for_search();
        tree
    }

    /// Settings Center or Quick Settings.
    pub fn flavor(&self) -> Flavor {
        self.flavor
    }

    /// The settings as the dialog shows them now.
    pub fn draft(&self) -> &Settings {
        &self.draft
    }

    fn scope(&self) -> Scope {
        self.places
            .get(self.place)
            .map_or(Scope::Global, |p| p.scope.clone())
    }

    /// A setting's group in this dialog; `None` for one it does not show.
    fn group_of(&self, spec: &SettingSpec) -> Option<TreeGroup> {
        spec.label?;
        match self.flavor {
            Flavor::Center => center_group(spec.id),
            Flavor::Quick => quick_group(spec.id),
        }
    }

    /// The settings this dialog shows, each with its group, in the table's order.
    fn shown(&self) -> impl Iterator<Item = (TreeGroup, &'static SettingSpec)> + '_ {
        SETTINGS
            .iter()
            .filter_map(move |spec| Some((self.group_of(spec)?, spec)))
    }

    /// A setting's name in the tree: JAWS's there where it differs from the dialog's label
    /// (`<label>-tree`, and Quick Settings' `<label>-quick`: "Select Forms Mode", "User
    /// Verbosity"), else the label.
    fn tree_label(&self, spec: &SettingSpec) -> String {
        let Some(label) = spec.label.map(|label| label()) else {
            return String::new();
        };
        let quick = (self.flavor == Flavor::Quick)
            .then(|| label.suffixed("quick"))
            .flatten();
        quick
            .or_else(|| label.suffixed("tree"))
            .map_or_else(|| plain(label), |named| named.to_string())
    }

    /// The rows for the words searched for: every group and its settings, or only the settings
    /// that match, in their groups.
    fn rows_for_search(&self) -> Vec<Row> {
        let words: Vec<String> = self
            .search
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let matches = |spec: &SettingSpec| {
            let label = self.tree_label(spec);
            let description = spec
                .label
                .and_then(|l| l().description())
                .map(|d| d.as_str().to_owned())
                .unwrap_or_default();
            let text = format!("{label} {description}").to_lowercase();
            words.iter().all(|w| text.contains(w))
        };
        let levels = self.flavor == Flavor::Center
            && words.iter().all(|w| {
                msg::settings_tree_configure_verbosity()
                    .as_str()
                    .to_lowercase()
                    .contains(w)
            });
        let order: &[TreeGroup] = match self.flavor {
            Flavor::Center => &TreeGroup::CENTER,
            Flavor::Quick => &TreeGroup::QUICK,
        };
        // Each group's own rows: its settings found, and Configure Verbosity Levels.
        let own = |group: TreeGroup| -> Vec<Row> {
            let mut rows: Vec<Row> = self
                .shown()
                .filter(|(g, spec)| *g == group && matches(spec))
                .map(|(_, spec)| Row::Setting(spec))
                .collect();
            if levels && group == TreeGroup::SpeechVerbosity {
                rows.push(Row::VerbosityLevels);
            }
            rows
        };
        // A group is shown when it or one of its subgroups has something to show.
        let filled: Vec<TreeGroup> = order
            .iter()
            .copied()
            .filter(|&g| !own(g).is_empty())
            .collect();
        let mut rows = Vec::new();
        for &group in order {
            if !filled.iter().any(|f| f.within(group)) {
                continue;
            }
            rows.push(Row::Group(group));
            rows.extend(own(group));
        }
        rows
    }

    /// A setting's value as the tree names it: a choice's label, a number, a voice's name.
    fn value_text(&self, spec: &SettingSpec) -> Option<String> {
        let value = registry::get(&self.draft, spec.id).unwrap_or(Json::Null);
        match spec.kind {
            Kind::Choice(choices) => choices()
                .into_iter()
                .find(|c| c.value == value)
                .map(|c| c.label.to_string()),
            Kind::Range { .. } => value.as_i64().map(|n| n.to_string()),
            Kind::Voice => Some(
                value
                    .as_str()
                    .map_or_else(|| msg::settings_voice_default().to_string(), str::to_owned),
            ),
            Kind::Check { .. } | Kind::Values(_) | Kind::Structured => None,
        }
    }

    /// A row as the tree shows it.
    fn item(&self, row: &Row) -> TreeItem {
        match row {
            Row::VerbosityLevels => TreeItem {
                text: msg::settings_tree_configure_verbosity().to_string(),
                level: TreeGroup::SpeechVerbosity.depth() + 1,
                open: true,
                checked: None,
            },
            Row::Group(group) => TreeItem {
                text: group.label().to_string(),
                level: group.depth(),
                open: !self.search.is_empty() || self.open.contains(group),
                checked: None,
            },
            Row::Setting(spec) => {
                let label = self.tree_label(spec);
                let level = self.group_of(spec).map_or(1, |g| g.depth() + 1);
                let checked = match spec.kind {
                    Kind::Check { on, .. } => {
                        Some(registry::get(&self.draft, spec.id) == Some(on.value()))
                    }
                    _ => None,
                };
                let text = match self.value_text(spec) {
                    Some(value) => format!("{label} {value}"),
                    None => label,
                };
                TreeItem {
                    text,
                    level,
                    open: true,
                    checked,
                }
            }
        }
    }

    fn items(&self) -> Vec<TreeItem> {
        self.rows.iter().map(|row| self.item(row)).collect()
    }

    /// The selected item's description: a group's, or a setting's.
    fn description(&self) -> String {
        match self.rows.get(self.selected) {
            Some(Row::Setting(spec)) => spec
                .label
                .and_then(|l| l().description())
                .map(|d| d.to_string())
                .unwrap_or_default(),
            Some(Row::VerbosityLevels) => {
                msg::settings_tree_configure_verbosity_description().to_string()
            }
            Some(Row::Group(_)) | None => msg::settings_tree_group_description().to_string(),
        }
    }

    fn title(&self) -> String {
        let place = self
            .places
            .get(self.place)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        match self.flavor {
            Flavor::Center => msg::settings_center_title(place.as_str()).render(),
            Flavor::Quick => msg::quick_settings_title(place.as_str()).render(),
        }
    }

    /// The dialog: the search box, the tree, the description, OK, Cancel, Apply, and in
    /// Settings Center the place, in JAWS's Tab order.
    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        let mut controls = vec![
            control(
                SEARCH,
                msg::settings_tree_search().to_string(),
                ControlKind::ComboEdit {
                    text: self.search.clone(),
                    history: Vec::new(),
                },
            ),
            control(
                TREE,
                String::new(),
                ControlKind::Tree {
                    items: self.items(),
                    selected: self.selected,
                },
            ),
            control(
                DESCRIPTION,
                String::new(),
                ControlKind::Note {
                    text: self.description(),
                },
            ),
            control(
                "ok",
                msg::settings_ok().to_string(),
                ControlKind::Button { default: true },
            ),
            control(
                "cancel",
                msg::settings_cancel().to_string(),
                ControlKind::Button { default: false },
            ),
            Control {
                enabled: self.changed,
                ..control(
                    "apply",
                    msg::settings_apply().to_string(),
                    ControlKind::Button { default: false },
                )
            },
        ];
        if self.flavor == Flavor::Center {
            controls.push(control(
                PLACE,
                msg::settings_tree_application().to_string(),
                ControlKind::Choice {
                    options: self.places.iter().map(|p| p.name.clone()).collect(),
                    selected: self.place,
                },
            ));
        }
        DialogSpec {
            id: self.id,
            title: self.title(),
            controls,
        }
    }

    /// The setting `spec`'s next value (`back`: its previous), wrapping around; `None` for one
    /// the tree cannot change, or the place cannot keep.
    fn next_value(&self, spec: &SettingSpec, back: bool) -> Option<Json> {
        if !spec.scopes.allows(&self.scope()) {
            return None;
        }
        let value = registry::get(&self.draft, spec.id).unwrap_or(Json::Null);
        let step = |values: Vec<Json>| -> Option<Json> {
            let count = values.len();
            let at = values.iter().position(|v| *v == value).unwrap_or(0);
            let next = if back { at + count - 1 } else { at + 1 } % count.max(1);
            values.into_iter().nth(next)
        };
        match spec.kind {
            Kind::Choice(choices) => step(choices().into_iter().map(|c| c.value).collect()),
            Kind::Check { on, off } => step(vec![off.value(), on.value()]),
            Kind::Voice => step(
                std::iter::once(Json::Null)
                    .chain(self.voices.iter().map(|v| Json::String(v.clone())))
                    .collect(),
            ),
            Kind::Range { min, max } => {
                // A tenth of the range a press, as JAWS's sliders move by Page Up and Down.
                let step = ((max - min) / 10).max(1);
                let now = value
                    .as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .unwrap_or(min);
                let next = if back { now - step } else { now + step };
                let next = if next > max {
                    min
                } else if next < min {
                    max
                } else {
                    next
                };
                Some(Json::from(next))
            }
            Kind::Values(_) | Kind::Structured => None,
        }
    }

    /// Changes the selected setting's value: its item and the Apply button are updated.
    fn change(&mut self, back: bool) -> Outcome {
        let spec = match self.rows.get(self.selected).copied() {
            Some(Row::Setting(spec)) => spec,
            Some(Row::VerbosityLevels) => return Outcome::OpenVerbosityLevels,
            Some(Row::Group(_)) | None => return Outcome::Nothing,
        };
        let Some(value) = self.next_value(spec, back) else {
            return Outcome::Nothing;
        };
        if !registry::set(&mut self.draft, spec.id, value) {
            return Outcome::Nothing;
        }
        self.changed = true;
        // A change can bring another with it (a layout its Okur key): every item is said anew.
        let mut sent: Vec<ToUi> = self
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| ToUi::DialogSet {
                id: self.id,
                control: TREE.to_owned(),
                value: Value::TreeItem(index, self.item(row)),
            })
            .collect();
        sent.push(ToUi::DialogEnable {
            id: self.id,
            control: "apply".to_owned(),
            enabled: true,
        });
        Outcome::Send(sent)
    }

    /// The whole tree again, after a search or another place.
    fn refill(&self) -> Vec<ToUi> {
        vec![
            ToUi::DialogTree {
                id: self.id,
                control: TREE.to_owned(),
                items: self.items(),
                selected: self.selected,
            },
            ToUi::DialogSet {
                id: self.id,
                control: DESCRIPTION.to_owned(),
                value: Value::Text(self.description()),
            },
        ]
    }

    fn apply(&mut self, close: bool) -> Outcome {
        let before = self
            .places
            .get(self.place)
            .map(|p| p.settings.clone())
            .unwrap_or_default();
        if let Some(place) = self.places.get_mut(self.place) {
            place.settings = self.draft.clone();
        }
        self.changed = false;
        Outcome::Apply {
            scope: self.scope(),
            before: Box::new(before),
            settings: Box::new(self.draft.clone()),
            close,
        }
    }

    /// What the dialog's window reported.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    (TREE, Value::Selected(index)) => {
                        self.selected = *index;
                        Outcome::Send(vec![ToUi::DialogSet {
                            id: self.id,
                            control: DESCRIPTION.to_owned(),
                            value: Value::Text(self.description()),
                        }])
                    }
                    (TREE, Value::ItemOpened(index, open)) => {
                        if let Some(Row::Group(group)) = self.rows.get(*index).copied() {
                            self.open.retain(|g| *g != group);
                            if *open {
                                self.open.push(group);
                            }
                        }
                        Outcome::Nothing
                    }
                    (SEARCH, Value::Text(text)) => {
                        self.search = text.trim().to_owned();
                        self.rows = self.rows_for_search();
                        // The first setting found, or the first group.
                        self.selected = if self.search.is_empty() {
                            0
                        } else {
                            self.rows
                                .iter()
                                .position(|r| matches!(r, Row::Setting(_)))
                                .unwrap_or(0)
                        };
                        Outcome::Send(self.refill())
                    }
                    (PLACE, Value::Selected(place)) if *place < self.places.len() => {
                        self.place = *place;
                        self.draft = self
                            .places
                            .get(*place)
                            .map(|p| p.settings.clone())
                            .unwrap_or_default();
                        Outcome::Send(self.refill())
                    }
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogKey { id, control, key } if *id == self.id && control == TREE => {
                match key {
                    DialogKey::Change => self.change(false),
                    DialogKey::ChangeBack => self.change(true),
                    DialogKey::Delete | DialogKey::Rename => Outcome::Nothing,
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => self.apply(true),
                "apply" => self.apply(false),
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(flavor: Flavor) -> SettingsTree {
        let places = vec![
            Place {
                scope: Scope::Global,
                name: "Default (All Applications)".into(),
                settings: Settings::default(),
            },
            Place {
                scope: Scope::Application("notepad".into()),
                name: "Notepad".into(),
                settings: Settings::default(),
            },
        ];
        SettingsTree::new(7, flavor, places, 1, vec!["Zira".into()])
    }

    fn tree_items(spec: &DialogSpec) -> Vec<TreeItem> {
        spec.controls
            .iter()
            .find_map(|c| match &c.kind {
                ControlKind::Tree { items, .. } => Some(items.clone()),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// Settings Center: the search box first, then the tree of groups, closed, each setting
    /// named with its value, the description, OK, Cancel, Apply unavailable, and the place
    /// [obs, walk of Settings Center, the redesign's §1.1].
    #[test]
    fn settings_center_is_a_search_box_over_a_tree() {
        let dialog = tree(Flavor::Center);
        let spec = dialog.spec();
        assert_eq!(spec.title, "Notepad - Okur Settings Center");
        let ids: Vec<&str> = spec.controls.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids,
            [SEARCH, TREE, DESCRIPTION, "ok", "cancel", "apply", PLACE]
        );
        assert!(spec.controls.iter().any(|c| c.id == "apply" && !c.enabled));
        let items = tree_items(&spec);
        let first = items.first().cloned();
        assert_eq!(
            first.map(|i| (i.text, i.level, i.open)),
            Some(("User".to_owned(), 0, false))
        );
        assert!(items
            .iter()
            .any(|i| i.text == "Typing Echo Characters" && i.level == 1));
        assert!(items
            .iter()
            .any(|i| i.text == "Enable Mouse Echo" && i.checked == Some(false)));
    }

    /// Space changes the selected setting to its next value, Shift+Space back; the item is
    /// named anew and Apply becomes available; OK applies the draft to the place.
    #[test]
    fn space_changes_a_setting_and_ok_applies_it() {
        let mut dialog = tree(Flavor::Center);
        let index = dialog
            .rows
            .iter()
            .position(|r| matches!(r, Row::Setting(spec) if spec.id == "typing_echo"))
            .unwrap_or_default();
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: TREE.into(),
            value: Value::Selected(index),
        });
        let Outcome::Send(sent) = dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::Change,
        }) else {
            panic_free_fail("no update");
            return;
        };
        assert!(sent.iter().any(|m| matches!(
            m,
            ToUi::DialogSet { value: Value::TreeItem(i, item), .. }
                if *i == index && item.text == "Typing Echo Words"
        )));
        assert!(sent.iter().any(
            |m| matches!(m, ToUi::DialogEnable { control, enabled: true, .. } if control == "apply")
        ));
        dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::ChangeBack,
        });
        dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::ChangeBack,
        });
        assert_eq!(
            registry::get(dialog.draft(), "typing_echo"),
            Some(Json::String("none".into()))
        );
        let applied = dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: "ok".into(),
        });
        assert!(matches!(
            applied,
            Outcome::Apply { scope: Scope::Application(ref app), close: true, .. } if app == "notepad"
        ));
    }

    /// Typing in the search box keeps the settings that match, in their groups, open.
    #[test]
    fn a_search_finds_settings_by_their_label_and_description() {
        let mut dialog = tree(Flavor::Center);
        let Outcome::Send(sent) = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("echo".into()),
        }) else {
            panic_free_fail("no tree");
            return;
        };
        let Some(ToUi::DialogTree { items, .. }) = sent.first() else {
            panic_free_fail("no tree");
            return;
        };
        let texts: Vec<&str> = items.iter().map(|i| i.text.as_str()).collect();
        assert!(texts.contains(&"Typing Echo Characters"), "{texts:?}");
        assert!(texts.contains(&"Enable Mouse Echo"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.starts_with("Rate")), "{texts:?}");
        assert!(items.iter().filter(|i| i.level == 0).all(|i| i.open));
    }

    /// Quick Settings offers the settings for the application in front, without the place.
    #[test]
    fn quick_settings_has_no_place_box() {
        let spec = tree(Flavor::Quick).spec();
        assert_eq!(spec.title, "QuickSettings - Notepad");
        assert!(!spec.controls.iter().any(|c| c.id == PLACE));
        let items = tree_items(&spec);
        assert!(!items.iter().any(|i| i.text.starts_with("Rate")));
        // Quick Settings' own groups and names, as in Notepad.
        assert!(items
            .iter()
            .any(|i| i.text == "General Options" && i.level == 0));
        assert!(items.iter().any(|i| i.text == "User Verbosity Beginner"));
    }

    /// Settings Center's subgroups, as JAWS's: Reading in Web / HTML / PDFs, with the
    /// setting a level below, and JAWS's names for settings where they differ.
    #[test]
    fn settings_center_nests_jaws_subgroups() {
        let items = tree_items(&tree(Flavor::Center).spec());
        let at = |text: &str| items.iter().find(|i| i.text == text).map(|i| i.level);
        assert_eq!(at("Web / HTML / PDFs"), Some(0));
        assert_eq!(at("Reading"), Some(1));
        assert_eq!(
            at("Document and Web Pages Automatically Read When Loaded"),
            Some(2)
        );
        assert_eq!(at("Select Forms Mode Auto"), Some(1));
        assert!(items
            .iter()
            .any(|i| i.text.starts_with("Punctuation Level ")));
    }

    fn panic_free_fail(message: &str) {
        assert!(message.is_empty(), "{message}");
    }
}
