//! The screen reader engine: keys and accessibility events in, speech out.
//!
//! This module holds the engine's state and its two entry points,
//! [`ScreenReaderEngine::handle_key_event`] and
//! [`ScreenReaderEngine::handle_accessibility_event`]. The handlers live in submodules:
//! `commands` (reading commands and JAWS-key chords), `focus` (focus and menu events),
//! `text_feedback` (what edit keys say), `page` (pages, changes, selections, live regions) and
//! `state` (list, desktop, taskbar and File Explorer navigation state).

use crate::buffer::VirtualBuffer;
use crate::cursor::VirtualCursor;
use crate::event::{AccessibilityEvent, FocusEvent};
use crate::input::{Key, KeyAction, KeyEvent};
use crate::messages::msg;
use crate::speech::SpeechOutput;

/// The longest gap between two presses of a command that counts as "twice quickly", as JAWS
/// counts it, not Windows' double-click time (500 ms): Say Line pressed again 300 ms later
/// spelled the line in 3 of 3 trials, 350 ms later in 1 of 3, 400 ms later in none
/// [obs, 654]; Keyboard Help took 400 ms as two presses [obs, 642/s3].
const DOUBLE_PRESS_MS: u64 = 350;

mod clipboard;
mod commands;
mod console;
mod cursors;
mod find;
mod flexible_web;
mod focus;
mod help;
mod information;
mod jump;
mod key_help;
mod labels;
mod layer;
mod mouse_echo;
mod notifications;
mod page;
mod placemarkers;
mod quit;
mod reading;
mod routing;
mod say_all;
mod screen_text;
mod skim;
mod speech_mode;
mod startup_wizard;
mod state;
#[cfg(test)]
mod tests;
mod text_feedback;
mod toggles;
mod tray;
#[cfg(test)]
mod web_app_tests;
mod window_list;
mod word_list;

pub use state::AppMode;

/// A key press counted for "pressed twice quickly": a command, CapsLock alone (which the
/// laptop layout's Okur key also is), or a key Keyboard Help describes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Press {
    Command(crate::command::Command),
    CapsLock,
    /// A key described by Keyboard Help, with its modifiers.
    Described(Key, crate::input::Modifiers),
    /// An application's key that does more pressed twice quickly (Word's Alt+Shift+Apostrophe
    /// shows the comment in the Virtual Viewer), with its modifiers.
    Application(Key, crate::input::Modifiers),
}

/// A Virtual Viewer that opens once its text is read; `window` is Virtualize Window, not
/// Virtualize Current Control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingViewer {
    /// A console's buffer, arriving as a text context.
    Console { window: bool },
    /// What the window or control shows, arriving as viewer text.
    Screen { window: bool },
}

/// The marks the reading choices put on a page's root ([`ScreenReaderEngine::reading_marks`]):
/// each mark with its value, `None` where the mark is left off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingMarks([(&'static str, Option<String>); 3]);

impl ReadingMarks {
    /// Whether `page` carries these marks already.
    pub fn on(&self, page: &crate::aom::AomNode) -> bool {
        self.0
            .iter()
            .all(|(mark, value)| page.attributes.get(*mark) == value.as_ref())
    }

    /// Puts these marks on `page`'s root.
    pub fn mark(&self, page: &mut crate::aom::AomNode) {
        for (mark, value) in &self.0 {
            match value {
                Some(value) => {
                    page.attributes.insert((*mark).to_owned(), value.clone());
                }
                None => {
                    page.attributes.remove(*mark);
                }
            }
        }
    }
}

impl ScreenReaderEngine {
    /// `message`, or with short messages (Intermediate and Advanced verbosity) its short form
    /// where the catalog has one: "no form fields" [obs, 883/s2], "No PlaceMarkers" [obs,
    /// 883/s4].
    pub(crate) fn said_short(&self, message: crate::messages::StaticMessage) -> String {
        let said =
            if self.settings.verbosity.message_length() == crate::settings::MessageLength::Short {
                message.short().unwrap_or(message)
            } else {
                message
            };
        said.to_string()
    }
}

/// Primary runtime engine for screen reader execution, coordinating virtual cursor navigation,
/// speech output, and interactive input event handling.
pub struct ScreenReaderEngine {
    pub cursor: VirtualCursor,
    pub speech: Box<dyn SpeechOutput>,
    pub forms_mode: bool,
    pub app_mode: AppMode,
    pub last_window_title: Option<String>,
    /// The active window's class, with `last_window_title` identifying the window.
    last_window_kind: Option<String>,
    /// Container of the last focused native control; entering a new one announces it.
    native_focus: crate::native_focus::NativeFocus,
    /// The page node that last had focus, to drop repeated focus events on it.
    last_page_focus: Option<usize>,
    /// The page node that had the focus as the last key went down.
    focus_at_key: Option<usize>,
    /// The last key routed the focus to the virtual cursor (Ctrl+Insert+Delete).
    routed: bool,
    /// The select box with the focus opened its list with the last key: its list is said as
    /// "expanded" [obs, 276/s7].
    select_opened: bool,
    /// The last page focus was said as its line, without the table it is in: the next focus
    /// says the table as newly entered.
    focus_table_unsaid: bool,
    /// The page node the browser starts Tab from: the focus, or where Tab was last sent from.
    tab_start: Option<usize>,
    /// The list or grid item Enter was passed to in forms mode, until the next key: a focus
    /// that then lands elsewhere was put there by what the item opened.
    activated_item: Option<usize>,
    /// The page was asked to be captured again for a focus the buffer did not hold: the
    /// virtual cursor goes to the focus when the capture arrives.
    cursor_to_focus: bool,
    /// The page changed while forms mode was on: it is captured again when forms mode ends.
    page_stale: bool,
    /// The list whose option the focused field points at (aria-activedescendant), by name.
    option_list: Option<String>,
    /// The option of that list said last, which a repeated focus does not say again.
    option_said: Option<String>,
    /// The field an arrow put the focus on (Auto Forms Mode), already said: its focus event
    /// is not said again.
    arrow_focus: Option<usize>,
    /// A key has moved the virtual cursor since the page loaded: without Auto Forms Mode, Tab
    /// onto a field turns forms mode on only then [obs, 629, 630, 631].
    cursor_moved_since_load: bool,
    /// Keyboard Help (Insert+1) is on: keys are described, not run.
    keyboard_help: bool,
    /// The field an arrow last left in forms mode, and whether it went down, while the page's
    /// focus stays in it: Down back onto it after Up turns forms mode on silently, straight
    /// back or later [obs, 626/s5, s9; 567/s12; 632/s7].
    field_left: Option<(usize, bool)>,
    /// The text last known in a page's fields, by node, kept as the focus leaves them: the
    /// buffer keeps the values the page had when it was captured, and an arrow back onto a
    /// field says what was typed in it [obs, 284/s6].
    field_texts: std::collections::HashMap<usize, String>,
    /// The page's application region (ARIA role "application") the focus is in, by name:
    /// every key goes to the page there, Escape too.
    web_application: Option<String>,
    /// The page's focus is a text field or a combo box (one is typed in, its caret read).
    page_focus_is_field: bool,
    /// The last native focus outside menus, re-announced when menus close.
    last_native_focus: Option<FocusEvent>,
    /// The page of the wizard in front, kept while the focus is on its buttons, which are
    /// the sheet's and on no page.
    wizard_page: Option<String>,
    /// The item a field's list last selected, since the focus last moved: the same one again
    /// says its name alone.
    last_selected: Option<String>,
    /// Items an edit combo's lists selected since the last key, with their positions: the one
    /// the field's new text names is its own [obs, 541/s3].
    combo_items: Vec<(String, usize)>,
    /// The character the last key typed, if it typed one: a console's notification of just
    /// that character is its echo of the key.
    last_typed: Option<char>,
    /// The word typed so far in a text field, for word echo.
    typed_word: String,
    /// An edit key's result was read in a field its application also announces (a console):
    /// its announcements of the same change, until the next key, are not said.
    edit_read_in_console: bool,
    /// The last key extended a selection: Shift with an arrow key, Home, End or a page key.
    last_key_selects: bool,
    /// The last key was Tab or Shift+Tab: the focus it moves is the user's.
    last_key_tabs: bool,
    /// The last key was Escape alone: a dialog it closes may first move the focus to its
    /// Cancel button.
    last_key_escapes: bool,
    /// The last key was a letter alone: a ribbon's key tip, whose tab is not said.
    last_key_letter: bool,
    /// The last key pressed and its modifiers, for the application modules.
    last_key: Option<(Key, crate::input::Modifiers)>,
    /// Each page's temporary PlaceMarker (Ctrl+Windows+K), by its address, for the session.
    temp_placemarkers: std::collections::HashMap<String, usize>,
    /// The PlaceMarkers saved (Ctrl+Shift+K), and where they are kept.
    placemarkers: crate::placemarkers::PlaceMarkers,
    placemarker_store: Option<Box<dyn crate::placemarkers::PlaceMarkerStore>>,
    /// The PlaceMarker List while it is open, and the dialog it opened.
    placemarker_list: Option<placemarkers::ListState>,
    /// The page whose PlaceMarker List was opened since it loaded: K without PlaceMarkers
    /// opens the list there [obs, 703/s4].
    placemarker_list_seen: Option<String>,
    /// Flexible Web's wizard, while it is open.
    flexible_web: Option<crate::ui::flexible_web::FlexibleWebWizard>,
    /// The customizations Flexible Web made on the page in front.
    page_customizations: flexible_web::PageCustomizations,
    /// The Flexible Web rules saved, and where they are kept.
    flexible_web_rules: crate::flexible_web::Rules,
    flexible_web_store: Option<Box<dyn crate::flexible_web::RuleStore>>,
    /// Temporary customizations of a page left, while Okur asks whether to save them.
    flexible_web_leaving: Option<flexible_web::Leaving>,
    /// Flexible Web's rules dialog, while it is open.
    flexible_web_rules_dialog: Option<crate::ui::flexible_web::RulesDialog>,
    /// The custom labels (Insert+Ctrl+Tab), and where they are kept.
    custom_labels: crate::labels::CustomLabels,
    label_store: Option<Box<dyn crate::labels::LabelStore>>,
    /// Custom Label while it is open, and the node it labels.
    label_dialog: Option<(crate::ui::label::LabelDialog, usize)>,
    /// The Alt key is down: pressed on its own and not yet released (Alt held through
    /// Tabs in the task switcher).
    alt_held: bool,
    /// The next key, with the modifiers held with it, goes to the application as it is
    /// (Pass Key Through).
    pass_next_key: bool,
    menus: crate::native_focus::Menus,
    /// The page re-read after a change, waiting for the change's events to be handled.
    pending_page: Option<VirtualBuffer>,
    /// Page builds the host makes off the engine after a page node changed.
    page_builds: page::PageBuilds,
    /// Forms mode was just turned on by Enter (not by focus moving into a field).
    forms_mode_by_enter: bool,
    /// The focused control's access key, whatever the Access Key output: Say Focus says it.
    focus_access_key: Option<String>,
    /// The focused application's product name and version, as the last focus gave them
    /// (Ctrl+Insert+V).
    focus_app_version: Option<(String, String)>,
    /// The focused element's place on the screen: the PC cursor's.
    focus_bounds: Option<[i32; 4]>,
    /// Where the mouse pointer is, as the platform last reported it: the JAWS cursor's place.
    mouse: Option<(i32, i32)>,
    /// The cursor the reading keys act on.
    active_cursor: cursors::ActiveCursor,
    /// The Okur cursor's line in the focused text field, once routed to the PC cursor.
    okur_place: Option<cursors::OkurPlace>,
    /// Where the Okur cursor may go (Insert+R).
    okur_restriction: cursors::Restriction,
    /// Forms mode was just turned on by one of JAWS's element lists giving a field the focus:
    /// the field's focus is followed by its line, as Say Line reads it [obs, 249/s3].
    forms_mode_by_list: bool,
    /// The control Select a Form Field gave the focus to, after saying the page itself as it
    /// closed: the control's focus comes within the page, without the title [obs, 225/s10,
    /// 228/s9].
    page_said_for: Option<usize>,
    /// The edit key whose effect is spoken when the field next reports its text.
    pending_edit: Option<crate::editing::EditKey>,
    /// Ctrl+Home (true) or Ctrl+End (false) pressed in an application's grid: the cell it
    /// reaches is followed by "top of file" or "Bottom of file" and the cell again.
    pending_file_edge: Option<bool>,
    /// The focused text field's last known text, caret and selection.
    last_text: Option<crate::event::TextContext>,
    /// Actions on the real application (focus, default action); None when headless.
    actions: Option<Box<dyn crate::actions::ApplicationActions>>,
    /// The local time and locale formats for Say System Time; None when headless.
    clock: Option<Box<dyn crate::clock::Clock>>,
    /// Plays sound files: a notification rule's Play Sound.
    sounds: Option<Box<dyn crate::sound::SoundOutput>>,
    /// Okur's own windows (`okur-ui`); None when headless.
    ui: Option<Box<dyn crate::ui::UiHost>>,
    /// Where changed settings are kept; None when headless.
    settings_store: Option<Box<dyn crate::settings::SettingsStore>>,
    /// The settings in their layers; `settings` is what they give where the focus is.
    settings_layers: crate::settings_layers::SettingsLayers,
    /// The application and the site `settings` were looked up for.
    settings_place: (Option<String>, Option<String>),
    /// The application in front when the settings dialog opened, whose site settings it keeps.
    settings_dialog_application: Option<String>,
    /// The verbosity levels' dialog (Settings Center's Configure Verbosity Levels), while it
    /// is open.
    settings_dialog: Option<crate::ui::settings::SettingsDialog>,
    /// Settings Center or Quick Settings, while it is open.
    settings_tree: Option<crate::ui::settings_tree::SettingsTree>,
    /// Said before the next focus spoken: what one of Okur's dialogs did as it moved the focus
    /// ("Search edit box cleared.").
    said_before_focus: Option<String>,
    /// Basics, while it is open.
    basics_dialog: Option<crate::ui::basics::BasicsDialog>,
    /// The Startup Wizard while it is open, and the session layer as it was before it, given
    /// back when it closes.
    startup_wizard: Option<crate::ui::startup_wizard::StartupWizard>,
    /// The application in front as Okur's window opened, which its Voice Profiles menu offers.
    okur_window_application: Option<String>,
    /// Where the user's start after sign-in is kept (the Startup Wizard's).
    start_options: Option<Box<dyn crate::settings::StartOptions>>,
    startup_wizard_session: crate::settings_layers::Values,
    /// Voice Adjustment, while it is open.
    voice_adjustment: Option<crate::ui::voice_adjustment::VoiceAdjustment>,
    /// Select a Voice Profile, while it is open.
    voice_profile_dialog: Option<crate::ui::voice_profile::VoiceProfileDialog>,
    /// Manage Application Settings, while it is open.
    applications_dialog: Option<crate::ui::applications::ManageApplications>,
    /// Restore to Factory Settings' question, while it is open, by its dialog's id.
    restore_question: Option<u32>,
    /// The application the until-the-focus-moves settings were changed for: the focus moving
    /// to another clears them.
    until_focus_moves_for: Option<String>,
    /// The settings changed lately, newest first, for the dialogs' Last Changed Settings. Kept
    /// while Okur runs.
    settings_history: crate::ui::settings_tree::History,
    /// The list dialog open (the Links List, the Heading List).
    list_dialog: Option<crate::ui::lists::ListDialog>,
    /// The mode the list dialog gives the keys back to: a document's Heading List (Word's)
    /// returns to its application, any other list to the page.
    list_return_mode: AppMode,
    /// The native focus a list dialog was opened from, which closing it gives back.
    list_return_focus: Option<FocusEvent>,
    /// The focused document's headings were asked for its Heading List, which opens when they
    /// come.
    document_headings_asked: bool,
    /// The focused document's links were asked for its Links List, and are awaited.
    document_links_asked: bool,
    /// The focused document's fields were asked for its list of fields, and are awaited.
    document_fields_asked: bool,
    /// The focused document's comments or footnotes were asked for their list, and are awaited.
    document_notes_asked: Option<crate::event::NoteKind>,
    /// The focused document's notes were asked for the Virtual Viewer, and are awaited.
    notes_viewer_asked: Option<crate::event::NoteKind>,
    /// Say Bottom Line of Window asked for Word's status bar, and awaits it.
    status_bar_asked: bool,
    /// The menus ended back on a focus its application says itself: "Leaving menus" waits for
    /// that focus, to be said with it.
    menus_left_unsaid: bool,
    /// Excel's active cell was asked for, to say it, its grid focused without a cell.
    sheet_cell_asked: bool,
    /// One of Excel's lists was asked for, and is awaited.
    sheet_list_asked: Option<crate::event::SheetListKind>,
    /// One of Excel's row or column readers asked for the active cell's row and column, and
    /// awaits them.
    sheet_reading_asked: Option<crate::apps::windows::excel::SheetReading>,
    /// A key for Word's revisions asked whether the document tracks its changes, which it
    /// needs, and awaits the answer.
    revisions_asked: Option<crate::apps::windows::word::RevisionKey>,
    /// Say All waits for Word's document's paragraphs.
    say_all_paragraphs_asked: bool,
    /// Say Window Title in Word asked for the document's view, said with this title when it
    /// comes.
    document_view_title: Option<String>,
    /// Word's view as its view keys last chose it ("Draft View"), for Say Window Title where
    /// Word's status bar marks no view.
    document_view_by_key: Option<String>,
    /// An application module waits for the document's view to say the focus.
    view_asked_by_app: bool,
    /// A drop-down form field of Word's opened its list (Alt+Down): the document's read once
    /// the list closes says the field with its new choice.
    word_list_from_field: bool,
    /// The focused edit combo's own list is open (Word's Font box, Alt+Down): a popup, but the
    /// keys stay the field's.
    combo_list_open: bool,
    /// The property page the focus was last on in the window in front: a control that
    /// reports none (one in an untitled inner dialog of Notepad's Print page) does not
    /// forget it.
    last_dialog_page: Option<String>,
    /// Word shows the document in Read Mode, which Okur reads no better than JAWS: its
    /// screen-sensitive help says how to leave it.
    word_read_mode: bool,
    /// A document's Heading List moved the caret: the next read of the field says its line.
    moved_by_document_list: bool,
    /// What a move by a document's list of comments or footnotes says before "Edit", in place
    /// of the line it reached: nothing for a comment [obs, 1087/s6], the footnote for a
    /// footnote [obs, 1088/s6].
    document_list_says: Option<String>,
    /// The notes the list of a document's comments, footnotes or endnotes shows, by its entries'
    /// nodes.
    listed_notes: Vec<crate::event::DocumentNote>,
    /// The word chosen in the Word List, which W and Shift+W move to.
    word_from_list: Option<String>,
    /// Skim Reading while it is open, and its choices, which Ctrl+Insert+Down reads by.
    skim_dialog: Option<crate::ui::skim::SkimDialog>,
    skim_options: crate::ui::skim::SkimOptions,
    /// A skim started from Skim Reading: its summary, taken as it started, opens as it ends
    /// [obs, 769/s3, 771/s3].
    summary_after_skim: Option<Vec<(usize, String)>>,
    /// What the next skim reading says at its end: an attribute rule's "11 matches found" and
    /// the word it leaves the cursor in.
    skim_found: Option<say_all::SkimFound>,
    /// JAWS Find, while it is open.
    find_dialog: Option<crate::ui::find::FindDialog>,
    /// The Window List while it is open, and whether its windows were asked for and not come.
    window_list: Option<crate::ui::windows::WindowListDialog>,
    window_list_asked: bool,
    /// The system tray list while it is open, and whether its icons were asked for.
    tray_dialog: Option<crate::ui::tray::TrayDialog>,
    tray_asked: bool,
    /// Jump To Line while it is open, and where the last jump was made from (Shift+J).
    jump_dialog: Option<crate::ui::jump::JumpDialog>,
    jump_return: Option<usize>,
    /// Ctrl+R or F5 was pressed in a page: its next load is a new one, announced, not a
    /// capture of the page again [obs, 507/s1].
    reload_asked: bool,
    /// JAWS Find's check boxes and direction, kept between openings.
    find_options: crate::ui::find::FindOptions,
    /// JAWS Find's searches, the latest first, by site ("" for every site).
    find_histories: std::collections::HashMap<String, Vec<String>>,
    /// Until when the focus returning from one of Okur's dialogs is not announced.
    quiet_focus_until: Option<std::time::Instant>,
    /// The field Select a Form Field gave the focus to: its focus is said even while the
    /// focus returns quietly from the dialog, as JAWS says it [obs, 225/s10].
    dialog_focus_target: Option<usize>,
    /// Dialogs opened so far: each gets the next number.
    dialog_count: u32,
    /// The last JAWS-key command, when it was pressed and how many times quickly in a row, to
    /// tell a double or triple press.
    last_press: Option<(Press, u64, u32)>,
    /// A Say All in progress, whose cursor follows the synthesizer.
    say_all: Option<say_all::SayAll>,
    /// The mark the next part of a Say All is given.
    next_mark: u64,
    /// What the quick keys do (Insert+N), and whether they are suspended (Insert+Shift+Z).
    quick_keys: toggles::QuickKeys,
    quick_keys_suspended: bool,
    /// The virtual cursor turned off (Insert+Z): the page's keys are the browser's.
    virtual_cursor_off: bool,
    /// The voice rate's change for now (Ctrl+Alt+PageUp and PageDown), until Insert+Escape.
    rate_change: i32,
    /// Insert+Space was pressed: the next key is the layer's.
    layer_open: bool,
    /// The mouse buttons held down by their locks (Insert+Numpad Slash and Star).
    left_button_locked: bool,
    right_button_locked: bool,
    /// Where drag and drop starts, once Ctrl+Insert+Numpad Slash was pressed there.
    drag_from: Option<(i32, i32)>,
    /// The notifications that came, the latest last, for Insert+Space, Shift+N and the
    /// Notification History.
    notifications: std::collections::VecDeque<crate::ui::notifications::Notification>,
    /// The Notification History while it is open, and the details shown over it.
    notification_history: Option<crate::ui::notifications::HistoryDialog>,
    notification_details: Option<u32>,
    /// The notification rules made with Create Rule, the first that matches deciding, with
    /// the Notification History's Enable Rules, and where they are kept.
    notification_rules: crate::ui::notifications::NotificationRules,
    notification_rule_store: Option<Box<dyn crate::ui::notifications::NotificationRuleStore>>,
    /// Create Rule while it is open.
    notification_rule_dialog: Option<crate::ui::notifications::CreateRuleDialog>,
    /// Name Your Rule while it is open, and the notification the rule was made from.
    notification_name_dialog: Option<crate::ui::notifications::NameRuleDialog>,
    notification_rule_text: Option<String>,
    /// Manage Rules while it is open, Delete's question over it (its dialog and the rule), and
    /// the rule Modify... or Rename... is changing.
    notification_manage: Option<crate::ui::notifications::ManageRulesDialog>,
    notification_confirm: Option<(u32, usize)>,
    notification_editing: Option<usize>,
    /// The last notification that came, for Insert+Space, Shift+N, even one the history
    /// leaves out.
    last_notification: Option<String>,
    /// Toasts are kept but not said (Insert+Space, Ctrl+N).
    toasts_unsaid: bool,
    /// The table layer (Insert+Space, T): the arrows move by cell until Escape.
    table_layer: bool,
    /// The mouse echo layer (Insert+Space, E): the next key is its.
    mouse_echo_layer: bool,
    /// The OCR layer (Insert+Space, O): the next key is its.
    ocr_layer: bool,
    /// A recognition is under way; its words come as [`AccessibilityEvent::Recognized`].
    recognizing: bool,
    /// A file's recognition is under way (Insert+Space, O, F): its text opens in the Results
    /// Viewer when it comes, unless Q cancelled it.
    recognizing_file: bool,
    /// The kind of window F was pressed in, which the focus returns to from Okur's message box.
    file_window_kind: Option<String>,
    /// Background OCR (Insert+Space, O, B): when it last asked for the window's words, and
    /// whether that recognition is still under way.
    background_ocr: Option<(std::time::Instant, bool)>,
    /// The window a recognition was asked for, by its title: its words are filed under it.
    recognition_window: Option<String>,
    /// The Okur cursor's model of the window in front.
    screen_model: Option<crate::screen_model::ScreenModel>,
    /// The Okur cursor was routed to the PC cursor before the window's text arrived: it is
    /// routed again by it when it comes, unless the cursor moved meanwhile.
    route_awaits_screen: bool,
    /// When the mouse last moved, until mouse echo reads what it rests on.
    mouse_moved_at: Option<std::time::Instant>,
    /// Until when the mouse's moves are Okur's own, not echoed.
    own_mouse_move_until: Option<std::time::Instant>,
    /// What mouse echo said last, not said again while the mouse stays in it.
    last_mouse_echo: Option<String>,
    /// The page element Numpad Slash clicked at the virtual cursor: its focus is said as
    /// Enter's is, with the regions it is in [obs, 534/s2].
    clicked: Option<usize>,
    /// The radio button that Space or Numpad Slash checked while another of its group was
    /// checked: its focus is said without its position [obs, 205/s6, 534/s9].
    radio_switched: Option<usize>,
    /// The named groups the virtual cursor entered as an element list's Enter moved it to
    /// a control: said before the page as the focus comes back [obs, 260/s3, 246/s3].
    list_entered_groups: Vec<String>,
    /// Full Speech or less speech for this session, as Insert+Space, S set it; else the
    /// settings'.
    speech_mode_session: Option<crate::settings::SpeechMode>,
    /// Less speech's kind for this session, as Insert+Space, Shift+S set it.
    less_speech_session: Option<crate::settings::LessSpeech>,
    /// User settings (JAWS defaults unless configured).
    pub settings: crate::settings::Settings,
    /// Caps Lock's state as toggled through the JAWS key, when the platform does not report it.
    caps_lock_on: bool,
    /// The Virtual Viewer, while it is open (it takes the keys).
    viewer: Option<crate::viewer::VirtualViewer>,
    /// A Virtual Viewer waiting for what it will show, read off the keyboard hook.
    pending_viewer: Option<PendingViewer>,
    /// Say Font waiting for the page elements that show the font (`true`: pressed twice,
    /// shown in the Virtual Viewer).
    pending_font: Option<bool>,
    /// Say Font in Word's document waiting for the line spacing its object model gives.
    pending_word_font: bool,
    /// Behavior for particular applications, offered every focus first ([`crate::apps`]).
    apps: Vec<Box<dyn crate::apps::AppModule>>,
    /// Each live region's lines when it last changed, by DOM id.
    last_live_text: std::collections::HashMap<String, Vec<String>>,
    /// Live regions being read as they grow, by DOM id (`LiveRegionAppend::Appended`).
    live_streams: std::collections::HashMap<String, page::LiveStream>,
    /// The question before Okur exits, while it is open (Insert+F4).
    quit_dialog: Option<crate::ui::quit::QuitDialog>,
    /// Okur's message box while it is open (Convenient OCR's refusals).
    message_box: Option<crate::ui::message_box::MessageBox>,
    /// The selection was added to the clipboard (Okur key+Windows+C): Ctrl+C and Ctrl+X ask
    /// before overwriting it, until a copy or a cut does.
    clipboard_appended: bool,
    /// The focus is on a ribbon's menus, its tabs or commands, which JAWS reads as menus.
    in_ribbon_menu: bool,
    /// The rows a console showed when last read, which its next writing is compared with.
    screen_rows: Option<Vec<String>>,
    /// The question asked before overwriting the clipboard, while it is open.
    overwrite_dialog: Option<crate::ui::clipboard::OverwriteDialog>,
    /// How the engine asks its host to end Okur; None when headless.
    exit: Option<quit::ExitRequest>,
}

impl ScreenReaderEngine {
    pub fn new(cursor: VirtualCursor, speech: Box<dyn SpeechOutput>) -> Self {
        Self {
            cursor,
            speech,
            forms_mode: false,
            app_mode: AppMode::VirtualBuffer,
            last_window_title: None,
            last_window_kind: None,
            native_focus: crate::native_focus::NativeFocus::default(),
            last_page_focus: None,
            focus_at_key: None,
            routed: false,
            select_opened: false,
            focus_table_unsaid: false,
            tab_start: None,
            activated_item: None,
            cursor_to_focus: false,
            page_stale: false,
            option_list: None,
            option_said: None,
            arrow_focus: None,
            cursor_moved_since_load: false,
            keyboard_help: false,
            field_left: None,
            field_texts: std::collections::HashMap::new(),
            web_application: None,
            page_focus_is_field: false,
            last_native_focus: None,
            wizard_page: None,
            last_selected: None,
            combo_items: Vec::new(),
            last_typed: None,
            typed_word: String::new(),
            edit_read_in_console: false,
            last_key_selects: false,
            last_key_tabs: false,
            last_key_escapes: false,
            last_key_letter: false,
            last_key: None,
            temp_placemarkers: std::collections::HashMap::new(),
            placemarkers: crate::placemarkers::PlaceMarkers::default(),
            placemarker_store: None,
            placemarker_list: None,
            placemarker_list_seen: None,
            flexible_web: None,
            page_customizations: flexible_web::PageCustomizations::default(),
            flexible_web_rules: crate::flexible_web::Rules::default(),
            flexible_web_store: None,
            flexible_web_leaving: None,
            flexible_web_rules_dialog: None,
            custom_labels: crate::labels::CustomLabels::default(),
            label_store: None,
            label_dialog: None,
            alt_held: false,
            pass_next_key: false,
            menus: crate::native_focus::Menus::default(),
            pending_edit: None,
            pending_file_edge: None,
            forms_mode_by_enter: false,
            focus_access_key: None,
            focus_app_version: None,
            focus_bounds: None,
            mouse: None,
            active_cursor: cursors::ActiveCursor::Pc,
            okur_place: None,
            okur_restriction: cursors::Restriction::default(),
            forms_mode_by_list: false,
            page_said_for: None,
            pending_page: None,
            page_builds: page::PageBuilds::default(),
            last_text: None,
            actions: None,
            clock: None,
            sounds: None,
            ui: None,
            settings_store: None,
            settings_layers: crate::settings_layers::SettingsLayers::default(),
            settings_place: (None, None),
            settings_dialog_application: None,
            settings_dialog: None,
            settings_tree: None,
            said_before_focus: None,
            basics_dialog: None,
            startup_wizard: None,
            okur_window_application: None,
            start_options: None,
            startup_wizard_session: crate::settings_layers::Values::new(),
            voice_adjustment: None,
            voice_profile_dialog: None,
            applications_dialog: None,
            restore_question: None,
            until_focus_moves_for: None,
            settings_history: Vec::new(),
            list_dialog: None,
            list_return_mode: AppMode::VirtualBuffer,
            list_return_focus: None,
            document_headings_asked: false,
            document_links_asked: false,
            document_fields_asked: false,
            document_notes_asked: None,
            notes_viewer_asked: None,
            revisions_asked: None,
            status_bar_asked: false,
            menus_left_unsaid: false,
            sheet_reading_asked: None,
            sheet_list_asked: None,
            sheet_cell_asked: false,
            say_all_paragraphs_asked: false,
            document_view_title: None,
            document_view_by_key: None,
            view_asked_by_app: false,
            word_list_from_field: false,
            combo_list_open: false,
            last_dialog_page: None,
            word_read_mode: false,
            moved_by_document_list: false,
            document_list_says: None,
            listed_notes: Vec::new(),
            word_from_list: None,
            skim_dialog: None,
            skim_options: crate::ui::skim::SkimOptions::default(),
            summary_after_skim: None,
            skim_found: None,
            find_dialog: None,
            window_list: None,
            window_list_asked: false,
            tray_dialog: None,
            tray_asked: false,
            jump_dialog: None,
            jump_return: None,
            reload_asked: false,
            find_options: crate::ui::find::FindOptions::default(),
            find_histories: std::collections::HashMap::new(),
            quiet_focus_until: None,
            dialog_focus_target: None,
            dialog_count: 0,
            last_press: None,
            say_all: None,
            next_mark: 0,
            quick_keys: toggles::QuickKeys::On,
            quick_keys_suspended: false,
            virtual_cursor_off: false,
            rate_change: 0,
            layer_open: false,
            left_button_locked: false,
            right_button_locked: false,
            drag_from: None,
            notifications: std::collections::VecDeque::new(),
            notification_history: None,
            notification_details: None,
            notification_rules: crate::ui::notifications::NotificationRules::default(),
            notification_rule_store: None,
            notification_rule_dialog: None,
            notification_name_dialog: None,
            notification_rule_text: None,
            notification_manage: None,
            notification_confirm: None,
            notification_editing: None,
            last_notification: None,
            toasts_unsaid: false,
            table_layer: false,
            mouse_echo_layer: false,
            ocr_layer: false,
            recognizing: false,
            recognizing_file: false,
            file_window_kind: None,
            background_ocr: None,
            recognition_window: None,
            screen_model: None,
            route_awaits_screen: false,
            mouse_moved_at: None,
            own_mouse_move_until: None,
            last_mouse_echo: None,
            clicked: None,
            radio_switched: None,
            list_entered_groups: Vec::new(),
            speech_mode_session: None,
            less_speech_session: None,
            settings: crate::settings::Settings::default(),
            caps_lock_on: false,
            viewer: None,
            pending_viewer: None,
            pending_font: None,
            pending_word_font: false,
            apps: crate::apps::builtin(),
            last_live_text: std::collections::HashMap::new(),
            live_streams: std::collections::HashMap::new(),
            quit_dialog: None,
            message_box: None,
            clipboard_appended: false,
            screen_rows: None,
            in_ribbon_menu: false,
            overwrite_dialog: None,
            exit: None,
        }
    }

    /// Replaces the active document buffer and resets the cursor.
    pub fn load_buffer(&mut self, buffer: VirtualBuffer) {
        self.forget_customizations();
        self.keep_original(&buffer);
        let buffer = self.prepared(buffer);
        crate::retire::retire(std::mem::replace(
            &mut self.cursor,
            VirtualCursor::new(buffer),
        ));
        self.cursor.move_to_start();
        self.cursor.verbosity = self.settings.verbosity.clone();
        self.forms_mode = false;
        self.app_mode = AppMode::VirtualBuffer;
        self.say_all = None;
        self.placemarker_list_seen = None;
    }

    /// A page as Okur reads it: with the custom labels given on its site, the user's reading
    /// choices, and what Flexible Web hides on it.
    pub(super) fn prepared(&self, buffer: VirtualBuffer) -> VirtualBuffer {
        self.with_customizations(self.with_reading_choices(self.labeled(buffer)))
    }

    /// The page as the user's reading choices read it ([`Self::chosen_page`]), marked on the
    /// page so that every buffer built from it keeps them.
    fn with_reading_choices(&self, buffer: VirtualBuffer) -> VirtualBuffer {
        let Some(page) = buffer.page().and_then(|page| self.chosen_page(page)) else {
            return buffer;
        };
        crate::retire::retire(buffer);
        VirtualBuffer::from_aom_owned(page)
    }

    /// The marks the reading choices put on a page: same page links read as plain links,
    /// graphics left out [obs, 724, 727], how a heading is said. A host that builds pages off
    /// the engine marks them first ([`ReadingMarks::mark`]): a page captured again came
    /// without them, and was copied and built again under the engine for every refresh
    /// (performance review 2026-10-23, C2).
    pub fn reading_marks(&self) -> ReadingMarks {
        use crate::settings::HeadingIndication;
        let on = |on: bool| on.then(|| "true".to_owned());
        ReadingMarks([
            (
                crate::linearize::PLAIN_SAME_PAGE_LINKS,
                on(!self.settings.identify_same_page_links),
            ),
            (
                crate::linearize::NO_GRAPHICS,
                on(!self.settings.show_graphics),
            ),
            (
                crate::linearize::HEADING_INDICATION,
                match self.settings.heading_indication {
                    HeadingIndication::Level => None,
                    other => serde_json::to_value(other)
                        .ok()
                        .and_then(|v| v.as_str().map(str::to_owned)),
                },
            ),
        ])
    }

    /// `page` marked as the reading choices ask, when its marks differ from them.
    fn chosen_page(&self, page: &crate::aom::AomNode) -> Option<crate::aom::AomNode> {
        let marks = self.reading_marks();
        if marks.on(page) {
            return None;
        }
        let mut marked = page.clone();
        marks.mark(&mut marked);
        Some(marked)
    }

    /// The page in front built again with the reading choices in force, the cursor on the
    /// element it was on.
    fn reread_with_choices(&mut self) {
        let Some(page) = self
            .cursor
            .buffer
            .page()
            .and_then(|page| self.chosen_page(page))
        else {
            return;
        };
        let at = self.cursor.current_node_id;
        let offset = self.cursor.offset;
        crate::retire::retire(std::mem::replace(
            &mut self.cursor.buffer,
            VirtualBuffer::from_aom_owned(page),
        ));
        self.say_all = None;
        match at.and_then(|id| self.cursor.buffer.start_of(id)) {
            Some(start) => self.cursor.move_to(start),
            None => self
                .cursor
                .move_to(offset.min(self.cursor.buffer.text.len())),
        }
    }

    /// The application module that owns `focus`, whose hooks apply to it.
    fn app_for(&self, focus: &FocusEvent) -> Option<&dyn crate::apps::AppModule> {
        self.apps
            .iter()
            .find(|app| app.owns(focus))
            .map(|app| app.as_ref())
    }

    /// The application module that owns the page in front, by the page's address.
    fn page_app(&self) -> Option<&dyn crate::apps::AppModule> {
        let url = self.cursor.buffer.document_url()?;
        self.apps
            .iter()
            .find(|app| app.owns_page(url))
            .map(|app| app.as_ref())
    }

    /// Handles a live keyboard event from the low-level hook.
    /// Intercepts navigation keys, manages forms mode transitions, silences ongoing speech,
    /// and returns whether the key was consumed or should pass through to the OS.
    pub fn handle_key_event(&mut self, event: &KeyEvent) -> KeyAction {
        self.sync_speech_mode();
        let offset = self.cursor.offset;
        let action = self.handle_key(event);
        if self.app_mode == AppMode::VirtualBuffer && self.cursor.offset != offset {
            self.cursor_moved_since_load = true;
        }
        self.refresh_stale_page();
        action
    }

    fn handle_key(&mut self, event: &KeyEvent) -> KeyAction {
        if event.key == Key::Alt {
            self.alt_held = event.is_down;
        }
        if !event.is_down {
            return KeyAction::PassThrough;
        }
        // Which held keys act as the JAWS key depends on the keyboard settings.
        // Pass Key Through: the next key goes to the application as it is, with the modifiers
        // held for it.
        if self.pass_next_key
            && !matches!(
                event.key,
                Key::Control | Key::Alt | Key::Shift | Key::Windows | Key::Insert | Key::CapsLock
            )
        {
            self.pass_next_key = false;
            return KeyAction::PassThrough;
        }
        let resolved = self.settings.keyboard.resolve(event);
        let event = &resolved;
        // Keyboard Help: a key is described, not run [obs, 509, 634].
        if self.keyboard_help {
            if let Some(action) = self.describe_key(event) {
                return action;
            }
        }
        if self.app_mode == AppMode::VirtualBuffer {
            let m = &event.modifiers;
            let plain = !m.alt && !m.shift && !m.insert && !m.win;
            if plain
                && ((m.ctrl && event.key == Key::Alpha('R')) || (!m.ctrl && event.key == Key::F(5)))
            {
                self.reload_asked = true;
            }
        }
        self.last_typed = event.text;
        self.edit_read_in_console = false;
        self.last_key_tabs = event.key == Key::Tab && !event.modifiers.ctrl && !event.modifiers.alt;
        self.last_key_escapes =
            event.key == Key::Escape && event.modifiers == crate::input::Modifiers::none();
        self.last_key = Some((event.key, event.modifiers));
        self.last_key_letter = matches!(event.key, Key::Alpha(_))
            && event.modifiers == crate::input::Modifiers::none();
        self.last_key_selects = event.modifiers.shift
            && !event.modifiers.alt
            && matches!(
                event.key,
                Key::UpArrow
                    | Key::DownArrow
                    | Key::LeftArrow
                    | Key::RightArrow
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown
            );
        // The hook waits for this: a slow key leaks to the application (Space on an edit held
        // 758 ms and toggled the focused check box, old-pc 2026-10-18), so its stages are
        // timed.
        let started = std::time::Instant::now();
        self.apply_pending_page();
        let page_applied = started.elapsed();

        // A key ends the quiet return from Okur's dialogs: what it does is said.
        self.quiet_focus_until = None;
        self.dialog_focus_target = None;
        self.page_said_for = None;
        self.focus_at_key = self.last_page_focus;
        self.routed = false;
        self.select_opened = false;
        self.combo_items.clear();

        // A Say All key steers the reading instead of ending it.
        if let Some(action) = self.say_all_key(event) {
            return action;
        }

        // With Reading Interrupt off, a key bound to a command reads on through a Say All: one
        // that only reports is answered after what is being said, any other changes nothing
        // [obs, 982, 983].
        if self.say_all.is_some() && !self.settings.reading_interrupt && event.is_down {
            if let Some(command) = crate::command::command_for(event) {
                if command.only_reports() {
                    let speech = std::mem::replace(
                        &mut self.speech,
                        Box::new(crate::speech::MockSpeechOutput::new()),
                    );
                    let said = self.execute(command);
                    self.speech = speech;
                    if let Some(said) = said.filter(|s| !s.trim().is_empty()) {
                        self.speech.speak(&said, false);
                    }
                }
                return KeyAction::Consume;
            }
        }

        // 1. Any key press interrupts speech immediately (NVDA & JAWS standard behavior)
        self.speech.stop();
        // It ends a Say All, whose cursor stays on the line last heard.
        self.end_say_all();
        let stopped = started.elapsed();
        if stopped > std::time::Duration::from_millis(50) {
            tracing::warn!(
                key = ?event.key,
                page_ms = page_applied.as_millis(),
                stop_ms = (stopped - page_applied).as_millis(),
                "slow start of a key"
            );
        }

        // The key after Insert+Space is the layer's.
        if let Some(action) = self.layer_key(event) {
            return action;
        }
        if let Some(action) = self.mouse_echo_layer_key(event) {
            return action;
        }
        if let Some(action) = self.ocr_layer_key(event) {
            return action;
        }
        if let Some(action) = self.table_layer_key(event) {
            return action;
        }

        // 2. Standalone Control tap is speech silence key
        if event.key == Key::Control && event.modifiers.is_empty() {
            return KeyAction::PassThrough;
        }
        // The sentence and paragraph keys and the hot key in an application or a page's field,
        // where the keys would otherwise go to it [obs, 504/s14, s15; 537/s1].
        if self.app_mode != AppMode::VirtualBuffer || self.forms_mode {
            use crate::command::Command;
            if let Some(
                command @ (Command::SaySentence
                | Command::SayNextSentence
                | Command::SayPriorSentence
                | Command::SayParagraph
                | Command::SayHotKey),
            ) = crate::command::command_for(event)
            {
                let _ = self.execute(command);
                return KeyAction::Consume;
            }
        }
        // The cursors, in any application [obs, 516]. Numpad Plus is the PC cursor from the
        // JAWS cursor or in an application; in a page it is the virtual PC cursor's key.
        {
            use crate::command::Command;
            use cursors::ActiveCursor;
            if let Some(
                command @ (Command::OkurCursor
                | Command::PcCursor
                | Command::RouteOkurToPc
                | Command::RoutePcToOkur
                | Command::RestrictOkurCursor
                | Command::SayActiveCursor
                | Command::LeftMouseClick
                | Command::RightMouseClick
                | Command::LeftMouseButtonLock
                | Command::RightMouseButtonLock
                | Command::ModifiedLeftMouseClick(_)
                | Command::MoveMouse(_)
                | Command::DragAndDrop),
            ) = crate::command::command_for(event)
            {
                let in_page = self.app_mode == AppMode::VirtualBuffer;
                if command != Command::PcCursor
                    || self.active_cursor == ActiveCursor::Okur
                    || !in_page
                {
                    let _ = self.execute(command);
                    return KeyAction::Consume;
                }
            }
            // With the JAWS cursor active the arrows move it, not the focus or the virtual
            // cursor: they stay out of the application [obs, 516/s5], and read where it goes
            // [obs, 690, 752, 753].
            if self.active_cursor == ActiveCursor::Okur {
                if self.okur_cursor_key(event).is_some() {
                    return KeyAction::Consume;
                }
                if self.app_mode != AppMode::VirtualBuffer
                    && event.modifiers.is_empty()
                    && matches!(
                        event.key,
                        Key::UpArrow | Key::DownArrow | Key::LeftArrow | Key::RightArrow
                    )
                {
                    return KeyAction::Consume;
                }
            }
        }
        // The voice rate for now, in any application [obs, 512].
        if let Some(
            command @ (crate::command::Command::RateFaster | crate::command::Command::RateSlower),
        ) = crate::command::command_for(event)
        {
            let _ = self.execute(command);
            return KeyAction::Consume;
        }

        // The JAWS key (Insert, or CapsLock in the laptop layout) and every combination with
        // it are JAWS commands: they never reach the application, bound or not, in any mode.
        if self.settings.keyboard.is_okur_key(event.key) {
            if event.key == Key::CapsLock && !event.is_repeat {
                return self.caps_lock_press(event);
            }
            return KeyAction::Consume;
        }
        // The Virtual Viewer takes the keys while it is open.
        if let Some(action) = self.viewer.as_mut().map(|v| v.key(event)) {
            use crate::viewer::ViewerAction;
            match action {
                ViewerAction::Speak(speech) => {
                    if let Some(speech) = speech {
                        self.speech.speak(&speech, true);
                        self.mark_viewer_window();
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Character(text) => {
                    if !text.is_empty() {
                        self.speech.speak_character(&text, true);
                    }
                    self.mark_viewer_window();
                    return KeyAction::Consume;
                }
                // A word as the word keys say one, whatever the punctuation level: a lone
                // symbol as that character, "period.", the others with their symbols named
                // [obs, 1063/s7, s10].
                ViewerAction::Word(text) => {
                    let mut chars = text.chars();
                    match (chars.next(), chars.next()) {
                        (None, _) => {}
                        (Some(c), None) if !c.is_alphanumeric() => {
                            self.speech.speak_character(&text, true)
                        }
                        _ => self.speech.speak_symbols(&text, true),
                    }
                    self.mark_viewer_window();
                    return KeyAction::Consume;
                }
                ViewerAction::Copy(text) => {
                    if let Some(actions) = &self.actions {
                        actions.copy_text(&text);
                    }
                    // [obs, 311/s11], worded for the level as the other copies are.
                    if let Some(copied) = self.copied_message() {
                        self.speech.speak(&copied, true);
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Close => {
                    let own_window = self.viewer.as_ref().is_some_and(|v| v.own_window());
                    self.viewer = None;
                    self.close_viewer_window();
                    if own_window {
                        // The application's window, as the focus comes back to it, is
                        // announced again: "sr-notepad-fixture.txt - Notepad | edit | Type in
                        // text." [obs, 825/s7].
                        self.last_window_title = None;
                        self.native_focus.reset();
                    } else {
                        self.speak_focus_again();
                    }
                    return KeyAction::Consume;
                }
                ViewerAction::Run(command) => {
                    self.viewer = None;
                    self.close_viewer_window();
                    let _ = self.execute(command);
                    return KeyAction::Consume;
                }
                ViewerAction::MoveTo(offset) => {
                    self.viewer = None;
                    self.close_viewer_window();
                    self.move_to_summary_place(offset);
                    return KeyAction::Consume;
                }
                ViewerAction::Ignore => {}
            }
        }
        if event.modifiers.insert {
            self.okur_key_command(event);
            return KeyAction::Consume;
        }
        // Numpad 5 is Say Character wherever the focus is, in an application's text field
        // too: JAWS keeps it from the application [obs, 401/s1-s3].
        if event.key == Key::Numpad5 && event.modifiers.is_empty() {
            self.execute_pressed(crate::command::Command::SayCharacter, event.time_ms);
            return KeyAction::Consume;
        }

        // Who takes the keys, after the JAWS keys and the Virtual Viewer: an open menu, then
        // forms mode (a page's field or application region), then the focused control outside
        // a document, and last the virtual PC cursor in a page. JAWS reads a page with the
        // virtual cursor, and anything outside it (the browser's address bar, its menus) with
        // the PC cursor.
        //
        // A menu takes every key, Escape too, which closes it, whatever cursor was active
        // before it: a page's ARIA menu (Google Docs', in forms mode), a WinUI context menu,
        // or Chrome's app menu over a page, which opens with no item focused: the Down that
        // focuses its first item went to the virtual cursor, and the menu never had a focus
        // (dev box, 2026-10-17).
        // Without Auto Forms Mode a page's menu leaves the arrows to the virtual cursor, as
        // JAWS did on old-pc (218: Tab onto an ARIA menu bar, then Down read the bar's next
        // items as lines).
        // Alt and a letter outside a page may open a menu at once [obs, 541/s1, 544/s1].
        if self.app_mode != AppMode::VirtualBuffer {
            let m = event.modifiers;
            self.menus.access_key(
                m.alt && !m.ctrl && !m.shift && !m.insert && matches!(event.key, Key::Alpha(_)),
            );
        }
        // A page's menu keeps its keys in forms mode, which Auto Forms Mode turns on in it and
        // Semi-Auto when Tab brought the focus there.
        let page_menu_in_browse = self.menus.is_page()
            && match self.settings.forms_mode {
                crate::settings::FormsMode::Auto => false,
                crate::settings::FormsMode::SemiAuto => !self.forms_mode,
                crate::settings::FormsMode::Manual => true,
            };
        if self.menus.in_menu_mode() && !page_menu_in_browse {
            return KeyAction::PassThrough;
        }

        // Enter on a list or grid item in forms mode may open something (a chat from its
        // list): the next focus elsewhere is then the page's, not the user's.
        self.activated_item = (self.forms_mode && event.key == Key::Enter)
            .then_some(self.last_page_focus)
            .flatten()
            .filter(|&id| self.in_widget_item(id));

        // 3. Handle Forms Mode
        if self.forms_mode {
            // In a page's application region every key is the page's (JAWS on the dev box:
            // Escape closes VS Code's command palette).
            if self.web_application.is_some() {
                // Typing and caret keys are read in its text fields only: on its buttons and
                // lists the page answers keys itself, and JAWS says nothing for them.
                if self.page_focus_is_field {
                    self.edit_key_feedback(event);
                }
                return KeyAction::PassThrough;
            }
            if let Some(action) = self.arrow_out_of_field(event) {
                return action;
            }
            // Numpad Plus leaves forms mode as Escape does (JAWS 2025 Settings Center help, Forms
            // Mode Options; Surf's Up, Forms).
            let numpad_plus = event.key == Key::NumpadPlus && event.modifiers.is_empty();
            if event.key == Key::Escape || numpad_plus {
                self.forms_mode = false;
                let _ = self.cursor.exit_forms_mode();
                // [obs, 205/s11]
                let said = self.virtual_pc_message();
                self.speech.speak(&said, true);
                return KeyAction::Consume;
            }
            // In forms mode, pass all typing and navigation through to the active control;
            // caret movement keys then read the text they moved over, typed keys are echoed.
            // A select (a combo box one does not type in) answers the arrows itself, and its
            // value change says the new choice; reading its text would answer first, with
            // nothing [obs, 205/s9, s10].
            if !self.page_focus_is_select() {
                self.edit_key_feedback(event);
            }
            return KeyAction::PassThrough;
        }

        // 4. Handle Escape when inside an expanded combobox or modal dialog
        if event.key == Key::Escape && event.modifiers.is_empty() {
            if self.cursor.is_combobox_expanded() {
                if let Some(msg) = self.cursor.collapse_combobox() {
                    self.speech.speak(&msg, true);
                }
                return KeyAction::Consume;
            }
            if self.cursor.is_in_modal() {
                if let Some(msg) = self.cursor.exit_modal() {
                    self.speech.speak(&msg, true);
                }
                return KeyAction::Consume;
            }
        }

        // 5. In non-VirtualBuffer modes (Desktop, Shell, native controls),
        // navigation and typing keys MUST pass through to the OS so Windows
        // handles arrow navigation, Tab cycling, and initial-letter jumps natively!
        if self.app_mode != AppMode::VirtualBuffer {
            if self.ask_before_overwriting(event) {
                return KeyAction::Consume;
            }
            // Ctrl+Home and Ctrl+End in an application's grid are its top and bottom of file
            // [obs, 545/s6, s7].
            let file_key = event.modifiers.ctrl
                && !event.modifiers.alt
                && !event.modifiers.shift
                && matches!(event.key, Key::Home | Key::End);
            self.pending_file_edge = (file_key
                && self
                    .last_native_focus
                    .as_ref()
                    .is_some_and(|f| f.role == crate::aom::Role::TableCell))
            .then_some(event.key == Key::Home);
            if let Some(action) = self.document_table_key(event) {
                return action;
            }
            // Word's Ctrl+Shift+Apostrophe: its comments listed [obs, 1082/s1]; the key is the
            // application's elsewhere.
            let m = event.modifiers;
            // Excel's Ctrl+Shift+S in a sheet: its worksheets listed, JAWS's Move to Worksheet
            // [obs, 521/s13]; the key is the application's elsewhere.
            if event.key == Key::Alpha('S')
                && m.ctrl
                && m.shift
                && !m.alt
                && !m.insert
                && !m.win
                && self.open_sheet_list(crate::event::SheetListKind::Worksheets)
            {
                return KeyAction::Consume;
            }
            // Windows+Semicolon in Word's document: Items to Display in Virtual Viewer [obs,
            // 1094/s1]; elsewhere the key is Windows'.
            if event.key == Key::Semicolon
                && m.win
                && !m.ctrl
                && !m.alt
                && !m.shift
                && !m.insert
                && self.in_word_document()
            {
                let dialog = crate::ui::lists::ListDialog::notes_to_view(self.dialog_count + 1);
                let mode = self.app_mode;
                self.show_list(dialog, mode);
                return KeyAction::Consume;
            }
            if event.key == Key::Apostrophe
                && m.ctrl
                && m.shift
                && !m.alt
                && !m.insert
                && self.open_document_notes(crate::event::NoteKind::Comment)
            {
                return KeyAction::Consume;
            }
            // Word's Alt+Shift+E: the footnote or endnote at the caret [obs, 1092/s1, s7, s14];
            // the key is the application's elsewhere.
            if event.key == Key::Alpha('E') && m.alt && m.shift && !m.ctrl && !m.insert {
                if let Some(said) = self
                    .last_text
                    .as_ref()
                    .filter(|_| self.editing_text())
                    .and_then(crate::apps::windows::word::note_at_caret)
                {
                    self.speech.speak(&said, true);
                    return KeyAction::Consume;
                }
            }
            // Word's Alt+Shift+Apostrophe: the comment at the caret [obs, 1081/s3, s8]; the key
            // is the application's elsewhere.
            if event.key == Key::Apostrophe && m.alt && m.shift && !m.ctrl && !m.insert {
                let context = self.last_text.clone().filter(|_| self.editing_text());
                if let Some(said) = context
                    .as_ref()
                    .and_then(crate::apps::windows::word::comment_at_caret)
                {
                    // Twice quickly, the comment in the Virtual Viewer with the text it is on,
                    // read whole [obs, 1095/s6].
                    let twice = self.pressed_twice(Press::Application(event.key, m), event.time_ms);
                    let viewer = context
                        .as_ref()
                        .and_then(crate::apps::windows::word::comment_viewer)
                        .filter(|_| twice && self.viewer.is_none());
                    let said = match viewer {
                        Some(lines) => {
                            let viewer = crate::viewer::VirtualViewer::message(lines);
                            let speech = viewer.message_speech();
                            self.viewer = Some(viewer);
                            self.show_viewer_window();
                            speech
                        }
                        None => said,
                    };
                    self.speech.speak(&said, true);
                    return KeyAction::Consume;
                }
            }
            // Word's view keys say the view chosen; the key goes on to Word.
            if self.editing_text()
                && self
                    .last_text
                    .as_ref()
                    .is_some_and(|t| t.structure.is_some())
            {
                if let Some(view) = crate::apps::windows::word::view_key(event) {
                    self.speech.speak(&view, true);
                    self.document_view_by_key = Some(view);
                }
                // Word's keys whose result is said: read once Word has acted on them.
                if let (Some(actions), Some(key)) =
                    (&self.actions, crate::apps::windows::word::word_key(event))
                {
                    actions.read_word_key(key);
                }
            }
            // Escape leaves Word's Read Mode [obs, 1059/s8].
            if event.key == Key::Escape {
                self.word_read_mode = false;
            }
            if self.editing_text() {
                self.edit_key_feedback(event);
            } else {
                // A field typing goes into though it is not an edit (Windows Terminal's text
                // area): typed characters are echoed as in any field, and the application's
                // own edit keys are read (its output arrives as notifications, but Escape's
                // cleared line does not).
                let (types_into, own_key) = self
                    .last_native_focus
                    .as_ref()
                    .and_then(|field| {
                        self.app_for(field)
                            .map(|app| (app.types_into(field), app.edit_key(field, event)))
                    })
                    .unwrap_or((false, None));
                if types_into {
                    use crate::editing::EditKey;
                    // Left, Right and Backspace read as in a field (JAWS: "c", "b"; the
                    // deleted "b"); Up and Down recall history, which the application
                    // announces itself. Ctrl+Left and Right are left out: the word read at
                    // the caret came out "blank" where JAWS said "ac" (dev box, 2026-10-17).
                    let caret_key = EditKey::of(event).filter(|k| {
                        matches!(
                            k,
                            EditKey::Character | EditKey::DeleteBackward | EditKey::DeleteForward
                        )
                    });
                    if let Some(key) = own_key.or(caret_key) {
                        self.expect_text_change(key);
                    } else {
                        if let Some(c) = event.text {
                            self.echo_typed(c);
                        }
                        // The field's known text kept current with no speech, so that the
                        // next key read finds its own change (Escape after Ctrl+Left read the
                        // line before it was cleared).
                        if let Some(actions) = &self.actions {
                            actions.read_at_caret(self.last_text.as_ref(), false);
                        }
                    }
                }
            }
            return KeyAction::PassThrough;
        }

        // 6. Tab and Shift+Tab belong to the page: the browser moves focus, and the focus
        //    event moves the virtual cursor and speaks [obs, 204]. Away from the focus, the
        //    virtual cursor is where the browser moves on from, as with JAWS (dev box).
        if event.key == Key::Tab && !event.modifiers.ctrl && !event.modifiers.alt {
            if self.tab_from_virtual_cursor(event.modifiers.shift) {
                return KeyAction::Consume;
            }
            return KeyAction::PassThrough;
        }

        // With the virtual cursor off the page's keys are the browser's, and so are the quick
        // keys while they are off [obs, 298].
        if self.virtual_cursor_off || self.quick_key_passes(event) {
            return KeyAction::PassThrough;
        }
        // 7. In Virtual PC Cursor mode, check if keystroke translates to a command
        if let Some(command) = crate::command::command_for(event) {
            let started = std::time::Instant::now();
            let res = self.execute(command.clone());
            if started.elapsed() > std::time::Duration::from_millis(50) {
                tracing::warn!(?command, ms = started.elapsed().as_millis(), "slow command");
            }
            if res.is_some() {
                return KeyAction::Consume;
            }
            // Ctrl+C without a selection made with the virtual cursor is the browser's copy.
            if command == crate::command::Command::CopySelection {
                return KeyAction::PassThrough;
            }
            // For navigation keys, consume even if boundary reached so host window doesn't scroll
            if matches!(
                event.key,
                Key::DownArrow
                    | Key::UpArrow
                    | Key::LeftArrow
                    | Key::RightArrow
                    | Key::Home
                    | Key::End
                    | Key::PageUp
                    | Key::PageDown
                    | Key::Tab
                    | Key::Alpha(_)
                    | Key::Digit(_)
            ) {
                return KeyAction::Consume;
            }
        }

        KeyAction::PassThrough
    }

    /// Tab (or Shift+Tab) with the virtual cursor outside the element the browser would start
    /// from: the page's next (or prior) tab stop after the cursor's line is focused, as NVDA's
    /// browse mode does. JAWS on the dev box (2026-10-16): with focus on a button and the
    /// virtual cursor moved past a link to a group's end, Tab went to the field after the
    /// group. A browser starts Tab from its focus whatever the page's caret, so the key alone
    /// cannot do it. In a dialog, or past the page's last stop, the key goes to the browser.
    fn tab_from_virtual_cursor(&mut self, backwards: bool) -> bool {
        if self.cursor.is_in_modal() {
            return false;
        }
        let buffer = &self.cursor.buffer;
        let Some(line) = buffer.line_at_offset(self.cursor.offset) else {
            return false;
        };
        let within_start = self
            .tab_start
            .and_then(|id| buffer.node(id))
            .is_some_and(|start| start.find_by_id(line.node_id).is_some());
        if within_start {
            return false;
        }
        // The controls that can take the focus (a capture can say one cannot: WhatsApp's
        // message times are buttons nothing focuses), and the other elements that can
        // (WhatsApp's messages), as JAWS goes to them.
        // Both are in document order (a buffer invariant the corpus checks), so the nearest
        // one each way is found from the cursor's line on, not by walking every stop: that
        // took 35-48 ms on a page with 5,000 of them, in the keyboard hook (responsiveness
        // review, 2026-10-19). A control and another element at one place: the other element
        // going back, the control going forward, as before.
        let focusable = |start: usize| {
            let node = buffer.node(buffer.line_at_offset(start)?.node_id)?;
            (node.states.focusable != Some(false)).then_some((start, node.id))
        };
        let from = line.start_offset;
        let (controls, others) = (&buffer.tab_stops, &buffer.focus_stops);
        let stop = if backwards {
            let before = controls.partition_point(|&s| s < from);
            let control = controls
                .get(..before)
                .unwrap_or_default()
                .iter()
                .rev()
                .find_map(|&s| focusable(s));
            let other = others
                .get(..others.partition_point(|&(s, _)| s < from))
                .and_then(<[_]>::last)
                .copied();
            [control, other]
                .into_iter()
                .flatten()
                .max_by_key(|&(s, _)| s)
        } else {
            let after = controls.partition_point(|&s| s <= from);
            let control = controls
                .get(after..)
                .unwrap_or_default()
                .iter()
                .find_map(|&s| focusable(s));
            let other = others
                .get(others.partition_point(|&(s, _)| s <= from)..)
                .and_then(<[_]>::first)
                .copied();
            [control, other]
                .into_iter()
                .flatten()
                .min_by_key(|&(s, _)| s)
        };
        let Some(node) = stop.and_then(|(_, id)| buffer.node(id)) else {
            return false;
        };
        let focused = self
            .actions
            .as_ref()
            .is_some_and(|actions| actions.focus(node));
        tracing::debug!(
            backwards,
            from = line.node_id,
            to = node.id,
            role = ?node.role,
            name = node.name.as_deref().unwrap_or_default(),
            focused,
            "tab from the virtual cursor"
        );
        if !focused {
            return false;
        }
        let (id, is_edit) = (node.id, node.role == crate::aom::Role::Edit);
        self.tab_start = Some(id);
        // Focus already there brings no focus event: the element is said here, as JAWS
        // says it (GitHub's current README tab, dev box).
        if self.last_page_focus == Some(id) {
            if is_edit {
                self.forms_mode = true;
                self.cursor.forms_mode_active = true;
                if self.settings.forms_mode != crate::settings::FormsMode::Manual {
                    self.cursor.move_to_node_id(id);
                    // A field an arrow had left, with Auto Forms Mode: Shift+Tab back onto it
                    // turned forms mode on and JAWS said nothing [obs, 284/s11]; Tab onto it
                    // said it as a focus, with the text typed in it, "Full name Edit Required
                    // | ada" [obs, 284/s8]. One observation each.
                    if backwards {
                        return true;
                    }
                    let mut field = self.cursor.buffer.node(id).cloned();
                    let typed = self
                        .last_text
                        .as_ref()
                        .map(|c| c.text.clone())
                        .or_else(|| self.field_texts.get(&id).cloned());
                    if let (Some(field), Some(typed)) = (field.as_mut(), typed) {
                        field.value = Some(typed);
                    }
                    if let Some(field) = field {
                        let said = crate::speech::SpeechFormatter::control_focus(&field, |_| false)
                            .render();
                        self.speech.speak(&said, true);
                    }
                    return true;
                }
            }
            let from = self
                .cursor
                .buffer
                .line_at_offset(self.cursor.offset)
                .map(|l| l.node_id);
            let entered = from
                .map(|from| self.entered_containers(from, id))
                .unwrap_or_default();
            if let Some(speech) = self.cursor.focus_node(id) {
                self.speech.speak(&format!("{entered}{speech}"), true);
            }
        }
        true
    }

    /// Handles an incoming accessibility event from the OS event listener.
    pub fn handle_accessibility_event(&mut self, event: &AccessibilityEvent) -> Option<String> {
        self.sync_speech_mode();
        if !matches!(
            event,
            AccessibilityEvent::LiveRegion(_)
                | AccessibilityEvent::DocumentLoaded { .. }
                | AccessibilityEvent::PageChanged
                | AccessibilityEvent::PagePartsLoaded(_)
        ) {
            self.apply_pending_page();
        }
        let spoken = match event {
            // Back from one of Okur's dialogs, the focus returns to where it was and is taken
            // in silently, as JAWS returns from its own Links List (dev box, 2026-10-17).
            AccessibilityEvent::Focus(focus)
                if self.quiet_return() && !self.reaches_dialog_focus_target(focus) =>
            {
                let speech = std::mem::replace(
                    &mut self.speech,
                    Box::new(crate::speech::MockSpeechOutput::new()),
                );
                self.handle_focus_event(focus);
                self.speech = speech;
                None
            }
            AccessibilityEvent::Focus(focus) => {
                let spoken = self.handle_focus_event(focus);
                self.forget_other_windows_screen();
                spoken
            }
            AccessibilityEvent::Menu(menu) => self.handle_menu_event(menu.clone()),
            AccessibilityEvent::Change { kind, control } => self.handle_change(*kind, control),
            AccessibilityEvent::TextContext(context) => {
                let shown = crate::apps::windows::word::as_shown(context);
                self.handle_text_context(shown.as_ref().unwrap_or(context))
            }
            AccessibilityEvent::ConsoleText(text) => self.console_text(text),
            AccessibilityEvent::Selected(item) => self.handle_selected(item),
            AccessibilityEvent::LiveRegion(live) => self.handle_live_region_event(live),
            AccessibilityEvent::ViewerText(lines) => self.open_screen_viewer(lines),
            AccessibilityEvent::ScreenText(text) => self.screen_text(text),
            AccessibilityEvent::Recognized(words) => self.recognized(words),
            AccessibilityEvent::FileRecognized(recognition) => self.file_recognized(recognition),
            AccessibilityEvent::FileRecognitionStarted => self.file_recognition_started(),
            AccessibilityEvent::TopWindows(windows) => self.open_window_list(windows),
            AccessibilityEvent::TrayIcons(icons) => self.open_tray_list(icons),
            AccessibilityEvent::PageElements(elements) => self.say_page_font(elements),
            AccessibilityEvent::DocumentHeadings(headings) => self.document_headings(headings),
            AccessibilityEvent::DocumentLinks(links) => self.document_links(links),
            AccessibilityEvent::LineSpacing(spacing) => self.word_font(*spacing),
            AccessibilityEvent::WordKeyState(state) if self.revisions_asked.is_some() => {
                self.revisions_tracked(*state)
            }
            AccessibilityEvent::WordKeyState(state) => {
                let said = crate::apps::windows::word::word_key_said((*state)?)?;
                self.speech.speak(&said, true);
                Some(said)
            }
            AccessibilityEvent::DocumentFields(fields) => self.document_fields(fields),
            AccessibilityEvent::StatusBar(items) => self.word_status_bar(items),
            AccessibilityEvent::SheetList {
                kind,
                column,
                entries,
            } => self.sheet_list(*kind, column, entries),
            AccessibilityEvent::SheetPlace(place) if self.sheet_cell_asked => {
                self.sheet_cell_asked = false;
                let said = crate::apps::windows::excel::blank_or_cell(place.as_ref()?)?;
                self.speech.speak(&said, true);
                Some(said)
            }
            AccessibilityEvent::SheetPlace(place) => {
                let reading = self.sheet_reading_asked.take()?;
                let said = reading.said(place.as_ref()?)?;
                self.speech.speak(&said, true);
                Some(said)
            }
            AccessibilityEvent::DocumentNotes(kind, notes)
                if self.notes_viewer_asked == Some(*kind) =>
            {
                self.notes_viewer(*kind, notes)
            }
            AccessibilityEvent::DocumentNotes(kind, notes) => self.document_notes(*kind, notes),
            AccessibilityEvent::DocumentParagraphs(paragraphs) => {
                self.say_all_by_paragraphs(paragraphs)
            }
            AccessibilityEvent::DocumentView(view)
                if std::mem::take(&mut self.view_asked_by_app) =>
            {
                self.word_read_mode =
                    view.as_deref() == Some(crate::apps::windows::word::READ_MODE);
                let said = self
                    .apps
                    .iter_mut()
                    .find_map(|app| app.document_view(view.as_deref()))?;
                self.speech.speak(&said, true);
                Some(said)
            }
            AccessibilityEvent::DocumentView(view) => {
                let title = self.document_view_title.take()?;
                self.word_read_mode =
                    view.as_deref() == Some(crate::apps::windows::word::READ_MODE);
                // A view the status bar marks, else the one Word's view keys chose.
                let view = match view.as_deref() {
                    Some(button) => crate::apps::windows::word::view_name(button),
                    None => self.document_view_by_key.clone(),
                };
                let said = std::iter::once(format!("{}\n{title}", msg::window_title_is()))
                    .chain(view)
                    .collect::<Vec<_>>()
                    .join("\n");
                self.speech.speak(&said, true);
                Some(said)
            }
            AccessibilityEvent::Ui(message) => self.handle_ui_event(message),
            AccessibilityEvent::PageChanged => {
                self.page_changed();
                None
            }
            AccessibilityEvent::PagePartsLoaded(parts) => {
                self.page_parts_loaded(parts);
                None
            }
            AccessibilityEvent::MouseMoved { x, y } => {
                self.mouse_moved(*x, *y);
                None
            }
            AccessibilityEvent::MousePoint(point) => self.mouse_point(point),
            AccessibilityEvent::Toast { text, sender } => self.toast_shown(text, sender.as_deref()),
            AccessibilityEvent::AppNotification {
                text,
                politeness,
                app,
            } => self.app_notification(text, *politeness, app.as_deref()),
            AccessibilityEvent::WindowRenamed(title) => {
                self.window_renamed(title);
                None
            }
            AccessibilityEvent::WindowShown(shown) => {
                // The focus's own window, read again once shown: its controls for Say Window
                // and the Virtual Viewer.
                if let Some(focus) = self
                    .last_native_focus
                    .as_mut()
                    .filter(|f| f.window_title == shown.window_title)
                {
                    focus.window_items = shown.window_items.clone();
                    if !shown.dialog_lines.is_empty() {
                        focus.dialog_lines = shown.dialog_lines.clone();
                    }
                }
                None
            }
            AccessibilityEvent::DocumentLoaded { title, root } => {
                self.handle_document_loaded(title, VirtualBuffer::from_aom(root))
            }
        };
        self.refresh_stale_page();
        spoken
    }

    /// Connects the engine to the real application, so that commands such as Enter on an
    /// edit act on the page and not only on the buffer.
    pub fn set_actions(&mut self, actions: Box<dyn crate::actions::ApplicationActions>) {
        self.actions = Some(actions);
    }

    /// Gives the engine the platform's sound output (a notification rule's Play Sound).
    pub fn set_sounds(&mut self, sounds: Box<dyn crate::sound::SoundOutput>) {
        self.sounds = Some(sounds);
    }

    /// Gives the engine the system clock and locale formats (Say System Time).
    pub fn set_clock(&mut self, clock: Box<dyn crate::clock::Clock>) {
        self.clock = Some(clock);
    }

    /// Gives the engine its own windows (`okur-ui`).
    pub fn set_ui(&mut self, ui: Box<dyn crate::ui::UiHost>) {
        self.ui = Some(ui);
    }

    /// Gives the engine where changed settings are kept.
    /// Where the user's start after sign-in is kept.
    pub fn set_start_options(&mut self, options: Box<dyn crate::settings::StartOptions>) {
        self.start_options = Some(options);
    }

    /// Last Changed Settings as it was kept the last time Okur ran.
    pub fn set_settings_history(&mut self, saved: &crate::ui::settings_tree::SavedHistory) {
        self.settings_history = saved.history();
    }

    /// Keeps Last Changed Settings for the next run.
    fn save_settings_history(&self) {
        if let Some(store) = &self.settings_store {
            store.save_history(&crate::ui::settings_tree::SavedHistory::of(
                &self.settings_history,
            ));
        }
    }

    pub fn set_settings_store(&mut self, store: Box<dyn crate::settings::SettingsStore>) {
        self.settings_store = Some(store);
    }

    /// Waits until the settings handed to the store are kept: Okur is exiting.
    pub fn flush_settings(&self) {
        if let Some(store) = &self.settings_store {
            store.flush();
        }
    }

    /// Makes `settings` Okur's for all applications, with no layer of their own: `okur run`'s
    /// settings, replay's reference machine.
    pub fn set_settings(&mut self, settings: crate::settings::Settings) {
        self.set_settings_layers(crate::settings_layers::SettingsLayers::with_global(
            &settings,
        ));
    }

    /// Gives the engine its settings in their layers (the user's saved files).
    pub fn set_settings_layers(&mut self, layers: crate::settings_layers::SettingsLayers) {
        self.settings_layers = layers;
        self.resolve_settings();
    }

    /// The settings in force from the layers, for the application and site the focus is in.
    fn resolve_settings(&mut self) {
        let (application, site) = &self.settings_place;
        let settings = self
            .settings_layers
            .resolve(application.as_deref(), site.as_deref());
        let voice_changed = settings.voice != self.settings.voice;
        let choices_changed = settings.identify_same_page_links
            != self.settings.identify_same_page_links
            || settings.show_graphics != self.settings.show_graphics
            || settings.heading_indication != self.settings.heading_indication;
        // The cursor says a page control's focus itself, at the verbosity in force.
        self.cursor.verbosity = settings.verbosity.clone();
        self.settings = settings;
        if voice_changed {
            self.configure_voice();
        }
        if choices_changed {
            self.reread_with_choices();
        }
        self.sync_speech_mode();
    }

    /// The focus is in `application` (as the platform names it), on a page of `site`: its
    /// settings are looked up anew when either changed.
    pub(crate) fn settings_place(&mut self, application: Option<&str>, site: Option<String>) {
        let place = (
            application.map(crate::settings_layers::application_id),
            site,
        );
        if place != self.settings_place {
            tracing::debug!(?place, "settings looked up for another place");
            // Another application than the one the until-the-focus-moves settings were changed
            // for, and not Okur's own windows (its dialogs come and go over the application),
            // ends them.
            let other = place.0.as_deref().is_some_and(|app| {
                app != OKUR_WINDOWS && self.until_focus_moves_for.as_deref() != Some(app)
            });
            if other && !self.settings_layers.until_focus_moves.is_empty() {
                self.settings_layers.until_focus_moves.clear();
                self.until_focus_moves_for = None;
            }
            self.settings_place = place;
            self.resolve_settings();
        }
    }

    /// Makes the settings dialog's changes Okur's, now, in `scope` (all applications, one
    /// application, one site), where they were `before`: the voice included, and kept in the
    /// settings store. Only what the dialog changed is kept: a level set for the session is not
    /// saved with it.
    fn apply_settings(
        &mut self,
        scope: &crate::settings_layers::Scope,
        before: &crate::settings::Settings,
        settings: &crate::settings::Settings,
        temporary: &[(&'static str, crate::ui::settings::Lifetime)],
    ) {
        use crate::settings::registry;
        use crate::ui::settings::Lifetime;
        // A change that is to last a while only goes to the session's layer or the one cleared
        // when the focus moves to another application (Quick Settings' lifetimes), not to
        // `scope`.
        let mut kept = settings.clone();
        for &(id, lifetime) in temporary {
            let (Some(was), Some(now)) = (registry::get(before, id), registry::get(settings, id))
            else {
                continue;
            };
            if was == now {
                continue;
            }
            registry::set(&mut kept, id, was);
            let path: Vec<&str> = id.split('.').collect();
            match lifetime {
                Lifetime::UntilExit => self.settings_layers.set_for_session(&path, now),
                Lifetime::UntilFocusMoves => {
                    self.settings_layers.set_until_focus_moves(&path, now);
                    self.until_focus_moves_for = self.settings_dialog_application.clone();
                }
                Lifetime::Saved => {}
            }
        }
        let settings = &kept;
        if settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            self.close_viewer_window();
        }
        let application = self.settings_dialog_application.clone();
        // The settings changed, newest first, for Last Changed Settings.
        for spec in crate::settings::registry::SETTINGS {
            let id = spec.id;
            if crate::settings::registry::get(before, id)
                != crate::settings::registry::get(settings, id)
            {
                self.settings_history
                    .retain(|(place, changed)| !(place == scope && *changed == id));
                self.settings_history.insert(0, (scope.clone(), id));
            }
        }
        self.settings_history
            .truncate(crate::ui::settings_tree::HISTORY_LENGTH * 4);
        self.settings_layers
            .change(scope, before, settings, application.as_deref());
        if let Some(store) = &self.settings_store {
            store.save(&self.settings_layers);
        }
        self.save_settings_history();
        self.resolve_settings();
    }

    /// Set Punctuation Level (Insert+Shift+2): the next level, said by its name, until Okur
    /// exits. JAWS keeps it in neither its voice profile nor its JCF [obs, 400 exploration,
    /// 2026-10-19], so it goes in the session's layer and the settings store is not written.
    pub(crate) fn next_punctuation_level(&mut self) -> Option<String> {
        let level = self.settings.voice.punctuation.next();
        self.settings_layers.set_for_session(
            &["voice", "punctuation"],
            serde_json::to_value(level).unwrap_or_default(),
        );
        self.resolve_settings();
        let name = level.name();
        self.speech.speak(&name, true);
        Some(name)
    }

    /// Typing Echo (Insert+2): the next echo, for this session only, as Set Punctuation Level.
    pub(crate) fn next_typing_echo(&mut self) -> Option<String> {
        let echo = self.settings.typing_echo.next();
        self.settings_layers.set_for_session(
            &["typing_echo"],
            serde_json::to_value(echo).unwrap_or_default(),
        );
        self.resolve_settings();
        let name = echo.name();
        self.speech.speak(&name, true);
        Some(name)
    }

    /// Screen Echo (Insert+S): the next echo, for this session only, as Typing Echo.
    pub(crate) fn next_screen_echo(&mut self) -> Option<String> {
        let echo = self.settings.screen_echo.next();
        self.settings_layers.set_for_session(
            &["screen_echo"],
            serde_json::to_value(echo).unwrap_or_default(),
        );
        self.resolve_settings();
        let name = echo.name();
        self.speech.speak(&name, true);
        Some(name)
    }

    /// Smart Navigation (Insert+X): on or off, for this session only, as Typing Echo.
    pub(crate) fn toggle_smart_navigation(&mut self) -> Option<String> {
        let navigation = self.settings.smart_navigation.toggled();
        self.settings_layers.set_for_session(
            &["smart_navigation"],
            serde_json::to_value(navigation).unwrap_or_default(),
        );
        self.resolve_settings();
        let name = navigation.name();
        self.speech.speak(&name, true);
        Some(name)
    }

    /// Where the settings dialog can keep its settings, each with the settings in force there,
    /// and the one it opens on: the application in front, as JAWS's Settings Center does, when
    /// the focus is in one; all applications, and the site when the focus is on a page. The
    /// application is remembered: once the dialog has the focus, it is the one in front.
    fn settings_places(&mut self) -> (Vec<crate::ui::settings::Place>, usize) {
        use crate::settings_layers::Scope;
        use crate::ui::settings::Place;
        let (application, site) = self.settings_place.clone();
        self.settings_dialog_application = application.clone();
        let layers = &self.settings_layers;
        let mut places = vec![Place {
            scope: Scope::Global,
            name: msg::settings_scope_all().to_string(),
            settings: layers.resolve(None, None),
        }];
        if let Some(app) = &application {
            places.push(Place {
                scope: Scope::Application(app.clone()),
                name: crate::settings_layers::application_name(app),
                settings: layers.resolve(Some(app), None),
            });
        }
        if let Some(site) = &site {
            places.push(Place {
                scope: Scope::Site(site.clone()),
                name: msg::settings_scope_site(site.as_str()).render(),
                settings: layers.resolve(application.as_deref(), Some(site)),
            });
        }
        let opened_on = usize::from(application.is_some());
        (places, opened_on)
    }

    /// Opens Settings Center (Insert+6) or Quick Settings (Insert+V), or brings it to the
    /// front; the other one gives way to it. Okur speaks nothing itself: it reads the dialog
    /// when the dialog takes the focus, as any application's.
    pub(crate) fn open_settings(
        &mut self,
        flavor: crate::ui::settings_tree::Flavor,
    ) -> Option<String> {
        if self.ui.is_none() {
            let speech = msg::settings_dialog_unavailable().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        let mut tree = match self.settings_tree.take() {
            Some(tree) if tree.flavor() == flavor => tree,
            other => {
                if let (Some(other), Some(ui)) = (other, &self.ui) {
                    ui.send(crate::ui::ToUi::DialogClose { id: other.id });
                }
                self.dialog_count += 1;
                let (places, place) = self.settings_places();
                crate::ui::settings_tree::SettingsTree::new(
                    self.dialog_count,
                    flavor,
                    places,
                    place,
                    self.speech.voices(),
                    self.settings_history.clone(),
                )
            }
        };
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: tree.spec(),
            });
            if let Some(pane) = tree.pane_update() {
                ui.send(pane);
            }
        }
        self.settings_tree = Some(tree);
        None
    }

    /// Opens the verbosity levels' dialog over Settings Center (Configure Verbosity Levels),
    /// or brings it to the front.
    fn open_verbosity_levels(&mut self) {
        let dialog = match self.settings_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                let (places, place) = self.settings_places();
                crate::ui::settings::SettingsDialog::new(
                    self.dialog_count,
                    places,
                    place,
                    self.speech.voices(),
                )
            }
        };
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.settings_dialog = Some(dialog);
    }

    /// Okur's main window (Insert+J), as JAWS's: "Okur" and its version, and its menus, of
    /// JAWS's items those Okur has, in JAWS's order (the redesign's §3.2). Okur says nothing
    /// itself: the window is read when it takes the focus.
    pub(crate) fn open_okur_window(&mut self) -> Option<String> {
        use crate::ui::{MainWindowSpec, MenuItemSpec, MenuSpec};
        let profiles_menu = self.voice_profiles_menu();
        let Some(ui) = &self.ui else {
            let speech = msg::settings_dialog_unavailable().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        };
        let item =
            |id: &str, label: crate::messages::StaticMessage, key: Option<&str>| MenuItemSpec {
                id: id.to_owned(),
                label: label.to_string(),
                key: key.map(str::to_owned),
                checked: false,
                items: Vec::new(),
            };
        let menu = |label: crate::messages::StaticMessage, items: Vec<MenuItemSpec>| MenuSpec {
            label: label.to_string(),
            items,
        };
        ui.send(crate::ui::ToUi::MainWindowShow {
            window: MainWindowSpec {
                title: msg::okur_window_title().to_string(),
                lines: vec![msg::okur_window_version(env!("CARGO_PKG_VERSION")).render()],
                menus: vec![
                    menu(
                        msg::okur_menu_options(),
                        vec![
                            item(MENU_BASICS, msg::okur_menu_basics(), None),
                            MenuItemSpec {
                                items: vec![
                                    item(
                                        MENU_VOICE_ADJUSTMENT,
                                        msg::okur_menu_voice_adjustment(),
                                        None,
                                    ),
                                    item(MENU_VOICE_PROFILE, msg::okur_menu_voice_profile(), None),
                                ],
                                ..item("voices", msg::okur_menu_voices(), None)
                            },
                            item(MENU_APPLICATIONS, msg::okur_menu_applications(), None),
                            item(MENU_RESTORE, msg::okur_menu_restore(), None),
                            item(
                                crate::ui::MAIN_WINDOW_EXIT,
                                msg::okur_menu_exit(),
                                Some(msg::okur_menu_exit_key().as_str()),
                            ),
                        ],
                    ),
                    menu(
                        msg::okur_menu_utilities(),
                        vec![
                            item(
                                MENU_NOTIFICATIONS,
                                msg::okur_menu_notification_history(),
                                Some(msg::okur_menu_notification_history_key().as_str()),
                            ),
                            item(MENU_SETTINGS_CENTER, msg::okur_menu_settings_center(), None),
                        ],
                    ),
                    menu(msg::okur_menu_language(), vec![profiles_menu]),
                    menu(
                        msg::okur_menu_help(),
                        vec![
                            item(MENU_ABOUT, msg::okur_menu_about(), None),
                            item(MENU_STARTUP_WIZARD, msg::okur_menu_startup_wizard(), None),
                        ],
                    ),
                ],
            },
        });
        None
    }

    /// Language's Voice Profiles, as JAWS's: Default with the profiles, the one in use checked;
    /// For <application> with Use Default first; No Speech; Create New... [obs, the walk of
    /// Language's Voice Profiles]. The application is the one in front as the window opens,
    /// as JAWS's menus from the tray offer it; JAWS's window offers itself, "For JAWS Window".
    fn voice_profiles_menu(&mut self) -> crate::ui::MenuItemSpec {
        use crate::ui::voice_profile::{current, profiles};
        use crate::ui::MenuItemSpec;
        let entry = |id: String, label: String, checked: bool| MenuItemSpec {
            id,
            label,
            key: None,
            checked,
            items: Vec::new(),
        };
        let global = self.settings_layers.resolve(None, None);
        let offered = profiles(&global.voice_profiles);
        let in_use = current(&offered, &global.voice);
        let mut items = vec![MenuItemSpec {
            items: offered
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    entry(
                        format!("{MENU_PROFILE_DEFAULT}{}", p.name),
                        p.name.clone(),
                        i == in_use,
                    )
                })
                .collect(),
            ..entry(
                "profile_default".into(),
                msg::okur_menu_profile_default().to_string(),
                false,
            )
        }];
        self.okur_window_application = self.settings_place.0.clone();
        if let Some(app) = &self.okur_window_application {
            let own = self
                .settings_layers
                .applications
                .get(app)
                .and_then(|values| values.get("voice"))
                .and_then(|voice| voice.get("profile"))
                .and_then(|profile| profile.as_str());
            let use_default = entry(
                MENU_PROFILE_USE_DEFAULT.into(),
                msg::voice_profile_use_default().to_string(),
                own.is_none(),
            );
            items.push(MenuItemSpec {
                items: std::iter::once(use_default)
                    .chain(offered.iter().map(|p| {
                        entry(
                            format!("{MENU_PROFILE_APPLICATION}{}", p.name),
                            p.name.clone(),
                            own == Some(p.name.as_str()),
                        )
                    }))
                    .collect(),
                ..entry(
                    "profile_application".into(),
                    msg::okur_menu_profile_for(app.as_str()).render(),
                    false,
                )
            });
        }
        items.push(entry(
            MENU_NO_SPEECH.into(),
            msg::okur_menu_no_speech().to_string(),
            false,
        ));
        items.push(entry(
            MENU_VOICE_ADJUSTMENT.into(),
            msg::okur_menu_create_profile().to_string(),
            false,
        ));
        MenuItemSpec {
            items,
            ..entry(
                "voice_profiles".into(),
                msg::okur_menu_voice_profiles().to_string(),
                false,
            )
        }
    }

    /// Language's No Speech: the synthesizer silent until a voice profile is chosen, or Okur
    /// starts again (JAWS's help, Language Menu Overview).
    fn no_speech(&mut self) {
        if let Ok(value) = serde_json::to_value(crate::settings::Synthesizer::NoSpeech) {
            self.settings_layers
                .set_for_session(&["voice", "synthesizer"], value);
            self.resolve_settings();
        }
    }

    /// One of the main window's menu items, by its id.
    fn main_menu_command(&mut self, id: &str) -> Option<String> {
        match id {
            MENU_BASICS => self.open_basics(),
            MENU_VOICE_ADJUSTMENT => self.open_voice_adjustment(),
            MENU_VOICE_PROFILE => {
                let _ = self.open_voice_profile();
            }
            MENU_APPLICATIONS => self.open_applications(),
            MENU_RESTORE => self.ask_restore_factory(),
            MENU_NOTIFICATIONS => self.open_notification_history(),
            MENU_SETTINGS_CENTER => {
                return self.open_settings(crate::ui::settings_tree::Flavor::Center);
            }
            MENU_ABOUT => {
                self.show_message_box(msg::okur_window_version(env!("CARGO_PKG_VERSION")).render())
            }
            MENU_STARTUP_WIZARD => return self.open_startup_wizard(),
            MENU_NO_SPEECH => self.no_speech(),
            MENU_PROFILE_USE_DEFAULT => {
                let app = self.okur_window_application.clone();
                self.choose_voice_profiles(None, app.map(|app| (app, None)));
            }
            _ if id.starts_with(MENU_PROFILE_DEFAULT) => {
                let name = id
                    .strip_prefix(MENU_PROFILE_DEFAULT)
                    .unwrap_or_default()
                    .to_owned();
                self.choose_voice_profiles(Some(name), None);
            }
            _ if id.starts_with(MENU_PROFILE_APPLICATION) => {
                let name = id
                    .strip_prefix(MENU_PROFILE_APPLICATION)
                    .unwrap_or_default()
                    .to_owned();
                let app = self.okur_window_application.clone();
                self.choose_voice_profiles(None, app.map(|app| (app, Some(name))));
            }
            crate::ui::MAIN_WINDOW_EXIT => return self.execute(crate::command::Command::ShutDown),
            _ => {}
        }
        None
    }

    /// Select a Voice Profile (Insert+Ctrl+S), or brings it to the front: the profile for all
    /// applications and for the application in front.
    pub(crate) fn open_voice_profile(&mut self) -> Option<String> {
        if self.ui.is_none() {
            let speech = msg::settings_dialog_unavailable().to_string();
            self.speech.speak(&speech, true);
            return Some(speech);
        }
        let dialog = match self.voice_profile_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                let global = self.settings_layers.resolve(None, None);
                let application = self.settings_place.0.clone().map(|app| {
                    let own = self
                        .settings_layers
                        .applications
                        .get(&app)
                        .and_then(|values| values.get("voice"))
                        .and_then(|voice| voice.get("profile"))
                        .and_then(|profile| profile.as_str())
                        .map(str::to_owned);
                    (app, own)
                });
                crate::ui::voice_profile::VoiceProfileDialog::new(
                    self.dialog_count,
                    &crate::ui::voice_profile::profiles(&global.voice_profiles),
                    &global.voice,
                    application,
                )
            }
        };
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.voice_profile_dialog = Some(dialog);
        None
    }

    /// The profiles chosen in Select a Voice Profile: `default` for all applications, then the
    /// application's, a profile or none to follow the default.
    fn choose_voice_profiles(
        &mut self,
        default: Option<String>,
        application: Option<(String, Option<String>)>,
    ) {
        use crate::settings_layers::Scope;
        // A profile chosen ends No Speech.
        self.settings_layers
            .clear_for_session(&["voice", "synthesizer"]);
        let global = self.settings_layers.resolve(None, None);
        let profiles = crate::ui::voice_profile::profiles(&global.voice_profiles);
        let profile = |name: &str| {
            profiles
                .iter()
                .find(|p| p.name == name)
                .map(|p| p.voice.clone())
        };
        if let Some((name, voice)) = default.and_then(|name| Some((name.clone(), profile(&name)?)))
        {
            let mut after = global.clone();
            after.voice = global.voice.with_profile(&name, &voice);
            self.apply_settings(&Scope::Global, &global, &after, &[]);
        }
        if let Some((app, choice)) = application {
            let before = self.settings_layers.resolve(Some(&app), None);
            let voice = match choice {
                Some(name) => profile(&name).map(|voice| before.voice.with_profile(&name, &voice)),
                // "Use Default": the voice for all applications.
                None => Some(
                    before
                        .voice
                        .with_voice_of(&self.settings_layers.resolve(None, None).voice),
                ),
            };
            if let Some(voice) = voice {
                let after = crate::settings::Settings {
                    voice,
                    ..before.clone()
                };
                self.settings_dialog_application = Some(app.clone());
                self.apply_settings(&Scope::Application(app), &before, &after, &[]);
            }
        }
    }

    /// Opens Manage Application Settings, or brings it to the front: the applications with
    /// settings of their own.
    fn open_applications(&mut self) {
        let dialog = match self.applications_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                let global = self.settings_layers.resolve(None, None);
                crate::ui::applications::ManageApplications::new(
                    self.dialog_count,
                    self.settings_layers.applications.keys().cloned().collect(),
                    &global.disabled_applications,
                )
            }
        };
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.applications_dialog = Some(dialog);
    }

    /// Manage Application Settings' OK: `disabled` applications' settings unused, `removed`
    /// ones' forgotten.
    fn manage_applications(&mut self, disabled: Vec<String>, removed: Vec<String>) {
        for application in &removed {
            self.settings_layers.remove_application(application);
        }
        let before = self.settings_layers.resolve(None, None);
        let after = crate::settings::Settings {
            disabled_applications: disabled
                .into_iter()
                .filter(|app| !removed.contains(app))
                .collect(),
            ..before.clone()
        };
        // Saved with the applications removed, and resolved anew, even with nothing else changed.
        self.apply_settings(&crate::settings_layers::Scope::Global, &before, &after, &[]);
    }

    /// Restore to Factory Settings: its question first.
    fn ask_restore_factory(&mut self) {
        self.dialog_count += 1;
        self.restore_question = Some(self.dialog_count);
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: crate::ui::applications::restore_question(self.dialog_count),
            });
        }
    }

    /// Restore to Factory Settings' Yes: every saved setting forgotten, Okur's defaults in force.
    fn restore_factory(&mut self) {
        self.settings_layers.restore_factory();
        self.settings_history.clear();
        if let Some(store) = &self.settings_store {
            store.save(&self.settings_layers);
        }
        self.save_settings_history();
        self.resolve_settings();
    }

    /// What Voice Adjustment, or its Save Voice Profile, decided.
    fn voice_adjustment_outcome(&mut self, outcome: crate::ui::voice_adjustment::Outcome) {
        use crate::settings_layers::Scope;
        use crate::ui::voice_adjustment::Outcome;
        let send = |engine: &Self, messages: Vec<crate::ui::ToUi>| {
            if let Some(ui) = &engine.ui {
                messages.into_iter().for_each(|m| ui.send(m));
            }
        };
        match outcome {
            Outcome::Nothing => {}
            Outcome::Send(messages) => send(self, messages),
            Outcome::Apply { before, settings } => {
                self.apply_settings(&Scope::Global, &before, &settings, &[]);
                self.close_voice_adjustment();
            }
            Outcome::AskName => {
                self.dialog_count += 1;
                let id = self.dialog_count;
                if let Some(dialog) = self.voice_adjustment.as_mut() {
                    let spec = dialog.save_as(id);
                    send(self, vec![crate::ui::ToUi::DialogShow { dialog: spec }]);
                }
            }
            Outcome::Save {
                name,
                voice,
                send: messages,
            } => {
                let before = self.settings_layers.resolve(None, None);
                let mut after = before.clone();
                after.voice_profiles.insert(name, *voice);
                self.apply_settings(&Scope::Global, &before, &after, &[]);
                send(self, messages);
            }
            Outcome::Delete {
                name,
                send: messages,
            } => {
                self.settings_layers.remove_voice_profile(&name);
                if let Some(store) = &self.settings_store {
                    store.save(&self.settings_layers);
                }
                self.resolve_settings();
                send(self, messages);
            }
            Outcome::Sample(voice) => {
                self.speech.configure(&voice);
                self.speech
                    .speak(msg::voice_adjustment_sample_text().as_str(), true);
                self.configure_voice();
            }
            Outcome::Closed => self.close_voice_adjustment(),
        }
    }

    /// Voice Adjustment closed, and its Save Voice Profile if it is open.
    fn close_voice_adjustment(&mut self) {
        if let (Some(dialog), Some(ui)) = (self.voice_adjustment.take(), &self.ui) {
            for id in dialog.save_as_id().into_iter().chain([dialog.id]) {
                ui.send(crate::ui::ToUi::DialogClose { id });
            }
        }
        self.returning_from_dialog();
    }

    /// Opens Voice Adjustment, or brings it to the front: the voice for all applications.
    fn open_voice_adjustment(&mut self) {
        let dialog = match self.voice_adjustment.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                let global = self.settings_layers.resolve(None, None);
                crate::ui::voice_adjustment::VoiceAdjustment::new(
                    self.dialog_count,
                    global,
                    self.speech.voices(),
                )
            }
        };
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.voice_adjustment = Some(dialog);
    }

    /// Opens Basics, or brings it to the front: the settings for all applications.
    fn open_basics(&mut self) {
        let dialog = match self.basics_dialog.take() {
            Some(dialog) => dialog,
            None => {
                self.dialog_count += 1;
                let global = self.settings_layers.resolve(None, None);
                crate::ui::basics::BasicsDialog::new(self.dialog_count, global)
            }
        };
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::DialogShow {
                dialog: dialog.spec(),
            });
        }
        self.basics_dialog = Some(dialog);
    }

    /// What a settings dialog decided: Settings Center's or Quick Settings' (`tree`), or the
    /// verbosity levels'.
    fn settings_outcome(&mut self, outcome: crate::ui::settings::Outcome, tree: bool) {
        use crate::ui::settings::Outcome;
        match outcome {
            Outcome::Nothing => {}
            Outcome::Send(messages) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
            }
            Outcome::Changed(messages, said) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
                self.speech.speak(&said, true);
            }
            Outcome::BeforeFocus(messages, said) => {
                self.said_before_focus = Some(said);
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
            }
            Outcome::Apply {
                scope,
                before,
                settings,
                close,
                temporary,
            } => {
                self.apply_settings(&scope, &before, &settings, &temporary);
                if close {
                    self.close_settings_dialog(tree);
                }
            }
            Outcome::Closed => {
                if tree {
                    self.settings_tree = None;
                } else {
                    self.settings_dialog = None;
                }
                self.returning_from_dialog();
            }
            Outcome::OpenVerbosityLevels => self.open_verbosity_levels(),
            Outcome::Forget {
                scope,
                setting,
                send,
            } => {
                self.settings_history.retain(|(place, id)| {
                    !(*place == scope && setting.is_none_or(|setting| setting == *id))
                });
                self.save_settings_history();
                if let Some(ui) = &self.ui {
                    send.into_iter().for_each(|m| ui.send(m));
                }
            }
            Outcome::Close => self.close_settings_dialog(tree),
            Outcome::AskToSave => {
                self.dialog_count += 1;
                let id = self.dialog_count;
                if let (Some(dialog), Some(ui)) = (self.settings_tree.as_mut(), &self.ui) {
                    ui.send(crate::ui::ToUi::DialogShow {
                        dialog: dialog.save_question(id),
                    });
                }
            }
        }
    }

    /// Closes the settings dialog (`tree`: Settings Center or Quick Settings), and the question
    /// it asked if it is open.
    fn close_settings_dialog(&mut self, tree: bool) {
        let ids: Vec<u32> = if tree {
            self.settings_tree
                .take()
                .map(|t| t.question().into_iter().chain([t.id]).collect())
                .unwrap_or_default()
        } else {
            self.settings_dialog
                .take()
                .map(|d| d.id)
                .into_iter()
                .collect()
        };
        if let Some(ui) = &self.ui {
            ids.into_iter()
                .for_each(|id| ui.send(crate::ui::ToUi::DialogClose { id }));
        }
        self.returning_from_dialog();
    }

    /// What the user did to one of Okur's windows.
    fn handle_ui_event(&mut self, message: &crate::ui::FromUi) -> Option<String> {
        match message {
            // The viewer's window took the focus or was closed: the viewer ends silently, and
            // the focus says where the user is (JAWS on the dev box, 2026-10-17: Alt+Tab to
            // its viewer's window ended the viewer, and the page took the focus back).
            crate::ui::FromUi::ViewerDismissed => {
                self.viewer = None;
                None
            }
            _ if self
                .list_dialog
                .as_ref()
                .is_some_and(|d| Some(d.id) == dialog_of(message)) =>
            {
                self.handle_list_event(message)
            }
            _ if self.is_find_event(message) => self.handle_find_event(message),
            _ if self.is_skim_event(message) => self.handle_skim_event(message),
            _ if self.is_notification_event(message) => self.handle_notification_event(message),
            _ if self.is_window_list_event(message) => self.handle_window_list_event(message),
            _ if self.is_tray_event(message) => self.handle_tray_event(message),
            _ if self.is_jump_event(message) => self.handle_jump_event(message),
            _ if self.is_label_event(message) => self.handle_label_event(message),
            _ if self.is_placemarker_event(message) => self.handle_placemarker_event(message),
            _ if self.is_flexible_web_event(message) => self.handle_flexible_web_event(message),
            _ if self.is_startup_wizard_event(message) => self.handle_startup_wizard_event(message),
            _ if self.is_quit_event(message) => self.handle_quit_event(message),
            _ if self.is_message_box_event(message) => self.handle_message_box_event(message),
            _ if self.is_overwrite_event(message) => self.handle_overwrite_event(message),
            crate::ui::FromUi::MainMenuCommand { id } => self.main_menu_command(id),
            _ if self.restore_question.is_some() && dialog_of(message) == self.restore_question => {
                match message {
                    crate::ui::FromUi::DialogPressed { id, control } if control == "yes" => {
                        if let Some(ui) = &self.ui {
                            ui.send(crate::ui::ToUi::DialogClose { id: *id });
                        }
                        self.restore_question = None;
                        self.restore_factory();
                        self.returning_from_dialog();
                    }
                    crate::ui::FromUi::DialogClosed { .. } => {
                        self.restore_question = None;
                        self.returning_from_dialog();
                    }
                    _ => {}
                }
                None
            }
            _ if self
                .applications_dialog
                .as_ref()
                .is_some_and(|d| Some(d.id) == dialog_of(message)) =>
            {
                use crate::ui::applications::Outcome;
                match self.applications_dialog.as_mut()?.handle(message) {
                    Outcome::Nothing => {}
                    Outcome::Send(messages) => {
                        if let Some(ui) = &self.ui {
                            messages.into_iter().for_each(|m| ui.send(m));
                        }
                    }
                    Outcome::Apply { disabled, removed } => {
                        if let (Some(dialog), Some(ui)) =
                            (self.applications_dialog.take(), &self.ui)
                        {
                            ui.send(crate::ui::ToUi::DialogClose { id: dialog.id });
                        }
                        self.manage_applications(disabled, removed);
                        self.returning_from_dialog();
                    }
                    Outcome::Closed => {
                        self.applications_dialog = None;
                        self.returning_from_dialog();
                    }
                }
                None
            }
            _ if self
                .voice_profile_dialog
                .as_ref()
                .is_some_and(|d| Some(d.id) == dialog_of(message)) =>
            {
                use crate::ui::voice_profile::Outcome;
                match self.voice_profile_dialog.as_mut()?.handle(message) {
                    Outcome::Choose {
                        default,
                        application,
                    } => {
                        if let (Some(dialog), Some(ui)) =
                            (self.voice_profile_dialog.take(), &self.ui)
                        {
                            ui.send(crate::ui::ToUi::DialogClose { id: dialog.id });
                        }
                        self.choose_voice_profiles(default, application);
                        self.returning_from_dialog();
                    }
                    Outcome::Closed => {
                        self.voice_profile_dialog = None;
                        self.returning_from_dialog();
                    }
                    Outcome::Nothing => {}
                }
                None
            }
            _ if self
                .voice_adjustment
                .as_ref()
                .is_some_and(|d| dialog_of(message).is_some_and(|id| d.owns(id))) =>
            {
                let outcome = self.voice_adjustment.as_mut()?.handle(message);
                self.voice_adjustment_outcome(outcome);
                None
            }
            _ if self
                .basics_dialog
                .as_ref()
                .is_some_and(|d| Some(d.id) == dialog_of(message)) =>
            {
                use crate::ui::settings::Outcome;
                match self.basics_dialog.as_mut()?.handle(message) {
                    Outcome::Apply {
                        scope,
                        before,
                        settings,
                        temporary,
                        ..
                    } => {
                        self.apply_settings(&scope, &before, &settings, &temporary);
                        if let (Some(dialog), Some(ui)) = (self.basics_dialog.take(), &self.ui) {
                            ui.send(crate::ui::ToUi::DialogClose { id: dialog.id });
                        }
                        self.returning_from_dialog();
                    }
                    Outcome::Closed => {
                        self.basics_dialog = None;
                        self.returning_from_dialog();
                    }
                    _ => {}
                }
                None
            }
            _ if self
                .settings_tree
                .as_ref()
                .is_some_and(|t| dialog_of(message).is_some_and(|id| t.owns(id))) =>
            {
                let outcome = self.settings_tree.as_mut()?.handle(message);
                self.settings_outcome(outcome, true);
                None
            }
            _ => {
                let outcome = self.settings_dialog.as_mut()?.handle(message);
                self.settings_outcome(outcome, false);
                None
            }
        }
    }

    /// Whether the page's focus is a select: a combo box one does not type in.
    fn page_focus_is_select(&self) -> bool {
        self.last_page_focus
            .and_then(|id| self.cursor.buffer.node(id))
            .is_some_and(|n| {
                n.role == crate::aom::Role::ComboBox
                    && n.attributes.get("editable").is_none_or(|e| e != "true")
            })
    }

    /// Whether the focus is still returning from one of Okur's dialogs.
    fn quiet_return(&self) -> bool {
        self.quiet_focus_until
            .is_some_and(|until| std::time::Instant::now() < until)
    }

    /// Whether `focus` is the field a dialog gave the focus to: said once, while the rest of
    /// the return (the page's own focus, Chrome's repeats of the field's) stays quiet.
    fn reaches_dialog_focus_target(&mut self, focus: &crate::event::FocusEvent) -> bool {
        let Some(target) = self.dialog_focus_target else {
            return false;
        };
        let node = self.page_node_for(focus);
        let reached = node == Some(target);
        tracing::debug!(
            target,
            ?node,
            reached,
            "focus while returning from a dialog"
        );
        if reached {
            self.dialog_focus_target = None;
        }
        reached
    }

    /// The page as JAWS names it when the focus comes back to it from one of its dialogs:
    /// "Web App Patterns document" [obs, 221/s2, 227/s11].
    fn page_on_return(&self) -> Option<String> {
        self.cursor.buffer.document_title().map(|title| {
            format!(
                "{title}{}",
                crate::linearize::role_marker(msg::role_document().as_str())
            )
        })
    }

    /// Starts the quiet return after one of Okur's dialogs closed: the window it came from
    /// takes the focus back within this.
    fn returning_from_dialog(&mut self) {
        const RETURN: std::time::Duration = std::time::Duration::from_millis(800);
        self.quiet_focus_until = Some(std::time::Instant::now() + RETURN);
    }

    /// Opens one of JAWS's list dialogs on the page's elements of its kind; with none, says
    /// so, as JAWS does.
    pub(crate) fn open_list(&mut self, kind: crate::ui::lists::ListKind) -> Option<String> {
        use crate::ui::lists::{ListDialog, ListKind};
        // A document whose structure the platform reads (Word's) lists its own headings,
        // which the platform reads first [obs, 519/s2].
        if kind == ListKind::Headings
            && self.app_mode != AppMode::VirtualBuffer
            && self.editing_document()
            && self
                .last_text
                .as_ref()
                .is_some_and(|t| t.structure.is_some())
            && self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_document_headings())
        {
            tracing::debug!("heading list: the document's headings asked for");
            self.document_headings_asked = true;
            return None;
        }
        // A document whose links the platform reads (Word's) lists its own links, as JAWS's
        // Links List shows Word's hyperlinks [obs, 1049/s1].
        if kind == ListKind::Links
            && self.app_mode != AppMode::VirtualBuffer
            && self.editing_document()
            && self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_document_links())
        {
            tracing::debug!("links list: the document's links asked for");
            self.document_links_asked = true;
            return None;
        }
        // Its fields too, in JAWS's List Fields [obs, 1050/s1, 1053].
        if kind == ListKind::FormFields
            && self.app_mode != AppMode::VirtualBuffer
            && self.editing_document()
            && self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_document_fields())
        {
            tracing::debug!("fields list: the document's fields asked for");
            self.document_fields_asked = true;
            return None;
        }
        // Outside a page, a list the application words is its own, never the buffer's: the
        // buffer can still hold the last page read, whose links Word's Links List would have
        // shown.
        let application_list = self
            .last_native_focus
            .as_ref()
            .filter(|_| self.app_mode != AppMode::VirtualBuffer)
            .and_then(|focus| self.app_for(focus)?.empty_list(focus, &kind));
        if let Some(none) = application_list {
            self.speech.speak(&none, true);
            return Some(none);
        }
        let id = self.dialog_count + 1;
        let (dialog, none) = match kind {
            ListKind::Links => {
                let links = self.cursor.buffer.get_links_list();
                (
                    (!links.is_empty()).then(|| ListDialog::links(id, &links)),
                    msg::links_list_none().to_string(),
                )
            }
            ListKind::Headings => {
                let headings = self.cursor.buffer.get_headings_list();
                (
                    (!headings.is_empty()).then(|| ListDialog::headings(id, &headings)),
                    msg::quicknav_none_headings().to_string(),
                )
            }
            ListKind::Features => (Some(ListDialog::features(id)), String::new()),
            ListKind::Words => {
                let words = crate::word_list::words(&self.cursor.buffer);
                (
                    (!words.is_empty()).then(|| ListDialog::words(id, &words)),
                    String::new(),
                )
            }
            ListKind::Regions => {
                let regions = crate::element_lists::regions(&self.cursor.buffer);
                (
                    (!regions.is_empty()).then(|| ListDialog::regions(id, &regions)),
                    msg::regions_list_none().to_string(),
                )
            }
            ListKind::Elements(elements) => {
                let items = elements.items(&self.cursor.buffer);
                (
                    (!items.is_empty()).then(|| ListDialog::elements(id, elements, &items)),
                    elements.none().to_string(),
                )
            }
            // A document's fields are only listed from the document, above.
            ListKind::DocumentFields => (None, msg::form_fields_list_none().to_string()),
            // A document's notes too, by Word's keys (`Self::open_document_notes`), and what
            // the Virtual Viewer shows of them.
            ListKind::DocumentNotes(_) | ListKind::NotesToView | ListKind::SheetList(_) => {
                return None
            }
            // With none, JAWS says so and then the page [obs, 221/s2].
            ListKind::FormFields => {
                let fields = self.cursor.buffer.get_form_fields_list();
                let page = self.page_on_return().map(|page| format!("\n{page}"));
                (
                    (!fields.is_empty()).then(|| ListDialog::form_fields(id, &fields)),
                    format!(
                        "{}{}",
                        self.said_short(msg::form_fields_list_none()),
                        page.unwrap_or_default()
                    ),
                )
            }
        };
        let Some(dialog) = dialog else {
            self.speech.speak(&none, true);
            return Some(none);
        };
        self.show_list(dialog, AppMode::VirtualBuffer)
    }

    /// Shows a list dialog, which gives the keys back to `return_mode` when it closes.
    fn show_list(
        &mut self,
        dialog: crate::ui::lists::ListDialog,
        return_mode: AppMode,
    ) -> Option<String> {
        let ui = self.ui.as_ref()?;
        self.dialog_count = dialog.id;
        ui.send(crate::ui::ToUi::DialogShow {
            dialog: dialog.spec(),
        });
        self.list_dialog = Some(dialog);
        self.list_return_mode = return_mode;
        self.list_return_focus = self.last_native_focus.clone();
        // The keys are the dialog's now, before its window reports the focus: a Down the
        // window has yet to take must not move the virtual cursor [obs, 242/s2].
        self.app_mode = AppMode::NativeControl;
        None
    }

    /// The focused document's headings, asked for its Heading List: the list, as a page's
    /// is, "Library Hours : 1" [obs, 519/s2]; with none, said so.
    fn document_headings(&mut self, headings: &[crate::event::DocumentHeading]) -> Option<String> {
        if !std::mem::take(&mut self.document_headings_asked) {
            return None;
        }
        tracing::debug!(
            headings = headings.len(),
            "heading list: the document's headings came"
        );
        if headings.is_empty() {
            let none = msg::quicknav_none_headings().to_string();
            self.speech.speak(&none, true);
            return Some(none);
        }
        let items: Vec<crate::buffer::HeadingListItem> = headings
            .iter()
            .map(|heading| crate::buffer::HeadingListItem {
                node_id: heading.paragraph,
                text: heading.text.clone(),
                level: u32::from(heading.level),
                offset: heading.paragraph,
            })
            .collect();
        let dialog = crate::ui::lists::ListDialog::headings(self.dialog_count + 1, &items);
        let mode = self.app_mode;
        self.show_list(dialog, mode)
    }

    /// The focused document's fields, asked for its list of fields: JAWS's List Fields, the first
    /// selected, "List Fields dialog | List1 List view | HYPERLINK opening hours | 1 of 4"
    /// [obs, 1050/s1]; with none, the application's words for it ("no fields" [obs, 1047/s1]).
    fn document_fields(&mut self, fields: &[crate::event::DocumentField]) -> Option<String> {
        if !std::mem::take(&mut self.document_fields_asked) {
            return None;
        }
        tracing::debug!(
            fields = fields.len(),
            "fields list: the document's fields came"
        );
        if fields.is_empty() {
            let none = self
                .last_native_focus
                .as_ref()
                .and_then(|focus| {
                    self.app_for(focus)?
                        .empty_list(focus, &crate::ui::lists::ListKind::FormFields)
                })
                .unwrap_or_else(|| msg::form_fields_list_none().to_string());
            self.speech.speak(&none, true);
            return Some(none);
        }
        let dialog = crate::ui::lists::ListDialog::document_fields(self.dialog_count + 1, fields);
        let mode = self.app_mode;
        self.show_list(dialog, mode)
    }

    /// Word's keys for JAWS's list of its document's comments or footnotes (Ctrl+Shift+Apostrophe,
    /// Insert+Shift+F): the notes are read from the document first, and the list opens when they
    /// come. False outside Word's document, where the key is not Word's.
    pub(crate) fn open_document_notes(&mut self, kind: crate::event::NoteKind) -> bool {
        if !self.in_word_document()
            || !self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_document_notes(kind))
        {
            return false;
        }
        tracing::debug!(?kind, "notes list: the document's notes asked for");
        self.document_notes_asked = Some(kind);
        true
    }

    /// Insert+Shift+R or Ctrl+Insert+R in Word's document: whether the document tracks its
    /// changes is asked first, which both need. False outside such a document.
    pub(crate) fn ask_revisions(&mut self, key: crate::apps::windows::word::RevisionKey) -> bool {
        if !self.in_word_document()
            || !self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_word_key(crate::event::WordKey::TrackChanges))
        {
            return false;
        }
        tracing::debug!(?key, "revisions: whether the document tracks changes asked");
        self.revisions_asked = Some(key);
        true
    }

    /// Whether the document tracks its changes came, for a key for its revisions: without,
    /// what JAWS says then [obs, 1109/s1, s3]; with, the list of revisions, or the revision at
    /// the caret [obs, 1108/s3, s8].
    fn revisions_tracked(&mut self, state: Option<crate::event::WordKeyState>) -> Option<String> {
        let key = self.revisions_asked.take()?;
        match state? {
            crate::event::WordKeyState::TrackChanges(false) => {
                let said = key.untracked();
                self.speech.speak(&said, true);
                Some(said)
            }
            crate::event::WordKeyState::TrackChanges(true) => match key {
                crate::apps::windows::word::RevisionKey::List => {
                    self.open_document_notes(crate::event::NoteKind::Revision);
                    None
                }
                crate::apps::windows::word::RevisionKey::Announce => {
                    let said =
                        crate::apps::windows::word::announce_revision(self.last_text.as_ref()?);
                    self.speech.speak(&said, true);
                    Some(said)
                }
            },
            _ => None,
        }
    }

    /// Say Bottom Line of Window in Word's document: its status bar is asked for. False
    /// outside such a document.
    pub(crate) fn ask_word_status_bar(&mut self) -> bool {
        if !self.in_word_document()
            || !self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_status_bar())
        {
            return false;
        }
        self.status_bar_asked = true;
        true
    }

    /// Word's status bar came, asked by Say Bottom Line of Window: its items, "Page Number
    /// Page 3 of 3 | Word Count 26 words | ... | Print Layout | Zoom Out | Zoom In | Zoom 100%"
    /// [obs, 1113/s9]; with none read, the document's last line, as elsewhere.
    fn word_status_bar(&mut self, items: &[String]) -> Option<String> {
        if !std::mem::take(&mut self.status_bar_asked) {
            return None;
        }
        let said = if items.is_empty() {
            crate::editing::text_lines(&self.last_text.as_ref()?.text)
                .into_iter()
                .rev()
                .find(|line| !line.trim().is_empty())?
        } else {
            items.join("\n")
        };
        self.speech.speak(&said, true);
        Some(said)
    }

    /// One of Excel's lists, asked for from a cell of its sheet. False elsewhere.
    pub(crate) fn open_sheet_list(&mut self, kind: crate::event::SheetListKind) -> bool {
        let in_sheet = self
            .last_native_focus
            .as_ref()
            .and_then(crate::apps::windows::excel::cell)
            .is_some();
        if !in_sheet
            || !self
                .actions
                .as_ref()
                .is_some_and(|actions| actions.read_sheet_list(kind))
        {
            return false;
        }
        self.sheet_list_asked = Some(kind);
        true
    }

    /// One of Excel's lists came: shown as JAWS's, the first entry selected, "Cells with
    /// Formulas dialog | List1 List view | B4: =SUM(B2:B3) | 1 of 2" [obs, 1116/s1]; with none,
    /// said so, "No cells with data in current column." [obs, 521/s9 in round-521e].
    fn sheet_list(
        &mut self,
        kind: crate::event::SheetListKind,
        column: &str,
        entries: &[crate::event::SheetEntry],
    ) -> Option<String> {
        if self.sheet_list_asked != Some(kind) {
            return None;
        }
        self.sheet_list_asked = None;
        if entries.is_empty() {
            let none = match kind {
                crate::event::SheetListKind::Formulas => msg::excel_no_formulas(),
                crate::event::SheetListKind::Column => msg::excel_no_column_cells(),
                crate::event::SheetListKind::Worksheets => msg::excel_no_worksheets(),
            }
            .to_string();
            self.speech.speak(&none, true);
            return Some(none);
        }
        let dialog =
            crate::ui::lists::ListDialog::sheet_list(self.dialog_count + 1, kind, column, entries);
        let mode = self.app_mode;
        self.show_list(dialog, mode)
    }

    /// Whether the keys go to a document whose structure Word's object model reads.
    fn in_word_document(&self) -> bool {
        self.app_mode != AppMode::VirtualBuffer
            && self.editing_text()
            && self
                .last_text
                .as_ref()
                .is_some_and(|t| t.structure.is_some())
    }

    /// The focused document's notes, asked for the Virtual Viewer: it opens on their count, "There
    /// is only one Comment to display.", then the notes, each after its link, and how to close
    /// it; its first line is said, then "Edit" [obs, 1096/s2-s6, 1097/s3-s6, 1099/s4-s7]. With
    /// none, no viewer: "There are no items to display. | Edit" [obs, 1100/s2, s3].
    fn notes_viewer(
        &mut self,
        kind: crate::event::NoteKind,
        notes: &[crate::event::DocumentNote],
    ) -> Option<String> {
        self.notes_viewer_asked = None;
        let edit = crate::linearize::role_marker(msg::role_edit().as_str());
        if notes.is_empty() {
            let speech = format!("{}\n{edit}", msg::word_viewer_none());
            self.speech.speak(&speech, false);
            return Some(speech);
        }
        let lines = crate::apps::windows::word::notes_viewer_lines(kind, notes)?;
        let viewer = crate::viewer::VirtualViewer::new(lines);
        let speech = format!("{}\n{edit}", viewer.line_text());
        self.viewer = Some(viewer);
        self.show_viewer_window();
        self.speech.speak(&speech, false);
        Some(speech)
    }

    /// The focused document's comments or footnotes, asked for their list: JAWS's Reviewers'
    /// Comments or Footnotes, the first selected, "Reviewers' Comments dialog | List1 List view
    /// | Check the summer hours. Lab Fri, Jan 02, 26 | 1 of 1" [obs, 1082/s1, 1083/s1]; with
    /// none, said so, "There are no comments in the document." [obs, 1089/s1, s3].
    fn document_notes(
        &mut self,
        kind: crate::event::NoteKind,
        notes: &[crate::event::DocumentNote],
    ) -> Option<String> {
        if self.document_notes_asked != Some(kind) {
            return None;
        }
        self.document_notes_asked = None;
        tracing::debug!(
            ?kind,
            notes = notes.len(),
            "notes list: the document's notes came"
        );
        if notes.is_empty() {
            let none = match kind {
                crate::event::NoteKind::Comment => msg::word_no_comments(),
                crate::event::NoteKind::Footnote => msg::word_no_footnotes(),
                crate::event::NoteKind::Endnote => msg::word_no_endnotes(),
                crate::event::NoteKind::Bookmark => msg::word_no_bookmarks(),
                crate::event::NoteKind::Revision => msg::word_no_revisions(),
            }
            .to_string();
            self.speech.speak(&none, true);
            return Some(none);
        }
        self.listed_notes = notes.to_vec();
        let dialog =
            crate::ui::lists::ListDialog::document_notes(self.dialog_count + 1, kind, notes);
        let mode = self.app_mode;
        self.show_list(dialog, mode)
    }

    /// Say Font in Word's document, with the line spacing asked for it (none where it could not
    /// be read: then left out).
    fn word_font(&mut self, spacing: Option<crate::event::LineSpacing>) -> Option<String> {
        if !std::mem::take(&mut self.pending_word_font) {
            return None;
        }
        let said = self
            .last_text
            .as_ref()
            .and_then(|text| crate::apps::windows::word::font(text, spacing.as_ref()))?;
        self.speech.speak(&said, true);
        Some(said)
    }

    /// The focused document's links, asked for its Links List: the list, as a page's is, the
    /// first link selected, "Links List dialog | links List view | opening hours | 1 of 2"
    /// [obs, 1049/s1]; with none, the application's words for it, or Okur's.
    fn document_links(&mut self, links: &[crate::event::DocumentLink]) -> Option<String> {
        if !std::mem::take(&mut self.document_links_asked) {
            return None;
        }
        tracing::debug!(links = links.len(), "links list: the document's links came");
        if links.is_empty() {
            let none = self
                .last_native_focus
                .as_ref()
                .and_then(|focus| {
                    self.app_for(focus)?
                        .empty_list(focus, &crate::ui::lists::ListKind::Links)
                })
                .unwrap_or_else(|| msg::links_list_none().to_string());
            self.speech.speak(&none, true);
            return Some(none);
        }
        // Each by its place in the document: the list's node is the link's index, its offset
        // its paragraph, by which Move To Link moves the caret.
        let items: Vec<crate::buffer::LinkListItem> = links
            .iter()
            .enumerate()
            .map(|(index, link)| crate::buffer::LinkListItem {
                node_id: index,
                text: link.text.clone(),
                href: link.url.clone(),
                is_same_page: false,
                visited: false,
                offset: link.paragraph,
                current: None,
            })
            .collect();
        let dialog = crate::ui::lists::ListDialog::links(self.dialog_count + 1, &items);
        let mode = self.app_mode;
        self.show_list(dialog, mode)
    }

    /// What the user did to the open list dialog.
    fn handle_list_event(&mut self, message: &crate::ui::FromUi) -> Option<String> {
        use crate::ui::lists::{ListKind, Outcome};
        let kind = self.list_dialog.as_ref()?.kind();
        let outcome = self.list_dialog.as_mut()?.handle(message);
        let close = |engine: &mut Self| {
            if let (Some(dialog), Some(ui)) = (engine.list_dialog.take(), &engine.ui) {
                ui.send(crate::ui::ToUi::DialogClose { id: dialog.id });
            }
            engine.app_mode = engine.list_return_mode;
            engine.returning_from_dialog();
        };
        // A document's list moves its caret, by the heading's paragraph.
        let in_document = self.list_return_mode != AppMode::VirtualBuffer;
        match outcome {
            Outcome::Nothing => None,
            // The heading's line is said when the caret is read there, then "Edit": "heading
            // level 1 Library Hours | Edit" [obs, 1036/s3, s7]. The document's focus comes back
            // first, which drops a pending edit key: the move is kept apart.
            // A link's entry keeps its paragraph as its offset, its index as its node.
            Outcome::MoveTo(entry) if in_document => {
                close(self);
                self.moved_by_document_list = true;
                if let Some(actions) = &self.actions {
                    match kind {
                        // OK in List Fields: the caret on the field [obs, 1054/s3].
                        ListKind::DocumentFields => {
                            actions.move_to_document_field(self.last_text.as_ref(), entry.node_id);
                        }
                        ListKind::Links => {
                            actions.move_to_paragraph(self.last_text.as_ref(), entry.offset);
                        }
                        // OK in one of Excel's lists: its cell selected, or its worksheet
                        // activated; the cell's focus then says itself [obs, 1116/s3].
                        ListKind::SheetList(list) => {
                            self.moved_by_document_list = false;
                            // The cell gone to is the answer: its focus is said at once.
                            self.quiet_focus_until = None;
                            if let Some(target) = entry.address.as_deref() {
                                actions.move_to_sheet_entry(list, target);
                            }
                        }
                        // OK in Items to Display in Virtual Viewer: what is gathered is said,
                        // and the viewer opens when the notes come [obs, 1096/s2].
                        ListKind::NotesToView => {
                            self.moved_by_document_list = false;
                            let kind = crate::ui::lists::NOTES_TO_VIEW
                                .get(entry.node_id)
                                .and_then(|(_, kind)| *kind);
                            if let Some(kind) = kind {
                                if let Some(processing) =
                                    crate::apps::windows::word::notes_processing(kind)
                                {
                                    self.speech.speak(&processing, true);
                                }
                                if actions.read_document_notes(kind) {
                                    self.notes_viewer_asked = Some(kind);
                                }
                            }
                        }
                        // OK on a comment or footnote: the caret on its mark, and "Edit" said
                        // after the footnote, not the line [obs, 1087/s6, 1088/s6].
                        ListKind::DocumentNotes(note) => {
                            // A note by its mark, as the document shows it: "end note
                            // reference: i Renewals are free." [obs, 1090/s6].
                            let shown = self
                                .listed_notes
                                .get(entry.node_id)
                                .map(|n| format!("{} {}", n.shown_mark(), n.text.trim()));
                            let shown = shown.as_deref().unwrap_or(entry.text.as_str());
                            self.document_list_says = Some(match note {
                                crate::event::NoteKind::Comment => String::new(),
                                // A revision by its text and what it changed: "nine Deleted |
                                // Edit" [obs, 1107/s8].
                                crate::event::NoteKind::Revision => self
                                    .listed_notes
                                    .get(entry.node_id)
                                    .map(|n| {
                                        let mark = n
                                            .revision
                                            .and_then(
                                                crate::apps::windows::word::revision_kind_mark,
                                            )
                                            .unwrap_or_default();
                                        format!("{} {mark}", n.text.trim()).trim().to_owned()
                                    })
                                    .unwrap_or_default(),
                                crate::event::NoteKind::Footnote => {
                                    msg::word_footnote_reference(shown).render()
                                }
                                crate::event::NoteKind::Endnote => {
                                    msg::word_endnote_reference(shown).render()
                                }
                                // A bookmark by the text it marks: "three weeks | Edit" [obs,
                                // 1091/s9].
                                crate::event::NoteKind::Bookmark => self
                                    .listed_notes
                                    .get(entry.node_id)
                                    .map(|n| n.text.trim().to_owned())
                                    .unwrap_or_default(),
                            });
                            actions.move_to_document_note(
                                self.last_text.as_ref(),
                                note,
                                entry.node_id,
                            );
                        }
                        _ => {
                            actions.move_to_paragraph(self.last_text.as_ref(), entry.node_id);
                        }
                    }
                }
                None
            }
            // Activate Link in a document's list: the document follows the link itself, and
            // what opens says itself.
            Outcome::Activate(link) if in_document => {
                close(self);
                if let Some(actions) = &self.actions {
                    actions.activate_document_link(link.node_id);
                }
                None
            }
            Outcome::Send(messages) => {
                if let Some(ui) = &self.ui {
                    messages.into_iter().for_each(|m| ui.send(m));
                }
                None
            }
            // JAWS 2025 after Move To Link (dev box, 2026-10-17): "same page link Reputation
            // for Quality Service | To activate press Enter."
            // A heading's line is read as the virtual cursor reads it, not as H says the
            // heading: "heading level 1 Web App Patterns" [obs, 224/s10].
            // OK in the Word List moves to the word's first line from the virtual cursor's, as
            // W does after it [obs, 560/s6].
            Outcome::Summary(entry) => {
                close(self);
                self.show_word_summary(&entry.sort_key)
            }
            Outcome::MoveTo(entry) if kind == ListKind::Words => {
                close(self);
                self.word_from_list = Some(entry.sort_key);
                let said = self.word_from_list_line(true, true)?;
                self.speech.speak(&said, true);
                Some(said)
            }
            Outcome::MoveTo(entry) => {
                close(self);
                let said = self.cursor.jump_to_element_offset(entry.offset);
                let speech = match kind {
                    ListKind::Links => format!("{said}\n{}", msg::hint_activate_enter()),
                    // Select a Table enters the table, as T does [obs, 257/s3].
                    ListKind::Elements(crate::element_lists::ElementList::Tables) => self
                        .cursor
                        .enter_table_at(entry.offset)
                        .unwrap_or_else(|| self.cursor.current_line()),
                    // Move To region start is silent; Say Line then reads the region's start
                    // [obs, 256/s3, s4].
                    ListKind::Regions => {
                        self.cursor.jump_to_element_offset(entry.offset);
                        return None;
                    }
                    // The controls list moves to a combo box as C does, its label said with it:
                    // "Home branch Combo box collapsed Northside" [obs, 248/s3].
                    ListKind::Elements(crate::element_lists::ElementList::Controls) => {
                        self.cursor.control_speech_here()
                    }
                    ListKind::Headings
                    | ListKind::FormFields
                    | ListKind::DocumentFields
                    | ListKind::DocumentNotes(_)
                    | ListKind::NotesToView
                    | ListKind::SheetList(_)
                    | ListKind::Features
                    | ListKind::Words
                    | ListKind::Elements(_) => self.cursor.current_line(),
                };
                self.speech.speak(&speech, true);
                Some(speech)
            }
            Outcome::Activate(link) => {
                close(self);
                self.cursor.jump_to_element_offset(link.offset);
                let activated = self.execute(crate::command::Command::Enter);
                if !link.same_page {
                    return activated;
                }
                // A link within the page keeps the focus, which coming back from the dialog
                // is taken quietly: JAWS says its usage hint alone [obs, 222/s10].
                let hint = msg::hint_activate_enter().to_string();
                self.speech.speak(&hint, false);
                Some(match activated.filter(|s| !s.trim().is_empty()) {
                    Some(said) => format!("{said}\n{hint}"),
                    None => hint,
                })
            }
            // OK in Select a Form Field: JAWS names the page, reads the field as the virtual
            // cursor does, and says it again as it takes the focus ("Web App Patterns | Outline
            // button menu | Outline button menu"); an edit then takes typing [obs, 225/s10,
            // 228/s9, s10].
            Outcome::Focus(field) => {
                close(self);
                // The named groups around the control that the virtual cursor was outside of.
                let from = self.cursor.offset;
                self.list_entered_groups = self
                    .cursor
                    .buffer
                    .ancestors(field.node_id)
                    .into_iter()
                    .rev()
                    .filter(|a| {
                        matches!(a.role, crate::aom::Role::Group | crate::aom::Role::TabList)
                    })
                    .filter_map(|a| {
                        let name = a.name.as_deref().map(str::trim).filter(|n| !n.is_empty())?;
                        let span = self.cursor.buffer.bounded.iter().find(|&&(s, e)| {
                            self.cursor
                                .buffer
                                .line_at_offset(s)
                                .is_some_and(|l| l.node_id == a.id)
                                && e >= s
                        })?;
                        (from < span.0 || from > span.1).then(|| name.to_owned())
                    })
                    .collect();
                self.cursor.jump_to_element_offset(field.offset);
                // JAWS's element lists leave the speech to the focus: the page and its regions
                // as the focus enters them, then the control [obs, 247/s3, 250/s3].
                let element_list = matches!(kind, ListKind::Elements(_));
                let node = self.cursor.buffer.node(field.node_id);
                let typed_in = node.is_some_and(|n| {
                    matches!(n.role, crate::aom::Role::Edit | crate::aom::Role::ComboBox)
                });
                let edit = node.is_some_and(|n| n.role == crate::aom::Role::Edit);
                let focused = match (node, &self.actions) {
                    (Some(node), Some(actions)) => actions.focus(node),
                    _ => false,
                };
                tracing::debug!(node = field.node_id, focused, "form field list: focus");
                if focused {
                    self.dialog_focus_target = Some(field.node_id);
                    // A field one types in is then in forms mode, as after Enter on it, whatever
                    // Auto Forms Mode says [obs, 228/s10]; JAWS's documentation: "Forms mode is
                    // automatically enabled" (Surf's Up, Using Forms with JAWS). Without Enter's
                    // "Type in text." [obs, 249/s3, 228/s9].
                    if typed_in {
                        self.forms_mode = true;
                        self.cursor.forms_mode_active = true;
                        self.forms_mode_by_list = element_list;
                    }
                    if !element_list {
                        self.page_said_for = Some(field.node_id);
                    }
                }
                if element_list {
                    // The page's own focus on the way to the control is not the user's: the
                    // control's focus enters the page.
                    self.last_page_focus = None;
                    return None;
                }
                // Select a Form Field says the page and the line it left the virtual cursor on,
                // then an edit's line in forms mode, before the field's own focus [obs, 228/s9].
                let speech = [
                    self.cursor.buffer.document_title().map(str::to_owned),
                    Some(self.cursor.current_line()),
                    (focused && edit)
                        .then(|| self.field_forms_line(field.node_id))
                        .flatten(),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("\n");
                self.speech.speak(&speech, true);
                Some(speech)
            }
            // JAWS opens the chosen list in place of Virtual HTML Features [obs, 226/s3]: the
            // new dialog is shown before this one goes, so the focus moves from one to the
            // other. A list Okur has not built yet is said so.
            Outcome::Open(feature) => {
                let features = self.list_dialog.take();
                let opened = match feature.list() {
                    Some(kind) => self.open_list(kind),
                    None => {
                        let speech = msg::html_feature_not_yet(feature.name().as_str()).to_string();
                        self.speech.speak(&speech, true);
                        Some(speech)
                    }
                };
                if let (Some(features), Some(ui)) = (features, &self.ui) {
                    ui.send(crate::ui::ToUi::DialogClose { id: features.id });
                }
                if self.list_dialog.is_none() {
                    self.returning_from_dialog();
                }
                opened
            }
            // Select a Form Field gives the page back named, or, with the virtual cursor on a
            // field, that field as E reads it ("lamp Edit blank"), else on a line of the page,
            // that line; the Links List quietly [obs, 227/s11, 688/s19, 223/s3].
            Outcome::Closed => {
                self.list_dialog = None;
                self.app_mode = self.list_return_mode;
                self.returning_from_dialog();
                // An application that says its focus itself says it (Excel's cell, "22 | B 4"
                // [obs, 521/s10]).
                let again = self
                    .list_return_focus
                    .take()
                    .as_ref()
                    .filter(|_| in_document)
                    .and_then(|focus| self.apps.iter().find_map(|app| app.focus_again(focus)));
                if let Some(again) = again {
                    self.speech.speak(&again, true);
                    return Some(again);
                }
                // Word's document is named as the focus comes back to it: "Edit" [obs, 519/s4].
                if in_document {
                    let edit = crate::linearize::role_marker(msg::role_edit().as_str());
                    self.speech.speak(&edit, true);
                    return Some(edit);
                }
                if kind != ListKind::FormFields {
                    return None;
                }
                let buffer = &self.cursor.buffer;
                let offset = self.cursor.offset;
                let page = if let Some(field) = buffer.find_form_control_node(offset) {
                    crate::speech::SpeechFormatter::format_control_focus(field, |_| true)
                } else if buffer
                    .line_at_offset(offset)
                    .is_none_or(|line| line.role == crate::aom::Role::Document)
                {
                    self.page_on_return()?
                } else {
                    self.line_here()
                };
                self.speech.speak(&page, true);
                Some(page)
            }
        }
    }

    /// Shows the open Virtual Viewer in its window, when the settings show it on screen.
    pub(crate) fn show_viewer_window(&self) {
        if self.settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            return;
        }
        if let (Some(ui), Some(viewer)) = (&self.ui, &self.viewer) {
            ui.send(crate::ui::ToUi::ViewerShow {
                title: viewer
                    .title()
                    .map_or_else(|| msg::viewer_title().to_string(), str::to_owned),
                lines: viewer.lines(),
                mark: viewer.mark(),
            });
        }
    }

    /// Moves the viewer window's mark to what the viewer read last.
    fn mark_viewer_window(&self) {
        if self.settings.viewer_display != crate::settings::ViewerDisplay::OnScreen {
            return;
        }
        if let (Some(ui), Some(viewer)) = (&self.ui, &self.viewer) {
            ui.send(crate::ui::ToUi::ViewerMark {
                mark: viewer.mark(),
            });
        }
    }

    /// Closes the viewer's window, if one is showing.
    fn close_viewer_window(&self) {
        if let Some(ui) = &self.ui {
            ui.send(crate::ui::ToUi::ViewerClose);
        }
    }
}

/// The main window's menu items' ids.
const MENU_BASICS: &str = "basics";
const MENU_VOICE_ADJUSTMENT: &str = "voice_adjustment";
const MENU_VOICE_PROFILE: &str = "voice_profile";
const MENU_APPLICATIONS: &str = "manage_applications";
const MENU_RESTORE: &str = "restore_factory";
const MENU_NOTIFICATIONS: &str = "notification_history";
const MENU_SETTINGS_CENTER: &str = "settings_center";
const MENU_ABOUT: &str = "about";
const MENU_STARTUP_WIZARD: &str = "startup_wizard";
const MENU_NO_SPEECH: &str = "no_speech";
const MENU_PROFILE_USE_DEFAULT: &str = "profile_use_default";
/// A profile's item in Default and in For <application>: the prefix, then its name.
const MENU_PROFILE_DEFAULT: &str = "profile_default:";
const MENU_PROFILE_APPLICATION: &str = "profile_application:";

/// The application Okur's own windows are in (okur-ui's process), as the settings name it.
const OKUR_WINDOWS: &str = "okur-ui";

/// The dialog an event from one of Okur's windows is about.
fn dialog_of(message: &crate::ui::FromUi) -> Option<u32> {
    use crate::ui::FromUi;
    match message {
        FromUi::DialogChanged { id, .. }
        | FromUi::DialogPressed { id, .. }
        | FromUi::DialogKey { id, .. }
        | FromUi::DialogMenuChosen { id, .. }
        | FromUi::DialogClosed { id } => Some(*id),
        FromUi::ViewerDismissed | FromUi::MainMenuCommand { .. } => None,
    }
}
