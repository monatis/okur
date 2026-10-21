//! The Notification History (Insert+Space, N) and the details of a notification. Its controls
//! are JAWS 2025's [obs, 774]: the list "Recent Notifications", the latest first, each row a
//! notification's text, its sender and the time it came ("New notification from Windows
//! PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1 Windows PowerShell 11:07 PM");
//! then Enable Rules (checked), Create Rule..., Clear History and Close, in that Tab order.
//! Enter on a row shows its details: "Notification Details", the text in a read-only edit
//! "Notification Text:" [obs, 774/s11]. JAWS keeps the past 24 hours (JAWS 2025 help,
//! Notification History).

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};
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
    /// Another message in its place (its Shortened Text, not built yet: said whole).
    Shorten,
    /// A sound in its place (not built yet: nothing said).
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
        self.rules.iter().find(|rule| rule.matches(text, sender))
    }
}

/// Where the notification rules are kept between runs (Okur's settings folder).
pub trait NotificationRuleStore: Send {
    fn save(&self, rules: &NotificationRules);
}

impl Rule {
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
            },
            from: sender.map(str::to_owned),
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
            title: msg::notification_rule_title().to_string(),
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
                    0,
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
                    0,
                ),
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
                    ControlKind::CheckBox { checked: false },
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
                        }
                    }
                    ("braille", Value::Selected(i)) => self.rule.braille = *i,
                    ("hidden", Value::Checked(on)) => self.rule.hidden = *on,
                    _ => {}
                }
                RuleOutcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "ok" if !self.rule.text.trim().is_empty() => RuleOutcome::Save(self.rule.clone()),
                "cancel" => RuleOutcome::Closed,
                _ => RuleOutcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => RuleOutcome::Closed,
            _ => RuleOutcome::Nothing,
        }
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

    /// Create Rule's controls, in JAWS's order [obs, 775/s6-s13], and its rule.
    #[test]
    fn create_rule_is_jaws_and_makes_a_rule() {
        const TEXT: &str = "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1";
        let mut dialog = CreateRuleDialog::new(5, TEXT, Some("Windows PowerShell"));
        let spec = dialog.spec();
        assert_eq!(spec.title, "Create Rule");
        let labels: Vec<&str> = spec.controls.iter().map(|c| c.label.as_str()).collect();
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
            [
                "Recent Notifications",
                "Enable Rules",
                "Create Rule...",
                "Manage Rules...",
                "Clear History",
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
