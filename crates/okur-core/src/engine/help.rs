//! Okur's help keys: screen-sensitive help (Insert+F1), the hot keys (Insert+H) and the
//! Windows keys (Insert+W). JAWS shows its help in the Virtual Viewer and reads it all as it
//! opens: the text, a "List JAWS Hot Keys" link after the control's help, and "Press ESCAPE
//! to close this message." [obs, 526/s1, 527/s1, s3, s5]; Escape reads the focus again
//! [obs, 526/s6, 527/s2]. Okur's keys and viewer follow JAWS. The words are Okur's own: JAWS's
//! help texts are Freedom Scientific's.

use super::{AppMode, ScreenReaderEngine};
use crate::aom::Role;
use crate::command::Command;
use crate::messages::{msg, StaticMessage};
use crate::viewer::VirtualViewer;

/// A line of a help viewer that is a link, as JAWS reads one: "Link" and its text.
fn link(text: impl std::fmt::Display) -> String {
    format!("\u{1d} {} \u{1e} {text}", msg::role_link())
}

/// Okur's keys for a page, then for every window, with the command each runs.
const PAGE_KEYS: [(fn() -> StaticMessage, Command); 6] = [
    (msg::help_key_say_all, Command::SayAll),
    (msg::help_key_headings_list, Command::SelectAHeading),
    (msg::help_key_links_list, Command::SelectALink),
    (msg::help_key_form_fields_list, Command::SelectAFormField),
    (msg::help_key_element_lists, Command::VirtualHtmlFeatures),
    (msg::help_key_find, Command::VirtualFind),
];
const WINDOW_KEYS: [(fn() -> StaticMessage, Command); 13] = [
    (msg::help_key_control_help, Command::ScreenSensitiveHelp),
    (msg::help_key_windows_keys, Command::WindowsKeysHelp),
    (msg::help_key_window_title, Command::SayWindowTitle),
    (msg::help_key_read_window, Command::SayWindow),
    (msg::help_key_current_control, Command::SayCurrentFocus),
    (msg::help_key_default_button, Command::SayDefaultButton),
    (msg::help_key_window_list, Command::WindowList),
    (msg::help_key_system_tray, Command::SystemTrayList),
    (msg::help_key_time, Command::SaySystemTime),
    (msg::help_key_settings_in_use, Command::SayConfiguration),
    (msg::help_key_quick_settings, Command::QuickSettings),
    (msg::help_key_virtualize_window, Command::VirtualizeWindow),
    (msg::help_key_find, Command::OkurFind),
];

const WINDOWS_KEYS: [fn() -> StaticMessage; 10] = [
    msg::help_windows_intro,
    msg::help_windows_menu_bar,
    msg::help_windows_start,
    msg::help_windows_switch,
    msg::help_windows_close,
    msg::help_windows_context_menu,
    msg::help_windows_properties,
    msg::help_windows_desktop,
    msg::help_windows_tray,
    msg::help_windows_help,
];

impl ScreenReaderEngine {
    /// Insert+F1: what the focused control is for, then the link to the hot keys. A page's
    /// link and heading have no link to the hot keys [obs, 531/s5, s8].
    pub(super) fn screen_sensitive_help(&mut self) -> Option<String> {
        let (mut lines, hot_keys_link) = match self.page_help() {
            Some(help) => help,
            None => (vec![self.control_help().to_string()], true),
        };
        lines.push(String::new());
        let mut links = Vec::new();
        if hot_keys_link {
            links.push((lines.len(), Command::HotKeyHelp));
            lines.push(link(msg::viewer_hot_keys_link()));
        }
        lines.push(msg::viewer_close_hint().to_string());
        self.open_help(lines, links)
    }

    /// Help on the page under the virtual cursor, as JAWS's is about the element it is on
    /// [obs, 531/s2, s5, s8]: a link's address, then how a link is used; a heading's level
    /// and the heading keys; elsewhere the page's summary and how a page is read. `None` out
    /// of a page's virtual cursor, and on a page's control, which its kind's help covers.
    fn page_help(&self) -> Option<(Vec<String>, bool)> {
        if self.app_mode != AppMode::VirtualBuffer || self.forms_mode {
            return None;
        }
        let node = self.node_under_cursor();
        Some(match node.map(|n| &n.role) {
            Some(Role::Link { href }) => (
                href.iter()
                    .cloned()
                    .chain(std::iter::once(msg::help_control_link().to_string()))
                    .collect(),
                false,
            ),
            Some(Role::Heading { level }) => (
                vec![msg::help_heading(i64::from(*level)).to_string()],
                false,
            ),
            Some(
                Role::Edit
                | Role::CheckBox
                | Role::RadioButton
                | Role::ComboBox
                | Role::Button
                | Role::Slider
                | Role::Tab,
            ) => return None,
            _ => (
                vec![self.page_summary(), msg::help_page().to_string()],
                true,
            ),
        })
    }

    /// Insert+H: Okur's keys for the page or window in front, each a link.
    pub(super) fn hot_key_help(&mut self) -> Option<String> {
        let in_page = self.app_mode == AppMode::VirtualBuffer;
        let mut lines = vec![
            if in_page {
                msg::help_hot_keys_page_intro()
            } else {
                msg::help_hot_keys_intro()
            }
            .to_string(),
            String::new(),
        ];
        let page_keys = in_page.then_some(PAGE_KEYS.iter()).into_iter().flatten();
        // Find is the page's Ctrl+F, or Insert+Ctrl+F where there is no page.
        let window_keys = WINDOW_KEYS
            .iter()
            .filter(|(_, command)| !in_page || *command != Command::OkurFind);
        let mut links = Vec::new();
        for (words, command) in page_keys.chain(window_keys) {
            links.push((lines.len(), command.clone()));
            lines.push(link(words()));
        }
        lines.push(String::new());
        lines.push(msg::viewer_close_hint().to_string());
        self.open_help(lines, links)
    }

    /// Insert+W: the keys Windows gives every application.
    pub(super) fn windows_keys_help(&mut self) -> Option<String> {
        let mut lines: Vec<String> = WINDOWS_KEYS.iter().map(|m| m().to_string()).collect();
        lines.push(String::new());
        lines.push(msg::viewer_close_hint().to_string());
        self.open_help(lines, Vec::new())
    }

    /// Shows help in the Virtual Viewer, in place of what it shows, and reads it all.
    fn open_help(&mut self, lines: Vec<String>, links: Vec<(usize, Command)>) -> Option<String> {
        let viewer = VirtualViewer::message(lines).with_links(links);
        let speech = viewer.message_speech();
        self.viewer = Some(viewer);
        self.show_viewer_window();
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// The help for where the focus is: the page, or the control's kind.
    fn control_help(&self) -> StaticMessage {
        // In a page: the field forms mode is in, or the control under the virtual cursor.
        let page_role = match (self.app_mode == AppMode::VirtualBuffer, self.forms_mode) {
            (true, true) => self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.node(id))
                .map(|node| node.role.clone()),
            (true, false) => self.node_under_cursor().map(|node| node.role.clone()),
            (false, _) => None,
        };
        let native = self.last_native_focus.as_ref();
        let role = page_role.or_else(|| native.map(|f| f.role.clone()));
        let in_list_view = native.is_some_and(|f| f.container_is_list_view);
        match role {
            Some(Role::Edit) if native.is_some_and(|f| f.multi_line) => {
                msg::help_control_text_area()
            }
            Some(Role::Edit) => msg::help_control_edit(),
            Some(Role::Document) => msg::help_control_text_area(),
            Some(Role::CheckBox) => msg::help_control_check_box(),
            Some(Role::RadioButton) => msg::help_control_radio_button(),
            Some(Role::ComboBox) => msg::help_control_combo_box(),
            Some(Role::ListItem | Role::List) if in_list_view => msg::help_control_list_view(),
            Some(Role::ListItem | Role::List) => msg::help_control_list_box(),
            Some(Role::Tree | Role::TreeItem) => msg::help_control_tree_view(),
            Some(Role::Button | Role::SplitButton | Role::MenuButton) => msg::help_control_button(),
            Some(Role::Tab | Role::TabList) => msg::help_control_tab(),
            Some(Role::Slider) => msg::help_control_slider(),
            Some(Role::MenuItem | Role::MenuBar) => msg::help_control_menu(),
            Some(Role::Link { .. }) => msg::help_control_link(),
            Some(Role::TableCell | Role::TableRow) => msg::help_control_cell(),
            _ => msg::help_control_other(),
        }
    }
}
