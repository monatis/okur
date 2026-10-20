//! The information keys about where the user is: Say Default Button of Dialog (Insert+E),
//! the settings in use with the application they belong to (Insert+Q), and a list view's
//! columns (Ctrl+Insert+1 to 0) (JAWS 2025 help, Keystrokes for Dialog Boxes; Information
//! Keystrokes).

use super::{AppMode, ScreenReaderEngine};
use crate::messages::msg;

impl ScreenReaderEngine {
    /// Insert+E: with no dialog box in front, "Not in a dialog box" [obs, 513/s1, s7]. In one,
    /// its default button, which is the focused button when a button has the focus: "default
    /// button is No", "default button is Cancel" after Tab [obs, 537/s3, s6]; else the one the
    /// dialog names, "default button is Find Next" [obs, 540/s2].
    pub(super) fn say_default_button(&mut self) -> Option<String> {
        let focus = self
            .last_native_focus
            .as_ref()
            .filter(|f| self.app_mode != AppMode::VirtualBuffer && f.in_dialog_box);
        let Some(focus) = focus else {
            return self.say(msg::not_in_dialog_box().to_string());
        };
        let name = if focus.role == crate::aom::Role::Button {
            focus.name.clone().unwrap_or_default()
        } else {
            focus.default_button.clone()?
        };
        // With short messages, the button alone: "Find Next" [obs, 602/s2, 612/s2].
        if self.settings.verbosity.message_length() == crate::settings::MessageLength::Short {
            return self.say(name.trim().to_owned());
        }
        self.say(msg::default_button_is(name.trim()).to_string())
    }

    /// Shift+Numpad5: the focused control's hot key, as the application gives it, "Alt+r"; in a
    /// dialog box a button's or check box's name, then "alt+" and the letter as its label writes
    /// it, "No", "alt+N", "Wrap around", "alt+r"; an edit's or combo box's name before the key,
    /// "Find what:", "Alt+n"; an edit combo's key alone, "Alt+n"; "no hot key" without one
    /// [obs, 537/s1, s4, s7; 540/s3, s7; 532/s3, s6].
    pub(super) fn say_hot_key(&mut self) -> Option<String> {
        if self.app_mode == AppMode::VirtualBuffer {
            return None;
        }
        let focus = self.last_native_focus.as_ref()?;
        // Whatever the Access Key output, which the stored focus follows.
        let access_key = self.focus_access_key.clone();
        let Some(key) = access_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
        else {
            return self.say(msg::no_hot_key().to_string());
        };
        let name = focus.name.as_deref().unwrap_or_default().trim();
        let letter = key.rsplit('+').next().and_then(|k| {
            let mut chars = k.chars();
            chars.next().filter(|_| chars.next().is_none())
        });
        let in_label = letter.and_then(|letter| {
            name.chars()
                .find(|c| c.to_lowercase().eq(letter.to_lowercase()))
        });
        use crate::aom::Role;
        let edit_combo = focus.container_role == Some(Role::ComboBox);
        let said = match in_label {
            Some(c)
                if focus.in_dialog_box
                    && matches!(
                        focus.role,
                        Role::Button | Role::CheckBox | Role::RadioButton
                    ) =>
            {
                vec![
                    name.to_owned(),
                    msg::hot_key_in_dialog(c.to_string()).to_string(),
                ]
            }
            _ if focus.in_dialog_box
                && matches!(focus.role, Role::Edit | Role::ComboBox)
                && !edit_combo =>
            {
                vec![name.to_owned(), key.to_owned()]
            }
            _ => vec![key.to_owned()],
        };
        // With short messages, the name and the key in one, "Find what:, Alt+n", "Wrap
        // around, alt+r" [obs, 602/s3, s7; 612/s3, s7].
        let short =
            self.settings.verbosity.message_length() == crate::settings::MessageLength::Short;
        let said = if short && said.len() > 1 {
            vec![said.join(", ")]
        } else {
            said
        };
        for (i, part) in said.iter().enumerate() {
            self.speech.speak(part, i == 0);
        }
        Some(said.join("\n"))
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

    /// Ctrl+Insert+V: the application's product name and version, as its executable gives
    /// them: "Microsoft® Windows® Operating System Version 10.0.19041.1" for a PowerShell
    /// window [obs, 513/s4]. Without them nothing is said: what JAWS says then is not
    /// recorded.
    pub(super) fn say_app_version(&mut self) -> Option<String> {
        let (product, version) = self.focus_app_version.clone()?;
        self.say(msg::app_version(product.as_str(), version.as_str()).to_string())
    }

    /// Ctrl+Insert+1 to 0 on a list view's item: the column's header and its text, "Name",
    /// "budget.xlsx"; past the last column, "3 out of range", "this listview only contains 2
    /// columns." [obs, 514/s5-s7, s9, s10]. The first column is the item's name.
    pub(super) fn say_list_column(&mut self, column: u8) -> Option<String> {
        let focus = self
            .last_native_focus
            .as_ref()
            .filter(|f| self.app_mode != AppMode::VirtualBuffer && f.container_is_list_view)?;
        let values: Vec<&str> = std::iter::once(focus.name.as_deref().unwrap_or_default())
            .chain(focus.columns.iter().map(String::as_str))
            .collect();
        let count = values.len().max(focus.column_headers.len());
        let index = usize::from(column).checked_sub(1)?;
        let said = match values.get(index) {
            Some(value) => focus
                .column_headers
                .get(index)
                .map(|header| header.trim())
                .filter(|header| !header.is_empty())
                .into_iter()
                .chain(std::iter::once(value.trim()))
                .map(str::to_owned)
                .collect::<Vec<_>>(),
            None => vec![
                msg::list_column_out_of_range(column).to_string(),
                msg::list_view_columns(count).to_string(),
            ],
        };
        for (i, part) in said.iter().enumerate() {
            self.speech.speak(part, i == 0);
        }
        Some(said.join("\n"))
    }
}
