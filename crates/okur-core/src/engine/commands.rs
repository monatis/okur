//! Reading commands: the JAWS script names `execute_command` runs, the JAWS-key chords, and
//! the commands that need more than the buffer (Say Font, Virtualize Window, the clock).

use super::{AppMode, PendingViewer, Press, ScreenReaderEngine, DOUBLE_PRESS_MS};
use crate::aom::Role;
use crate::command::{Command, Element};
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;
use crate::speech::SpeechFormatter;

impl ScreenReaderEngine {
    /// Runs the command a name calls ([`Command::from_name`]): a JAWS script name or the key
    /// notation of JAWS's key maps. Speaks the result and returns it.
    pub fn execute_command(&mut self, name: &str) -> Option<String> {
        self.execute(Command::from_name(name)?)
    }

    /// Runs `command`, speaks what it reads, and returns it. With Speech On Demand on, a
    /// command of its list speaks (and the Say All it starts); the others are written to the
    /// speech history alone [obs, 700].
    pub fn execute(&mut self, command: Command) -> Option<String> {
        let lifted = self.on_demand() && command.speaks_on_demand();
        if lifted {
            self.speech.set_unspoken(false);
        }
        let said = self.execute_command_itself(command);
        if lifted {
            self.sync_speech_mode();
        }
        said
    }

    fn execute_command_itself(&mut self, command: Command) -> Option<String> {
        // The reading keys read where the Okur cursor is, with it active on a text.
        if let Some(said) = self.okur_cursor_command(&command) {
            return said;
        }
        if let Some(said) = self.read_where_focused(&command) {
            return said;
        }
        match command {
            Command::LayerStart => return self.start_layer(),
            Command::NavigationModeToggle => return self.next_quick_keys_mode(),
            Command::QuickKeysSuspendResume => return self.suspend_or_resume_quick_keys(),
            Command::VirtualCursorToggle => return self.toggle_virtual_cursor(),
            Command::RefreshScreen => return self.refresh_screen(),
            Command::ShutDown => return self.shut_down(),
            Command::SayDefaultButton => return self.say_default_button(),
            Command::SayClipboard => return self.say_clipboard(),
            Command::ViewClipboardText => return self.view_clipboard_text(),
            Command::AppendSelectedTextToClipboard => return self.append_selection_to_clipboard(),
            Command::SayConfiguration => return self.say_configuration(),
            Command::SayAppVersion => return self.say_app_version(),
            Command::OkurCursor => return self.activate_okur_cursor(),
            Command::PcCursor => return self.activate_pc_cursor(),
            Command::RouteOkurToPc => return self.route_okur_to_pc(),
            Command::SayActiveCursor => return self.say_active_cursor(),
            Command::SayHotKey => return self.say_hot_key(),
            Command::MoveToNext(crate::command::Element::MailLink) => {
                return self.move_to_mail_link(true)
            }
            Command::MoveToPrior(crate::command::Element::MailLink) => {
                return self.move_to_mail_link(false)
            }
            Command::SayListColumn(column) => return self.say_list_column(column),
            Command::SayLinkAddress => return self.say_link_address(),
            Command::RouteVirtualToPc => return self.route_virtual_to_pc(),
            Command::RoutePcToVirtual => return self.route_pc_to_virtual(),
            Command::SelectAFrame => return self.select_a_frame(),
            Command::LeftMouseClick => return self.left_mouse_click(),
            Command::RightMouseClick => return self.right_mouse_click(),
            Command::LeftMouseButtonLock => return self.mouse_button_lock(false),
            Command::ModifiedLeftMouseClick(modifier) => {
                return self.modified_left_mouse_click(modifier)
            }
            Command::MoveMouse(step) => return self.move_mouse_step(step),
            Command::DragAndDrop => return self.drag_and_drop(),
            Command::RightMouseButtonLock => return self.mouse_button_lock(true),
            Command::RoutePcToOkur => return self.route_pc_to_okur(),
            Command::RestrictOkurCursor => return self.restrict_okur_cursor(),
            Command::WindowList => return self.ask_window_list(),
            Command::SystemTrayList => return self.ask_tray_list(),
            Command::JumpToLine => return self.open_jump(),
            Command::DefineTempPlaceMarker => return self.define_temp_placemarker(),
            Command::SelectAPlaceMarker => return self.open_placemarker_list(),
            Command::WordList => return self.open_list(crate::ui::lists::ListKind::Words),
            Command::NextWordFromList => return self.move_to_word_from_list(true),
            Command::PriorWordFromList => return self.move_to_word_from_list(false),
            Command::SpeakPlaceMarker(number) => return self.say_placemarker_number(number, false),
            Command::MoveToPlaceMarkerNumber(number) => {
                return self.say_placemarker_number(number, true)
            }
            Command::AssignCustomLabel => return self.open_custom_label(),
            Command::JumpReturnFromLine => return self.jump_back(),
            Command::ScreenSensitiveHelp => return self.screen_sensitive_help(),
            Command::HotKeyHelp => return self.hot_key_help(),
            Command::WindowsKeysHelp => return self.windows_keys_help(),
            Command::RateFaster => return self.change_rate(true),
            Command::RateSlower => return self.change_rate(false),
            Command::SayWindow => {
                // Dialog boxes; other windows are not read this way yet.
                let focus = self
                    .last_native_focus
                    .as_ref()
                    .filter(|f| !f.window_items.is_empty())?;
                let speech = crate::native_focus::say_window(
                    focus.window_title.as_deref().unwrap_or_default(),
                    &focus.window_items,
                );
                let speech = self.said(speech).render();
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            // A page's focus is said as it was when it got the focus, without moving the
            // virtual cursor: "Document content edit | Contains text" (JAWS in Google Docs,
            // dev box 2026-10-16).
            // In a menu, its item (JAWS in Google Docs' menu: "menu | Accessibility | 10 of 10";
            // the position is not known here).
            Command::SayCurrentFocus if self.menus.current().is_some() => {
                let speech = self.menus.current()?.to_owned();
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SayCurrentFocus if self.app_mode == AppMode::VirtualBuffer => {
                let id = self.last_page_focus?;
                let (offset, node) = (self.cursor.offset, self.cursor.current_node_id);
                let speech = self.cursor.focus_node(id);
                self.cursor.offset = offset;
                self.cursor.current_node_id = node;
                let speech = speech.filter(|s| !s.trim().is_empty())?;
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            // In Word's document, its line, or in a table the cell and where it is: "Branch |
            // column 1 of 2 | row 1 of 2 | table 1" [obs, 1034/s13, 1035/s10, s16].
            Command::SayCurrentFocus
                if self.app_mode == AppMode::NativeControl
                    && self.editing_text()
                    && self
                        .last_text
                        .as_ref()
                        .and_then(crate::apps::windows::word::document_focus)
                        .is_some() =>
            {
                let speech = self
                    .last_text
                    .as_ref()
                    .and_then(crate::apps::windows::word::document_focus)?;
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SayCurrentFocus => {
                // Native controls.
                let speech = self
                    .last_native_focus
                    .clone()
                    .filter(|_| self.app_mode == AppMode::NativeControl)
                    .map(|mut focus| {
                        focus.access_key = self.focus_access_key.clone();
                        crate::native_focus::say_focus(&focus)
                    })
                    .filter(|s| !s.is_empty())?;
                // A text field's line (an edit, or a document such as Notepad's): JAWS's text
                // analysis names its inconsistencies after the field ("edit | Type in text. | Space
                // run at column 4" [obs, 568/s4]).
                let analysis = self
                    .last_native_focus
                    .as_ref()
                    .filter(|f| {
                        matches!(f.role, crate::aom::Role::Edit | crate::aom::Role::Document)
                    })
                    .and(self.last_text.as_ref())
                    .map(|text| {
                        crate::text_analysis::describe_line_as(
                            &text.text,
                            text.caret,
                            self.settings.verbosity.message_length()
                                == crate::settings::MessageLength::Short,
                        )
                    })
                    .unwrap_or_default();
                let speech = std::iter::once(speech)
                    .chain(analysis)
                    .collect::<Vec<_>>()
                    .join("\n");
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            // "Enter key to pass through" (JAWS on the dev box, 2026-10-17, WhatsApp).
            Command::PassKeyThrough => {
                self.pass_next_key = true;
                let speech = msg::pass_key_through().to_string();
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SayWindowTitle => {
                // The Virtual Viewer is a window of its own: JAWS names it, whatever it shows
                // ("Virtual Viewer", without "Title is"; dev box, Windows Terminal and Notepad,
                // 2026-10-15).
                // A viewer of its own title says it as a window's: "Title is | SR OCR Sign - OCR
                // Results" [obs, 1024/s10].
                if let Some(viewer) = &self.viewer {
                    let speech = match viewer.title() {
                        Some(title) => format!("{}\n{title}", msg::window_title_is()),
                        None => msg::viewer_title().to_string(),
                    };
                    self.speech.speak(&speech, true);
                    return Some(speech);
                }
                // Word's document adds the view it is shown in, which the platform reads first:
                // "Title is | sr-word-fixture.docx - Word | Print View" [obs, 519/s1].
                if let Some(title) = self.last_window_title.clone().filter(|_| {
                    self.app_mode != AppMode::VirtualBuffer
                        && self.editing_document()
                        && self
                            .last_text
                            .as_ref()
                            .is_some_and(|t| t.structure.is_some())
                        && self
                            .actions
                            .as_ref()
                            .is_some_and(|actions| actions.read_document_view())
                }) {
                    self.document_view_title = Some(title);
                    return None;
                }
                // [obs, 306/s12]: "Title is | SR Dialog Fixture". In a browser it is the browser
                // window's title, not the page's [obs, 209/s1: "... - Google Chrome - <profile>"].
                let title = self.last_window_title.clone().or_else(|| {
                    (self.app_mode == AppMode::VirtualBuffer)
                        .then(|| self.cursor.buffer.document_title().map(str::to_owned))
                        .flatten()
                })?;
                let mut lines = vec![
                    crate::announcement::Line::of(
                        crate::announcement::PartKind::Message,
                        msg::window_title_is(),
                    )
                    .short(""),
                    crate::announcement::Line::of(crate::announcement::PartKind::Name, title),
                ];
                // In a page, where the virtual cursor is: its region, level and heading
                // [obs, 624, 625].
                if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
                    lines.extend(self.title_context());
                }
                let speech = crate::announcement::Announcement::new(lines);
                let speech = self.said(speech).render();
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::SaySystemTime
            | Command::SaySystemTimeWithSeconds
            | Command::SaySystemDate
            | Command::SayWeekNumber => {
                let speech = self.clock_command(&command)?;
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::VirtualizeWindow => return self.virtualize(true),
            Command::VirtualizeControl => return self.virtualize(false),
            Command::QuickSettings => {
                return self.open_settings(crate::ui::settings_tree::Flavor::Quick)
            }
            Command::SettingsCenter => {
                return self.open_settings(crate::ui::settings_tree::Flavor::Center)
            }
            Command::OkurWindow => return self.open_okur_window(),
            Command::SelectVoiceProfile => return self.open_voice_profile(),
            Command::SetPunctuationLevel => return self.next_punctuation_level(),
            Command::NextTypingEcho => return self.next_typing_echo(),
            Command::NextScreenEcho => return self.next_screen_echo(),
            Command::ToggleSmartNavigation => return self.toggle_smart_navigation(),
            Command::SpellWord => return self.spell(false),
            Command::SpellLine => return self.spell(true),
            Command::SayFont => return self.say_font(false),
            Command::SayFontInViewer => return self.say_font(true),
            Command::SayColor => return self.say_color(),
            Command::KeyboardHelp => return self.toggle_keyboard_help(),
            _ => {}
        }
        // On the desktop and the taskbar, Windows moves and the focus is read: the page
        // commands below have nothing to act on.
        if matches!(self.app_mode, AppMode::NativeDesktop | AppMode::NativeShell) {
            return None;
        }
        match &command {
            Command::Find(query) => {
                let speech = self.cursor.find_text(query, false);
                if let Some(ref text) = speech {
                    self.speech.speak(text, true);
                }
                return speech;
            }
            Command::JumpToHeading(target) => {
                let list = self.cursor.buffer.get_headings_list();
                let item = list
                    .iter()
                    .find(|h| h.text.eq_ignore_ascii_case(target) || h.text.contains(target))?;
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::JumpToLink(target) => {
                let list = self.cursor.buffer.get_links_list();
                let item = list
                    .iter()
                    .find(|l| l.text.eq_ignore_ascii_case(target) || l.text.contains(target))?;
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            Command::JumpToFormField(target) => {
                let list = self.cursor.buffer.get_form_fields_list();
                let item = list
                    .iter()
                    .find(|f| f.name.eq_ignore_ascii_case(target) || f.name.contains(target))?;
                let speech = self.cursor.jump_to_element_offset(item.offset);
                self.speech.speak(&speech, true);
                return Some(speech);
            }
            _ => {}
        }

        let speech = match &command {
            Command::SayNextLine => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_next_option()
                } else {
                    self.cursor.move_next_line()
                }
            }
            Command::SayPriorLine => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.combobox_prior_option()
                } else {
                    self.cursor.move_prior_line()
                }
            }
            Command::SayNextWord => self.cursor.move_next_word(),
            Command::SayPriorWord => self.cursor.move_prior_word(),
            Command::SayNextCharacter => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_expand_or_next(),
                    Some(Role::Slider) => self.cursor.slider_increase(),
                    _ => self.cursor.move_next_char(),
                }
            }
            Command::SayPriorCharacter => {
                let role = self.cursor.current_node().map(|n| n.role.clone());
                match role {
                    Some(Role::TreeItem) => self.cursor.tree_item_collapse_or_parent(),
                    Some(Role::Slider) => self.cursor.slider_decrease(),
                    _ => self.cursor.move_prior_char(),
                }
            }
            Command::SayNextParagraph => self.cursor.move_next_paragraph(),
            Command::SayParagraph => Some(self.cursor.current_paragraph()),
            Command::SaySentence => Some(self.cursor.current_sentence()),
            Command::SayNextSentence => Some(self.cursor.move_sentence(true)),
            Command::SayPriorSentence => Some(self.cursor.move_sentence(false)),
            // In a page, the laptop layout's CapsLock+Ctrl+O is the articles list.
            Command::NextParagraphOrArticles => {
                return self.execute(Command::SelectElement(
                    crate::element_lists::ElementList::Articles,
                ))
            }
            Command::SayPriorParagraph => self.cursor.move_prior_paragraph(),
            // Home and End move silently [obs, 502/s5, s6].
            Command::LineStart => {
                self.cursor.move_to_line_edge(false);
                return None;
            }
            Command::LineEnd => {
                self.cursor.move_to_line_edge(true);
                return None;
            }
            Command::PageDown => Some(self.cursor.move_page(true)),
            Command::PageUp => Some(self.cursor.move_page(false)),
            Command::TopOfFile => Some(self.cursor.move_to_start()),
            Command::BottomOfFile => Some(self.cursor.move_to_end()),

            // Quick navigation, JAWS's Quick Navigation Keys
            // (docs/references/jaws-quick-navigation-keys.md).
            Command::MoveToNext(element) => self.move_to_element(*element, true),
            Command::MoveToPrior(element) => self.move_to_element(*element, false),
            Command::MoveToMainRegion => self.cursor.move_to_main_region(),
            Command::NextNonLinkText => Some(self.cursor.move_to_non_link_text(true)),
            Command::PriorNonLinkText => Some(self.cursor.move_to_non_link_text(false)),
            Command::NextSameElement => Some(self.cursor.move_by_element_type(true, true)),
            Command::PriorSameElement => Some(self.cursor.move_by_element_type(true, false)),
            Command::NextDifferentElement => Some(self.cursor.move_by_element_type(false, true)),
            Command::PriorDifferentElement => Some(self.cursor.move_by_element_type(false, false)),

            // Tables
            Command::TableCellRight => self.cursor.move_table_cell_right(),
            Command::TableCellLeft => self.cursor.move_table_cell_left(),
            Command::TableCellDown => self.cursor.move_table_cell_down(),
            Command::TableCellUp => self.cursor.move_table_cell_up(),
            Command::TableFirstCell => self.cursor.move_table_first_cell(),
            Command::TableLastCell => self.cursor.move_table_last_cell(),
            Command::MoveToStartOfRow => self.cursor.move_table_row_start(),
            Command::MoveToEndOfRow => self.cursor.move_table_row_end(),
            Command::MoveToTopOfColumn => self.cursor.move_table_col_top(),
            Command::MoveToBottomOfColumn => self.cursor.move_table_col_bottom(),
            Command::SayCurrentCell => self.cursor.read_current_cell(),
            Command::SayCurrentRow => self.cursor.read_current_row(),
            Command::SayCurrentColumn => self.cursor.read_current_column(),
            Command::FirstHeading => self.cursor.move_edge_heading(false),
            Command::LastHeading => self.cursor.move_edge_heading(true),
            Command::FirstHeadingAtLevel(level) => {
                self.cursor.move_edge_heading_level(*level, false)
            }
            Command::LastHeadingAtLevel(level) => self.cursor.move_edge_heading_level(*level, true),
            Command::FirstFormField => self.cursor.move_edge_form_control(false),
            Command::LastFormField => self.cursor.move_edge_form_control(true),
            Command::ReadNextRow => self.cursor.read_adjacent_row(1),
            Command::ReadPriorRow => self.cursor.read_adjacent_row(-1),
            Command::ReadNextColumn => self.cursor.read_adjacent_column(1),
            Command::ReadPriorColumn => self.cursor.read_adjacent_column(-1),
            Command::StepToStartOfElement => self.cursor.move_element_edge(false),
            Command::StepToEndOfElement => self.cursor.move_element_edge(true),
            Command::FlowTo => self.cursor.move_flow(true),
            Command::FlowFrom => self.cursor.move_flow(false),
            Command::SayColumnHeader => self.cursor.read_current_col_header(),
            Command::SayRowHeader => self.cursor.read_current_row_header(),

            Command::EnterMathViewer => self.cursor.enter_math_viewer(),
            Command::ExpandComboBox => self.cursor.expand_combobox(),
            Command::CollapseComboBox => self.cursor.collapse_combobox(),

            // Interactive navigation & actions
            Command::NextTabStop => self.cursor.move_next_tab_stop(),
            Command::PriorTabStop => self.cursor.move_prior_tab_stop(),
            // Space with the virtual cursor, JAWS's VirtualSpacebar: the element under the
            // cursor takes the focus and is clicked, as Enter clicks it. Buttons, check boxes,
            // radio buttons and links get a click and no key [obs, 229 event log]; the focus
            // event and the page's state change then speak ("Count Button | To activate press
            // Enter.", "Subscribe check box not checked | ... | checked"). A field one types
            // in takes the focus in forms mode, typing nothing [obs, 229/s14], and a combo
            // box's list opens [obs, 229/s21]. Freedom Scientific: "the SPACEBAR toggles the
            // check mark on or off for check boxes ... moves the selection of a radio button
            // ... and activates buttons" (Surf's Up, Using Forms with JAWS).
            // Space on text does nothing, and the page never gets the key [obs, 229/s2: no
            // "page keydown Space" in its event log].
            Command::Activate => {
                self.radio_switched = self.radio_switching_here();
                if let (Some(actions), Some(node)) = (&self.actions, self.node_under_cursor()) {
                    let editable = node.attributes.get("editable").is_some_and(|e| e == "true");
                    let (handled, forms_mode) = match node.role {
                        Role::Edit => (actions.focus(node), true),
                        Role::ComboBox if editable => (actions.focus(node), true),
                        Role::ComboBox => (actions.focus(node) && actions.activate(node), true),
                        Role::Link { .. }
                        | Role::Button
                        | Role::MenuButton
                        | Role::SplitButton
                        | Role::CheckBox
                        | Role::RadioButton
                        | Role::Tab => (actions.focus(node) && actions.activate(node), false),
                        _ => (false, false),
                    };
                    if handled && forms_mode {
                        self.forms_mode = true;
                        self.cursor.forms_mode_active = true;
                        self.forms_mode_by_enter = true;
                    }
                }
                Some(String::new())
            }
            Command::Enter => {
                let node = self.node_under_cursor();
                let same_page_link = node
                    .as_ref()
                    .is_some_and(|n| n.attributes.get("same_page").is_some_and(|v| v == "true"));
                if let (Some(actions), Some(node)) = (&self.actions, node) {
                    let handled = match node.role {
                        // Forms mode on (JAWS plays a sound) and real focus in the field; the
                        // browser's focus event then speaks it.
                        Role::Edit | Role::ComboBox => {
                            let focused = actions.focus(node);
                            if focused {
                                self.forms_mode = true;
                                self.cursor.forms_mode_active = true;
                                self.forms_mode_by_enter = true;
                            }
                            focused
                        }
                        Role::Link { .. } => !same_page_link && actions.activate(node),
                        Role::Button
                        | Role::MenuButton
                        | Role::CheckBox
                        | Role::RadioButton
                        | Role::Tab => actions.activate(node),
                        _ => false,
                    };
                    if handled {
                        return Some(String::new());
                    }
                }
                if let Some(link_speech) = self.cursor.activate_current_link() {
                    Some(link_speech)
                } else if let Some(math_speech) = self.cursor.enter_math_viewer() {
                    Some(math_speech)
                } else {
                    let res = self.cursor.enter_forms_mode();
                    if res.is_some() {
                        self.forms_mode = true;
                    }
                    res
                }
            }
            Command::Escape => {
                if self.cursor.is_combobox_expanded() {
                    self.cursor.collapse_combobox()
                } else if self.forms_mode {
                    let res = self
                        .cursor
                        .exit_forms_mode()
                        .map(|_| self.virtual_pc_message());
                    self.forms_mode = false;
                    res
                } else if self.cursor.is_in_modal() {
                    self.cursor.exit_modal()
                } else {
                    None
                }
            }

            // Text selection commands
            Command::SelectNextCharacter => self.cursor.select_next_char(),
            Command::SelectPriorCharacter => self.cursor.select_prior_char(),
            Command::SelectNextLine => self.cursor.select_next_line(),
            Command::SelectPriorLine => self.cursor.select_prior_line(),
            Command::SelectNextWord => self.cursor.select_next_word(),
            Command::SelectPriorWord => self.cursor.select_prior_word(),
            Command::SelectToLineEnd => self.cursor.select_to_line_edge(true),
            Command::SelectToLineStart => self.cursor.select_to_line_edge(false),
            Command::SelectToBottom => self.cursor.select_to_page_edge(true),
            Command::SelectToTop => self.cursor.select_to_page_edge(false),
            Command::SelectNextPage => self.cursor.select_page(true),
            Command::SelectPriorPage => self.cursor.select_page(false),
            Command::SelectAll => self.cursor.select_all(),
            Command::SelectHtmlElement => self.cursor.select_current_element(),
            Command::CopySelection => self.copy_selection(),
            Command::SaySelectedText => Some(self.cursor.say_selection()),

            // Modal dialog commands
            Command::EnterModal => {
                let target_dlg = if let Some(node_id) = self.cursor.current_node_id {
                    self.cursor.buffer.dialog_for_node(node_id)
                } else {
                    None
                }
                .or_else(|| self.cursor.buffer.dialogs.first());

                if let Some(dlg) = target_dlg {
                    self.cursor.enter_modal(dlg.node_id)
                } else {
                    None
                }
            }
            Command::ExitModal => self.cursor.exit_modal(),

            // JAWS Say Scripts
            Command::SayAll => return self.start_say_all(),
            Command::SkimRead => return self.start_skim_read(),
            Command::SkimReadDialog => return self.open_skim_dialog(),
            // A page's focus with nothing of it in the buffer (the empty element Google Sheets
            // and Docs type into) has a blank line, as JAWS says (dev box, 2026-10-16), not
            // the virtual cursor's.
            Command::SayLine
                if self.forms_mode
                    && self.app_mode == AppMode::VirtualBuffer
                    && self
                        .last_page_focus
                        .is_some_and(|id| !self.cursor.buffer.shows(id)) =>
            {
                Some(msg::blank().to_string())
            }
            Command::SayLine => Some(self.at_margin(self.line_here())),
            // The Virtual Viewer's word where it is open [obs, 1063/s9].
            Command::SayWord => Some(match &self.viewer {
                Some(viewer) => viewer.word(),
                None => self.cursor.current_word(),
            }),
            Command::SayCharacter | Command::SayCharacterPhonetic => Some(self.character_here()),
            // Its value in decimal, said digit by digit [obs, 401/s3: "Character 7 4" for J;
            // s10: 8364 for the euro sign].
            Command::SayCharacterValue => self.character_here().chars().next().map(|c| {
                let digits = u32::from(c).to_string();
                let digits: Vec<String> = digits.chars().map(String::from).collect();
                msg::character_value(digits.join(" ")).render()
            }),

            // In-buffer find commands
            // "Virtual Find" first in a page, with long messages only [obs, 506/s1; 926/s1,
            // 927/s1].
            Command::VirtualFind => {
                if self.app_mode == AppMode::VirtualBuffer
                    && self.settings.verbosity.message_length()
                        == crate::settings::MessageLength::Long
                {
                    self.speech.speak(msg::virtual_find().as_str(), true);
                }
                return self.open_find();
            }
            Command::OkurFind => return self.open_find(),
            Command::FindNext => self.cursor.find_next(),
            Command::FindPrior => self.cursor.find_prior(),

            // Element list dialogs, in Okur's windows; with nothing to list, said so.
            Command::SelectAHeading => return self.open_list(crate::ui::lists::ListKind::Headings),
            Command::SelectARegion => return self.open_list(crate::ui::lists::ListKind::Regions),
            Command::SelectElement(list) => {
                return self.open_list(crate::ui::lists::ListKind::Elements(*list))
            }
            Command::VirtualHtmlFeatures => {
                return self.open_list(crate::ui::lists::ListKind::Features)
            }
            Command::SelectALink => return self.open_list(crate::ui::lists::ListKind::Links),
            Command::SelectAFormField => {
                return self.open_list(crate::ui::lists::ListKind::FormFields)
            }

            _ => None,
        };
        let speech = if matches!(command, Command::SayNextLine | Command::SayPriorLine) {
            self.arrow_onto_field(command == Command::SayNextLine)
                .or(speech)
        } else {
            speech
        };

        if let Some(ref text) = speech {
            // Left/Right read a character, which the synthesizer names ("space").
            let character = matches!(
                command,
                Command::SayNextCharacter | Command::SayPriorCharacter | Command::SayCharacter
            ) && text.chars().count() == 1;
            // A character selected or unselected is named as when read alone, between its
            // line's marks and the selection word: the character ends the first line
            // [obs, 501/s2, 504/s17].
            let selected_character = matches!(
                command,
                Command::SelectNextCharacter | Command::SelectPriorCharacter
            )
            .then(|| {
                let (first, rest) = text.split_once('\n').unwrap_or((text.as_str(), ""));
                let character = first.chars().next_back()?;
                let before = first.get(..first.len() - character.len_utf8())?;
                Some((before, character.to_string(), rest))
            })
            .flatten();
            // A selection of lines, words or the page is read with every symbol named, as JAWS
            // reads it: "a clean virtual buffer period" [obs, 501/s12, 617/s3].
            let selected_text = matches!(
                command,
                Command::SelectNextLine
                    | Command::SelectPriorLine
                    | Command::SelectNextWord
                    | Command::SelectPriorWord
                    | Command::SelectToLineEnd
                    | Command::SelectToLineStart
                    | Command::SelectToBottom
                    | Command::SelectToTop
                    | Command::SelectNextPage
                    | Command::SelectPriorPage
                    | Command::SelectAll
                    | Command::SelectHtmlElement
            )
                // A word read on its own too, whatever the punctuation level: "month comma,",
                // "period.", "em dash —" [obs, 106/s4, s6, s23].
                || matches!(
                    command,
                    Command::SayNextWord | Command::SayPriorWord | Command::SayWord
                );
            // A word that is one symbol is read as that character alone: "em dash —" [obs,
            // 106/s23].
            let lone_symbol = matches!(
                command,
                Command::SayNextWord | Command::SayPriorWord | Command::SayWord
            ) && {
                let mut chars = text.chars();
                matches!((chars.next(), chars.next()), (Some(c), None) if !c.is_alphanumeric())
            };
            if command == Command::SayCharacterPhonetic && text.chars().count() == 1 {
                self.speech.speak_phonetic(text, true);
            } else if character || lone_symbol {
                self.speech.speak_character(text, true);
            } else if let Some((before, character, after)) = selected_character {
                self.speech
                    .speak_selected_character(before, &character, after, true);
            } else if selected_text && !text.is_empty() {
                // A long selection's last characters that begin with a line break: JAWS's
                // synthesizer says the break as "blank" after "characters up to", which its
                // history leaves out [obs, 620/s10, s14, s35]. Only a long selection's summary
                // keeps an empty line ([`crate::cursor`]).
                match text.split_once("\n\n") {
                    Some((summary, last)) => {
                        self.speech.speak_symbols(summary, true);
                        self.speech.speak_character("\n", false);
                        self.speech.speak_symbols(last, false);
                    }
                    None => self.speech.speak_symbols(text, true),
                }
            } else if !text.is_empty() {
                self.speech.speak(text, true);
            }
        }

        speech
    }

    /// Whether the text field typed in is a document, which reads some keys as a document
    /// does: a native one (Windows 11 Notepad), a native edit of several lines (a WinForms text
    /// box: Say Line gives the line alone, Ctrl+Home "top of file" [obs, 400/s1, s5]), or a
    /// page's multi-line field (Google Docs' document with braille support; JAWS on the dev
    /// box, 2026-10-16).
    pub(super) fn editing_document(&self) -> bool {
        let native = self.app_mode != AppMode::VirtualBuffer
            && self.last_native_focus.as_ref().is_some_and(|f| {
                f.role == Role::Document || (f.role == Role::Edit && f.reads_as_document())
            });
        let page = self.forms_mode
            && self.app_mode == AppMode::VirtualBuffer
            && self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.node(id))
                .is_some_and(|n| n.attributes.get("multiline").is_some_and(|m| m == "true"));
        native || page
    }

    /// Whether keys go to a text field: forms mode in a page, or a focused native edit or
    /// text document (Windows 11 Notepad, Word).
    pub(super) fn editing_text(&self) -> bool {
        // A combo box one types in is a text field too (Google Sheets' cell editor).
        (self.forms_mode
            && self.node_under_cursor().is_some_and(|n| {
                n.role == Role::Edit
                    || (n.role == Role::ComboBox
                        && n.attributes.get("editable").is_some_and(|e| e == "true"))
            }))
            || (self.app_mode != AppMode::VirtualBuffer
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| matches!(f.role, Role::Edit | Role::Document)))
    }

    /// Auto Forms Mode: an arrow that lands on a field one types in puts the focus there and
    /// turns forms mode on, and the field is said as its focus is, without the page's title or
    /// "blank": "Full name Edit Required", "Email address Edit Required | sam@example.com"
    /// [obs, 280/s5, s7, s11, s20]. JAWS's documentation: "When you use the ARROW keys ...
    /// and land in an edit field ..., forms mode is automatically on" (Surf's Up, "Using Forms
    /// with JAWS"); other controls stay in browse mode (280/s13, s18). The focus event that
    /// follows is not said again.
    pub(super) fn arrow_onto_field(&mut self, down: bool) -> Option<String> {
        if self.settings.forms_mode != crate::settings::FormsMode::Auto
            || self.forms_mode
            || self.app_mode != AppMode::VirtualBuffer
        {
            return None;
        }
        let mut node = self
            .node_under_cursor()
            .filter(|n| {
                let attribute = |name: &str| n.attributes.get(name).map(String::as_str);
                // An edit, or a combo box one types in (an autocomplete: Down then goes to the
                // page, which opens its list [obs, 286/s18, s19]).
                let typed_in = n.role == Role::Edit
                    || (n.role == Role::ComboBox && attribute("editable") == Some("true"));
                typed_in && attribute("readonly") != Some("true")
            })?
            .clone();
        // Down back onto the field Up left, straight back or a few lines later: JAWS turns forms
        // mode on with its sound alone, saying nothing, and the next arrow reads the field's
        // line where its caret was [obs, 626/s5, s6, s9; 567/s12; 632/s7]. Up back onto a
        // field Down left reads it, at once or later, though the focus stayed in it all along
        // [obs, 284/s7; 283/s13: no focus event from s8 to s14].
        if down && self.field_left == Some((node.id, false)) {
            self.forms_mode = true;
            self.cursor.forms_mode_active = true;
            self.page_focus_is_field = true;
            return Some(String::new());
        }
        // What was typed in it, which the captured page does not have.
        if let Some(text) = self.field_texts.get(&node.id) {
            node.value = Some(text.clone());
        }
        // The field as its focus is said, but for its description, which the next arrow reads
        // as a line of its own (280/s9), and an empty field's "blank" (280/s5).
        let mut field = SpeechFormatter::control_focus(&node, |_| false);
        field.retain(|kind| kind != crate::announcement::PartKind::Description);
        let said = field.render();
        let blank = msg::blank();
        let said = said
            .strip_suffix(blank.as_str())
            .map_or(said.as_str(), |s| s.trim_end_matches(['\n', ' ']))
            .to_owned();
        // A text area reached by an arrow reads its text, where Tab says only that it has
        // some [obs, 285/s1].
        let contains = msg::field_contains_text();
        let said = match (said.strip_suffix(contains.as_str()), node.value.as_deref()) {
            (Some(field), Some(text)) if !text.trim().is_empty() => format!("{field}{text}"),
            _ => said,
        };
        if let Some(actions) = &self.actions {
            actions.focus(&node);
        }
        // The field is the page's focus now, with its caret at the start where focusing puts
        // it [obs, 280/s7, 284/s6], until the focus event and a read of the field say more.
        self.remember_field_text();
        self.arrow_focus = Some(node.id);
        self.last_page_focus = Some(node.id);
        self.page_focus_is_field = true;
        self.last_text = Some(crate::event::TextContext {
            text: node.value.clone().unwrap_or_default(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        });
        self.forms_mode = true;
        self.cursor.forms_mode_active = true;
        Some(said)
    }

    /// Keeps the text known in the page's focused field, as the focus or an arrow leaves it.
    pub(super) fn remember_field_text(&mut self) {
        let Some(id) = self.last_page_focus else {
            return;
        };
        let is_field = self
            .cursor
            .buffer
            .node(id)
            .is_some_and(|n| n.role == Role::Edit);
        if let (true, Some(context)) = (is_field, self.last_text.as_ref()) {
            self.field_texts.insert(id, context.text.clone());
        }
    }

    /// Auto Forms Mode: Up or Down in a page's field whose caret cannot move further that way
    /// leaves forms mode, and the virtual cursor reads the line after or before the field:
    /// Down in an empty "Full name" says "Email address"; in "sam@example.com", Down first
    /// moves its caret and reads it, then leaves [obs, 280/s6, s8, s9, s12]. "If the caret
    /// reaches the field's boundary and you press an arrow key again, JAWS exits Forms Mode"
    /// (Surf's Up). After a text area JAWS jumped to the page's end instead (280/s21), an
    /// accepted divergence.
    pub(super) fn arrow_out_of_field(&mut self, event: &KeyEvent) -> Option<KeyAction> {
        let down = match event.key {
            Key::DownArrow => true,
            Key::UpArrow => false,
            _ => return None,
        };
        if self.settings.forms_mode != crate::settings::FormsMode::Auto
            || !event.modifiers.is_empty()
        {
            return None;
        }
        let field = self
            .last_page_focus
            .and_then(|id| self.cursor.buffer.node(id))
            .filter(|n| n.role == Role::Edit)
            .map(|n| n.id)?;
        let context = self.last_text.as_ref()?;
        let at_edge = if down {
            context.caret >= context.text.chars().count()
        } else {
            context.caret == 0
        };
        if !at_edge {
            return None;
        }
        self.remember_field_text();
        self.forms_mode = false;
        let _ = self.cursor.exit_forms_mode();
        self.field_left = Some((field, down));
        self.cursor.move_to_node_id(field);
        let speech = if down {
            self.cursor.move_next_line()
        } else {
            self.cursor.move_prior_line()
        };
        // The line reached may be a field itself.
        let speech = self.arrow_onto_field(down).or(speech);
        if let Some(text) = speech.as_deref().filter(|t| !t.is_empty()) {
            self.speech.speak(text, true);
        }
        Some(KeyAction::Consume)
    }

    /// Spell Word and Spell Line (Say Word or Say Line pressed twice quickly): the word or line
    /// at the caret in a text field, at the virtual cursor elsewhere, spelled
    /// [obs, 406/s2, s5, s7; 408/s3, s6, s8].
    pub(super) fn spell(&mut self, line: bool) -> Option<String> {
        let text = if self.editing_text() {
            let context = self.last_text.as_ref()?;
            if line {
                crate::editing::line_at(context)
            } else {
                crate::editing::word_around(context)
            }
        } else if line {
            self.cursor
                .buffer
                .line_at_offset(self.cursor.offset)
                .map(|l| l.text.clone())?
        } else {
            self.cursor
                .buffer
                .word_at_offset(self.cursor.offset)
                .map(|w| w.text.clone())?
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        // The virtual cursor's line is spelled to its end, which JAWS says as a space: "b a n
        // n e r space" [obs, 654/s2, 408/s8]; a text field's line is not (406/s8), nor a
        // page's last line where its text ends without a line break [obs, 613/s24].
        let buffer = &self.cursor.buffer;
        let last_line = buffer.lines.len().checked_sub(1);
        let at_page_end =
            Some(self.cursor.current_line_index()) == last_line && !buffer.ends_with_line_break();
        let text = if line && !self.editing_text() && !at_page_end {
            format!("{trimmed} ")
        } else {
            trimmed.to_owned()
        };
        self.speech.speak_spelled(&text, true);
        Some(crate::pronounce::spelled(&text))
    }

    /// The line Say Line reads. On the field that has the page's focus, a text field or a
    /// combo box whose label is a line of its own, it is the field as its quick key says it,
    /// without "blank" or its description: "Full name Edit Required", "Home branch Combo box
    /// collapsed Northside" [obs, 507/s7, 248/s4, 249/s4]; Down onto it reads the line alone
    /// [obs, 201/s5, s17]. A radio button is said with its position [obs, 246/s4]; JAWS leaves
    /// it out right after its mouse click checked one, an accepted divergence (534/s10).
    /// `line`, the virtual cursor's line read whole, with "Left Margin" before it when
    /// indentation is said and the line starts at the margin [obs, 729/s1, s7, s8].
    pub(super) fn at_margin(&self, line: String) -> String {
        let offset = self.cursor.offset;
        self.with_margin_mark(offset, line)
    }

    /// `text`, read from the virtual cursor's line at `offset`, with "Left Margin" before
    /// it when indentation is said and that line starts at the margin.
    pub(super) fn with_margin_mark(&self, offset: usize, text: String) -> String {
        let at_margin = self.settings.indentation
            && self.app_mode == AppMode::VirtualBuffer
            && !self.forms_mode
            && self
                .cursor
                .buffer
                .line_at_offset(offset)
                .is_some_and(|l| l.text.starts_with(|c: char| !c.is_whitespace()));
        if at_margin {
            format!(
                "{}{text}",
                crate::linearize::state_marker(msg::indentation_left_margin().as_str())
            )
        } else {
            text
        }
    }

    pub(super) fn line_here(&self) -> String {
        use crate::announcement::PartKind;
        // A focused tab adds its place among its list's tabs: "Advanced Tab 2 of 2" [obs,
        // 260/s4].
        let line = self.cursor.buffer.line_at_offset(self.cursor.offset);
        if let Some(tab) = line
            .filter(|line| Some(line.node_id) == self.last_page_focus)
            .and_then(|line| {
                self.cursor
                    .buffer
                    .tabs
                    .iter()
                    .find(|t| t.node_id == line.node_id)
            })
        {
            return format!(
                "{}\n{}",
                self.cursor.current_line(),
                msg::position(tab.index.to_string(), tab.total.to_string())
            );
        }
        // A field's line, focused or only under the virtual cursor, is said as the field with
        // its name, which its line leaves to the label above it: "Full name Edit Required"
        // where Down says "Required Edit" [obs, 564/s2, s15, s16].
        let focused_field = self
            .cursor
            .buffer
            .line_at_offset(self.cursor.offset)
            .and_then(|line| self.cursor.buffer.node(line.node_id))
            .filter(|node| {
                matches!(
                    node.role,
                    crate::aom::Role::Edit | crate::aom::Role::ComboBox
                )
            })
            // A text area's lines under the virtual cursor are read as lines: "Edit |
            // C:\Users\Ada\notes.txt and /usr/local/bin" [obs, 508/s6].
            .filter(|node| {
                Some(node.id) == self.last_page_focus
                    || node.attributes.get("multiline").is_none_or(|m| m != "true")
            });
        let Some(node) = focused_field else {
            return self.cursor.current_line();
        };
        let mut field = SpeechFormatter::control_focus(node, |_| true);
        field.retain(|kind| kind != PartKind::Description);
        let blank = msg::blank().to_string();
        field.lines.retain(|line| line.render().trim() != blank);
        field.render()
    }

    /// The character Say Character reads: the caret's in a text field, the virtual cursor's
    /// elsewhere.
    fn character_here(&self) -> String {
        if self.editing_text() {
            return self
                .last_text
                .as_ref()
                .and_then(crate::editing::char_at)
                .map(String::from)
                .unwrap_or_default();
        }
        self.cursor.current_char()
    }

    /// An Okur key command (Insert+…). In a text field the reading commands read the field;
    /// elsewhere they run as virtual cursor or application commands.
    pub(super) fn okur_key_command(&mut self, event: &KeyEvent) {
        // Word's Insert+Shift+F, Insert+Shift+E and Insert+Alt+B: its footnotes, endnotes and
        // bookmarks listed [obs, 1083/s1, 1090/s1, 1091/s1].
        let m = event.modifiers;
        let notes = match (event.key, m.shift, m.alt) {
            (Key::Alpha('F'), true, false) => Some(crate::event::NoteKind::Footnote),
            (Key::Alpha('E'), true, false) => Some(crate::event::NoteKind::Endnote),
            (Key::Alpha('B'), false, true) => Some(crate::event::NoteKind::Bookmark),
            _ => None,
        };
        if let Some(kind) = notes.filter(|_| !m.ctrl) {
            if self.open_document_notes(kind) {
                return;
            }
        }
        // Word's Insert+Delete, the caret's line and column: Route Virtual to PC elsewhere.
        if event.key == Key::Delete && !m.ctrl && !m.shift && !m.alt && !m.win {
            let said = self
                .last_text
                .as_ref()
                .filter(|_| self.in_word_document())
                .and_then(crate::apps::windows::word::line_and_column);
            if let Some(said) = said {
                self.speech.speak(&said, true);
                return;
            }
        }
        // Word's Insert+Shift+R and Ctrl+Insert+R, its revisions: before the regions list,
        // whose key Ctrl+Insert+R is elsewhere.
        if let Some(key) = crate::apps::windows::word::RevisionKey::of(event) {
            if self.ask_revisions(key) {
                return;
            }
        }
        let layout_command = crate::command::layout_command_for(&self.settings.keyboard, event);
        let say_line = layout_command == Some(Command::SayLine)
            || crate::editing::EditKey::of(event).is_some();
        // The Okur cursor reads where it is [obs, 690/s6, s10; 752].
        if self.okur_place_active() {
            let command = if say_line {
                Some(Command::SayLine)
            } else if event.key == Key::Numpad5 && event.modifiers.is_empty() {
                Some(Command::SayCharacter)
            } else {
                layout_command
                    .clone()
                    .or_else(|| crate::command::command_for(event))
            };
            if command
                .and_then(|command| self.okur_cursor_command(&command))
                .is_some()
            {
                return;
            }
        }
        if self.editing_text() && say_line {
            // Pressed twice quickly, the line is spelled [obs, 406/s7].
            if self.presses(Press::Command(Command::SayLine), event.time_ms, 2) == 2 {
                let _ = self.spell(true);
                return;
            }
            // Say Line does not change the field, so no report follows: speak the field's
            // last known text now, or read it when none is known [obs, 305/s9, s15]. A
            // document (Notepad, Google Docs) gives the line alone (JAWS, dev box).
            let document = self.editing_document();
            let key = if document {
                crate::editing::EditKey::Line
            } else {
                crate::editing::EditKey::SayLine
            };
            match self.last_text.clone() {
                Some(context) => {
                    let speech = if document {
                        // A form field of Word's at the caret, as a control [obs, 1056/s1];
                        // a heading of Word's with its level [obs, 518/s1].
                        crate::apps::windows::word::form_field_at(&context)
                            .map(|field| {
                                crate::apps::windows::word::form_field_speech(
                                    &context, field, false,
                                )
                            })
                            .or_else(|| {
                                Some(crate::apps::windows::word::spoken_line(
                                    &context,
                                    crate::editing::line_at(&context)
                                        .trim_matches(|c: char| c.is_whitespace() || c == '﻿')
                                        .to_owned(),
                                ))
                            })
                    } else {
                        crate::editing::speech_in(
                            key,
                            None,
                            &context,
                            &self.text_field_line(),
                            self.in_plain_edit(),
                            crate::editing::Wording::of(&self.settings.verbosity),
                        )
                    };
                    match speech {
                        // An empty line is read as its line break: "blank", kept out of
                        // JAWS's history [obs, 315, Insert+Up].
                        Some(line) if line.trim().is_empty() => {
                            self.speech.speak_character("\n", true);
                        }
                        Some(speech) => self.speech.speak(&speech, true),
                        None => {}
                    }
                }
                None => self.expect_text_change(key),
            }
            return;
        }
        // A reading command that moves first is the matching arrow key: the application's
        // own in a native control (sent, then read as any arrow key is), the virtual
        // cursor's in a page. Insert+Right and Insert+Left move by word in both layouts
        // [obs, 504/s12, s13].
        let moving_command = layout_command.clone().or_else(|| {
            crate::command::command_for(event)
                .filter(|c| matches!(c, Command::SayNextWord | Command::SayPriorWord))
        });
        let moving = match &moving_command {
            Some(Command::SayNextLine) => Some(vec![Key::DownArrow]),
            Some(Command::SayPriorLine) => Some(vec![Key::UpArrow]),
            Some(Command::SayNextWord) => Some(vec![Key::Control, Key::RightArrow]),
            Some(Command::SayPriorWord) => Some(vec![Key::Control, Key::LeftArrow]),
            Some(Command::SayPriorCharacter) => Some(vec![Key::LeftArrow]),
            _ => None,
        };
        if let (Some(chord), Some(command)) = (moving, &moving_command) {
            // A word on a native control that is not a text field is the control itself
            // [obs, 503/s6, s7].
            let native_control_word = self.app_mode != AppMode::VirtualBuffer
                && !self.editing_text()
                && matches!(command, Command::SayNextWord | Command::SayPriorWord);
            if (self.app_mode == AppMode::VirtualBuffer && !self.forms_mode) || native_control_word
            {
                let _ = self.execute(command.clone());
            } else if let Some(actions) = &self.actions {
                actions.press(&chord);
            }
            return;
        }
        // Say All in a document reads its lines from the caret's to the last (JAWS in Google
        // Docs' document with braille support, dev box 2026-10-16), not the page around it.
        let say_all = layout_command
            .clone()
            .or_else(|| crate::command::command_for(event))
            == Some(Command::SayAll);
        if self.editing_document() && say_all {
            let _ = self.start_field_say_all();
            return;
        }
        // The desktop layout's Say Word (Insert+Numpad5) reads the field in the laptop layout
        // too, as JAWS's did: "Misspelled teh" [obs, 567/s8].
        let word_command = layout_command
            .clone()
            .or_else(|| crate::command::command_for(event));
        // In the Virtual Viewer the viewer's word is read, not the field behind it [obs,
        // 1063/s9].
        if self.editing_text() && self.viewer.is_none() && word_command == Some(Command::SayWord) {
            // Pressed twice quickly, the word is spelled [obs, 406/s2, s5].
            if self.presses(Press::Command(Command::SayWord), event.time_ms, 2) == 2 {
                let _ = self.spell(false);
                return;
            }
            // A misspelled word says so first, as the word keys do [obs, 567/s8].
            let plain_edit = self.in_plain_edit();
            let word =
                self.last_text
                    .as_ref()
                    .map(|text| {
                        let mark = crate::editing::misspelled_word_mark(text).unwrap_or_default();
                        // In a plain Win32 edit, on a symbol, the symbol alone: "\" where UI
                        // Automation's word is ":\" [obs, 1037/s6, s40, s73].
                        let symbol =
                            text.text.chars().nth(text.caret).filter(|c| {
                                plain_edit && !c.is_alphanumeric() && !c.is_whitespace()
                            });
                        let word = match symbol {
                            Some(symbol) => symbol.to_string(),
                            None => crate::editing::word_around(text),
                        };
                        let word = word.trim();
                        if word.is_empty() {
                            String::new()
                        } else {
                            format!("{mark}{word}")
                        }
                    })
                    .unwrap_or_default();
            // Every symbol named, whatever the punctuation level: "colon back slash" [obs,
            // 1041/s40]; a word that is one symbol, the character alone.
            let mut chars = word.chars();
            match (chars.next(), chars.next()) {
                (None, _) => {}
                (Some(c), None) if !c.is_alphanumeric() => self.speech.speak_character(&word, true),
                _ => self.speech.speak_symbols(&word, true),
            }
            return;
        }
        if let Some(command) = layout_command.or_else(|| crate::command::command_for(event)) {
            self.execute_pressed(command, event.time_ms);
        }
    }

    /// Runs `command`, or what pressing its keys twice or three times quickly runs.
    pub(super) fn execute_pressed(&mut self, command: Command, time_ms: u64) {
        let most = if command.pressed_thrice().is_some() {
            3
        } else {
            2
        };
        let command = match self.presses(Press::Command(command.clone()), time_ms, most) {
            2 => command.pressed_twice().unwrap_or(command),
            3 => command.pressed_thrice().unwrap_or(command),
            _ => command,
        };
        let _ = self.execute(command);
    }

    /// How many times in a row `press` has been pressed, each quickly after the last, counting
    /// up to `most` ([`Command::pressed_twice`], [`Command::pressed_thrice`]); the press after
    /// the last one counted is a first press again.
    fn presses(&mut self, press: Press, time_ms: u64, most: u32) -> u32 {
        let count = match &self.last_press {
            Some((last, at, count))
                if *last == press
                    && time_ms != 0
                    && time_ms.saturating_sub(*at) <= DOUBLE_PRESS_MS
                    && *count < most =>
            {
                count + 1
            }
            _ => 1,
        };
        self.last_press = (count < most).then_some((press, time_ms, count));
        count
    }

    /// Whether `press` repeats the last press quickly enough to be a double press. A double
    /// press is not the first press of another.
    pub(super) fn pressed_twice(&mut self, press: Press, time_ms: u64) -> bool {
        self.presses(press, time_ms, 2) == 2
    }

    /// CapsLock as the JAWS key: a single press is the JAWS key; pressed twice quickly it
    /// toggles Caps Lock (the second press reaches the system), which JAWS announces
    /// [obs, 310: "Caps lock on", then "Caps lock off"].
    pub(super) fn caps_lock_press(&mut self, event: &KeyEvent) -> KeyAction {
        if !self.pressed_twice(Press::CapsLock, event.time_ms) {
            return KeyAction::Consume;
        }
        let was_on = event.caps_lock_on.unwrap_or(self.caps_lock_on);
        self.caps_lock_on = !was_on;
        let speech = if was_on {
            msg::caps_lock_off()
        } else {
            msg::caps_lock_on()
        };
        self.speech.speak(speech.as_str(), true);
        KeyAction::PassThrough
    }

    /// Say System Time and its variants, from the clock in the user's locale formats.
    fn clock_command(&self, command: &Command) -> Option<String> {
        let clock = self.clock.as_deref();
        match command {
            Command::SaySystemTime => Some(crate::clock::say_time(clock?, false)),
            Command::SaySystemTimeWithSeconds => Some(crate::clock::say_time(clock?, true)),
            Command::SaySystemDate => Some(crate::clock::say_date(clock?, false)),
            Command::SayWeekNumber => Some(crate::clock::say_date(clock?, true)),
            _ => None,
        }
    }

    /// Quick navigation to the next or prior `element`.
    fn move_to_element(&mut self, element: Element, next: bool) -> Option<String> {
        if element == Element::PlaceMarker {
            return self.move_to_placemarker(next);
        }
        let c = &mut self.cursor;
        match (element, next) {
            (Element::Heading, true) => c.move_next_heading(),
            (Element::Heading, false) => c.move_prior_heading(),
            (Element::HeadingLevel(level), true) => c.move_next_heading_level(u32::from(level)),
            (Element::HeadingLevel(level), false) => c.move_prior_heading_level(u32::from(level)),
            (Element::Link, true) => c.move_next_link(),
            (Element::Link, false) => c.move_prior_link(),
            (Element::Table, true) => c.move_next_table(),
            (Element::Table, false) => c.move_prior_table(),
            (Element::Article, true) => c.move_next_article(),
            (Element::Article, false) => c.move_prior_article(),
            (Element::OnMouseOver, true) => c.move_next_on_mouse_over(),
            (Element::OnMouseOver, false) => c.move_prior_on_mouse_over(),
            (Element::OnClick, next) => c.move_on_click(next),
            (Element::Tab, true) => c.move_next_tab(),
            (Element::Tab, false) => c.move_prior_tab(),
            // Handled before: the engine keeps the PlaceMarkers.
            (Element::PlaceMarker, _) => None,
            (Element::Graphic, true) => c.move_next_graphic(),
            (Element::Graphic, false) => c.move_prior_graphic(),
            (Element::RadioButton, true) => c.move_next_radio_button(),
            (Element::RadioButton, false) => c.move_prior_radio_button(),
            (Element::Math, true) => c.move_next_math(),
            (Element::Math, false) => c.move_prior_math(),
            (Element::Division, forward) => c.move_division(forward),
            (Element::Separator, forward) => c.move_separator(forward),
            (Element::MailLink, forward) => c.move_mail_link(forward),
            (Element::VisitedLink, forward) => c.move_link_by_visit(true, forward),
            (Element::UnvisitedLink, forward) => c.move_link_by_visit(false, forward),
            (Element::FormField, true) => c.move_next_form_control(),
            (Element::FormField, false) => c.move_prior_form_control(),
            (Element::Button, true) => c.move_next_button(),
            (Element::Button, false) => c.move_prior_button(),
            (Element::Edit, true) => c.move_next_edit(),
            (Element::Edit, false) => c.move_prior_edit(),
            (Element::CheckBox, true) => c.move_next_checkbox(),
            (Element::CheckBox, false) => c.move_prior_checkbox(),
            (Element::ComboBox, true) => c.move_next_combobox(),
            (Element::ComboBox, false) => c.move_prior_combobox(),
            (Element::List, true) => c.move_next_list(),
            (Element::List, false) => c.move_prior_list(),
            (Element::ListItem, true) => c.move_next_list_item(),
            (Element::ListItem, false) => c.move_prior_list_item(),
            (Element::Landmark, true) => c.move_next_landmark(),
            (Element::Landmark, false) => c.move_prior_landmark(),
            (Element::Frame, true) => c.move_next_frame(),
            (Element::Frame, false) => c.move_prior_frame(),
        }
    }

    /// Say Font (`Insert+F`): the font at the cursor. Pressed twice quickly, JAWS shows it in
    /// the Virtual Viewer and reads the message [obs, 209/s7, 314/s2]; with no font it just
    /// says "No font" again (313/s2). A native field reports its font at the caret (UIA text
    /// attributes) or its window's font; JAWS's size for classic Notepad (314/s1, "22
    /// point") awaits a trace that recorded the font.
    fn say_font(&mut self, in_viewer: bool) -> Option<String> {
        // A page that draws its text itself shows the font in its own controls (Google Docs'
        // toolbar): they are read as they are now, and the font is said when they arrive
        // (`say_page_font`).
        if self.app_mode == AppMode::VirtualBuffer && self.forms_mode {
            let elements = self
                .last_page_focus
                .and_then(|id| self.cursor.buffer.node(id))
                .and_then(|focus| focus.attributes.get("id"))
                .zip(self.page_app())
                .map(|(id, app)| app.font_elements(id))
                .unwrap_or_default();
            if !elements.is_empty()
                && self
                    .actions
                    .as_ref()
                    .is_some_and(|a| a.read_page_elements(elements))
            {
                self.pending_font = Some(in_viewer);
                return None;
            }
        }
        // Word's document has a Say Font of its own [obs, 1034/s1, s3], with the line spacing
        // Word's object model gives, asked for first and said when it comes (`word_font`)
        // [obs, 1040/s1, s17, s19].
        if let Some(said) = self
            .last_text
            .as_ref()
            .filter(|_| {
                !in_viewer && self.app_mode != AppMode::VirtualBuffer && self.editing_text()
            })
            .and_then(|text| crate::apps::windows::word::font(text, None))
        {
            if self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_line_spacing())
            {
                self.pending_word_font = true;
                return None;
            }
            self.speech.speak(&said, true);
            return Some(said);
        }
        let formatting = if self.app_mode == AppMode::VirtualBuffer && !self.forms_mode {
            self.cursor.buffer.formatting_at_offset(self.cursor.offset)
        } else {
            // A text field reports the font at its caret.
            self.last_text
                .as_ref()
                .and_then(|t| t.font.as_ref())
                .map(crate::event::FontInfo::formatting)
        };
        self.speak_font(formatting, in_viewer)
    }

    /// Say Color (Insert+5): in a page, the text's color and its background's at the virtual
    /// cursor, "blue2 on white"; "default on default" where the cursor is on no text
    /// [obs, 565, 618, 530/s2]. Elsewhere a text field's colors, which JAWS reads from the
    /// screen; other controls have none.
    fn say_color(&mut self) -> Option<String> {
        let caret_colors = || {
            self.last_text
                .as_ref()
                .and_then(|t| t.font.as_ref())
                .map(crate::event::FontInfo::formatting)
        };
        let formatting = if self.app_mode != AppMode::VirtualBuffer || self.forms_mode {
            // A text field's colors at its caret, "black on white" [obs, 661/s1], a page's in
            // forms mode too, an empty one included ("navy on yellow", "black on white"
            // [obs, 676/s4, s6, s8; 662/s5]); JAWS says "default on default" where it finds
            // none, on a dialog's button [obs, 513/s3].
            caret_colors()
        } else if let Some(field) = self.cursor.buffer.find_edit_node(self.cursor.offset) {
            // With the virtual cursor a page's field has the colors of its text, and none
            // while it is empty: the focused field's as typed in, another's as the page shows
            // them [obs, 662/s2, s7; 676/s2, s10, s12].
            let focused = Some(field.id) == self.last_page_focus;
            match self.last_text.as_ref().filter(|_| focused) {
                Some(typed) if typed.text.is_empty() => None,
                Some(_) => caret_colors().or_else(|| field.formatting.clone()),
                None => field.formatting.clone(),
            }
        } else {
            self.cursor
                .buffer
                .text_formatting_at_offset(self.cursor.offset)
        };
        let name = |color: Option<crate::color::Rgb>| {
            color
                .and_then(crate::color::name_of)
                .map_or_else(|| msg::color_default().to_string(), str::to_owned)
        };
        let (foreground, background) = match formatting {
            Some(f) => (name(f.color), name(f.background)),
            None => (name(None), name(None)),
        };
        self.say(msg::color_on(foreground, background).to_string())
    }

    /// Says the font that the page's own controls show, read for Say Font (`pending_font`).
    /// Elements nobody waits for say nothing.
    pub(super) fn say_page_font(&mut self, elements: &[crate::aom::AomNode]) -> Option<String> {
        let in_viewer = self.pending_font.take()?;
        let formatting = self
            .page_app()
            .and_then(|app| app.font_from_elements(elements));
        self.speak_font(formatting, in_viewer)
    }

    /// Says `formatting` as Say Font does, or shows it in the Virtual Viewer (`in_viewer`).
    fn speak_font(
        &mut self,
        formatting: Option<crate::aom::TextFormatting>,
        in_viewer: bool,
    ) -> Option<String> {
        let description = formatting
            .as_ref()
            .and_then(SpeechFormatter::font_description);
        // A native text field that reports its alignment adds its paragraph formatting, after
        // a comma (JAWS 2025 in Windows 11 Notepad, dev box: "Font is Consolas 11 point, |
        // Paragraph formatting left justified"), and so does a page's field (Google Docs'
        // document with braille support, dev box 2026-10-16); the virtual cursor (209/s3) and
        // fields known only by their window font (314/s1) do not.
        let in_browse_mode = self.app_mode == AppMode::VirtualBuffer && !self.forms_mode;
        let paragraph = formatting
            .as_ref()
            .and_then(|f| f.alignment.as_deref())
            .filter(|_| !in_browse_mode)
            .map(SpeechFormatter::paragraph_formatting);
        let description = match (description, &paragraph) {
            (Some(font), Some(_)) => Some(format!("{font},")),
            (font, _) => font,
        };
        let speech = match (in_viewer, description) {
            (true, Some(font)) if self.viewer.is_none() => {
                let viewer = crate::viewer::VirtualViewer::message(vec![
                    msg::viewer_font_information().to_string(),
                    font,
                    paragraph.unwrap_or_default(),
                    String::new(),
                    format!(
                        "\u{1d} {} \u{1e} {}",
                        msg::role_link(),
                        msg::viewer_hot_keys_link()
                    ),
                    msg::viewer_close_hint().to_string(),
                ])
                .with_links(vec![(4, Command::HotKeyHelp)]);
                let speech = viewer.message_speech();
                self.viewer = Some(viewer);
                self.show_viewer_window();
                speech
            }
            (_, Some(font)) => match paragraph {
                Some(paragraph) => format!("{}\n{paragraph}", msg::font_is(font)),
                None => msg::font_is(font).render(),
            },
            (_, None) => SpeechFormatter::format_font(None),
        };
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Virtualize Window or Virtualize Current Control: opens the Virtual Viewer on the
    /// window's or the control's text and reads its first line [obs, 311/s1, s14; 312/s1].
    fn virtualize(&mut self, window: bool) -> Option<String> {
        // A page's virtual cursor is a virtual view already: JAWS virtualizes neither the
        // window nor the control (observed on the dev box with JAWS 2025 in Chrome,
        // 2026-10-15; scenario 318 records it on old-pc).
        let in_page = self.app_mode == AppMode::VirtualBuffer && !self.forms_mode;
        if self.viewer.is_some() || in_page {
            // [obs, 311/s32]
            let speech = msg::viewer_already_virtual().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        // A console's text is its buffer as it is now: it is read fresh, and the viewer opens
        // when it arrives (`open_console_viewer`).
        let console = self
            .last_native_focus
            .as_ref()
            .and_then(|focus| self.app_for(focus))
            .is_some_and(|app| app.virtualizes_text());
        if console && self.actions.as_ref().is_some_and(|a| a.read_text()) {
            self.pending_viewer = Some(PendingViewer::Console { window });
            return None;
        }
        // A text field's text is its lines: the control's, and the window's when the text is
        // what the window is for (Notepad, where JAWS shows only "paragraph"). A field among
        // others (a form's Name field) is one row of its window. Anything else is read as it
        // is on screen, and the viewer opens when it arrives (`open_screen_viewer`).
        let text_field = self.editing_text() && (!window || self.text_is_the_window());
        let native = self.app_mode != AppMode::VirtualBuffer;
        if native && !text_field && self.actions.as_ref().is_some_and(|a| a.read_window(window)) {
            self.pending_viewer = Some(PendingViewer::Screen { window });
            return None;
        }
        let lines = self.virtualized_lines(window)?;
        self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
    }

    /// Whether the focused text is what its window shows: a document, or a field of several
    /// lines outside a dialog box (an editor's page, not a form's field).
    fn text_is_the_window(&self) -> bool {
        let Some(focus) = self.last_native_focus.as_ref() else {
            return false;
        };
        let dialog = focus.in_dialog_box;
        let lines = self
            .last_text
            .as_ref()
            .is_some_and(|t| t.text.contains(['\r', '\n']));
        focus.role == Role::Document || (lines && !dialog)
    }

    /// Opens the Virtual Viewer that waits for the screen's text (`pending_viewer`). With
    /// nothing read, it shows what the focus says, as without a screen read.
    pub(super) fn open_screen_viewer(&mut self, lines: &[String]) -> Option<String> {
        let Some(PendingViewer::Screen { window }) = self.pending_viewer else {
            return None;
        };
        self.pending_viewer = None;
        let lines = if lines.is_empty() {
            self.virtualized_lines(window)?
        } else {
            lines.to_vec()
        };
        self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
    }

    /// Opens `viewer` and speaks what JAWS speaks: what was virtualized, and the line the
    /// viewer is on.
    fn open_viewer(
        &mut self,
        viewer: crate::viewer::VirtualViewer,
        window: bool,
    ) -> Option<String> {
        let what = if window {
            msg::viewer_virtualizing_window()
        } else {
            msg::viewer_virtualizing_control()
        };
        let speech = format!("{what}\n{}", viewer.line_text());
        self.viewer = Some(viewer);
        self.show_viewer_window();
        self.speech.speak(&speech, true);
        Some(speech)
    }

    /// Opens the Virtual Viewer that waits for a console's text (`pending_viewer`), at the
    /// cursor's line. Returns `None` when no viewer is waiting.
    pub(super) fn open_console_viewer(
        &mut self,
        context: &crate::event::TextContext,
    ) -> Option<Option<String>> {
        let Some(PendingViewer::Console { window }) = self.pending_viewer else {
            return None;
        };
        self.pending_viewer = None;
        self.last_text = Some(context.clone());
        // The application that virtualizes its text lays the lines out (a console).
        let (lines, line) = self
            .last_native_focus
            .as_ref()
            .and_then(|focus| {
                self.app_for(focus)?
                    .viewer_lines(focus, &context.text, context.caret)
            })
            .unwrap_or_default();
        if lines.is_empty() {
            // Nothing read: the viewer shows the control, as for any other.
            let lines = self.virtualized_lines(window);
            return Some(lines.and_then(|lines| {
                self.open_viewer(crate::viewer::VirtualViewer::new(lines), window)
            }));
        }
        let viewer = crate::viewer::VirtualViewer::at_line(lines, line);
        Some(self.open_viewer(viewer, window))
    }

    /// The Virtual Viewer's lines for the focused native control or its window:
    /// - a dialog box: its title, its text, and its buttons on one line [obs, 311/s14–s22];
    /// - a text field: its lines, after the window title for the whole window [obs, 312];
    /// - another control: its name and type, then a blank line [obs, 311/s1–s11].
    fn virtualized_lines(&self, window: bool) -> Option<Vec<String>> {
        let focus = self.last_native_focus.as_ref()?;
        let title = focus.window_title.clone().unwrap_or_default();
        if window && focus.in_dialog_box {
            let mut lines = vec![title];
            // As laid out on screen: wrapped text by its rows, a label beside its field
            // (JAWS on the Run dialog: "Open: C:", "OK Cancel Browse...").
            if !focus.dialog_lines.is_empty() {
                lines.extend(focus.dialog_lines.iter().cloned());
                return Some(lines);
            }
            lines.extend(
                focus
                    .dialog_text
                    .iter()
                    .flat_map(|t| t.lines())
                    .map(str::to_owned),
            );
            if !focus.dialog_buttons.is_empty() {
                lines.push(focus.dialog_buttons.join(" "));
            }
            return Some(lines);
        }
        if self.editing_text() {
            if let Some(text) = &self.last_text {
                let mut lines: Vec<String> = Vec::new();
                // The window: its title, its menu bar, the text, its status bar, as on screen
                // ("File Edit Format View Help" ... "Ln 1, Col 1 100% Windows (CRLF) UTF-8"
                // [obs, 314/s10, s11]).
                if window {
                    lines.push(title);
                    if !focus.menu_bar.is_empty() {
                        lines.push(focus.menu_bar.join(" "));
                    }
                }
                lines.extend(crate::editing::text_lines(&text.text));
                if window && !focus.status_bar.is_empty() {
                    lines.push(focus.status_bar.join(" "));
                }
                return Some(lines);
            }
        }
        let name = focus.name.clone().unwrap_or_default();
        let control = match crate::native_focus::role_word(&focus.role) {
            Some(role) => format!("{name} {role}"),
            None => name,
        };
        Some(vec![control, " ".to_owned()])
    }

    /// What JAWS reads when the Virtual Viewer closes: the text field's line, or the focused
    /// control's name and type [obs, 311/s12, s23; 312/s11].
    pub(super) fn speak_focus_again(&mut self) {
        let text_app = self
            .last_native_focus
            .as_ref()
            .and_then(|focus| self.app_for(focus))
            .filter(|app| app.virtualizes_text());
        let speech = if let Some(app) = text_app {
            // A console: its cursor's line, as last read (JAWS on the dev box, closing the
            // viewer: "C:\Users\Gigabyte>").
            self.last_text
                .as_ref()
                .and_then(|text| app.line_after_viewer(&text.text, text.caret))
        } else if self.editing_text() {
            // A field of one line says nothing: JAWS was silent closing the viewer over the
            // forms fixture's "Name: Ada" [obs, 322/s6, 526/s2, s22], and read the line of an
            // edit of several lines [obs, 312/s11, 527/s2].
            let single_line = self.app_mode != AppMode::VirtualBuffer
                && self.last_native_focus.as_ref().is_some_and(|f| {
                    f.role == Role::Edit
                        && !f.multi_line
                        && f.container_role != Some(Role::ComboBox)
                });
            // Word's document reads its line as a line read says it, then names itself: "The
            // library opens at has comment nine in the morning . | Edit" [obs, 1095/s10,
            // 1096/s7].
            let word_line = self
                .last_text
                .as_ref()
                .filter(|t| t.structure.is_some())
                .map(|t| {
                    let line = crate::editing::line_at(t)
                        .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
                        .to_owned();
                    format!(
                        "{}\n{}",
                        crate::apps::windows::word::spoken_line(t, line),
                        crate::linearize::role_marker(msg::role_edit().as_str())
                    )
                });
            let line = word_line.or_else(|| {
                self.last_text
                    .as_ref()
                    .filter(|_| !single_line)
                    .map(crate::editing::line_at)
            });
            // A combo box's field adds its list position (JAWS on the Run dialog, dev box:
            // "C: | 1 of 6").
            let position = self
                .last_native_focus
                .as_ref()
                .filter(|f| f.container_role == Some(Role::ComboBox))
                .and_then(|f| Some(msg::position(f.item_index?, f.item_count?).render()));
            match (line, position) {
                (Some(line), Some(position)) => Some(format!("{line}\n{position}")),
                (line, _) => line,
            }
        } else if self.app_mode == AppMode::VirtualBuffer {
            // In a page, the virtual cursor's line [obs, 209/s11].
            Some(self.cursor.current_line())
        } else {
            self.last_native_focus
                .as_ref()
                .map(crate::native_focus::after_viewer_line)
        };
        if let Some(speech) = speech.filter(|s| !s.is_empty()) {
            self.speech.speak(&speech, true);
        }
    }
}
