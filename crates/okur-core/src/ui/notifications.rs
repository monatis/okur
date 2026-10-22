//! The Notification History (Insert+Space, N) and the details of a notification. Its controls
//! are JAWS 2025's [obs, 774]: the list "Recent Notifications", the latest first, each row a
//! notification's text, its sender and the time it came ("New notification from Windows
//! PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1 Windows PowerShell 11:07 PM");
//! then Enable Rules (checked), Create Rule..., Clear History and Close, in that Tab order.
//! Enter on a row shows its details: "Notification Details", the text in a read-only edit
//! "Notification Text:" [obs, 774/s11]. JAWS keeps the past 24 hours (JAWS 2025 help,
//! Notification History).

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};
use serde::{Deserialize, Serialize};

use crate::messages::{msg, StaticMessage};

/// A notification as it came.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub text: String,
    /// The application it came from, as the toast names it ("Windows PowerShell").
    pub sender: Option<String>,
    /// When it came, as Say System Time says it ("11:07 PM").
    pub time: String,
}

impl Notification {
    /// Its row in the history: text, sender, time.
    pub fn row(&self) -> String {
        [
            Some(self.text.as_str()),
            self.sender.as_deref(),
            Some(self.time.as_str()),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
    }
}

/// What the engine does after the history handled a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Enter on row `usize`: its details.
    Details(usize),
    /// Create Rule... on row `usize`.
    CreateRule(usize),
    /// Clear History.
    Clear,
    /// Manage Rules...
    Manage,
    Closed,
}

/// The Notification History while it is open.
#[derive(Debug, Clone)]
pub struct HistoryDialog {
    pub id: u32,
    rows: Vec<String>,
    selected: usize,
    rules: bool,
    /// Rules are kept: Manage Rules... is available (JAWS 2025 help: "not available until you
    /// have created at least one rule"; after Create Rule... [obs, 783/s10]).
    has_rules: bool,
}

fn control(id: &str, label: StaticMessage, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label: label.to_string(),
        kind,
        enabled: true,
    }
}

impl HistoryDialog {
    /// The history over `rows`, the latest first.
    pub fn new(id: u32, rows: Vec<String>, rules: bool, has_rules: bool) -> Self {
        Self {
            id,
            rows,
            selected: 0,
            rules,
            has_rules,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let button = |id: &str, label: StaticMessage| {
            control(id, label, ControlKind::Button { default: false })
        };
        DialogSpec {
            id: self.id,
            title: msg::notification_history_title().to_string(),
            controls: vec![
                control(
                    "notifications",
                    msg::notification_history_list(),
                    ControlKind::ListBox {
                        items: self.rows.clone(),
                        selected: 0,
                    },
                ),
                control(
                    "rules",
                    msg::notification_history_rules(),
                    ControlKind::CheckBox {
                        checked: self.rules,
                    },
                ),
                button("create", msg::notification_history_create_rule()),
                Control {
                    enabled: self.has_rules,
                    ..button("manage", msg::notification_history_manage())
                },
                button("clear", msg::notification_history_clear()),
                button("cancel", msg::notification_history_close()),
            ],
        }
    }

    /// Whether rules are on (Enable Rules).
    pub fn rules(&self) -> bool {
        self.rules
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("notifications", Value::Selected(i)) => self.selected = *i,
                    ("rules", Value::Checked(on)) => self.rules = *on,
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "notifications" if self.selected < self.rows.len() => {
                    Outcome::Details(self.selected)
                }
                "create" if self.selected < self.rows.len() => Outcome::CreateRule(self.selected),
                "clear" => {
                    self.rows.clear();
                    self.selected = 0;
                    Outcome::Clear
                }
                "manage" if self.has_rules => Outcome::Manage,
                "cancel" => Outcome::Closed,
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

/// What a rule compares a notification's text with: Rule Type's options (JAWS 2025 help,
/// Notification History, Creating a Notification Rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleType {
    Contains,
    BeginsWith,
    EndsWith,
    /// A regular expression.
    MatchesPattern,
}

impl RuleType {
    pub const ALL: [Self; 4] = [
        Self::Contains,
        Self::BeginsWith,
        Self::EndsWith,
        Self::MatchesPattern,
    ];

    fn label(self) -> StaticMessage {
        match self {
            Self::Contains => msg::notification_rule_contains(),
            Self::BeginsWith => msg::notification_rule_begins(),
            Self::EndsWith => msg::notification_rule_ends(),
            Self::MatchesPattern => msg::notification_rule_pattern(),
        }
    }
}

/// What is said of a notification a rule matches: Speech Action's options. Mute is a new
/// rule's [obs, 775/s9].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechAction {
    Mute,
    /// Another message in its place: its Shortened Text.
    Shorten,
    /// A sound in its place: its Path to Sound File.
    PlaySound,
    SpeakFull,
}

impl SpeechAction {
    pub const ALL: [Self; 4] = [Self::Mute, Self::Shorten, Self::PlaySound, Self::SpeakFull];

    fn label(self) -> StaticMessage {
        match self {
            Self::Mute => msg::notification_rule_mute(),
            Self::Shorten => msg::notification_rule_shorten(),
            Self::PlaySound => msg::notification_rule_sound(),
            Self::SpeakFull => msg::notification_rule_speak(),
        }
    }
}

/// Braille Action's options; Show flash message is a new rule's [obs, 775/s10]. Okur has no
/// braille yet: the choice is kept.
const BRAILLE_ACTIONS: [fn() -> StaticMessage; 3] = [
    msg::notification_rule_braille_none,
    msg::notification_rule_braille_flash,
    msg::notification_rule_braille_short,
];

/// A notification rule: which notifications it matches and what is done with them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// The name given in Name Your Rule [obs, 783/s7].
    #[serde(default)]
    pub name: String,
    pub kind: RuleType,
    pub text: String,
    /// Limited to the notifications of this sender.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    pub speech: SpeechAction,
    /// Braille Action's option, by its index.
    #[serde(default)]
    pub braille: usize,
    /// Don't Show in History.
    #[serde(default)]
    pub hidden: bool,
    /// Shorten's message, said in the notification's place.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub shortened: String,
    /// Play Sound's file, played in the notification's place.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sound: String,
    /// Turned off in Manage Rules (Disable): kept, deciding nothing [obs, 784/s6].
    #[serde(default = "rules_enabled")]
    pub enabled: bool,
}

/// The notification rules kept, the first that matches deciding, and whether they apply
/// (Enable Rules).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationRules {
    #[serde(default = "rules_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn rules_enabled() -> bool {
    true
}

impl Default for NotificationRules {
    fn default() -> Self {
        Self {
            enabled: true,
            rules: Vec::new(),
        }
    }
}

impl NotificationRules {
    /// The rule that decides for a notification of `sender` saying `text`, while rules apply.
    pub fn deciding(&self, text: &str, sender: Option<&str>) -> Option<&Rule> {
        if !self.enabled {
            return None;
        }
        self.rules
            .iter()
            .find(|rule| rule.enabled && rule.matches(text, sender))
    }
}

/// `shortened` with `\n` replaced by capture `n` (none for a capture that did not take
/// part) and `\\` by a backslash.
fn with_captures(shortened: &str, captures: Option<&regex::Captures>) -> String {
    let mut out = String::with_capacity(shortened.len());
    let mut chars = shortened.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('\\') => {
                out.push('\\');
                chars.next();
            }
            Some(digit) if digit.is_ascii_digit() => {
                chars.next();
                let index = digit.to_digit(10).and_then(|d| usize::try_from(d).ok());
                if let Some(found) = index
                    .zip(captures)
                    .and_then(|(index, captures)| captures.get(index))
                {
                    out.push_str(found.as_str());
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Where the notification rules are kept between runs (Okur's settings folder).
pub trait NotificationRuleStore: Send {
    fn save(&self, rules: &NotificationRules);
}

impl Rule {
    /// What is said for a notification saying `text` that this rule decides: nothing for
    /// Mute and Play Sound (its sound is played instead), the whole of it for Speak the full
    /// message, and Shorten's message, in which a pattern rule's `\1`, `\2` ... are the
    /// pattern's captures and `\\` a backslash (JAWS 2025 help, Notification History).
    pub fn spoken(&self, text: &str) -> Option<String> {
        match self.speech {
            SpeechAction::Mute | SpeechAction::PlaySound => None,
            SpeechAction::SpeakFull => Some(text.to_owned()),
            SpeechAction::Shorten if self.kind == RuleType::MatchesPattern => {
                let captures = regex::Regex::new(&self.text)
                    .ok()
                    .and_then(|pattern| pattern.captures(text));
                Some(with_captures(&self.shortened, captures.as_ref()))
            }
            SpeechAction::Shorten => Some(self.shortened.clone()),
        }
    }

    /// Whether the rule acts on a notification of `sender` saying `text`. A pattern that is
    /// not a regular expression matches nothing.
    pub fn matches(&self, text: &str, sender: Option<&str>) -> bool {
        if self.sender.is_some() && self.sender.as_deref() != sender {
            return false;
        }
        match self.kind {
            RuleType::Contains => text.contains(&self.text),
            RuleType::BeginsWith => text.starts_with(&self.text),
            RuleType::EndsWith => text.ends_with(&self.text),
            RuleType::MatchesPattern => {
                regex::Regex::new(&self.text).is_ok_and(|pattern| pattern.is_match(text))
            }
        }
    }
}

/// What the engine does after Create Rule handled a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleOutcome {
    Nothing,
    /// Send these to the dialog.
    Send(Vec<ToUi>),
    /// Play this sound file (Play Sound's preview).
    Play(String),
    /// OK: the rule.
    Save(Rule),
    Closed,
}

/// Name Your Rule, after Create Rule's OK [obs, 783/s7]: "Please enter a name for the rule.",
/// and Rule Name holding the notification's text; OK keeps the rule under that name.
#[derive(Debug, Clone)]
pub struct NameRuleDialog {
    pub id: u32,
    rule: Rule,
    name: String,
}

impl NameRuleDialog {
    pub fn new(id: u32, rule: Rule, proposed: &str) -> Self {
        Self {
            id,
            rule,
            name: proposed.to_owned(),
        }
    }

    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::notification_rule_name_title().to_string(),
            controls: vec![
                control(
                    "prompt",
                    msg::notification_rule_name_prompt(),
                    ControlKind::Message {
                        text: msg::notification_rule_name_prompt().to_string(),
                    },
                ),
                control(
                    "name",
                    msg::notification_rule_name(),
                    ControlKind::Edit {
                        text: self.name.clone(),
                    },
                ),
                control(
                    "ok",
                    msg::notification_rule_ok(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::notification_rule_cancel(),
                    ControlKind::Button { default: false },
                ),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> RuleOutcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                if let ("name", Value::Text(text)) = (control.as_str(), value) {
                    self.name = text.clone();
                }
                RuleOutcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" if !self.name.trim().is_empty() => RuleOutcome::Save(Rule {
                    name: self.name.trim().to_owned(),
                    ..self.rule.clone()
                }),
                "cancel" => RuleOutcome::Closed,
                _ => RuleOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => RuleOutcome::Closed,
            _ => RuleOutcome::Nothing,
        }
    }
}

/// Create Rule, over the Notification History, for one notification. Its controls are JAWS
/// 2025's [obs, 775/s6-s13]: in "Received Notification", Rule Type (Contains), the
/// notification's text in an edit with no name, "Limit to notifications from <sender>"
/// (checked); Speech Action (Mute) in "Speech or Sound Action"; Braille Action (Show flash
/// message) in "Braille Action"; Don't Show in History; OK and Cancel.
#[derive(Debug, Clone)]
pub struct CreateRuleDialog {
    pub id: u32,
    rule: Rule,
    /// The notification's sender, which Limit to notifications from names.
    from: Option<String>,
    /// Modify... in Manage Rules: "Modify Rule", the rule's choices shown [obs, 787/s11].
    modifying: bool,
}

impl CreateRuleDialog {
    pub fn new(id: u32, text: &str, sender: Option<&str>) -> Self {
        Self {
            id,
            rule: Rule {
                name: String::new(),
                kind: RuleType::Contains,
                text: text.to_owned(),
                sender: sender.map(str::to_owned),
                speech: SpeechAction::Mute,
                braille: 1,
                hidden: false,
                shortened: String::new(),
                sound: String::new(),
                enabled: true,
            },
            from: sender.map(str::to_owned),
            modifying: false,
        }
    }

    /// Whether this is Modify Rule, for a rule kept.
    pub fn modifying(&self) -> bool {
        self.modifying
    }

    /// Modify Rule for a rule kept: its own choices [obs, 787/s11-s14].
    pub fn modify(id: u32, rule: &Rule) -> Self {
        Self {
            id,
            rule: rule.clone(),
            from: rule.sender.clone(),
            modifying: true,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let labelled = |id: &str, label: String, kind: ControlKind| Control {
            id: id.to_owned(),
            label,
            kind,
            enabled: true,
        };
        let choice = |id: &str, label: StaticMessage, options: Vec<String>, selected: usize| {
            control(id, label, ControlKind::Choice { options, selected })
        };
        DialogSpec {
            id: self.id,
            title: if self.modifying {
                msg::notification_rule_modify_title().to_string()
            } else {
                msg::notification_rule_title().to_string()
            },
            controls: vec![
                control(
                    "received",
                    msg::notification_rule_received(),
                    ControlKind::Group,
                ),
                choice(
                    "type",
                    msg::notification_rule_type(),
                    RuleType::ALL
                        .iter()
                        .map(|t| t.label().to_string())
                        .collect(),
                    RuleType::ALL
                        .iter()
                        .position(|&t| t == self.rule.kind)
                        .unwrap_or(0),
                ),
                labelled(
                    "text",
                    String::new(),
                    ControlKind::Edit {
                        text: self.rule.text.clone(),
                    },
                ),
                labelled(
                    "limit",
                    msg::notification_rule_limit(self.from.clone().unwrap_or_default()).render(),
                    ControlKind::CheckBox {
                        checked: self.rule.sender.is_some(),
                    },
                ),
                control(
                    "speech_group",
                    msg::notification_rule_speech_group(),
                    ControlKind::Group,
                ),
                choice(
                    "speech",
                    msg::notification_rule_speech(),
                    SpeechAction::ALL
                        .iter()
                        .map(|a| a.label().to_string())
                        .collect(),
                    SpeechAction::ALL
                        .iter()
                        .position(|&a| a == self.rule.speech)
                        .unwrap_or(0),
                ),
                // Shorten's message, available once Shorten is chosen (JAWS 2025 help: "press
                // TAB to move to the Shortened Text edit field").
                Control {
                    enabled: self.rule.speech == SpeechAction::Shorten,
                    ..control(
                        "shortened",
                        msg::notification_rule_shortened(),
                        ControlKind::Edit {
                            text: self.rule.shortened.clone(),
                        },
                    )
                },
                // Play Sound's file, Select Sound... and Play Sound, available once Play sound
                // is chosen [obs, 801/s11-s13].
                Control {
                    enabled: self.rule.speech == SpeechAction::PlaySound,
                    ..control(
                        "sound",
                        msg::notification_rule_sound_path(),
                        ControlKind::Edit {
                            text: self.rule.sound.clone(),
                        },
                    )
                },
                Control {
                    enabled: self.rule.speech == SpeechAction::PlaySound,
                    ..control(
                        "select_sound",
                        msg::notification_rule_select_sound(),
                        ControlKind::Button { default: false },
                    )
                },
                Control {
                    enabled: self.rule.speech == SpeechAction::PlaySound,
                    ..control(
                        "play_sound",
                        msg::notification_rule_play_sound(),
                        ControlKind::Button { default: false },
                    )
                },
                control(
                    "braille_group",
                    msg::notification_rule_braille(),
                    ControlKind::Group,
                ),
                choice(
                    "braille",
                    msg::notification_rule_braille(),
                    BRAILLE_ACTIONS.iter().map(|a| a().to_string()).collect(),
                    self.rule.braille,
                ),
                control(
                    "hidden",
                    msg::notification_rule_hidden(),
                    ControlKind::CheckBox {
                        checked: self.rule.hidden,
                    },
                ),
                control(
                    "ok",
                    msg::notification_rule_ok(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "cancel",
                    msg::notification_rule_cancel(),
                    ControlKind::Button { default: false },
                ),
            ],
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> RuleOutcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("type", Value::Selected(i)) => {
                        if let Some(&kind) = RuleType::ALL.get(*i) {
                            self.rule.kind = kind;
                        }
                    }
                    ("text", Value::Text(text)) => self.rule.text = text.clone(),
                    ("limit", Value::Checked(on)) => {
                        self.rule.sender = on.then(|| self.from.clone()).flatten();
                    }
                    ("speech", Value::Selected(i)) => {
                        if let Some(&speech) = SpeechAction::ALL.get(*i) {
                            self.rule.speech = speech;
                            let enable = |control: &str, enabled: bool| ToUi::DialogEnable {
                                id: self.id,
                                control: control.to_owned(),
                                enabled,
                            };
                            let sound = speech == SpeechAction::PlaySound;
                            return RuleOutcome::Send(vec![
                                enable("shortened", speech == SpeechAction::Shorten),
                                enable("sound", sound),
                                enable("select_sound", sound),
                                enable("play_sound", sound),
                            ]);
                        }
                    }
                    ("shortened", Value::Text(text)) => self.rule.shortened = text.clone(),
                    ("sound", Value::Text(text)) => self.rule.sound = text.clone(),
                    ("braille", Value::Selected(i)) => self.rule.braille = *i,
                    ("hidden", Value::Checked(on)) => self.rule.hidden = *on,
                    _ => {}
                }
                RuleOutcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" if !self.rule.text.trim().is_empty() => RuleOutcome::Save(self.rule.clone()),
                // Play Sound: the file chosen, played now.
                "play_sound" if !self.rule.sound.trim().is_empty() => {
                    RuleOutcome::Play(self.rule.sound.clone())
                }
                // Select Sound...: the system's Open dialog, for Path to Sound File, as JAWS's
                // opens Windows' [obs, 802/s12].
                "select_sound" => RuleOutcome::Send(vec![ToUi::FileOpen {
                    id: self.id,
                    control: "sound".to_owned(),
                    filter: vec![
                        (
                            msg::notification_rule_sound_files().to_string(),
                            "*.wav".to_owned(),
                        ),
                        (
                            msg::notification_rule_all_files().to_string(),
                            "*.*".to_owned(),
                        ),
                    ],
                    folder: None,
                }]),
                "cancel" => RuleOutcome::Closed,
                _ => RuleOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => RuleOutcome::Closed,
            _ => RuleOutcome::Nothing,
        }
    }
}

/// A rule's row in Manage Rules: its name, its sender, what is said, what braille shows, and
/// whether it is on: "Okur lab rule Windows PowerShell Mute Show flash message Enabled" [obs,
/// 787/s8, s16].
pub fn rule_row(rule: &Rule) -> String {
    let braille = BRAILLE_ACTIONS
        .get(rule.braille)
        .map(|label| label().to_string());
    let state = if rule.enabled {
        msg::notification_rule_enabled()
    } else {
        msg::notification_rule_disabled()
    };
    [
        Some(rule.name.clone()),
        rule.sender.clone(),
        Some(rule.speech.label().to_string()),
        braille,
        Some(state.to_string()),
    ]
    .into_iter()
    .flatten()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join(" ")
}

/// What the engine does after Manage Rules handled a message, each on the rule chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageOutcome {
    Nothing,
    Modify(usize),
    Rename(usize),
    /// Disable, or Enable.
    Toggle(usize),
    Delete(usize),
    Closed,
}

/// Manage Notification Rules (Manage Rules... in the history). Its controls are JAWS 2025's
/// [obs, 784/s6-s10, 787/s8]: the list "Rules", then Modify..., Rename..., Disable and Delete,
/// available with a rule chosen, and Close.
#[derive(Debug, Clone)]
pub struct ManageRulesDialog {
    pub id: u32,
    rows: Vec<String>,
    selected: usize,
}

impl ManageRulesDialog {
    pub fn new(id: u32, rules: &NotificationRules) -> Self {
        Self {
            id,
            rows: rules.rules.iter().map(rule_row).collect(),
            selected: 0,
        }
    }

    pub fn spec(&self) -> DialogSpec {
        let any = !self.rows.is_empty();
        let button = |id: &str, label: StaticMessage| Control {
            enabled: any,
            ..control(id, label, ControlKind::Button { default: false })
        };
        DialogSpec {
            id: self.id,
            title: msg::notification_rules_title().to_string(),
            controls: vec![
                control(
                    "rules",
                    msg::notification_rules_list(),
                    ControlKind::ListBox {
                        items: self.rows.clone(),
                        selected: self.selected,
                    },
                ),
                button("modify", msg::notification_rules_modify()),
                button("rename", msg::notification_rules_rename()),
                button("toggle", msg::notification_rules_toggle()),
                button("delete", msg::notification_rules_delete()),
                control(
                    "cancel",
                    msg::notification_rules_close(),
                    ControlKind::Button { default: false },
                ),
            ],
        }
    }

    /// The rows after a change, the chosen one kept where it still is.
    pub fn set_rows(&mut self, rows: Vec<String>) -> Vec<okur_ui_model::ToUi> {
        use okur_ui_model::ToUi;
        let had = !self.rows.is_empty();
        self.rows = rows;
        self.selected = self.selected.min(self.rows.len().saturating_sub(1));
        let mut messages = vec![ToUi::DialogItems {
            id: self.id,
            control: "rules".to_owned(),
            items: self.rows.clone(),
            selected: self.selected,
        }];
        if had && self.rows.is_empty() {
            messages.extend(["modify", "rename", "toggle", "delete"].map(|control| {
                ToUi::DialogEnable {
                    id: self.id,
                    control: control.to_owned(),
                    enabled: false,
                }
            }));
        }
        messages
    }

    pub fn handle(&mut self, event: &FromUi) -> ManageOutcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                if let ("rules", Value::Selected(i)) = (control.as_str(), value) {
                    self.selected = *i;
                }
                ManageOutcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                let chosen = (self.selected < self.rows.len()).then_some(self.selected);
                match (control.as_str(), chosen) {
                    ("modify", Some(i)) => ManageOutcome::Modify(i),
                    ("rename", Some(i)) => ManageOutcome::Rename(i),
                    ("toggle", Some(i)) => ManageOutcome::Toggle(i),
                    ("delete", Some(i)) => ManageOutcome::Delete(i),
                    ("cancel", _) => ManageOutcome::Closed,
                    _ => ManageOutcome::Nothing,
                }
            }
            FromUi::DialogClosed { id } if *id == self.id => ManageOutcome::Closed,
            _ => ManageOutcome::Nothing,
        }
    }
}

/// Delete's question: "Confirm Delete | Are you sure you want to delete this rule? | Yes",
/// Yes the default [obs, 787/s17].
pub fn confirm_delete(id: u32) -> DialogSpec {
    DialogSpec {
        id,
        title: msg::notification_rules_confirm_title().to_string(),
        controls: vec![
            control(
                "question",
                msg::notification_rules_confirm(),
                ControlKind::Message {
                    text: msg::notification_rules_confirm().to_string(),
                },
            ),
            control(
                "ok",
                msg::notification_rules_yes(),
                ControlKind::Button { default: true },
            ),
            control(
                "cancel",
                msg::notification_rules_no(),
                ControlKind::Button { default: false },
            ),
        ],
    }
}

/// A notification's details: "Notification Details", its text in the read-only edit
/// "Notification Text:" [obs, 774/s11].
pub fn details(id: u32, text: &str) -> DialogSpec {
    DialogSpec {
        id,
        title: msg::notification_details_title().to_string(),
        controls: vec![control(
            "text",
            msg::notification_details_text(),
            ControlKind::Note {
                text: text.to_owned(),
            },
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a rule says for a notification: nothing for Mute and Play Sound, the whole for
    /// Speak the full message, Shorten's message, with a pattern's captures for \1 and \2
    /// and \\ for a backslash (JAWS 2025 help, Notification History).
    #[test]
    fn a_rule_says_the_notification_as_its_action_chooses() {
        const TEXT: &str = "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1";
        let mut rule = CreateRuleDialog::new(1, "Build finished", None).rule;
        assert_eq!(rule.spoken(TEXT), None);
        rule.speech = SpeechAction::PlaySound;
        assert_eq!(rule.spoken(TEXT), None);
        rule.speech = SpeechAction::SpeakFull;
        assert_eq!(rule.spoken(TEXT).as_deref(), Some(TEXT));
        rule.speech = SpeechAction::Shorten;
        rule.shortened = "Build done".into();
        assert_eq!(rule.spoken(TEXT).as_deref(), Some("Build done"));
        rule.kind = RuleType::MatchesPattern;
        rule.text = "^New notification from (.*?), (.*?),".into();
        rule.shortened = r"\2 from \1, \\1 kept, \3 none".into();
        assert_eq!(
            rule.spoken(TEXT).as_deref(),
            Some(r"SR Fixture from Windows PowerShell, \1 kept,  none")
        );
    }

    /// Play sound makes Path to Sound File, Select Sound... and Play Sound available, and Play
    /// Sound plays the file given [obs, 801/s10-s13].
    #[test]
    fn play_sound_shows_its_controls_and_plays_the_file() {
        let mut dialog = CreateRuleDialog::new(5, "Build finished", None);
        let chose = dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: "speech".into(),
            value: Value::Selected(2),
        });
        let RuleOutcome::Send(messages) = chose else {
            assert_eq!(chose, RuleOutcome::Nothing, "no Send");
            return;
        };
        let enabled: Vec<(String, bool)> = messages
            .into_iter()
            .filter_map(|m| match m {
                ToUi::DialogEnable {
                    control, enabled, ..
                } => Some((control, enabled)),
                _ => None,
            })
            .collect();
        assert_eq!(
            enabled,
            [
                ("shortened".to_owned(), false),
                ("sound".to_owned(), true),
                ("select_sound".to_owned(), true),
                ("play_sound".to_owned(), true),
            ]
        );
        let labels: Vec<String> = dialog
            .spec()
            .controls
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.label.clone())
            .collect();
        assert!(labels
            .windows(3)
            .any(|w| w == ["Path to Sound File:", "Select Sound...", "Play Sound"]));
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: "sound".into(),
            value: Value::Text(r"C:\Windows\Media\chimes.wav".into()),
        });
        assert_eq!(
            dialog.handle(&FromUi::DialogPressed {
                id: 5,
                control: "play_sound".into(),
            }),
            RuleOutcome::Play(r"C:\Windows\Media\chimes.wav".into())
        );
        // Select Sound...: the system's Open dialog for the sound's path.
        let open = dialog.handle(&FromUi::DialogPressed {
            id: 5,
            control: "select_sound".into(),
        });
        assert!(
            matches!(&open, RuleOutcome::Send(messages) if matches!(
                messages.as_slice(),
                [ToUi::FileOpen { id: 5, control, .. }] if control == "sound"
            )),
            "{open:?}"
        );
    }

    /// Shortened Text is available once Shorten is chosen.
    #[test]
    fn shortened_text_follows_shorten() {
        let mut dialog = CreateRuleDialog::new(5, "Build finished", None);
        let shortened = |dialog: &CreateRuleDialog| {
            dialog
                .spec()
                .controls
                .iter()
                .find(|c| c.id == "shortened")
                .map(|c| c.enabled)
        };
        assert_eq!(shortened(&dialog), Some(false));
        let chose = dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: "speech".into(),
            value: Value::Selected(1),
        });
        assert!(
            matches!(&chose, RuleOutcome::Send(messages) if messages.contains(&ToUi::DialogEnable {
                id: 5,
                control: "shortened".into(),
                enabled: true,
            })),
            "{chose:?}"
        );
        assert_eq!(shortened(&dialog), Some(true));
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: "shortened".into(),
            value: Value::Text("Build done".into()),
        });
        assert_eq!(dialog.rule.shortened, "Build done");
    }

    /// Create Rule's controls, in JAWS's order [obs, 775/s6-s13], and its rule.
    #[test]
    fn create_rule_is_jaws_and_makes_a_rule() {
        const TEXT: &str = "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1";
        let mut dialog = CreateRuleDialog::new(5, TEXT, Some("Windows PowerShell"));
        let spec = dialog.spec();
        assert_eq!(spec.title, "Create Rule");
        // The controls Tab reaches: Shortened Text waits for Shorten.
        let labels: Vec<&str> = spec
            .controls
            .iter()
            .filter(|c| c.enabled)
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(
            labels,
            [
                "Received Notification",
                "Rule Type",
                "",
                "Limit to notifications from Windows PowerShell",
                "Speech or Sound Action",
                "Speech Action",
                "Braille Action",
                "Braille Action",
                "Don't Show in History",
                "OK",
                "Cancel"
            ]
        );
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 5,
            control: "text".to_owned(),
            value: Value::Text("SR Fixture".to_owned()),
        });
        let RuleOutcome::Save(rule) = dialog.handle(&FromUi::DialogPressed {
            id: 5,
            control: "ok".to_owned(),
        }) else {
            panic_free_fail("no rule saved");
            return;
        };
        assert!(rule.matches(TEXT, Some("Windows PowerShell")));
        assert!(!rule.matches(TEXT, Some("Mail")));
        assert_eq!(rule.speech, SpeechAction::Mute);
        let pattern = Rule {
            name: "Sender".to_owned(),
            kind: RuleType::MatchesPattern,
            text: "^New notification from Windows PowerShell, (.*?),".to_owned(),
            sender: None,
            ..rule
        };
        assert!(pattern.matches(TEXT, None));
    }

    fn panic_free_fail(message: &str) {
        assert!(message.is_empty(), "{message}");
    }

    #[test]
    fn the_history_is_jaws_notification_history() {
        let toast = Notification {
            text: "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1".to_owned(),
            sender: Some("Windows PowerShell".to_owned()),
            time: "11:07 PM".to_owned(),
        };
        let mut dialog = HistoryDialog::new(4, vec![toast.row()], true, false);
        let spec = dialog.spec();
        assert_eq!(spec.title, "Notification History");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            // With JAWS's access keys (Alt+C for Create Rule... [obs, 787/s4]).
            [
                "Recent &Notifications",
                "&Enable Rules",
                "&Create Rule...",
                "&Manage Rules...",
                "C&lear History",
                "Close"
            ]
        );
        assert_eq!(
            spec.controls.first().map(|c| c.kind.clone()),
            Some(ControlKind::ListBox {
                items: vec!["New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1 Windows PowerShell 11:07 PM".to_owned()],
                selected: 0
            })
        );
        let pressed = |dialog: &mut HistoryDialog, control: &str| {
            dialog.handle(&FromUi::DialogPressed {
                id: 4,
                control: control.to_owned(),
            })
        };
        assert_eq!(pressed(&mut dialog, "notifications"), Outcome::Details(0));
        assert_eq!(pressed(&mut dialog, "clear"), Outcome::Clear);
        // Nothing left to show.
        assert_eq!(pressed(&mut dialog, "notifications"), Outcome::Nothing);
        assert_eq!(pressed(&mut dialog, "cancel"), Outcome::Closed);
    }
}
