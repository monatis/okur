//! Windows' notifications: a toast said as it shows, whatever is in front, and kept for
//! Insert+Space, Shift+N (RepeatLastNotification) (`docs/references/notifications-research.md`).

use super::ScreenReaderEngine;
use crate::messages::msg;

/// How many notifications are kept: JAWS's Notification History holds at most 500 (FSCast
/// 216, June 2022).
const KEPT: usize = 500;

impl ScreenReaderEngine {
    /// A toast as it shows: JAWS says it by the shell's name for it, "New notification from
    /// Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1", after what was
    /// being said [obs, 770/s1].
    /// With toast announcements turned off it is kept, not said [obs, 773/s3, s5].
    pub(super) fn toast_shown(&mut self, text: &str) -> Option<String> {
        self.notifications.push_back(text.to_owned());
        while self.notifications.len() > KEPT {
            self.notifications.pop_front();
        }
        if self.toasts_unsaid {
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
            .notifications
            .back()
            .cloned()
            .unwrap_or_else(|| msg::notification_none().to_string());
        self.speech.speak(&said, true);
    }
}
