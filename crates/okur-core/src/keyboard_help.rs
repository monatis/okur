//! Keyboard Help's catalog (Insert+1): while it is on, a key is not run; Okur says the key's
//! name, then a line on what the key does, and a longer one when the key is pressed twice
//! quickly. In JAWS: "Insert+UpArrow", then what Say Line does; "Tab", then that the key goes
//! to the application [obs, 509].
//!
//! The key names are JAWS's key notation ("Insert+UpArrow", "Control+Insert+F", "NumPad5"):
//! they are how its users name keys. The descriptions are Okur's own words, from the catalog
//! (`locales/en-US/keyboard-help.ftl`); JAWS's are Freedom Scientific's and are not copied.
//!
//! This module only names keys and describes commands. Turning Keyboard Help on and off, and
//! holding keys back from their commands while it is on, is the engine's.

use crate::command::{Command, Element};
use crate::element_lists::ElementList;
use crate::input::{Key, KeyEvent};
use crate::messages::{msg, Message, StaticMessage};

/// The keys of `event` in JAWS's key notation: the modifiers held, in the order Control, Alt,
/// Shift, Windows, the Okur key, then the key pressed, joined by "+" ("Control+Insert+F"). A
/// modifier pressed alone is named alone ("Shift").
///
/// The Okur key is named as the key it is: CapsLock when CapsLock is held as the Okur key (the
/// laptop layout), Insert otherwise. `event` may be resolved by the keyboard settings
/// ([`crate::settings::KeyboardSettings::resolve`]) or not.
pub fn key_name(event: &KeyEvent) -> String {
    let m = &event.modifiers;
    let key = event.key;
    // A modifier key is named once, as the key pressed, even while its own flag is set.
    let okur_key = if m.caps_lock && key != Key::CapsLock {
        Some(msg::key_caps_lock())
    } else if m.insert && okur_flag_is_another_key(event) {
        Some(msg::key_insert())
    } else {
        None
    };
    let held = [
        (m.ctrl && key != Key::Control).then(msg::key_control),
        (m.alt && key != Key::Alt).then(msg::key_alt),
        (m.shift && key != Key::Shift).then(msg::key_shift),
        (m.win && key != Key::Windows).then(msg::key_windows),
        okur_key,
    ];
    held.iter()
        .rev()
        .flatten()
        .fold(main_key_name(event), |keys, modifier| {
            msg::key_chord(*modifier, keys).render()
        })
}

/// Whether `modifiers.insert` stands for a key other than the one pressed: Insert pressed, or
/// the key that is the Okur key pressed, sets the flag by itself.
fn okur_flag_is_another_key(event: &KeyEvent) -> bool {
    match event.key {
        Key::Insert | Key::ScrollLock => false,
        // CapsLock pressed while Insert is held: unless CapsLock is the Okur key itself.
        Key::CapsLock => !event.modifiers.caps_lock,
        _ => true,
    }
}

/// The name of the key pressed, without its modifiers.
fn main_key_name(event: &KeyEvent) -> String {
    let name = match event.key {
        Key::UpArrow => msg::key_up_arrow(),
        Key::DownArrow => msg::key_down_arrow(),
        Key::LeftArrow => msg::key_left_arrow(),
        Key::RightArrow => msg::key_right_arrow(),
        Key::Home => msg::key_home(),
        Key::End => msg::key_end(),
        Key::PageUp => msg::key_page_up(),
        Key::PageDown => msg::key_page_down(),
        Key::Tab => msg::key_tab(),
        Key::Enter => msg::key_enter(),
        Key::Escape => msg::key_escape(),
        Key::Space => msg::key_space(),
        Key::Backspace => msg::key_backspace(),
        Key::Delete => msg::key_delete(),
        Key::Insert => msg::key_insert(),
        Key::CapsLock => msg::key_caps_lock(),
        Key::ScrollLock => msg::key_scroll_lock(),
        Key::Semicolon => msg::key_semicolon(),
        Key::Apostrophe => msg::key_apostrophe(),
        Key::Comma => msg::key_comma(),
        Key::Period => msg::key_period(),
        Key::Minus => msg::key_dash(),
        Key::Equals => msg::key_equals(),
        Key::Slash => msg::key_slash(),
        Key::Backslash => msg::key_backslash(),
        Key::Numpad5 => msg::key_numpad_five(),
        Key::NumpadPlus => msg::key_numpad_plus(),
        Key::NumpadMinus => msg::key_numpad_minus(),
        Key::NumpadSlash => msg::key_numpad_slash(),
        Key::NumpadStar => msg::key_numpad_star(),
        Key::Control => msg::key_control(),
        Key::Alt => msg::key_alt(),
        Key::Shift => msg::key_shift(),
        Key::Windows => msg::key_windows(),
        Key::Alpha(letter) => {
            return msg::key_character(letter.to_uppercase().to_string()).render();
        }
        Key::Digit(digit) => return msg::key_character(digit.to_string()).render(),
        Key::F(number) => return msg::key_function(number).render(),
        // A key Okur has no name for: the character it types, else its code.
        Key::Other(code) => {
            return match event.text.filter(|c| !c.is_whitespace() && !c.is_control()) {
                Some(c) => msg::key_character(c.to_uppercase().to_string()).render(),
                None => msg::key_unnamed(code).render(),
            };
        }
    };
    name.to_string()
}

/// What a key Okur does not use does: it goes to the application.
pub fn passes() -> String {
    msg::key_help_passes().to_string()
}

/// What `command` does, in Okur's words; with `twice`, the longer description, for a key
/// pressed twice quickly (the short one where there is no longer one).
pub fn description(command: &Command, twice: bool) -> String {
    let (short, long) = descriptions(command);
    match long {
        Some(long) if twice => long.render(),
        _ => short.render(),
    }
}

/// A command's short description, and its longer one if it has one.
type Descriptions = (Message, Option<Message>);

fn one(short: impl Into<Message>) -> Descriptions {
    (short.into(), None)
}

fn two(short: impl Into<Message>, long: impl Into<Message>) -> Descriptions {
    (short.into(), Some(long.into()))
}

/// Every command's descriptions. No arm is a catch-all, so a new command does not build
/// until it is described.
fn descriptions(command: &Command) -> Descriptions {
    use Command::*;
    match command {
        // Reading where the cursor is.
        SayAll => two(msg::command_say_all(), msg::command_say_all_long()),
        SkimRead => one(msg::command_skim_read()),
        SkimReadDialog => one(msg::command_skim_read_dialog()),
        SayLine => two(msg::command_say_line(), msg::command_say_line_long()),
        SayWord => two(msg::command_say_word(), msg::command_say_word_long()),
        SayCharacter => two(
            msg::command_say_character(),
            msg::command_say_character_long(),
        ),
        SayCharacterPhonetic => one(msg::command_say_character_phonetic()),
        SayCharacterValue => two(
            msg::command_say_character_value(),
            msg::command_say_character_value_long(),
        ),
        SpellWord => one(msg::command_spell_word()),
        SpellLine => one(msg::command_spell_line()),

        // Moving, then reading.
        SayNextLine => one(msg::command_say_next_line()),
        SayPriorLine => one(msg::command_say_prior_line()),
        SayNextWord => one(msg::command_say_next_word()),
        SayPriorWord => one(msg::command_say_prior_word()),
        SayNextCharacter => one(msg::command_say_next_character()),
        SayPriorCharacter => one(msg::command_say_prior_character()),
        SayNextParagraph => one(msg::command_say_next_paragraph()),
        SayPriorParagraph => one(msg::command_say_prior_paragraph()),
        SayParagraph => one(msg::command_say_paragraph()),
        SaySentence => one(msg::command_say_sentence()),
        SayNextSentence => one(msg::command_say_next_sentence()),
        SayPriorSentence => one(msg::command_say_prior_sentence()),
        NextParagraphOrArticles => one(msg::command_next_paragraph_or_articles()),
        LineStart => one(msg::command_line_start()),
        LineEnd => one(msg::command_line_end()),
        PageUp => one(msg::command_page_up()),
        PageDown => one(msg::command_page_down()),
        TopOfFile => one(msg::command_top_of_file()),
        BottomOfFile => one(msg::command_bottom_of_file()),

        // Quick navigation.
        MoveToNext(element) => {
            let name = element_name(*element);
            two(
                msg::command_move_to_next(name.as_str()),
                msg::command_move_to_next_long(name),
            )
        }
        MoveToPrior(element) => {
            let name = element_name(*element);
            two(
                msg::command_move_to_prior(name.as_str()),
                msg::command_move_to_prior_long(name),
            )
        }
        MoveToMainRegion => one(msg::command_move_to_main_region()),
        NextNonLinkText => two(
            msg::command_next_non_link_text(),
            msg::command_next_non_link_text_long(),
        ),
        PriorNonLinkText => one(msg::command_prior_non_link_text()),
        NextSameElement => one(msg::command_next_same_element()),
        PriorSameElement => one(msg::command_prior_same_element()),
        NextDifferentElement => one(msg::command_next_different_element()),
        PriorDifferentElement => one(msg::command_prior_different_element()),

        // Tables, headings and form fields.
        TableCellRight => one(msg::command_table_cell_right()),
        TableCellLeft => one(msg::command_table_cell_left()),
        TableCellDown => one(msg::command_table_cell_down()),
        TableCellUp => one(msg::command_table_cell_up()),
        TableFirstCell => one(msg::command_table_first_cell()),
        TableLastCell => one(msg::command_table_last_cell()),
        MoveToStartOfRow => one(msg::command_move_to_start_of_row()),
        MoveToEndOfRow => one(msg::command_move_to_end_of_row()),
        MoveToTopOfColumn => one(msg::command_move_to_top_of_column()),
        MoveToBottomOfColumn => one(msg::command_move_to_bottom_of_column()),
        SayCurrentCell => one(msg::command_say_current_cell()),
        SayColumnHeader => one(msg::command_say_column_header()),
        SayRowHeader => one(msg::command_say_row_header()),
        FirstHeading => one(msg::command_first_heading()),
        LastHeading => one(msg::command_last_heading()),
        FirstHeadingAtLevel(level) => one(msg::command_first_heading_at_level(*level)),
        LastHeadingAtLevel(level) => one(msg::command_last_heading_at_level(*level)),
        FirstFormField => one(msg::command_first_form_field()),
        LastFormField => one(msg::command_last_form_field()),
        SayCurrentRow => one(msg::command_say_current_row()),
        SayCurrentColumn => one(msg::command_say_current_column()),
        ReadNextRow => one(msg::command_read_next_row()),
        ReadPriorRow => one(msg::command_read_prior_row()),
        ReadNextColumn => one(msg::command_read_next_column()),
        ReadPriorColumn => one(msg::command_read_prior_column()),
        StepToStartOfElement => one(msg::command_step_to_start_of_element()),
        StepToEndOfElement => one(msg::command_step_to_end_of_element()),
        FlowTo => one(msg::command_flow_to()),
        FlowFrom => one(msg::command_flow_from()),

        // A page's controls.
        NextTabStop => one(msg::command_next_tab_stop()),
        PriorTabStop => one(msg::command_prior_tab_stop()),
        Activate => two(msg::command_activate(), msg::command_activate_long()),
        Enter => two(msg::command_enter(), msg::command_enter_long()),
        Escape => one(msg::command_escape()),
        ExpandComboBox => one(msg::command_expand_combo_box()),
        CollapseComboBox => one(msg::command_collapse_combo_box()),
        EnterMathViewer => one(msg::command_enter_math_viewer()),
        EnterModal => one(msg::command_enter_modal()),
        ExitModal => one(msg::command_exit_modal()),

        // Selecting.
        SelectNextCharacter => one(msg::command_select_next_character()),
        SelectPriorCharacter => one(msg::command_select_prior_character()),
        SelectNextLine => one(msg::command_select_next_line()),
        SelectPriorLine => one(msg::command_select_prior_line()),
        SelectNextWord => one(msg::command_select_next_word()),
        SelectPriorWord => one(msg::command_select_prior_word()),
        SelectToLineEnd => one(msg::command_select_to_line_end()),
        SelectToLineStart => one(msg::command_select_to_line_start()),
        SelectToBottom => one(msg::command_select_to_bottom()),
        SelectToTop => one(msg::command_select_to_top()),
        SelectNextPage => one(msg::command_select_next_page()),
        SelectPriorPage => one(msg::command_select_prior_page()),
        SelectAll => one(msg::command_select_all()),
        SelectHtmlElement => two(
            msg::command_select_html_element(),
            msg::command_select_html_element_long(),
        ),
        CopySelection => one(msg::command_copy_selection()),
        SaySelectedText => two(
            msg::command_say_selected_text(),
            msg::command_say_selected_text_long(),
        ),
        SpellSelectedText => one(msg::command_spell_selected_text()),

        // Okur's modes and the voice.
        LayerStart => two(msg::command_layer_start(), msg::command_layer_start_long()),
        NavigationModeToggle => one(msg::command_navigation_mode_toggle()),
        QuickKeysSuspendResume => two(
            msg::command_quick_keys_suspend_resume(),
            msg::command_quick_keys_suspend_resume_long(),
        ),
        VirtualCursorToggle => two(
            msg::command_virtual_cursor_toggle(),
            msg::command_virtual_cursor_toggle_long(),
        ),
        RefreshScreen => one(msg::command_refresh_screen()),
        RateFaster => one(msg::command_rate_faster()),
        RateSlower => one(msg::command_rate_slower()),

        // Lines of the window and of the cursor.
        SayTopLineOfWindow => one(msg::command_say_top_line_of_window()),
        SayBottomLineOfWindow => one(msg::command_say_bottom_line_of_window()),
        SayToCursor => two(
            msg::command_say_to_cursor(),
            msg::command_say_to_cursor_long(),
        ),
        SayFromCursor => two(
            msg::command_say_from_cursor(),
            msg::command_say_from_cursor_long(),
        ),
        SpellToCursor => one(msg::command_spell_to_cursor()),
        SpellFromCursor => one(msg::command_spell_from_cursor()),

        // Windows, dialogs and the application.
        SayDefaultButton => two(
            msg::command_say_default_button(),
            msg::command_say_default_button_long(),
        ),
        SayConfiguration => one(msg::command_say_configuration()),
        SayAppVersion => one(msg::command_say_app_version()),
        OkurCursor => one(msg::command_okur_cursor()),
        PcCursor => one(msg::command_pc_cursor()),
        RouteOkurToPc => two(
            msg::command_route_okur_to_pc(),
            msg::command_route_okur_to_pc_long(),
        ),
        SayActiveCursor => one(msg::command_say_active_cursor()),
        SayHotKey => one(msg::command_say_hot_key()),
        SayListColumn(column) => one(msg::command_say_list_column(*column)),
        SayLinkAddress => one(msg::command_say_link_address()),
        RouteVirtualToPc => one(msg::command_route_virtual_to_pc()),
        RoutePcToVirtual => one(msg::command_route_pc_to_virtual()),
        SelectAFrame => one(msg::command_select_a_frame()),
        LeftMouseClick => one(msg::command_left_mouse_click()),
        RightMouseClick => one(msg::command_right_mouse_click()),
        LeftMouseButtonLock => one(msg::command_left_mouse_lock()),
        ModifiedLeftMouseClick(_) => one(msg::command_modified_mouse_click()),
        MoveMouse(_) => one(msg::command_move_mouse()),
        RightMouseButtonLock => one(msg::command_right_mouse_lock()),
        RestrictOkurCursor => one(msg::command_restrict_okur_cursor()),
        RoutePcToOkur => one(msg::command_route_pc_to_okur()),
        WindowList => one(msg::command_window_list()),
        SystemTrayList => one(msg::command_system_tray_list()),
        JumpToLine => one(msg::command_jump_to_line()),
        DefineTempPlaceMarker => one(msg::command_define_temp_placemarker()),
        SelectAPlaceMarker => one(msg::command_select_a_placemarker()),
        WordList => one(msg::command_word_list()),
        NextWordFromList => one(msg::command_next_word_from_list()),
        PriorWordFromList => one(msg::command_prior_word_from_list()),
        SpeakPlaceMarker(number) => one(msg::command_speak_placemarker(*number)),
        MoveToPlaceMarkerNumber(number) => one(msg::command_move_to_placemarker(*number)),
        AssignCustomLabel => one(msg::command_assign_custom_label()),
        JumpReturnFromLine => one(msg::command_jump_return_from_line()),

        // Help.
        ScreenSensitiveHelp => two(
            msg::command_screen_sensitive_help(),
            msg::command_screen_sensitive_help_long(),
        ),
        HotKeyHelp => two(
            msg::command_hot_key_help(),
            msg::command_hot_key_help_long(),
        ),
        WindowsKeysHelp => one(msg::command_windows_keys_help()),

        // Finding.
        VirtualFind => two(
            msg::command_virtual_find(),
            msg::command_virtual_find_long(),
        ),
        OkurFind => two(msg::command_okur_find(), msg::command_okur_find_long()),
        FindNext => one(msg::command_find_next()),
        FindPrior => one(msg::command_find_prior()),
        Find(text) => one(msg::command_find(text)),

        // Lists of a page's elements.
        SelectAHeading => one(msg::command_select_a_heading()),
        SelectALink => one(msg::command_select_a_link()),
        SelectAFormField => one(msg::command_select_a_form_field()),
        VirtualHtmlFeatures => one(msg::command_virtual_html_features()),
        SelectElement(list) => one(msg::command_select_element(elements_name(*list))),
        SelectARegion => one(msg::command_select_a_region()),
        JumpToHeading(name) => one(msg::command_jump_to_heading(name)),
        JumpToLink(name) => one(msg::command_jump_to_link(name)),
        JumpToFormField(name) => one(msg::command_jump_to_form_field(name)),

        // Windows, the focus and the time.
        PassKeyThrough => one(msg::command_pass_key_through()),
        SayWindow => one(msg::command_say_window()),
        SayCurrentFocus => one(msg::command_say_current_focus()),
        SayWindowTitle => one(msg::command_say_window_title()),
        SaySystemTime => two(
            msg::command_say_system_time(),
            msg::command_say_system_time_long(),
        ),
        SaySystemTimeWithSeconds => two(
            msg::command_say_system_time_with_seconds(),
            msg::command_say_system_time_with_seconds_long(),
        ),
        SaySystemDate => one(msg::command_say_system_date()),
        SayWeekNumber => one(msg::command_say_week_number()),

        // The Virtual Viewer, fonts and colors.
        VirtualizeWindow => one(msg::command_virtualize_window()),
        VirtualizeControl => one(msg::command_virtualize_control()),
        SayFont => two(msg::command_say_font(), msg::command_say_font_long()),
        SayFontInViewer => one(msg::command_say_font_in_viewer()),
        SayColor => one(msg::command_say_color()),
        KeyboardHelp => one(msg::command_keyboard_help()),

        // Okur's settings, and Okur itself.
        QuickSettings | SettingsCenter => one(msg::command_open_settings()),
        SetPunctuationLevel => one(msg::command_set_punctuation_level()),
        NextTypingEcho => one(msg::command_next_typing_echo()),
        ShutDown => two(msg::command_shut_down(), msg::command_shut_down_long()),
    }
}

/// What a quick key moves to, as said in "Moves to the next heading."
fn element_name(element: Element) -> String {
    let name = match element {
        Element::Heading => msg::element_kind_heading(),
        Element::HeadingLevel(level) => return msg::element_kind_heading_level(level).render(),
        Element::Link => msg::element_kind_link(),
        Element::VisitedLink => msg::element_kind_visited_link(),
        Element::UnvisitedLink => msg::element_kind_unvisited_link(),
        Element::Table => msg::element_kind_table(),
        Element::Article => msg::element_kind_article(),
        Element::OnMouseOver => msg::element_kind_on_mouse_over(),
        Element::OnClick => msg::element_kind_on_click(),
        Element::Tab => msg::element_kind_tab(),
        Element::PlaceMarker => msg::element_kind_placemarker(),
        Element::Graphic => msg::element_kind_graphic(),
        Element::RadioButton => msg::element_kind_radio_button(),
        Element::Math => msg::element_kind_math(),
        Element::FormField => msg::element_kind_form_field(),
        Element::Button => msg::element_kind_button(),
        Element::Edit => msg::element_kind_edit(),
        Element::CheckBox => msg::element_kind_check_box(),
        Element::ComboBox => msg::element_kind_combo_box(),
        Element::List => msg::element_kind_list(),
        Element::ListItem => msg::element_kind_list_item(),
        Element::Landmark => msg::element_kind_region(),
        Element::Frame => msg::element_kind_frame(),
        Element::Division => msg::element_kind_division(),
        Element::Separator => msg::element_kind_separator(),
        Element::MailLink => msg::element_kind_mail_link(),
    };
    name.to_string()
}

/// What an element list lists, as said in "Lists the page's buttons, to move to one."
fn elements_name(list: ElementList) -> StaticMessage {
    match list {
        ElementList::RadioButtons => msg::elements_radio_buttons(),
        ElementList::Buttons => msg::elements_buttons(),
        ElementList::Controls => msg::elements_controls(),
        ElementList::Edits => msg::elements_edits(),
        ElementList::CheckBoxes => msg::elements_check_boxes(),
        ElementList::Graphics => msg::elements_graphics(),
        ElementList::Lists => msg::elements_lists(),
        ElementList::Articles => msg::elements_articles(),
        ElementList::Paragraphs => msg::elements_paragraphs(),
        ElementList::BlockQuotes => msg::elements_block_quotes(),
        ElementList::Tables => msg::elements_tables(),
        ElementList::Divisions => msg::elements_divisions(),
        ElementList::Tabs => msg::elements_tabs(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Modifiers;

    fn press(key: Key, set: impl FnOnce(&mut Modifiers)) -> KeyEvent {
        let mut modifiers = Modifiers::none();
        set(&mut modifiers);
        KeyEvent::new(key, modifiers, true)
    }

    /// The names JAWS said in Keyboard Help: "Insert+UpArrow", "Tab", "Insert+T" [obs, 509].
    #[test]
    fn keys_are_named_in_the_key_maps_notation() {
        let cases = [
            (press(Key::UpArrow, |m| m.insert = true), "Insert+UpArrow"),
            (press(Key::Tab, |_| {}), "Tab"),
            (press(Key::Alpha('t'), |m| m.insert = true), "Insert+T"),
            (
                press(Key::Alpha('f'), |m| {
                    m.ctrl = true;
                    m.insert = true;
                }),
                "Control+Insert+F",
            ),
            (
                press(Key::Digit(1), |m| {
                    m.insert = true;
                    m.win = true;
                    m.shift = true;
                    m.alt = true;
                    m.ctrl = true;
                }),
                "Control+Alt+Shift+Windows+Insert+1",
            ),
            (press(Key::F(12), |m| m.insert = true), "Insert+F12"),
            (press(Key::Numpad5, |_| {}), "NumPad5"),
            (
                press(Key::NumpadMinus, |m| m.insert = true),
                "Insert+NumPadMinus",
            ),
            (press(Key::Minus, |m| m.shift = true), "Shift+Dash"),
            (press(Key::Space, |m| m.insert = true), "Insert+SpaceBar"),
            (press(Key::PageDown, |m| m.insert = true), "Insert+PageDown"),
            (press(Key::Comma, |m| m.shift = true), "Shift+Comma"),
        ];
        for (event, name) in cases {
            assert_eq!(key_name(&event), name, "{event:?}");
        }
    }

    /// CapsLock held as the Okur key (the laptop layout, resolved or not) is named CapsLock;
    /// Insert, in the laptop layout too, Insert.
    #[test]
    fn the_okur_key_is_named_as_the_key_held() {
        let resolved_caps = press(Key::Alpha('t'), |m| {
            m.insert = true;
            m.caps_lock = true;
        });
        assert_eq!(key_name(&resolved_caps), "CapsLock+T");
        let caps_only = press(Key::Alpha('t'), |m| m.caps_lock = true);
        assert_eq!(key_name(&caps_only), "CapsLock+T");
        let insert = press(Key::Alpha('t'), |m| m.insert = true);
        assert_eq!(key_name(&insert), "Insert+T");
    }

    /// A modifier pressed alone is named once, whatever flags its own press sets.
    #[test]
    fn a_modifier_alone_is_named_alone() {
        let cases = [
            (press(Key::Shift, |m| m.shift = true), "Shift"),
            (press(Key::Control, |m| m.ctrl = true), "Control"),
            (press(Key::Alt, |m| m.alt = true), "Alt"),
            (press(Key::Windows, |m| m.win = true), "Windows"),
            (press(Key::Insert, |m| m.insert = true), "Insert"),
            (
                press(Key::CapsLock, |m| {
                    m.insert = true;
                    m.caps_lock = true;
                }),
                "CapsLock",
            ),
            (press(Key::Shift, |m| m.ctrl = true), "Control+Shift"),
            (press(Key::CapsLock, |m| m.insert = true), "Insert+CapsLock"),
        ];
        for (event, name) in cases {
            assert_eq!(key_name(&event), name, "{event:?}");
        }
    }

    /// A key without a name of its own: the character it types, else its code.
    #[test]
    fn an_unnamed_key_is_named_by_its_character_or_code() {
        let mut bracket = press(Key::Other(0xDB), |_| {});
        bracket.text = Some('[');
        assert_eq!(key_name(&bracket), "[");
        let unknown = press(Key::Other(0xFF), |m| m.insert = true);
        assert_eq!(key_name(&unknown), "Insert+Key 255");
    }

    #[test]
    fn descriptions_are_okurs_words() {
        assert_eq!(
            description(&Command::SayLine, false),
            "Reads the line at the cursor."
        );
        assert_eq!(
            description(&Command::SayLine, true),
            "Reads the line at the cursor. Pressed twice quickly, it spells the line."
        );
        assert_eq!(
            description(&Command::MoveToNext(Element::Heading), false),
            "Moves to the next heading."
        );
        assert_eq!(
            description(&Command::MoveToPrior(Element::HeadingLevel(2)), false),
            "Moves to the previous heading at level 2."
        );
        assert_eq!(
            description(&Command::SelectElement(ElementList::Buttons), false),
            "Lists the page's buttons, to move to one."
        );
        assert_eq!(
            description(&Command::SayListColumn(10), false),
            "Reads column 10 of the list item that has the focus, with its header."
        );
        // Without a longer description, twice says the short one.
        assert_eq!(
            description(&Command::SayNextLine, true),
            description(&Command::SayNextLine, false)
        );
        assert_eq!(passes(), "Goes to the application.");
    }

    /// Every command has a description: one sentence or more, ending with a period; the longer
    /// one starts with the short one.
    #[test]
    fn every_command_is_described() {
        use Command::*;
        let mut commands = vec![
            SayAll,
            SkimRead,
            SkimReadDialog,
            SayLine,
            SayWord,
            SayCharacter,
            SayCharacterPhonetic,
            SayCharacterValue,
            SpellWord,
            SpellLine,
            SayNextLine,
            SayPriorLine,
            SayNextWord,
            SayPriorWord,
            SayNextCharacter,
            SayPriorCharacter,
            SayNextParagraph,
            SayPriorParagraph,
            SayParagraph,
            SaySentence,
            SayNextSentence,
            SayPriorSentence,
            NextParagraphOrArticles,
            LineStart,
            LineEnd,
            PageUp,
            PageDown,
            TopOfFile,
            BottomOfFile,
            MoveToMainRegion,
            NextNonLinkText,
            PriorNonLinkText,
            NextSameElement,
            PriorSameElement,
            NextDifferentElement,
            PriorDifferentElement,
            TableCellRight,
            TableCellLeft,
            TableCellDown,
            TableCellUp,
            TableFirstCell,
            TableLastCell,
            MoveToStartOfRow,
            MoveToEndOfRow,
            MoveToTopOfColumn,
            MoveToBottomOfColumn,
            SayCurrentCell,
            SayColumnHeader,
            SayRowHeader,
            FirstHeading,
            LastHeading,
            FirstHeadingAtLevel(1),
            LastHeadingAtLevel(6),
            FirstFormField,
            LastFormField,
            SayCurrentRow,
            SayCurrentColumn,
            ReadNextRow,
            ReadPriorRow,
            ReadNextColumn,
            ReadPriorColumn,
            StepToStartOfElement,
            StepToEndOfElement,
            FlowTo,
            FlowFrom,
            NextTabStop,
            PriorTabStop,
            Activate,
            Enter,
            Escape,
            ExpandComboBox,
            CollapseComboBox,
            EnterMathViewer,
            EnterModal,
            ExitModal,
            SelectNextCharacter,
            SelectPriorCharacter,
            SelectNextLine,
            SelectPriorLine,
            SelectNextWord,
            SelectPriorWord,
            SelectToLineEnd,
            SelectToLineStart,
            SelectToBottom,
            SelectToTop,
            SelectNextPage,
            SelectPriorPage,
            SelectAll,
            SelectHtmlElement,
            CopySelection,
            SaySelectedText,
            LayerStart,
            NavigationModeToggle,
            QuickKeysSuspendResume,
            VirtualCursorToggle,
            RefreshScreen,
            RateFaster,
            RateSlower,
            SayTopLineOfWindow,
            SayToCursor,
            SayFromCursor,
            SpellSelectedText,
            SpellToCursor,
            SpellFromCursor,
            SayBottomLineOfWindow,
            SayDefaultButton,
            SayConfiguration,
            SayAppVersion,
            OkurCursor,
            PcCursor,
            RouteOkurToPc,
            SayActiveCursor,
            SayHotKey,
            SayListColumn(3),
            SayLinkAddress,
            RouteVirtualToPc,
            RoutePcToVirtual,
            SelectAFrame,
            LeftMouseClick,
            RightMouseClick,
            LeftMouseButtonLock,
            RightMouseButtonLock,
            ModifiedLeftMouseClick(crate::command::ClickModifier::Control),
            MoveMouse(crate::command::MouseStep::Up),
            RoutePcToOkur,
            RestrictOkurCursor,
            WindowList,
            SystemTrayList,
            JumpToLine,
            DefineTempPlaceMarker,
            SelectAPlaceMarker,
            WordList,
            NextWordFromList,
            PriorWordFromList,
            SpeakPlaceMarker(1),
            MoveToPlaceMarkerNumber(1),
            AssignCustomLabel,
            JumpReturnFromLine,
            ScreenSensitiveHelp,
            HotKeyHelp,
            WindowsKeysHelp,
            VirtualFind,
            OkurFind,
            FindNext,
            FindPrior,
            Find("engine".to_owned()),
            SelectAHeading,
            SelectALink,
            SelectAFormField,
            VirtualHtmlFeatures,
            SelectARegion,
            JumpToHeading("Contents".to_owned()),
            JumpToLink("Home".to_owned()),
            JumpToFormField("Search".to_owned()),
            PassKeyThrough,
            SayWindow,
            SayCurrentFocus,
            SayWindowTitle,
            SaySystemTime,
            SaySystemTimeWithSeconds,
            SaySystemDate,
            SayWeekNumber,
            VirtualizeWindow,
            VirtualizeControl,
            SayFont,
            SayFontInViewer,
            SayColor,
            KeyboardHelp,
            QuickSettings,
            SettingsCenter,
            SetPunctuationLevel,
            NextTypingEcho,
            ShutDown,
        ];
        let elements = [
            Element::Heading,
            Element::HeadingLevel(4),
            Element::Link,
            Element::Table,
            Element::Article,
            Element::OnMouseOver,
            Element::OnClick,
            Element::Tab,
            Element::PlaceMarker,
            Element::Graphic,
            Element::RadioButton,
            Element::Math,
            Element::FormField,
            Element::Button,
            Element::Edit,
            Element::CheckBox,
            Element::ComboBox,
            Element::List,
            Element::ListItem,
            Element::Landmark,
            Element::Frame,
            Element::Division,
            Element::Separator,
            Element::MailLink,
            Element::VisitedLink,
            Element::UnvisitedLink,
        ];
        for element in elements {
            commands.push(MoveToNext(element));
            commands.push(MoveToPrior(element));
        }
        let lists = [
            ElementList::RadioButtons,
            ElementList::Buttons,
            ElementList::Controls,
            ElementList::Edits,
            ElementList::CheckBoxes,
            ElementList::Graphics,
            ElementList::Lists,
            ElementList::Articles,
            ElementList::Paragraphs,
            ElementList::BlockQuotes,
            ElementList::Tables,
            ElementList::Divisions,
            ElementList::Tabs,
        ];
        commands.extend(lists.into_iter().map(SelectElement));
        for command in &commands {
            let short = description(command, false);
            let long = description(command, true);
            assert!(
                short.len() > 10 && short.ends_with('.'),
                "{command:?}: {short:?}"
            );
            assert!(
                long.starts_with(&short) && long.ends_with('.'),
                "{command:?}: {long:?}"
            );
            assert!(
                !short.contains('{') && !long.contains('{'),
                "{command:?}: a variable left out: {long:?}"
            );
        }
    }
}
