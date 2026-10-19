//! The information keys about where the user is: Say Default Button of Dialog (Insert+E) and
//! the settings in use with the application they belong to (Insert+Q) (JAWS 2025 help,
//! Keystrokes for Dialog Boxes; Information Keystrokes).

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;

impl ScreenReaderEngine {
    /// Insert+E: with no dialog box in front, "Not in a dialog box" [obs, 513/s1, s7]. A dialog
    /// box's default button is not read yet: it waits for its recording (532).
    pub(super) fn say_default_button(&mut self) -> Option<String> {
        let in_dialog = self.app_mode != AppMode::VirtualBuffer
            && self
                .last_native_focus
                .as_ref()
                .is_some_and(|f| f.in_dialog_box);
        if in_dialog {
            return None;
        }
        self.say(msg::not_in_dialog_box().to_string())
    }

    /// Insert+Q: "Default settings are loaded" when the application has no settings of its
    /// own, then its executable, said and spelled: "The application currently being used is
    /// powershell.exe", "p o w e r s h e l l . e x e" [obs, 513/s5]. What JAWS says for an
    /// application with settings of its own is not recorded yet; Okur names the application
    /// alone then.
    pub(super) fn say_configuration(&mut self) -> Option<String> {
        let app = self.last_native_focus.as_ref()?.app.clone()?;
        let id = crate::settings_layers::application_id(&app);
        let mut said = Vec::new();
        if !self.settings_layers.applications.contains_key(&id) {
            said.push(msg::default_settings_loaded().to_string());
        }
        said.push(msg::application_in_use(app.as_str()).to_string());
        for (i, text) in said.iter().enumerate() {
            self.speech.speak(text, i == 0);
        }
        self.speech.speak_spelled(&app, false);
        said.push(crate::pronounce::spelled(&app));
        Some(said.join("\n"))
    }
}
