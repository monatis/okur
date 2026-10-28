//! Settings Center (Insert+6) and Quick Settings (Insert+V), as JAWS's: a search box over a
//! tree of groups and settings, each setting named with its value ("Typing Echo Characters"),
//! Space changing it and Shift+Space back, the selected item's description below the tree, then
//! OK, Cancel, Apply once something changed, and in Settings Center the place the settings are
//! kept (`docs/references/okur-settings-redesign.md` §3.1). While the search box has words, a
//! list box of the settings found is in the tree's place, as JAWS's. Generated from the settings table
//! ([`crate::settings::registry`]): a setting added there with a label is here too.
//!
//! The dialog keeps a draft of the settings, as [`super::settings::SettingsDialog`] does; Apply
//! and OK make it Okur's settings, Cancel drops what was not applied; Settings Center's asks
//! first whether to save it.

use okur_ui_model::{Control, ControlKind, DialogKey, DialogSpec, FromUi, ToUi, TreeItem, Value};

use super::settings::{Lifetime, Outcome, Place};
use crate::aom::Role;
use crate::event::FocusEvent;
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
/// Quick Settings' "Expand Tree View" check box.
const EXPAND: &str = "expand";
/// Ctrl+Shift+D's press: Settings Center on the Default settings.
const DEFAULT_PLACE: &str = "default_place";
/// Enter and Escape in the tree, which Okur decides: on a search's results, the whole tree
/// again; else OK, and the dialog closed.
const TREE_ENTER: &str = "tree_enter";
const TREE_ESCAPE: &str = "tree_escape";
/// Quick Settings' Shift+F10 and Applications key on a setting or group: its lifetime.
const LIFETIME: &str = "lifetime";
/// F6 in the tree: the focus to the setting's control in the configuration display pane.
const PANE_KEY: &str = "pane";
/// The search's results: a list box in the tree's place while the search box has words ("11
/// Search Results List box" [obs, the walk of Settings Center's search]).
const RESULTS: &str = "results";
/// Down in the search box: into the results, or into the tree without a search.
const SEARCH_DOWN: &str = "search_down";
/// F6 elsewhere than the tree: back to the tree, or to the results in its place.
const TO_LIST: &str = "to_list";
/// Enter, Escape, and Space or Shift+Space on a result.
const RESULT_ENTER: &str = "result_enter";
const RESULT_ESCAPE: &str = "result_escape";
const RESULT_CHANGE: &str = "result_change";
/// A pane control's id: this, then its setting's.
const PANE_PREFIX: &str = "pane:";
/// The lifetimes, as the menu lists them.
const LIFETIMES: [Lifetime; 3] = [
    Lifetime::UntilExit,
    Lifetime::UntilFocusMoves,
    Lifetime::Saved,
];

/// A focus on one of the search's results, as Okur says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultFocus {
    /// The result with its group's path after it, without its place in the list, entered with
    /// the list's own hints: "11 Search Results List box | Typing Echo Off | User | Use the
    /// arrow keys to select an item. | Press the space bar to toggle between the choices" [obs,
    /// the walk of Settings Center's search].
    Detailed(Box<FocusEvent>),
    /// The result whose value Space just changed, its item made anew: Space has said it.
    Unsaid,
}

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
    WebNavigation,
    WebLinks,
    WebHeadings,
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
    const CENTER: [TreeGroup; 19] = [
        Self::User,
        Self::Web,
        Self::WebReading,
        Self::WebNavigation,
        Self::WebLinks,
        Self::WebHeadings,
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
            Self::WebReading | Self::WebNavigation | Self::WebLinks | Self::WebHeadings => {
                Some(Self::Web)
            }
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
            Self::WebNavigation => msg::settings_tree_navigation(),
            Self::WebHeadings => msg::settings_tree_headings_and_frames(),
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
        "typing_echo" | "viewer_display" | "screen_echo" => TreeGroup::User,
        "smart_navigation" => TreeGroup::WebNavigation,
        "heading_indication" => TreeGroup::WebHeadings,
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

/// Last Changed Settings as kept between runs: each change's place and setting, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedHistory {
    #[serde(default)]
    pub changed: Vec<SavedChange>,
}

/// A change kept: for one application, one site, or (neither) all applications.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedChange {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub application: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site: Option<String>,
    pub setting: String,
}

impl SavedHistory {
    /// The history to keep: the session's changes are not kept, as its settings are not.
    pub fn of(history: &History) -> Self {
        let changed = history
            .iter()
            .filter_map(|(place, id)| {
                let (application, site) = match place {
                    Scope::Global => (None, None),
                    Scope::Application(name) => (Some(name.clone()), None),
                    Scope::Site(host) => (None, Some(host.clone())),
                    Scope::Session => return None,
                };
                Some(SavedChange {
                    application,
                    site,
                    setting: (*id).to_owned(),
                })
            })
            .collect();
        Self { changed }
    }

    /// The history kept, without the settings Okur no longer has.
    pub fn history(&self) -> History {
        self.changed
            .iter()
            .filter_map(|change| {
                let spec = SETTINGS.iter().find(|s| s.id == change.setting)?;
                let place = match (&change.application, &change.site) {
                    (_, Some(host)) => Scope::Site(host.clone()),
                    (Some(name), None) => Scope::Application(name.clone()),
                    (None, None) => Scope::Global,
                };
                Some((place, spec.id))
            })
            .collect()
    }
}

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
    /// The words searched for; empty, no search.
    search: String,
    /// The settings the search found, in the tree's order, while the search box has words.
    results: Vec<Row>,
    /// The result selected.
    result: usize,
    /// A result whose value Space just changed, by its new text: its list box item, made anew,
    /// takes the focus again.
    renamed: Option<String>,
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
    /// The question whether to save the changes, while it is open, by its dialog's id.
    question: Option<u32>,
    /// Quick Settings' "Expand Tree View": every group open.
    expand: bool,
    /// The configuration display pane shown (Settings Center's): its group, and the settings
    /// it was shown with.
    pane: Option<(TreeGroup, Settings)>,
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
            results: Vec::new(),
            result: 0,
            renamed: None,
            history,
            recent_open: false,
            lifetimes: Vec::new(),
            menu_for: None,
            open: Vec::new(),
            rows: Vec::new(),
            selected: 0,
            changed: false,
            question: None,
            expand: flavor == Flavor::Quick,
            pane: None,
        };
        if tree.expand {
            tree.expand_all(true);
        }
        tree.rows = tree.tree_rows();
        tree
    }

    /// Every group opened, or closed (Quick Settings' "Expand Tree View" checked or cleared);
    /// closed, a setting selected gives its place to its group at the top.
    fn expand_all(&mut self, open: bool) {
        self.open = if open {
            match self.flavor {
                Flavor::Center => TreeGroup::CENTER.to_vec(),
                Flavor::Quick => TreeGroup::QUICK.to_vec(),
            }
        } else {
            Vec::new()
        };
        self.recent_open = open;
        if !open {
            let top = self.rows.get(..=self.selected).and_then(|rows| {
                rows.iter().rposition(|r| match r {
                    Row::Group(group) => group.depth() == 0,
                    Row::LastChanged => true,
                    _ => false,
                })
            });
            self.selected = top.unwrap_or(0);
        }
    }

    /// Whether dialog `id` is this one or its question.
    pub fn owns(&self, id: u32) -> bool {
        id == self.id || self.question == Some(id)
    }

    /// The question whether to save the changes, while it is open.
    pub fn question(&self) -> Option<u32> {
        self.question
    }

    /// The question asked when Settings Center is cancelled with changes not applied, as
    /// dialog `id`: JAWS's "JAWS Settings Center dialog | You have made changes to NotePad's
    /// settings. Do you want to save them? | Yes Button", then No and Cancel; Cancel or Escape
    /// goes back to Settings Center [obs, the walks of Settings Center's questions].
    pub fn save_question(&mut self, id: u32) -> DialogSpec {
        self.question = Some(id);
        let place = self
            .places
            .get(self.place)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        let control = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
            hidden: false,
        };
        DialogSpec {
            id,
            title: msg::settings_save_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::settings_save_question(place.as_str()).render(),
                    },
                ),
                control(
                    "yes",
                    msg::settings_save_yes().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "no",
                    msg::settings_save_no().to_string(),
                    ControlKind::Button { default: false },
                ),
                // Escape's: back to Settings Center [obs, the second walk of Settings
                // Center's questions].
                control(
                    "cancel",
                    msg::settings_save_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// Cancel: the dialog closes, or Settings Center asks first whether to save what changed.
    fn cancel(&self) -> Outcome {
        if self.changed && self.flavor == Flavor::Center {
            Outcome::AskToSave
        } else {
            Outcome::Close
        }
    }

    /// What a search says as each key is typed: the first setting found and how many there
    /// are, "Screen Echo | 1 of 6 Search Results", or "No matches found." [obs, the walk of
    /// Settings Center's search]. Nothing once the search box is empty.
    fn search_said(&self) -> Option<String> {
        if self.search.is_empty() {
            return None;
        }
        let first = self.results.first().map(|row| match row {
            Row::Setting(spec) | Row::Recent(spec) => self.tree_label(spec),
            other => self.item(other).text,
        });
        Some(match first {
            Some(first) => format!(
                "{first}\n{}",
                msg::settings_search_results(self.results.len()).render()
            ),
            None => msg::settings_search_none().to_string(),
        })
    }

    /// The settings the search finds, in the tree's order, each once (Last Changed Settings
    /// holds some again), and Configure Verbosity Levels.
    fn found(&self) -> Vec<Row> {
        if self.search.is_empty() {
            return Vec::new();
        }
        let mut found: Vec<Row> = Vec::new();
        for row in self.rows_matching(&self.search) {
            let row = match row {
                Row::Recent(spec) => Row::Setting(spec),
                Row::Group(_) | Row::LastChanged => continue,
                row => row,
            };
            if !found.contains(&row) {
                found.push(row);
            }
        }
        found
    }

    /// A result as the list shows it: the setting and its value, a check box's state in words
    /// ("Allow Mouse Echo not checked" [obs, the walk of Settings Center's search]).
    fn result_text(&self, row: &Row) -> String {
        let item = self.item(row);
        match item.checked {
            Some(true) => format!("{} {}", item.text, msg::state_checked()),
            Some(false) => format!("{} {}", item.text, msg::state_not_checked()),
            None => item.text,
        }
    }

    fn result_texts(&self) -> Vec<String> {
        self.results
            .iter()
            .map(|row| self.result_text(row))
            .collect()
    }

    /// The group a row is in.
    fn row_group(&self, row: &Row) -> Option<TreeGroup> {
        match row {
            Row::Setting(spec) | Row::Recent(spec) => self.group_of(spec),
            Row::VerbosityLevels => Some(TreeGroup::SpeechVerbosity),
            Row::Group(group) => group.parent(),
            Row::LastChanged => None,
        }
    }

    /// A result's group, then the groups that one is in: "Navigation Web / HTML / PDFs", said
    /// after the result. JAWS runs the names together ("Smart Navigation Mode Off |
    /// NavigationWeb / HTML / PDFs" [obs, the walk of Settings Center's search]); Okur keeps a
    /// space between them.
    fn result_path(&self, row: &Row) -> String {
        let mut names = Vec::new();
        let mut group = self.row_group(row);
        while let Some(g) = group {
            names.push(g.label().to_string());
            group = g.parent();
        }
        names.join(" ")
    }

    /// The item selected: the result while there is a search, else the tree's.
    fn current(&self) -> Option<Row> {
        if self.search.is_empty() {
            self.rows.get(self.selected).copied()
        } else {
            self.results.get(self.result).copied()
        }
    }

    /// The list that stands in the tree's place: the results while there is a search.
    fn list(&self) -> &'static str {
        if self.search.is_empty() {
            TREE
        } else {
            RESULTS
        }
    }

    /// The results' list box filled, and named by their count.
    fn results_shown(&self) -> Vec<ToUi> {
        vec![
            ToUi::DialogItems {
                id: self.id,
                control: RESULTS.to_owned(),
                items: self.result_texts(),
                selected: self.result,
            },
            ToUi::DialogLabel {
                id: self.id,
                control: RESULTS.to_owned(),
                label: msg::settings_search_results_list(self.results.len()).render(),
            },
        ]
    }

    /// The results shown in the tree's place, or the tree in theirs.
    fn swap(&self, results: bool) -> ToUi {
        let (show, hide) = if results {
            (RESULTS, TREE)
        } else {
            (TREE, RESULTS)
        };
        ToUi::DialogSwap {
            id: self.id,
            show: show.to_owned(),
            hide: hide.to_owned(),
        }
    }

    /// The description of the item selected, shown below the list.
    fn description_shown(&self) -> ToUi {
        ToUi::DialogSet {
            id: self.id,
            control: DESCRIPTION.to_owned(),
            value: Value::Text(self.description()),
        }
    }

    /// A focus on one of the search's results, as Okur says it; `None` for any other focus.
    pub fn result_focus(&mut self, focus: &FocusEvent) -> Option<ResultFocus> {
        if self.search.is_empty()
            || focus.role != Role::ListItem
            || focus.window_title.as_deref() != Some(self.title().as_str())
        {
            return None;
        }
        let name = focus.name.as_deref()?;
        if self.renamed.take().as_deref() == Some(name) {
            return Some(ResultFocus::Unsaid);
        }
        let row = self
            .results
            .iter()
            .find(|row| self.result_text(row) == name)?;
        let mut detailed = focus.clone();
        detailed.columns = vec![self.result_path(row)];
        detailed.item_index = None;
        detailed.item_count = None;
        detailed.own_hints = vec![
            msg::hint_select_item_arrows().to_string(),
            msg::hint_space_toggle_choices().to_string(),
        ];
        Some(ResultFocus::Detailed(Box::new(detailed)))
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

    /// Another place's settings (the Application box, Ctrl+Shift+D): its draft, and its rows,
    /// Last Changed Settings among them; the previous place's stayed (performance review
    /// 2026-10-25, finding 16).
    fn change_place(&mut self, place: usize) {
        self.place = place;
        self.draft = self
            .places
            .get(place)
            .map(|p| p.settings.clone())
            .unwrap_or_default();
        self.rows = self.tree_rows();
        self.selected = self.selected.min(self.rows.len().saturating_sub(1));
        self.results = self.found();
        self.result = self.result.min(self.results.len().saturating_sub(1));
    }

    /// The tree's rows: every group and its settings.
    fn tree_rows(&self) -> Vec<Row> {
        self.rows_matching("")
    }

    /// The rows for the words `search`: every group and its settings, or only the settings
    /// that match, in their groups.
    fn rows_matching(&self, search: &str) -> Vec<Row> {
        let words: Vec<String> = search.split_whitespace().map(str::to_lowercase).collect();
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
            Kind::Check { .. } | Kind::Structured => None,
        }
    }

    /// A row as the tree shows it.
    /// The tree's items that differ from `before`, each set anew: a change can bring another
    /// with it (a layout its Okur key). Each row was sent at every Space, and okur-ui walked
    /// its whole tree for each (performance review 2026-10-25, finding 8).
    fn items_changed(&self, before: &[TreeItem]) -> Vec<ToUi> {
        self.items()
            .into_iter()
            .enumerate()
            .filter(|(index, item)| before.get(*index) != Some(item))
            .map(|(index, item)| ToUi::DialogSet {
                id: self.id,
                control: TREE.to_owned(),
                value: Value::TreeItem(index, item),
            })
            .collect()
    }

    fn item(&self, row: &Row) -> TreeItem {
        match row {
            Row::LastChanged => TreeItem {
                text: msg::settings_tree_last_changed().to_string(),
                level: 0,
                open: self.recent_open,
                checked: None,
            },
            // As the setting's own item, with its place: "Typing Echo Words (User)", as JAWS's
            // "Medium... (Web / HTML / PDFs ,Reading,Configure Web Verbosity Levels)" [obs, 1st
            // walk of Quick Settings, the redesign's §3.1].
            Row::Recent(spec) => {
                // The label, its group's path, then the value: "Typing Echo (User) Characters"
                // [obs, the walk of Last Changed Settings].
                let mut item = self.item(&Row::Setting(spec));
                if let Some(group) = self.group_of(spec) {
                    let label = self.tree_label(spec);
                    let value = item.text.strip_prefix(label.as_str()).unwrap_or_default();
                    item.text = format!("{label} ({}){value}", Self::path_of(group));
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
                open: self.open.contains(group),
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
        match self.current().as_ref() {
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
            hidden: false,
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
            Control {
                hidden: !self.search.is_empty(),
                ..control(
                    TREE,
                    String::new(),
                    ControlKind::Tree {
                        items: self.items(),
                        selected: self.selected,
                    },
                )
            },
            // The search's results, in the tree's place while there is a search.
            Control {
                hidden: self.search.is_empty(),
                ..control(
                    RESULTS,
                    msg::settings_search_results_list(self.results.len()).render(),
                    ControlKind::ListBox {
                        items: self.result_texts(),
                        selected: self.result,
                    },
                )
            },
            control(
                DESCRIPTION,
                String::new(),
                ControlKind::Note {
                    text: self.description(),
                },
            ),
        ];
        let ok = control(
            "ok",
            msg::settings_ok().to_string(),
            ControlKind::Button { default: true },
        );
        match self.flavor {
            // Apply, available once something changed, comes before OK and Cancel; the place
            // last [obs, the walk of Settings Center's questions].
            Flavor::Center => controls.extend([
                Control {
                    enabled: self.changed,
                    ..control(
                        "apply",
                        msg::settings_apply().to_string(),
                        ControlKind::Button { default: false },
                    )
                },
                ok,
                control(
                    "cancel",
                    msg::settings_cancel().to_string(),
                    ControlKind::Button { default: false },
                ),
                control(
                    PLACE,
                    msg::settings_tree_application().to_string(),
                    ControlKind::Choice {
                        options: self.places.iter().map(|p| p.name.clone()).collect(),
                        selected: self.place,
                    },
                ),
            ]),
            // "Expand Tree View check box checked", then OK: no Apply and no Cancel [obs, the
            // second walk of Settings Center's questions].
            Flavor::Quick => controls.extend([
                control(
                    EXPAND,
                    msg::quick_settings_expand().to_string(),
                    ControlKind::CheckBox {
                        checked: self.expand,
                    },
                ),
                ok,
            ]),
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
        // Down from the search box into the results, or the tree; Enter, Escape and Space on
        // a result (JAWS's Settings Center help, the walk of its search). F6 from the tree to
        // the setting's control in the pane, and from anywhere else back to the tree, or the
        // results in its place (JAWS's Settings Center help).
        let mut keys = vec![
            from("F6", PANE_KEY, TREE),
            shortcut("F6", TO_LIST),
            shortcut("Ctrl+E", SEARCH),
            from("Down", SEARCH_DOWN, SEARCH),
            from("Enter", TREE_ENTER, TREE),
            from("Escape", TREE_ESCAPE, TREE),
            from("Enter", RESULT_ENTER, RESULTS),
            from("Escape", RESULT_ESCAPE, RESULTS),
            from("Space", RESULT_CHANGE, RESULTS),
            from("Shift+Space", RESULT_CHANGE, RESULTS),
        ];
        if self.flavor == Flavor::Center {
            keys.push(shortcut("Ctrl+Shift+D", DEFAULT_PLACE));
            // Escape, Cancel and the close box are Okur's to decide: the question first when
            // something changed.
            keys.push(shortcut("Escape", "cancel"));
        }
        // The item's context menu: Last Changed Settings' Remove and Remove All in both, and
        // in Quick Settings a setting's or group's lifetime (JAWS's Quick Settings help).
        keys.push(from("Shift+F10", LIFETIME, TREE));
        keys.push(from("Apps", LIFETIME, TREE));
        DialogSpec {
            id: self.id,
            title: self.title(),
            controls,
            keys,
        }
    }

    /// The setting `spec`'s next value (`back`: its previous), wrapping around; `None` for one
    /// the tree cannot change, or the place cannot keep.
    fn next_value(&self, spec: &SettingSpec) -> Option<Json> {
        if !spec.scopes.allows(&self.scope()) {
            return None;
        }
        let value = registry::get(&self.draft, spec.id).unwrap_or(Json::Null);
        let step = |values: Vec<Json>| -> Option<Json> {
            let count = values.len();
            let at = values.iter().position(|v| *v == value).unwrap_or(0);
            let next = (at + 1) % count.max(1);
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
                let next = now.saturating_add(step);
                let next = if next > max { min } else { next };
                Some(Json::from(next))
            }
            Kind::Structured => None,
        }
    }

    /// Changes the selected setting's value: its item and the Apply button are updated.
    fn change(&mut self) -> Outcome {
        let spec = match self.rows.get(self.selected).copied() {
            Some(Row::Setting(spec) | Row::Recent(spec)) => spec,
            Some(Row::VerbosityLevels) => return Outcome::OpenVerbosityLevels,
            Some(Row::Group(_) | Row::LastChanged) | None => return Outcome::Nothing,
        };
        self.change_setting(spec)
    }

    /// Space on a result: its value changed as in the tree, the results' texts with it, and
    /// the value said with its place ("Play Sound | 4 of 5" [obs, the walk of Settings
    /// Center's search]).
    fn change_result(&mut self) -> Outcome {
        let spec = match self.results.get(self.result).copied() {
            Some(Row::Setting(spec) | Row::Recent(spec)) => spec,
            Some(Row::VerbosityLevels) => return Outcome::OpenVerbosityLevels,
            Some(Row::Group(_) | Row::LastChanged) | None => return Outcome::Nothing,
        };
        let before = self.result_texts();
        let Outcome::Changed(mut sent, said) = self.change_setting(spec) else {
            return Outcome::Nothing;
        };
        for (index, text) in self.result_texts().into_iter().enumerate() {
            if before.get(index) == Some(&text) {
                continue;
            }
            if index == self.result {
                self.renamed = Some(text.clone());
            }
            sent.push(ToUi::DialogSet {
                id: self.id,
                control: RESULTS.to_owned(),
                value: Value::ItemRenamed(index, text),
            });
        }
        Outcome::Changed(sent, said)
    }

    /// `spec` set to its next value: the tree's items that show it anew, Apply available, and
    /// the value said.
    fn change_setting(&mut self, spec: &'static SettingSpec) -> Outcome {
        let Some(value) = self.next_value(spec) else {
            return Outcome::Nothing;
        };
        let before = self.items();
        if !registry::set(&mut self.draft, spec.id, value) {
            return Outcome::Nothing;
        }
        self.changed = true;
        let mut sent = self.items_changed(&before);
        sent.push(ToUi::DialogEnable {
            id: self.id,
            control: "apply".to_owned(),
            enabled: true,
        });
        Outcome::Changed(sent, self.value_said(spec))
    }

    /// A setting's value as Space says it: the value and its place among the values,
    /// "Characters | 2 of 4" [obs, the walk of Settings Center's questions]; a check box's
    /// state.
    fn value_said(&self, spec: &SettingSpec) -> String {
        let value = registry::get(&self.draft, spec.id).unwrap_or(Json::Null);
        let place = |values: Vec<Json>| {
            values
                .iter()
                .position(|v| *v == value)
                .map(|i| msg::position(i + 1, values.len()).render())
        };
        let text = self.value_text(spec).unwrap_or_default();
        let position = match spec.kind {
            Kind::Choice(choices) => place(choices().into_iter().map(|c| c.value).collect()),
            Kind::Voice => place(
                std::iter::once(Json::Null)
                    .chain(self.voices.iter().map(|v| Json::String(v.clone())))
                    .collect(),
            ),
            Kind::Check { on, .. } => {
                return if value == on.value() {
                    msg::state_checked().to_string()
                } else {
                    msg::state_not_checked().to_string()
                };
            }
            Kind::Range { .. } | Kind::Structured => None,
        };
        match position {
            Some(position) => format!(
                "{text}
{position}"
            ),
            None => text,
        }
    }

    /// Ends a search: the search box empty and the tree back in the results' place, at `at`
    /// with its groups open (Enter on a result), or where it was (Escape); "Search edit box
    /// cleared." said before it [obs, the walk of Settings Center's search].
    fn end_search(&mut self, at: Option<Row>) -> Outcome {
        self.search.clear();
        self.results.clear();
        self.result = 0;
        self.renamed = None;
        if let Some(at) = at {
            let mut group = self.row_group(&at);
            while let Some(g) = group {
                if !self.open.contains(&g) {
                    self.open.push(g);
                }
                group = g.parent();
            }
            self.rows = self.tree_rows();
            self.selected = self
                .rows
                .iter()
                .position(|r| *r == at)
                .unwrap_or(self.selected);
        }
        let mut sent = vec![ToUi::DialogSet {
            id: self.id,
            control: SEARCH.to_owned(),
            value: Value::Text(String::new()),
        }];
        sent.extend(self.refill());
        sent.push(self.swap(false));
        sent.push(ToUi::DialogFocus {
            id: self.id,
            control: TREE.to_owned(),
        });
        Outcome::BeforeFocus(sent, msg::settings_search_cleared().to_string())
    }

    /// The whole tree again, and the results while there is a search, after a search or
    /// another place.
    fn refill(&self) -> Vec<ToUi> {
        let mut sent = vec![ToUi::DialogTree {
            id: self.id,
            control: TREE.to_owned(),
            items: self.items(),
            selected: self.selected,
        }];
        if !self.search.is_empty() {
            sent.extend(self.results_shown());
        }
        sent.push(self.description_shown());
        sent
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

    /// Shift+F10 on the selected item: Last Changed Settings' Remove All, unavailable while it
    /// is empty, and an entry's Remove [obs, the walk of Last Changed Settings]; in Quick
    /// Settings a setting's or group's lifetime.
    fn context_menu(&mut self) -> Outcome {
        let row = self.rows.get(self.selected).copied();
        let entry = |text: StaticMessage, enabled: bool| okur_ui_model::MenuEntry {
            text: text.to_string(),
            checked: false,
            enabled,
        };
        let items = match row {
            Some(Row::LastChanged) => vec![entry(
                msg::settings_last_changed_remove_all(),
                !self.recent().is_empty(),
            )],
            Some(Row::Recent(_)) => vec![entry(msg::settings_last_changed_remove(), true)],
            _ if self.flavor == Flavor::Quick => return self.lifetime_menu(),
            _ => return Outcome::Nothing,
        };
        self.menu_for = row;
        Outcome::Send(vec![ToUi::DialogMenu {
            id: self.id,
            control: TREE.to_owned(),
            items,
        }])
    }

    /// Remove (an entry's `setting`) or Remove All (none) in Last Changed Settings: forgotten
    /// at once for the place. After Remove All the group keeps the focus, as JAWS's ("Last
    /// Changed Settings | open"); after Remove, the entry that took its place, else the group.
    fn forget(&mut self, setting: Option<&'static str>) -> Outcome {
        let scope = self.scope();
        self.history.retain(|(place, id)| {
            !(*place == scope && setting.is_none_or(|setting| setting == *id))
        });
        let at = self.selected;
        self.rows = self.tree_rows();
        let group = self
            .rows
            .iter()
            .position(|row| *row == Row::LastChanged)
            .unwrap_or_default();
        self.selected = match (setting, self.rows.get(at)) {
            (Some(_), Some(Row::Recent(_))) => at,
            _ => group,
        };
        Outcome::Forget {
            scope,
            setting,
            send: self.refill(),
        }
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
                    enabled: true,
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

    /// Settings Center's configuration display pane, for the group of the item selected:
    /// what to send when the group changed, or the settings it shows changed elsewhere than in
    /// the pane. JAWS's pane holds the group's controls, "User page | Typing Echo Off radio
    /// button checked | 1 of 4" [obs, the walk of Settings Center's F6].
    pub fn pane_update(&mut self) -> Option<ToUi> {
        let group = self.pane_group()?;
        if self
            .pane
            .as_ref()
            .is_some_and(|(shown, draft)| *shown == group && *draft == self.draft)
        {
            return None;
        }
        self.pane = Some((group, self.draft.clone()));
        Some(ToUi::DialogPane {
            id: self.id,
            page: self.pane_page(group),
        })
    }

    /// The group the pane is for: the selected setting's, or the group selected; Settings
    /// Center's only.
    fn pane_group(&self) -> Option<TreeGroup> {
        if self.flavor != Flavor::Center {
            return None;
        }
        match self.rows.get(self.selected).copied()? {
            Row::Setting(spec) | Row::Recent(spec) => self.group_of(spec),
            Row::Group(group) => Some(group),
            Row::VerbosityLevels | Row::LastChanged => None,
        }
    }

    /// The pane's page for `group`: its settings' controls, named by the group.
    fn pane_page(&self, group: TreeGroup) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: group.label().to_string(),
            controls: self
                .shown()
                .filter(|(g, _)| *g == group)
                .filter_map(|(_, spec)| self.pane_control(spec))
                .collect(),
            keys: Vec::new(),
        }
    }

    /// A setting's control in the pane: radio buttons for a few values, a combo box for more
    /// (JAWS's Typing Echo and Progress Bar Updates Announcement), a check box, a slider; one
    /// the place cannot keep is unavailable.
    fn pane_control(&self, spec: &'static SettingSpec) -> Option<Control> {
        let value = registry::get(&self.draft, spec.id).unwrap_or(Json::Null);
        let label = self.tree_label(spec);
        let (label, kind) = match spec.kind {
            Kind::Choice(choices) => {
                let choices = choices();
                let selected = choices.iter().position(|c| c.value == value).unwrap_or(0);
                let options = choices.iter().map(|c| c.label.to_string()).collect();
                let kind = if choices.len() <= 4 {
                    ControlKind::Radios { options, selected }
                } else {
                    ControlKind::Choice { options, selected }
                };
                (label, kind)
            }
            Kind::Check { on, .. } => (
                label,
                ControlKind::CheckBox {
                    checked: value == on.value(),
                },
            ),
            // Its label shows its number after the colon.
            Kind::Range { min, max } => (
                format!("{label}:"),
                ControlKind::Slider {
                    min,
                    max,
                    value: value
                        .as_i64()
                        .and_then(|n| i32::try_from(n).ok())
                        .unwrap_or(min),
                },
            ),
            Kind::Voice => (
                label,
                ControlKind::Choice {
                    selected: value
                        .as_str()
                        .and_then(|name| self.voices.iter().position(|v| v == name))
                        .map_or(0, |i| i + 1),
                    options: std::iter::once(msg::settings_voice_default().to_string())
                        .chain(self.voices.iter().cloned())
                        .collect(),
                },
            ),
            Kind::Structured => return None,
        };
        Some(Control {
            id: format!("{PANE_PREFIX}{}", spec.id),
            label,
            kind,
            enabled: spec.scopes.allows(&self.scope()),
            hidden: false,
        })
    }

    /// A value set in the pane: the draft's, and the tree's items say it.
    fn pane_changed(&mut self, id: &str, value: &Value) -> Outcome {
        let Some(spec) = SETTINGS.iter().find(|s| s.id == id) else {
            return Outcome::Nothing;
        };
        let new = match (spec.kind, value) {
            (Kind::Choice(choices), Value::Selected(i)) => {
                choices().into_iter().nth(*i).map(|c| c.value)
            }
            (Kind::Check { on, off }, Value::Checked(checked)) => {
                Some(if *checked { on.value() } else { off.value() })
            }
            (Kind::Range { min, max }, Value::Number(n)) => Some(Json::from((*n).clamp(min, max))),
            (Kind::Voice, Value::Selected(0)) => Some(Json::Null),
            (Kind::Voice, Value::Selected(i)) => self
                .voices
                .get(i - 1)
                .map(|name| Json::String(name.clone())),
            _ => None,
        };
        let Some(new) = new else {
            return Outcome::Nothing;
        };
        let before = self.items();
        if !registry::set(&mut self.draft, spec.id, new) {
            return Outcome::Nothing;
        }
        self.changed = true;
        // The pane already shows it.
        if let Some((_, shown)) = self.pane.as_mut() {
            *shown = self.draft.clone();
        }
        let mut sent = self.items_changed(&before);
        sent.push(ToUi::DialogEnable {
            id: self.id,
            control: "apply".to_owned(),
            enabled: true,
        });
        Outcome::Send(sent)
    }

    /// F6 in the tree: the selected setting's control in the pane, else the pane's first.
    fn to_pane(&self) -> Outcome {
        let Some(page) = self.pane_group().map(|group| self.pane_page(group)) else {
            return Outcome::Nothing;
        };
        let selected = match self.rows.get(self.selected) {
            Some(Row::Setting(spec) | Row::Recent(spec)) => {
                Some(format!("{PANE_PREFIX}{}", spec.id))
            }
            _ => None,
        };
        let control = selected
            .filter(|id| page.controls.iter().any(|c| &c.id == id))
            .or_else(|| page.controls.first().map(|c| c.id.clone()));
        match control {
            Some(control) => Outcome::Send(vec![ToUi::DialogFocus {
                id: self.id,
                control,
            }]),
            None => Outcome::Nothing,
        }
    }

    /// What the dialog's window reported, and the pane brought up to date with it.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        let from_pane = matches!(message,
            FromUi::DialogChanged { id, control, .. }
                if *id == self.id && control.starts_with(PANE_PREFIX));
        let outcome = self.handle_message(message);
        if from_pane {
            return outcome;
        }
        let Some(pane) = self.pane_update() else {
            return outcome;
        };
        match outcome {
            Outcome::Nothing => Outcome::Send(vec![pane]),
            Outcome::Send(mut sent) => {
                sent.push(pane);
                Outcome::Send(sent)
            }
            Outcome::Changed(mut sent, said) => {
                sent.push(pane);
                Outcome::Changed(sent, said)
            }
            other => other,
        }
    }

    fn handle_message(&mut self, message: &FromUi) -> Outcome {
        match message {
            FromUi::DialogChanged { id, control, value }
                if *id == self.id && control.starts_with(PANE_PREFIX) =>
            {
                let setting = control.trim_start_matches(PANE_PREFIX).to_owned();
                self.pane_changed(&setting, value)
            }
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    (TREE, Value::Selected(index)) => {
                        self.selected = *index;
                        Outcome::Send(vec![self.description_shown()])
                    }
                    (RESULTS, Value::Selected(index)) => {
                        self.result = *index;
                        self.renamed = None;
                        Outcome::Send(vec![self.description_shown()])
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
                    // The results in the tree's place as the search box gets words, the tree
                    // again as it is emptied; the tree is as it was.
                    (SEARCH, Value::Text(text)) => {
                        let searching = !self.search.is_empty();
                        self.search = text.trim().to_owned();
                        self.results = self.found();
                        self.result = 0;
                        self.renamed = None;
                        let mut sent = if self.search.is_empty() {
                            Vec::new()
                        } else {
                            self.results_shown()
                        };
                        if searching == self.search.is_empty() {
                            sent.push(self.swap(!searching));
                        }
                        sent.push(self.description_shown());
                        match self.search_said() {
                            Some(said) => Outcome::Changed(sent, said),
                            None => Outcome::Send(sent),
                        }
                    }
                    (EXPAND, Value::Checked(on)) => {
                        self.expand = *on;
                        self.expand_all(*on);
                        Outcome::Send(self.refill())
                    }
                    (PLACE, Value::Selected(place)) if *place < self.places.len() => {
                        self.change_place(*place);
                        Outcome::Send(self.refill())
                    }
                    _ => Outcome::Nothing,
                }
            }
            // Remove, Remove All, or a lifetime chosen for the menu's setting or group.
            FromUi::DialogMenuChosen { id, index, .. } if *id == self.id => {
                let row = self.menu_for.take();
                match (row, index) {
                    (Some(Row::LastChanged), Some(0)) => return self.forget(None),
                    (Some(Row::Recent(spec)), Some(0)) => return self.forget(Some(spec.id)),
                    _ => {}
                }
                let (Some(row), Some(&lifetime)) = (row, index.and_then(|i| LIFETIMES.get(i)))
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
                    // Shift+Space goes on as Space does: "Words", then "Characters and Words |
                    // 4 of 4" [obs, the walk of Settings Center's questions].
                    DialogKey::Change | DialogKey::ChangeBack => self.change(),
                    DialogKey::Delete | DialogKey::Rename => Outcome::Nothing,
                }
            }
            // The question: Yes saves and closes, No closes, Escape goes back to the dialog.
            FromUi::DialogPressed { id, control } if Some(*id) == self.question => {
                match control.as_str() {
                    "yes" => self.apply(true),
                    "no" => Outcome::Close,
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogClosed { id } if Some(*id) == self.question => {
                self.question = None;
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" => self.apply(true),
                "apply" => self.apply(false),
                "cancel" => self.cancel(),
                // In the tree Enter and Escape are OK and Cancel; on a result, Enter shows the
                // tree at the setting found, Escape where it was.
                TREE_ENTER => self.apply(true),
                TREE_ESCAPE => self.cancel(),
                RESULT_ENTER => {
                    let at = self.results.get(self.result).copied();
                    self.end_search(at)
                }
                RESULT_ESCAPE => self.end_search(None),
                RESULT_CHANGE => self.change_result(),
                SEARCH_DOWN | TO_LIST => Outcome::Send(vec![ToUi::DialogFocus {
                    id: self.id,
                    control: self.list().to_owned(),
                }]),
                LIFETIME => self.context_menu(),
                PANE_KEY => self.to_pane(),
                DEFAULT_PLACE if self.flavor == Flavor::Center => {
                    self.change_place(0);
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
            [
                SEARCH,
                TREE,
                RESULTS,
                DESCRIPTION,
                "apply",
                "ok",
                "cancel",
                PLACE
            ]
        );
        // The results' list box waits hidden in the tree's place.
        let hidden: Vec<&str> = spec
            .controls
            .iter()
            .filter(|c| c.hidden)
            .map(|c| c.id.as_str())
            .collect();
        assert_eq!(hidden, [RESULTS]);
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
        let Outcome::Changed(sent, said) = dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::Change,
        }) else {
            panic_free_fail("no update");
            return;
        };
        assert_eq!(said, "Words\n3 of 4");
        assert!(sent.iter().any(|m| matches!(
            m,
            ToUi::DialogSet { value: Value::TreeItem(i, item), .. }
                if *i == index && item.text == "Typing Echo Words"
        )));
        // Only the items that changed: the setting's, and its entry in Last Changed Settings.
        // Every row was set at each Space (performance review 2026-10-25, finding 8).
        let set = sent
            .iter()
            .filter(|m| {
                matches!(
                    m,
                    ToUi::DialogSet {
                        value: Value::TreeItem(..),
                        ..
                    }
                )
            })
            .count();
        assert!((1..=2).contains(&set), "{set} items set");
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

    /// Typing in the search box shows the settings found in a list box in the tree's place,
    /// named by their count, each with its value; emptied, the tree is back [obs, the walk of
    /// Settings Center's search].
    #[test]
    fn a_search_shows_its_results_in_the_trees_place() {
        let mut dialog = tree(Flavor::Center);
        let Outcome::Changed(sent, said) = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("echo".into()),
        }) else {
            panic_free_fail("nothing said");
            return;
        };
        // The first setting found and the count, as each key typed says.
        assert!(
            said.starts_with("Typing Echo\n1 of ") && said.ends_with(" Search Results"),
            "{said}"
        );
        let Some(ToUi::DialogItems { control, items, .. }) = sent.first() else {
            panic_free_fail("no results");
            return;
        };
        assert_eq!(control, RESULTS);
        assert!(
            items.contains(&"Typing Echo Characters".to_owned()),
            "{items:?}"
        );
        assert!(
            items
                .iter()
                .any(|i| i == "Enable Mouse Echo checked" || i == "Enable Mouse Echo not checked"),
            "{items:?}"
        );
        assert!(!items.iter().any(|i| i.starts_with("Rate")), "{items:?}");
        let count = items.len();
        assert!(sent
            .iter()
            .any(|m| matches!(m, ToUi::DialogLabel { label, .. }
            if *label == format!("{count} Search Results"))));
        assert!(sent
            .iter()
            .any(|m| matches!(m, ToUi::DialogSwap { show, hide, .. }
            if show == RESULTS && hide == TREE)));
        // The tree is as it was, its groups closed.
        assert!(dialog
            .items()
            .iter()
            .filter(|i| i.level == 0)
            .all(|i| !i.open));
        // Another search: no swap, the list filled anew.
        let Outcome::Changed(sent, said) = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("zzzz".into()),
        }) else {
            panic_free_fail("nothing said");
            return;
        };
        assert_eq!(said, "No matches found.");
        assert!(!sent.iter().any(|m| matches!(m, ToUi::DialogSwap { .. })));
        // Emptied: the tree again.
        let Outcome::Send(sent) = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text(String::new()),
        }) else {
            panic_free_fail("nothing sent");
            return;
        };
        assert!(sent
            .iter()
            .any(|m| matches!(m, ToUi::DialogSwap { show, hide, .. }
            if show == TREE && hide == RESULTS)));
    }

    /// Screen Echo in User, Smart Navigation Mode and Indicate Headings in their Web groups, each
    /// with its value as Settings Center names it, then its group's path [obs, the walk of
    /// Settings Center's search].
    #[test]
    fn the_echo_navigation_and_heading_results_say_their_groups() {
        let mut dialog = tree(Flavor::Center);
        for (search, item, path) in [
            ("screen echo", "Screen Echo Echo Highlighted Text", "User"),
            (
                "smart navigation",
                "Smart Navigation Mode Off",
                "Navigation Web / HTML / PDFs",
            ),
            (
                "indicate",
                "Indicate Headings Headings with Level",
                "Headings and Frames Web / HTML / PDFs",
            ),
        ] {
            dialog.handle(&FromUi::DialogChanged {
                id: 7,
                control: SEARCH.into(),
                value: Value::Text(search.into()),
            });
            let found = dialog
                .results
                .iter()
                .find(|row| dialog.result_text(row) == item)
                .map(|row| dialog.result_path(row));
            assert_eq!(found.as_deref(), Some(path), "{search}");
        }
    }

    /// Enter on a result shows the tree at it, its groups open; Escape on the results the tree
    /// where it was; each says "Search edit box cleared." before the tree [obs, the walk of
    /// Settings Center's search].
    #[test]
    fn enter_and_escape_end_a_search() {
        let mut dialog = tree(Flavor::Center);
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("dollars".into()),
        });
        let found = dialog.results.first().copied();
        assert!(matches!(found, Some(Row::Setting(spec)) if spec.id == "voice.text.say_dollars"));
        let Outcome::BeforeFocus(sent, said) = dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: RESULT_ENTER.into(),
        }) else {
            panic_free_fail("no tree");
            return;
        };
        assert_eq!(said, "Search edit box cleared.");
        assert!(sent
            .iter()
            .any(|m| matches!(m, ToUi::DialogSwap { show, .. } if show == TREE)));
        assert!(matches!(sent.last(), Some(ToUi::DialogFocus { control, .. }) if control == TREE));
        assert!(dialog.search.is_empty());
        assert_eq!(dialog.rows.get(dialog.selected).copied(), found);
        let items = dialog.items();
        let open = |text: &str| items.iter().any(|i| i.text == text && i.open);
        assert!(open("Text Processing") && open("Number And Date Processing"));
        // Escape on a search's results: the tree where it was.
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("echo".into()),
        });
        assert!(matches!(
            dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: RESULT_ESCAPE.into(),
            }),
            Outcome::BeforeFocus(..)
        ));
        assert_eq!(dialog.rows.get(dialog.selected).copied(), found);
        // In the tree, Escape closes.
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: TREE_ESCAPE.into(),
            }),
            Outcome::Close
        );
    }

    /// Down from the search box goes into the results while there is a search, else into the
    /// tree.
    #[test]
    fn down_from_the_search_box_goes_to_the_list_in_the_trees_place() {
        let mut dialog = tree(Flavor::Center);
        let down = FromUi::DialogPressed {
            id: 7,
            control: SEARCH_DOWN.into(),
        };
        // The focus moved first; the pane may follow it.
        let focus = |outcome: Outcome| match outcome {
            Outcome::Send(sent) => match sent.first() {
                Some(ToUi::DialogFocus { control, .. }) => control.clone(),
                _ => String::new(),
            },
            _ => String::new(),
        };
        assert_eq!(focus(dialog.handle(&down)), TREE);
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("echo".into()),
        });
        assert_eq!(focus(dialog.handle(&down)), RESULTS);
    }

    /// A result's focus says its group's path after it and no place in the list, with the
    /// list's own hints; Space changes its value, says it with its place, and the result's item
    /// made anew is not said again [obs, the walk of Settings Center's search].
    #[test]
    fn a_result_is_said_with_its_group_and_changed_by_space() {
        let mut dialog = tree(Flavor::Center);
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: SEARCH.into(),
            value: Value::Text("typing echo".into()),
        });
        let first = dialog.result_texts().first().cloned().unwrap_or_default();
        let mut focus = FocusEvent::new(Role::ListItem).with_name(first.clone());
        focus.window_title = Some(dialog.title());
        focus.item_index = Some(1);
        focus.item_count = Some(1);
        let Some(ResultFocus::Detailed(detailed)) = dialog.result_focus(&focus) else {
            panic_free_fail("no result");
            return;
        };
        assert_eq!(detailed.columns, ["User"]);
        assert_eq!((detailed.item_index, detailed.item_count), (None, None));
        assert_eq!(
            detailed.own_hints,
            [
                "Use the arrow keys to select an item.",
                "Press the space bar to toggle between the choices"
            ]
        );
        let Outcome::Changed(sent, said) = dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: RESULT_CHANGE.into(),
        }) else {
            panic_free_fail("nothing changed");
            return;
        };
        assert!(said.ends_with(" of 4"), "{said}");
        let renamed = sent.iter().find_map(|m| match m {
            ToUi::DialogSet {
                control,
                value: Value::ItemRenamed(0, text),
                ..
            } if control == RESULTS => Some(text.clone()),
            _ => None,
        });
        assert!(
            renamed.as_ref().is_some_and(|text| *text != first),
            "{sent:?}"
        );
        let again = FocusEvent::new(Role::ListItem).with_name(renamed.unwrap_or_default());
        let again = FocusEvent {
            window_title: Some(dialog.title()),
            ..again
        };
        assert_eq!(dialog.result_focus(&again), Some(ResultFocus::Unsaid));
        assert!(matches!(
            dialog.result_focus(&again),
            Some(ResultFocus::Detailed(_))
        ));
        // Another dialog's list item is not a result.
        let other = FocusEvent::new(Role::ListItem).with_name(first);
        assert_eq!(dialog.result_focus(&other), None);
    }

    /// Settings Center cancelled with a change not applied asks whether to save it: Escape goes
    /// back to the dialog, Yes saves and closes, No closes [obs, the walk of Settings Center's
    /// questions]. Nothing changed, it closes.
    #[test]
    fn settings_center_asks_to_save_on_cancel() {
        let mut dialog = tree(Flavor::Center);
        let cancel = FromUi::DialogPressed {
            id: 7,
            control: "cancel".into(),
        };
        assert_eq!(dialog.handle(&cancel), Outcome::Close);
        assert!(dialog
            .spec()
            .keys
            .iter()
            .any(|k| k.key == "Escape" && k.control == "cancel" && k.from.is_none()));
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
        dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::Change,
        });
        assert_eq!(dialog.handle(&cancel), Outcome::AskToSave);
        let question = dialog.save_question(9);
        assert!(dialog.owns(9) && dialog.owns(7) && !dialog.owns(8));
        assert_eq!(question.title, "Okur Settings Center");
        let texts: Vec<String> = question
            .controls
            .iter()
            .map(|c| match &c.kind {
                ControlKind::Message { text } => text.clone(),
                _ => c.label.replace('&', ""),
            })
            .collect();
        assert_eq!(
            texts,
            [
                "You have made changes to Notepad's settings. Do you want to save them?",
                "Yes",
                "No",
                "Cancel"
            ]
        );
        // Escape: back to Settings Center, the change kept.
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 9 }),
            Outcome::Nothing
        );
        assert_eq!(dialog.question(), None);
        // Escape in the tree asks again; No closes, Yes saves.
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: TREE_ESCAPE.into(),
            }),
            Outcome::AskToSave
        );
        dialog.save_question(10);
        assert_eq!(
            dialog.clone().handle(&FromUi::DialogPressed {
                id: 10,
                control: "no".into(),
            }),
            Outcome::Close
        );
        assert!(matches!(
            dialog.handle(&FromUi::DialogPressed {
                id: 10,
                control: "yes".into(),
            }),
            Outcome::Apply { close: true, .. }
        ));
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

    /// Quick Settings' Tab order: the search box, the tree, the description, "Expand Tree View"
    /// checked, OK; no Apply and no Cancel. Its groups start open; clearing the check box
    /// closes them, the setting selected giving its place to its group [obs, the second walk of
    /// Settings Center's questions].
    #[test]
    fn quick_settings_expands_its_tree() {
        let mut dialog = tree(Flavor::Quick);
        let spec = dialog.spec();
        let ids: Vec<&str> = spec.controls.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, [SEARCH, TREE, RESULTS, DESCRIPTION, EXPAND, "ok"]);
        assert!(spec
            .controls
            .iter()
            .any(|c| c.label == "Expand Tree View"
                && c.kind == ControlKind::CheckBox { checked: true }));
        assert!(tree_items(&spec)
            .iter()
            .filter(|i| i.level == 0)
            .all(|i| i.open));
        let setting = dialog
            .rows
            .iter()
            .position(|r| matches!(r, Row::Setting(spec) if spec.id == "typing_echo"))
            .unwrap_or_default();
        dialog.selected = setting;
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: EXPAND.into(),
            value: Value::Checked(false),
        });
        let items = tree_items(&dialog.spec());
        assert!(items.iter().filter(|i| i.level == 0).all(|i| !i.open));
        assert_eq!(
            items.get(dialog.selected).map(|i| i.text.as_str()),
            Some("Editing Options")
        );
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
            Some("Typing Echo (User) Characters")
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

    /// Another place chosen, by the Application box or Ctrl+Shift+D: its own Last Changed
    /// Settings, where Notepad's stayed (performance review 2026-10-25, finding 16).
    #[test]
    fn another_place_has_its_own_last_changed_settings() {
        let mut dialog = tree(Flavor::Center);
        let recent = |dialog: &SettingsTree| {
            let items = dialog.items();
            let last = items
                .iter()
                .position(|i| i.text == "Last Changed Settings")
                .map_or(0, |i| i + 1);
            items.get(last).map(|i| i.text.clone()).unwrap_or_default()
        };
        assert_eq!(recent(&dialog), "Typing Echo (User) Characters");
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: PLACE.into(),
            value: Value::Selected(0),
        });
        assert!(recent(&dialog).starts_with("Rate"), "{}", recent(&dialog));
        assert!(dialog.selected < dialog.rows.len());
        dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: PLACE.into(),
            value: Value::Selected(1),
        });
        assert_eq!(recent(&dialog), "Typing Echo (User) Characters");
        dialog.handle(&FromUi::DialogPressed {
            id: 7,
            control: DEFAULT_PLACE.into(),
        });
        assert!(recent(&dialog).starts_with("Rate"), "{}", recent(&dialog));
    }

    /// Shift+F10 in Last Changed Settings: an entry's menu is Remove, the group's Remove All,
    /// unavailable once it is empty; each forgets at once, for the place only [obs, the walk of
    /// Last Changed Settings].
    #[test]
    fn last_changed_settings_entries_can_be_removed() {
        let mut dialog = tree(Flavor::Center);
        let menu = |dialog: &mut SettingsTree, row: Row| {
            let index = dialog
                .rows
                .iter()
                .position(|r| *r == row)
                .unwrap_or_default();
            dialog.handle(&FromUi::DialogChanged {
                id: 7,
                control: TREE.into(),
                value: Value::Selected(index),
            });
            match dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: LIFETIME.into(),
            }) {
                Outcome::Send(sent) => match sent.first() {
                    Some(ToUi::DialogMenu { items, .. }) => items
                        .iter()
                        .map(|i| (i.text.clone(), i.enabled))
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            }
        };
        let typing = SETTINGS.iter().find(|s| s.id == "typing_echo");
        let Some(typing) = typing else {
            panic_free_fail("no typing echo");
            return;
        };
        assert_eq!(
            menu(&mut dialog, Row::Recent(typing)),
            [("Remove".to_owned(), true)]
        );
        let chosen = FromUi::DialogMenuChosen {
            id: 7,
            control: TREE.into(),
            index: Some(0),
        };
        let Outcome::Forget { scope, setting, .. } = dialog.handle(&chosen) else {
            panic_free_fail("not forgotten");
            return;
        };
        assert_eq!(scope, Scope::Application("notepad".into()));
        assert_eq!(setting, Some("typing_echo"));
        assert!(!dialog.rows.iter().any(|r| matches!(r, Row::Recent(_))));
        assert_eq!(dialog.rows.get(dialog.selected), Some(&Row::LastChanged));
        // All applications' history stays.
        assert!(dialog.history.contains(&(Scope::Global, "voice.rate")));
        assert_eq!(
            menu(&mut dialog, Row::LastChanged),
            [("Remove All".to_owned(), false)]
        );
        // A setting's Shift+F10 in Settings Center has no menu: lifetimes are Quick Settings'.
        assert!(menu(&mut dialog, Row::Setting(typing)).is_empty());
    }

    /// Settings Center's pane holds the selected setting's group, as radio buttons, check boxes
    /// and sliders; F6 goes to the setting's control there; a value set there is the draft's and
    /// the tree's, and the pane comes again only when the group or a value changes elsewhere
    /// [obs, the walk of Settings Center's F6].
    #[test]
    fn the_pane_holds_the_group_and_f6_reaches_it() {
        let mut dialog = tree(Flavor::Center);
        let typing = dialog
            .rows
            .iter()
            .position(|r| matches!(r, Row::Setting(spec) if spec.id == "typing_echo"))
            .unwrap_or_default();
        let select = FromUi::DialogChanged {
            id: 7,
            control: TREE.into(),
            value: Value::Selected(typing),
        };
        let Outcome::Send(sent) = dialog.handle(&select) else {
            panic_free_fail("nothing sent");
            return;
        };
        let Some(ToUi::DialogPane { page, .. }) = sent.last() else {
            panic_free_fail(&format!("no pane: {sent:?}"));
            return;
        };
        assert_eq!(page.title, "User");
        let echo = page.controls.iter().find(|c| c.id == "pane:typing_echo");
        assert!(
            matches!(echo.map(|c| &c.kind), Some(ControlKind::Radios { options, selected: 1 }) if options.len() == 4),
            "{echo:?}"
        );
        // The same group again: no new pane.
        assert_eq!(
            dialog.handle(&select),
            Outcome::Send(vec![ToUi::DialogSet {
                id: 7,
                control: DESCRIPTION.into(),
                value: Value::Text(dialog.description()),
            }])
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 7,
                control: PANE_KEY.into(),
            }),
            Outcome::Send(vec![ToUi::DialogFocus {
                id: 7,
                control: "pane:typing_echo".into(),
            }])
        );
        // Words set in the pane: the draft's and the tree's, the pane left as it is.
        let Outcome::Send(sent) = dialog.handle(&FromUi::DialogChanged {
            id: 7,
            control: "pane:typing_echo".into(),
            value: Value::Selected(2),
        }) else {
            panic_free_fail("nothing sent");
            return;
        };
        assert_eq!(
            registry::get(dialog.draft(), "typing_echo"),
            Some(Json::String("words".into()))
        );
        assert!(!sent.iter().any(|m| matches!(m, ToUi::DialogPane { .. })));
        assert!(sent.iter().any(|m| matches!(m,
            ToUi::DialogSet { value: Value::TreeItem(_, item), .. } if item.text == "Typing Echo Words")));
        // Space in the tree: the pane comes again with the new value.
        let Outcome::Changed(sent, _) = dialog.handle(&FromUi::DialogKey {
            id: 7,
            control: TREE.into(),
            key: DialogKey::Change,
        }) else {
            panic_free_fail("not changed");
            return;
        };
        assert!(matches!(sent.last(), Some(ToUi::DialogPane { .. })));
        // Quick Settings has none.
        let mut quick = tree(Flavor::Quick);
        assert_eq!(quick.pane_update(), None);
    }

    /// Last Changed Settings kept between runs: each change's place, never the session's, and
    /// back without settings Okur no longer has.
    #[test]
    fn the_history_is_kept_with_its_places() {
        let history: History = vec![
            (Scope::Application("notepad".into()), "typing_echo"),
            (Scope::Session, "voice.rate"),
            (Scope::Site("example.org".into()), "smart_navigation"),
            (Scope::Global, "voice.rate"),
        ];
        let mut saved = SavedHistory::of(&history);
        assert_eq!(saved.changed.len(), 3);
        saved.changed.push(SavedChange {
            application: None,
            site: None,
            setting: "no_such_setting".into(),
        });
        assert_eq!(
            saved.history(),
            [
                (Scope::Application("notepad".into()), "typing_echo"),
                (Scope::Site("example.org".into()), "smart_navigation"),
                (Scope::Global, "voice.rate"),
            ]
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
