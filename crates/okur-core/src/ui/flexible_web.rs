//! Flexible Web's wizard (Insert+Space, then X), as JAWS 2025's [obs, 705, 708, 716, 720,
//! 725, 726; the guide, Surf's Up chapter 16]:
//! - "Flexible Web": "What would you like to do?", five radio buttons, those not yet possible
//!   unavailable: Create a new customization, Retry and Undo (naming the customization made
//!   last), Save temporary customizations as a rule, View or change where rules apply; and
//!   Enable Flexible Web;
//! - "Choose an Action": "How would you like to customize the page?", Hide an element or Start
//!   reading at an element;
//! - "Choose an element": the elements at the virtual cursor, innermost first
//!   ([`crate::flexible_web::elements_at`]);
//! - "Choose a customization": what finds that element again
//!   ([`crate::flexible_web::customizations`]); Finish makes it;
//! - "Save Temporary Customizations", after Save temporary customizations as a rule, or
//!   opened alone when a page loads after customizations were made on the one before: "Rule
//!   Name:", the customizations made, each checked, and "How would you like this rule to
//!   apply?"; Finish, unavailable until the rule has a name, saves it [obs, 740, 742].
//!
//! Back, Next and Finish come from the wizard's window; Okur turns its pages.

use okur_ui_model::{
    Control, ControlKind, DialogKey, DialogSpec, FromUi, ToUi, TreeItem, Value, WizardButtons,
    WizardSpec,
};

use crate::aom::AomNode;
use crate::flexible_web::{customizations, Action, Area, Customization, Element, Rules, Scope};
use crate::messages::msg;

/// The wizard's pages, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Start,
    Action,
    Element,
    Customization,
    Save,
}

/// The first page's choices, in order.
const CREATE: usize = 0;
const RETRY: usize = 1;
const UNDO: usize = 2;
const SAVE: usize = 3;
const VIEW: usize = 4;

/// The scope a rule is saved with unless the user chooses another: "To the current site"
/// [obs, 742/s19].
const DEFAULT_SCOPE: usize = 1;

/// What the engine does after a message from the wizard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Turn the wizard to this page.
    Turn(DialogSpec, WizardButtons),
    /// Finish: make the customization; the wizard closes.
    Apply(Customization),
    /// Undo the customization made last; the wizard closes.
    Undo,
    /// Next with no element at the cursor to customize.
    NoElements,
    /// The page's buttons changed (Finish once the rule has a name).
    Buttons(WizardButtons),
    /// Finish on the last page: save the checked customizations as a rule; the wizard
    /// closes.
    Save {
        name: String,
        scope: Scope,
        customizations: Vec<Customization>,
    },
    /// Finish with View or change where rules apply chosen: open the rules dialog; the
    /// wizard closes [obs, 748/s6].
    View,
    /// Cancel or Escape: the wizard is gone.
    Closed,
}

fn control(id: &str, label: String, kind: ControlKind) -> Control {
    Control {
        id: id.to_owned(),
        label,
        kind,
        enabled: true,
    }
}

/// The wizard while it is open.
#[derive(Debug, Clone)]
pub struct FlexibleWebWizard {
    pub id: u32,
    page: Page,
    task: usize,
    /// The temporary customizations made on the page; Retry and Undo name the last.
    made: Vec<Customization>,
    /// Whether rules are saved, which View or change where rules apply needs.
    rules: bool,
    enabled: bool,
    action: Action,
    elements: Vec<Element>,
    element: usize,
    offered: Vec<Customization>,
    customization: usize,
    /// The last page's rule: its name, the customizations it keeps, where it applies.
    name: String,
    checked: Vec<bool>,
    scope: usize,
    /// Opened on the last page alone, with no page before it.
    saving_only: bool,
}

impl FlexibleWebWizard {
    /// The wizard on its first page, for the `elements` at the cursor, after the temporary
    /// customizations `made` on the page, `rules` saying whether rules are saved.
    pub fn new(id: u32, elements: Vec<Element>, made: Vec<Customization>, rules: bool) -> Self {
        Self {
            id,
            page: Page::Start,
            task: CREATE,
            checked: vec![true; made.len()],
            made,
            rules,
            enabled: true,
            action: Action::Hide,
            elements,
            element: 0,
            offered: Vec::new(),
            customization: 0,
            name: String::new(),
            scope: DEFAULT_SCOPE,
            saving_only: false,
        }
    }

    /// The wizard on its last page alone, saving `made` as a rule: as JAWS's opens when the
    /// question before another page loads is answered Yes [obs, 742/s31].
    pub fn saving(id: u32, made: Vec<Customization>) -> Self {
        Self {
            page: Page::Save,
            saving_only: true,
            ..Self::new(id, Vec::new(), made, false)
        }
    }

    /// The wizard as it opens.
    pub fn spec(&self) -> WizardSpec {
        let (page, buttons) = self.page_spec();
        WizardSpec {
            id: self.id,
            page,
            buttons,
        }
    }

    fn page_spec(&self) -> (DialogSpec, WizardButtons) {
        let (title, controls, buttons) = match self.page {
            Page::Start => {
                let (retry, undo) = match self.made.last().map(Customization::label) {
                    Some(last) => (
                        msg::flexible_web_retry(last.clone()).to_string(),
                        msg::flexible_web_undo(last).to_string(),
                    ),
                    None => (
                        msg::flexible_web_retry_none().to_string(),
                        msg::flexible_web_undo_none().to_string(),
                    ),
                };
                let made = !self.made.is_empty();
                (
                    msg::flexible_web_title(),
                    vec![
                        control(
                            "question",
                            String::new(),
                            ControlKind::Message {
                                text: msg::flexible_web_start_question().to_string(),
                            },
                        ),
                        control(
                            "task",
                            String::new(),
                            ControlKind::RadioButtons {
                                options: vec![
                                    msg::flexible_web_create().to_string(),
                                    retry,
                                    undo,
                                    msg::flexible_web_save().to_string(),
                                    msg::flexible_web_view().to_string(),
                                ],
                                selected: self.task,
                                enabled: vec![true, made, made, made, self.rules],
                            },
                        ),
                        control(
                            "enabled",
                            msg::flexible_web_enable().to_string(),
                            ControlKind::CheckBox {
                                checked: self.enabled,
                            },
                        ),
                    ],
                    self.start_buttons(),
                )
            }
            Page::Action => (
                msg::flexible_web_action_title(),
                vec![
                    control(
                        "question",
                        String::new(),
                        ControlKind::Message {
                            text: msg::flexible_web_action_question().to_string(),
                        },
                    ),
                    control(
                        "action",
                        String::new(),
                        ControlKind::RadioButtons {
                            options: vec![
                                msg::flexible_web_hide().to_string(),
                                msg::flexible_web_start_reading().to_string(),
                            ],
                            selected: usize::from(self.action == Action::StartAt),
                            enabled: vec![true, true],
                        },
                    ),
                ],
                WizardButtons {
                    back: true,
                    next: true,
                    finish: false,
                },
            ),
            Page::Element => (
                msg::flexible_web_element_title(),
                vec![control(
                    "element",
                    msg::flexible_web_element_question().to_string(),
                    ControlKind::ListBox {
                        items: self.elements.iter().map(Element::label).collect(),
                        selected: self.element,
                    },
                )],
                WizardButtons {
                    back: true,
                    next: true,
                    finish: false,
                },
            ),
            Page::Customization => (
                msg::flexible_web_customization_title(),
                vec![control(
                    "customization",
                    msg::flexible_web_customization_question().to_string(),
                    ControlKind::ListBox {
                        items: self.offered.iter().map(Customization::label).collect(),
                        selected: self.customization,
                    },
                )],
                WizardButtons {
                    back: true,
                    next: false,
                    finish: true,
                },
            ),
            Page::Save => (
                msg::flexible_web_save_title(),
                vec![
                    control(
                        "name",
                        msg::flexible_web_rule_name().to_string(),
                        ControlKind::Edit {
                            text: self.name.clone(),
                        },
                    ),
                    control(
                        "customizations",
                        String::new(),
                        ControlKind::CheckList {
                            columns: Vec::new(),
                            rows: self.made.iter().map(|c| vec![c.label()]).collect(),
                            renamable: false,
                            checked: self.checked.clone(),
                            selected: 0,
                        },
                    ),
                    control(
                        "scope",
                        msg::flexible_web_scope_question().to_string(),
                        ControlKind::Choice {
                            options: Scope::ALL.iter().map(|s| s.label()).collect(),
                            selected: self.scope,
                        },
                    ),
                ],
                self.save_buttons(),
            ),
        };
        (
            DialogSpec {
                id: self.id,
                title: title.to_string(),
                controls,
                keys: Vec::new(),
            },
            buttons,
        )
    }

    /// The first page's buttons: Next, or Finish in its place for View or change where rules
    /// apply [obs, 743/s34-s37].
    fn start_buttons(&self) -> WizardButtons {
        let view = self.task == VIEW;
        WizardButtons {
            back: false,
            next: !view,
            finish: view,
        }
    }

    /// The last page's buttons: Back unless it was opened alone, Finish once the rule has a
    /// name [obs, 740, 742/s26].
    fn save_buttons(&self) -> WizardButtons {
        WizardButtons {
            back: !self.saving_only,
            next: false,
            finish: !self.name.trim().is_empty(),
        }
    }

    fn turn(&mut self, page: Page) -> Outcome {
        self.page = page;
        let (spec, buttons) = self.page_spec();
        Outcome::Turn(spec, buttons)
    }

    /// Handles what the user did to the wizard, on `page` (the page in front, whose elements
    /// the customizations find); events of other windows are ignored.
    pub fn handle(&mut self, event: &FromUi, page: Option<&AomNode>) -> Outcome {
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("task", Value::Selected(i)) => {
                        let before = self.start_buttons();
                        self.task = *i;
                        let after = self.start_buttons();
                        if after != before {
                            return Outcome::Buttons(after);
                        }
                    }
                    ("enabled", Value::Checked(checked)) => self.enabled = *checked,
                    ("action", Value::Selected(i)) => {
                        self.action = if *i == 1 {
                            Action::StartAt
                        } else {
                            Action::Hide
                        };
                    }
                    ("element", Value::Selected(i)) => self.element = *i,
                    ("customization", Value::Selected(i)) => self.customization = *i,
                    ("name", Value::Text(text)) => {
                        let before = self.save_buttons();
                        text.clone_into(&mut self.name);
                        let after = self.save_buttons();
                        if after != before {
                            return Outcome::Buttons(after);
                        }
                    }
                    ("customizations", Value::ItemChecked(i, checked)) => {
                        if let Some(c) = self.checked.get_mut(*i) {
                            *c = *checked;
                        }
                    }
                    ("scope", Value::Selected(i)) if *i < Scope::ALL.len() => self.scope = *i,
                    _ => {}
                }
                Outcome::Nothing
            }
            FromUi::DialogPressed { id, control } if *id == self.id => {
                match (control.as_str(), self.page) {
                    ("next", Page::Start) => match self.task {
                        CREATE | RETRY if self.elements.is_empty() => Outcome::NoElements,
                        CREATE | RETRY => self.turn(Page::Action),
                        UNDO => Outcome::Undo,
                        SAVE => self.turn(Page::Save),
                        _ => Outcome::Nothing,
                    },
                    ("next", Page::Action) => self.turn(Page::Element),
                    ("next", Page::Element) => {
                        let element = self.elements.get(self.element).cloned();
                        self.offered = match (element, page) {
                            (Some(element), Some(page)) => {
                                customizations(page, &element, self.action)
                            }
                            _ => Vec::new(),
                        };
                        self.customization = 0;
                        self.turn(Page::Customization)
                    }
                    ("back", Page::Action) => self.turn(Page::Start),
                    ("back", Page::Element) => self.turn(Page::Action),
                    ("back", Page::Customization) => self.turn(Page::Element),
                    ("back", Page::Save) if !self.saving_only => self.turn(Page::Start),
                    ("finish", Page::Save) if !self.name.trim().is_empty() => Outcome::Save {
                        name: self.name.trim().to_owned(),
                        scope: Scope::ALL.get(self.scope).copied().unwrap_or(Scope::Site),
                        customizations: self
                            .made
                            .iter()
                            .zip(&self.checked)
                            .filter(|(_, checked)| **checked)
                            .map(|(c, _)| c.clone())
                            .collect(),
                    },
                    ("finish", Page::Start) if self.task == VIEW => Outcome::View,
                    ("finish", Page::Customization) => self
                        .offered
                        .get(self.customization)
                        .cloned()
                        .map_or(Outcome::Nothing, Outcome::Apply),
                    _ => Outcome::Nothing,
                }
            }
            FromUi::DialogClosed { id } if *id == self.id => Outcome::Closed,
            _ => Outcome::Nothing,
        }
    }
}

/// What the engine does after a message from the rules dialog or its question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RulesOutcome {
    Nothing,
    /// Messages for the dialog's windows.
    Send(Vec<ToUi>),
    /// Turn `rule` on or off in `area`.
    Set {
        rule: String,
        area: Area,
        on: bool,
    },
    /// Rename `from` to `to`, which no other rule has.
    Rename {
        from: String,
        to: String,
    },
    /// Delete `rule`: the question, `question`, answered Yes.
    Delete {
        rule: String,
        question: u32,
    },
    /// Close or Escape: the dialog is gone.
    Closed,
}

/// JAWS's View or Change Where Rules are Applied, opened from the wizard's first page [obs,
/// 748]: its text, a tree of the areas around the page in front (Global, its site, the
/// page; the site closed at first), and the rules as a list with check boxes under Rule and
/// Set for: checked where the rule is on in the area chosen, Set for naming the area that
/// says so. Space turns a rule on or off there; Delete asks, then deletes it everywhere.
#[derive(Debug, Clone)]
pub struct RulesDialog {
    pub id: u32,
    areas: [Area; 3],
    area: usize,
    rule: usize,
    /// The question before a rule is deleted, while it is open, and the rule.
    question: Option<(u32, String)>,
}

impl RulesDialog {
    /// The dialog for the page at `url`, on Global.
    pub fn new(id: u32, url: &str) -> Self {
        Self {
            id,
            areas: Area::around(url),
            area: 0,
            rule: 0,
            question: None,
        }
    }

    /// Whether `id` is the dialog's or its question's.
    pub fn owns(&self, id: u32) -> bool {
        id == self.id || self.question.as_ref().is_some_and(|(q, _)| *q == id)
    }

    /// The rules list's rows, and their checks, for the area chosen.
    fn rows(&self, rules: &Rules) -> (Vec<Vec<String>>, Vec<bool>) {
        rules
            .rules
            .iter()
            .map(|rule| {
                let (on, set_for) = rules.state(&rule.name, &self.areas, self.area);
                (vec![rule.name.clone(), set_for.set_for()], on)
            })
            .unzip()
    }

    pub fn spec(&self, rules: &Rules) -> DialogSpec {
        let (rows, checked) = self.rows(rules);
        DialogSpec {
            id: self.id,
            title: msg::flexible_web_rules_title().to_string(),
            controls: vec![
                control(
                    "areas",
                    String::new(),
                    ControlKind::Tree {
                        items: self
                            .areas
                            .iter()
                            .enumerate()
                            .map(|(level, area)| TreeItem {
                                text: area.label(),
                                level,
                                // The site closed, as JAWS's opens [obs, 748/s9].
                                open: level != 1,
                                checked: None,
                            })
                            .collect(),
                        selected: self.area,
                    },
                ),
                control(
                    "rules",
                    String::new(),
                    ControlKind::CheckList {
                        columns: vec![
                            msg::flexible_web_rules_column_rule().to_string(),
                            msg::flexible_web_rules_column_set_for().to_string(),
                        ],
                        rows,
                        checked,
                        selected: self.rule,
                        renamable: true,
                    },
                ),
                control(
                    "close",
                    msg::flexible_web_rules_close().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "text",
                    msg::flexible_web_rules_title().to_string(),
                    ControlKind::Note {
                        text: msg::flexible_web_rules_text().to_string(),
                    },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// What the tree says of the area named `name` after its place: how many rules are on
    /// there ("1 Active" [obs, 748/s13]).
    pub fn tree_status(&self, name: &str, rules: &Rules) -> Option<String> {
        let at = self.areas.iter().position(|area| area.label() == name)?;
        Some(msg::flexible_web_area_active(rules.active(&self.areas, at)).to_string())
    }

    /// The rules list as it stands, for the area chosen.
    pub fn refreshed(&mut self, rules: &Rules) -> ToUi {
        self.rule = self.rule.min(rules.rules.len().saturating_sub(1));
        let (rows, checked) = self.rows(rules);
        ToUi::DialogRows {
            id: self.id,
            control: "rules".to_owned(),
            rows,
            checked,
            selected: self.rule,
        }
    }

    /// The question before `rule` is deleted: "You are about to delete rule ..." with Yes and
    /// No, named as Okur names itself where JAWS's says "JAWS" [obs, 748/s20].
    fn question(id: u32, rule: &str) -> DialogSpec {
        DialogSpec {
            id,
            title: msg::flexible_web_delete_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::flexible_web_delete_question(rule.to_owned()).to_string(),
                    },
                ),
                control(
                    "yes",
                    msg::flexible_web_delete_yes().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "no",
                    msg::flexible_web_delete_no().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// What the user did to the dialog or its question, on `rules`; `next_id` numbers the
    /// question.
    pub fn handle(&mut self, event: &FromUi, rules: &Rules, next_id: u32) -> RulesOutcome {
        if let Some((question, rule)) = self.question.clone() {
            let answer = match event {
                FromUi::DialogPressed { id, control } if *id == question => Some(control == "yes"),
                FromUi::DialogClosed { id } if *id == question => Some(false),
                _ => None,
            };
            if let Some(yes) = answer {
                self.question = None;
                return if yes {
                    RulesOutcome::Delete { rule, question }
                } else {
                    RulesOutcome::Send(vec![ToUi::DialogClose { id: question }])
                };
            }
        }
        match event {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                match (control.as_str(), value) {
                    ("areas", Value::Selected(i)) if *i < self.areas.len() && *i != self.area => {
                        self.area = *i;
                        RulesOutcome::Send(vec![self.refreshed(rules)])
                    }
                    ("rules", Value::Selected(i)) => {
                        self.rule = *i;
                        RulesOutcome::Nothing
                    }
                    ("rules", Value::ItemRenamed(i, to)) => match rules.rules.get(*i) {
                        Some(rule)
                            if !to.trim().is_empty()
                                && rule.name != *to
                                && !rules.rules.iter().any(|r| r.name == *to) =>
                        {
                            RulesOutcome::Rename {
                                from: rule.name.clone(),
                                to: to.trim().to_owned(),
                            }
                        }
                        _ => RulesOutcome::Nothing,
                    },
                    // A check turned: only a change from the rule's state there is the user's.
                    ("rules", Value::ItemChecked(i, on)) => {
                        match (rules.rules.get(*i), self.areas.get(self.area)) {
                            (Some(rule), Some(area))
                                if rules.state(&rule.name, &self.areas, self.area).0 != *on =>
                            {
                                RulesOutcome::Set {
                                    rule: rule.name.clone(),
                                    area: area.clone(),
                                    on: *on,
                                }
                            }
                            _ => RulesOutcome::Nothing,
                        }
                    }
                    _ => RulesOutcome::Nothing,
                }
            }
            FromUi::DialogKey {
                id,
                control,
                key: DialogKey::Delete,
            } if *id == self.id && control == "rules" => match rules.rules.get(self.rule) {
                Some(rule) => {
                    self.question = Some((next_id, rule.name.clone()));
                    RulesOutcome::Send(vec![ToUi::DialogShow {
                        dialog: Self::question(next_id, &rule.name),
                    }])
                }
                None => RulesOutcome::Nothing,
            },
            FromUi::DialogPressed { id, control } if *id == self.id && control == "close" => {
                RulesOutcome::Closed
            }
            FromUi::DialogClosed { id } if *id == self.id => RulesOutcome::Closed,
            _ => RulesOutcome::Nothing,
        }
    }
}

/// The question before the temporary customizations made on a page are lost to another page
/// loading: "Flexible Web", the question, Yes (the default, with the focus) and No [obs,
/// 742/s30]. JAWS says the dialog and Yes without the question; Okur reads it, as JAWS
/// reads its other questions [obs, 702/s27] (`docs/references/accepted-divergences.md`).
#[derive(Debug, Clone)]
pub struct UnsavedQuestion {
    pub id: u32,
}

impl UnsavedQuestion {
    pub fn spec(&self) -> DialogSpec {
        DialogSpec {
            id: self.id,
            title: msg::flexible_web_title().to_string(),
            controls: vec![
                control(
                    "question",
                    String::new(),
                    ControlKind::Message {
                        text: msg::flexible_web_unsaved_question().to_string(),
                    },
                ),
                control(
                    "yes",
                    msg::flexible_web_unsaved_yes().to_string(),
                    ControlKind::Button { default: true },
                ),
                control(
                    "no",
                    msg::flexible_web_unsaved_no().to_string(),
                    ControlKind::Button { default: false },
                ),
            ],
            keys: Vec::new(),
        }
    }

    /// The answer `event` gives: yes, or no (No, or the question closed); none for anything
    /// else.
    pub fn answer(&self, event: &FromUi) -> Option<bool> {
        match event {
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "yes" => Some(true),
                "no" => Some(false),
                _ => None,
            },
            FromUi::DialogClosed { id } if *id == self.id => Some(false),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aom::Role;
    use crate::flexible_web::elements_at;

    fn page() -> AomNode {
        let mut main = AomNode::new(
            1,
            Role::Landmark {
                landmark_type: "main".into(),
            },
        )
        .with_children(vec![
            AomNode::new(2, Role::Heading { level: 1 }).with_text("Weekend events")
        ]);
        main.name = None;
        AomNode::new(0, Role::Document).with_children(vec![main])
    }

    /// "Hide MainRegion 1 from the top".
    fn hide_main() -> Customization {
        Customization {
            action: Action::Hide,
            kind: crate::flexible_web::Kind::Region("main".into()),
            text: None,
            id: None,
            place: crate::flexible_web::Place::FromTop(1),
        }
    }

    fn pressed(control: &str) -> FromUi {
        FromUi::DialogPressed {
            id: 7,
            control: control.into(),
        }
    }

    fn texts(spec: &DialogSpec) -> Vec<String> {
        spec.controls
            .iter()
            .map(|c| match &c.kind {
                ControlKind::Message { text } => text.clone(),
                ControlKind::RadioButtons { options, .. } => options.join(" | "),
                ControlKind::ListBox { items, .. } => format!("{}: {}", c.label, items.join(" | ")),
                _ => c.label.clone(),
            })
            .collect()
    }

    /// The pages as JAWS's wizard has them, from the heading: Next twice, the element list,
    /// Next, the customizations; Finish makes the first [obs, 705/s3, s9, s15; 708/s15].
    #[test]
    fn the_wizard_goes_through_its_pages() {
        let page = page();
        let mut wizard = FlexibleWebWizard::new(7, elements_at(&page, 2), Vec::new(), false);
        let start = wizard.spec();
        assert_eq!(start.page.title, "Flexible Web");
        assert_eq!(
            texts(&start.page),
            [
                "What would you like to do?",
                "Create a new customization | &Retry  . . . | &Undo  . . . | &Save temporary customizations as a rule | &View or change where rules apply",
                "Enable Flexible Web"
            ]
        );
        assert!(matches!(
            &start.page.controls.get(1).map(|c| &c.kind),
            Some(ControlKind::RadioButtons { enabled, .. }) if enabled == &[true, false, false, false, false]
        ));
        let outcome = wizard.handle(&pressed("next"), Some(&page));
        assert!(matches!(outcome, Outcome::Turn(..)), "{outcome:?}");
        let Outcome::Turn(action, _) = outcome else {
            return;
        };
        assert_eq!(action.title, "Choose an Action");
        let outcome = wizard.handle(&pressed("next"), Some(&page));
        assert!(matches!(outcome, Outcome::Turn(..)), "{outcome:?}");
        let Outcome::Turn(element, _) = outcome else {
            return;
        };
        assert_eq!(
            texts(&element),
            ["Which of the following elements would you like to customize?: heading level 1 Weekend events | MainRegion"]
        );
        wizard.handle(
            &FromUi::DialogChanged {
                id: 7,
                control: "element".into(),
                value: Value::Selected(1),
            },
            Some(&page),
        );
        let outcome = wizard.handle(&pressed("next"), Some(&page));
        assert!(matches!(outcome, Outcome::Turn(..)), "{outcome:?}");
        let Outcome::Turn(choice, buttons) = outcome else {
            return;
        };
        assert_eq!(choice.title, "Choose a customization");
        assert!(buttons.finish && !buttons.next && buttons.back);
        assert_eq!(
            texts(&choice),
            ["Select a customization: Hide MainRegion 1 from the top | Hide MainRegion 1 from the bottom | Hide all MainRegions"]
        );
        let made = wizard.handle(&pressed("finish"), Some(&page));
        assert!(
            matches!(&made, Outcome::Apply(c) if c.label() == "Hide MainRegion 1 from the top"),
            "{made:?}"
        );
    }

    /// After a customization, Retry and Undo name it; Undo with Next undoes it [obs, 716/s25,
    /// s26, s29].
    #[test]
    fn undo_names_the_customization_and_undoes_it() {
        let page = page();
        let mut wizard = FlexibleWebWizard::new(7, elements_at(&page, 2), vec![hide_main()], false);
        let start = wizard.spec();
        assert!(texts(&start.page)
            .get(1)
            .is_some_and(|t| t.contains("&Undo Hide MainRegion 1 from the top")));
        wizard.handle(
            &FromUi::DialogChanged {
                id: 7,
                control: "task".into(),
                value: Value::Selected(UNDO),
            },
            Some(&page),
        );
        assert_eq!(wizard.handle(&pressed("next"), Some(&page)), Outcome::Undo);
        assert_eq!(
            wizard.handle(&FromUi::DialogClosed { id: 7 }, Some(&page)),
            Outcome::Closed
        );
        let mut nothing = FlexibleWebWizard::new(8, Vec::new(), Vec::new(), false);
        assert_eq!(
            nothing.handle(
                &FromUi::DialogPressed {
                    id: 8,
                    control: "next".into()
                },
                Some(&page)
            ),
            Outcome::NoElements
        );
    }

    fn changed(control: &str, value: Value) -> FromUi {
        FromUi::DialogChanged {
            id: 7,
            control: control.into(),
            value,
        }
    }

    /// Save temporary customizations as a rule, then Next: the rule's name, the customizations
    /// checked, where it applies (To the current site first); Finish once the name is typed
    /// [obs, 742/s16-s28].
    #[test]
    fn the_last_page_saves_a_named_rule() {
        let page = page();
        let mut wizard = FlexibleWebWizard::new(7, Vec::new(), vec![hide_main()], false);
        wizard.handle(&changed("task", Value::Selected(SAVE)), Some(&page));
        let outcome = wizard.handle(&pressed("next"), Some(&page));
        assert!(matches!(outcome, Outcome::Turn(..)), "{outcome:?}");
        let Outcome::Turn(save, buttons) = outcome else {
            return;
        };
        assert_eq!(save.title, "Save Temporary Customizations");
        let kinds: Vec<(&str, &str, &ControlKind)> = save
            .controls
            .iter()
            .map(|c| (c.id.as_str(), c.label.as_str(), &c.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                (
                    "name",
                    "Rule Name:",
                    &ControlKind::Edit {
                        text: String::new()
                    }
                ),
                (
                    "customizations",
                    "",
                    &ControlKind::CheckList {
                        columns: Vec::new(),
                        rows: vec![vec!["Hide MainRegion 1 from the top".into()]],
                        checked: vec![true],
                        selected: 0,
                        renamable: false,
                    }
                ),
                (
                    "scope",
                    "How would you like this rule to apply?",
                    &ControlKind::Choice {
                        options: vec![
                            "Globally".into(),
                            "To the current site".into(),
                            "To the current page".into()
                        ],
                        selected: 1,
                    }
                ),
            ]
        );
        assert_eq!(
            buttons,
            WizardButtons {
                back: true,
                next: false,
                finish: false
            }
        );
        // Finish does nothing before the rule has a name.
        assert_eq!(
            wizard.handle(&pressed("finish"), Some(&page)),
            Outcome::Nothing
        );
        assert_eq!(
            wizard.handle(
                &changed("name", Value::Text("Okur lab rule".into())),
                Some(&page)
            ),
            Outcome::Buttons(WizardButtons {
                back: true,
                next: false,
                finish: true
            })
        );
        wizard.handle(&changed("scope", Value::Selected(2)), Some(&page));
        assert_eq!(
            wizard.handle(&pressed("finish"), Some(&page)),
            Outcome::Save {
                name: "Okur lab rule".into(),
                scope: Scope::Page,
                customizations: vec![hide_main()],
            }
        );
    }

    /// Opened alone, from the question before another page loads, the last page has nothing
    /// before it [obs, 742/s31].
    #[test]
    fn the_last_page_alone_has_no_back() {
        let wizard = FlexibleWebWizard::saving(7, vec![hide_main()]);
        let spec = wizard.spec();
        assert_eq!(spec.page.title, "Save Temporary Customizations");
        assert!(!spec.buttons.back && !spec.buttons.next && !spec.buttons.finish);
    }
}
