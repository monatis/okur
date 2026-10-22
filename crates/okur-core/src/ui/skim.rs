//! Skim Reading (Ctrl+Shift+Insert+Down, JAWS's SkimReadDialog): how Ctrl+Insert+Down skims a
//! page. Its controls are JAWS 2025's, as its dialog reports them [obs, 760, 763, 776]: Use
//! Rule Type, of four; the text rules' controls (Rules History, Text Search Pattern, Speak
//! Unit, Use regular expression, Delete Rule, Delete All Rules) and the attribute and color
//! rules' (Text Color, Background Color, Bold to Outline); Skim Reading Indication; and the
//! buttons Start Skim Reading, Create Summary (the default), Close and Help. The rules'
//! controls are available by the rule type ([`available`]): with the first line or sentence,
//! Tab goes from the rule type to Skim Reading Indication [obs, 760/s2-s7]; with text rules,
//! through Rules History, Text Search Pattern, Speak Unit and Use regular expression first
//! [obs, 776/s4-s8].

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, ToUi, Value};

use crate::messages::{msg, StaticMessage};

/// What Ctrl+Insert+Down reads of each paragraph: Use Rule Type's options, in their order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkimRule {
    /// Read First Line of Every Paragraph, JAWS's default [obs, 760/s1].
    #[default]
    FirstLine,
    /// Read First Sentence of Every Paragraph.
    FirstSentence,
    /// Text Rules Apply for Reading: the lines a text rule finds.
    TextRules,
    /// Attributes and Color Rules Apply for Reading: the text in an attribute or color.
    Attributes,
}

impl SkimRule {
    pub const ALL: [Self; 4] = [
        Self::FirstLine,
        Self::FirstSentence,
        Self::TextRules,
        Self::Attributes,
    ];

    fn label(self) -> StaticMessage {
        match self {
            Self::FirstLine => msg::skim_rule_first_line(),
            Self::FirstSentence => msg::skim_rule_first_sentence(),
            Self::TextRules => msg::skim_rule_text(),
            Self::Attributes => msg::skim_rule_attributes(),
        }
    }
}

/// What a text rule reads of the text it finds: Speak Unit's options [obs, 777/s6-s8].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpeakUnit {
    #[default]
    Lines,
    Sentences,
    Paragraphs,
}

impl SpeakUnit {
    pub const ALL: [Self; 3] = [Self::Lines, Self::Sentences, Self::Paragraphs];
}

/// The dialog's choices, kept from one opening to the next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkimOptions {
    pub rule: SkimRule,
    /// The attribute check boxes checked, by their control ids ("bold", "italic"): text
    /// with all of them is what an attribute rule finds.
    pub attributes: Vec<String>,
    /// Skim Reading Indication, checked at first [obs, 760/s2].
    pub indication: bool,
    /// Text Search Pattern: what a text rule looks for.
    pub pattern: String,
    /// Use regular expression: the pattern is one.
    pub regex: bool,
    pub unit: SpeakUnit,
}

impl Default for SkimOptions {
    fn default() -> Self {
        Self {
            rule: SkimRule::FirstLine,
            attributes: Vec::new(),
            indication: true,
            pattern: String::new(),
            regex: false,
            unit: SpeakUnit::Lines,
        }
    }
}

/// The attribute check boxes, by control id, in JAWS's order.
pub const ATTRIBUTES: [&str; 9] = [
    "bold",
    "italic",
    "underline",
    "strikeout",
    "double_strikeout",
    "superscript",
    "subscript",
    "shadow",
    "outline",
];

impl SkimOptions {
    /// Whether text formatted as `formatting` has every attribute checked ("The selections
    /// made are cumulative", JAWS 2025 help, Skim Reading Dialog): bold, italic and
    /// underline, which Okur reads from the page; the others, which it does not, find
    /// nothing. With none checked, nothing is found.
    pub fn has_attributes(&self, formatting: &crate::aom::TextFormatting) -> bool {
        !self.attributes.is_empty()
            && self.attributes.iter().all(|a| match a.as_str() {
                "bold" => formatting.bold,
                "italic" => formatting.italic,
                "underline" => formatting.underline,
                _ => false,
            })
    }

    /// Whether a text rule finds its pattern in `text`, whatever its capitals. A pattern that is
    /// not a regular expression finds nothing.
    pub fn finds(&self, text: &str) -> bool {
        let pattern = self.pattern.trim();
        if pattern.is_empty() {
            return false;
        }
        if self.regex {
            return regex::RegexBuilder::new(pattern)
                .case_insensitive(true)
                .build()
                .is_ok_and(|r| r.is_match(text));
        }
        text.to_lowercase().contains(&pattern.to_lowercase())
    }
}

/// The rules' controls, in the dialog's order.
const RULE_CONTROLS: [&str; 17] = [
    "history",
    "pattern",
    "unit",
    "regex",
    "delete",
    "delete_all",
    "text_color",
    "background",
    "bold",
    "italic",
    "underline",
    "strikeout",
    "double_strikeout",
    "superscript",
    "subscript",
    "shadow",
    "outline",
];

/// Whether a rules' control is available with `rule` chosen: Rules History, Text Search
/// Pattern, Speak Unit and Use regular expression with text rules [obs, 776/s4-s7]; Rules
/// History, the colors and the attributes with attribute and color rules [obs, 777/s15-s19];
/// Delete Rule and Delete All Rules once a rule is saved, which Okur does not yet.
fn available(rule: SkimRule, control: &str) -> bool {
    match control {
        "history" => matches!(rule, SkimRule::TextRules | SkimRule::Attributes),
        "pattern" | "unit" | "regex" => rule == SkimRule::TextRules,
        "delete" | "delete_all" => false,
        _ => rule == SkimRule::Attributes,
    }
}

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Send these to the dialog: the rules' controls made available or not.
    Send(Vec<ToUi>),
    /// Start Skim Reading: the dialog closes and skim reading starts with these choices.
    Start(SkimOptions),
    /// Create Summary: the dialog closes and the summary shows.
    Summary(SkimOptions),
    /// Closed with Close or Escape, the choices kept.
    Closed(SkimOptions),
}

/// Skim Reading while it is open.
#[derive(Debug, Clone)]
pub struct SkimDialog {
    pub id: u32,
    options: SkimOptions,
}

impl SkimDialog {
    pub fn new(id: u32, options: SkimOptions) -> Self {
        Self { id, options }
    }

    pub fn spec(&self) -> DialogSpec {
        let control = |id: &str, label: StaticMessage, kind: ControlKind, enabled: bool| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled,
        };
        let button = |id: &str, label: StaticMessage, default: bool| {
            control(id, label, ControlKind::Button { default }, true)
        };
        let rule = self.options.rule;
        let rule_choice = |id: &str, label: StaticMessage, option: StaticMessage| {
            control(
                id,
                label,
                ControlKind::Choice {
                    options: vec![option.to_string()],
                    selected: 0,
                },
                available(rule, id),
            )
        };
        let unavailable = |id: &str, label: StaticMessage, kind: ControlKind| {
            control(id, label, kind, available(rule, id))
        };
        let mut controls = vec![
            control(
                "rule",
                msg::skim_rule(),
                ControlKind::Radios {
                    options: SkimRule::ALL
                        .iter()
                        .map(|rule| rule.label().to_string())
                        .collect(),
                    selected: SkimRule::ALL
                        .iter()
                        .position(|&rule| rule == self.options.rule)
                        .unwrap_or(0),
                },
                true,
            ),
            rule_choice("history", msg::skim_rules_history(), msg::skim_rules_new()),
            unavailable(
                "pattern",
                msg::skim_pattern(),
                ControlKind::Edit {
                    text: self.options.pattern.clone(),
                },
            ),
            control(
                "unit",
                msg::skim_unit(),
                ControlKind::Choice {
                    options: [
                        msg::skim_unit_lines(),
                        msg::skim_unit_sentences(),
                        msg::skim_unit_paragraphs(),
                    ]
                    .iter()
                    .map(ToString::to_string)
                    .collect(),
                    selected: SpeakUnit::ALL
                        .iter()
                        .position(|&unit| unit == self.options.unit)
                        .unwrap_or(0),
                },
                available(rule, "unit"),
            ),
            unavailable(
                "regex",
                msg::skim_regex(),
                ControlKind::CheckBox {
                    checked: self.options.regex,
                },
            ),
            unavailable(
                "delete",
                msg::skim_delete(),
                ControlKind::Button { default: false },
            ),
            unavailable(
                "delete_all",
                msg::skim_delete_all(),
                ControlKind::Button { default: false },
            ),
            control("colors", msg::skim_color_rules(), ControlKind::Group, true),
            rule_choice("text_color", msg::skim_text_color(), msg::skim_any_color()),
            rule_choice("background", msg::skim_background(), msg::skim_any_color()),
        ];
        controls.extend(
            [
                ("bold", msg::skim_bold()),
                ("italic", msg::skim_italic()),
                ("underline", msg::skim_underline()),
                ("strikeout", msg::skim_strikeout()),
                ("double_strikeout", msg::skim_double_strikeout()),
                ("superscript", msg::skim_superscript()),
                ("subscript", msg::skim_subscript()),
                ("shadow", msg::skim_shadow()),
                ("outline", msg::skim_outline()),
            ]
            .into_iter()
            .map(|(id, label)| {
                let checked = self.options.attributes.iter().any(|a| a == id);
                unavailable(id, label, ControlKind::CheckBox { checked })
            }),
        );
        controls.extend([
            control(
                "indication",
                msg::skim_indication(),
                ControlKind::CheckBox {
                    checked: self.options.indication,
                },
                true,
            ),
            button("start", msg::skim_start(), false),
            button("summary", msg::skim_summary(), true),
            button("cancel", msg::skim_close(), false),
            button("help", msg::skim_help(), false),
        ]);
        DialogSpec {
            id: self.id,
            title: msg::skim_dialog().to_string(),
            controls,
        }
    }

    pub fn handle(&mut self, event: &FromUi) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("rule", Value::Selected(i)) => {
                        let Some(&rule) = SkimRule::ALL.get(*i) else {
                            return Outcome::Nothing;
                        };
                        let before = std::mem::replace(&mut self.options.rule, rule);
                        let changes: Vec<ToUi> = RULE_CONTROLS
                            .iter()
                            .filter(|id| available(before, id) != available(rule, id))
                            .map(|id| ToUi::DialogEnable {
                                id: self.id,
                                control: (*id).to_owned(),
                                enabled: available(rule, id),
                            })
                            .collect();
                        if !changes.is_empty() {
                            return Outcome::Send(changes);
                        }
                    }
                    ("indication", Value::Checked(on)) => self.options.indication = *on,
                    ("pattern", Value::Text(text)) => self.options.pattern = text.clone(),
                    ("regex", Value::Checked(on)) => self.options.regex = *on,
                    (attribute, Value::Checked(on)) if ATTRIBUTES.contains(&attribute) => {
                        self.options.attributes.retain(|a| a != attribute);
                        if *on {
                            self.options.attributes.push(attribute.to_owned());
                        }
                    }
                    ("unit", Value::Selected(i)) => {
                        if let Some(&unit) = SpeakUnit::ALL.get(*i) {
                            self.options.unit = unit;
                        }
                    }
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "start" => Outcome::Start(self.options.clone()),
                "summary" => Outcome::Summary(self.options.clone()),
                "cancel" => Outcome::Closed(self.options.clone()),
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed(self.options.clone()),
            _ => Outcome::Nothing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dialog_is_jaws_skim_reading() {
        let spec = SkimDialog::new(2, SkimOptions::default()).spec();
        assert_eq!(spec.title, "Skim Reading");
        // Tab's stops, as JAWS's [obs, 760/s1-s7]: the rule type, Skim Reading Indication,
        // and the buttons, Create Summary the default.
        let stops: Vec<&str> = spec
            .controls
            .iter()
            .filter(|c| c.enabled && c.kind != ControlKind::Group)
            .map(|c| c.label.as_str())
            .collect();
        assert_eq!(
            stops,
            [
                "Use Rule Type:",
                "Skim Reading &Indication",
                "&Start Skim Reading",
                "Create Summar&y",
                "&Close",
                "&Help"
            ]
        );
        assert!(spec
            .controls
            .iter()
            .any(|c| c.id == "summary" && c.kind == ControlKind::Button { default: true }));
    }

    /// Text rules make their controls available, in JAWS's Tab order [obs, 776/s4-s8].
    #[test]
    fn text_rules_make_their_controls_available() {
        let mut dialog = SkimDialog::new(1, SkimOptions::default());
        let outcome = dialog.handle(&FromUi::DialogChanged {
            id: 1,
            control: "rule".to_owned(),
            value: Value::Selected(2),
        });
        let enabled: Vec<(String, bool)> = match outcome {
            Outcome::Send(messages) => messages
                .into_iter()
                .filter_map(|m| match m {
                    ToUi::DialogEnable {
                        control, enabled, ..
                    } => Some((control, enabled)),
                    _ => None,
                })
                .collect(),
            other => no_messages(other),
        };
        let expected: Vec<(String, bool)> = ["history", "pattern", "unit", "regex"]
            .iter()
            .map(|id| ((*id).to_owned(), true))
            .collect();
        assert_eq!(enabled, expected);
        let stops: Vec<&str> = {
            let spec = dialog.spec();
            spec.controls
                .iter()
                .filter(|c| c.enabled && c.kind != ControlKind::Group)
                .map(|c| c.id.as_str())
                .collect::<Vec<_>>()
                .into_iter()
                .map(|id| match id {
                    "rule" => "rule",
                    "history" => "history",
                    "pattern" => "pattern",
                    "unit" => "unit",
                    "regex" => "regex",
                    "indication" => "indication",
                    _ => "button",
                })
                .collect()
        };
        assert_eq!(
            stops,
            [
                "rule",
                "history",
                "pattern",
                "unit",
                "regex",
                "indication",
                "button",
                "button",
                "button",
                "button"
            ]
        );
    }

    /// An outcome that is not messages: none to compare.
    fn no_messages(outcome: Outcome) -> Vec<(String, bool)> {
        assert_eq!(outcome, Outcome::Nothing, "expected messages");
        Vec::new()
    }

    #[test]
    fn the_choices_go_with_the_buttons() {
        let mut dialog = SkimDialog::new(1, SkimOptions::default());
        let _ = dialog.handle(&FromUi::DialogChanged {
            id: 1,
            control: "rule".to_owned(),
            value: Value::Selected(1),
        });
        let chosen = SkimOptions {
            rule: SkimRule::FirstSentence,
            ..SkimOptions::default()
        };
        let pressed = |dialog: &mut SkimDialog, control: &str| {
            dialog.handle(&FromUi::DialogPressed {
                id: 1,
                control: control.to_owned(),
            })
        };
        assert_eq!(
            pressed(&mut dialog, "start"),
            Outcome::Start(chosen.clone())
        );
        assert_eq!(
            pressed(&mut dialog, "summary"),
            Outcome::Summary(chosen.clone())
        );
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 1 }),
            Outcome::Closed(chosen)
        );
    }
}
