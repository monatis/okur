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
//!   ([`crate::flexible_web::customizations`]); Finish makes it.
//!
//! Back, Next and Finish come from the wizard's window; Okur turns its pages.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value, WizardButtons, WizardSpec};

use crate::aom::AomNode;
use crate::flexible_web::{customizations, Action, Customization, Element};
use crate::messages::msg;

/// The wizard's pages, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Start,
    Action,
    Element,
    Customization,
}

/// The first page's choices, in order.
const CREATE: usize = 0;
const RETRY: usize = 1;
const UNDO: usize = 2;

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
    /// The customization made last on the page, which Retry and Undo name.
    last: Option<String>,
    enabled: bool,
    action: Action,
    elements: Vec<Element>,
    element: usize,
    offered: Vec<Customization>,
    customization: usize,
}

impl FlexibleWebWizard {
    /// The wizard on its first page, for the `elements` at the cursor, after the customization
    /// made `last` on the page, if any.
    pub fn new(id: u32, elements: Vec<Element>, last: Option<String>) -> Self {
        Self {
            id,
            page: Page::Start,
            task: CREATE,
            last,
            enabled: true,
            action: Action::Hide,
            elements,
            element: 0,
            offered: Vec::new(),
            customization: 0,
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
                let (retry, undo) = match &self.last {
                    Some(last) => (
                        msg::flexible_web_retry(last.clone()).to_string(),
                        msg::flexible_web_undo(last.clone()).to_string(),
                    ),
                    None => (
                        msg::flexible_web_retry_none().to_string(),
                        msg::flexible_web_undo_none().to_string(),
                    ),
                };
                let made = self.last.is_some();
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
                                enabled: vec![true, made, made, made, false],
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
                    WizardButtons {
                        back: false,
                        next: true,
                        finish: false,
                    },
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
        };
        (
            DialogSpec {
                id: self.id,
                title: title.to_string(),
                controls,
            },
            buttons,
        )
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
                    ("task", Value::Selected(i)) => self.task = *i,
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
        let mut wizard = FlexibleWebWizard::new(7, elements_at(&page, 2), None);
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
        let mut wizard = FlexibleWebWizard::new(
            7,
            elements_at(&page, 2),
            Some("Hide MainRegion 1 from the top".into()),
        );
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
        let mut nothing = FlexibleWebWizard::new(8, Vec::new(), None);
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
}
