//! Skim Reading (Ctrl+Shift+Insert+Down, JAWS's SkimReadDialog): how Ctrl+Insert+Down skims a
//! page. Its controls are JAWS 2025's, as its dialog reports them [obs, 760, 763]: Use Rule
//! Type, of four; the text rules' controls (Rules History, Text Search Pattern, Use regular
//! expression, Speak Unit, Delete Rule, Delete All Rules) and the attribute and color rules'
//! (Text Color, Background Color, Bold to Outline), unavailable with the first two rule types;
//! Skim Reading Indication; and the buttons Start Skim Reading, Create Summary (the default),
//! Close and Help. Tab goes from the rule type to Skim Reading Indication and through the
//! buttons [obs, 760/s2-s7].

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value};

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

/// The dialog's choices, kept from one opening to the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkimOptions {
    pub rule: SkimRule,
    /// Skim Reading Indication, checked at first [obs, 760/s2].
    pub indication: bool,
}

impl Default for SkimOptions {
    fn default() -> Self {
        Self {
            rule: SkimRule::FirstLine,
            indication: true,
        }
    }
}

/// What the engine does after a message from the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
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
        // The rules' controls, for the rule types Okur does not read by yet.
        let rule_choice = |id: &str, label: StaticMessage, option: StaticMessage| {
            control(
                id,
                label,
                ControlKind::Choice {
                    options: vec![option.to_string()],
                    selected: 0,
                },
                false,
            )
        };
        let unavailable =
            |id: &str, label: StaticMessage, kind: ControlKind| control(id, label, kind, false);
        let unchecked = || ControlKind::CheckBox { checked: false };
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
                    text: String::new(),
                },
            ),
            unavailable("regex", msg::skim_regex(), unchecked()),
            rule_choice("unit", msg::skim_unit(), msg::skim_unit_lines()),
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
            .map(|(id, label)| unavailable(id, label, unchecked())),
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
                        if let Some(&rule) = SkimRule::ALL.get(*i) {
                            self.options.rule = rule;
                        }
                    }
                    ("indication", Value::Checked(on)) => self.options.indication = *on,
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "start" => Outcome::Start(self.options),
                "summary" => Outcome::Summary(self.options),
                "cancel" => Outcome::Closed(self.options),
                _ => Outcome::Nothing,
            },
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed(self.options),
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
            .filter(|c| c.enabled)
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
            indication: true,
        };
        let pressed = |dialog: &mut SkimDialog, control: &str| {
            dialog.handle(&FromUi::DialogPressed {
                id: 1,
                control: control.to_owned(),
            })
        };
        assert_eq!(pressed(&mut dialog, "start"), Outcome::Start(chosen));
        assert_eq!(pressed(&mut dialog, "summary"), Outcome::Summary(chosen));
        assert_eq!(
            dialog.handle(&FromUi::DialogClosed { id: 1 }),
            Outcome::Closed(chosen)
        );
    }
}
