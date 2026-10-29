//! The Startup Wizard (the Okur window's Help, Startup Wizard..., and Okur's first start), as
//! JAWS's: the most common settings a page at a time under Back, Next or Finish, and Cancel
//! (old-pc's walk, the redesign's §1.1). A change is heard at once; Finish keeps the changes
//! for all applications, Cancel and Escape keep nothing (JAWS's help).
//!
//! JAWS's window keeps its title, "JAWS Startup Wizard", and says each page's own as the page
//! comes ("Speech Settings | Rate: 20"). Okur's wizard is a Windows property-sheet wizard, whose
//! window takes each page's title, and no screen reader says a page's name otherwise (probed on
//! old-pc with JAWS reading okur-ui): so each page is titled with the page's name, then the
//! wizard's, "Speech Settings - Okur Startup Wizard", which says both.
//!
//! Of JAWS's pages only those with settings Okur has, and on them only Okur's settings, in
//! JAWS's order: Speech Settings (Rate, Punctuation, Typing Echo), Run Okur Settings (Prompt to
//! confirm when exiting Okur), Common Options (Smart Navigation, Forms Mode, Use Keyboard
//! Layout), Verbosity Settings (Tutor Messages, Access Key, Verbosity Level). Okur collects no
//! usage data, so JAWS's first page is not here, nor its braille pages; its start options come
//! with setup's start entries.

use okur_ui_model::{Control, ControlKind, DialogSpec, FromUi, Value, WizardButtons, WizardSpec};

use crate::messages::{msg, StaticMessage};
use crate::settings::registry::{self, choices, Choice};
use crate::settings::{
    FormsMode, KeyboardLayout, OutputMode, OutputType, Punctuation, Settings, SmartNavigation,
    TypingEcho, UserStart, VerbosityLevel,
};

const RATE: &str = "rate";
const PUNCTUATION: &str = "punctuation";
const TYPING_ECHO: &str = "typing_echo";
const CONFIRM_EXIT: &str = "confirm_exit";
const START: &str = "start_after_sign_in";
const SMART_NAVIGATION: &str = "smart_navigation";
const FORMS_MODE: &str = "forms_mode";
const LAYOUT: &str = "layout";
const TUTOR: &str = "tutor";
const ACCESS_KEY: &str = "access_key";
const VERBOSITY: &str = "verbosity";

/// The wizard's pages, in JAWS's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Speech,
    Run,
    Common,
    Verbosity,
}

const PAGES: [Page; 4] = [Page::Speech, Page::Run, Page::Common, Page::Verbosity];

/// What the wizard asks of Okur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Nothing,
    /// Show this page, with these buttons.
    Turn(DialogSpec, WizardButtons),
    /// A setting changed: `settings` in force at once, kept nowhere yet, where they were
    /// `before`.
    Try {
        before: Box<Settings>,
        settings: Box<Settings>,
    },
    /// Finish: keep `settings` for all applications, where they were `before`; the user's
    /// start after sign-in, when the wizard offered it.
    Finish {
        before: Box<Settings>,
        settings: Box<Settings>,
        start: Option<UserStart>,
    },
    /// Cancel, Escape: nothing is kept.
    Closed,
}

/// The wizard's state: the settings for all applications as it opened, and as changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupWizard {
    pub id: u32,
    page: usize,
    before: Settings,
    draft: Settings,
    /// The user's start after sign-in, offered where Okur can keep it.
    start: Option<UserStart>,
}

impl StartupWizard {
    /// The wizard over `settings`, those for all applications, on its first page.
    pub fn new(id: u32, settings: Settings) -> Self {
        Self {
            id,
            page: 0,
            before: settings.clone(),
            draft: settings,
            start: None,
        }
    }

    /// The wizard offering the user's start after sign-in, now `start`.
    pub fn with_start(self, start: UserStart) -> Self {
        Self {
            start: Some(start),
            ..self
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

    /// The choice of `choices` the draft's setting `id` has.
    fn chosen(&self, id: &str, choices: &[Choice]) -> usize {
        let value = registry::get(&self.draft, id);
        choices
            .iter()
            .position(|c| Some(&c.value) == value.as_ref())
            .unwrap_or(0)
    }

    fn page_spec(&self) -> (DialogSpec, WizardButtons) {
        let control = |id: &str, label: StaticMessage, kind: ControlKind| Control {
            id: id.to_owned(),
            label: label.to_string(),
            kind,
            enabled: true,
            hidden: false,
        };
        let labels = |choices: &[Choice]| choices.iter().map(|c| c.label.to_string()).collect();
        let radios = |id: &str, label: StaticMessage, setting: &str, choices: Vec<Choice>| {
            control(
                id,
                label,
                ControlKind::Radios {
                    selected: self.chosen(setting, &choices),
                    options: labels(&choices),
                },
            )
        };
        let page = PAGES.get(self.page).copied().unwrap_or(Page::Speech);
        let (title, controls) = match page {
            Page::Speech => {
                let punctuation = choices::<Punctuation>();
                (
                    msg::startup_wizard_speech(),
                    vec![
                        control(
                            RATE,
                            msg::startup_wizard_rate(),
                            ControlKind::Slider {
                                min: -10,
                                max: 10,
                                value: self.draft.voice.rate,
                            },
                        ),
                        control(
                            PUNCTUATION,
                            msg::startup_wizard_punctuation(),
                            ControlKind::Choice {
                                selected: self.chosen("voice.punctuation", &punctuation),
                                options: labels(&punctuation),
                            },
                        ),
                        radios(
                            TYPING_ECHO,
                            msg::startup_wizard_typing_echo(),
                            "typing_echo",
                            choices::<TypingEcho>(),
                        ),
                    ],
                )
            }
            // JAWS's start options before it: of them Okur has this user's start after
            // sign-in ("Start JAWS after logon for this user: Combo box | Always | 1 of 3" [obs,
            // the walk of the Startup Wizard]). The sign-in screen's comes with its secure mode,
            // and "for all users" is setup's.
            Page::Run => (
                msg::startup_wizard_run(),
                self.start
                    .map(|start| {
                        control(
                            START,
                            msg::startup_wizard_start(),
                            ControlKind::Choice {
                                options: UserStart::ALL
                                    .iter()
                                    .map(|s| Self::start_label(*s).to_string())
                                    .collect(),
                                selected: UserStart::ALL
                                    .iter()
                                    .position(|s| *s == start)
                                    .unwrap_or(0),
                            },
                        )
                    })
                    .into_iter()
                    .chain([control(
                        CONFIRM_EXIT,
                        msg::basics_confirm_exit(),
                        ControlKind::CheckBox {
                            checked: self.draft.confirm_exit,
                        },
                    )])
                    .collect(),
            ),
            Page::Common => {
                let layouts = choices::<KeyboardLayout>();
                (
                    msg::startup_wizard_common(),
                    vec![
                        radios(
                            SMART_NAVIGATION,
                            msg::startup_wizard_smart_navigation(),
                            "smart_navigation",
                            Self::smart_navigation(),
                        ),
                        radios(
                            FORMS_MODE,
                            msg::startup_wizard_forms_mode(),
                            "forms_mode",
                            Self::forms_mode(),
                        ),
                        control(
                            LAYOUT,
                            msg::basics_layout(),
                            ControlKind::Choice {
                                selected: self.chosen("keyboard.layout", &layouts),
                                options: labels(&layouts),
                            },
                        ),
                    ],
                )
            }
            Page::Verbosity => {
                let radios = |id: &str, label: StaticMessage, names: &[StaticMessage], selected| {
                    control(
                        id,
                        label,
                        ControlKind::Radios {
                            options: names.iter().map(ToString::to_string).collect(),
                            selected,
                        },
                    )
                };
                let levels = Self::levels();
                (
                    msg::startup_wizard_verbosity(),
                    vec![
                        radios(
                            TUTOR,
                            msg::basics_tutor(),
                            &[
                                msg::basics_tutor_off(),
                                msg::basics_tutor_on(),
                                msg::basics_tutor_custom(),
                            ],
                            self.draft
                                .verbosity
                                .chosen(OutputType::Hint, &OutputMode::TUTOR),
                        ),
                        radios(
                            ACCESS_KEY,
                            msg::basics_access_key(),
                            &[
                                msg::basics_access_key_off(),
                                msg::startup_wizard_access_key_all(),
                                msg::basics_access_key_menus(),
                                msg::basics_access_key_dialogs(),
                            ],
                            self.draft
                                .verbosity
                                .chosen(OutputType::AccessKey, &OutputMode::ACCESS_KEY),
                        ),
                        control(
                            VERBOSITY,
                            msg::startup_wizard_verbosity_level(),
                            ControlKind::Radios {
                                selected: self.chosen("verbosity.level", &levels),
                                options: labels(&levels),
                            },
                        ),
                    ],
                )
            }
        };
        let last = self.page + 1 == PAGES.len();
        (
            DialogSpec {
                id: self.id,
                title: msg::startup_wizard_page(title.to_string()).render(),
                controls,
                keys: Vec::new(),
            },
            WizardButtons {
                back: self.page > 0,
                next: !last,
                finish: last,
            },
        )
    }

    /// `choices` under the names the wizard gives them, which are not always Settings Center's:
    /// "Controls Only", "Semi Auto", "Beginner, Highest" [obs, the walk of the Startup Wizard].
    fn named<const N: usize>(choices: Vec<Choice>, names: [StaticMessage; N]) -> Vec<Choice> {
        choices
            .into_iter()
            .zip(names)
            .map(|(choice, label)| Choice { label, ..choice })
            .collect()
    }

    fn levels() -> Vec<Choice> {
        Self::named(
            choices::<VerbosityLevel>(),
            [
                msg::startup_wizard_beginner(),
                msg::startup_wizard_intermediate(),
                msg::startup_wizard_advanced(),
            ],
        )
    }

    fn smart_navigation() -> Vec<Choice> {
        Self::named(
            choices::<SmartNavigation>(),
            [
                msg::startup_wizard_navigation_off(),
                msg::startup_wizard_navigation_controls(),
                msg::startup_wizard_navigation_tables(),
                msg::startup_wizard_navigation_both(),
            ],
        )
    }

    fn forms_mode() -> Vec<Choice> {
        Self::named(
            choices::<FormsMode>(),
            [
                msg::startup_wizard_forms_auto(),
                msg::startup_wizard_forms_semi_auto(),
                msg::startup_wizard_forms_manual(),
            ],
        )
    }

    /// A start choice's name, as JAWS's help gives them: "Always, Never, and Using the All
    /// Users Setting".
    fn start_label(start: UserStart) -> StaticMessage {
        match start {
            UserStart::Always => msg::startup_wizard_start_always(),
            UserStart::Never => msg::startup_wizard_start_never(),
            UserStart::AllUsers => msg::startup_wizard_start_all_users(),
        }
    }

    /// The settings as the wizard shows them now.
    pub fn draft(&self) -> &Settings {
        &self.draft
    }

    fn turn(&mut self, page: usize) -> Outcome {
        self.page = page;
        let (page, buttons) = self.page_spec();
        Outcome::Turn(page, buttons)
    }

    /// What the wizard's window reported.
    pub fn handle(&mut self, message: &FromUi) -> Outcome {
        let pick = |draft: &mut Settings, id: &str, choices: Vec<Choice>, i: usize| {
            if let Some(choice) = choices.into_iter().nth(i) {
                registry::set(draft, id, choice.value);
            }
        };
        match message {
            FromUi::DialogChanged { id, control, value } if *id == self.id => {
                let draft = &mut self.draft;
                match (control.as_str(), value) {
                    (RATE, Value::Number(rate)) => draft.voice.rate = (*rate).clamp(-10, 10),
                    (PUNCTUATION, Value::Selected(i)) => {
                        pick(draft, "voice.punctuation", choices::<Punctuation>(), *i);
                    }
                    (TYPING_ECHO, Value::Selected(i)) => {
                        pick(draft, "typing_echo", choices::<TypingEcho>(), *i);
                    }
                    (CONFIRM_EXIT, Value::Checked(on)) => draft.confirm_exit = *on,
                    // Kept at Finish; nothing to hear at once.
                    (START, Value::Selected(i)) => {
                        if let (Some(start), Some(chosen)) =
                            (&mut self.start, UserStart::ALL.get(*i))
                        {
                            *start = *chosen;
                        }
                        return Outcome::Nothing;
                    }
                    (SMART_NAVIGATION, Value::Selected(i)) => {
                        pick(draft, "smart_navigation", choices::<SmartNavigation>(), *i);
                    }
                    (FORMS_MODE, Value::Selected(i)) => {
                        pick(draft, "forms_mode", choices::<FormsMode>(), *i);
                    }
                    (LAYOUT, Value::Selected(i)) => {
                        pick(draft, "keyboard.layout", choices::<KeyboardLayout>(), *i);
                    }
                    (TUTOR, Value::Selected(i)) => {
                        draft
                            .verbosity
                            .choose(OutputType::Hint, &OutputMode::TUTOR, *i)
                    }
                    (ACCESS_KEY, Value::Selected(i)) => {
                        draft
                            .verbosity
                            .choose(OutputType::AccessKey, &OutputMode::ACCESS_KEY, *i)
                    }
                    (VERBOSITY, Value::Selected(i)) => {
                        pick(draft, "verbosity.level", choices::<VerbosityLevel>(), *i);
                    }
                    _ => return Outcome::Nothing,
                }
                Outcome::Try {
                    before: Box::new(self.before.clone()),
                    settings: Box::new(self.draft.clone()),
                }
            }
            FromUi::DialogPressed { id, control } if *id == self.id => match control.as_str() {
                "next" if self.page + 1 < PAGES.len() => self.turn(self.page + 1),
                "back" if self.page > 0 => self.turn(self.page - 1),
                "finish" => Outcome::Finish {
                    before: Box::new(self.before.clone()),
                    settings: Box::new(self.draft.clone()),
                    start: self.start,
                },
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

    fn panic_free_fail(message: &str) {
        assert!(message.is_empty(), "{message}");
    }

    fn pressed(control: &str) -> FromUi {
        FromUi::DialogPressed {
            id: 4,
            control: control.into(),
        }
    }

    /// The pages in JAWS's order, each with Okur's settings of it in JAWS's Tab order, Back
    /// from the second, Finish on the last [obs, the walk of the Startup Wizard].
    #[test]
    fn the_pages_are_jaws_with_okurs_settings() {
        let mut wizard = StartupWizard::new(4, Settings::default());
        let spec = wizard.spec();
        assert_eq!(spec.page.title, "Speech Settings - Okur Startup Wizard");
        assert!(!spec.buttons.back && spec.buttons.next && !spec.buttons.finish);
        let labels = |page: &DialogSpec| -> Vec<String> {
            page.controls.iter().map(|c| c.label.clone()).collect()
        };
        assert_eq!(labels(&spec.page), ["Rate:", "Punctuation:", "Typing Echo"]);
        let mut pages = Vec::new();
        while let Outcome::Turn(page, buttons) = wizard.handle(&pressed("next")) {
            pages.push((page.title.clone(), labels(&page), buttons));
        }
        let titles: Vec<&str> = pages.iter().map(|(t, ..)| t.as_str()).collect();
        assert_eq!(
            titles,
            [
                "Run Okur Settings - Okur Startup Wizard",
                "Common Options - Okur Startup Wizard",
                "Verbosity Settings - Okur Startup Wizard"
            ]
        );
        assert_eq!(
            pages.get(1).map(|(_, l, _)| l.clone()),
            Some(vec![
                "Smart Navigation".to_owned(),
                "Forms Mode".to_owned(),
                "Use Keyboard Layout:".to_owned()
            ])
        );
        assert_eq!(
            pages.last().map(|(.., b)| *b),
            Some(WizardButtons {
                back: true,
                next: false,
                finish: true
            })
        );
        assert!(
            matches!(wizard.handle(&pressed("back")), Outcome::Turn(page, _) if page.title.starts_with("Common Options"))
        );
    }

    /// Where Okur can keep it, Run Okur Settings offers this user's start after sign-in, as
    /// JAWS's, before Prompt to confirm; Finish carries the choice.
    #[test]
    fn the_run_page_offers_the_start_after_sign_in() {
        let mut wizard = StartupWizard::new(4, Settings::default()).with_start(UserStart::AllUsers);
        let Outcome::Turn(run, _) = wizard.handle(&pressed("next")) else {
            panic_free_fail("no Run Okur Settings");
            return;
        };
        let start = run.controls.first();
        assert_eq!(
            start.map(|c| c.label.as_str()),
            Some("Start Okur after logon for this user:")
        );
        assert!(matches!(start.map(|c| &c.kind),
            Some(ControlKind::Choice { options, selected: 2 })
                if options == &["Always", "Never", "Using the All Users Setting"]));
        wizard.handle(&FromUi::DialogChanged {
            id: 4,
            control: START.into(),
            value: Value::Selected(1),
        });
        assert!(matches!(
            wizard.handle(&pressed("finish")),
            Outcome::Finish {
                start: Some(UserStart::Never),
                ..
            }
        ));
        // Without a place to keep it, only Prompt to confirm.
        let mut plain = StartupWizard::new(4, Settings::default());
        let Outcome::Turn(run, _) = plain.handle(&pressed("next")) else {
            panic_free_fail("no Run Okur Settings");
            return;
        };
        assert_eq!(run.controls.len(), 1);
    }

    /// The radio buttons' names are the wizard's own [obs, the walk of the Startup Wizard].
    #[test]
    fn the_wizard_names_its_values() {
        let mut wizard = StartupWizard::new(4, Settings::default());
        wizard.handle(&pressed("next"));
        let Outcome::Turn(common, _) = wizard.handle(&pressed("next")) else {
            panic_free_fail("no Common Options");
            return;
        };
        let options = |page: &DialogSpec, id: &str| -> Vec<String> {
            page.controls
                .iter()
                .find(|c| c.id == id)
                .map(|c| match &c.kind {
                    ControlKind::Radios { options, .. } => options.clone(),
                    _ => Vec::new(),
                })
                .unwrap_or_default()
        };
        assert_eq!(
            options(&common, SMART_NAVIGATION),
            ["Off", "Controls Only", "Tables", "Controls and Tables"]
        );
        assert_eq!(
            options(&common, FORMS_MODE),
            ["Auto", "Semi Auto", "Manual"]
        );
        let Outcome::Turn(verbosity, _) = wizard.handle(&pressed("next")) else {
            panic_free_fail("no Verbosity Settings");
            return;
        };
        assert_eq!(
            options(&verbosity, VERBOSITY),
            ["Beginner, Highest", "Intermediate", "Advanced, Lowest"]
        );
        assert_eq!(
            options(&verbosity, ACCESS_KEY),
            ["Off", "Speak all", "Speak Menus Only", "Speak Dialogs Only"]
        );
        assert_eq!(
            options(&verbosity, TUTOR),
            [
                "Turn Off Menu and Control Help",
                "Announce Menu and Control Help",
                "Announce Custom Messages Only"
            ]
        );
    }

    /// A change is tried at once; Finish keeps the changes, Escape nothing.
    #[test]
    fn a_change_is_tried_then_finished() {
        let mut wizard = StartupWizard::new(4, Settings::default());
        let tried = wizard.handle(&FromUi::DialogChanged {
            id: 4,
            control: TYPING_ECHO.into(),
            value: Value::Selected(2),
        });
        assert!(
            matches!(&tried, Outcome::Try { settings, .. } if settings.typing_echo == TypingEcho::Words),
            "{tried:?}"
        );
        wizard.handle(&FromUi::DialogChanged {
            id: 4,
            control: RATE.into(),
            value: Value::Number(4),
        });
        let finished = wizard.handle(&pressed("finish"));
        assert!(
            matches!(&finished, Outcome::Finish { before, settings, .. }
                if settings.voice.rate == 4 && before.voice.rate == Settings::default().voice.rate),
            "{finished:?}"
        );
        assert_eq!(
            wizard.handle(&FromUi::DialogClosed { id: 4 }),
            Outcome::Closed
        );
    }
}
