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

use super::settings::{Lifetime, Outcome, Place};
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
/// Ctrl+Shift+D's press: Settings Center on the Default settings.
const DEFAULT_PLACE: &str = "default_place";
/// Enter and Escape in the tree, which Okur decides: on a search's results, the whole tree
/// again; else OK, and the dialog closed.
const TREE_ENTER: &str = "tree_enter";
const TREE_ESCAPE: &str = "tree_escape";
/// Quick Settings' Shift+F10 and Applications key on a setting or group: its lifetime.
const LIFETIME: &str = "lifetime";
/// The lifetimes, as the menu lists them.
const LIFETIMES: [Lifetime; 3] = [
    Lifetime::UntilExit,
    Lifetime::UntilFocusMoves,
    Lifetime::Saved,
];

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
    WebLinks,
    FormsMode,
    TextProcessing,
    TextGeneral,
    TextNumbers,
    SpeechVerbosity,
    SpeechOptions,
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
    const CENTER: [TreeGroup; 17] = [
        Self::User,
        Self::Web,
        Self::WebReading,
        Self::WebLinks,
        Self::FormsMode,
        Self::TextProcessing,
        Self::TextGeneral,
        Self::TextNumbers,
        Self::SpeechVerbosity,
        Self::SpeechOptions,
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
            Self::WebReading | Self::WebLinks => Some(Self::Web),
            Self::TextGeneral | Self::TextNumbers => Some(Self::TextProcessing),
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
            Self::WebLinks => msg::settings_tree_links(),
            Self::TextNumbers => msg::settings_tree_numbers(),
            Self::SpeechOptions => msg::settings_tree_speech_options(),
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
        "identify_same_page_links" => TreeGroup::WebLinks,
        "voice.indicate_caps" | "live_region_append" | "voice.text.mixed_case" | "indentation" => {
            TreeGroup::TextGeneral
        }
        "voice.text.say_dollars" => TreeGroup::TextNumbers,
        "verbosity.level" => TreeGroup::SpeechVerbosity,
        "speech_mode" | "less_speech" => TreeGroup::SpeechOptions,
        "say_all_unit" => TreeGroup::SayAll,
        "voice.synthesizer" | "voice.voice" | "voice.rate" | "voice.volume" => {
            TreeGroup::Synthesizer
        }
        "voice.punctuation" => TreeGroup::Punctuation,
        "keyboard.layout" | "keyboard.okur_key" => TreeGroup::KeyboardGeneral,
        "mouse_echo.enabled"
        | "mouse_echo.interrupt"
        | "mouse_echo.instant"
        | "mouse_echo.unit"
        | "mouse_echo.description"
        | "mouse_echo.control_type_and_state" => TreeGroup::MouseEcho,
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

/// Settings changed lately, newest first, each with the place it was changed for: the
/// dialog's Last Changed Settings shows the place's 25 latest.
pub type History = Vec<(Scope, &'static str)>;

/// The most Last Changed Settings holds, as JAWS's: the oldest go first.
pub const HISTORY_LENGTH: usize = 25;

/// A row of the tree: a group, a setting in the group above it, or an item that opens a
/// dialog of its own.
#[derive(Debug, Clone, Copy)]
enum Row {
    Group(TreeGroup),
    Setting(&'static SettingSpec),
    /// Speech Verbosity's "Configure Verbosity Levels": Space opens its dialog.
    VerbosityLevels,
    /// The last group, Last Changed Settings.
    LastChanged,
    /// A setting changed lately, in Last Changed Settings, named with its place in the tree.
    Recent(&'static SettingSpec),
}

impl PartialEq for Row {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Group(a), Self::Group(b)) => a == b,
            (Self::Setting(a), Self::Setting(b)) => a.id == b.id,
            (Self::VerbosityLevels, Self::VerbosityLevels) => true,
            (Self::LastChanged, Self::LastChanged) => true,
            (Self::Recent(a), Self::Recent(b)) => a.id == b.id,
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
    /// The tree's item selected when the search began, Escape's place to go back to.
    before_search: Option<Row>,
    /// The settings changed lately, for every place (Last Changed Settings).
    history: History,
    /// Last Changed Settings is open.
    recent_open: bool,
    /// Quick Settings' settings that are to last a while only; the others are saved.
    lifetimes: Vec<(&'static str, Lifetime)>,
    /// The row whose lifetime menu is open.
    menu_for: Option<Row>,
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
        history: History,
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
            before_search: None,
            history,
            recent_open: false,
            lifetimes: Vec::new(),
            menu_for: None,
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
        spec.label.or(spec.tree_label)?;
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
        if let (None, Some(name)) = (spec.label, spec.tree_label) {
            return name().to_string();
        }
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
                .or(spec.tree_label)
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
        // Last Changed Settings, the last group, always there; the place's latest changes in
        // it, newest first, those found by a search.
        let recent: Vec<Row> = self
            .recent()
            .into_iter()
            .filter(|spec| matches(spec))
            .map(Row::Recent)
            .collect();
        if words.is_empty() || !recent.is_empty() {
            rows.push(Row::LastChanged);
            rows.extend(recent);
        }
        rows
    }

    /// The place's settings changed lately that this dialog shows, newest first, at most
    /// [`HISTORY_LENGTH`].
    fn recent(&self) -> Vec<&'static SettingSpec> {
        let scope = self.scope();
        let mut seen: Vec<&str> = Vec::new();
        self.history
            .iter()
            .filter(|(place, _)| *place == scope)
            .filter_map(|(_, id)| {
                let spec = SETTINGS.iter().find(|s| s.id == *id)?;
                self.group_of(spec)?;
                (!seen.contains(id)).then(|| {
                    seen.push(id);
                    spec
                })
            })
            .take(HISTORY_LENGTH)
            .collect()
    }

    /// A group's path in the tree, outermost first: "Web / HTML / PDFs,Reading".
    fn path_of(group: TreeGroup) -> String {
        let mut names = vec![group.label().to_string()];
        let mut up = group.parent();
        while let Some(parent) = up {
            names.insert(0, parent.label().to_string());
            up = parent.parent();
        }
        names.join(",")
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
            Row::LastChanged => TreeItem {
                text: msg::settings_tree_last_changed().to_string(),
                level: 0,
                open: !self.search.is_empty() || self.recent_open,
                checked: None,
            },
            // As the setting's own item, with its place: "Typing Echo Words (User)", as JAWS's
            // "Medium... (Web / HTML / PDFs ,Reading,Configure Web Verbosity Levels)" [obs, 1st
            // walk of Quick Settings, the redesign's §3.1].
            Row::Recent(spec) => {
                let mut item = self.item(&Row::Setting(spec));
                if let Some(group) = self.group_of(spec) {
                    item.text = format!("{} ({})", item.text, Self::path_of(group));
                }
                item.level = 1;
                item
            }
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
                // A setting the place cannot keep is unavailable here, as JAWS says of one kept
                // only for all applications: "Insert Key ModeUnavailable" [obs, the walk of
                // Settings Center].
                let available = spec.scopes.allows(&self.scope());
                let text = match (available, self.value_text(spec)) {
                    (false, _) => format!("{label} {}", msg::settings_tree_unavailable()),
                    (true, Some(value)) => format!("{label} {value}"),
                    (true, None) => label,
                };
                TreeItem {
                    text,
                    level,
                    open: true,
                    checked: checked.filter(|_| available),
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
            // Why it is unavailable, and the key that shows where it is kept.
            Some(Row::Setting(spec) | Row::Recent(spec)) if !spec.scopes.allows(&self.scope()) => {
                msg::settings_tree_default_only().to_string()
            }
            Some(Row::LastChanged) => msg::settings_tree_last_changed_description().to_string(),
            Some(Row::Setting(spec) | Row::Recent(spec)) => spec
                .label
                .or(spec.tree_label)
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
                // An edit, "SearchBox Edit" as JAWS's [obs, the walk of Settings Center].
                ControlKind::Edit {
                    text: self.search.clone(),
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
        // Ctrl+E to the search box from anywhere; Ctrl+Shift+D to the Default settings, in
        // Settings Center (JAWS's help, the redesign's §1).
        let shortcut = |key: &str, control: &str| okur_ui_model::DialogShortcut {
            key: key.to_owned(),
            control: control.to_owned(),
            from: None,
        };
        let from = |key: &str, control: &str, from: &str| okur_ui_model::DialogShortcut {
            from: Some(from.to_owned()),
            ..shortcut(key, control)
        };
        // Down from the search box into the tree; Enter and Escape in the tree for a search's
        // results (JAWS's Settings Center help).
        let mut keys = vec![
            shortcut("Ctrl+E", SEARCH),
            from("Down", TREE, SEARCH),
            from("Enter", TREE_ENTER, TREE),
            from("Escape", TREE_ESCAPE, TREE),
        ];
        if self.flavor == Flavor::Center {
            keys.push(shortcut("Ctrl+Shift+D", DEFAULT_PLACE));
        } else {
            // A setting's or group's lifetime (JAWS's Quick Settings help).
            keys.push(from("Shift+F10", LIFETIME, TREE));
            keys.push(from("Apps", LIFETIME, TREE));
        }
        DialogSpec {
            id: self.id,
            title: self.title(),
            controls,
            keys,
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
            Some(Row::Setting(spec) | Row::Recent(spec)) => spec,
            Some(Row::VerbosityLevels) => return Outcome::OpenVerbosityLevels,
            Some(Row::Group(_) | Row::LastChanged) | None => return Outcome::Nothing,
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

    /// Ends a search: the whole tree, the search box empty, at `at` with its groups open.
    fn end_search(&mut self, at: Option<Row>) -> Outcome {
        self.search.clear();
        self.rows = self.rows_for_search();
        if let Some(Row::Setting(spec) | Row::Recent(spec)) = at {
            let mut group = self.group_of(spec);
            while let Some(g) = group {
                if !self.open.contains(&g) {
                    self.open.push(g);
                }
                group = g.parent();
            }
        }
        self.selected = at
            .and_then(|at| self.rows.iter().position(|r| *r == at))
            .unwrap_or(0);
        let mut sent = vec![ToUi::DialogSet {
            id: self.id,
            control: SEARCH.to_owned(),
            value: Value::Text(String::new()),
        }];
        sent.extend(self.refill());
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

    /// The settings a row stands for: a setting, or a group's settings in it and its
    /// subgroups.
    fn settings_of(&self, row: Row) -> Vec<&'static SettingSpec> {
        match row {
            Row::Setting(spec) | Row::Recent(spec) => vec![spec],
            Row::Group(group) => self
                .shown()
                .filter(|(g, _)| g.within(group))
                .map(|(_, spec)| spec)
                .collect(),
            Row::VerbosityLevels | Row::LastChanged => Vec::new(),
        }
    }

    /// How long a setting's change is to last.
    fn lifetime(&self, spec: &SettingSpec) -> Lifetime {
        self.lifetimes
            .iter()
            .find(|(id, _)| *id == spec.id)
            .map_or(Lifetime::Saved, |(_, lifetime)| *lifetime)
    }

    /// The lifetime menu on the selected setting or group, its lifetime checked (a group's
    /// when all its settings share one).
    fn lifetime_menu(&mut self) -> Outcome {
        let Some(row) = self.rows.get(self.selected).copied() else {
            return Outcome::Nothing;
        };
        let settings = self.settings_of(row);
        let Some(first) = settings.first() else {
            return Outcome::Nothing;
        };
        let shared = self.lifetime(first);
        let checked = settings
            .iter()
            .all(|spec| self.lifetime(spec) == shared)
            .then_some(shared);
        self.menu_for = Some(row);
        let label = |lifetime: Lifetime| match lifetime {
            Lifetime::UntilExit => msg::settings_lifetime_until_exit(),
            Lifetime::UntilFocusMoves => msg::settings_lifetime_until_focus_moves(),
            Lifetime::Saved => msg::settings_lifetime_saved(),
        };
        Outcome::Send(vec![ToUi::DialogMenu {
            id: self.id,
            control: TREE.to_owned(),
            items: LIFETIMES
                .iter()
                .map(|&lifetime| okur_ui_model::MenuEntry {
                    text: label(lifetime).to_string(),
                    checked: checked == Some(lifetime),
                })
                .collect(),
        }])
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
            temporary: self.lifetimes.clone(),
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
                        match self.rows.get(*index).copied() {
                            Some(Row::Group(group)) => {
                                self.open.retain(|g| *g != group);
                                if *open {
                                    self.open.push(group);
                                }
                            }
                            Some(Row::LastChanged) => self.recent_open = *open,
                            _ => {}
                        }
                        Outcome::Nothing
                    }
                    (SEARCH, Value::Text(text)) => {
                        if self.search.is_empty() {
                            self.before_search = self.rows.get(self.selected).copied();
                        }
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
            // A lifetime chosen for the menu's setting or group.
            FromUi::DialogMenuChosen { id, index, .. } if *id == self.id => {
                let (Some(row), Some(&lifetime)) =
                    (self.menu_for.take(), index.and_then(|i| LIFETIMES.get(i)))
                else {
                    return Outcome::Nothing;
                };
                for spec in self.settings_of(row) {
                    self.lifetimes.retain(|(id, _)| *id != spec.id);
                    if lifetime != Lifetime::Saved {
                        self.lifetimes.push((spec.id, lifetime));
                    }
                }
                Outcome::Nothing
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
                // On a search's results Enter shows the whole tree at the setting found, Escape
                // at the item before the search; elsewhere they are OK and Cancel.
                TREE_ENTER if !self.search.is_empty() => {
                    let at = self.rows.get(self.selected).copied();
                    self.end_search(at)
                }
                TREE_ENTER => self.apply(true),
                TREE_ESCAPE if !self.search.is_empty() => {
                    let at = self.before_search;
                    self.end_search(at)
                }
                TREE_ESCAPE => Outcome::Close,
                LIFETIME if self.flavor == Flavor::Quick => self.lifetime_menu(),
                DEFAULT_PLACE if self.flavor == Flavor::Center => {
                    self.place = 0;
                    self.draft = self
                        .places
                        .first()
                        .map(|p| p.settings.clone())
                        .unwrap_or_default();
                    let mut sent = self.refill();
                    sent.push(ToUi::DialogSet {
                        id: self.id,
                        control: PLACE.to_owned(),
                        value: Value::Selected(0),
                    });
                    Outcome::Send(sent)
                }
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
        SettingsTree::new(
            7,
            flavor,
            places,
            1,
            vec!["Zira".into()],
            vec![
                (Scope::Application("notepad".into()), "typing_echo"),
                (Scope::Global, "voice.rate"),
            ],
        )
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

    /// Enter on a search's result shows the whole tree at it, its groups open; Escape on the
    /// results goes back to the item before the search (JAWS's Settings Center help).
    #[test]
    fn enter_and_escape_end_a_search() {
        let mut dialog = tree(Flavor::Center);
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("dollars".into()),
        });
        let found = dialog.rows.get(dialog.selected).copied();
        assert!(matches!(found, Some(Row::Setting(spec)) if spec.id == "voice.text.say_dollars"));
        dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: TREE_ENTER.into(),
        });
        assert!(dialog.search.is_empty());
        assert_eq!(dialog.rows.get(dialog.selected).copied(), found);
        let items = dialog.items();
        let open = |text: &str| items.iter().any(|i| i.text == text && i.open);
        assert!(open("Text Processing") && open("Number And Date Processing"));
        // Escape on a search's results: back where it began.
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("echo".into()),
        });
        dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: TREE_ESCAPE.into(),
        });
        assert_eq!(dialog.rows.get(dialog.selected).copied(), found);
        // Without a search, Escape closes and Enter is OK.
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: TREE_ESCAPE.into(),
            }),
            Outcome::Close
        );
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
        // Settings the settings dialog does not show, by their names in JAWS's tree.
        assert_eq!(at("Number And Date Processing"), Some(1));
        assert_eq!(at("Speak Dollars"), Some(2));
        assert_eq!(at("Speech Mode Full Speech"), Some(1));
        assert_eq!(at("Identify \"Same Page\" Links"), Some(2));
    }

    fn panic_free_fail(message: &str) {
        assert!(message.is_empty(), "{message}");
    }

    /// Last Changed Settings is the last group: the place's latest changes, newest first,
    /// each named with its value and its place in the tree, changeable there.
    #[test]
    fn last_changed_settings_holds_the_places_latest_changes() {
        let mut dialog = tree(Flavor::Center);
        let items = dialog.items();
        let last = items.iter().position(|i| i.text == "Last Changed Settings");
        assert_eq!(
            last.and_then(|i| items.get(i + 1)).map(|i| i.text.as_str()),
            Some("Typing Echo Characters (User)")
        );
        // Notepad's history only: the rate was changed for all applications.
        assert!(!items
            .iter()
            .any(|i| i.text.starts_with("Rate") && i.text.ends_with(')')));
        let index = last.map_or(0, |i| i + 1);
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: TREE.into(),
            value: Value::Selected(index),
        });
        dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::Change,
        });
        assert_eq!(
            registry::get(dialog.draft(), "typing_echo"),
            Some(Json::String("words".into()))
        );
    }

    /// Quick Settings' Shift+F10 on a setting offers its lifetime, Saved checked at first; a
    /// change made with another is applied as temporary.
    #[test]
    fn a_quick_setting_can_last_until_okur_exits() {
        let mut dialog = tree(Flavor::Quick);
        let typing = dialog
            .rows
            .iter()
            .position(|r| matches!(r, Row::Setting(spec) if spec.id == "typing_echo"))
            .unwrap_or_default();
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: TREE.into(),
            value: Value::Selected(typing),
        });
        let Outcome::Send(sent) = dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: LIFETIME.into(),
        }) else {
            panic_free_fail("no menu");
            return;
        };
        let Some(ToUi::DialogMenu { items, .. }) = sent.first() else {
            panic_free_fail("no menu");
            return;
        };
        let checked: Vec<bool> = items.iter().map(|i| i.checked).collect();
        assert_eq!(checked, [false, false, true]);
        dialog.handle(&FromUi::DialogMenuChosen {
            id: 7,
            control: TREE.into(),
            index: Some(0),
        });
        dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::Change,
        });
        let applied = dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: "ok".into(),
        });
        assert!(matches!(
            applied,
            Outcome::Apply { ref temporary, .. }
                if temporary == &[("typing_echo", Lifetime::UntilExit)]
        ));
    }

    /// A setting kept only for all applications is unavailable in an application's settings,
    /// with JAWS's reason, and available again on the Default settings.
    #[test]
    fn a_default_only_setting_is_unavailable_for_an_application() {
        let mut dialog = tree(Flavor::Center);
        let key = |dialog: &SettingsTree| {
            tree_items(&dialog.spec())
                .into_iter()
                .find(|i| i.text.starts_with("Okur Key"))
                .map(|i| i.text)
        };
        assert_eq!(key(&dialog).as_deref(), Some("Okur Key Unavailable"));
        let index = dialog
            .rows
            .iter()
            .position(|r| matches!(r, Row::Setting(spec) if spec.id == "keyboard.okur_key"))
            .unwrap_or_default();
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: TREE.into(),
            value: Value::Selected(index),
        });
        assert!(dialog
            .description()
            .starts_with("Only available when modifying the Default"));
        assert_eq!(
            dialog.handle(&FromUi::DialogKey {
                id: 7,
                control: TREE.into(),
                key: DialogKey::Change,
            }),
            Outcome::Nothing
        );
        dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: DEFAULT_PLACE.into(),
        });
        assert_eq!(key(&dialog).as_deref(), Some("Okur Key Insert"));
    }

    /// Ctrl+E goes to the search box from anywhere; in Settings Center, Ctrl+Shift+D shows the
    /// Default settings, its Application box on them.
    #[test]
    fn ctrl_shift_d_goes_to_the_default_settings() {
        let mut dialog = tree(Flavor::Center);
        let keys: Vec<(String, String)> = dialog
            .spec()
            .keys
            .into_iter()
            .map(|k| (k.key, k.control))
            .collect();
        assert!(keys.contains(&("Ctrl+E".to_owned(), SEARCH.to_owned())));
        assert!(keys.contains(&("Ctrl+Shift+D".to_owned(), DEFAULT_PLACE.to_owned())));
        let Outcome::Send(sent) = dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: DEFAULT_PLACE.into(),
        }) else {
            panic_free_fail("no change");
            return;
        };
        assert!(sent.contains(&ToUi::DialogSet {
            id: 7,
            control: PLACE.into(),
            value: Value::Selected(0),
        }));
        assert_eq!(dialog.scope(), Scope::Global);
        // Quick Settings has no Default.
        assert!(!tree(Flavor::Quick)
            .spec()
            .keys
            .iter()
            .any(|k| k.control == DEFAULT_PLACE));
    }
}
