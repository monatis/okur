//! Windows' notifications: a toast said as it shows, whatever is in front, and kept for
//! Insert+Space, Shift+N (RepeatLastNotification) and the Notification History (Insert+Space,
//! N) (`docs/references/notifications-research.md`).

use super::ScreenReaderEngine;
use crate::messages::msg;
use crate::ui::notifications::{
    CreateRuleDialog, HistoryDialog, NameRuleDialog, Notification, Outcome, RuleOutcome,
    SpeechAction,
};
use crate::ui::{FromUi, ToUi};

/// How many notifications are kept: JAWS's Notification History holds at most 500 (FSCast
/// 216, June 2022).
const KEPT: usize = 500;

impl ScreenReaderEngine {
    /// A toast as it shows: JAWS says it by the shell's name for it, "New notification from
    /// Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1", after what was
    /// being said [obs, 770/s1]. With toast announcements turned off it is kept, not said
    /// [obs, 773/s3, s5].
    /// While Enable Rules is checked, the first rule that matches it decides: Mute (or Play
    /// Sound, not built yet) says nothing, Don't Show in History leaves it out of the history
    /// (JAWS 2025 help, Notification History).
    pub(super) fn toast_shown(&mut self, text: &str, sender: Option<&str>) -> Option<String> {
        let rule = self.notification_rules.deciding(text, sender);
        let hidden = rule.is_some_and(|rule| rule.hidden);
        let silent = rule.is_some_and(|rule| {
            matches!(rule.speech, SpeechAction::Mute | SpeechAction::PlaySound)
        });
        self.last_notification = Some(text.to_owned());
        if !hidden {
            let time = self
                .clock
                .as_deref()
                .map(|clock| crate::clock::say_time(clock, false))
                .unwrap_or_default();
            self.notifications.push_back(Notification {
                text: text.to_owned(),
                sender: sender.map(str::to_owned),
                time,
            });
            while self.notifications.len() > KEPT {
                self.notifications.pop_front();
            }
        }
        if self.toasts_unsaid || silent {
            return None;
        }
        self.speech.speak(text, false);
        Some(text.to_owned())
    }

    /// Insert+Space, Ctrl+N: toasts said or not, until Okur exits, as JAWS's until it
    /// restarts (What's New in JAWS 2023) [obs, 773/s2, s7].
    pub(super) fn toggle_toast_announcements(&mut self) {
        self.toasts_unsaid = !self.toasts_unsaid;
        let said = if self.toasts_unsaid {
            msg::toast_announcements_off()
        } else {
            msg::toast_announcements_on()
        };
        self.speech.speak(said.as_str(), true);
    }

    /// Insert+Space, Shift+N: the last notification, again [obs, 770/s4]; with none, JAWS's
    /// "No notification" [obs, 533/s3].
    pub(super) fn repeat_last_notification(&mut self) {
        let said = self
            .last_notification
            .clone()
            .unwrap_or_else(|| msg::notification_none().to_string());
        self.speech.speak(&said, true);
    }

    /// The notification rules kept, and where to keep them when they change.
    pub fn set_notification_rules(
        &mut self,
        rules: crate::ui::notifications::NotificationRules,
        store: Option<Box<dyn crate::ui::notifications::NotificationRuleStore>>,
    ) {
        self.notification_rules = rules;
        self.notification_rule_store = store;
    }

    fn save_notification_rules(&self) {
        if let Some(store) = &self.notification_rule_store {
            store.save(&self.notification_rules);
        }
    }

    /// Insert+Space, N: the Notification History, the latest first [obs, 774/s3]. Okur says
    /// nothing itself: it reads the dialog as the dialog takes the focus.
    pub(super) fn open_notification_history(&mut self) {
        let Some(ui) = self.ui.as_ref() else {
            return;
        };
        if let Some(open) = &self.notification_history {
            ui.send(ToUi::DialogShow {
                dialog: open.spec(),
            });
            return;
        }
        self.dialog_count += 1;
        let rows = self
            .notifications
            .iter()
            .rev()
            .map(Notification::row)
            .collect();
        let dialog = HistoryDialog::new(
            self.dialog_count,
            rows,
            self.notification_rules.enabled,
            !self.notification_rules.rules.is_empty(),
        );
        ui.send(ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.notification_history = Some(dialog);
    }

    /// Whether `message` is the Notification History's or its details'.
    pub(super) fn is_notification_event(&self, message: &FromUi) -> bool {
        let id = super::dialog_of(message);
        id.is_some()
            && (self.notification_history.as_ref().map(|d| d.id) == id
                || self.notification_details == id
                || self.notification_rule_dialog.as_ref().map(|d| d.id) == id
                || self.notification_name_dialog.as_ref().map(|d| d.id) == id)
    }

    /// What the user did to the Notification History or a notification's details.
    pub(super) fn handle_notification_event(&mut self, message: &FromUi) -> Option<String> {
        if self.notification_details.is_some()
            && self.notification_details == super::dialog_of(message)
        {
            // The details close; the history is in front again.
            if matches!(message, FromUi::DialogClosed { .. }) {
                self.notification_details = None;
            }
            return None;
        }
        if let Some(dialog) = self
            .notification_rule_dialog
            .as_mut()
            .filter(|d| Some(d.id) == super::dialog_of(message))
        {
            match dialog.handle(message) {
                RuleOutcome::Nothing => {}
                // OK asks the rule's name, the notification's text proposed [obs, 783/s7].
                RuleOutcome::Save(rule) => {
                    self.dialog_count += 1;
                    let proposed = self
                        .notification_rule_text
                        .clone()
                        .unwrap_or_else(|| rule.text.clone());
                    let naming = NameRuleDialog::new(self.dialog_count, rule, &proposed);
                    if let Some(ui) = &self.ui {
                        ui.send(ToUi::DialogShow {
                            dialog: naming.spec(),
                        });
                    }
                    self.notification_name_dialog = Some(naming);
                }
                RuleOutcome::Closed => self.notification_rule_dialog = None,
            }
            return None;
        }
        if let Some(naming) = self
            .notification_name_dialog
            .as_mut()
            .filter(|d| Some(d.id) == super::dialog_of(message))
        {
            match naming.handle(message) {
                RuleOutcome::Nothing => {}
                // The rule kept, both dialogs go: the history is in front again, Manage
                // Rules... available [obs, 783/s9, s10].
                RuleOutcome::Save(rule) => {
                    self.notification_rules.rules.push(rule);
                    self.save_notification_rules();
                    let naming = self.notification_name_dialog.take();
                    let creating = self.notification_rule_dialog.take();
                    if let Some(ui) = &self.ui {
                        for id in [naming.map(|d| d.id), creating.map(|d| d.id)]
                            .into_iter()
                            .flatten()
                        {
                            ui.send(ToUi::DialogClose { id });
                        }
                        if let Some(history) = &self.notification_history {
                            ui.send(ToUi::DialogEnable {
                                id: history.id,
                                control: "manage".to_owned(),
                                enabled: true,
                            });
                        }
                    }
                }
                RuleOutcome::Closed => self.notification_name_dialog = None,
            }
            return None;
        }
        let outcome = self.notification_history.as_mut()?.handle(message);
        match outcome {
            Outcome::Nothing => None,
            Outcome::Details(row) => {
                let latest = self.notifications.len().checked_sub(row + 1)?;
                let text = self.notifications.get(latest)?.text.clone();
                let ui = self.ui.as_ref()?;
                self.dialog_count += 1;
                ui.send(ToUi::DialogShow {
                    dialog: crate::ui::notifications::details(self.dialog_count, &text),
                });
                self.notification_details = Some(self.dialog_count);
                None
            }
            // Create Rule... for the notification chosen [obs, 775/s6].
            Outcome::CreateRule(row) => {
                let latest = self.notifications.len().checked_sub(row + 1)?;
                let chosen = self.notifications.get(latest)?;
                self.dialog_count += 1;
                let dialog = CreateRuleDialog::new(
                    self.dialog_count,
                    &chosen.text,
                    chosen.sender.as_deref(),
                );
                self.notification_rule_text = Some(chosen.text.clone());
                self.ui.as_ref()?.send(ToUi::DialogShow {
                    dialog: dialog.spec(),
                });
                self.notification_rule_dialog = Some(dialog);
                None
            }
            // Clear History forgets them all, the last one with them.
            Outcome::Clear => {
                self.notifications.clear();
                self.last_notification = None;
                let ui = self.ui.as_ref()?;
                let id = self.notification_history.as_ref()?.id;
                ui.send(ToUi::DialogItems {
                    id,
                    control: "notifications".to_owned(),
                    items: Vec::new(),
                    selected: 0,
                });
                None
            }
            Outcome::Closed => {
                if let Some(dialog) = self.notification_history.take() {
                    if self.notification_rules.enabled != dialog.rules() {
                        self.notification_rules.enabled = dialog.rules();
                        self.save_notification_rules();
                    }
                    if let (FromUi::DialogPressed { .. }, Some(ui)) = (message, &self.ui) {
                        ui.send(ToUi::DialogClose { id: dialog.id });
                    }
                }
                // The window left takes the focus back and is read again, as JAWS reads it
                // after its Notification History [obs, 774/s13, 770/s7].
                None
            }
        }
    }
}
