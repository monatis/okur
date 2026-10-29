use super::*;
use crate::aom::{AomNode, Role};
use crate::event::{LiveRegionEvent, Politeness};
use crate::input::Modifiers;
use crate::settings::{OutputMode, OutputType};
use crate::speech::MockSpeechOutput;

fn create_test_buffer() -> VirtualBuffer {
    let aom = AomNode::new(0, Role::Document)
        .with_name("Interactive Test Document")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Main Title"),
            AomNode::new(2, Role::Paragraph).with_text("This is interactive paragraph."),
            AomNode::new(3, Role::Edit).with_name("Search Input"),
            AomNode::new(4, Role::Button).with_name("Submit Button"),
            {
                let mut cb = AomNode::new(5, Role::CheckBox).with_name("Subscribe Checkbox");
                cb.states.checked = Some(true);
                cb
            },
        ]);
    VirtualBuffer::from_aom(&aom)
}

/// Space on a page's disclosure button opens it: JAWS 2025 says "expanded", then
/// "collapsed" when it closes (dev box, GitHub's "Solutions" menu button).
#[test]
fn a_page_buttons_expanded_state_is_spoken() {
    let mut button = AomNode::new(2, Role::Button).with_name("Solutions");
    button.states.expanded = Some(false);
    let page = AomNode::new(0, Role::Document)
        .with_name("GitHub")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Site"),
            button,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let changed = |expanded: bool| {
        let mut control = FocusEvent::new(Role::Button)
            .with_node_id(2)
            .with_name("Solutions");
        control.in_document = true;
        control.states.expanded = Some(expanded);
        AccessibilityEvent::Change {
            kind: crate::event::ChangeKind::State,
            control,
        }
    };
    assert_eq!(
        engine.handle_accessibility_event(&changed(true)).as_deref(),
        Some(" \u{1d}expanded\u{1e} ")
    );
    assert_eq!(
        engine
            .handle_accessibility_event(&changed(false))
            .as_deref(),
        Some(" \u{1d}collapsed\u{1e} ")
    );
    // The same state again says nothing.
    assert_eq!(engine.handle_accessibility_event(&changed(false)), None);
}

/// Only the focused control's changes are said: Modify Rule opening changes its Rule Type
/// combo box before the focus reaches it, and JAWS says nothing of it [obs, 787/s11]; the
/// focused combo box's own change is said.
#[test]
fn a_change_of_a_control_without_the_focus_is_not_said() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let rules = FocusEvent::new(Role::List).with_name("Rules");
    engine.handle_focus_event(&rules);
    let combo = |value: &str| {
        let mut control = FocusEvent::new(Role::ComboBox).with_name("Rule Type");
        control.value = Some(value.to_owned());
        AccessibilityEvent::Change {
            kind: crate::event::ChangeKind::Value,
            control,
        }
    };
    assert_eq!(engine.handle_accessibility_event(&combo("Contains")), None);
    let mut focused = FocusEvent::new(Role::ComboBox).with_name("Rule Type");
    focused.value = Some("Contains".to_owned());
    engine.handle_focus_event(&focused);
    assert!(engine
        .handle_accessibility_event(&combo("Begins with"))
        .is_some_and(|said| said.contains("Begins with")));
}

/// A check box toggled on a big page: the host builds the page again off the engine
/// (performance review P3). The change is said as when the page is built at once, and the
/// node has the new state at once (the same change again says nothing); the lines keep the
/// old state until the host's build is installed and applied. A build made before a later
/// change is refused, and a page smaller than the host's threshold is built at once.
#[test]
fn a_page_is_built_again_by_the_host_unless_another_change_came() {
    let page = || {
        let mut check = AomNode::new(2, Role::CheckBox).with_name("Subscribe");
        check.states.checked = Some(false);
        AomNode::new(0, Role::Document)
            .with_name("Form")
            .with_children(vec![
                AomNode::new(1, Role::Paragraph).with_text("Choose."),
                check,
            ])
    };
    let engine_from = |nodes: Option<usize>| {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page()));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        if let Some(nodes) = nodes {
            engine.set_page_builds_elsewhere(nodes);
        }
        engine
    };
    let toggled = |checked: bool| {
        let mut control = FocusEvent::new(Role::CheckBox)
            .with_node_id(2)
            .with_name("Subscribe");
        control.in_document = true;
        control.states.checked = Some(checked);
        AccessibilityEvent::Change {
            kind: crate::event::ChangeKind::State,
            control,
        }
    };
    // The lines as said: a check box's state is speech, not text.
    let lines = |engine: &ScreenReaderEngine| {
        engine
            .cursor
            .buffer
            .lines
            .iter()
            .map(|l| l.speech.clone())
            .collect::<Vec<_>>()
    };
    let build =
        |page: std::sync::Arc<AomNode>| VirtualBuffer::from_aom_owned(AomNode::clone(&page));

    let mut here = engine_from(None);
    let mut elsewhere = engine_from(Some(0));
    let said = here.handle_accessibility_event(&toggled(true));
    assert!(said.is_some());
    assert_eq!(elsewhere.handle_accessibility_event(&toggled(true)), said);
    assert_eq!(elsewhere.handle_accessibility_event(&toggled(true)), None);
    assert_ne!(lines(&elsewhere), lines(&here));
    let wanted = elsewhere.take_page_build();
    assert!(elsewhere.take_page_build().is_none());
    assert!(wanted.is_some());
    let Some((generation, page)) = wanted else {
        return;
    };
    assert!(elsewhere.install_page_build(generation, build(page)));
    elsewhere.on_idle();
    assert_eq!(lines(&elsewhere), lines(&here));

    // Unchecked, then checked again before the first build came back: it is refused.
    elsewhere.handle_accessibility_event(&toggled(false));
    let wanted = elsewhere.take_page_build();
    assert!(wanted.is_some());
    let Some((first, page)) = wanted else {
        return;
    };
    let stale = build(page);
    elsewhere.handle_accessibility_event(&toggled(true));
    assert!(!elsewhere.install_page_build(first, stale));

    // A page smaller than the threshold is built at once.
    let mut small = engine_from(Some(1_000));
    small.handle_accessibility_event(&toggled(true));
    assert!(small.take_page_build().is_none());
    assert_eq!(lines(&small), lines(&here));
}

/// A page captured again (read quickly first, then in full; or after a live region grew
/// above the cursor) keeps the virtual cursor on its line, found by its text.
#[test]
fn a_recaptured_page_keeps_the_cursor_on_its_line() {
    let page = |lines: &[&str]| {
        AomNode::new(0, Role::Document)
            .with_name("News")
            .with_children(
                lines
                    .iter()
                    .enumerate()
                    .map(|(i, text)| AomNode::new(i + 1, Role::Paragraph).with_text(*text))
                    .collect(),
            )
    };
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.handle_document_loaded(
        "News",
        VirtualBuffer::from_aom(&page(&["First story.", "Second story.", "Third story."])),
    );
    engine.execute_command("{DOWN}");
    engine.execute_command("{DOWN}");
    assert_eq!(
        engine.execute_command("SayLine").as_deref(),
        Some("Second story.")
    );
    // The same page again, with a new line above the cursor's.
    engine.handle_document_loaded(
        "News",
        VirtualBuffer::from_aom(&page(&[
            "Breaking: a new story.",
            "First story.",
            "Second story.",
            "Third story.",
        ])),
    );
    // Applied once events go quiet.
    engine.on_idle();
    assert_eq!(
        engine.execute_command("SayLine").as_deref(),
        Some("Second story.")
    );
}

/// Q moves to the main region (JAWS's Quick Navigation Keys,
/// docs/references/jaws-quick-navigation-keys.md); it was handled by name only, so the key
/// reached the page.
#[test]
fn q_moves_to_the_main_region() {
    let aom = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 1 }).with_text("Site name"),
        AomNode::new(
            2,
            Role::Landmark {
                landmark_type: "main".to_owned(),
            },
        )
        .with_children(vec![
            AomNode::new(3, Role::Paragraph).with_text("The article starts here.")
        ]),
    ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&aom));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let q = KeyEvent::new(Key::Alpha('q'), Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&q), KeyAction::Consume);
    let spoken = speech.take();
    assert!(
        spoken.iter().any(|s| s.contains("main region")),
        "{spoken:?}"
    );
}

#[test]
fn test_engine_navigation_and_speech() {
    let buffer = create_test_buffer();
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Down arrow navigation
    let speech = engine.execute_command("{DOWN}");
    assert_eq!(
        speech,
        Some(" \u{001d} heading level 1 \u{001e} Main Title".to_string())
    );

    // Heading navigation
    let h_speech = engine.execute_command("H");
    assert_eq!(
        h_speech,
        Some("wrapping to top\nMain Title\n \u{001d} heading level 1 \u{001e}".to_string())
    );
}

#[test]
fn test_engine_key_event_handling_and_forms_mode() {
    let buffer = create_test_buffer();
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Move to edit field
    engine.execute_command("E");

    // Press Enter to enter forms mode
    let enter_event = KeyEvent::new(Key::Enter, Modifiers::none(), true);
    let action = engine.handle_key_event(&enter_event);
    assert_eq!(action, KeyAction::Consume);
    assert!(engine.forms_mode);

    // In forms mode, regular keys pass through
    let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);

    // Press Escape to exit forms mode
    let esc_event = KeyEvent::new(Key::Escape, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&esc_event), KeyAction::Consume);
    assert!(!engine.forms_mode);

    // Back in Virtual PC Cursor mode, Down arrow is consumed
    assert_eq!(engine.handle_key_event(&down_event), KeyAction::Consume);
}

/// A page field's description is said as it takes the focus at Beginner and Intermediate
/// verbosity, not at Advanced [obs, 552-553/s4].
#[test]
fn a_fields_description_is_left_out_at_advanced() {
    use crate::settings::VerbosityLevel;
    let mut email = AomNode::new(2, Role::Edit)
        .with_name("Email address")
        .with_value("sam@example.com");
    email.attributes.insert(
        "description".to_owned(),
        "We never share your email.".to_owned(),
    );
    let page = AomNode::new(0, Role::Document)
        .with_name("Form")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Sign up"),
            email,
        ]);
    for (level, described) in [
        (VerbosityLevel::Intermediate, true),
        (VerbosityLevel::Advanced, false),
    ] {
        let (mut engine, _speech) = engine_with_speech();
        let mut settings = engine.settings.clone();
        settings.verbosity.level = level;
        engine.set_settings(settings);
        engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
            title: "Form".to_owned(),
            root: page.clone(),
        });
        let mut focus = FocusEvent::new(Role::Edit)
            .with_node_id(2)
            .with_name("Email address");
        focus.in_document = true;
        let said = engine.handle_focus_event(&focus).unwrap_or_default();
        assert!(said.contains("sam@example.com"), "{level:?}: {said}");
        assert_eq!(
            said.contains("We never share"),
            described,
            "{level:?}: {said}"
        );
    }
}

/// Escape out of forms mode says "virtual PC Cursor" at Beginner verbosity, and JAWS's short
/// message, "virtual PC", above it [obs, 555/s10; 552/s10, 553/s10].
#[test]
fn leaving_forms_mode_says_the_cursor_at_the_levels_length() {
    use crate::settings::VerbosityLevel;
    for (level, said) in [
        (VerbosityLevel::Beginner, "virtual PC Cursor"),
        (VerbosityLevel::Intermediate, "virtual PC"),
        (VerbosityLevel::Advanced, "virtual PC"),
    ] {
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(speech.clone()),
        );
        let mut settings = engine.settings.clone();
        settings.verbosity.level = level;
        engine.set_settings(settings);
        engine.execute_command("E");
        engine.handle_key_event(&key(Key::Enter));
        assert!(engine.forms_mode);
        speech.take();
        engine.handle_key_event(&key(Key::Escape));
        assert_eq!(speech.take(), vec![said.to_owned()], "{level:?}");
    }
}

#[test]
fn test_engine_auto_forms_mode_on_focus_edit() {
    let buffer = create_test_buffer();
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    assert!(!engine.forms_mode);

    // Focus event moving into Search Input
    let focus_ev = FocusEvent::new(Role::Edit)
        .with_node_id(3)
        .with_name("Search Input");
    let speech = engine.handle_focus_event(&focus_ev).unwrap_or_default();

    // Forms mode follows focus into the edit silently; a page edit has no typing hint
    // [obs, 204/s1-s6].
    assert!(engine.forms_mode);
    assert!(engine.cursor.forms_mode_active);
    assert_eq!(engine.cursor.current_node_id, Some(3));
    assert!(speech.contains("Search Input"));
    assert!(!speech.contains("Type in text."));

    // Focus transitions to Submit Button -> should auto exit forms mode
    let btn_focus = FocusEvent::new(Role::Button)
        .with_node_id(4)
        .with_name("Submit Button");
    let btn_speech = engine.handle_focus_event(&btn_focus).unwrap_or_default();

    assert!(!engine.forms_mode);
    assert!(!engine.cursor.forms_mode_active);
    assert_eq!(engine.cursor.current_node_id, Some(4));
    assert!(btn_speech.contains("Submit Button"));
    assert!(btn_speech.contains("To activate press Enter."));
    assert!(!btn_speech.contains("virtual PC Cursor"));
}

#[test]
fn test_engine_live_region_announcements() {
    let buffer = create_test_buffer();
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Assertive live region
    let alert = LiveRegionEvent::new("Security update installed.", Politeness::Assertive);
    let alert_speech = engine.handle_live_region_event(&alert).unwrap_or_default();
    assert_eq!(alert_speech, "Security update installed.");

    // Polite live region
    let status = LiveRegionEvent::new("Download finished.", Politeness::Polite);
    let status_speech = engine.handle_live_region_event(&status).unwrap_or_default();
    assert_eq!(status_speech, "Download finished.");

    // Politeness::Off
    let off = LiveRegionEvent::new("Ignored mutation.", Politeness::Off);
    assert_eq!(engine.handle_live_region_event(&off), None);
}

#[test]
fn a_page_is_already_a_virtual_view() {
    let aom = AomNode::new(0, Role::Document)
        .with_children(vec![AomNode::new(1, Role::Paragraph).with_text("Hello")]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&aom));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.load_buffer(VirtualBuffer::from_aom(&aom));
    for command in ["VirtualizeWindow", "VirtualizeControl"] {
        assert_eq!(
            engine.execute_command(command).as_deref(),
            Some("The current location is already in a virtual view")
        );
    }
    assert!(engine.viewer.is_none());
}

/// Windows 11 Notepad (JAWS 2025 on the dev box): the font, then its paragraph formatting.
#[test]
fn a_native_field_says_its_paragraph_formatting() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut focus = FocusEvent::new(Role::Document)
        .with_name("Text editor")
        .with_window_title("viewer-test.txt - Notepad");
    focus.text = Some(crate::event::TextContext {
        text: "First line".to_owned(),
        caret: 0,
        selection: None,
        font: Some(crate::event::FontInfo {
            family: "Consolas".to_owned(),
            size_tenths: 110,
            alignment: Some("left".to_owned()),
            ..Default::default()
        }),
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&focus);
    assert_eq!(
        engine.execute_command("Insert+F").as_deref(),
        Some("Font is Consolas 11 point,\nParagraph formatting left justified")
    );
    assert_eq!(
        engine.execute_command("SayFontInViewer").as_deref(),
        Some(
            "Font information:\nConsolas 11 point,\nParagraph formatting left justified\n\
             \u{1d} Link \u{1e} List Okur Hot Keys\nPress ESCAPE to close this message."
        )
    );
}

#[test]
fn test_engine_text_selection_and_font_query() {
    let aom = AomNode::new(0, Role::Document).with_children(vec![AomNode::new(1, Role::Paragraph)
        .with_text("Hello World")
        .with_formatting(
            crate::aom::TextFormatting::new()
                .with_size(16.0)
                .with_bold(true)
                .with_family("Arial")
                .with_alignment("left"),
        )]);
    let buffer = VirtualBuffer::from_aom(&aom);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Font query, in JAWS's phrasing [obs, 209/s3]
    let font_speech = engine.execute_command("Insert+F");
    assert_eq!(
        font_speech,
        Some("Font is bold  Arial 16 point".to_string())
    );

    // Select next character
    let sel1 = engine.execute_command("+{RIGHT}");
    assert_eq!(sel1, Some("H\nselected".to_string()));

    let sel2 = engine.execute_command("+{RIGHT}");
    assert_eq!(sel2, Some("e\nselected".to_string()));

    // Unselect prior character
    let unsel = engine.execute_command("+{LEFT}");
    assert_eq!(unsel, Some("e\nunselected".to_string()));

    // Query active selection [obs, 501/s8]
    let say_sel = engine.execute_command("SaySelection");
    assert_eq!(say_sel, Some("Selection is\nH".to_string()));

    // Moving cursor clears selection
    engine.execute_command("{RIGHT}");
    let no_sel = engine.execute_command("SaySelection");
    assert_eq!(no_sel, Some("Nothing selected".to_string()));
}

/// Keyboard Help: Insert+1 turns it on; a key is then described, not run: its name, then what
/// it does; in a dialog Tab goes to the application; Insert+1 turns it off and keys run again
/// [obs, 509/s1, s2, s4, s6; 634].
#[test]
fn keyboard_help_describes_keys_instead_of_running_them() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.app_mode = AppMode::NativeControl;
    let mut insert = Modifiers::none();
    insert.insert = true;
    let press = |engine: &mut ScreenReaderEngine, key: Key, modifiers: Modifiers| {
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true))
    };
    assert_eq!(
        press(&mut engine, Key::Digit(1), insert),
        KeyAction::Consume
    );
    assert_eq!(speech.take(), ["Keyboard Help on"]);
    assert_eq!(press(&mut engine, Key::UpArrow, insert), KeyAction::Consume);
    let said = speech.take().join("\n");
    assert_eq!(
        said.lines().next(),
        Some("Insert+UpArrow"),
        "the key's name first: {said}"
    );
    assert_eq!(
        said.lines().nth(1),
        Some(crate::keyboard_help::description(&crate::command::Command::SayLine, false).as_str())
    );
    assert_eq!(
        press(&mut engine, Key::Tab, Modifiers::none()),
        KeyAction::Consume
    );
    assert_eq!(
        speech.take().join("\n"),
        format!("Tab\n{}", crate::keyboard_help::passes())
    );
    press(&mut engine, Key::Digit(1), insert);
    assert_eq!(speech.take(), ["Keyboard Help off"]);
    assert_eq!(
        press(&mut engine, Key::Tab, Modifiers::none()),
        KeyAction::PassThrough
    );
}

/// Insert+T in a page says, after the title, the region around the virtual cursor, its level
/// and the heading it is under; at the top, outside every region, the title alone
/// [obs, 624/s1, s9; 625/s5].
#[test]
fn say_window_title_in_a_page_says_where_the_cursor_is() {
    let mut article = AomNode::new(2, Role::Group).with_children(vec![
        AomNode::new(3, Role::Heading { level: 2 }).with_text("What changes"),
        AomNode::new(4, Role::Paragraph).with_text("Membership costs less."),
    ]);
    article
        .attributes
        .insert("aria_role".to_owned(), "article".to_owned());
    let main = AomNode::new(
        1,
        Role::Landmark {
            landmark_type: "main".to_owned(),
        },
    )
    .with_children(vec![article]);
    let page = AomNode::new(0, Role::Document)
        .with_name("News")
        .with_children(vec![main]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.cursor.move_to_start();
    assert_eq!(
        engine.execute_command("SayWindowTitle").as_deref(),
        Some("Title is\nNews")
    );
    engine.cursor.move_to_node_id(4);
    assert_eq!(
        engine.execute_command("SayWindowTitle").as_deref(),
        Some("Title is\nNews\narticle\nlevel 3\nheading is What changes")
    );
    // Short messages say the level and the heading alone [obs, 636/s7, 637/s7].
    let mut settings = engine.settings.clone();
    settings.verbosity.level = crate::settings::VerbosityLevel::Intermediate;
    engine.set_settings(settings);
    let said = engine.execute_command("SayWindowTitle").unwrap_or_default();
    assert!(said.ends_with("article\n3\nWhat changes"), "{said:?}");
}

/// A selection of up to 1000 characters is read whole; a longer one is said by its size and its
/// last 20 characters, a line break counting one: 13 lines of 80 are 1052, "1032 characters up
/// to | d of all of it here." [obs, 617/s21, s24].
#[test]
fn a_long_selection_is_said_by_its_size_and_end() {
    let line = |n: usize| {
        let text = if n == 15 {
            format!("Paragraph {n:02} of fifteen, eighty characters each: the very end of all of it here.")
        } else {
            format!("Paragraph {n:02} of fifteen, eighty characters each: alpha bravo charlie delta echo.")
        };
        let padded = format!("{text:<80}");
        AomNode::new(n, Role::Paragraph).with_text(padded.trim_end())
    };
    let page = AomNode::new(0, Role::Document)
        .with_name("Selection Sizes")
        .with_children((1..=15).map(line).collect());
    let engine_at = |line_index: usize| {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        engine.cursor.move_to_line(line_index);
        engine
    };
    let length = |engine: &ScreenReaderEngine| {
        let start = engine.cursor.offset;
        engine
            .cursor
            .buffer
            .text
            .get(start..)
            .map_or(0, |t| t.chars().count())
    };
    // From the 4th of 15 lines (the title is line 0): 12 lines.
    let mut short = engine_at(4);
    let short_length = length(&short);
    let said = short.execute_command("SelectToBottom").unwrap_or_default();
    assert!(short_length <= 1000, "{short_length}");
    assert!(said.starts_with("selected\nParagraph 04"), "{said}");
    // From the 3rd: 13 lines, over 1000.
    let mut long = engine_at(3);
    let long_length = length(&long);
    assert!(long_length > 1000, "{long_length}");
    assert_eq!(
        long.execute_command("SelectToBottom").as_deref(),
        Some(
            format!(
                "selected\n{} characters up to\nd of all of it here.",
                long_length - 20
            )
            .as_str()
        )
    );
}

/// Say Line pressed again within 350 ms spells the virtual cursor's line to its end, which
/// JAWS says as a space; 400 ms later it reads the line again [obs, 654/s3, s5]. Ctrl+End
/// goes to the last line's start [obs, 643/s22].
#[test]
fn say_line_twice_spells_to_the_line_end_within_jaws_window() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Window")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("banner"),
            AomNode::new(2, Role::Paragraph).with_text("News"),
        ]);
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    engine.cursor.move_to_line(1);
    let say_line = |time_ms: u64| {
        let mut key = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
        key.modifiers.insert = true;
        key.time_ms = time_ms;
        key
    };
    let said = |engine: &mut ScreenReaderEngine, time_ms: u64| {
        speech.take();
        engine.handle_key_event(&say_line(time_ms));
        norm(&speech.take().join(" "))
    };
    assert_eq!(said(&mut engine, 1000), "banner");
    assert!(said(&mut engine, 1300).ends_with("space"));
    assert_eq!(said(&mut engine, 5000), "banner");
    assert_eq!(said(&mut engine, 5400), "banner");
    engine.execute_command("BottomOfFile");
    let last = engine.cursor.buffer.lines.last().map(|l| l.start_offset);
    assert_eq!(Some(engine.cursor.offset), last);
}

/// A dialog focused while still hidden has no controls yet; once it is shown and read again,
/// Say Window reads them, saying nothing as they arrive [obs, 665/s3].
#[test]
fn a_dialog_shown_after_its_focus_gives_say_window_its_controls() {
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(create_test_buffer()),
        Box::new(speech.clone()),
    );
    let mut focus = FocusEvent::new(Role::Edit).with_name("Font:");
    focus.window_title = Some("Font".to_owned());
    engine.handle_focus_event(&focus);
    assert_eq!(engine.execute_command("SayWindow"), None);
    let mut shown = focus.clone();
    shown.window_items = vec![crate::event::WindowItem {
        role: Role::Button,
        name: "OK".to_owned(),
        value: None,
        expanded: None,
        checked: None,
        read_only: false,
        group: None,
        place: None,
        in_combo_box: false,
    }];
    speech.take();
    assert_eq!(
        engine.handle_accessibility_event(&AccessibilityEvent::WindowShown(shown)),
        None
    );
    assert!(speech.take().is_empty());
    let said = engine.execute_command("SayWindow").unwrap_or_default();
    assert!(said.contains("OK"), "{said:?}");
}

/// Say All to a page's end says its last word again, where reading leaves the cursor
/// [obs, 613/s26, 408/s10]; Say Line twice on that last line spells it without a space at
/// its end, the page's text ending there without a line break [obs, 613/s24].
#[test]
fn say_all_ends_on_the_pages_last_word() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Türkçe Metin")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Son"),
            AomNode::new(2, Role::Paragraph).with_text("ÇĞ çğ"),
        ]);
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    engine.cursor.move_to_start();
    let said = engine.execute_command("SayAll").unwrap_or_default();
    assert!(said.ends_with("ÇĞ çğ\nçğ"), "{said:?}");
    let say_line = |time_ms: u64| {
        let mut key = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
        key.modifiers.insert = true;
        key.time_ms = time_ms;
        key
    };
    engine.handle_key_event(&say_line(1000));
    speech.take();
    engine.handle_key_event(&say_line(1100));
    let spelled = norm(&speech.take().join(" "));
    assert!(spelled.starts_with("Ç Ğ"), "{spelled:?}");
    assert!(!spelled.ends_with("space"), "{spelled:?}");
}

/// End goes past the line's last letter, saying nothing; Left then says that letter, as JAWS
/// says it after End on a link's line [obs, 635/s20, s21].
#[test]
fn end_goes_past_the_last_letter() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Ends")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Riverside Gazette"),
            AomNode::new(2, Role::Paragraph).with_text("News"),
        ]);
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    engine.cursor.move_to_line(1);
    speech.take();
    engine.handle_key_event(&key(Key::End));
    assert_eq!(norm(&speech.take().join(" ")), "");
    engine.handle_key_event(&key(Key::LeftArrow));
    assert_eq!(norm(&speech.take().join(" ")), "e");
}

/// A line selected whole is said as the line is, its marks where they fall: a list item that
/// is only a link, "• same page link Council approves library budget" [obs, 562/s2, s8].
#[test]
fn a_selected_list_item_says_its_link() {
    let link = |id: usize, href: &str, text: &str| {
        AomNode::new(
            id,
            Role::Link {
                href: Some(href.to_owned()),
            },
        )
        .with_name(text)
    };
    let page = AomNode::new(0, Role::Document)
        .with_name("Related")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Related stories"),
            AomNode::new(2, Role::List).with_children(vec![
                AomNode::new(3, Role::ListItem).with_children(vec![
                    AomNode::new(7, Role::Text).with_text("• "),
                    link(4, "#budget", "Council approves library budget"),
                ]),
                AomNode::new(5, Role::ListItem).with_children(vec![
                    AomNode::new(8, Role::Text).with_text("• "),
                    link(6, "#bookfair", "Spring book fair returns"),
                ]),
            ]),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.cursor.move_to_start();
    let said = norm(&engine.execute_command("SelectToBottom").unwrap_or_default());
    let words = said
        .replace(['\u{1d}', '\u{1e}'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        words.contains("• same page link Council approves library budget"),
        "{said}"
    );
}

/// A page whose last line closes a region ends with a line break in JAWS's text: a selection
/// to the end counts one more character, and its last 20 end with the region's end line
/// [obs, 562/s20, 650/s1]; a page ending with a paragraph does not [obs, 617, 651].
#[test]
fn a_selection_to_a_region_end_counts_its_line_break() {
    let paragraph = |n: usize| {
        AomNode::new(n, Role::Paragraph).with_text(format!(
            "Paragraph {n:02} of fifteen, eighty characters each: alpha bravo charlie delta."
        ))
    };
    let footer = AomNode::new(
        20,
        Role::Landmark {
            landmark_type: "contentinfo".to_owned(),
        },
    )
    .with_children((1..=15).map(paragraph).collect());
    let page = AomNode::new(0, Role::Document)
        .with_name("Regions")
        .with_children(vec![footer]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.cursor.move_to_start();
    let length = engine.cursor.buffer.text.chars().count();
    assert_eq!(
        engine.execute_command("SelectToBottom").as_deref(),
        Some(
            format!(
                "selected\n{} characters up to\nent information end",
                length + 1 - 20
            )
            .as_str()
        )
    );
}

/// Say Color (Insert+5) names the text's color and its background's from the color table:
/// "red on white", the link blue "blue2 on white"; the page's title line, which is no text of
/// the page, is "default on default" [obs, 565/s1, s5, s7].
#[test]
fn say_color_names_the_text_and_background_colors() {
    use crate::color::Rgb;
    let colored = |id: usize, text: &str, color: Rgb| {
        let mut formatting = crate::aom::TextFormatting::new();
        formatting.color = Some(color);
        formatting.background = Some(Rgb::new(255, 255, 255));
        AomNode::new(id, Role::Paragraph)
            .with_text(text)
            .with_formatting(formatting)
    };
    let aom = AomNode::new(0, Role::Document)
        .with_name("Text Colors")
        .with_children(vec![
            colored(1, "Pure red text.", Rgb::new(255, 0, 0)),
            colored(2, "Link blue text.", Rgb::new(0, 0, 238)),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&aom));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.cursor.move_to_start();
    assert_eq!(
        engine.execute_command("SayColor").as_deref(),
        Some("default on default")
    );
    engine.execute_command("{DOWN}");
    assert_eq!(
        engine.execute_command("Insert+5").as_deref(),
        Some("red on white")
    );
    engine.execute_command("{DOWN}");
    assert_eq!(
        engine.execute_command("SayColor").as_deref(),
        Some("blue2 on white")
    );
    // Outside a page a control without text colors is "default on default", as JAWS says on
    // a dialog's button [obs, 513/s3]; a text field says its own [obs, 661/s1].
    engine.handle_focus_event(&FocusEvent::new(Role::Button).with_name("Open second window"));
    assert_eq!(
        engine.execute_command("SayColor").as_deref(),
        Some("default on default")
    );
    let mut field = FocusEvent::new(Role::Edit).with_name("Notes");
    field.text = Some(crate::event::TextContext {
        text: "a".into(),
        caret: 0,
        selection: None,
        font: Some(crate::event::FontInfo {
            family: "Consolas".into(),
            size_tenths: 110,
            color: Some(Rgb { r: 0, g: 0, b: 0 }),
            background: Some(Rgb {
                r: 255,
                g: 255,
                b: 255,
            }),
            ..crate::event::FontInfo::default()
        }),
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&field);
    assert_eq!(
        engine.execute_command("SayColor").as_deref(),
        Some("black on white")
    );
}

/// V and Shift+V move by the links whose pages were opened before, U and Shift+U by the
/// others (JAWS's MoveToNextVisitedLink and MoveToNextUnvisitedLink); a page without one says
/// so.
#[test]
fn v_and_u_move_by_visited_and_unvisited_links() {
    let link = |id: usize, name: &str, visited: bool| {
        let mut node = AomNode::new(
            id,
            Role::Link {
                href: Some(format!("page{id}.html")),
            },
        )
        .with_name(name);
        if visited {
            node.attributes
                .insert("visited".to_owned(), "true".to_owned());
        }
        AomNode::new(id + 10, Role::Paragraph).with_children(vec![node])
    };
    let page = AomNode::new(0, Role::Document)
        .with_name("Reading List")
        .with_children(vec![
            link(1, "This reading list", true),
            link(2, "Spring catalog", false),
            link(3, "City Library", true),
            link(4, "Volunteer form", false),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.cursor.move_to_start();
    let said = |engine: &mut ScreenReaderEngine, command: &str| {
        engine.execute_command(command).unwrap_or_default()
    };
    // A visited link's line says it visited, before its role [obs, 689/s5].
    let first = said(&mut engine, "V");
    assert!(first.contains("This reading list"), "{first}");
    assert!(
        first.contains(&format!("\u{1d}{}\u{1e}", msg::state_visited())),
        "{first}"
    );
    assert!(said(&mut engine, "V").contains("City Library"));
    assert!(said(&mut engine, "+V").contains("This reading list"));
    assert!(said(&mut engine, "U").contains("Spring catalog"));
    assert!(said(&mut engine, "U").contains("Volunteer form"));
    assert!(said(&mut engine, "+U").contains("Spring catalog"));
    // The keys themselves: V to the next visited link, U to the next unvisited one.
    engine.cursor.move_to_start();
    let press = |engine: &mut ScreenReaderEngine, c: char| {
        engine.handle_key_event(&KeyEvent::new(Key::Alpha(c), Modifiers::none(), true));
        engine.cursor.current_line()
    };
    assert!(press(&mut engine, 'V').contains("This reading list"));
    assert!(press(&mut engine, 'U').contains("Spring catalog"));
    assert!(press(&mut engine, 'V').contains("City Library"));
    let unvisited = AomNode::new(0, Role::Document).with_children(vec![link(2, "Spring", false)]);
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&unvisited)),
        Box::new(MockSpeechOutput::new()),
    );
    assert_eq!(
        engine.execute_command("V").as_deref(),
        Some(msg::quicknav_none_visited_links().as_str())
    );
}

/// The Okur cursor in a text field, routed to the caret, moves over the field's lines with Up
/// and Down and reads them without their indentation; Insert+Up reads its line; on a line it
/// is before the first character, where Left says nothing and Right says the first, a space;
/// Ctrl+Right says the next word, which punctuation ends [obs, 690, 752, 754/s11]. The mouse
/// goes with it, onto the line's first character and then the character it is at.
#[test]
fn the_okur_cursor_reads_a_text_fields_lines() {
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
        Box::new(speech.clone()),
    );
    engine.set_settings(crate::settings::Settings::reference_machine());
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mouse_to = |actions: &crate::actions::RecordingActions| {
        actions
            .requests()
            .iter()
            .filter(|(what, _)| what == "mouse to text")
            .map(|(_, offset)| *offset)
            .collect::<Vec<_>>()
    };
    let mut field = FocusEvent::new(Role::Edit).with_name("Text Editor");
    field.text = Some(crate::event::TextContext {
        text: "fn main() {\r\n    let total = 1;\r\n    println!(\"{total}\");\r\n}".into(),
        caret: 0,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&field);
    let press = |engine: &mut ScreenReaderEngine, key: Key, insert: bool| {
        let _ = speech.take();
        let modifiers = Modifiers {
            insert,
            ..Modifiers::none()
        };
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true));
        speech.take().join("\n")
    };
    press(&mut engine, Key::NumpadMinus, false);
    press(&mut engine, Key::NumpadMinus, true);
    assert_eq!(press(&mut engine, Key::DownArrow, false), "let total = 1;");
    assert_eq!(
        press(&mut engine, Key::DownArrow, false),
        "println!(\"{total}\");"
    );
    assert_eq!(
        press(&mut engine, Key::UpArrow, true),
        "println!(\"{total}\");"
    );
    assert_eq!(press(&mut engine, Key::LeftArrow, false), "");
    assert_eq!(press(&mut engine, Key::RightArrow, false), "space");
    let ctrl_right = |engine: &mut ScreenReaderEngine| {
        let _ = speech.take();
        let modifiers = Modifiers {
            ctrl: true,
            ..Modifiers::none()
        };
        engine.handle_key_event(&KeyEvent::new(Key::RightArrow, modifiers, true));
        speech.take().join(
            "
",
        )
    };
    assert_eq!(ctrl_right(&mut engine), "println");
    // Insert+Numpad Minus to the caret at 0; Down twice to the lines starting at 13 and 33;
    // Insert+Up does not move it; Left stays before the first character; Right to the space
    // at 33; Ctrl+Right to "println" at 37.
    assert_eq!(mouse_to(&actions), [0, 13, 33, 33, 33, 37]);
}

/// The Okur cursor on a page, routed to the virtual cursor, reads the page's lines as its
/// screen shows them, without their roles; Page Up goes to the page's first line; the virtual
/// cursor stays where it was [obs, 753].
#[test]
fn the_okur_cursor_reads_a_pages_lines() {
    let page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 1 }).with_text("Riverside Gazette"),
        AomNode::new(2, Role::Heading { level: 1 }).with_text("Weekend events"),
        AomNode::new(3, Role::Paragraph).with_text("Three events open to everyone."),
    ]);
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    let heading = engine.cursor.buffer.start_of(2).unwrap_or_default();
    engine.cursor.move_to(heading);
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let press = |engine: &mut ScreenReaderEngine, key: Key, modifiers: Modifiers| {
        let _ = speech.take();
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true));
        speech.take().join(
            "
",
        )
    };
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::none()
    };
    press(&mut engine, Key::NumpadMinus, insert);
    press(&mut engine, Key::NumpadMinus, Modifiers::none());
    assert_eq!(press(&mut engine, Key::UpArrow, insert), "Weekend events");
    assert_eq!(
        press(&mut engine, Key::DownArrow, Modifiers::none()),
        "Three events open to everyone."
    );
    assert_eq!(press(&mut engine, Key::RightArrow, Modifiers::none()), "T");
    assert_eq!(press(&mut engine, Key::RightArrow, ctrl), "events");
    assert_eq!(press(&mut engine, Key::PageUp, Modifiers::none()), "");
    assert_eq!(
        press(&mut engine, Key::UpArrow, insert),
        "Riverside Gazette"
    );
    press(&mut engine, Key::NumpadPlus, Modifiers::none());
    assert!(engine.cursor.current_line().contains("Weekend events"));
    // The mouse went with it: onto the heading for Insert+Numpad Minus, the paragraph for
    // Down, Right and Ctrl+Right; Page Up, to the window's top, leaves it.
    let to: Vec<usize> = actions
        .requests()
        .iter()
        .filter(|(what, _)| what == "mouse to node")
        .map(|(_, node)| *node)
        .collect();
    assert_eq!(to, [2, 3, 3, 3]);
}

/// Ctrl+Insert+Tab on a page's field opens Custom Label; OK with "lamp" names the field "lamp"
/// in the page's reading and keeps it in the store; the virtual cursor stays on the field.
/// An emptied label gives the field its name back [obs, 688/s9-s18].
#[test]
fn a_custom_label_names_a_pages_field() {
    use crate::ui::{FromUi, ToUi, Value};
    let mut field = AomNode::new(2, Role::Edit).with_name("Branch");
    field
        .attributes
        .insert("id".to_owned(), "pickup-branch".to_owned());
    let mut page = AomNode::new(0, Role::Document)
        .with_name("Book Pickup")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Book pickup"),
            field,
        ]);
    page.attributes.insert(
        "url".to_owned(),
        "https://library.example/pickup".to_owned(),
    );
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(MockSpeechOutput::new()),
    );
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    #[derive(Clone, Default)]
    struct Kept(std::sync::Arc<std::sync::Mutex<Vec<crate::labels::CustomLabels>>>);
    impl crate::labels::LabelStore for Kept {
        fn save(&self, labels: &crate::labels::CustomLabels) {
            if let Ok(mut kept) = self.0.lock() {
                kept.push(labels.clone());
            }
        }
    }
    let kept = Kept::default();
    engine.set_custom_labels(Default::default(), Some(Box::new(kept.clone())));
    engine.cursor.move_to_start();
    // On text there is nothing to label, and no dialog opens.
    assert_eq!(
        engine.execute_command("CreatePrompt").as_deref(),
        Some("A Custom Label cannot be created for this item.")
    );
    assert!(ui.take().is_empty());
    for _ in 0..5 {
        if engine.cursor.current_line().contains("Branch") {
            break;
        }
        engine.execute_command("{DOWN}");
    }
    let ctrl_insert = Modifiers {
        ctrl: true,
        insert: true,
        ..Modifiers::none()
    };
    engine.handle_key_event(&KeyEvent::new(Key::Tab, ctrl_insert, true));
    let shown = ui.take();
    let id = shown
        .iter()
        .find_map(|m| match m {
            ToUi::DialogShow { dialog } if dialog.title == "Custom Label" => Some(dialog.id),
            _ => None,
        })
        .unwrap_or_default();
    assert!(id > 0, "{shown:?}");
    let typed = |text: &str| FromUi::DialogChanged {
        id,
        control: "label".to_owned(),
        value: Value::Text(text.to_owned()),
    };
    let ok = FromUi::DialogPressed {
        id,
        control: "ok".to_owned(),
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(typed("lamp")));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(ok.clone()));
    assert!(engine.cursor.current_line().contains("lamp"));
    assert!(!engine.cursor.current_line().contains("Branch"));
    let saved = kept.0.lock().map(|k| k.clone()).unwrap_or_default();
    assert_eq!(
        saved
            .last()
            .and_then(|l| l.places.get("library.example"))
            .and_then(|p| p.get("id pickup-branch"))
            .map(String::as_str),
        Some("lamp")
    );
    // Emptied, the label is taken away.
    engine.handle_key_event(&KeyEvent::new(Key::Tab, ctrl_insert, true));
    let id = ui
        .take()
        .iter()
        .find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog.id),
            _ => None,
        })
        .unwrap_or_default();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "label".to_owned(),
        value: Value::Text(String::new()),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".to_owned(),
    }));
    assert!(engine.cursor.current_line().contains("Branch"));
}

/// Ctrl+Windows+K sets the page's temporary PlaceMarker, "Temporary PlaceMarker set" and the
/// line; K and Shift+K go back to it and say the element there, again from the marker itself;
/// before one is set K says there are none [obs, 687/s4-s9, 221/s4].
#[test]
fn a_temporary_placemarker_is_set_and_found_with_k() {
    let mut link = AomNode::new(
        2,
        Role::Link {
            href: Some("page.html".to_owned()),
        },
    )
    .with_name("Maria Lopez");
    link.attributes
        .insert("same_page".to_owned(), "true".to_owned());
    let page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 1 }).with_text("Library news"),
        AomNode::new(3, Role::Paragraph).with_children(vec![link]),
        AomNode::new(4, Role::Paragraph).with_text("The end."),
    ]);
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    engine.cursor.move_to_start();
    let key = |engine: &mut ScreenReaderEngine, c: char, modifiers: Modifiers| {
        let _ = speech.take();
        engine.handle_key_event(&KeyEvent::new(Key::Alpha(c), modifiers, true));
        speech.take().join("\n")
    };
    assert_eq!(
        key(&mut engine, 'K', Modifiers::none()),
        msg::quicknav_none_placemarkers().to_string()
    );
    engine.execute_command("{DOWN}");
    let ctrl_win = Modifiers {
        ctrl: true,
        win: true,
        ..Modifiers::none()
    };
    let set = key(&mut engine, 'K', ctrl_win);
    assert!(set.starts_with("Temporary PlaceMarker set\n"), "{set}");
    assert!(set.contains("Maria Lopez"), "{set}");
    engine.cursor.move_to_start();
    let found = engine
        .cursor
        .buffer
        .node(2)
        .map(|link| crate::speech::SpeechFormatter::format_link_focus("Maria Lopez", link))
        .unwrap_or_default();
    assert!(found.contains("Maria Lopez"), "{found}");
    assert_eq!(key(&mut engine, 'K', Modifiers::none()), found);
    assert!(engine.cursor.current_line().contains("Maria Lopez"));
    assert_eq!(key(&mut engine, 'K', Modifiers::none()), found);
    let shift = Modifiers {
        shift: true,
        ..Modifiers::none()
    };
    assert_eq!(key(&mut engine, 'K', shift), found);
}

/// A saved PlaceMarker of the page is found with K where its line is now, and K says its line
/// [obs, 702/s11, s12]; another page's is not offered.
#[test]
fn k_finds_a_saved_placemarker_of_the_page() {
    use crate::placemarkers::{PlaceMarker, PlaceMarkers};
    let url = "file:///C:/corpus/fixtures/placemarkers.html";
    let mut page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 2 }).with_text("Food and drink"),
        AomNode::new(2, Role::Paragraph).with_text("Water in a closed bottle is allowed."),
        AomNode::new(3, Role::Paragraph).with_text("Phones stay silent."),
    ]);
    page.attributes.insert("url".to_owned(), url.to_owned());
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    let mut markers = PlaceMarkers::default();
    // Saved where the line was on an earlier visit.
    markers.add(
        url,
        PlaceMarker {
            name: "Water in a closed bottle is allowed.".to_owned(),
            offset: 3,
            anchor: Some("Water in a closed bottle".to_owned()),
            domain: false,
        },
    );
    markers.add(
        "file:///C:/corpus/fixtures/other.html",
        PlaceMarker {
            name: "Elsewhere".to_owned(),
            offset: 0,
            anchor: None,
            domain: false,
        },
    );
    engine.set_placemarkers(markers, None);
    engine.cursor.move_to_start();
    let k = |engine: &mut ScreenReaderEngine, shift: bool| {
        let _ = speech.take();
        let modifiers = Modifiers {
            shift,
            ..Modifiers::none()
        };
        engine.handle_key_event(&KeyEvent::new(Key::Alpha('K'), modifiers, true));
        speech.take().join("\n")
    };
    assert_eq!(
        k(&mut engine, false),
        "Water in a closed bottle is allowed."
    );
    assert_eq!(
        k(&mut engine, false),
        "Water in a closed bottle is allowed."
    );
    assert_eq!(k(&mut engine, true), "Water in a closed bottle is allowed.");
}

/// Ctrl+Shift+K opens the PlaceMarker List; Add proposes the cursor's line; OK keeps it and
/// closes both dialogs; 7 says it, 8 has none; Remove All, answered Yes, takes it away and
/// closes both [obs, 702/s4-s6, s13, s14, s27, s28].
#[test]
fn a_placemarker_is_added_and_removed_in_the_list() {
    use crate::ui::{ControlKind, FromUi, ToUi};
    let url = "file:///C:/corpus/fixtures/placemarkers.html";
    let mut page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 2 }).with_text("Food and drink"),
        AomNode::new(2, Role::Paragraph).with_text("Water in a closed bottle is allowed."),
    ]);
    page.attributes.insert("url".to_owned(), url.to_owned());
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.cursor.move_to_start();
    engine.execute_command("{DOWN}");
    assert!(engine.cursor.current_line().contains("Water"));
    let ctrl_shift = Modifiers {
        ctrl: true,
        shift: true,
        ..Modifiers::none()
    };
    let shown = |ui: &crate::ui::RecordingUiHost| {
        ui.take().into_iter().find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog),
            _ => None,
        })
    };
    let ui_event = |engine: &mut ScreenReaderEngine, message: FromUi| {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message))
    };
    engine.handle_key_event(&KeyEvent::new(Key::Alpha('K'), ctrl_shift, true));
    let list = shown(&ui);
    assert_eq!(
        list.as_ref().map(|d| d.title.as_str()),
        Some("PlaceMarker List")
    );
    let list_id = list.map_or(0, |d| d.id);
    ui_event(
        &mut engine,
        FromUi::DialogPressed {
            id: list_id,
            control: "add".into(),
        },
    );
    let add = shown(&ui);
    assert_eq!(
        add.as_ref()
            .and_then(|d| d.controls.first())
            .map(|c| c.kind.clone()),
        Some(ControlKind::Edit {
            text: "Water in a closed bottle is allowed.".into()
        })
    );
    let add_id = add.map_or(0, |d| d.id);
    let said = ui_event(
        &mut engine,
        FromUi::DialogPressed {
            id: add_id,
            control: "ok".into(),
        },
    );
    assert_eq!(said, None);
    assert_eq!(
        ui.take(),
        [
            ToUi::DialogClose { id: add_id },
            ToUi::DialogClose { id: list_id }
        ]
    );
    // Back on the heading, 7 says the PlaceMarker's line where it is; 8 has none.
    engine.cursor.move_to_start();
    let digit = |engine: &mut ScreenReaderEngine, d: u8| {
        let _ = speech.take();
        engine.handle_key_event(&KeyEvent::new(Key::Digit(d), Modifiers::none(), true));
        speech.take().join("\n")
    };
    assert_eq!(
        digit(&mut engine, 7),
        "Water in a closed bottle is allowed."
    );
    assert!(engine.cursor.current_line().contains("Food"));
    assert_eq!(digit(&mut engine, 8), "No place marker #2");
    // Remove All, answered Yes.
    engine.handle_key_event(&KeyEvent::new(Key::Alpha('K'), ctrl_shift, true));
    let list = shown(&ui);
    let rows = list
        .as_ref()
        .and_then(|d| d.controls.first())
        .map(|c| c.kind.clone());
    assert!(
        matches!(&rows, Some(ControlKind::Table { rows, .. }) if rows.len() == 1),
        "{rows:?}"
    );
    let list_id = list.map_or(0, |d| d.id);
    ui_event(
        &mut engine,
        FromUi::DialogPressed {
            id: list_id,
            control: "removeall".into(),
        },
    );
    let question = shown(&ui).map_or(0, |d| d.id);
    ui_event(
        &mut engine,
        FromUi::DialogPressed {
            id: question,
            control: "yes".into(),
        },
    );
    assert_eq!(
        ui.take(),
        [
            ToUi::DialogClose { id: question },
            ToUi::DialogClose { id: list_id }
        ]
    );
    assert_eq!(digit(&mut engine, 7), "No place marker #1");
    // The list was opened on this page: K without PlaceMarkers opens it [obs, 703/s4]; on the
    // page loaded again, K says there are none [obs, 703/s1].
    engine.handle_key_event(&KeyEvent::new(Key::Alpha('K'), Modifiers::none(), true));
    assert_eq!(
        shown(&ui).map(|d| d.title),
        Some("PlaceMarker List".to_owned())
    );
    engine.load_buffer(VirtualBuffer::from_aom(&page));
    let _ = speech.take();
    engine.handle_key_event(&KeyEvent::new(Key::Alpha('K'), Modifiers::none(), true));
    assert_eq!(
        speech.take().join("\n"),
        msg::quicknav_none_placemarkers().to_string()
    );
}

/// Flexible Web: the advertisement hidden, then saved as a rule for the page, holds when the
/// page loads again, with no question; a customization left unsaved is asked about as the
/// page loads, and Yes opens the wizard on its last page alone [obs, 742/s16-s31].
#[test]
fn a_flexible_web_rule_is_saved_and_holds_when_the_page_loads() {
    use crate::flexible_web::Area;
    use crate::ui::{FromUi, ToUi, Value};
    let url = "file:///C:/corpus/fixtures/flexible-web.html";
    let mut advertisement = AomNode::new(
        1,
        Role::Landmark {
            landmark_type: "complementary".into(),
        },
    )
    .with_children(vec![
        AomNode::new(2, Role::Paragraph).with_text("Spring sale at Main Street Shoes.")
    ]);
    advertisement.name = Some("Advertisement".into());
    let mut page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(3, Role::Heading { level: 1 }).with_text("Weekend events"),
        advertisement,
    ]);
    page.attributes.insert("url".to_owned(), url.to_owned());
    page.name = Some("Weekend Events".into());
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let wizard_shown = |ui: &crate::ui::RecordingUiHost| {
        ui.take().into_iter().find_map(|m| match m {
            ToUi::WizardShow { wizard } => Some(wizard),
            _ => None,
        })
    };
    let ui_event = |engine: &mut ScreenReaderEngine, message: FromUi| {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message))
    };
    let press = |engine: &mut ScreenReaderEngine, id: u32, control: &str| {
        ui_event(
            engine,
            FromUi::DialogPressed {
                id,
                control: control.into(),
            },
        )
    };
    let change = |engine: &mut ScreenReaderEngine, id: u32, control: &str, value: Value| {
        ui_event(
            engine,
            FromUi::DialogChanged {
                id,
                control: control.into(),
                value,
            },
        )
    };
    let reload = |engine: &mut ScreenReaderEngine| {
        let _ = speech.take();
        engine.reload_asked = true;
        engine.handle_document_loaded("Weekend Events", VirtualBuffer::from_aom(&page))
    };
    let reads_advertisement =
        |engine: &ScreenReaderEngine| engine.cursor.buffer.text.contains("Spring sale");

    // The advertisement hidden: Next three times from its paragraph, Finish.
    let paragraph = engine.cursor.buffer.start_of(2).unwrap_or_default();
    engine.cursor.move_to(paragraph);
    assert!(engine.cursor.current_line().contains("Spring sale"));
    let _ = engine.open_flexible_web();
    let id = wizard_shown(&ui).map_or(0, |w| w.id);
    for _ in 0..3 {
        press(&mut engine, id, "next");
    }
    press(&mut engine, id, "finish");
    assert!(!reads_advertisement(&engine));

    // Saved as a rule for the page: Save temporary customizations as a rule, Next, a name,
    // To the current page, Finish.
    let _ = engine.open_flexible_web();
    let id = wizard_shown(&ui).map_or(0, |w| w.id);
    change(&mut engine, id, "task", Value::Selected(3));
    press(&mut engine, id, "next");
    let _ = ui.take();
    change(&mut engine, id, "name", Value::Text("Okur lab rule".into()));
    assert!(ui.take().iter().any(|m| matches!(
        m,
        ToUi::WizardButtons { buttons, .. } if buttons.finish
    )));
    change(&mut engine, id, "scope", Value::Selected(2));
    press(&mut engine, id, "finish");
    let rules = &engine.flexible_web_rules;
    assert_eq!(rules.rules.len(), 1);
    let areas = Area::around(url);
    assert_eq!(
        rules.state("Okur lab rule", &areas, 2),
        (true, Area::Page(url.to_owned()))
    );
    assert_eq!(
        rules.state("Okur lab rule", &areas, 1),
        (false, Area::Global)
    );
    assert!(!reads_advertisement(&engine));

    // Loaded again: the rule holds, and nothing is left to ask about.
    let said = reload(&mut engine).unwrap_or_default();
    assert!(said.starts_with("Weekend Events"), "{said}");
    assert!(!reads_advertisement(&engine));
    assert!(!ui
        .take()
        .iter()
        .any(|m| matches!(m, ToUi::DialogShow { .. })));

    // A heading hidden and left unsaved: loading the page again asks, after its title alone.
    let heading = engine.cursor.buffer.start_of(3).unwrap_or_default();
    engine.cursor.move_to(heading);
    let _ = engine.open_flexible_web();
    let id = wizard_shown(&ui).map_or(0, |w| w.id);
    for _ in 0..3 {
        press(&mut engine, id, "next");
    }
    press(&mut engine, id, "finish");
    assert!(!engine.cursor.buffer.text.contains("Weekend events"));
    let said = reload(&mut engine);
    assert_eq!(
        said.as_deref(),
        Some("Weekend Events  \u{1d} document \u{1e}")
    );
    let question = ui.take().into_iter().find_map(|m| match m {
        ToUi::DialogShow { dialog } => Some(dialog),
        _ => None,
    });
    assert_eq!(
        question.as_ref().map(|q| q.title.as_str()),
        Some("Flexible Web")
    );
    // The page loaded as it is, but for the rule.
    assert!(engine.cursor.buffer.text.contains("Weekend events"));
    assert!(!reads_advertisement(&engine));
    // Yes: the wizard's last page alone, with the heading's customization.
    press(&mut engine, question.map_or(0, |q| q.id), "yes");
    let saving = wizard_shown(&ui);
    assert_eq!(
        saving.as_ref().map(|w| w.page.title.as_str()),
        Some("Save Temporary Customizations")
    );
    assert!(saving.is_some_and(|w| !w.buttons.back));
}

/// A page's field has its text's colors. With the virtual cursor: another field's as the page
/// shows them ("navy on yellow"), "default on default" while it is empty, the focused field's
/// as typed in; in forms mode the colors at the caret, an empty field's too [obs, 662/s2, s5,
/// s7; 676/s2, s8, s10].
#[test]
fn say_color_in_a_page_field_is_its_texts() {
    use crate::color::Rgb;
    let mut navy = crate::aom::TextFormatting::new();
    navy.color = Some(Rgb::new(0, 0, 128));
    navy.background = Some(Rgb::new(255, 255, 0));
    let mut note = AomNode::new(2, Role::Edit).with_name("Note");
    note.value = Some("Bring a lamp".to_owned());
    note.formatting = Some(navy);
    let page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 1 }).with_text("Event notes"),
        note,
        AomNode::new(3, Role::Edit).with_name("Full name"),
    ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    // The reference machine's settings: Auto Forms Mode off, a field entered with Enter.
    engine.settings = crate::settings::Settings::reference_machine();
    let context = |text: &str, colored: bool| crate::event::TextContext {
        text: text.into(),
        caret: text.chars().count(),
        selection: None,
        font: colored.then(|| crate::event::FontInfo {
            color: Some(Rgb::new(0, 0, 0)),
            background: Some(Rgb::new(255, 255, 255)),
            ..crate::event::FontInfo::default()
        }),
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    let say_color = |engine: &mut ScreenReaderEngine| engine.execute_command("SayColor");
    engine.execute_command("{DOWN}");
    assert_eq!(say_color(&mut engine).as_deref(), Some("navy on yellow"));
    engine.execute_command("{DOWN}");
    assert_eq!(
        say_color(&mut engine).as_deref(),
        Some("default on default")
    );
    let mut focus = FocusEvent::new(Role::Edit)
        .with_node_id(3)
        .with_name("Full name");
    focus.in_document = true;
    focus.text = Some(context("", true));
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    engine.handle_key_event(&KeyEvent::new(Key::Enter, Modifiers::none(), true));
    assert!(engine.forms_mode);
    assert_eq!(say_color(&mut engine).as_deref(), Some("black on white"));
    engine.handle_key_event(&KeyEvent::new(Key::Escape, Modifiers::none(), true));
    assert!(!engine.forms_mode);
    assert_eq!(
        say_color(&mut engine).as_deref(),
        Some("default on default")
    );
    engine.handle_key_event(&KeyEvent::new(Key::Enter, Modifiers::none(), true));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context("ab", true)));
    engine.handle_key_event(&KeyEvent::new(Key::Escape, Modifiers::none(), true));
    assert!(!engine.forms_mode);
    assert_eq!(say_color(&mut engine).as_deref(), Some("black on white"));
}

#[test]
fn test_engine_modal_dialog_confinement_and_escape() {
    let mut dlg = AomNode::new(10, Role::Dialog)
        .with_name("Confirm Deletion")
        .with_children(vec![
            AomNode::new(11, Role::Heading { level: 2 }).with_text("Are you sure?"),
            AomNode::new(12, Role::Button).with_name("Confirm"),
            AomNode::new(13, Role::Button).with_name("Cancel"),
        ]);
    dlg.states.modal = true;

    let aom = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Heading { level: 1 }).with_text("Dashboard"),
        dlg,
        AomNode::new(20, Role::Paragraph).with_text("Backdrop footer text"),
    ]);

    let buffer = VirtualBuffer::from_aom(&aom);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Focus event moving into dialog
    let focus_dlg = FocusEvent::new(Role::Dialog)
        .with_node_id(10)
        .with_name("Confirm Deletion");
    let speech = engine.handle_focus_event(&focus_dlg).unwrap_or_default();
    assert_eq!(speech, "Dialog Confirm Deletion");
    assert!(engine.cursor.is_in_modal());

    // Quick navigation 'H' inside modal wraps within modal and finds level 2 heading
    let h_speech = engine.execute_command("H").unwrap_or_default();
    assert!(h_speech.contains("Are you sure?"));

    // Tab moves to Confirm button
    let tab1 = engine.execute_command("{TAB}").unwrap_or_default();
    assert!(tab1.contains("Confirm"));

    // Tab moves to Cancel button
    let tab2 = engine.execute_command("{TAB}").unwrap_or_default();
    assert!(tab2.contains("Cancel"));

    // Tab wraps back to Confirm button inside dialog (cannot escape to backdrop!)
    let tab3 = engine.execute_command("{TAB}").unwrap_or_default();
    assert!(tab3.contains("Confirm"));

    // Escape exits modal dialog
    let esc_event = KeyEvent::new(Key::Escape, Modifiers::none(), true);
    let action = engine.handle_key_event(&esc_event);
    assert_eq!(action, KeyAction::Consume);
    assert!(!engine.cursor.is_in_modal());
}

#[test]
fn test_engine_desktop_entry_vs_subsequent_item_navigation() {
    let aom = AomNode::new(0, Role::Document);
    let buffer = VirtualBuffer::from_aom(&aom);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // First entry to Desktop (e.g. Win+M / Win+D)
    let focus_entry = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("Progman")
        .with_name("Dropbox")
        .with_item_index(7)
        .with_item_count(11);
    let s1 = engine.handle_focus_event(&focus_entry).unwrap_or_default();
    assert!(s1.starts_with("desktop  \u{1d}  \u{1e} \nFolderView  \u{1d} List view \u{1e} "));
    assert!(s1.contains("Dropbox"));
    assert!(s1.contains("7 of 11"));
    assert!(s1.contains("To move to items use the Arrow keys."));

    // Subsequent item navigation (e.g. Down Arrow to Microsoft Edge)
    let focus_down = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("Progman")
        .with_name("Microsoft Edge")
        .with_item_index(9)
        .with_item_count(11);
    let s2 = engine.handle_focus_event(&focus_down).unwrap_or_default();
    assert_eq!(s2, "Microsoft Edge\n9 of 11");

    // Initial letter jump (e.g. 'j' to JAWS 2025)
    let focus_letter = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("Progman")
        .with_name("JAWS 2025")
        .with_item_index(4)
        .with_item_count(11);
    let s3 = engine.handle_focus_event(&focus_letter).unwrap_or_default();
    assert_eq!(s3, "JAWS 2025\n4 of 11");

    // Moving to another item directly on desktop speaks item-only
    let focus_recycle = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("SysListView32")
        .with_name("Recycle Bin")
        .with_item_index(1)
        .with_item_count(11);
    let s4 = engine
        .handle_focus_event(&focus_recycle)
        .unwrap_or_default();
    assert_eq!(s4, "Recycle Bin\n1 of 11");
}

#[test]
fn window_title_on_the_desktop_is_the_virtual_desktop() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let edit = FocusEvent::new(Role::Edit)
        .with_window_title("Untitled - Notepad")
        .with_window_class("Edit");
    engine.handle_focus_event(&edit);
    assert_eq!(
        engine.execute_command("SayWindowTitle").as_deref(),
        Some("Title is\nUntitled - Notepad")
    );
    // Win+M: the desktop's list takes focus; the tracer names the window after the
    // virtual desktop.
    let desktop = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("SysListView32")
        .with_window_title("Desktop 1")
        .with_name("Recycle Bin")
        .with_item_index(1)
        .with_item_count(3);
    let entry = engine.handle_focus_event(&desktop).unwrap_or_default();
    assert!(entry.starts_with("desktop  \u{1d}  \u{1e} \n"), "{entry:?}");
    assert_eq!(
        engine.execute_command("SayWindowTitle").as_deref(),
        Some("Title is\nDesktop 1")
    );
    // Back in Notepad, its window is announced again.
    let back = engine.handle_focus_event(&edit).unwrap_or_default();
    assert!(back.starts_with("Untitled - Notepad"), "{back:?}");
}

/// Windows search (JAWS on the dev box): Down in the search box selects the next result,
/// which is read with its position; the search box's line is not.
#[test]
fn a_selected_result_replaces_the_fields_read() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut search_box = FocusEvent::new(Role::Edit).with_name("Search box");
    search_box.window_title = Some("Search".to_owned());
    engine.handle_focus_event(&search_box);
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    engine.handle_key_event(&down);
    let result = FocusEvent::new(Role::ListItem)
        .with_name(" notepad, See more search results, Press right to switch preview")
        .with_item_index(2)
        .with_item_count(5);
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::Selected(result))
            .as_deref(),
        Some("notepad, See more search results, Press right to switch preview\n2 of 5")
    );
    // The field's text read after Down speaks nothing now.
    let read = AccessibilityEvent::TextContext(crate::event::TextContext {
        text: "note".to_owned(),
        caret: 4,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    assert_eq!(engine.handle_accessibility_event(&read), None);
}

/// Windows 11 sends the desktop's focus several times, the first from no window class
/// and with the list named "Desktop": it is announced once, as the desktop.
#[test]
fn the_desktop_is_announced_once() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let item = |class: Option<&str>, container: &str| {
        let mut focus = FocusEvent::new(Role::ListItem)
            .with_container_name(container)
            .with_window_title("Desktop 1")
            .with_name("Microsoft Edge")
            .with_item_index(9)
            .with_item_count(11);
        focus.platform.window_class = class.map(str::to_owned);
        focus.platform.top_window_class = Some("Progman".to_owned());
        focus
    };
    let first = engine
        .handle_focus_event(&item(None, "Desktop"))
        .unwrap_or_default();
    assert!(
        first.starts_with("desktop  \u{1d}  \u{1e} \nFolderView  \u{1d} List view \u{1e} "),
        "{first:?}"
    );
    assert_eq!(
        engine.handle_focus_event(&item(Some("Progman"), "FolderView")),
        None
    );
    assert_eq!(
        engine.handle_focus_event(&item(Some("SysListView32"), "FolderView")),
        None
    );
}

#[test]
fn task_switcher_items_after_the_desktop_are_not_desktop_items() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let desktop = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("SysListView32")
        .with_window_title("Desktop 1")
        .with_name("Recycle Bin")
        .with_item_index(1)
        .with_item_count(3);
    engine.handle_focus_event(&desktop);
    // Alt+Tab with Alt held: the switcher's item takes focus through UIA.
    engine.handle_key_event(&key(Key::Alt));
    let mut item = FocusEvent::new(Role::ListItem)
        .with_window_class("ForegroundStaging")
        .with_window_title("Task Switching")
        .with_name("Untitled - Notepad");
    item.platform.top_window_class = Some("MultitaskingViewFrame".to_owned());
    // The switcher's item, not a desktop item ("Untitled - Notepad | 1 of 3").
    assert_eq!(
        engine.handle_focus_event(&item).as_deref(),
        Some("Untitled - Notepad\nTo move to an item press the Arrow keys.")
    );
    // Alt released: the switcher closes on the window chosen, and a switcher focus read
    // after that says nothing [obs, 316/s3].
    engine.handle_key_event(&KeyEvent::new(Key::Alt, Modifiers::none(), false));
    let mut late = item.clone();
    late.name = Some("SR Editor Fixture".to_owned());
    assert_eq!(engine.handle_focus_event(&late), None);
}

#[test]
fn test_engine_file_explorer_entry_and_item_navigation() {
    let aom = AomNode::new(0, Role::Document);
    let buffer = VirtualBuffer::from_aom(&aom);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Desktop item
    let desktop_focus = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("Progman")
        .with_name("Dropbox")
        .with_item_index(7)
        .with_item_count(11);
    let _ = engine.handle_focus_event(&desktop_focus);

    // User presses Enter on Dropbox -> Opens File Explorer window
    let mut explorer_entry = FocusEvent::new(Role::ListItem)
        .with_window_title("Dropbox - File Explorer")
        .with_folder_name("Dropbox")
        .with_container_name("Items View")
        .with_window_class("CabinetWClass")
        .with_name(".cache")
        .with_item_index(1)
        .with_item_count(143);
    explorer_entry.states.selected = Some(false);

    let s1 = engine
        .handle_focus_event(&explorer_entry)
        .unwrap_or_default();
    // As JAWS said it on the dev box ("live - File Explorer | Items View MultiSelect
    // ListBox | Not Selected combo.cs | 1 of 8"); the folder line is this test's own.
    assert_eq!(
        s1,
        "Dropbox - File Explorer\nDropbox\nItems View \u{1d} MultiSelect ListBox \u{1e} \nNot Selected .cache\n1 of 143"
    );

    // Press 'a' inside File Explorer -> ailabs
    let explorer_item = FocusEvent::new(Role::ListItem)
        .with_window_title("Dropbox - File Explorer")
        .with_folder_name("Dropbox")
        .with_container_name("Items View")
        .with_window_class("CabinetWClass")
        .with_name("ailabs")
        .with_item_index(3)
        .with_item_count(143);
    let s2 = engine
        .handle_focus_event(&explorer_item)
        .unwrap_or_default();
    assert_eq!(s2, "ailabs\n3 of 143");

    // Focus shifts back to Desktop -> Full entry announcement re-triggers
    let back_to_desktop = FocusEvent::new(Role::ListItem)
        .with_container_name("FolderView")
        .with_window_class("Progman")
        .with_name("Dropbox")
        .with_item_index(7)
        .with_item_count(11);
    let s3 = engine
        .handle_focus_event(&back_to_desktop)
        .unwrap_or_default();
    assert!(s3.contains("FolderView   List view  "));
    assert!(s3.contains("7 of 11"));
}

#[test]
fn test_engine_native_control_key_passthrough_and_focus_announcement() {
    let empty_doc = AomNode::new(0, Role::Document);
    let buffer = VirtualBuffer::from_aom(&empty_doc);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);
    engine.app_mode = AppMode::NativeControl;

    // In NativeControl mode, arrow keys, Tab, Enter, typing MUST pass through!
    let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);

    let up_event = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&up_event), KeyAction::PassThrough);

    let tab_event = KeyEvent::new(Key::Tab, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&tab_event), KeyAction::PassThrough);

    let t_key = KeyEvent::new(Key::Alpha('t'), Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&t_key), KeyAction::PassThrough);
}

#[test]
fn test_okur_key_combinations_are_consumed_in_native_apps() {
    let empty_doc = AomNode::new(0, Role::Document);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&empty_doc));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.app_mode = AppMode::NativeControl;

    let mut ins_t = KeyEvent::new(Key::Alpha('t'), Modifiers::none(), true);
    ins_t.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&ins_t), KeyAction::Consume);
}

/// Speech shared with the test, for commands spoken from key events.
#[derive(Clone, Default)]
pub(super) struct SharedSpeech(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl SpeechOutput for SharedSpeech {
    fn speak(&mut self, text: &str, _interrupt: bool) {
        if let Ok(mut said) = self.0.lock() {
            said.push(text.to_owned());
        }
    }
    fn stop(&mut self) {}
}

impl SharedSpeech {
    pub(super) fn take(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|mut s| s.drain(..).collect())
            .unwrap_or_default()
    }
}

/// The laptop layout's reading keys in Windows 11 Notepad (JAWS 2025 on the dev box):
/// CapsLock+I "First line of the viewer test", CapsLock+K "First", and Insert+Up the
/// line alone in a multi-line document.
#[test]
fn laptop_reading_keys_in_a_document() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.set_settings(crate::settings::Settings::reference_machine());
    let mut notepad = FocusEvent::new(Role::Document)
        .with_name("Text editor")
        .with_window_title("viewer-test.txt - Notepad");
    notepad.text = Some(crate::event::TextContext {
        text: "First line of the viewer test\rSecond line".to_owned(),
        caret: 2,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&notepad);
    speech.take();
    let caps = |key| {
        let mut event = KeyEvent::new(key, Modifiers::none(), true);
        event.modifiers.caps_lock = true;
        event
    };
    engine.handle_key_event(&caps(Key::Alpha('I')));
    engine.handle_key_event(&caps(Key::Alpha('K')));
    let mut say_line = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
    say_line.modifiers.insert = true;
    engine.handle_key_event(&say_line);
    assert_eq!(
        speech.take(),
        [
            "First line of the viewer test",
            "First",
            "First line of the viewer test"
        ]
    );
    // The commands that move first send the application its arrow key, which comes
    // back through the hook and is read as one.
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    engine.handle_key_event(&caps(Key::Alpha('O')));
    engine.handle_key_event(&caps(Key::Alpha('L')));
    let pressed: Vec<String> = actions
        .requests()
        .into_iter()
        .map(|(action, _)| action)
        .filter(|a| a.starts_with("press"))
        .collect();
    assert_eq!(
        pressed,
        ["press [DownArrow]", "press [Control, RightArrow]"]
    );
}

/// In the laptop layout CapsLock is the JAWS key and Insert works as well; CapsLock
/// pressed twice quickly toggles Caps Lock [obs, 309, 310].
#[test]
fn test_caps_lock_as_the_okur_key() {
    let empty_doc = AomNode::new(0, Role::Document);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&empty_doc));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.app_mode = AppMode::NativeControl;
    engine.last_window_title = Some("SR Editor Fixture".to_owned());

    let caps = |time_ms: u64, repeat: bool| {
        let mut key = KeyEvent::new(Key::CapsLock, Modifiers::none(), true);
        key.modifiers.caps_lock = true;
        key.time_ms = time_ms;
        key.is_repeat = repeat;
        key
    };
    let mut caps_t = KeyEvent::new(Key::Alpha('T'), Modifiers::none(), true);
    caps_t.modifiers.caps_lock = true;

    // Desktop layout (the default): CapsLock is an ordinary key.
    assert_eq!(
        engine.handle_key_event(&caps(1_000, false)),
        KeyAction::PassThrough
    );
    assert_ne!(engine.handle_key_event(&caps_t), KeyAction::Consume);
    assert!(speech.take().is_empty());

    engine.set_settings(crate::settings::Settings::reference_machine());
    assert_eq!(
        engine.handle_key_event(&caps(5_000, false)),
        KeyAction::Consume
    );
    // Held down, the key repeats: that is not a second press.
    assert_eq!(
        engine.handle_key_event(&caps(5_100, true)),
        KeyAction::Consume
    );
    assert_eq!(engine.handle_key_event(&caps_t), KeyAction::Consume);
    assert_eq!(speech.take(), ["Title is\nSR Editor Fixture"]);

    assert_eq!(
        engine.handle_key_event(&caps(9_000, false)),
        KeyAction::Consume
    );
    assert_eq!(
        engine.handle_key_event(&caps(9_300, false)),
        KeyAction::PassThrough
    );
    assert_eq!(speech.take(), ["Caps lock on"]);
    assert_eq!(
        engine.handle_key_event(&caps(12_000, false)),
        KeyAction::Consume
    );
    assert_eq!(
        engine.handle_key_event(&caps(12_300, false)),
        KeyAction::PassThrough
    );
    assert_eq!(speech.take(), ["Caps lock off"]);
}

/// Insert+F12 speaks the time; pressed twice quickly, the time and then the date. With
/// Ctrl, seconds, then the week [obs, 308/s1–s4].
#[test]
fn test_say_system_time_and_its_double_press() {
    use crate::clock::{FixedClock, LocalDateTime, LocaleFormats};
    let empty_doc = AomNode::new(0, Role::Document);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&empty_doc));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.app_mode = AppMode::NativeControl;
    let recorded = LocalDateTime::from_unix_ms(1_792_005_107_069, 180);
    engine.set_clock(Box::new(FixedClock::new(recorded, LocaleFormats::en_us())));

    let press = |engine: &mut ScreenReaderEngine, ctrl: bool, time_ms: u64| {
        let mut key = KeyEvent::new(Key::F(12), Modifiers::none(), true);
        key.modifiers.insert = true;
        key.modifiers.ctrl = ctrl;
        key.time_ms = time_ms;
        assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
    };
    press(&mut engine, false, 1_000);
    press(&mut engine, false, 5_000);
    assert_eq!(speech.take(), ["10:11 PM", "10:11 PM"], "separate presses");
    press(&mut engine, false, 9_000);
    press(&mut engine, false, 9_300);
    assert_eq!(
        speech.take(),
        ["10:11 PM", "Wednesday, October 14, 2026"],
        "twice quickly"
    );
    press(&mut engine, true, 20_000);
    press(&mut engine, true, 20_300);
    assert_eq!(speech.take(), ["10:11:47 PM", "Week 42"]);
    // A third quick press starts over.
    press(&mut engine, true, 20_600);
    assert_eq!(speech.take(), ["10:11:47 PM"]);
}

/// In a text field, the JAWS key is consumed too, and Say Line (Insert+Up) reads the
/// caret's line instead of reaching the field.
#[test]
fn test_okur_key_in_forms_mode_reads_the_caret_line() {
    let mut cursor = VirtualCursor::new(create_test_buffer());
    assert!(cursor.move_to_node_id(3), "the test page has an edit");
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    engine.forms_mode = true;

    let insert = KeyEvent::new(Key::Insert, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&insert), KeyAction::Consume);
    let mut say_line = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
    say_line.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&say_line), KeyAction::Consume);
    assert_eq!(actions.requests().len(), 1, "one read of the field");
    let spoken = engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
        crate::event::TextContext {
            text: "weather today".to_owned(),
            caret: 3,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        },
    ));
    assert_eq!(
        spoken.as_deref(),
        Some("Search Input  \u{1d} Edit \u{1e} \nweather today")
    );
}

#[test]
fn test_engine_dynamic_document_loaded_and_virtual_pc_cursor_navigation() {
    let empty_doc = AomNode::new(0, Role::Document);
    let buffer = VirtualBuffer::from_aom(&empty_doc);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);
    engine.app_mode = AppMode::NativeControl;

    // User switches to Chrome: DocumentLoaded event received
    let web_aom = AomNode::new(0, Role::Document)
        .with_name("ALTAI - Model and Agent Platform")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Main Heading"),
            AomNode::new(2, Role::Link { href: None }).with_name("Documentation Link"),
            AomNode::new(3, Role::Button).with_name("Get Started Button"),
        ]);

    let load_event = AccessibilityEvent::DocumentLoaded {
        title: "ALTAI - Model and Agent Platform".to_string(),
        root: web_aom,
    };

    let announcement = engine
        .handle_accessibility_event(&load_event)
        .unwrap_or_default();
    assert!(announcement.contains("ALTAI - Model and Agent Platform"));
    // JAWS's summary of the page follows its title [obs, 539/s1].
    assert!(announcement.contains("Page has 1heading and 1link"));
    assert_eq!(engine.app_mode, AppMode::VirtualBuffer);

    // Now in VirtualBuffer mode: Down Arrow navigates virtual lines and consumes
    let down_event = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&down_event), KeyAction::Consume);

    // H key moves to heading and consumes
    let h_key = KeyEvent::new(Key::Alpha('h'), Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&h_key), KeyAction::Consume);

    // User switches to Windows Terminal (native control)
    let terminal_focus = FocusEvent::new(Role::Window)
        .with_window_title("Command Prompt")
        .with_name("Command Prompt");
    let term_speech = engine
        .handle_focus_event(&terminal_focus)
        .unwrap_or_default();
    assert!(term_speech.contains("Command Prompt"));
    assert_eq!(engine.app_mode, AppMode::NativeControl);

    // In terminal: Down Arrow immediately passes through!
    assert_eq!(engine.handle_key_event(&down_event), KeyAction::PassThrough);
    // H key immediately passes through!
    assert_eq!(engine.handle_key_event(&h_key), KeyAction::PassThrough);
}

#[test]
fn test_engine_window_change_announcement_in_native_control() {
    let empty_doc = AomNode::new(0, Role::Document);
    let buffer = VirtualBuffer::from_aom(&empty_doc);
    let cursor = VirtualCursor::new(buffer);
    let mock_speech = Box::new(MockSpeechOutput::new());
    let mut engine = ScreenReaderEngine::new(cursor, mock_speech);

    // Focus enters Notepad
    let notepad_focus = FocusEvent::new(Role::Edit)
        .with_window_title("Untitled - Notepad")
        .with_name("Text Editor");
    let s1 = engine
        .handle_focus_event(&notepad_focus)
        .unwrap_or_default();
    assert!(s1.contains("Untitled - Notepad"));
    assert!(s1.contains("Text Editor"));

    // Focus moves to another control within same Notepad window -> Does NOT repeat window title
    let notepad_btn = FocusEvent::new(Role::Button)
        .with_window_title("Untitled - Notepad")
        .with_name("File");
    let s2 = engine.handle_focus_event(&notepad_btn).unwrap_or_default();
    assert!(!s2.contains("Untitled - Notepad"));
    assert!(s2.contains("File"));

    // Focus switches to Command Prompt -> Announces new window title
    let cmd_focus = FocusEvent::new(Role::Window)
        .with_window_title("Command Prompt")
        .with_name("Command Prompt");
    let s3 = engine.handle_focus_event(&cmd_focus).unwrap_or_default();
    assert!(s3.contains("Command Prompt"));
}

/// Local Disk (C:) Properties (JAWS 2025 on the dev box, 2026-10-15): Virtualize Window shows
/// the dialog as it is on screen, read off the keyboard hook; the viewer opens when it
/// arrives, on its first line.
#[test]
fn a_dialog_is_virtualized_from_its_screen() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let actions = crate::actions::RecordingActions::reading_windows();
    engine.set_actions(Box::new(actions.clone()));
    let mut focus = FocusEvent::new(Role::Tab)
        .with_name("General")
        .with_window_title("Local Disk (C:) Properties");
    focus.platform.top_window_class = Some("#32770".to_owned());
    focus.in_dialog_box = true;
    engine.handle_focus_event(&focus);

    assert_eq!(engine.execute_command("VirtualizeWindow"), None);
    assert!(actions
        .requests()
        .iter()
        .any(|(request, _)| request == "read window true"));
    let lines: Vec<String> = [
        "Local Disk (C:) Properties",
        "General Tools Hardware Sharing",
        "Type: Local Disk",
        "OK Cancel Apply",
    ]
    .map(str::to_owned)
    .to_vec();
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::ViewerText(lines))
            .as_deref(),
        Some("Virtualizing current window\nLocal Disk (C:) Properties")
    );
    speech.take();
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    engine.handle_key_event(&down);
    engine.handle_key_event(&down);
    let spoken = speech.take();
    assert!(spoken.iter().any(|s| s == "Type: Local Disk"), "{spoken:?}");

    // Viewer text nobody waits for opens nothing.
    let escape = KeyEvent::new(Key::Escape, Modifiers::none(), true);
    engine.handle_key_event(&escape);
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::ViewerText(vec!["stray".to_owned()])),
        None
    );
}

/// An engine with its Virtual Viewer open on a dialog's three lines, and the window host
/// that shows it.
fn engine_with_open_viewer() -> (ScreenReaderEngine, crate::ui::RecordingUiHost) {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.set_actions(Box::new(crate::actions::RecordingActions::reading_windows()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let focus = FocusEvent::new(Role::Button)
        .with_name("OK")
        .with_window_title("About Windows");
    engine.handle_focus_event(&focus);
    assert_eq!(engine.execute_command("VirtualizeWindow"), None);
    let lines = ["About Windows", "Version 25H2 (OS Build 26200)", "OK"]
        .map(str::to_owned)
        .to_vec();
    engine.handle_accessibility_event(&AccessibilityEvent::ViewerText(lines));
    (engine, ui)
}

fn span(line: usize, column: usize, length: usize) -> crate::ui::TextSpan {
    crate::ui::TextSpan {
        line,
        column,
        length,
    }
}

/// With "Show Virtual Viewer on Screen" on (JAWS's default), the viewer's text shows in a
/// window that marks what was read last; the keys stay with the viewer, and Escape closes
/// the window (JAWS on the dev box, 2026-10-17).
#[test]
fn the_virtual_viewer_shows_on_screen_and_marks_what_was_read() {
    use crate::ui::ToUi;
    let (mut engine, ui) = engine_with_open_viewer();
    assert_eq!(
        ui.take(),
        [ToUi::ViewerShow {
            title: "Virtual Viewer".to_owned(),
            lines: ["About Windows", "Version 25H2 (OS Build 26200)", "OK"]
                .map(str::to_owned)
                .to_vec(),
            mark: span(0, 0, 13),
        }]
    );
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&down), KeyAction::Consume);
    assert_eq!(
        ui.take(),
        [ToUi::ViewerMark {
            mark: span(1, 0, 29)
        }]
    );
    let mut next_word = KeyEvent::new(Key::RightArrow, Modifiers::none(), true);
    next_word.modifiers.ctrl = true;
    engine.handle_key_event(&next_word);
    assert_eq!(
        ui.take(),
        [ToUi::ViewerMark {
            mark: span(1, 8, 4)
        }]
    );
    let escape = KeyEvent::new(Key::Escape, Modifiers::none(), true);
    engine.handle_key_event(&escape);
    assert_eq!(ui.take(), [ToUi::ViewerClose]);
}

/// The viewer's window taking the focus, or closed by its close button, ends the viewer, as
/// JAWS's does: the next key is no viewer key, and Virtualize Window opens a new one.
#[test]
fn a_dismissed_viewer_window_ends_the_viewer() {
    let (mut engine, ui) = engine_with_open_viewer();
    ui.take();
    assert_eq!(
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(
            crate::ui::FromUi::ViewerDismissed
        )),
        None
    );
    // Not "The current location is already in a virtual view": the window is read anew.
    assert_eq!(engine.execute_command("VirtualizeWindow"), None);
    assert!(ui.take().is_empty());
}

/// With the viewer shown by speech only, no window opens.
#[test]
fn a_speech_only_viewer_opens_no_window() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut changed = engine.settings.clone();
    changed.viewer_display = crate::settings::ViewerDisplay::SpeechOnly;
    engine.set_settings(changed);
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let focus = FocusEvent::new(Role::Button)
        .with_name("Save...")
        .with_window_title("SR Dialog Fixture");
    engine.handle_focus_event(&focus);
    assert!(engine.execute_command("VirtualizeWindow").is_some());
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    engine.handle_key_event(&down);
    assert!(ui
        .take()
        .iter()
        .all(|m| !matches!(m, crate::ui::ToUi::ViewerShow { .. })));
}

/// The index of the first item of a dialog's tree whose text starts with `starting`.
fn tree_item_index(dialog: &crate::ui::DialogSpec, starting: &str) -> Option<usize> {
    dialog.controls.iter().find_map(|c| match &c.kind {
        crate::ui::ControlKind::Tree { items, .. } => {
            items.iter().position(|i| i.text.starts_with(starting))
        }
        _ => None,
    })
}

/// Last Changed Settings as kept from the last run, and Remove on its entry: forgotten at once
/// and in the file [obs, the walk of Last Changed Settings].
#[test]
fn last_changed_settings_is_kept_and_its_entries_removed() {
    use crate::ui::settings_tree::{SavedChange, SavedHistory};
    use crate::ui::{FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let kept = |setting: &str| SavedChange {
        application: None,
        site: None,
        setting: setting.into(),
    };
    engine.set_settings_history(&SavedHistory {
        changed: vec![kept("typing_echo"), kept("voice.rate")],
    });
    let store = crate::settings::RecordingSettingsStore::default();
    engine.set_settings_store(Box::new(store.clone()));
    engine.execute_command("SettingsCenter");
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    let id = dialog.id;
    let Some(entry) = tree_item_index(dialog, "Typing Echo (") else {
        panic_free_fail("no Last Changed entry");
        return;
    };
    for message in [
        FromUi::DialogChanged {
            id,
            control: "tree".into(),
            value: Value::Selected(entry),
        },
        FromUi::DialogPressed {
            id,
            control: "lifetime".into(),
        },
        FromUi::DialogMenuChosen {
            id,
            control: "tree".into(),
            index: Some(0),
        },
    ] {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message));
    }
    assert_eq!(
        store.history().map(|h| h.changed),
        Some(vec![kept("voice.rate")])
    );
}

/// Insert+6 opens Settings Center, Insert+V Quick Settings in its place; Space changes the
/// selected setting and OK applies it, keeps it in the settings store, and closes the dialog.
/// Okur says nothing itself: it reads the dialog when the dialog takes the focus.
#[test]
fn settings_center_applies_and_keeps_changes() {
    use crate::ui::{DialogKey, FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let store = crate::settings::RecordingSettingsStore::default();
    engine.set_settings_store(Box::new(store.clone()));
    let mut six = KeyEvent::new(Key::Digit(6), Modifiers::none(), true);
    six.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&six), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(
        dialog.title,
        "Default (All Applications) - Okur Settings Center"
    );
    let id = dialog.id;
    let typing = tree_item_index(dialog, "Typing Echo").unwrap_or_default();
    // Insert+V: Quick Settings in its place.
    assert_eq!(engine.execute_command("QuickSettings"), None);
    let sent = ui.take();
    assert_eq!(sent.first(), Some(&ToUi::DialogClose { id }));
    assert!(
        matches!(sent.get(1), Some(ToUi::DialogShow { dialog }) if dialog.title.starts_with("QuickSettings - "))
    );
    // Back to Settings Center: Typing Echo's next value, then OK.
    engine.execute_command("SettingsCenter");
    // The dialog, then its pane.
    let id = ui
        .take()
        .iter()
        .find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog.id),
            _ => None,
        })
        .unwrap_or_default();
    for message in [
        FromUi::DialogChanged {
            id,
            control: "tree".into(),
            value: Value::Selected(typing),
        },
        FromUi::DialogKey {
            id,
            control: "tree".into(),
            key: DialogKey::Change,
        },
    ] {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message));
    }
    assert_eq!(
        engine.settings.typing_echo,
        crate::settings::TypingEcho::Characters
    );
    ui.take();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".into(),
    }));
    assert_eq!(
        engine.settings.typing_echo,
        crate::settings::TypingEcho::Words
    );
    // Kept for all applications: only what the dialog changed.
    assert_eq!(
        store.take(),
        [crate::settings_layers::SettingsLayers::with_global(
            &engine.settings
        )]
    );
    // Last Changed Settings is kept for the next run too.
    assert_eq!(
        store.history().map(|h| h.changed),
        Some(vec![crate::ui::settings_tree::SavedChange {
            application: None,
            site: None,
            setting: "typing_echo".into(),
        }])
    );
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
    // Cancelled, a new dialog changes nothing.
    engine.execute_command("SettingsCenter");
    let id = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) => dialog.id,
        _ => 0,
    };
    for message in [
        FromUi::DialogChanged {
            id,
            control: "tree".into(),
            value: Value::Selected(typing),
        },
        FromUi::DialogKey {
            id,
            control: "tree".into(),
            key: DialogKey::Change,
        },
        FromUi::DialogClosed { id },
    ] {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message));
    }
    assert_eq!(
        engine.settings.typing_echo,
        crate::settings::TypingEcho::Words
    );
    assert!(store.take().is_empty());
}

/// Settings Center's search results: Down from the search box into them, each result said
/// with its group after it and no place in the list, the list entered with its own hints;
/// Escape says "Search edit box cleared." before the tree [obs, the walk of Settings Center's
/// search].
#[test]
fn settings_centers_results_are_said_as_jaws_says_them() {
    use crate::ui::{FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.execute_command("SettingsCenter");
    let Some((id, title)) = ui.take().iter().find_map(|m| match m {
        ToUi::DialogShow { dialog } => Some((dialog.id, dialog.title.clone())),
        _ => None,
    }) else {
        panic_free_fail("no dialog");
        return;
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "search".into(),
        value: Value::Text("typing echo".into()),
    }));
    let sent = ui.take();
    let (Some(first), Some(label)) = (
        sent.iter().find_map(|m| match m {
            ToUi::DialogItems { items, .. } => items.first().cloned(),
            _ => None,
        }),
        sent.iter().find_map(|m| match m {
            ToUi::DialogLabel { label, .. } => Some(label.clone()),
            _ => None,
        }),
    ) else {
        panic_free_fail(&format!("no results: {sent:?}"));
        return;
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "search_down".into(),
    }));
    let mut result = FocusEvent::new(Role::ListItem).with_name(first.clone());
    result.window_title = Some(title.clone());
    result.container_role = Some(Role::List);
    result.container_name = Some(label.clone());
    result.platform.container_class = Some("LISTBOX".into());
    result.item_index = Some(1);
    result.item_count = Some(1);
    // After the window's title, this being the first focus in it here.
    let said = engine.handle_focus_event(&result).unwrap_or_default();
    assert_eq!(
        norm(&said).strip_prefix(&norm(&title)).map(str::trim),
        Some(norm(&format!(
            "{label} \u{1d} List box \u{1e} \n{first} \nUser\nUse the arrow keys to select an \
             item.\nPress the space bar to toggle between the choices"
        )))
        .as_deref()
    );
    // Escape: the tree, after "Search edit box cleared.".
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "result_escape".into(),
    }));
    assert!(ui
        .take()
        .iter()
        .any(|m| matches!(m, ToUi::DialogFocus { control, .. } if control == "tree")));
    let mut user = FocusEvent::new(Role::TreeItem).with_name("User");
    user.window_title = Some(title);
    user.tree_level = Some(0);
    let said = engine.handle_focus_event(&user).unwrap_or_default();
    assert!(said.starts_with("Search edit box cleared.\n"), "{said}");
}

/// Settings Center's Configure Verbosity Levels opens the verbosity levels' dialog over it.
#[test]
fn configure_verbosity_levels_opens_its_dialog() {
    use crate::ui::{DialogKey, FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.execute_command("SettingsCenter");
    let Some(ToUi::DialogShow { dialog }) = ui.take().first().cloned() else {
        panic_free_fail("no dialog shown");
        return;
    };
    let levels = tree_item_index(&dialog, "Configure Verbosity Levels").unwrap_or_default();
    for message in [
        FromUi::DialogChanged {
            id: dialog.id,
            control: "tree".into(),
            value: Value::Selected(levels),
        },
        FromUi::DialogKey {
            id: dialog.id,
            control: "tree".into(),
            key: DialogKey::Change,
        },
    ] {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message));
    }
    assert!(ui
        .take()
        .iter()
        .any(|m| matches!(m, ToUi::DialogShow { dialog } if dialog.title == "Okur Settings")));
}

/// Fails a test without the panic macros the commit gate refuses.
fn panic_free_fail(message: &str) {
    assert!(message.is_empty(), "{message}");
}

/// Without Okur's window process, the settings dialog says it cannot open.
#[test]
fn without_okur_ui_the_settings_dialog_says_so() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    assert_eq!(
        engine.execute_command("SettingsCenter").as_deref(),
        Some("The settings dialog is not available")
    );
}

/// When the screen gives nothing, the viewer shows what the focus says, as on old-pc
/// [obs, 311/s1]: the control's name and type, then a blank line.
#[test]
fn a_window_with_no_screen_text_shows_its_focus() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.set_actions(Box::new(crate::actions::RecordingActions::reading_windows()));
    let focus = FocusEvent::new(Role::Button)
        .with_name("Save...")
        .with_window_title("SR Dialog Fixture");
    engine.handle_focus_event(&focus);
    assert_eq!(engine.execute_command("VirtualizeWindow"), None);
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::ViewerText(Vec::new()))
            .as_deref(),
        Some("Virtualizing current window\nSave... Button")
    );
}

/// A form's one-line field is one row of its window: Virtualize Window reads the window
/// (JAWS 2025 on the dev box shows the whole WinForms form), Virtualize Current Control the
/// field's text.
#[test]
fn a_forms_field_is_virtualized_with_its_window() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::reading_windows();
    engine.set_actions(Box::new(actions.clone()));
    let mut name = FocusEvent::new(Role::Edit)
        .with_name("Name:")
        .with_window_title("SR Desktop Fixture");
    name.text = Some(crate::event::TextContext {
        text: "Ada".to_owned(),
        caret: 3,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&name);
    assert_eq!(engine.execute_command("VirtualizeWindow"), None);
    assert!(actions
        .requests()
        .iter()
        .any(|(request, _)| request == "read window true"));
    engine.handle_accessibility_event(&AccessibilityEvent::ViewerText(Vec::new()));
    let escape = KeyEvent::new(Key::Escape, Modifiers::none(), true);
    engine.handle_key_event(&escape);
    assert_eq!(
        engine.execute_command("VirtualizeControl").as_deref(),
        Some("Virtualizing current control\nAda")
    );
}

/// A text editor's window shows the text's lines (Notepad, where JAWS shows "paragraph"):
/// the screen is not read.
#[test]
fn a_text_editor_is_virtualized_from_its_text() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::reading_windows();
    engine.set_actions(Box::new(actions.clone()));
    let edit = FocusEvent::new(Role::Edit)
        .with_name("Text editor")
        .with_window_title("Untitled - Notepad");
    engine.handle_focus_event(&edit);
    let context = crate::event::TextContext {
        text: "First line\rSecond line".to_owned(),
        caret: 0,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context));
    assert_eq!(
        engine.execute_command("VirtualizeWindow").as_deref(),
        Some("Virtualizing current window\nUntitled - Notepad")
    );
    assert!(!actions
        .requests()
        .iter()
        .any(|(request, _)| request.starts_with("read window")));
}

/// Windows Terminal (JAWS 2025 on the dev box, 2026-10-15): the Virtual Viewer shows the
/// console's text, opens at the prompt, and is titled "Virtual Viewer".
#[test]
fn a_console_is_virtualized_from_its_text_at_the_prompt() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut focus = FocusEvent::new(Role::Text)
        .with_name("Command Prompt")
        .with_window_title("Command Prompt");
    focus.platform.element_class = Some("TermControl".to_owned());
    engine.handle_focus_event(&focus);

    // Nothing is spoken until the console's text is read.
    assert_eq!(engine.execute_command("VirtualizeWindow"), None);
    assert!(actions
        .requests()
        .iter()
        .any(|(request, _)| request == "read text"));
    let text = "Hello from the console     \r\n\
                Second line                \r\n\
                C:\\Users\\Gigabyte>         \r\n\
                \x20                          \r\n";
    let caret = text.find("C:\\").unwrap_or_default() + "C:\\Users\\Gigabyte>".len();
    let context = crate::event::TextContext {
        text: text.to_owned(),
        caret,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::TextContext(context))
            .as_deref(),
        Some("Virtualizing current window\nC:\\Users\\Gigabyte>")
    );
    assert_eq!(
        engine.execute_command("SayWindowTitle").as_deref(),
        Some("Virtual Viewer")
    );
    let home = KeyEvent::new(
        Key::Home,
        Modifiers {
            ctrl: true,
            ..Modifiers::none()
        },
        true,
    );
    speech.take();
    engine.handle_key_event(&home);
    let up = KeyEvent::new(Key::UpArrow, Modifiers::none(), true);
    engine.handle_key_event(&up);
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    engine.handle_key_event(&down);
    let spoken = speech.take();
    assert!(
        spoken.iter().any(|s| s == "Hello from the console"),
        "{spoken:?}"
    );
    assert!(spoken.iter().any(|s| s == "Second line"), "{spoken:?}");

    // Escape closes the viewer and reads the console's cursor line again.
    let escape = KeyEvent::new(Key::Escape, Modifiers::none(), true);
    engine.handle_key_event(&escape);
    assert_eq!(speech.take(), vec!["C:\\Users\\Gigabyte>".to_owned()]);
}

/// A live region event for region `reply` with `text`, named "Reply", polite.
fn reply_region(text: &str) -> LiveRegionEvent {
    let mut live = LiveRegionEvent::new(text, Politeness::Polite).with_container("Reply");
    live.region_id = Some("reply".to_owned());
    live
}

fn engine_with_speech() -> (ScreenReaderEngine, SharedSpeech) {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = SharedSpeech::default();
    let engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    (engine, speech)
}

/// A streamed reply is read once, piece by piece: the region's name before the first piece,
/// each piece from where the last ended, and a word still growing only once it is whole.
#[test]
fn a_streamed_live_region_is_read_as_it_grows() {
    let (mut engine, speech) = engine_with_speech();
    for text in [
        "This",
        "This reply arr",
        "This reply arrives a",
        "This reply arrives a few words.",
    ] {
        engine.handle_live_region_event(&reply_region(text));
    }
    assert_eq!(
        speech.take(),
        vec!["Reply\nThis", "reply", "arrives", "a few words."]
    );
}

/// The last word of a reply that stops inside it is read once the region has been quiet.
#[test]
fn a_streamed_reply_ends_with_its_held_back_word_when_quiet() {
    let (mut engine, speech) = engine_with_speech();
    engine.handle_live_region_event(&reply_region("Hello"));
    engine.handle_live_region_event(&reply_region("Hello wor"));
    engine.handle_live_region_event(&reply_region("Hello world"));
    assert_eq!(speech.take(), vec!["Reply\nHello"]);
    for _ in 0..3 {
        engine.on_idle();
    }
    assert_eq!(speech.take(), vec!["world"]);
    // Read once: more quiet says nothing more.
    engine.on_idle();
    assert!(speech.take().is_empty());
}

/// A new message on a line of its own is read whole at once, its last word included.
#[test]
fn a_new_message_in_a_live_region_is_read_whole() {
    let (mut engine, speech) = engine_with_speech();
    engine.handle_live_region_event(&reply_region("You: Message 1"));
    engine.handle_live_region_event(&reply_region("You: Message 1\nYou: Message 2"));
    assert_eq!(
        speech.take(),
        vec!["Reply\nYou: Message 1", "You: Message 2"]
    );
}

/// JAWS reads a growing region's whole text on every change (dev box, 2026-10-15); the
/// reference machine's settings keep that, so replay compares with JAWS.
#[test]
fn jaws_mode_reads_a_growing_region_whole_each_time() {
    let (mut engine, speech) = engine_with_speech();
    engine.set_settings(crate::settings::Settings::reference_machine());
    engine.handle_live_region_event(&reply_region("This"));
    engine.handle_live_region_event(&reply_region("This reply"));
    assert_eq!(speech.take(), vec!["Reply\nThis", "Reply\nThis reply"]);
}

/// Speech that reports itself busy while the test says so.
#[derive(Clone, Default)]
struct BusySpeech {
    said: SharedSpeech,
    busy: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl SpeechOutput for BusySpeech {
    fn speak(&mut self, text: &str, interrupt: bool) {
        self.said.speak(text, interrupt);
    }
    fn stop(&mut self) {}
    fn is_speaking(&self) -> bool {
        self.busy.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// While a piece is still being spoken, the stream gathers, and is read as one phrase once
/// speech is free.
#[test]
fn a_fast_stream_is_read_in_phrases_as_speech_frees() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = BusySpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_live_region_event(&reply_region("This"));
    assert_eq!(speech.said.take(), vec!["Reply\nThis"]);

    speech.busy.store(true, std::sync::atomic::Ordering::SeqCst);
    for text in [
        "This reply",
        "This reply arrives",
        "This reply arrives a few",
    ] {
        engine.handle_live_region_event(&reply_region(text));
    }
    engine.on_idle();
    assert!(speech.said.take().is_empty());

    speech
        .busy
        .store(false, std::sync::atomic::Ordering::SeqCst);
    engine.on_idle();
    assert_eq!(speech.said.take(), vec!["reply arrives a"]);
    engine.on_idle();
    engine.on_idle();
    assert_eq!(speech.said.take(), vec!["few"]);
}

/// Tab with the virtual cursor moved away from the focus moves on from the cursor, as JAWS's
/// does (dev box, 2026-10-16): the page's next tab stop is focused; Shift+Tab, the prior one.
#[test]
fn tab_moves_on_from_the_virtual_cursor() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Slides")
        .with_children(vec![
            AomNode::new(1, Role::Button).with_name("Next slide"),
            AomNode::new(
                2,
                Role::Link {
                    href: Some("#x".to_owned()),
                },
            )
            .with_name("Rust"),
            AomNode::new(3, Role::Heading { level: 2 }).with_text("Inside"),
            AomNode::new(4, Role::Edit).with_name("Code"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut focus = FocusEvent::new(Role::Button)
        .with_node_id(1)
        .with_name("Next slide");
    focus.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    let key = |key: Key, shift: bool| {
        let modifiers = Modifiers {
            shift,
            ..Modifiers::none()
        };
        KeyEvent::new(key, modifiers, true)
    };
    // On the focused button, Tab is the browser's.
    assert_eq!(
        engine.handle_key_event(&key(Key::Tab, false)),
        KeyAction::PassThrough
    );
    engine.handle_key_event(&key(Key::DownArrow, false));
    engine.handle_key_event(&key(Key::DownArrow, false));
    assert_eq!(
        engine.handle_key_event(&key(Key::Tab, false)),
        KeyAction::Consume
    );
    assert_eq!(
        engine.handle_key_event(&key(Key::Tab, true)),
        KeyAction::Consume
    );
    assert_eq!(
        actions.requests(),
        vec![("focus".to_owned(), 4), ("focus".to_owned(), 2)]
    );
}

/// An autocomplete (a combo box one types in, its options pointed at with
/// aria-activedescendant), JAWS on the dev box (2026-10-16): the list on the way in, each
/// option with its position, forms mode kept, and the field again once an option is chosen.
#[test]
fn an_autocompletes_options_are_read_and_forms_mode_stays() {
    let mut combo = AomNode::new(1, Role::ComboBox).with_name("Fruit");
    combo
        .attributes
        .insert("editable".to_owned(), "true".to_owned());
    combo.states.expanded = Some(false);
    let page = AomNode::new(0, Role::Document)
        .with_name("Search")
        .with_children(vec![combo]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let field = |value: &str| {
        let mut focus = FocusEvent::new(Role::ComboBox)
            .with_node_id(1)
            .with_name("Fruit");
        focus.in_document = true;
        focus.value = Some(value.to_owned());
        focus.states.expanded = Some(false);
        AccessibilityEvent::Focus(focus)
    };
    let option = |name: &str, index: usize| {
        let mut focus = FocusEvent::new(Role::ListItem).with_name(name);
        focus.in_document = true;
        focus.container_name = Some("Fruits".to_owned());
        focus.item_index = Some(index);
        focus.item_count = Some(3);
        AccessibilityEvent::Focus(focus)
    };
    engine.handle_accessibility_event(&field(""));
    engine.forms_mode = true;
    let hint = "To move to an item press the Arrow keys.";
    assert_eq!(
        engine.handle_accessibility_event(&option("Apple", 1)),
        Some(format!(
            "Fruits  \u{1d} list \u{1e} \nwith 3 items\nApple\n1 of 3\n{hint}"
        ))
    );
    // Down moves to the next option, which its focus says: the field is not read.
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&down), KeyAction::PassThrough);
    assert!(actions.requests().is_empty());
    assert_eq!(
        engine.handle_accessibility_event(&option("Apricot", 2)),
        Some(format!("Apricot\n2 of 3\n{hint}"))
    );
    assert!(engine.forms_mode);
    // Enter chose it: the field again, with its new value.
    assert_eq!(
        engine.handle_accessibility_event(&field("Apricot")),
        Some(
            "Fruit  \u{1d} edit combo \u{1e}   \u{1d}collapsed\u{1e} \nApricot\n\
             To set the value use the Arrow keys or type the value."
                .to_owned()
        )
    );
}

/// A page's application region (ARIA role "application", VS Code's workbench), JAWS on the
/// dev box (2026-10-16): its name on the way in, and every key is the page's, Escape too.
#[test]
fn an_application_region_takes_every_key() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Tools")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 2 }).with_text("Before"),
            AomNode::new(2, Role::Button).with_name("First tool"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut focus = FocusEvent::new(Role::Button)
        .with_node_id(2)
        .with_name("First tool");
    focus.in_document = true;
    focus.web_application = Some("Workbench".to_owned());
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(focus))
        .unwrap_or_default();
    assert!(said.contains("Workbench\nFirst tool"), "{said}");
    for key in [Key::Alpha('h'), Key::DownArrow, Key::Escape] {
        let event = KeyEvent::new(key, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&event), KeyAction::PassThrough);
    }
    // A button's keys are the page's to answer: no field is read for them.
    assert!(actions.requests().is_empty());
    assert!(engine.forms_mode);
}

/// Focus entering a list in a navigation region from where the virtual cursor was names
/// them, as JAWS does (dev box, GitHub's README tab: "Repository files NavigationRegion |
/// list | with 1 items | README ...").
#[test]
fn focus_names_the_lists_and_regions_it_enters() {
    let link = AomNode::new(
        4,
        Role::Link {
            href: Some("#readme".to_owned()),
        },
    )
    .with_name("README");
    let nav = AomNode::new(
        2,
        Role::Landmark {
            landmark_type: "navigation".to_owned(),
        },
    )
    .with_name("Repository files")
    .with_children(vec![AomNode::new(3, Role::List).with_children(vec![
        AomNode::new(5, Role::ListItem).with_children(vec![link]),
    ])]);
    let page = AomNode::new(0, Role::Document)
        .with_name("Repo")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 2 }).with_text("Files"),
            nav,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut heading = FocusEvent::new(Role::Heading { level: 2 })
        .with_node_id(1)
        .with_name("Files");
    heading.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(heading));
    let mut focus = FocusEvent::new(Role::Link {
        href: Some("#readme".to_owned()),
    })
    .with_node_id(4)
    .with_name("README");
    focus.in_document = true;
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(focus))
        .unwrap_or_default();
    assert!(
        said.starts_with(
            "Repository files \u{1d} NavigationRegion \u{1e} \n \u{1d} list \u{1e} \nwith 1 items\nREADME"
        ),
        "{said}"
    );
}

/// A page read quickly and then in full: the full capture knows more of the focused control
/// (an autocomplete is an "edit combo" only through IA2), so its line's text changes. A cursor
/// on the focus stays on it, found by the focus's state, and Enter acts on it, not on whatever
/// line now sits at the old offset (it clicked the next button, live on the dev box).
#[test]
fn a_cursor_on_the_focus_stays_on_it_when_the_page_is_captured_again() {
    let page = |editable: bool| {
        let mut combo = AomNode::new(3, Role::ComboBox).with_name("Fruit");
        combo.states.focused = true;
        if editable {
            combo
                .attributes
                .insert("editable".to_owned(), "true".to_owned());
        }
        // The full capture also has a line above (a status that appeared meanwhile).
        let mut children = Vec::new();
        if editable {
            children.push(AomNode::new(7, Role::Paragraph).with_text("Three results."));
        }
        children.extend([
            AomNode::new(1, Role::Heading { level: 2 }).with_text("Fruit search"),
            combo,
            AomNode::new(5, Role::Button).with_name("Pick one"),
        ]);
        AomNode::new(0, Role::Document)
            .with_name("Search")
            .with_children(children)
    };
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.handle_document_loaded("Search", VirtualBuffer::from_aom(&page(false)));
    let mut focus = FocusEvent::new(Role::ComboBox)
        .with_node_id(3)
        .with_name("Fruit");
    focus.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    engine.handle_document_loaded("Search", VirtualBuffer::from_aom(&page(true)));
    engine.on_idle();
    let line = engine
        .cursor
        .buffer
        .line_at_offset(engine.cursor.offset)
        .map(|l| (l.node_id, l.text.clone()));
    // Its role and state are speech, not text [obs, 977].
    assert_eq!(line, Some((3, "Fruit".to_owned())));
    assert_eq!(engine.cursor.current_node_id, Some(3));
}

/// Leaving a page while its autocomplete's list was open (Alt+Tab to Notepad): the native
/// field's Up and Down are read again, not left to an option focus that never comes.
#[test]
fn leaving_an_autocomplete_for_a_native_field_reads_its_lines_again() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Search")
        .with_children(vec![AomNode::new(1, Role::ComboBox).with_name("Fruit")]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    engine.forms_mode = true;
    let mut option = FocusEvent::new(Role::ListItem).with_name("Apple");
    option.in_document = true;
    option.container_name = Some("Fruits".to_owned());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(option));
    let notepad = FocusEvent::new(Role::Edit)
        .with_name("Text editor")
        .with_window_title("Untitled - Notepad");
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(notepad));
    let down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&down), KeyAction::PassThrough);
    assert_eq!(actions.requests(), vec![("caret".to_owned(), 0)]);
}

/// The other landmarks' names on the way in, JAWS on the dev box (2026-10-16): "BannerRegion",
/// "Related ComplementaryRegion", "Site SearchRegion", "Updates Region", "ContentInfoRegion".
#[test]
fn focus_names_every_landmark_it_enters() {
    let landmark = |id: usize, kind: &str, name: Option<&str>| {
        let link = AomNode::new(
            id + 1,
            Role::Link {
                href: Some("#x".to_owned()),
            },
        )
        .with_name("Go");
        let mut node = AomNode::new(
            id,
            Role::Landmark {
                landmark_type: kind.to_owned(),
            },
        )
        .with_children(vec![link]);
        node.name = name.map(str::to_owned);
        node
    };
    let page = AomNode::new(0, Role::Document)
        .with_name("Landmarks")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 2 }).with_text("Start"),
            landmark(10, "banner", None),
            landmark(20, "complementary", Some("Related")),
            landmark(30, "search", Some("Site")),
            landmark(40, "region", Some("Updates")),
            landmark(50, "contentinfo", None),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let focus = |id: usize| {
        let mut focus = FocusEvent::new(Role::Link {
            href: Some("#x".to_owned()),
        })
        .with_node_id(id)
        .with_name("Go");
        focus.in_document = true;
        AccessibilityEvent::Focus(focus)
    };
    let mut heading = FocusEvent::new(Role::Heading { level: 2 })
        .with_node_id(1)
        .with_name("Start");
    heading.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(heading));
    let said: Vec<String> = [11, 21, 31, 41, 51]
        .iter()
        .map(|&id| {
            engine
                .handle_accessibility_event(&focus(id))
                .unwrap_or_default()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .collect();
    assert_eq!(
        said,
        [
            " \u{1d} BannerRegion \u{1e} ",
            "Related \u{1d} ComplementaryRegion \u{1e} ",
            "Site \u{1d} SearchRegion \u{1e} ",
            "Updates \u{1d} Region \u{1e} ",
            " \u{1d} ContentInfoRegion \u{1e} ",
        ]
    );
}

/// A files table under a "Files" heading: Name and Message columns, a link in each cell.
fn files_table_engine() -> ScreenReaderEngine {
    let header = |id: usize, text: &str| {
        let mut cell = AomNode::new(id, Role::TableCell).with_text(text);
        cell.attributes.insert("tag".to_owned(), "th".to_owned());
        cell.attributes.insert("scope".to_owned(), "col".to_owned());
        cell
    };
    let link_cell = |id: usize, text: &str| {
        AomNode::new(id, Role::TableCell).with_children(vec![AomNode::new(
            id + 1,
            Role::Link {
                href: Some("#x".to_owned()),
            },
        )
        .with_name(text)])
    };
    let table = AomNode::new(1, Role::Table).with_children(vec![
        AomNode::new(2, Role::TableRow)
            .with_children(vec![header(3, "Name"), header(4, "Message")]),
        AomNode::new(5, Role::TableRow).with_children(vec![
            link_cell(10, "alpha.rs"),
            link_cell(20, "first commit"),
        ]),
        AomNode::new(6, Role::TableRow).with_children(vec![
            link_cell(30, "beta.rs"),
            link_cell(40, "second commit"),
        ]),
    ]);
    let page = AomNode::new(0, Role::Document)
        .with_name("Files")
        .with_children(vec![
            AomNode::new(90, Role::Heading { level: 2 }).with_text("Files"),
            table,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()))
}

/// The lines the focus on the files table's link `id` speaks.
fn link_focus_lines(engine: &mut ScreenReaderEngine, id: usize, name: &str) -> Vec<String> {
    let mut focus = FocusEvent::new(Role::Link {
        href: Some("#x".to_owned()),
    })
    .with_node_id(id)
    .with_name(name);
    focus.in_document = true;
    engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(focus))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Tab through links in a table, JAWS on the dev box (2026-10-16): the column header when the
/// column changes, then the position that changed after the link.
#[test]
fn tab_in_a_table_says_the_header_and_position_that_changed() {
    let mut engine = files_table_engine();
    let mut heading = FocusEvent::new(Role::Heading { level: 2 })
        .with_node_id(90)
        .with_name("Files");
    heading.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(heading));
    let mut said = |id: usize, name: &str| {
        link_focus_lines(&mut engine, id, name)
            .into_iter()
            .filter(|l| !l.contains("link") && !l.starts_with("To activate"))
            .collect::<Vec<_>>()
    };
    assert_eq!(said(11, "alpha.rs"), ["Name", "column 1 row 2"]);
    assert_eq!(said(21, "first commit"), ["Message", "Column 2"]);
    assert_eq!(said(41, "second commit"), ["Row 3"]);
}

/// The page's first focus on a link is said as the virtual cursor's line, with the page title
/// and no table [obs, 216/s10]; the next focus then says the table as entered: "Message |
/// first commit same page link | column 2 row 2" [obs, 216/s11].
#[test]
fn a_first_focus_on_a_link_is_said_as_its_line() {
    let mut engine = files_table_engine();
    let first = link_focus_lines(&mut engine, 11, "alpha.rs");
    assert_eq!(first.first().map(String::as_str), Some("Files"));
    assert!(first
        .iter()
        .any(|l| l.contains("link") && l.contains("alpha.rs")));
    assert!(
        !first.iter().any(|l| l.contains("column") || l == "Name"),
        "{first:?}"
    );
    let next = link_focus_lines(&mut engine, 21, "first commit");
    assert_eq!(next.first().map(String::as_str), Some("Message"));
    assert!(next.iter().any(|l| l == "column 2 row 2"), "{next:?}");
}

/// Google Docs draws its document itself, and keys go to an empty element in an application
/// region, where JAWS's Say Font says "No font" (dev box, 2026-10-16). Okur reads the font
/// at the caret from the toolbar, as it is when Insert+F is pressed, and says it as JAWS says
/// a font in Notepad.
#[test]
fn say_font_in_google_docs_reads_the_toolbar() {
    let mut page_root =
        AomNode::new(0, Role::Document).with_name("Untitled document - Google Docs");
    page_root.attributes.insert(
        "url".to_owned(),
        "https://docs.google.com/document/d/1XYG/edit?tab=t.0".to_owned(),
    );
    let mut target = AomNode::new(2, Role::Group);
    target.attributes.insert(
        "id".to_owned(),
        "docs-texteventtarget-descendant".to_owned(),
    );
    let page = page_root.with_children(vec![
        AomNode::new(1, Role::Heading { level: 1 }).with_text("Untitled document"),
        target,
    ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let actions = crate::actions::RecordingActions::reading_page_elements();
    engine.set_actions(Box::new(actions.clone()));
    let mut focus = FocusEvent::new(Role::Group).with_node_id(2);
    focus.in_document = true;
    focus.web_application = Some(String::new());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    speech.take();

    // The page captured again after Docs' announcements, numbered anew: the menu bar is
    // still "focused" through IA2 after a menu closed, before the document's text target.
    let focused = crate::aom::NodeStates {
        focused: true,
        ..Default::default()
    };
    let mut menu_bar = AomNode::new(11, Role::Group).with_states(focused.clone());
    menu_bar
        .attributes
        .insert("id".to_owned(), "docs-menubar".to_owned());
    let mut target = AomNode::new(12, Role::Group).with_states(focused);
    target.attributes.insert(
        "id".to_owned(),
        "docs-texteventtarget-descendant".to_owned(),
    );
    let mut again = page.clone();
    again.children = vec![
        AomNode::new(10, Role::Heading { level: 1 }).with_text("Untitled document"),
        menu_bar,
        target,
    ];
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "Untitled document - Google Docs".to_owned(),
        root: again,
    });

    let insert_f = KeyEvent::new(
        Key::Alpha('f'),
        Modifiers {
            insert: true,
            ..Modifiers::none()
        },
        true,
    );
    engine.handle_key_event(&insert_f);
    assert!(speech.take().is_empty());
    assert!(
        actions.requests().iter().any(|(request, _)| request
            == "read page elements docs-font-family fontSizeSelect boldButton italicButton underlineButton"),
        "{:?}",
        actions.requests()
    );
    let with_id = |node: AomNode, id: &str| {
        let mut node = node;
        node.attributes.insert("id".to_owned(), id.to_owned());
        node
    };
    let selected = crate::aom::NodeStates {
        selected: Some(true),
        ..Default::default()
    };
    let pressed = crate::aom::NodeStates {
        checked: Some(true),
        ..Default::default()
    };
    let elements = vec![
        with_id(
            AomNode::new(10, Role::List).with_name("Font"),
            "docs-font-family",
        )
        .with_children(vec![AomNode::new(11, Role::ListItem)
            .with_name("Font list. Arial selected.")
            .with_states(selected)]),
        with_id(
            AomNode::new(12, Role::ComboBox).with_name("Font size"),
            "fontSizeSelect",
        )
        .with_children(vec![AomNode::new(13, Role::Group)
            .with_name("Font size list. 11 selected.")
            .with_children(vec![AomNode::new(14, Role::Edit)
                .with_name("Font size")
                .with_value("11")])]),
        with_id(
            AomNode::new(15, Role::Button).with_name("Bold (Ctrl+B)"),
            "boldButton",
        )
        .with_states(pressed),
    ];
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::PageElements(elements.clone()))
            .as_deref(),
        Some("Font is bold  Arial 11 point")
    );
    // Elements nobody waits for say nothing.
    assert_eq!(
        engine.handle_accessibility_event(&AccessibilityEvent::PageElements(elements)),
        None
    );
}

/// An atomic region (aria-atomic) filled again with the same text says it again: Google Docs
/// says the character the caret passes over in one, so Right then Left over the same letter
/// says it twice. What it adds is read alone: Backspace adds the deleted character to what
/// Docs said last, and only the deleted one is said, as Notepad's is (dev box, 2026-10-16:
/// JAWS reads the region whole, "e r"). A region that is not atomic does not repeat itself.
#[test]
fn an_atomic_live_region_says_a_repeat_and_its_additions() {
    let (mut engine, speech) = engine_with_speech();
    let docs = |text: &str| {
        let mut live = LiveRegionEvent::new(text, Politeness::Assertive);
        live.region_id = Some("docs-aria-speakable".to_owned());
        live.atomic = true;
        AccessibilityEvent::LiveRegion(live)
    };
    for text in ["o", "o", "e", "e\nr", "e\nr\ne"] {
        engine.handle_accessibility_event(&docs(text));
    }
    assert_eq!(speech.take(), ["o", "o", "e", "r", "e"].map(str::to_owned));

    for _ in 0..2 {
        engine.handle_accessibility_event(&AccessibilityEvent::LiveRegion(reply_region("Done")));
    }
    assert_eq!(speech.take(), vec!["Reply\nDone".to_owned()]);
}

/// A page's menu (Google Docs' Accessibility menu, dev box 2026-10-16): the browser opens the
/// popup before an item has the focus, reports each item twice, and ends no menu mode. The
/// first item is said as JAWS says it ("menu | Accessibility | 10 of 10 | To move through
/// items press left or right arrow."), each item once, and the focus back in the page says
/// "Leaving menus", as JAWS does; nothing about a sub menu follows.
#[test]
fn a_page_menu_is_left_when_the_focus_returns_to_the_page() {
    let mut target = AomNode::new(2, Role::Group);
    target.attributes.insert(
        "id".to_owned(),
        "docs-texteventtarget-descendant".to_owned(),
    );
    let page = AomNode::new(0, Role::Document)
        .with_name("Untitled document - Google Docs")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Untitled document"),
            target,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let mut text_target = FocusEvent::new(Role::Group).with_node_id(2);
    text_target.in_document = true;
    text_target.web_application = Some(String::new());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(text_target.clone()));
    speech.take();

    let item = |name: &str, on_bar: bool| {
        let mut focus = FocusEvent::new(Role::MenuItem).with_name(name);
        focus.in_document = true;
        focus.in_menu_bar = on_bar;
        if on_bar {
            focus.item_index = Some(10);
            focus.item_count = Some(10);
        }
        AccessibilityEvent::Focus(focus)
    };
    let popup = AccessibilityEvent::Menu(crate::event::MenuEvent::PopupStart);
    engine.handle_accessibility_event(&popup);
    engine.handle_accessibility_event(&item("Accessibility", true));
    engine.handle_accessibility_event(&item("Accessibility", true));
    engine.handle_accessibility_event(&item("Verbalize selection s", false));
    engine.handle_accessibility_event(&popup);
    let spoken: Vec<String> = speech
        .take()
        .iter()
        .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect();
    assert_eq!(
        spoken,
        vec![
            "\u{1d} menu \u{1e} Accessibility 10 of 10 \
             To move through items press left or right arrow.",
            "Verbalize selection s",
        ]
    );

    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::Focus(text_target))
            .as_deref(),
        Some("Leaving menus")
    );
    let end = AccessibilityEvent::Menu(crate::event::MenuEvent::PopupEnd);
    engine.handle_accessibility_event(&end);
    engine.handle_accessibility_event(&end);
    assert_eq!(engine.on_idle(), None);
}

/// A focus with nothing of it in the buffer (Google Docs' empty element it types into) is
/// said by its name and, for a group, its role ("group", as JAWS said for Google Sheets'
/// grid): entering the page says the title once, not the title again as the line the virtual
/// cursor was on (dev box, 2026-10-16).
#[test]
fn an_empty_focus_is_not_read_as_the_cursors_line() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Untitled document - Google Docs")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Toolbar text"),
            AomNode::new(2, Role::Group),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut focus = FocusEvent::new(Role::Group).with_node_id(2);
    focus.in_document = true;
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(focus))
        .unwrap_or_default();
    assert_eq!(
        said.trim(),
        "Untitled document - Google Docs\n\u{1d} group \u{1e}"
    );
}

/// A page's menu opened from a text field (the web menus fixture's Alt+Shift+F, dev box
/// 2026-10-16): its keys are the menu's. Down is not read as the field's next line (Okur
/// said the item it left), and Escape closes the menu instead of leaving forms mode.
#[test]
fn a_page_menu_opened_from_a_field_takes_its_keys() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus"),
            AomNode::new(2, Role::Edit).with_name("Notes"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut notes = FocusEvent::new(Role::Edit)
        .with_node_id(2)
        .with_name("Notes");
    notes.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(notes));
    assert!(engine.forms_mode);

    engine.handle_accessibility_event(&AccessibilityEvent::Menu(
        crate::event::MenuEvent::PopupStart,
    ));
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(
        FocusEvent::new(Role::MenuItem).with_name("New"),
    ));
    for key in [Key::DownArrow, Key::Escape] {
        let event = KeyEvent::new(key, Modifiers::none(), true);
        assert_eq!(engine.handle_key_event(&event), KeyAction::PassThrough);
    }
    assert!(actions.requests().is_empty(), "{:?}", actions.requests());
    assert!(engine.forms_mode);
}

/// A plain multi-line edit (Windows 10 Notepad's, a WinForms text box) says nothing for Home,
/// as JAWS there [obs, 504/s5, 664/s9, 406/s8], and "top of file" for Ctrl+Home as a
/// document does [obs, 660/s4].
#[test]
fn a_plain_multi_line_edit_is_silent_on_home() {
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(create_test_buffer()),
        Box::new(speech.clone()),
    );
    engine.set_actions(Box::new(crate::actions::RecordingActions::default()));
    let context = |caret: usize| crate::event::TextContext {
        text: "fn main\r\n    let x".to_owned(),
        caret,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    let mut field = FocusEvent::new(Role::Edit).with_name("Text Editor");
    field.multi_line = true;
    field.text = Some(context(7));
    engine.handle_focus_event(&field);
    speech.take();
    engine.handle_key_event(&key(Key::Home));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(0)));
    assert_eq!(norm(&speech.take().join(" ")), "");
    let mut ctrl = Modifiers::none();
    ctrl.ctrl = true;
    engine.handle_key_event(&KeyEvent::new(Key::End, ctrl, true));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(18)));
    engine.handle_key_event(&KeyEvent::new(Key::Home, ctrl, true));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(0)));
    assert!(
        norm(&speech.take().join(" ")).contains("top of file"),
        "Ctrl+Home names the top"
    );
    // Windows 10 Notepad's edit, a Document to UI Automation, is a plain edit by its class.
    let mut notepad = FocusEvent::new(Role::Document).with_name("Text Editor");
    notepad.platform.element_class = Some("Edit".to_owned());
    notepad.multi_line = true;
    notepad.text = Some(context(7));
    engine.handle_focus_event(&notepad);
    speech.take();
    engine.handle_key_event(&key(Key::Home));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(0)));
    assert_eq!(norm(&speech.take().join(" ")), "");
}

/// A page's multi-line field reads keys as a document does: Google Docs' document with
/// braille support, JAWS on the dev box (2026-10-16). Home says the character it lands on,
/// Ctrl+Home "top of file" and the line, Say Line the line alone, and Say Font the font and
/// then the paragraph's alignment.
#[test]
fn a_pages_multi_line_field_reads_keys_as_a_document() {
    let mut field = AomNode::new(2, Role::Edit).with_name("Document content");
    field
        .attributes
        .insert("multiline".to_owned(), "true".to_owned());
    let page = AomNode::new(0, Role::Document)
        .with_name("Untitled document - Google Docs")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Untitled document"),
            field,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.set_actions(Box::new(crate::actions::RecordingActions::default()));
    let context = |caret: usize| crate::event::TextContext {
        text: "First line\nSecond line with a bold word".to_owned(),
        caret,
        selection: None,
        font: Some(crate::event::FontInfo {
            family: "Arial".to_owned(),
            size_tenths: 110,
            bold: true,
            alignment: Some("left".to_owned()),
            ..Default::default()
        }),
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    let mut focus = FocusEvent::new(Role::Edit)
        .with_node_id(2)
        .with_name("Document content");
    focus.in_document = true;
    focus.text = Some(context(15));
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    assert!(engine.forms_mode);
    speech.take();

    assert_eq!(
        engine.execute_command("Insert+F").as_deref(),
        Some("Font is bold  Arial 11 point,\nParagraph formatting left justified")
    );
    let jaws = |key: Key, ctrl: bool| {
        KeyEvent::new(
            key,
            Modifiers {
                ctrl,
                insert: key == Key::UpArrow,
                ..Modifiers::none()
            },
            true,
        )
    };
    speech.take();
    engine.handle_key_event(&jaws(Key::UpArrow, false));
    assert_eq!(
        speech.take(),
        vec!["Second line with a bold word".to_owned()]
    );

    engine.handle_key_event(&jaws(Key::Home, false));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(11)));
    engine.handle_key_event(&jaws(Key::Home, true));
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(0)));
    assert_eq!(
        speech.take(),
        vec!["S".to_owned(), "top of file\nFirst line".to_owned()]
    );

    // Say All: the lines from the caret's to the last, not the page around the field.
    let say_all = KeyEvent::new(
        Key::DownArrow,
        Modifiers {
            insert: true,
            ..Modifiers::none()
        },
        true,
    );
    engine.handle_key_event(&say_all);
    assert_eq!(
        speech.take(),
        vec![
            "First line".to_owned(),
            "Second line with a bold word".to_owned()
        ]
    );
}

/// A page captured again finds its focus by role and name when the focus has no DOM id and
/// another node keeps a focused state (Google Docs' menu bar, before its document's field).
#[test]
fn a_refreshed_page_finds_a_focus_without_an_id() {
    let focused = crate::aom::NodeStates {
        focused: true,
        ..Default::default()
    };
    let page = |first: usize| {
        let mut menu_bar = AomNode::new(first, Role::Group).with_states(focused.clone());
        menu_bar
            .attributes
            .insert("id".to_owned(), "docs-menubar".to_owned());
        AomNode::new(0, Role::Document)
            .with_name("Untitled document - Google Docs")
            .with_children(vec![
                menu_bar,
                AomNode::new(first + 1, Role::Edit)
                    .with_name("Document content")
                    .with_states(focused.clone()),
            ])
    };
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page(1)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut focus = FocusEvent::new(Role::Edit)
        .with_node_id(2)
        .with_name("Document content");
    focus.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "Untitled document - Google Docs".to_owned(),
        root: page(10),
    });
    engine.on_idle();
    assert_eq!(engine.last_page_focus, Some(11));
}

/// Google Sheets (and Docs without braille support) types into an empty element in an
/// application region: Say Line there is "blank", as JAWS says (dev box, 2026-10-16), not
/// the line the virtual cursor is on (the page title).
#[test]
fn say_line_on_an_empty_page_focus_is_blank() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Untitled spreadsheet - Google Sheets")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Untitled spreadsheet"),
            AomNode::new(2, Role::Group),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut focus = FocusEvent::new(Role::Group).with_node_id(2);
    focus.in_document = true;
    focus.web_application = Some(String::new());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    assert_eq!(engine.execute_command("SayLine").as_deref(), Some("blank"));
}

/// Google Sheets' cell editor (JAWS on the dev box, 2026-10-16): a combo box one types in,
/// named by its cell, which the page captured without that name. Typing into a cell says
/// "D1 edit combo | n | To set the value use the Arrow keys or type the value.", without a
/// state (it opens nothing); Enter back in the grid says "group".
#[test]
fn a_sheets_cell_editor_is_said_with_its_cell() {
    let mut editor = AomNode::new(3, Role::ComboBox);
    editor
        .attributes
        .insert("editable".to_owned(), "true".to_owned());
    let page = AomNode::new(0, Role::Document)
        .with_name("Untitled spreadsheet - Google Sheets")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Untitled spreadsheet"),
            AomNode::new(2, Role::Group),
            editor,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let in_grid = |id: usize, role: Role| {
        let mut focus = FocusEvent::new(role).with_node_id(id);
        focus.in_document = true;
        focus.web_application = Some(String::new());
        focus
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(in_grid(2, Role::Group)));
    let mut typing = in_grid(3, Role::ComboBox).with_name("D1");
    typing.value = Some("\u{feff}n\n".to_owned());
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::Focus(typing))
            .as_deref(),
        Some(
            "D1  \u{1d} edit combo \u{1e} \nn\n\
             To set the value use the Arrow keys or type the value."
        )
    );
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::Focus(in_grid(2, Role::Group)))
            .map(|s| s.trim().to_owned())
            .as_deref(),
        Some("\u{1d} group \u{1e}")
    );
}

/// Say Focus (Insert+Tab) on a page: the focus as it was announced, without moving the
/// virtual cursor, and in a menu its item (JAWS in Google Docs, dev box 2026-10-16:
/// "Document content edit | Contains text"; "menu | Accessibility | 10 of 10").
#[test]
fn say_focus_reads_a_pages_focus_and_a_menus_item() {
    let mut field = AomNode::new(2, Role::Edit)
        .with_name("Document content")
        .with_value("Some text");
    field
        .attributes
        .insert("multiline".to_owned(), "true".to_owned());
    let page = AomNode::new(0, Role::Document)
        .with_name("Untitled document - Google Docs")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Untitled document"),
            field,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut focus = FocusEvent::new(Role::Edit)
        .with_node_id(2)
        .with_name("Document content");
    focus.in_document = true;
    let on_focus = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(focus))
        .unwrap_or_default();
    let offset = engine.cursor.offset;
    let said = engine
        .execute_command("SayCurrentFocus")
        .unwrap_or_default();
    assert!(on_focus.ends_with(&said), "{on_focus:?} / {said:?}");
    assert!(said.starts_with("Document content"), "{said:?}");
    assert_eq!(engine.cursor.offset, offset);

    engine.handle_accessibility_event(&AccessibilityEvent::Focus(
        FocusEvent::new(Role::MenuItem).with_name("Accessibility"),
    ));
    assert_eq!(
        engine.execute_command("SayCurrentFocus").as_deref(),
        Some("Accessibility")
    );
}

/// Typing echo follows the setting: the character with JAWS's default, nothing with
/// `TypingEcho=0` (the dev box's JAWS made no sound for letters typed in the Run box, and spoke
/// for Backspace; 2026-10-17).
#[test]
fn typing_echo_follows_the_setting() {
    use crate::settings::TypingEcho;
    for (echo, expected) in [
        (TypingEcho::Characters, vec!["a".to_owned()]),
        (TypingEcho::None, Vec::new()),
    ] {
        let (mut engine, speech) = engine_with_speech();
        let mut changed = engine.settings.clone();
        changed.typing_echo = echo;
        engine.set_settings(changed);
        engine.handle_focus_event(
            &FocusEvent::new(Role::Edit)
                .with_name("Open:")
                .with_window_title("Run"),
        );
        speech.take();
        let mut a = KeyEvent::new(Key::Alpha('A'), Modifiers::none(), true);
        a.text = Some('a');
        engine.handle_key_event(&a);
        assert_eq!(speech.take(), expected, "{echo:?}");
    }
}

/// Windows search as the results change while typing (JAWS on the dev box, 2026-10-17): the
/// same result selected again says its name alone; another says its position.
#[test]
fn a_result_selected_again_says_its_name_alone() {
    let (mut engine, _speech) = engine_with_speech();
    engine.handle_focus_event(&FocusEvent::new(Role::Edit).with_name("Search box"));
    let result = |name: &str, index: usize, count: usize| {
        AccessibilityEvent::Selected(
            FocusEvent::new(Role::ListItem)
                .with_name(name)
                .with_item_index(index)
                .with_item_count(count),
        )
    };
    let settings = "Settings, System, Press right to view options";
    assert_eq!(
        engine
            .handle_accessibility_event(&result(settings, 1, 4))
            .as_deref(),
        Some("Settings, System, Press right to view options\n1 of 4")
    );
    assert_eq!(
        engine
            .handle_accessibility_event(&result(settings, 1, 6))
            .as_deref(),
        Some(settings)
    );
    let web = "Find results in search the web, click enter to see more";
    assert_eq!(
        engine
            .handle_accessibility_event(&result(web, 2, 2))
            .as_deref(),
        Some("Find results in search the web, click enter to see more\n2 of 2")
    );
}

/// Down in the Font dialog's font field: its list selects the next font and puts it in the
/// field, and the dialog's style list follows, selecting its own item. The field's line is
/// said with its own item's position [obs, 541/s3].
#[test]
fn an_edit_combos_line_is_said_with_its_items_position() {
    let (mut engine, _speech) = engine_with_speech();
    let text = |t: &str| crate::event::TextContext {
        text: t.to_owned(),
        caret: 0,
        selection: Some((0, t.chars().count())),
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    let mut field = FocusEvent::new(Role::Edit)
        .with_name("Font:")
        .with_item_index(38)
        .with_item_count(187);
    field.container_role = Some(Role::ComboBox);
    field.value = Some("Consolas".to_owned());
    field.text = Some(text("Consolas"));
    engine.handle_focus_event(&field);
    engine.handle_key_event(&key(Key::DownArrow));
    let item = |name: &str, index: usize| {
        AccessibilityEvent::Selected(
            FocusEvent::new(Role::ListItem)
                .with_name(name)
                .with_item_index(index)
                .with_item_count(187),
        )
    };
    assert_eq!(
        engine.handle_accessibility_event(&item("Constantia", 39)),
        None
    );
    assert_eq!(engine.handle_accessibility_event(&item("Regular", 1)), None);
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::TextContext(text("Constantia")))
            .as_deref(),
        Some("Constantia\n39 of 187")
    );
}

/// Windows Terminal notifies the console's echo of a typed "ı" (Turkish layout, dev box,
/// 2026-10-17): typing echo says typed characters, so that notification says nothing; the
/// output after it is said.
#[test]
fn a_notification_echoing_the_typed_character_says_nothing() {
    let (mut engine, _speech) = engine_with_speech();
    let mut key = KeyEvent::new(Key::Alpha('I'), Modifiers::none(), true);
    key.text = Some('ı');
    engine.handle_key_event(&key);
    let notification = |text: &str| AccessibilityEvent::AppNotification {
        text: text.to_owned(),
        politeness: Politeness::Polite,
        app: Some("WindowsTerminal".to_owned()),
    };
    assert_eq!(engine.handle_accessibility_event(&notification("ı")), None);
    assert_eq!(
        engine
            .handle_accessibility_event(&notification("hı"))
            .as_deref(),
        Some("hı")
    );
}

/// Chrome's address bar and Escape back to the page (JAWS on the dev box, 2026-10-17): the
/// field says its shortcut after its hint; the page, focused again, its title and "document".
#[test]
fn the_address_bar_says_its_shortcut_and_the_page_its_name_on_return() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![AomNode::new(1, Role::Paragraph).with_text("Notes")]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let mut document = FocusEvent::new(Role::Document)
        .with_name("Web Menus")
        .with_window_title("Web Menus - Google Chrome");
    document.in_document = true;
    document.node_id = Some(0);
    engine.handle_focus_event(&document);
    let mut address = FocusEvent::new(Role::Edit)
        .with_name("Address and search bar")
        .with_window_title("Web Menus - Google Chrome");
    address.accelerator_key = Some("Ctrl+L".to_owned());
    let said = engine.handle_focus_event(&address).unwrap_or_default();
    assert!(said.ends_with("Type in text.\nCtrl+L"), "{said}");
    assert_eq!(
        engine.handle_focus_event(&document).as_deref(),
        Some("Web Menus \u{1d} document \u{1e} ")
    );
}

/// Ctrl+Tab from a new tab's address bar back to a page (JAWS on the dev box, 2026-10-17):
/// "Web Menus document | Web Menus - Google Chrome".
#[test]
fn another_tabs_page_says_its_name_and_the_window() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![AomNode::new(1, Role::Paragraph).with_text("Notes")]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let address = FocusEvent::new(Role::Edit)
        .with_name("Address and search bar")
        .with_window_title("New Tab - Google Chrome");
    engine.handle_focus_event(&address);
    let mut document = FocusEvent::new(Role::Document)
        .with_name("Web Menus")
        .with_window_title("Web Menus - Google Chrome");
    document.in_document = true;
    document.node_id = Some(0);
    assert_eq!(
        engine.handle_focus_event(&document).as_deref(),
        Some("Web Menus \u{1d} document \u{1e} \nWeb Menus - Google Chrome")
    );
}

/// A page in the virtual cursor, with the focus on its document.
fn engine_on_a_page() -> (ScreenReaderEngine, SharedSpeech) {
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus"),
            AomNode::new(2, Role::Paragraph).with_text("Notes"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    (engine, speech)
}

/// With Okur's windows, Insert+F7 opens a real Links List; Move To Link closes it, moves
/// the virtual cursor to the link and says it with JAWS's hint; the focus coming back to the
/// page is taken in silently (JAWS on the dev box, 2026-10-17).
#[test]
fn the_links_list_moves_the_cursor_to_the_chosen_link() {
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus"),
            AomNode::new(
                2,
                Role::Link {
                    href: Some("https://example.com/a".into()),
                },
            )
            .with_name("Alpha"),
            AomNode::new(3, Role::Paragraph).with_text("Notes"),
            AomNode::new(
                4,
                Role::Link {
                    href: Some("https://example.com/b".into()),
                },
            )
            .with_name("Beta"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    speech.take();
    let mut f7 = KeyEvent::new(Key::F(7), Modifiers::none(), true);
    f7.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&f7), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Links List");
    assert_eq!(
        dialog.controls.first().map(|c| c.kind.clone()),
        Some(ControlKind::List {
            items: vec!["Alpha".into(), "Beta".into()],
            selected: 0
        })
    );
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "links".into(),
        value: Value::Selected(1),
    }));
    assert_eq!(
        ui.take(),
        [ToUi::DialogSet {
            id,
            control: "address".into(),
            value: Value::Text("https://example.com/b".into()),
        }]
    );
    let moved = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "move".into(),
    }));
    assert!(
        moved
            .as_deref()
            .is_some_and(|s| s.contains("Beta") && s.ends_with("To activate press Enter.")),
        "{moved:?}"
    );
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
    speech.take();
    // The page's focus comes back: nothing is said, and the cursor stays on the link.
    assert_eq!(
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(page_document())),
        None
    );
    assert!(speech.take().is_empty());
    assert!(engine
        .execute_command("SayLine")
        .is_some_and(|line| line.contains("Beta")));
}

/// In Word's document after a page was read, Insert+F7 is Word's Links List, never the last
/// page's links, which the buffer still held: Word's words for it without links [obs,
/// 1044/s1], as Okur does not read Word's links yet.
#[test]
fn words_links_list_is_not_the_last_pages() {
    use crate::ui::ToUi;
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![AomNode::new(
            1,
            Role::Link {
                href: Some("https://example.com/a".into()),
            },
        )
        .with_name("Alpha")]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut word = FocusEvent::new(Role::Document).with_name("sr-word-fixture.docx");
    word.app = Some("WINWORD.EXE".into());
    word.window_title = Some("sr-word-fixture.docx - Word".into());
    engine.handle_focus_event(&word);
    speech.take();
    ui.take();
    let mut f7 = KeyEvent::new(Key::F(7), Modifiers::none(), true);
    f7.modifiers.insert = true;
    engine.handle_key_event(&f7);
    assert!(
        !ui.take()
            .iter()
            .any(|m| matches!(m, ToUi::DialogShow { .. })),
        "the page's Links List was shown in Word"
    );
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("no links\nCheck for link type fields with Insert+F5.")
    );
}

/// Insert+F6 opens a real Heading List; Move To Heading reads the heading's line as the
/// virtual cursor does, its level first: "heading level 1 Web App Patterns" [obs, 224/s10].
#[test]
fn the_heading_list_reads_the_chosen_heading_as_its_line() {
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus"),
            AomNode::new(2, Role::Paragraph).with_text("Notes"),
            AomNode::new(3, Role::Heading { level: 2 }).with_text("Second part"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut f6 = KeyEvent::new(Key::F(6), Modifiers::none(), true);
    f6.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&f6), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Heading List");
    assert_eq!(
        dialog.controls.first().map(|c| c.kind.clone()),
        Some(ControlKind::List {
            items: vec!["Web Menus : 1".into(), "Second part : 2".into()],
            selected: 0
        })
    );
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "headings".into(),
        value: Value::Selected(1),
    }));
    let moved = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "move".into(),
    }));
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
    let line = engine.execute_command("SayLine");
    assert_eq!(moved, line);
    let level_first = moved
        .as_deref()
        .and_then(|s| Some(s.find("heading level 2")? < s.find("Second part")?));
    assert_eq!(level_first, Some(true), "{moved:?}");
}

/// Insert+Ctrl+B opens JAWS's Select a Button; OK gives the button the focus and leaves the
/// speech to the focus [obs, 232, 247]. A list with nothing in it is said so, in JAWS's words,
/// with no dialog [obs, 271].
#[test]
fn an_element_list_focuses_its_control_or_says_there_is_none() {
    use crate::ui::{ControlKind, FromUi, ToUi};
    let page = AomNode::new(0, Role::Document)
        .with_name("Sign up")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Fill in the form."),
            AomNode::new(2, Role::Button).with_name("Create account"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let chord = |letter: char| {
        let mut key = KeyEvent::new(Key::Alpha(letter), Modifiers::none(), true);
        key.modifiers.insert = true;
        key.modifiers.ctrl = true;
        key
    };
    assert_eq!(engine.handle_key_event(&chord('b')), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Select a Button");
    assert_eq!(
        dialog.controls.first().map(|c| c.kind.clone()),
        Some(ControlKind::List {
            items: vec!["Create account Button".into()],
            selected: 0
        })
    );
    let said = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "ok".into(),
    }));
    assert_eq!(said, None);
    assert_eq!(ui.take(), [ToUi::DialogClose { id: dialog.id }]);
    assert_eq!(actions.requests(), [("focus".to_owned(), 2)]);
    // No tables: JAWS's sentence, and no dialog.
    engine.handle_key_event(&chord('t'));
    assert!(ui.take().is_empty());
    assert!(speech.take().iter().any(|s| s == "No tables found"));
}

/// Select an Edit Box (Insert+Ctrl+E) leaves the speech to the edit's focus, which enters the
/// page and forms mode: the title, the field, then its line as Say Line reads it in forms
/// mode, and no "Type in text." [obs, 249/s3].
#[test]
fn an_edit_from_its_list_is_said_with_its_line_in_forms_mode() {
    use crate::ui::{FromUi, ToUi};
    let page = AomNode::new(0, Role::Document)
        .with_name("Sign up")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Fill in the form."),
            AomNode::new(2, Role::Edit)
                .with_name("Email address")
                .with_value("sam@example.com"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut chord = KeyEvent::new(Key::Alpha('e'), Modifiers::none(), true);
    chord.modifiers.insert = true;
    chord.modifiers.ctrl = true;
    assert_eq!(engine.handle_key_event(&chord), KeyAction::Consume);
    let id = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) => dialog.id,
        other => {
            panic_free_fail(&format!("no dialog shown: {other:?}"));
            return;
        }
    };
    let said = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".into(),
    }));
    assert_eq!(said, None);
    assert_eq!(actions.requests(), [("focus".to_owned(), 2)]);
    let mut email = FocusEvent::new(Role::Edit)
        .with_name("Email address")
        .with_value("sam@example.com")
        .with_node_id(2)
        .with_window_title("Sign up - Google Chrome");
    email.in_document = true;
    let focused = engine.handle_accessibility_event(&AccessibilityEvent::Focus(email));
    assert!(
        focused
            .as_deref()
            .is_some_and(|s| s.starts_with("Sign up\n")
                && s.ends_with("\nEmail address  \u{1d} Edit \u{1e} \nsam@example.com")
                && !s.contains("Type in text.")),
        "{focused:?}"
    );
}

/// Insert+F3 opens Virtual HTML Features; OK on Headings List opens the Heading List before
/// the first dialog goes, as JAWS opens it in its place [obs, 226/s3]. A list Okur has not
/// built yet is said so, and the dialog goes.
#[test]
fn virtual_html_features_opens_the_chosen_list() {
    use crate::ui::{FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus")
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut f3 = KeyEvent::new(Key::F(3), Modifiers::none(), true);
    f3.modifiers.insert = true;
    let open = |engine: &mut ScreenReaderEngine, item: usize| {
        assert_eq!(engine.handle_key_event(&f3), KeyAction::Consume);
        let id = match ui.take().first() {
            Some(ToUi::DialogShow { dialog }) if dialog.title == "Virtual HTML Features" => {
                dialog.id
            }
            other => {
                panic_free_fail(&format!("no Virtual HTML Features: {other:?}"));
                return (0, None);
            }
        };
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
            id,
            control: "lists".into(),
            value: Value::Selected(item),
        }));
        let said =
            engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
                id,
                control: "ok".into(),
            }));
        (id, said)
    };
    // Headings List, the 12th.
    let (id, _) = open(&mut engine, 11);
    let sent = ui.take();
    let titles: Vec<String> = sent
        .iter()
        .map(|m| match m {
            ToUi::DialogShow { dialog } => format!("show {}", dialog.title),
            ToUi::DialogClose { id: closed } if *closed == id => "close features".to_owned(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(titles, ["show Heading List", "close features"]);
    // Anchors List, which Okur has not built.
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id: id + 1 }));
    let (id, said) = open(&mut engine, 0);
    assert_eq!(said.as_deref(), Some("Okur has no Anchors List yet."));
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
}

/// Insert+F5 opens a real Select a Form Field. OK moves to the chosen field and focuses it,
/// naming the page and reading the cursor's line and an edit's line in forms mode; the
/// field's own focus is then said although the focus is coming back from the dialog, within
/// the page [obs, 225/s10, 228/s9]. Escape names the page [obs, 227/s11].
#[test]
fn select_a_form_field_focuses_the_chosen_field() {
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus"),
            AomNode::new(2, Role::Button).with_name("Start"),
            AomNode::new(3, Role::CheckBox)
                .with_name("Wrap")
                .with_states(crate::aom::NodeStates {
                    checked: Some(false),
                    ..crate::aom::NodeStates::default()
                }),
            AomNode::new(4, Role::Edit)
                .with_name("Notes")
                .with_value("the lazy dog"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut f5 = KeyEvent::new(Key::F(5), Modifiers::none(), true);
    f5.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&f5), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Select a Form Field");
    assert_eq!(
        dialog
            .controls
            .first()
            .map(|c| (c.label.clone(), c.kind.clone())),
        Some((
            "List1".into(),
            ControlKind::List {
                items: vec![
                    "Start Button".into(),
                    "Wrap check box not checked".into(),
                    "Notes Edit the lazy dog".into()
                ],
                selected: 0
            }
        ))
    );
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "fields".into(),
        value: Value::Selected(2),
    }));
    let chosen =
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
            id,
            control: "ok".into(),
        }));
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
    assert_eq!(actions.requests(), [("focus".to_owned(), 4)]);
    // The page, the cursor's line, then the field's line in forms mode [obs, 228/s9].
    assert!(
        chosen
            .as_deref()
            .is_some_and(|s| s.starts_with("Web Menus\n")
                && s.ends_with("the lazy dog\nNotes  \u{1d} Edit \u{1e} \nthe lazy dog")),
        "{chosen:?}"
    );
    // The page takes the focus back quietly; the field's focus is said, within the page:
    // without the title, and without Enter's "Type in text." [obs, 228/s9].
    assert_eq!(
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(page_document())),
        None
    );
    let mut notes = FocusEvent::new(Role::Edit)
        .with_name("Notes")
        .with_value("the lazy dog")
        .with_node_id(4)
        .with_window_title("Web Menus - Google Chrome");
    notes.in_document = true;
    let focused = engine.handle_accessibility_event(&AccessibilityEvent::Focus(notes));
    assert!(
        focused.as_deref().is_some_and(|s| s.starts_with("Notes")
            && !s.contains("Web Menus")
            && !s.contains("Type in text.")),
        "{focused:?}"
    );

    // Escape: the dialog goes and JAWS reads the field the virtual cursor is on [obs,
    // 688/s19], or, at the page's top, names the page [obs, 227/s11].
    let escape = |engine: &mut ScreenReaderEngine| {
        engine.handle_key_event(&f5);
        let id = match ui.take().first() {
            Some(ToUi::DialogShow { dialog }) => dialog.id,
            other => {
                panic_free_fail(&format!("no dialog shown: {other:?}"));
                return None;
            }
        };
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id }))
    };
    let on_field = escape(&mut engine);
    assert!(
        on_field
            .as_deref()
            .is_some_and(|s| s.starts_with("Notes") && s.ends_with("the lazy dog")),
        "{on_field:?}"
    );
    engine.cursor.move_to_start();
    assert_eq!(
        escape(&mut engine).as_deref(),
        Some("Web Menus \u{1d} document \u{1e} ")
    );
}

/// Space with the virtual cursor is JAWS's VirtualSpacebar: the control under the cursor takes
/// the focus and gets a click, never the Space key [obs, 229 event log]. A field one types in
/// takes the focus in forms mode; a combo box's list opens; on text Space does nothing.
#[test]
fn space_focuses_and_clicks_the_control_under_the_virtual_cursor() {
    let page = AomNode::new(0, Role::Document)
        .with_name("Activation")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Plain text"),
            AomNode::new(2, Role::Button).with_name("Count"),
            AomNode::new(3, Role::CheckBox).with_name("Subscribe"),
            AomNode::new(
                4,
                Role::Link {
                    href: Some("#status".into()),
                },
            )
            .with_name("Show status"),
            AomNode::new(5, Role::Edit)
                .with_name("City")
                .with_value("Izmir"),
            AomNode::new(6, Role::ComboBox)
                .with_name("Color")
                .with_value("Red"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut space_on = |id: usize| {
        let line = engine.cursor.buffer.lines.iter().find(|l| l.node_id == id);
        assert!(line.is_some(), "no line for node {id}");
        engine.cursor.offset = line.map(|l| l.start_offset).unwrap_or_default();
        engine.forms_mode = false;
        engine.cursor.forms_mode_active = false;
        let before = actions.requests().len();
        let action = engine.handle_key_event(&KeyEvent::new(Key::Space, Modifiers::none(), true));
        let requests: Vec<(String, usize)> = actions.requests().into_iter().skip(before).collect();
        (action, requests, engine.forms_mode)
    };
    let clicked = |id: usize| vec![("focus".to_owned(), id), ("activate".to_owned(), id)];
    assert_eq!(space_on(2), (KeyAction::Consume, clicked(2), false));
    assert_eq!(space_on(3), (KeyAction::Consume, clicked(3), false));
    assert_eq!(space_on(4), (KeyAction::Consume, clicked(4), false));
    assert_eq!(
        space_on(5),
        (KeyAction::Consume, vec![("focus".to_owned(), 5)], true)
    );
    assert_eq!(space_on(6), (KeyAction::Consume, clicked(6), true));
    // Text: nothing happens, and the page does not get the key [obs, 229/s2].
    assert_eq!(space_on(1), (KeyAction::Consume, Vec::new(), false));
}

fn page_document() -> FocusEvent {
    let mut document = FocusEvent::new(Role::Document)
        .with_name("Web Menus")
        .with_window_title("Web Menus - Google Chrome");
    document.in_document = true;
    document.node_id = Some(0);
    document
}

fn key(key: Key) -> KeyEvent {
    KeyEvent::new(key, Modifiers::none(), true)
}

/// Whitespace-insensitive speech, as parity compares it.
fn norm(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Auto Forms Mode with the arrows: Down onto a field focuses it, turns forms mode on and
/// says it as its focus, without "blank" or its description; the focus event that follows is
/// not said; Down that cannot move the field's caret leaves forms mode for the next line, and
/// Down that can is the field's [obs, 280/s4-s9].
#[test]
fn auto_forms_mode_arrows_into_and_out_of_fields() {
    let mut name = AomNode::new(2, Role::Edit).with_name("Full name");
    name.attributes.insert("required".into(), "true".into());
    let mut email = AomNode::new(4, Role::Edit).with_name("Email address");
    email.value = Some("sam@example.com".into());
    email
        .attributes
        .insert("description".into(), "We never share your email.".into());
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Full name"),
            name,
            AomNode::new(3, Role::Paragraph).with_text("Email address"),
            email,
            AomNode::new(5, Role::Paragraph).with_text("We never share your email."),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    engine.handle_focus_event(&page_document());
    engine.cursor.move_to_start();
    speech.take();
    let field = |id: usize, name: &str, text: &str| {
        let mut focus = FocusEvent::new(Role::Edit).with_node_id(id).with_name(name);
        focus.in_document = true;
        focus.text = Some(crate::event::TextContext {
            text: text.into(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        });
        focus
    };
    let down = |engine: &mut ScreenReaderEngine| {
        let action = engine.handle_key_event(&key(Key::DownArrow));
        (action, norm(&speech.take().join(" ")))
    };

    assert_eq!(down(&mut engine).1, "Full name");
    let (_, said) = down(&mut engine);
    assert_eq!(said, "Full name \u{1d} Edit \u{1e} \u{1d}Required\u{1e}");
    assert!(engine.forms_mode);
    assert!(actions.requests().contains(&("focus".to_owned(), 2)));
    assert_eq!(engine.handle_focus_event(&field(2, "Full name", "")), None);
    assert!(engine.forms_mode);

    // The empty field's caret cannot move: Down leaves it.
    assert_eq!(
        down(&mut engine),
        (KeyAction::Consume, "Email address".to_owned())
    );
    assert!(!engine.forms_mode);
    let (_, said) = down(&mut engine);
    assert_eq!(said, "Email address \u{1d} Edit \u{1e} sam@example.com");
    assert_eq!(
        engine.handle_focus_event(&field(4, "Email address", "sam@example.com")),
        None
    );

    // Its caret at the start: Down is the field's, then leaves it.
    assert_eq!(down(&mut engine).0, KeyAction::PassThrough);
    assert!(engine.forms_mode);
    let mut moved = field(4, "Email address", "sam@example.com");
    if let Some(text) = moved.text.as_mut() {
        text.caret = 15;
    }
    if let Some(context) = moved.text {
        engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context));
    }
    assert_eq!(norm(&speech.take().join(" ")), "sam@example.com");
    assert_eq!(
        down(&mut engine),
        (KeyAction::Consume, "We never share your email.".to_owned())
    );
}

/// Text typed in a field is said when an arrow comes back onto it, though the captured page
/// does not have it; Up enters with the caret at the start, and leaves at once
/// [obs, 284/s6, s7].
#[test]
fn an_arrow_back_onto_a_field_says_what_was_typed() {
    let mut name = AomNode::new(2, Role::Edit).with_name("Full name");
    name.attributes.insert("required".into(), "true".into());
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(4, Role::Paragraph).with_text("Fields marked required"),
            AomNode::new(1, Role::Paragraph).with_text("Full name"),
            name,
            AomNode::new(3, Role::Paragraph).with_text("Email address"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.set_actions(Box::new(crate::actions::RecordingActions::default()));
    engine.handle_focus_event(&page_document());
    let mut field = FocusEvent::new(Role::Edit)
        .with_node_id(2)
        .with_name("Full name");
    field.in_document = true;
    engine.handle_focus_event(&field);
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
        crate::event::TextContext {
            text: "ada".into(),
            caret: 3,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        },
    ));
    speech.take();
    // Down at the end of the text leaves the field; Up comes back onto it.
    engine.handle_key_event(&key(Key::DownArrow));
    assert_eq!(norm(&speech.take().join(" ")), "Email address");
    engine.handle_key_event(&key(Key::UpArrow));
    assert_eq!(
        norm(&speech.take().join(" ")),
        "Full name \u{1d} Edit \u{1e} \u{1d}Required\u{1e} ada"
    );
    engine.handle_key_event(&key(Key::UpArrow));
    assert_eq!(norm(&speech.take().join(" ")), "Full name");
    // Down straight back onto the field Up has just left says nothing: forms mode is on again,
    // by JAWS's sound alone [obs, 626/s5, 567/s12].
    engine.handle_key_event(&key(Key::DownArrow));
    assert_eq!(norm(&speech.take().join(" ")), "");
    assert!(engine.forms_mode);
    // Up leaves it again, and Down back onto it two lines later says nothing either
    // [obs, 632/s7].
    engine.handle_key_event(&key(Key::UpArrow));
    assert_eq!(norm(&speech.take().join(" ")), "Full name");
    engine.handle_key_event(&key(Key::UpArrow));
    assert_eq!(norm(&speech.take().join(" ")), "Fields marked required");
    engine.handle_key_event(&key(Key::DownArrow));
    assert_eq!(norm(&speech.take().join(" ")), "Full name");
    engine.handle_key_event(&key(Key::DownArrow));
    assert_eq!(norm(&speech.take().join(" ")), "");
    assert!(engine.forms_mode);
}

/// Without Auto Forms Mode, Tab onto a field turns forms mode on once the virtual cursor has
/// moved since the page loaded, with the field's hint; Tab first after the load leaves it off,
/// and an empty field is said without "blank" [obs, 629/s1, s2; 630/s2, s3].
#[test]
fn tab_turns_forms_mode_on_once_the_virtual_cursor_has_moved() {
    let mut name = AomNode::new(2, Role::Edit).with_name("Full name");
    name.attributes.insert("required".into(), "true".into());
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Create your account"),
            name,
        ]);
    let start = |moved: bool| {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
        let mut changed = engine.settings.clone();
        changed.forms_mode = crate::settings::FormsMode::Manual;
        engine.set_settings(changed);
        engine.handle_document_loaded("Web Menus", VirtualBuffer::from_aom(&page));
        engine.on_idle();
        engine.handle_focus_event(&page_document());
        if moved {
            engine.handle_key_event(&key(Key::DownArrow));
        }
        speech.take();
        engine.handle_key_event(&key(Key::Tab));
        let mut field = FocusEvent::new(Role::Edit)
            .with_node_id(2)
            .with_name("Full name");
        field.in_document = true;
        engine.handle_focus_event(&field);
        (engine, speech)
    };
    let (first, speech) = start(false);
    assert!(!first.forms_mode);
    let said = norm(&speech.take().join(" "));
    assert!(!said.contains("blank"), "{said}");
    assert!(!said.contains("Type in text."), "{said}");
    let (moved, speech) = start(true);
    assert!(moved.forms_mode);
    let said = norm(&speech.take().join(" "));
    assert!(said.ends_with("Type in text."), "{said}");
}

/// Without Auto Forms Mode, the arrows read a field's line and stay in browse mode [obs, 201].
#[test]
fn without_auto_forms_mode_the_arrows_read_a_fields_line() {
    let mut name = AomNode::new(2, Role::Edit).with_name("Full name");
    name.attributes.insert("required".into(), "true".into());
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Full name"),
            name,
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut changed = engine.settings.clone();
    changed.forms_mode = crate::settings::FormsMode::Manual;
    engine.set_settings(changed);
    engine.handle_focus_event(&page_document());
    engine.cursor.move_to_start();
    engine.handle_key_event(&key(Key::DownArrow));
    engine.handle_key_event(&key(Key::DownArrow));
    assert!(!engine.forms_mode);
}

/// Chrome's app menu over a page (Alt+F; dev box, 2026-10-17): the system says menu mode
/// started and a popup opened, and no item has the focus until the next Down. That Down is the
/// menu's, not the virtual cursor's; after the menu, the virtual cursor has the keys again.
#[test]
fn a_menu_takes_the_keys_before_any_item_is_focused() {
    let (mut engine, _speech) = engine_on_a_page();
    let down = key(Key::DownArrow);
    assert_eq!(engine.handle_key_event(&down), KeyAction::Consume);
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(crate::event::MenuEvent::Start {
        window_class: None,
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(
        crate::event::MenuEvent::PopupStart,
    ));
    assert_eq!(engine.handle_key_event(&down), KeyAction::PassThrough);
    // The menu's first item, outside the page: forms mode stays off.
    let mut item = FocusEvent::new(Role::MenuItem)
        .with_name("Yeni pencere Ctrl+N")
        .with_item_index(2)
        .with_item_count(23)
        .with_window_title("Web Menus - Google Chrome");
    item.accelerator_key = Some("n".to_owned());
    assert_eq!(
        engine.handle_focus_event(&item).as_deref(),
        Some(
            " \u{1d} Context Menu \u{1e} \n\nTo navigate press Up or Down Arrow.\n\
             Yeni pencere Ctrl+N\n2 of 23\nn"
        )
    );
    assert_eq!(engine.handle_key_event(&down), KeyAction::PassThrough);
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::Menu(crate::event::MenuEvent::End))
            .as_deref(),
        Some("Leaving menus")
    );
    engine.handle_focus_event(&page_document());
    assert_eq!(engine.handle_key_event(&down), KeyAction::Consume);
}

/// A menu item reporting no position (Win32) still takes the keys over a page, and a focus
/// landing back in the page ends the menu even with no menu-end event.
#[test]
fn the_page_takes_the_keys_back_when_its_focus_returns_from_a_menu() {
    let (mut engine, _speech) = engine_on_a_page();
    let down = key(Key::DownArrow);
    engine.handle_focus_event(
        &FocusEvent::new(Role::MenuItem)
            .with_name("Open")
            .with_window_title("Web Menus - Google Chrome"),
    );
    assert_eq!(engine.handle_key_event(&down), KeyAction::PassThrough);
    engine.handle_focus_event(&page_document());
    assert_eq!(engine.handle_key_event(&down), KeyAction::Consume);
}

/// Without Auto Forms Mode (old-pc's JAWS, `AutoFormsMode=0`), the focus landing on a field
/// leaves the virtual cursor in charge: Escape says nothing and Up reads the line above, as
/// 212 recorded. Enter turns forms mode on, and Tab to the next field keeps it.
#[test]
fn without_auto_forms_mode_a_field_waits_for_enter() {
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(create_test_buffer()),
        Box::new(MockSpeechOutput::new()),
    );
    let mut changed = engine.settings.clone();
    changed.forms_mode = crate::settings::FormsMode::Manual;
    engine.set_settings(changed);
    let search = || {
        let mut focus = FocusEvent::new(Role::Edit)
            .with_node_id(3)
            .with_name("Search Input");
        focus.in_document = true;
        focus
    };
    engine.handle_focus_event(&search());
    assert!(!engine.forms_mode);
    assert_eq!(
        engine.handle_key_event(&KeyEvent::new(Key::UpArrow, Modifiers::none(), true)),
        KeyAction::Consume
    );

    engine.execute_command("E");
    engine.handle_key_event(&KeyEvent::new(Key::Enter, Modifiers::none(), true));
    assert!(engine.forms_mode);
    engine.handle_focus_event(&search());
    assert!(engine.forms_mode, "a field keeps the forms mode it finds");
}

/// Without Auto Forms Mode a page's menu says its item but leaves the arrows to the virtual
/// cursor (218 on old-pc: Down read the menu bar's next item as a line).
#[test]
fn without_auto_forms_mode_a_page_menu_leaves_the_arrows_to_the_cursor() {
    let (mut engine, _speech) = engine_on_a_page();
    let mut changed = engine.settings.clone();
    changed.forms_mode = crate::settings::FormsMode::Manual;
    engine.set_settings(changed);
    let mut file = FocusEvent::new(Role::MenuItem).with_name("File");
    file.in_document = true;
    file.in_menu_bar = true;
    file.under_menu_bar = true;
    engine.handle_focus_event(&file);
    assert!(!engine.forms_mode);
    assert_eq!(
        engine.handle_key_event(&key(Key::DownArrow)),
        KeyAction::Consume
    );
}

/// A control's access key is said only with JAWS's Access Key output on: old-pc's JAWS has it
/// off (`ACCESS_KEY=0|0|0`), and said "Yes Button | To activate press spacebar." (306).
#[test]
fn access_keys_follow_the_access_key_output() {
    let button = || {
        let mut focus = FocusEvent::new(Role::Button).with_name("Yes");
        focus.access_key = Some("alt+Y".to_owned());
        focus
    };
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(create_test_buffer()),
        Box::new(MockSpeechOutput::new()),
    );
    let with_key = engine.handle_focus_event(&button()).unwrap_or_default();
    assert!(with_key.contains("alt+Y"), "{with_key:?}");

    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(create_test_buffer()),
        Box::new(MockSpeechOutput::new()),
    );
    engine
        .settings
        .verbosity
        .set_modes(OutputType::AccessKey, [OutputMode::Off; 3]);
    let without = engine.handle_focus_event(&button()).unwrap_or_default();
    assert!(without.contains("Yes"), "{without:?}");
    assert!(!without.contains("alt+Y"), "{without:?}");
}

/// The Access Key output also takes a field's shortcut: Chrome's address bar says "Ctrl+L"
/// with it on (dev box) and not with it off (old-pc, 220/s7).
#[test]
fn a_fields_shortcut_follows_the_access_key_output() {
    let address_bar = || {
        let mut focus = FocusEvent::new(Role::Edit).with_name("Address and search bar");
        focus.accelerator_key = Some("Ctrl+L".to_owned());
        focus
    };
    let said = |access_keys: bool| {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(MockSpeechOutput::new()),
        );
        let mode = if access_keys {
            OutputMode::On
        } else {
            OutputMode::Off
        };
        engine
            .settings
            .verbosity
            .set_modes(OutputType::AccessKey, [mode; 3]);
        engine
            .handle_focus_event(&address_bar())
            .unwrap_or_default()
    };
    assert!(said(true).contains("Ctrl+L"), "{:?}", said(true));
    assert!(!said(false).contains("Ctrl+L"), "{:?}", said(false));
}

/// Verbosity acts on a native focus's parts by kind: with the hint output type off at the
/// current level, a button says no usage hint. Okur has no setting that turns it off yet
/// (JAWS's default is on); this is the mechanism the verbosity settings will use.
#[test]
fn the_hint_output_type_drops_native_usage_hints() {
    let said = |hints: OutputMode| {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(MockSpeechOutput::new()),
        );
        engine
            .settings
            .verbosity
            .set_modes(OutputType::Hint, [hints; 3]);
        engine
            .handle_focus_event(&FocusEvent::new(Role::Button).with_name("Yes"))
            .unwrap_or_default()
    };
    assert_eq!(
        said(OutputMode::On),
        "Yes \u{1d} Button \u{1e} \nTo activate press spacebar."
    );
    assert_eq!(said(OutputMode::Off), "Yes \u{1d} Button \u{1e} ");
    // Announce Custom Messages Only: Okur has no custom message, so none is said.
    assert_eq!(said(OutputMode::CustomOnly), "Yes \u{1d} Button \u{1e} ");
}

/// JAWS's Access Key choices: Speak Menus Only says a menu item's access key and not a
/// dialog's control's, Speak Dialogs Only the other way round [the Startup Wizard's walk;
/// documented as menus' and dialogs' access keys].
#[test]
fn access_keys_are_said_in_menus_or_dialogs_only_as_chosen() {
    let said = |mode: OutputMode, focus: &FocusEvent| {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(MockSpeechOutput::new()),
        );
        engine
            .settings
            .verbosity
            .set_modes(OutputType::AccessKey, [mode; 3]);
        engine.keys_as_output(focus).access_key.is_some()
    };
    let mut check = FocusEvent::new(Role::CheckBox).with_name("Match case");
    check.access_key = Some("Alt+C".into());
    check.in_dialog_box = true;
    let mut item = FocusEvent::new(Role::MenuItem).with_name("Copy");
    item.access_key = Some("C".into());
    for (mode, in_dialog, in_menu) in [
        (OutputMode::On, true, true),
        (OutputMode::MenusOnly, false, true),
        (OutputMode::DialogsOnly, true, false),
        (OutputMode::Off, false, false),
    ] {
        assert_eq!(
            (said(mode, &check), said(mode, &item)),
            (in_dialog, in_menu),
            "{mode:?}"
        );
    }
}

/// Without Auto Forms Mode, Tab onto a page's menu bar leaves the virtual cursor there: the
/// item is said as its line, after "menu" and the page's name, and Down reads the next item
/// [obs, 218/s1-s2].
#[test]
fn a_page_menu_in_browse_mode_is_read_by_the_virtual_cursor() {
    let item = |id: usize, name: &str| {
        let mut item = AomNode::new(id, Role::MenuItem).with_name(name);
        item.attributes
            .insert("submenu".to_owned(), "true".to_owned());
        item.states.expanded = Some(false);
        item
    };
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 }).with_text("Web Menus"),
            AomNode::new(2, Role::MenuBar)
                .with_name("Main menu")
                .with_children(vec![item(3, "File"), item(4, "Edit")]),
        ]);
    let speech = SharedSpeech::default();
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let mut changed = engine.settings.clone();
    changed.forms_mode = crate::settings::FormsMode::Manual;
    engine.set_settings(changed);
    let mut file = FocusEvent::new(Role::MenuItem)
        .with_node_id(3)
        .with_name("File");
    file.in_document = true;
    file.in_menu_bar = true;
    file.states.expanded = Some(false);
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(file))
        .unwrap_or_default();
    let lines: Vec<&str> = said.lines().map(str::trim).collect();
    assert_eq!(lines.first().copied(), Some("\u{1d} menu \u{1e}"));
    assert_eq!(lines.get(1).copied(), Some("Web Menus"));
    assert!(
        lines.get(2).is_some_and(|l| l.starts_with("File")),
        "{said:?}"
    );
    assert_eq!(
        lines.last().copied(),
        Some("To move through items press left or right arrow.")
    );
    speech.take();
    engine.handle_key_event(&KeyEvent::new(Key::DownArrow, Modifiers::none(), true));
    let next = speech.take();
    assert!(
        next.first().is_some_and(|l| l.starts_with("Edit")),
        "{next:?}"
    );
}

/// Set Punctuation Level (Insert+Shift+2) moves from Most to All, None, Some and back, saying
/// the new level's name [obs, 400/s6, s12, s18, s24]. It lasts until Okur exits: the settings
/// store is not written.
#[test]
fn insert_shift_2_cycles_the_punctuation_level() {
    use crate::settings::Punctuation;
    let page = AomNode::new(0, Role::Document)
        .with_name("Web Menus")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Hello, world.")
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let store = crate::settings::RecordingSettingsStore::default();
    engine.set_settings_store(Box::new(store.clone()));
    let mut key = KeyEvent::new(Key::Digit(2), Modifiers::none(), true);
    key.modifiers.insert = true;
    key.modifiers.shift = true;
    speech.take();
    let mut said = Vec::new();
    for _ in 0..4 {
        assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
        said.extend(speech.take());
    }
    assert_eq!(said, ["All", "None", "Some", "Most"]);
    assert_eq!(engine.settings.voice.punctuation, Punctuation::Most);
    assert!(store.take().is_empty());
}

/// Typing Echo (Insert+2) moves from None to Characters, Words, Both and back, saying the new
/// echo's name; until Okur exits, as Set Punctuation Level [obs, 403/s7, s14, s21, s28].
#[test]
fn insert_2_cycles_the_typing_echo() {
    use crate::settings::TypingEcho;
    let (mut engine, speech) = engine_with_speech();
    let mut changed = engine.settings.clone();
    changed.typing_echo = TypingEcho::None;
    engine.set_settings(changed);
    let store = crate::settings::RecordingSettingsStore::default();
    engine.set_settings_store(Box::new(store.clone()));
    let mut key = KeyEvent::new(Key::Digit(2), Modifiers::none(), true);
    key.modifiers.insert = true;
    speech.take();
    let mut said = Vec::new();
    for _ in 0..4 {
        assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
        said.extend(speech.take());
    }
    assert_eq!(
        said,
        ["Characters", "Words", "Both characters and words", "None"]
    );
    assert!(store.take().is_empty());
}

/// "ab cd" and Enter typed in a field with each typing echo: the characters with Space and
/// Enter named; the words at Space and Enter, then Enter; both, Space then unnamed; or
/// nothing at all [obs, 403].
#[test]
fn typing_echo_says_characters_words_or_both() {
    use crate::settings::TypingEcho;
    let said = |echo: TypingEcho| {
        let (mut engine, speech) = engine_with_speech();
        let mut changed = engine.settings.clone();
        changed.typing_echo = echo;
        engine.set_settings(changed);
        engine.handle_focus_event(
            &FocusEvent::new(Role::Edit)
                .with_name("Text")
                .with_window_title("SR Reading Fixture"),
        );
        speech.take();
        for (key, text) in [
            (Key::Alpha('A'), Some('a')),
            (Key::Alpha('B'), Some('b')),
            (Key::Space, Some(' ')),
            (Key::Alpha('C'), Some('c')),
            (Key::Alpha('D'), Some('d')),
            (Key::Enter, None),
        ] {
            let mut event = KeyEvent::new(key, Modifiers::none(), true);
            event.text = text;
            engine.handle_key_event(&event);
        }
        speech.take()
    };
    assert_eq!(
        said(TypingEcho::Characters),
        ["a", "b", "Space", "c", "d", "Enter"]
    );
    assert_eq!(said(TypingEcho::Words), ["ab", "cd", "Enter"]);
    assert_eq!(
        said(TypingEcho::Both),
        ["a", "b", "ab", "c", "d", "cd", "Enter"]
    );
    assert!(said(TypingEcho::None).is_empty());
}

/// Numpad 5 is Say Character in an application's text field too: once the character, twice
/// quickly its phonetic word, three times its value
/// [obs, 401/s1-s3].
#[test]
fn numpad_5_says_the_character_its_word_and_its_value() {
    let (mut engine, speech) = engine_with_speech();
    let mut field = FocusEvent::new(Role::Edit)
        .with_name("Text")
        .with_window_title("SR Reading Fixture");
    field.text = Some(crate::event::TextContext {
        text: "ab\r\nJAWS".into(),
        caret: 4,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&field);
    speech.take();
    let mut said = Vec::new();
    for (i, time_ms) in [1000, 1200, 1400, 5000].into_iter().enumerate() {
        let mut key = KeyEvent::new(Key::Numpad5, Modifiers::none(), true);
        key.time_ms = time_ms;
        assert_eq!(
            engine.handle_key_event(&key),
            KeyAction::Consume,
            "press {i}"
        );
        said.extend(speech.take());
    }
    assert_eq!(said, ["J", "juliett", "Character 7 4", "J"]);
}

/// The settings in force follow the focus: an application's own, a site's over its
/// browser's, all applications' elsewhere.
#[test]
fn settings_follow_the_application_and_site_in_front() {
    use crate::settings::Punctuation;
    use serde_json::json;
    let mut page = AomNode::new(0, Role::Document).with_name("Mail");
    page.attributes
        .insert("url".into(), "https://mail.example.org/inbox".into());
    let page = page.with_children(vec![AomNode::new(1, Role::Paragraph).with_text("Inbox")]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut layers = crate::settings_layers::SettingsLayers::default();
    let level = |name: &str| match json!({"voice": {"punctuation": name}}) {
        serde_json::Value::Object(values) => values,
        _ => serde_json::Map::new(),
    };
    layers.applications.insert("notepad".into(), level("all"));
    layers
        .sites
        .insert("mail.example.org".into(), level("none"));
    engine.set_settings_layers(layers);
    let mut notepad = FocusEvent::new(Role::Edit).with_name("Text editor");
    notepad.app = Some("Notepad.exe".into());
    engine.handle_focus_event(&notepad);
    assert_eq!(engine.settings.voice.punctuation, Punctuation::All);
    let mut mail = page_document();
    mail.app = Some("chrome.exe".into());
    engine.handle_focus_event(&mail);
    assert_eq!(engine.settings.voice.punctuation, Punctuation::None);
    let mut explorer = FocusEvent::new(Role::ListItem).with_name("Documents");
    explorer.app = Some("explorer.exe".into());
    engine.handle_focus_event(&explorer);
    assert_eq!(engine.settings.voice.punctuation, Punctuation::Most);
}

/// Numpad Plus leaves forms mode as Escape does, and never types "+" into the field (JAWS 2025
/// Settings Center help, Forms Mode Options; Surf's Up, Forms).
#[test]
fn numpad_plus_leaves_forms_mode() {
    let cursor = VirtualCursor::new(create_test_buffer());
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.execute_command("E");
    let enter = KeyEvent::new(Key::Enter, Modifiers::none(), true);
    assert_eq!(engine.handle_key_event(&enter), KeyAction::Consume);
    assert!(engine.forms_mode);
    let mut plus = KeyEvent::new(Key::NumpadPlus, Modifiers::none(), true);
    plus.text = Some('+');
    assert_eq!(engine.handle_key_event(&plus), KeyAction::Consume);
    assert!(!engine.forms_mode);
}

/// Insert+F10 asks the platform for the windows and opens the Window List on them; Switch To
/// closes it and brings the window chosen to the front (JAWS 2025 help, Window List Dialog Box).
#[test]
fn the_window_list_switches_to_the_window_chosen() {
    use crate::event::{TopWindow, WindowState};
    use crate::ui::{FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.app_mode = AppMode::NativeControl;
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let actions = crate::actions::RecordingActions::reading_windows();
    engine.set_actions(Box::new(actions.clone()));
    let mut f10 = KeyEvent::new(Key::F(10), Modifiers::none(), true);
    f10.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&f10), KeyAction::Consume);
    assert!(actions.requests().iter().any(|(a, _)| a == "list windows"));
    let windows = vec![
        TopWindow {
            id: 11,
            title: "Command Prompt".to_owned(),
            state: WindowState::Minimized,
            on_top: false,
        },
        TopWindow {
            id: 12,
            title: "Untitled - Notepad".to_owned(),
            state: WindowState::Restored,
            on_top: false,
        },
    ];
    engine.handle_accessibility_event(&AccessibilityEvent::TopWindows(windows.clone()));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Window List");
    let id = dialog.id;
    // The windows coming again unasked open nothing.
    engine.handle_accessibility_event(&AccessibilityEvent::TopWindows(windows));
    assert!(ui.take().is_empty());
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "windows".into(),
        value: Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "switch".into(),
    }));
    assert!(matches!(ui.take().first(), Some(ToUi::DialogClose { id: closed }) if *closed == id));
    assert!(actions
        .requests()
        .iter()
        .any(|(a, window)| a == "switch to window" && *window == 12));
}

/// Insert+F11 asks the platform for the notification area's icons and opens the system tray
/// list on them; Enter (its default button, Right Single Click) closes it and right-clicks
/// the icon chosen (JAWS 2025 help, Select a System Tray Icon Dialog Box).
#[test]
fn the_system_tray_list_clicks_the_icon_chosen() {
    use crate::event::TrayIcon;
    use crate::ui::{FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.app_mode = AppMode::NativeControl;
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let actions = crate::actions::RecordingActions::reading_windows();
    engine.set_actions(Box::new(actions.clone()));
    let mut f11 = KeyEvent::new(Key::F(11), Modifiers::none(), true);
    f11.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&f11), KeyAction::Consume);
    assert!(actions
        .requests()
        .iter()
        .any(|(a, _)| a == "list tray icons"));
    let icon = |name: &str, x: i32| TrayIcon {
        name: name.to_owned(),
        x,
        y: 1060,
    };
    engine.handle_accessibility_event(&AccessibilityEvent::TrayIcons(vec![
        icon("Notification Chevron", 1500),
        icon("Touch keyboard", 1600),
    ]));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Select a System Tray Icon");
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "icons".into(),
        value: Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "right".into(),
    }));
    assert!(matches!(ui.take().first(), Some(ToUi::DialogClose { id: closed }) if *closed == id));
    assert!(actions
        .requests()
        .iter()
        .any(|(a, x)| a == "RightSingle click at 1600, 1060" && *x == 1600));
}

/// J opens Jump To Line on the cursor's line, with the page's line count; OK puts the cursor
/// on the line typed; Shift+J returns and reads the line, and says "No previous jump" with
/// none to return to [obs, 559/s1, s7].
#[test]
fn jump_to_line_and_back() {
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let (mut engine, speech) = engine_with_speech();
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let page = AomNode::new(0, Role::Document)
        .with_name("Page")
        .with_children(
            (1..=5)
                .map(|i| AomNode::new(i, Role::Paragraph).with_text(format!("Line {i}")))
                .collect(),
        );
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "Page".to_owned(),
        root: page,
    });
    let mut shift_j = KeyEvent::new(Key::Alpha('J'), Modifiers::none(), true);
    shift_j.modifiers.shift = true;
    speech.take();
    engine.handle_key_event(&shift_j);
    assert_eq!(speech.take(), vec!["No previous jump".to_owned()]);
    engine.handle_key_event(&key(Key::Alpha('J')));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Jump To Line");
    assert_eq!(
        dialog.controls.first().map(|c| (c.label.as_str(), &c.kind)),
        Some((
            "Enter Line Number (1 to 6):",
            &ControlKind::Edit {
                text: "1".to_owned()
            }
        ))
    );
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "line".into(),
        value: Value::Text("4".into()),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".into(),
    }));
    assert_eq!(engine.cursor.current_line(), "Line 3");
    speech.take();
    engine.handle_key_event(&shift_j);
    assert_eq!(speech.take(), vec!["Page".to_owned()]);
}

/// A WinForms data grid [obs, 545]: a cell is said with whether it is selected and its text;
/// Ctrl+End adds "Bottom of file" and the cell again, Say Focus reads a repeat's state.
#[test]
fn a_grid_cell_and_its_file_edges() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let cell = |name: &str, value: &str, selected: Option<bool>| {
        let mut focus = FocusEvent::new(Role::TableCell).with_name(name);
        focus.value = Some(value.to_owned());
        focus.states.selected = selected;
        focus.window_title = Some("SR Data Grid Fixture".to_owned());
        focus
    };
    engine.handle_focus_event(&cell("Item Row 0", "Pens", None));
    // The cell's MSAA report repeats it with its state: not said, but Say Focus has it.
    assert_eq!(
        engine.handle_focus_event(&cell("Item Row 0", "Pens", Some(true))),
        None
    );
    let say_focus = engine.execute(crate::command::Command::SayCurrentFocus);
    assert_eq!(
        say_focus.map(|s| norm(&s)),
        Some(norm("Item Row 0  \u{1d}selected\u{1e} \nPens"))
    );
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::none()
    };
    engine.handle_key_event(&KeyEvent::new(Key::End, ctrl, true));
    assert_eq!(
        engine
            .handle_focus_event(&cell("Price Row 2", "18.00", Some(true)))
            .map(|s| norm(&s)),
        Some(norm(
            "Price Row 2  \u{1d}selected\u{1e} \n18.00\nBottom of file\nPrice Row 2  \u{1d}selected\u{1e} \n18.00"
        ))
    );
    // Another key: the next cell is said alone.
    engine.handle_key_event(&KeyEvent::new(Key::UpArrow, Modifiers::none(), true));
    assert_eq!(
        engine
            .handle_focus_event(&cell("Price Row 1", "4.50", Some(true)))
            .map(|s| norm(&s)),
        Some(norm("Price Row 1  \u{1d}selected\u{1e} \n4.50"))
    );
}

/// The help keys [obs, 526, 527]: Insert+F1 shows the control's help in the Virtual Viewer
/// and reads it all, with the hot keys link and the closing line; Enter on the link opens the
/// hot key help, whose keys are links too; Escape reads the focus again.
#[test]
fn help_keys_open_the_viewer_and_follow_links() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut check_box = FocusEvent::new(Role::CheckBox).with_name("Subscribe to updates");
    check_box.states.checked = Some(true);
    check_box.window_title = Some("SR Desktop Fixture".to_owned());
    engine.handle_focus_event(&check_box);
    let help = engine
        .execute(crate::command::Command::ScreenSensitiveHelp)
        .unwrap_or_default();
    assert_eq!(
        help,
        format!(
            "{}\n\u{1d} {} \u{1e} {}\n{}",
            msg::help_control_check_box(),
            msg::role_link(),
            msg::viewer_hot_keys_link(),
            msg::viewer_close_hint()
        )
    );
    // The reading ends on the last line; Up reaches the link, Enter follows it.
    engine.handle_key_event(&key(Key::UpArrow));
    engine.handle_key_event(&key(Key::Enter));
    let hot_keys = engine
        .viewer
        .as_ref()
        .map(crate::viewer::VirtualViewer::lines)
        .unwrap_or_default();
    assert_eq!(
        hot_keys.first().map(String::as_str),
        Some(msg::help_hot_keys_intro().as_str())
    );
    assert!(hot_keys
        .iter()
        .any(|l| l.ends_with(msg::help_key_window_title().as_str())));
    // Escape: the check box again, as JAWS reads it.
    engine.handle_key_event(&key(Key::Escape));
    assert!(engine.viewer.is_none());
}

/// Windows 10 Notepad [obs, 547]: its edit is said as "edit" and "Type in text." alone; its
/// menu bar, entered with Alt and left with Alt, says "leaving menu bar" and the edit again,
/// though the focus comes back through the window itself, which stays the edit's.
#[test]
fn notepads_edit_and_its_menu_bar() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.app_mode = AppMode::NativeControl;
    let title = "sr-notepad-fixture.txt - Notepad";
    let mut edit = FocusEvent::new(Role::Document).with_name("Text Editor");
    edit.plain_edit = true;
    edit.read_only = Some(false);
    edit.window_title = Some(title.to_owned());
    edit.menu_bar = vec!["File".to_owned(), "Edit".to_owned()];
    edit.text = Some(crate::event::TextContext {
        text: "fn main() {\n}".to_owned(),
        caret: 0,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    let arrival = engine.handle_focus_event(&edit).unwrap_or_default();
    assert!(
        norm(&arrival).ends_with(&norm(&format!(
            "\u{1d} {} \u{1e}\n{}",
            msg::role_edit_multiline(),
            msg::hint_type_in_text()
        ))),
        "{arrival:?}"
    );
    assert!(!arrival.contains("Text Editor"), "{arrival:?}");
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(crate::event::MenuEvent::Start {
        window_class: None,
    }));
    let mut file = FocusEvent::new(Role::MenuItem).with_name("File");
    file.in_menu_bar = true;
    engine.handle_focus_event(&file);
    let mut window = FocusEvent::new(Role::Window).with_name(title);
    window.window_title = Some(title.to_owned());
    let left = engine.handle_focus_event(&window).unwrap_or_default();
    assert_eq!(
        norm(&left),
        norm(&format!(
            "{}\n\u{1d} {} \u{1e}\n{}",
            msg::menu_bar_left(),
            msg::role_edit_multiline(),
            msg::hint_type_in_text()
        ))
    );
    assert_eq!(
        engine.last_native_focus.as_ref().map(|f| f.role.clone()),
        Some(Role::Document)
    );
}

/// The table layer [obs, 535]: Insert+Space, T says "Table layer"; the arrows move by cell,
/// Shift+Up reads the row whole and Shift+Numpad5 the column; Escape leaves the layer
/// silently and Down is the virtual cursor's again. Windows+Alt+Down reads the next row
/// [obs, 297/s5, s6].
#[test]
fn the_table_layer_and_whole_rows() {
    let mut engine = files_table_engine();
    let _ = engine.execute(crate::command::Command::MoveToNext(
        crate::command::Element::Table,
    ));
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    engine.handle_key_event(&KeyEvent::new(Key::Space, insert, true));
    engine.handle_key_event(&key(Key::Alpha('T')));
    assert!(engine.table_layer);
    assert_eq!(
        engine.handle_key_event(&key(Key::DownArrow)),
        KeyAction::Consume
    );
    assert_eq!(engine.cursor.current_table_row(), Some(2));
    let shift = Modifiers {
        shift: true,
        ..Modifiers::none()
    };
    engine.handle_key_event(&KeyEvent::new(Key::UpArrow, shift, true));
    assert_eq!(engine.cursor.current_table_row(), Some(2));
    assert_eq!(
        engine
            .execute(crate::command::Command::SayCurrentRow)
            .as_deref(),
        Some("alpha.rsfirst commit")
    );
    assert_eq!(
        engine
            .execute(crate::command::Command::SayCurrentColumn)
            .as_deref(),
        Some("Namealpha.rsbeta.rs")
    );
    engine.handle_key_event(&key(Key::Escape));
    assert!(!engine.table_layer);
    assert_eq!(
        engine
            .execute(crate::command::Command::ReadNextRow)
            .as_deref(),
        Some("beta.rssecond commit")
    );
    assert_eq!(
        engine.execute(crate::command::Command::ReadNextRow),
        Some(msg::table_on_last_row().to_string())
    );
}

/// Separators, divisions and mail links by quick key [obs, 292, 293]: Minus says "----- |
/// separator", Z a division's line, Backslash a mail link by its text and "Send Mail Link"
/// once the link has the focus, and gives it the focus otherwise.
#[test]
fn separators_divisions_and_mail_links() {
    let mut div = AomNode::new(3, Role::Generic).with_text("A division holds this text.");
    div.attributes.insert("tag".to_owned(), "div".to_owned());
    let page = AomNode::new(0, Role::Document)
        .with_name("Keys")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Start"),
            AomNode::new(2, Role::Separator),
            div,
            AomNode::new(4, Role::Paragraph).with_children(vec![AomNode::new(
                5,
                Role::Link {
                    href: Some("mailto:help@example.org".to_owned()),
                },
            )
            .with_name("help@example.org")]),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    engine.cursor.move_to_start();
    use crate::command::{Command, Element};
    assert_eq!(
        engine
            .execute(Command::MoveToNext(Element::Separator))
            .map(|s| norm(&s)),
        Some(norm(&format!(
            "----- \u{1d} {} \u{1e}",
            msg::role_separator()
        )))
    );
    assert_eq!(
        engine
            .execute(Command::MoveToNext(Element::Division))
            .as_deref(),
        Some("A division holds this text.")
    );
    // Not focused yet: the link is given the focus, which speaks.
    assert_eq!(
        engine
            .execute(Command::MoveToNext(Element::MailLink))
            .as_deref(),
        Some("")
    );
    assert!(
        actions.requests().iter().any(|(a, _)| a == "focus"),
        "{:?}",
        actions.requests()
    );
}

/// Ctrl+Insert+V: the focused application's product name and version, as its executable
/// gives them [obs, 513/s4]; before any focus with them, nothing.
#[test]
fn ctrl_insert_v_says_the_applications_version() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    assert_eq!(engine.execute(crate::command::Command::SayAppVersion), None);
    let mut button = FocusEvent::new(Role::Button)
        .with_name("Save...")
        .with_window_title("SR Dialog Fixture");
    button.app_product = Some("Microsoft® Windows® Operating System".to_owned());
    button.app_version = Some("10.0.19041.1".to_owned());
    engine.handle_focus_event(&button);
    assert_eq!(
        engine
            .execute(crate::command::Command::SayAppVersion)
            .as_deref(),
        Some("Microsoft® Windows® Operating System Version 10.0.19041.1")
    );
}

/// The cursors: Numpad Minus "Okur Cursor" (JAWS's "JAWS Cursor"), Alt+Delete with the mouse's
/// place and the restriction, Insert+Numpad Minus moving the mouse to the middle of the focus,
/// Numpad Plus "PC Cursor", Alt+Delete with the focus's place [obs, 516, 513/s2].
#[test]
fn the_okur_and_pc_cursors() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut button = FocusEvent::new(Role::Button)
        .with_name("Save...")
        .with_window_title("SR Dialog Fixture");
    button.bounds = Some([1000, 530, 1090, 558]);
    engine.handle_focus_event(&button);
    engine.handle_accessibility_event(&crate::event::AccessibilityEvent::MouseMoved {
        x: 1834,
        y: 235,
    });
    use crate::command::Command;
    assert_eq!(
        engine.execute(Command::OkurCursor).as_deref(),
        Some("Okur Cursor")
    );
    assert_eq!(
        engine.execute(Command::SayActiveCursor).as_deref(),
        Some("Okur Cursor active\n1835, 236\nApplication window restriction")
    );
    assert_eq!(
        engine.execute(Command::RouteOkurToPc).as_deref(),
        Some("Route Okur to PC")
    );
    assert!(actions
        .requests()
        .iter()
        .any(|(what, _)| what == "mouse to 1044, 543"));
    assert_eq!(
        engine.execute(Command::PcCursor).as_deref(),
        Some("PC Cursor")
    );
    assert_eq!(
        engine.execute(Command::SayActiveCursor).as_deref(),
        Some("PC Cursor active\n1045, 544")
    );
}

/// A synthesizer that keeps what it is given apart from what it speaks, as Speech On Demand
/// tells it.
#[derive(Clone, Default)]
struct GatedSpeech {
    history: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    spoken: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    unspoken: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl SpeechOutput for GatedSpeech {
    fn speak(&mut self, text: &str, _interrupt: bool) {
        if let Ok(mut history) = self.history.lock() {
            history.push(text.to_owned());
        }
        if !self.unspoken.load(std::sync::atomic::Ordering::SeqCst) {
            if let Ok(mut spoken) = self.spoken.lock() {
                spoken.push(text.to_owned());
            }
        }
    }
    fn stop(&mut self) {}
    fn set_unspoken(&mut self, unspoken: bool) {
        self.unspoken
            .store(unspoken, std::sync::atomic::Ordering::SeqCst);
    }
}

impl GatedSpeech {
    fn take(log: &std::sync::Arc<std::sync::Mutex<Vec<String>>>) -> Vec<String> {
        log.lock()
            .map(|mut l| l.drain(..).collect())
            .unwrap_or_default()
    }
}

/// Speech On Demand: Down says nothing but is written to the history; Insert+Up speaks.
/// Insert+Space, S goes to full speech, saying so, and back [obs, 700/s1, s2, s11-s16].
#[test]
fn speech_on_demand_speaks_only_its_commands() {
    let page = AomNode::new(0, Role::Document).with_children(vec![
        AomNode::new(1, Role::Paragraph).with_text("First line."),
        AomNode::new(2, Role::Paragraph).with_text("Second line."),
    ]);
    let speech = GatedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&page)),
        Box::new(speech.clone()),
    );
    let mut settings = crate::settings::Settings::default();
    settings.speech_mode = crate::settings::SpeechMode::Less;
    engine.set_settings(settings);
    engine.cursor.move_to_start();
    let press = |engine: &mut ScreenReaderEngine, key: Key, modifiers: Modifiers| {
        engine.handle_key_event(&KeyEvent::new(key, modifiers, true));
        (
            GatedSpeech::take(&speech.history).join("\n"),
            GatedSpeech::take(&speech.spoken).join("\n"),
        )
    };
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    let (history, spoken) = press(&mut engine, Key::DownArrow, Modifiers::none());
    assert!(history.contains("Second line."), "{history}");
    assert_eq!(spoken, "");
    let (_, spoken) = press(&mut engine, Key::UpArrow, insert);
    assert!(spoken.contains("Second line."), "{spoken}");
    // Insert+Space, S: full speech, said.
    press(&mut engine, Key::Space, insert);
    let (_, spoken) = press(&mut engine, Key::Alpha('S'), Modifiers::none());
    assert_eq!(spoken, "Full Speech");
    let (_, spoken) = press(&mut engine, Key::UpArrow, Modifiers::none());
    assert!(spoken.contains("First line."), "{spoken}");
    press(&mut engine, Key::Space, insert);
    let (_, spoken) = press(&mut engine, Key::Alpha('S'), Modifiers::none());
    assert_eq!(spoken, "Speech On Demand");
    let (history, spoken) = press(&mut engine, Key::DownArrow, Modifiers::none());
    assert!(history.contains("Second line."), "{history}");
    assert_eq!(spoken, "");
}

/// The Word List: W before a word is chosen says how to choose one; OK moves to the chosen
/// word's first line, found within other words, and W and Shift+W go on from there, wrapping
/// [obs, 560/s3, s6-s10, 766/s5-s8].
#[test]
fn the_word_list_chooses_a_word_for_w() {
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Words")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("Opening hours"),
            AomNode::new(2, Role::Paragraph).with_text("The library opens early"),
            AomNode::new(3, Role::Paragraph).with_text("Hours change"),
            AomNode::new(4, Role::Paragraph).with_text("Reopens in May"),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    assert_eq!(
        engine.execute_command("MoveToNextWordFromList").as_deref(),
        Some("No word list has been created.\nTo create the word list, press control+Insert+W.")
    );
    engine.execute_command("WordList");
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Word List");
    let Some(ControlKind::List { items, .. }) = dialog.controls.first().map(|c| c.kind.clone())
    else {
        panic_free_fail("no list");
        return;
    };
    // "opens" twice ("Reopens" is another word), first of the words seen twice.
    let Some(opens) = items.iter().position(|item| item.starts_with("opens")) else {
        panic_free_fail(&format!("no opens in {items:?}"));
        return;
    };
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "words".into(),
        value: Value::Selected(opens),
    }));
    let moved = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".into(),
    }));
    assert_eq!(moved.as_deref(), Some("The library opens early"));
    assert_eq!(
        engine.execute_command("MoveToNextWordFromList").as_deref(),
        Some("Reopens in May")
    );
    assert_eq!(
        engine.execute_command("MoveToPriorWordFromList").as_deref(),
        Some("The library opens early")
    );
    // Before the first, the search goes on from the bottom [obs, 766/s7].
    assert_eq!(
        engine.execute_command("MoveToPriorWordFromList").as_deref(),
        Some(
            "wrapping to bottom
Reopens in May"
        )
    );
}

/// The Word List's Summary shows the sentences holding the word, whole and whatever its
/// capitals, each numbered and a link to its place (JAWS 2025 help, Word Index).
#[test]
fn the_word_lists_summary_lists_the_sentences_with_the_word() {
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Words")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph).with_text("The Library opens early. Hours change."),
            AomNode::new(2, Role::Paragraph)
                .with_text("Two libraries close. Ask the library staff."),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.execute_command("WordList");
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    let Some(ControlKind::List { items, .. }) = dialog.controls.first().map(|c| c.kind.clone())
    else {
        panic_free_fail("no list");
        return;
    };
    let Some(library) = items
        .iter()
        .position(|item| item.to_lowercase().starts_with("library"))
    else {
        panic_free_fail(&format!("no library in {items:?}"));
        return;
    };
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "words".into(),
        value: Value::Selected(library),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "summary".into(),
    }));
    let shown = ui.take();
    assert!(
        shown.iter().any(|m| matches!(m, ToUi::ViewerShow { title, lines, .. }
            if title == "Word List Summary"
                && lines == &["1. The Library opens early.".to_owned(), "2. Ask the library staff.".to_owned()])),
        "{shown:?}"
    );
}

/// Skim Reading's dialog chooses how Ctrl+Insert+Down reads: by first sentence, each
/// paragraph's first sentence only (JAWS 2025 help, Skim Reading); the choice stays.
#[test]
fn skim_reading_reads_by_the_rule_chosen() {
    use crate::ui::{FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Hours")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph)
                .with_text("The library opens late. It closes at nine."),
            AomNode::new(2, Role::Paragraph)
                .with_text("Weekends stay the same. Sundays are short."),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.execute_command("SkimReadDialog");
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Skim Reading");
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "rule".into(),
        value: Value::Selected(1),
    }));
    let read = engine
        .handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
            id,
            control: "start".into(),
        }))
        .unwrap_or_default();
    // Started from the dialog, the reading ends in its summary, from where it started [obs,
    // 769/s3, 771/s3].
    let sent = ui.take();
    assert_eq!(sent.first(), Some(&ToUi::DialogClose { id }));
    assert!(
        sent.iter()
            .any(|m| matches!(m, ToUi::ViewerShow { lines, .. }
            if lines.first().map(String::as_str) == Some("1. Hours"))),
        "{sent:?}"
    );
    assert!(read.contains("The library opens late."), "{read:?}");
    assert!(read.contains("Weekends stay the same."), "{read:?}");
    assert!(
        !read.contains("nine") && !read.contains("Sundays"),
        "{read:?}"
    );
    // The choice stays for Ctrl+Insert+Down.
    engine.execute_command("TopOfFile");
    let again = engine.execute_command("StartSkimRead").unwrap_or_default();
    assert!(!again.contains("Sundays"), "{again:?}");
}

/// Skimming by sentence reads a list item's mark with its sentence, a link's item by its mark
/// alone, as JAWS does [obs, 769/s3].
#[test]
fn skim_reading_by_sentence_reads_list_items_with_their_mark() {
    use crate::ui::{FromUi, ToUi, Value};
    let link = AomNode::new(
        3,
        Role::Link {
            href: Some("#news".into()),
        },
    )
    .with_name("News");
    let page = AomNode::new(0, Role::Document)
        .with_name("Hours")
        .with_children(vec![AomNode::new(1, Role::List).with_children(vec![
            AomNode::new(2, Role::ListItem)
                .with_children(vec![AomNode::new(5, Role::Text).with_text("• "), link]),
            AomNode::new(4, Role::ListItem).with_children(vec![
                AomNode::new(6, Role::Text).with_text("• "),
                AomNode::new(7, Role::Text).with_text("Weekday hours: 9 a.m. Late on Fridays."),
            ]),
        ])]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.execute_command("SkimReadDialog");
    let Some(ToUi::DialogShow { dialog }) = ui.take().first().cloned() else {
        panic_free_fail("no dialog shown");
        return;
    };
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "rule".into(),
        value: Value::Selected(1),
    }));
    let read = engine
        .handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
            id,
            control: "start".into(),
        }))
        .unwrap_or_default();
    let lines: Vec<&str> = read.lines().map(str::trim).collect();
    assert!(lines.contains(&"• Weekday hours: 9 a.m."), "{read:?}");
    assert!(lines.contains(&"•"), "{read:?}");
    assert!(!read.contains("Fridays"), "{read:?}");
}

/// A toast is said as it shows, and Insert+Space, Shift+N says it again; before one, "No
/// notification"; Insert+Space, Ctrl+N stops and starts saying them [obs, 770/s1, s4, 533/s3,
/// 773].
#[test]
fn a_toast_is_said_and_repeated_on_request() {
    let (mut engine, speech) = engine_on_a_page();
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    let shift = Modifiers {
        shift: true,
        ..Modifiers::none()
    };
    let layer_key = |engine: &mut ScreenReaderEngine, key: Key, modifiers: Modifiers| {
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(Key::Space, insert, down));
        }
        speech.take();
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(key, modifiers, down));
        }
        speech.take()
    };
    assert_eq!(
        layer_key(&mut engine, Key::Alpha('N'), shift),
        ["No notification"]
    );
    const TOAST: &str = "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1";
    assert_eq!(
        engine
            .handle_accessibility_event(&AccessibilityEvent::Toast {
                text: TOAST.to_owned(),
                sender: Some("Windows PowerShell".to_owned()),
            })
            .as_deref(),
        Some(TOAST)
    );
    assert_eq!(layer_key(&mut engine, Key::Alpha('N'), shift), [TOAST]);
    // Insert+Space, Ctrl+N: toasts kept, not said, until it is pressed again [obs, 773].
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::none()
    };
    assert_eq!(
        layer_key(&mut engine, Key::Alpha('N'), ctrl),
        ["Toast notification announcements disabled"]
    );
    const LATER: &str = "New notification from Mail, Lunch, Noon at the cafe. 1 of 1";
    assert_eq!(
        engine.handle_accessibility_event(&AccessibilityEvent::Toast {
            text: LATER.to_owned(),
            sender: Some("Mail".to_owned()),
        }),
        None
    );
    assert_eq!(layer_key(&mut engine, Key::Alpha('N'), shift), [LATER]);
    assert_eq!(
        layer_key(&mut engine, Key::Alpha('N'), ctrl),
        ["Toast notification announcements enabled"]
    );
}

/// Skim Reading's Create Summary: what skim reading would read, numbered, the page's title
/// first, in the viewer "Skim Reading Results"; Enter on a piece moves the virtual cursor there (JAWS 2025 help, Skim
/// Reading Dialog; [obs, 762/s2]). Close keeps no choice: Ctrl+Insert+Down reads by first line.
#[test]
fn create_summary_lists_what_skim_reading_reads() {
    use crate::ui::{FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Hours")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph)
                .with_text("The library opens late. It closes at nine."),
            AomNode::new(2, Role::Paragraph)
                .with_text("Weekends stay the same. Sundays are short."),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let open = |engine: &mut ScreenReaderEngine| {
        engine.execute_command("SkimReadDialog");
        match ui.take().first() {
            Some(ToUi::DialogShow { dialog }) => dialog.id,
            _ => 0,
        }
    };
    let id = open(&mut engine);
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "rule".into(),
        value: Value::Selected(1),
    }));
    let said = engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "summary".into(),
    }));
    assert_eq!(
        said.as_deref(),
        Some("Skim Reading Results\n1. Hours \u{1d} Link \u{1e} \nTo activate press Enter.")
    );
    let shown = ui.take();
    assert!(
        shown
            .iter()
            .any(|m| matches!(m, ToUi::ViewerShow { title, lines, .. }
            if title == "Skim Reading Results"
                && lines.last().map(String::as_str) == Some("3. Weekends stay the same."))),
        "{shown:?}"
    );
    speech.take();
    for key in [Key::DownArrow, Key::DownArrow, Key::Enter] {
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(key, Modifiers::none(), down));
        }
    }
    let spoken = speech.take().join(" ");
    assert!(spoken.contains("Weekends stay the same."), "{spoken}");
    assert!(engine
        .execute_command("SayLine")
        .is_some_and(|line| line.contains("Weekends")));
    // The summary chose by sentence without keeping it; Close keeps nothing either.
    let id = open(&mut engine);
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "rule".into(),
        value: Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "cancel".into(),
    }));
    engine.execute_command("TopOfFile");
    let skimmed = engine.execute_command("StartSkimRead").unwrap_or_default();
    assert!(skimmed.contains("It closes at nine."), "{skimmed:?}");
}

/// An application's own notification (UI Automation's) is said as it comes, with toast
/// announcements off too, kept in the history under its program's name, and said again by
/// Insert+Space, Shift+N [obs, 820/s1, s3, s8].
#[test]
fn an_applications_notification_is_said_and_kept_by_its_program() {
    use crate::ui::{ControlKind, ToUi};
    let (mut engine, speech) = engine_on_a_page();
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    let layer = |engine: &mut ScreenReaderEngine, key: Key, modifiers: Modifiers| {
        for (key, modifiers) in [(Key::Space, insert), (key, modifiers)] {
            for down in [true, false] {
                engine.handle_key_event(&KeyEvent::new(key, modifiers, down));
            }
        }
    };
    // Toast announcements off: Insert+Space, Ctrl+N.
    let ctrl = Modifiers {
        ctrl: true,
        ..Modifiers::none()
    };
    let shift = Modifiers {
        shift: true,
        ..Modifiers::none()
    };
    layer(&mut engine, Key::Alpha('N'), ctrl);
    speech.take();
    let said = engine.handle_accessibility_event(&AccessibilityEvent::AppNotification {
        text: "Upload finished: report.pdf".to_owned(),
        politeness: Politeness::Assertive,
        app: Some("powershell".to_owned()),
    });
    assert_eq!(said.as_deref(), Some("Upload finished: report.pdf"));
    layer(&mut engine, Key::Alpha('N'), shift);
    let repeated = speech.take();
    assert!(
        repeated
            .iter()
            .any(|said| said.contains("Upload finished: report.pdf")),
        "Shift+N repeats it: {repeated:?}"
    );
    layer(&mut engine, Key::Alpha('N'), Modifiers::none());
    let Some(ToUi::DialogShow { dialog }) = ui.take().first().cloned() else {
        panic_free_fail("no history shown");
        return;
    };
    // Without a clock the time is left out.
    assert_eq!(
        dialog.controls.first().map(|c| c.kind.clone()),
        Some(ControlKind::ListBox {
            items: vec!["Upload finished: report.pdf powershell".to_owned()],
            selected: 0
        })
    );
}

/// The clipboard keys in a text field [obs, 825]: Okur key+Windows+C adds the selection to the
/// clipboard on a line of its own, Windows+Okur key+X reads it a line each, and Ctrl+C after it
/// asks before overwriting the clipboard; Yes copies the selection.
#[test]
fn the_clipboard_keys_append_say_and_ask_before_overwriting() {
    use crate::actions::ApplicationActions;
    use crate::ui::{FromUi, ToUi};
    let (mut engine, speech) = engine_with_speech();
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let text = "fn main() {\r\nlet total = 1;";
    let second = text.find("let").unwrap_or_default();
    let context = |selection: (usize, usize)| crate::event::TextContext {
        text: text.to_owned(),
        caret: selection.1,
        selection: Some(selection),
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    };
    let mut field = FocusEvent::new(Role::Edit).with_window_title("Notes - Notepad");
    field.multi_line = true;
    field.text = Some(context((0, 11)));
    engine.handle_focus_event(&field);
    actions.set_clipboard("fn main() {");
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context((
        second,
        text.len(),
    ))));
    let with = |insert: bool, win: bool, ctrl: bool| Modifiers {
        insert,
        win,
        ctrl,
        ..Modifiers::none()
    };
    speech.take();
    engine.handle_key_event(&KeyEvent::new(
        Key::Alpha('C'),
        with(true, true, false),
        true,
    ));
    assert_eq!(
        actions.clipboard_text().as_deref(),
        Some("fn main() {\r\nlet total = 1;")
    );
    assert!(norm(&speech.take().join("\n")).contains("Appended selection to clipboard"));
    engine.handle_key_event(&KeyEvent::new(
        Key::Alpha('X'),
        with(true, true, false),
        true,
    ));
    assert_eq!(
        norm(&speech.take().join("\n")),
        "fn main() { let total = 1;"
    );
    assert_eq!(
        engine.handle_key_event(&KeyEvent::new(
            Key::Alpha('C'),
            with(false, false, true),
            true
        )),
        KeyAction::Consume
    );
    let Some(ToUi::DialogShow { dialog }) = ui.take().first().cloned() else {
        panic_free_fail("no question shown");
        return;
    };
    assert_eq!(dialog.title, "Overwrite Clipboard");
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "yes".into(),
    }));
    assert_eq!(actions.clipboard_text().as_deref(), Some("let total = 1;"));
    assert!(norm(&speech.take().join("\n")).contains("Copied selection to clipboard"));
}

/// Insert+Space, N: the Notification History, the latest first, each row the text, sender and
/// time; Enter on a row shows its details; Clear History empties it [obs, 774].
#[test]
fn the_notification_history_lists_the_toasts_that_came() {
    use crate::ui::{ControlKind, FromUi, ToUi};
    let (mut engine, speech) = engine_on_a_page();
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    for (text, sender) in [
        ("New notification from Mail, Lunch. 1 of 1", "Mail"),
        (
            "New notification from Windows PowerShell, Build finished. 1 of 1",
            "Windows PowerShell",
        ),
    ] {
        engine.handle_accessibility_event(&AccessibilityEvent::Toast {
            text: text.to_owned(),
            sender: Some(sender.to_owned()),
        });
    }
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    for (key, modifiers) in [(Key::Space, insert), (Key::Alpha('N'), Modifiers::none())] {
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(key, modifiers, down));
        }
    }
    speech.take();
    let Some(ToUi::DialogShow { dialog }) = ui.take().first().cloned() else {
        panic_free_fail("no history shown");
        return;
    };
    assert_eq!(dialog.title, "Notification History");
    // Without a clock the time is left out.
    assert_eq!(
        dialog.controls.first().map(|c| c.kind.clone()),
        Some(ControlKind::ListBox {
            items: vec![
                "New notification from Windows PowerShell, Build finished. 1 of 1 Windows PowerShell".to_owned(),
                "New notification from Mail, Lunch. 1 of 1 Mail".to_owned(),
            ],
            selected: 0
        })
    );
    let id = dialog.id;
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "notifications".into(),
    }));
    let Some(ToUi::DialogShow { dialog: details }) = ui.take().first().cloned() else {
        panic_free_fail("no details shown");
        return;
    };
    assert_eq!(details.title, "Notification Details");
    assert_eq!(
        details.controls.first().map(|c| c.kind.clone()),
        Some(ControlKind::Note {
            text: "New notification from Windows PowerShell, Build finished. 1 of 1".to_owned()
        })
    );
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed {
        id: details.id,
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "clear".into(),
    }));
    assert!(ui
        .take()
        .iter()
        .any(|m| matches!(m, ToUi::DialogItems { items, .. } if items.is_empty())));
    for (key, modifiers) in [
        (Key::Space, insert),
        (
            Key::Alpha('N'),
            Modifiers {
                shift: true,
                ..Modifiers::none()
            },
        ),
    ] {
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(key, modifiers, down));
        }
    }
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("No notification")
    );
}

/// A rule that plays a sound plays its file in the notification's place and says nothing
/// (JAWS 2025 help, Notification History: "plays a specified sound instead of speaking").
#[test]
fn a_play_sound_rule_plays_its_file_instead() {
    use crate::ui::notifications::{NotificationRules, Rule, RuleType, SpeechAction};
    let (mut engine, speech) = engine_on_a_page();
    let sounds = crate::sound::RecordingSounds::default();
    engine.set_sounds(Box::new(sounds.clone()));
    engine.set_notification_rules(
        NotificationRules {
            enabled: true,
            rules: vec![Rule {
                name: "Build".into(),
                kind: RuleType::Contains,
                text: "Build finished".into(),
                sender: None,
                speech: SpeechAction::PlaySound,
                braille: 1,
                hidden: false,
                shortened: String::new(),
                sound: r"C:\Windows\Media\chimes.wav".into(),
                enabled: true,
            }],
        },
        None,
    );
    let _ = speech.take();
    let said = engine.handle_accessibility_event(&AccessibilityEvent::Toast {
        text: "New notification from Windows PowerShell, SR Fixture, Build finished in 12 seconds. 1 of 1".to_owned(),
        sender: Some("Windows PowerShell".to_owned()),
    });
    assert_eq!(said, None);
    assert!(speech.take().is_empty());
    assert_eq!(sounds.take(), [r"C:\Windows\Media\chimes.wav"]);
    // Another notification is said, and plays nothing.
    let said = engine.handle_accessibility_event(&AccessibilityEvent::Toast {
        text: "New notification from Mail, Lunch. 1 of 1".to_owned(),
        sender: Some("Mail".to_owned()),
    });
    assert!(said.is_some());
    assert!(sounds.take().is_empty());
}

/// Create Rule... on a notification makes a rule, Mute at first: the next notification it
/// matches is kept, not said, and Shift+N still says it (JAWS 2025 help, Notification History;
/// [obs, 775]). Enable Rules unchecked, it is said again.
#[test]
fn a_notification_rule_mutes_what_it_matches() {
    use crate::ui::{FromUi, ToUi, Value};
    let (mut engine, speech) = engine_on_a_page();
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let toast = |engine: &mut ScreenReaderEngine, text: &str| {
        engine.handle_accessibility_event(&AccessibilityEvent::Toast {
            text: text.to_owned(),
            sender: Some("Mail".to_owned()),
        })
    };
    toast(
        &mut engine,
        "New notification from Mail, Weekly digest. 1 of 1",
    );
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    for (key, modifiers) in [(Key::Space, insert), (Key::Alpha('N'), Modifiers::none())] {
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(key, modifiers, down));
        }
    }
    let history = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) => dialog.id,
        _ => 0,
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: history,
        control: "create".into(),
    }));
    let rule = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) if dialog.title == "Create Rule" => dialog.id,
        other => {
            panic_free_fail(&format!("no Create Rule: {other:?}"));
            return;
        }
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id: rule,
        control: "text".into(),
        value: Value::Text("Weekly digest".into()),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: rule,
        control: "ok".into(),
    }));
    // Name Your Rule, the notification's text proposed [obs, 783/s7].
    let naming = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) if dialog.title == "Name Your Rule" => dialog.id,
        other => {
            panic_free_fail(&format!("no Name Your Rule: {other:?}"));
            return;
        }
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id: naming,
        control: "name".into(),
        value: Value::Text("Digest".into()),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: naming,
        control: "ok".into(),
    }));
    assert_eq!(
        ui.take(),
        [
            ToUi::DialogClose { id: naming },
            ToUi::DialogClose { id: rule },
            ToUi::DialogEnable {
                id: history,
                control: "manage".into(),
                enabled: true
            }
        ]
    );
    speech.take();
    assert_eq!(
        toast(
            &mut engine,
            "New notification from Mail, Weekly digest. 1 of 1"
        ),
        None
    );
    assert!(speech.take().is_empty());
    assert!(toast(&mut engine, "New notification from Mail, Lunch. 1 of 1").is_some());
    // The first rule made, Manage Rules... is available and opens [obs, 787/s8].
    let _ = ui.take();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: history,
        control: "manage".into(),
    }));
    let manage = ui.take().into_iter().find_map(|m| match m {
        ToUi::DialogShow { dialog } => Some(dialog),
        _ => None,
    });
    assert_eq!(
        manage.as_ref().map(|d| d.title.as_str()),
        Some("Manage Notification Rules")
    );
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed {
        id: manage.map_or(0, |d| d.id),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id: history,
        control: "rules".into(),
        value: Value::Checked(false),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed {
        id: history,
    }));
    assert!(toast(
        &mut engine,
        "New notification from Mail, Weekly digest. 1 of 1"
    )
    .is_some());
}

/// Skimming by a text rule reads the lines that hold its pattern, whatever its capitals; by
/// sentence, the sentences (JAWS 2025 help, Skim Reading Dialog; [obs, 776, 777]).
#[test]
fn skim_reading_by_a_text_rule_reads_what_holds_the_pattern() {
    use crate::ui::{FromUi, ToUi, Value};
    let page = AomNode::new(0, Role::Document)
        .with_name("Hours")
        .with_children(vec![
            AomNode::new(1, Role::Paragraph)
                .with_text("The Library opens late. It closes at nine."),
            AomNode::new(2, Role::Paragraph).with_text("Weekends stay the same."),
            AomNode::new(3, Role::Paragraph).with_text("Ask the library staff. They know."),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let skim = |engine: &mut ScreenReaderEngine, unit: usize| {
        engine.execute_command("TopOfFile");
        engine.execute_command("SkimReadDialog");
        let id = ui
            .take()
            .iter()
            .find_map(|m| match m {
                ToUi::DialogShow { dialog } => Some(dialog.id),
                _ => None,
            })
            .unwrap_or(0);
        for (control, value) in [
            ("rule", Value::Selected(2)),
            ("pattern", Value::Text("library".into())),
            ("unit", Value::Selected(unit)),
        ] {
            engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
                id,
                control: control.into(),
                value,
            }));
        }
        engine
            .handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
                id,
                control: "start".into(),
            }))
            .unwrap_or_default()
    };
    let by_line = skim(&mut engine, 0);
    assert!(
        by_line.contains("The Library opens late. It closes at nine."),
        "{by_line:?}"
    );
    assert!(by_line.contains("Ask the library staff."), "{by_line:?}");
    assert!(!by_line.contains("Weekends"), "{by_line:?}");
    let by_sentence = skim(&mut engine, 1);
    assert!(
        by_sentence.contains("The Library opens late."),
        "{by_sentence:?}"
    );
    assert!(
        !by_sentence.contains("nine") && !by_sentence.contains("They know"),
        "{by_sentence:?}"
    );
}

/// Skimming by an attribute rule with Bold checked reads the lines holding bold text: a
/// heading, a paragraph's line with a bold word inline, not a plain paragraph; then how many
/// were found, and the last line's first word [obs, 799/s10].
#[test]
fn skim_reading_by_bold_reads_the_lines_with_bold_text() {
    use crate::aom::TextFormatting;
    use crate::ui::{FromUi, ToUi, Value};
    let bold = TextFormatting {
        bold: true,
        ..TextFormatting::default()
    };
    let plain = TextFormatting::default();
    let page = AomNode::new(0, Role::Document)
        .with_name("Hours")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 1 })
                .with_text("Library hours")
                .with_formatting(bold.clone()),
            AomNode::new(2, Role::Paragraph).with_children(vec![
                AomNode::new(3, Role::Text)
                    .with_text("Open until ")
                    .with_formatting(plain.clone()),
                AomNode::new(4, Role::Text)
                    .with_text("9 p.m.")
                    .with_formatting(bold),
                AomNode::new(5, Role::Text)
                    .with_text(" on weekdays.")
                    .with_formatting(plain.clone()),
            ]),
            AomNode::new(6, Role::Paragraph)
                .with_text("Weekends stay the same.")
                .with_formatting(plain),
        ]);
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&page));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    engine.handle_focus_event(&page_document());
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.execute_command("TopOfFile");
    engine.execute_command("SkimReadDialog");
    let id = ui
        .take()
        .iter()
        .find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog.id),
            _ => None,
        })
        .unwrap_or(0);
    for (control, value) in [("rule", Value::Selected(3)), ("bold", Value::Checked(true))] {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
            id,
            control: control.into(),
            value,
        }));
    }
    let read = engine
        .handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
            id,
            control: "start".into(),
        }))
        .unwrap_or_default();
    assert!(read.contains("Library hours"), "{read:?}");
    assert!(read.contains("Open until 9 p.m. on weekdays."), "{read:?}");
    assert!(!read.contains("Weekends"), "{read:?}");
    // The count, then the word the last line's bold text starts in, where the cursor stops
    // ("11 matches found | Related" [obs, 799/s10]; "on" for a struck "on Monday" mid-line
    // [obs, 809/s13]).
    assert!(read.ends_with("2 matches found\n9"), "{read:?}");
}

/// Manage Rules: the rules' rows as JAWS's, "Digest Mail Mute Show flash message Enabled";
/// Disable, Rename... and Delete with its question [obs, 787/s8-s18].
#[test]
fn manage_rules_disables_renames_and_deletes() {
    use crate::ui::notifications::{NotificationRules, Rule, RuleType, SpeechAction};
    use crate::ui::{FromUi, ToUi, Value};
    let (mut engine, _speech) = engine_on_a_page();
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.set_notification_rules(
        NotificationRules {
            enabled: true,
            rules: vec![Rule {
                name: "Digest".into(),
                kind: RuleType::Contains,
                text: "Weekly digest".into(),
                sender: Some("Mail".into()),
                speech: SpeechAction::Mute,
                braille: 1,
                hidden: false,
                shortened: String::new(),
                sound: String::new(),
                enabled: true,
            }],
        },
        None,
    );
    let shown = |ui: &crate::ui::RecordingUiHost| {
        ui.take().into_iter().find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog),
            _ => None,
        })
    };
    let press = |engine: &mut ScreenReaderEngine, id: u32, control: &str| {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
            id,
            control: control.into(),
        }));
    };
    let insert = Modifiers {
        insert: true,
        ..Modifiers::none()
    };
    for (key, modifiers) in [(Key::Space, insert), (Key::Alpha('N'), Modifiers::none())] {
        for down in [true, false] {
            engine.handle_key_event(&KeyEvent::new(key, modifiers, down));
        }
    }
    let history = shown(&ui).map(|d| d.id).unwrap_or(0);
    press(&mut engine, history, "manage");
    let Some(manage) = shown(&ui) else {
        panic_free_fail("no Manage Rules");
        return;
    };
    assert_eq!(manage.title, "Manage Notification Rules");
    let rows = |messages: Vec<ToUi>| {
        messages.into_iter().find_map(|m| match m {
            ToUi::DialogItems { items, .. } => Some(items),
            _ => None,
        })
    };
    press(&mut engine, manage.id, "toggle");
    let toggled = ui.take();
    // The list takes the focus back, as JAWS's does [obs, 787/s16].
    assert!(toggled.iter().any(|m| matches!(
        m,
        ToUi::DialogFocus { id, control } if *id == manage.id && control == "rules"
    )));
    assert_eq!(
        rows(toggled),
        Some(vec![
            "Digest Mail Mute Show flash message Disabled".to_owned()
        ])
    );
    press(&mut engine, manage.id, "rename");
    let naming = shown(&ui).map(|d| d.id).unwrap_or(0);
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id: naming,
        control: "name".into(),
        value: Value::Text("Mail digest".into()),
    }));
    press(&mut engine, naming, "ok");
    assert_eq!(
        rows(ui.take()),
        Some(vec![
            "Mail digest Mail Mute Show flash message Disabled".to_owned()
        ])
    );
    press(&mut engine, manage.id, "delete");
    let confirm = match shown(&ui) {
        Some(dialog) if dialog.title == "Confirm Delete" => dialog.id,
        other => {
            panic_free_fail(&format!("no question: {other:?}"));
            return;
        }
    };
    press(&mut engine, confirm, "ok");
    assert_eq!(rows(ui.take()), Some(Vec::new()));
}

/// A page the host marked with the reading choices before building it is taken as built; a
/// page without the marks is built again with them (performance review 2026-10-23, C2).
#[test]
fn a_page_marked_with_the_reading_choices_is_not_built_again() {
    let page = || {
        AomNode::new(0, Role::Document)
            .with_name("Pictures")
            .with_children(vec![
                AomNode::new(1, Role::Image).with_name("A lighthouse"),
                AomNode::new(2, Role::Paragraph).with_text("At dusk."),
            ])
    };
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(create_test_buffer()),
        Box::new(MockSpeechOutput::default()),
    );
    engine.settings.show_graphics = false;
    let marks = engine.reading_marks();
    let mut marked = page();
    marks.mark(&mut marked);
    assert!(marks.on(&marked));
    let buffer = VirtualBuffer::from_aom_owned(marked);
    let built = buffer.page().map(|p| std::sync::Arc::as_ptr(p) as usize);
    let prepared = engine.prepared(buffer);
    assert_eq!(
        prepared.page().map(|p| std::sync::Arc::as_ptr(p) as usize),
        built
    );
    assert!(!prepared.text.contains("lighthouse"), "{}", prepared.text);

    let prepared = engine.prepared(VirtualBuffer::from_aom(&page()));
    assert!(prepared.page().is_some_and(|p| marks.on(p)));
    assert!(!prepared.text.contains("lighthouse"), "{}", prepared.text);
}

/// The text typed in a page's field goes with the field when a refresh numbers the page anew:
/// to the same object's new number, not to the field the old number now gives (performance
/// review 2026-10-23, C21).
#[test]
fn typed_field_text_follows_its_field_into_a_refreshed_page() {
    let field = |id: usize, object: &str, name: &str| {
        let mut node = AomNode::new(id, Role::Edit).with_name(name);
        node.attributes
            .insert(crate::aom::UNIQUE_ID_ATTR.to_owned(), object.to_owned());
        node
    };
    let page = |children: Vec<AomNode>| {
        let mut page = AomNode::new(0, Role::Document)
            .with_name("Sign up")
            .with_children(children);
        page.attributes
            .insert("url".to_owned(), "https://example.test/".to_owned());
        VirtualBuffer::from_aom_owned(page)
    };
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(page(vec![
            field(3, "-10", "Full name"),
            field(4, "-11", "Email"),
        ])),
        Box::new(MockSpeechOutput::default()),
    );
    engine.field_texts.insert(3, "ada".to_owned());
    let title = engine
        .cursor
        .buffer
        .document_title()
        .unwrap_or_default()
        .to_owned();
    engine.handle_page_refreshed(
        &title,
        page(vec![field(3, "-11", "Email"), field(7, "-10", "Full name")]),
    );
    engine.apply_pending_page();
    assert_eq!(engine.field_texts.get(&7).map(String::as_str), Some("ada"));
    assert_eq!(engine.field_texts.get(&3), None);
}

/// Insert+Z says "use virtual PC cursor off" at Beginner and "Off" above it, then "On"; Insert+
/// Escape says "Screen updated" but at Advanced, where JAWS leaves its own messages out
/// [obs, 298/s9-s13; 841/s9, s11; 842/s9, s11, s13].
#[test]
fn the_cursor_toggle_and_refresh_say_the_levels_messages() {
    use crate::settings::VerbosityLevel;
    for (level, off, on, refreshed) in [
        (
            VerbosityLevel::Beginner,
            "use virtual PC cursor off",
            "use virtual PC cursor on",
            Some("Screen updated"),
        ),
        (
            VerbosityLevel::Intermediate,
            "Off",
            "On",
            Some("Screen updated"),
        ),
        (VerbosityLevel::Advanced, "Off", "On", None),
    ] {
        let speech = SharedSpeech::default();
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(speech.clone()),
        );
        let mut settings = engine.settings.clone();
        settings.verbosity.level = level;
        engine.set_settings(settings);
        speech.take();
        engine.execute_command("VirtualPCCursorToggle");
        assert_eq!(speech.take(), vec![off.to_owned()], "{level:?}");
        engine.execute_command("VirtualPCCursorToggle");
        assert_eq!(speech.take(), vec![on.to_owned()], "{level:?}");
        engine.execute_command("RefreshScreen");
        assert_eq!(
            speech.take(),
            refreshed.map(str::to_owned).into_iter().collect::<Vec<_>>(),
            "{level:?}"
        );
    }
}

/// Append Selected Text to Clipboard says "Appended selection to clipboard" at Beginner,
/// "Appended" at Intermediate and nothing at Advanced; the clipboard is added to at every
/// level [obs, 825/s11, 859/s11, 860/s11].
#[test]
fn appending_to_the_clipboard_is_said_at_the_levels_length() {
    use crate::actions::ApplicationActions as _;
    use crate::settings::VerbosityLevel;
    for (level, said) in [
        (
            VerbosityLevel::Beginner,
            vec!["Appended selection to clipboard"],
        ),
        (VerbosityLevel::Intermediate, vec!["Appended"]),
        (VerbosityLevel::Advanced, Vec::new()),
    ] {
        let (mut engine, speech) = engine_with_speech();
        let mut settings = engine.settings.clone();
        settings.verbosity.level = level;
        engine.set_settings(settings);
        let actions = crate::actions::RecordingActions::default();
        engine.set_actions(Box::new(actions.clone()));
        let mut field = FocusEvent::new(Role::Edit).with_window_title("Notes - Notepad");
        field.multi_line = true;
        field.text = Some(crate::event::TextContext {
            text: "one two".to_owned(),
            caret: 7,
            selection: Some((4, 7)),
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        });
        engine.handle_focus_event(&field);
        actions.set_clipboard("one");
        speech.take();
        engine.handle_key_event(&KeyEvent::new(
            Key::Alpha('C'),
            Modifiers {
                insert: true,
                win: true,
                ..Modifiers::none()
            },
            true,
        ));
        assert_eq!(
            actions.clipboard_text().as_deref(),
            Some("one\r\ntwo"),
            "{level:?}"
        );
        assert_eq!(speech.take(), said, "{level:?}");
    }
}

/// A cell key past a table's edge says "end of row" or "bottom of column", and nothing at
/// Advanced, where JAWS leaves its own messages out; the key is taken at every level
/// [obs, 869/s4, s7; 870/s4, s7].
#[test]
fn a_tables_edge_is_said_but_at_advanced() {
    use crate::settings::VerbosityLevel;
    for (level, row_end, column_bottom) in [
        (VerbosityLevel::Beginner, "end of row", "bottom of column"),
        (
            VerbosityLevel::Intermediate,
            "end of row",
            "bottom of column",
        ),
        (VerbosityLevel::Advanced, "", ""),
    ] {
        let mut engine = files_table_engine();
        let mut settings = engine.settings.clone();
        settings.verbosity.level = level;
        engine.set_settings(settings);
        let _ = engine.cursor.find_text("second commit", true);
        assert_eq!(
            engine.execute_command("TableCellRight").as_deref(),
            Some(row_end),
            "{level:?}"
        );
        assert_eq!(
            engine.execute_command("TableCellDown").as_deref(),
            Some(column_bottom),
            "{level:?}"
        );
    }
}

/// With indentation said, Say Line puts "Left Margin" before a page line at the margin, and a
/// Say All its first line; off (JAWS's default), neither [obs, 729/s1, s10].
#[test]
fn indentation_says_left_margin_before_a_line_at_the_margin() {
    for indentation in [true, false] {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(MockSpeechOutput::default()),
        );
        let mut settings = engine.settings.clone();
        settings.indentation = indentation;
        engine.set_settings(settings);
        engine.app_mode = AppMode::VirtualBuffer;
        let _ = engine
            .cursor
            .find_text("This is interactive paragraph.", true);
        let said = engine.execute_command("SayLine").unwrap_or_default();
        let read = engine.start_say_all().unwrap_or_default();
        let first = read.lines().next().unwrap_or_default().to_owned();
        if indentation {
            assert!(said.contains("Left Margin"), "{said:?}");
            assert!(first.contains("Left Margin"), "{first:?}");
            assert_eq!(read.matches("Left Margin").count(), 1, "{read:?}");
        } else {
            assert!(!said.contains("Left Margin") && !read.contains("Left Margin"));
        }
    }
}

/// A quick key with nothing of its kind on the page says "There are no ... on this page." at
/// Beginner and "No ..." with short messages; Insert+F5 "no form fields were found" and "no
/// form fields" [obs, 883/s2, s4; 887/s4].
#[test]
fn nothing_found_is_said_at_the_levels_length() {
    use crate::settings::VerbosityLevel;
    for (level, buttons, placemarkers) in [
        (
            VerbosityLevel::Beginner,
            "There are no clickable elements on this page.",
            "There are no PlaceMarkers on this page.",
        ),
        (
            VerbosityLevel::Intermediate,
            "No clickable elements",
            "No PlaceMarkers",
        ),
        (
            VerbosityLevel::Advanced,
            "No clickable elements",
            "No PlaceMarkers",
        ),
    ] {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(create_test_buffer()),
            Box::new(MockSpeechOutput::default()),
        );
        let mut settings = engine.settings.clone();
        settings.verbosity.level = level;
        engine.set_settings(settings);
        engine.app_mode = AppMode::VirtualBuffer;
        assert_eq!(
            engine.execute_command("/").as_deref(),
            Some(buttons),
            "{level:?}"
        );
        assert_eq!(
            engine.execute_command("K").as_deref(),
            Some(placemarkers),
            "{level:?}"
        );
    }
    assert_eq!(
        crate::messages::msg::form_fields_list_none()
            .short()
            .map(|m| m.as_str()),
        Some("no form fields")
    );
    assert!(crate::messages::msg::blank().short().is_none());
}

/// A classic console's new rows after Enter are said: the command's output and the next
/// prompt; what is typed at the prompt is not new [obs, 962/s1, s2]. With Screen Echo off,
/// nothing [obs, 963/s2].
#[test]
fn a_consoles_new_output_is_said_while_screen_echo_is_on() {
    let console = |text: &str| {
        let mut focus = FocusEvent::new(Role::Document).with_name("Text Area");
        focus.platform.top_window_class = Some("ConsoleWindowClass".to_owned());
        focus.text = Some(crate::TextContext {
            text: text.to_owned(),
            caret: 0,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        });
        focus
    };
    let blank = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(blank, Box::new(MockSpeechOutput::new()));
    let screen = "Console echo fixture\nCommand: \n\n\n";
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(console(screen)));
    let typed = "Console echo fixture\nCommand: a\n\n\n";
    let said = engine.handle_accessibility_event(&AccessibilityEvent::ConsoleText(typed.into()));
    assert_eq!(said, None);
    let output = "Console echo fixture\nCommand: a\nOutput line 1 for a\nSecond output line 1\nCommand: \n\n";
    let said = engine.handle_accessibility_event(&AccessibilityEvent::ConsoleText(output.into()));
    assert_eq!(
        said.as_deref(),
        Some("Output line 1 for a\nSecond output line 1\nCommand:")
    );

    engine.settings.screen_echo = crate::settings::ScreenEcho::None;
    let more = format!("{output}b\nOutput line 2 for b\nCommand: ");
    let more = more.replace("Command: \n\nb", "Command: b");
    let said = engine.handle_accessibility_event(&AccessibilityEvent::ConsoleText(more));
    assert_eq!(said, None);
}

/// The ribbon is read as menus are: its tab entered from the document says "Upper Ribbon"
/// first, and the document taking the focus back says "Leaving menus" [obs, 971/s1, s2].
#[test]
fn a_ribbon_is_entered_and_left_as_menus_are() {
    let blank = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(blank, Box::new(MockSpeechOutput::new()));
    let mut document = FocusEvent::new(Role::Edit).with_name("Notes");
    document.window_title = Some("Notes - Editor".to_owned());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(document.clone()));
    let mut tab = FocusEvent::new(Role::Tab).with_name("Home");
    tab.window_title = document.window_title.clone();
    tab.in_ribbon = true;
    tab.ribbon_menu = true;
    tab.states.selected = Some(true);
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(tab))
        .unwrap_or_default();
    assert_eq!(
        said.lines().next(),
        Some(crate::linearize::role_marker("Upper Ribbon").as_str())
    );
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(document.clone()))
        .unwrap_or_default();
    assert!(said.starts_with(&msg::menus_left().to_string()), "{said}");

    // From its tabs to another of its buttons: "Leaving menus | Leaving ribbons", then the
    // document says nothing of them [obs, 523/s4, s6].
    let mut tab = FocusEvent::new(Role::Tab).with_name("View");
    tab.window_title = document.window_title.clone();
    tab.in_ribbon = true;
    tab.ribbon_menu = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(tab));
    let mut minimize = FocusEvent::new(Role::Button).with_name("Minimize the Ribbon");
    minimize.window_title = document.window_title.clone();
    minimize.in_ribbon = true;
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(minimize))
        .unwrap_or_default();
    let leaving = format!(
        "{}
{}
",
        msg::menus_left(),
        msg::ribbons_left()
    );
    assert!(said.starts_with(&leaving), "{said}");
    let said = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(document))
        .unwrap_or_default();
    assert!(!said.contains(&msg::menus_left().to_string()), "{said}");
}

/// A Win32 edit reports a selection by its ends, the start for the caret; the caret is the
/// end that moved: after Shift+Right three times from the fourth character, the seventh, and
/// back past where it began, the start [obs, 975/s14, s21].
#[test]
fn a_selections_caret_is_the_end_that_moved() {
    let blank = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(blank, Box::new(MockSpeechOutput::new()));
    let read = |caret: usize, selection: Option<(usize, usize)>| {
        AccessibilityEvent::TextContext(crate::TextContext {
            text: "fn main() {".to_owned(),
            caret,
            selection,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: None,
        })
    };
    let caret = |engine: &ScreenReaderEngine| engine.last_text.as_ref().map(|t| t.caret);
    engine.handle_accessibility_event(&read(3, None));
    engine.handle_accessibility_event(&read(3, Some((3, 4))));
    assert_eq!(caret(&engine), Some(4));
    engine.handle_accessibility_event(&read(3, Some((3, 6))));
    engine.handle_accessibility_event(&read(3, Some((3, 6))));
    assert_eq!(caret(&engine), Some(6));
    engine.handle_accessibility_event(&read(3, None));
    engine.handle_accessibility_event(&read(2, Some((2, 3))));
    engine.handle_accessibility_event(&read(1, Some((1, 3))));
    assert_eq!(caret(&engine), Some(1));
}

/// A focused document of Word's, its caret on `line` of `text` with `structure`.
fn word_document(
    text: &str,
    caret: usize,
    structure: crate::event::LineStructure,
) -> (ScreenReaderEngine, SharedSpeech) {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(speech.clone()));
    let mut document = FocusEvent::new(Role::Document)
        .with_name("sr-word-fixture.docx")
        .with_window_title("sr-word-fixture.docx - Word");
    document.app = Some("WINWORD.EXE".into());
    document.text = Some(crate::event::TextContext {
        text: text.to_owned(),
        caret,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: Some(structure),
    });
    engine.handle_focus_event(&document);
    speech.take();
    (engine, speech)
}

/// A dialog's page entered from outside any (Settings Center's pane, by F6 from its tree) is
/// named first: "User Page | Typing Echo Off radio button checked"; a control of the page that
/// reports none (Notepad's Print page has an untitled inner dialog) does not forget it [obs,
/// the walk of Settings Center's pane; 546/s6].
#[test]
fn a_page_entered_from_outside_it_is_named() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let title = "Notepad - Okur Settings Center";
    let mut tree = FocusEvent::new(Role::TreeItem)
        .with_name("Typing Echo Off")
        .with_window_title(title);
    tree.tree_level = Some(1);
    engine.handle_focus_event(&tree);
    let mut echo = FocusEvent::new(Role::RadioButton)
        .with_name("Off")
        .with_window_title(title);
    echo.dialog_page = Some("User".into());
    echo.states.checked = Some(true);
    let said = engine.handle_focus_event(&echo).unwrap_or_default();
    assert!(said.starts_with("User Page\n"), "{said}");
    // A control of the page without its name, then the page's again: not named again.
    let mut preferences = FocusEvent::new(Role::Button)
        .with_name("Preferences")
        .with_window_title(title);
    preferences.dialog_page = None;
    engine.handle_focus_event(&preferences);
    let mut screen = FocusEvent::new(Role::RadioButton)
        .with_name("Echo Highlighted Text")
        .with_window_title(title);
    screen.dialog_page = Some("User".into());
    let said = engine.handle_focus_event(&screen).unwrap_or_default();
    assert!(!said.contains("User Page"), "{said}");
}

/// An edit combo's own list dropping down (Word's Font box, Alt+Down) is no menu: Down stays
/// the field's key and the field is read for the text it puts there [obs, 1066/s5-s8].
#[test]
fn an_edit_combos_list_keeps_the_keys_on_the_field() {
    use crate::event::MenuEvent;
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut font = FocusEvent::new(Role::Edit)
        .with_name("Font:")
        .with_window_title("sr-word-fixture.docx - Word");
    font.app = Some("WINWORD.EXE".into());
    font.container_role = Some(Role::ComboBox);
    font.text = Some(crate::event::TextContext {
        text: "Calibri".into(),
        caret: 0,
        selection: Some((0, 7)),
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&font);
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(MenuEvent::PopupStart));
    let before = actions.requests().len();
    engine.handle_key_event(&key(Key::DownArrow));
    assert!(
        actions.requests().len() > before,
        "{:?}",
        actions.requests()
    );
    // The list closing leaves no menu behind.
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(MenuEvent::PopupEnd));
    assert!(!engine.menus.in_menu_mode());
}

/// Ctrl+Shift+E in Word's document asks whether Track Changes is on once Word has acted on it,
/// and says so: "Track Changes on", "Track Changes off"; nothing when it could not be read
/// [obs, 1068/s1, s8].
#[test]
fn words_track_changes_key_says_its_state() {
    let (mut engine, speech) = word_document(
        "Library Hours\r",
        0,
        crate::event::LineStructure {
            line: (0, 14),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut key = KeyEvent::new(Key::Alpha('E'), Modifiers::none(), true);
    key.modifiers.ctrl = true;
    key.modifiers.shift = true;
    engine.handle_key_event(&key);
    assert!(actions
        .requests()
        .iter()
        .any(|(request, _)| request == "track changes"));
    for (state, said) in [
        (Some(true), Some("Track Changes on")),
        (Some(false), Some("Track Changes off")),
        (None, None),
    ] {
        let state = state.map(crate::event::WordKeyState::TrackChanges);
        assert_eq!(
            engine
                .handle_accessibility_event(&AccessibilityEvent::WordKeyState(state))
                .as_deref(),
            said
        );
    }
    assert_eq!(speech.take().len(), 2);
}

/// Ctrl+E, R, J and L in Word's document ask for the paragraph's alignment once Word has acted
/// on them, and say it as JAWS does: "Centered", "aligned right", "justified", "Aligned left";
/// Ctrl+Shift+Period and Comma the font size, "16 point"; the line spacing keys say nothing
/// [obs, 1071/s2-s8, 1074/s3-s5].
#[test]
fn words_alignment_and_font_size_keys_say_what_they_leave() {
    let (mut engine, _speech) = word_document(
        "Library Hours\r",
        0,
        crate::event::LineStructure {
            line: (0, 14),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ctrl = |letter: char| {
        let mut key = KeyEvent::new(Key::Alpha(letter), Modifiers::none(), true);
        key.modifiers.ctrl = true;
        key
    };
    engine.handle_key_event(&ctrl('E'));
    let mut double = KeyEvent::new(Key::Digit(2), Modifiers::none(), true);
    double.modifiers.ctrl = true;
    engine.handle_key_event(&double);
    let mut grow = KeyEvent::new(Key::Period, Modifiers::none(), true);
    grow.modifiers.ctrl = true;
    grow.modifiers.shift = true;
    engine.handle_key_event(&grow);
    let asked: Vec<String> = actions
        .requests()
        .into_iter()
        .map(|(request, _)| request)
        .filter(|request| request == "paragraph alignment" || request == "font size")
        .collect();
    assert_eq!(asked, ["paragraph alignment", "font size"]);
    use crate::event::WordKeyState;
    for (state, said) in [
        (Some(WordKeyState::Alignment(1)), Some("Centered")),
        (Some(WordKeyState::Alignment(2)), Some("aligned right")),
        (Some(WordKeyState::Alignment(3)), Some("justified")),
        (Some(WordKeyState::Alignment(0)), Some("Aligned left")),
        (Some(WordKeyState::Alignment(9)), None),
        (Some(WordKeyState::FontSize(160)), Some("16 point")),
        (Some(WordKeyState::FontSize(105)), Some("10.5 point")),
        (None, None),
    ] {
        assert_eq!(
            engine
                .handle_accessibility_event(&AccessibilityEvent::WordKeyState(state))
                .as_deref(),
            said
        );
    }
}

/// A drop-down form field of Word's: Alt+Down says "Open List Box" first; once its list
/// closes, the menus are left without the document's name and the field is said with its new
/// choice and the combo box's hint [obs, 1057/s7, s9].
#[test]
fn a_word_drop_down_fields_list_is_opened_and_closed() {
    use crate::event::{DocumentFormField, FormFieldKind, MenuEvent};
    let text = "Branch: \rThank you.";
    let branch = |value: &str, chosen: usize| DocumentFormField {
        kind: FormFieldKind::DropDown,
        name: "Branch".into(),
        start: 8,
        end: 8,
        value: value.into(),
        checked: None,
        choice: Some((chosen, 3)),
    };
    let structure = |value: &str, chosen: usize| crate::event::LineStructure {
        line: (0, 9),
        form_fields: vec![branch(value, chosen)],
        ..Default::default()
    };
    let (mut engine, speech) = word_document(text, 8, structure("Central", 1));
    let mut alt_down = KeyEvent::new(Key::DownArrow, Modifiers::none(), true);
    alt_down.modifiers.alt = true;
    engine.handle_key_event(&alt_down);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("Open List Box")
    );
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(MenuEvent::Start {
        window_class: None,
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(MenuEvent::PopupStart));
    let mut item = FocusEvent::new(Role::MenuItem).with_name("Northside");
    item.app = Some("WINWORD.EXE".into());
    item.platform.window_class = Some("#32768".into());
    engine.handle_focus_event(&item);
    speech.take();
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(MenuEvent::PopupEnd));
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(MenuEvent::End));
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("Leaving menus")
    );
    let said = engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
        crate::event::TextContext {
            text: text.to_owned(),
            caret: 8,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: Some(structure("Northside", 2)),
        },
    ));
    assert_eq!(
        said.as_deref(),
        Some("Branch:  \u{1d} Combo box \u{1e} \nNorthside\n2 of 3\nTo change the selection use the Arrow keys.")
    );
}

/// Insert+F6 in Word lists the document's headings, which the platform reads, in the Heading
/// List a page has ("Library Hours : 1" [obs, 519/s2]); Move To Heading moves the caret by
/// the heading's paragraph, and Escape gives the document back as "Edit" [obs, 519/s4].
#[test]
fn words_heading_list_lists_the_documents_headings() {
    use crate::ui::{ControlKind, FromUi, ToUi};
    let (mut engine, _speech) = word_document(
        "Library Hours\rThe library opens.\rWhat changes\r",
        0,
        crate::event::LineStructure {
            line: (0, 14),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut f6 = KeyEvent::new(Key::F(6), Modifiers::none(), true);
    f6.modifiers.insert = true;
    engine.handle_key_event(&f6);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document headings"));
    let heading = |text: &str, level, paragraph| crate::event::DocumentHeading {
        text: text.to_owned(),
        level,
        paragraph,
    };
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentHeadings(vec![
        heading("Library Hours", 1, 0),
        heading("What changes", 2, 2),
    ]));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Heading List");
    assert!(dialog.controls.iter().any(|c| c.kind
        == ControlKind::List {
            items: vec!["Library Hours : 1".into(), "What changes : 2".into()],
            selected: 0
        }));
    let id = dialog.id;
    // Escape: the document comes back, named.
    let closed =
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id }));
    assert_eq!(closed.as_deref().map(str::trim), Some("\u{1d} Edit \u{1e}"));
    assert_eq!(engine.app_mode, AppMode::NativeControl);
}

/// Insert+F7 in Word lists the document's links, which the platform reads, in the Links List
/// a page has ("Links List dialog | links List view | opening hours | 1 of 2" [obs, 1049/s1]);
/// Move To Link moves the caret to the link's paragraph, Activate Link has the document follow
/// it; with none, Word's words for it [obs, 1044/s1].
#[test]
fn words_links_list_lists_the_documents_links() {
    use crate::ui::{ControlKind, FromUi, ToUi};
    let (mut engine, speech) = word_document(
        "Library Card\rSee the opening hours before you come.\r",
        0,
        crate::event::LineStructure {
            line: (0, 13),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut f7 = KeyEvent::new(Key::F(7), Modifiers::none(), true);
    f7.modifiers.insert = true;
    engine.handle_key_event(&f7);
    // Opened again without a second Insert+F7, which would be a double press.
    let links_list = |engine: &mut ScreenReaderEngine| {
        engine.open_list(crate::ui::lists::ListKind::Links);
    };
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document links"));
    let link = |text: &str, paragraph| crate::event::DocumentLink {
        text: text.to_owned(),
        url: Some(format!("https://example.org/{text}")),
        paragraph,
    };
    let links = vec![link("opening hours", 1), link("branches", 4)];
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLinks(links.clone()));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Links List");
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 } if items == &["opening hours", "branches"]
    )));
    // Move To Link on the second: the caret to its paragraph.
    let id = dialog.id;
    let list = dialog
        .controls
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: list,
        value: crate::ui::Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "move".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "paragraph 4"));
    // Activate Link on the first: the document follows it.
    links_list(&mut engine);
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLinks(links));
    let shown = ui.take().into_iter().find_map(|m| match m {
        ToUi::DialogShow { dialog } => Some(dialog),
        _ => None,
    });
    let Some(dialog) = shown else {
        panic_free_fail("no second dialog shown");
        return;
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "activate".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "activate document link 0"));
    // None: Word's words for it.
    links_list(&mut engine);
    speech.take();
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLinks(Vec::new()));
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("no links\nCheck for link type fields with Insert+F5.")
    );
}

/// Insert+F5 in Word lists the document's fields, which the platform reads, by their code's
/// first word and the text they show: "List Fields dialog | List1 List view | HYPERLINK opening
/// hours | 1 of 4" [obs, 1050/s1]; OK moves the caret to the field [obs, 1054/s3]; with none,
/// Word's words for it [obs, 1047/s1].
#[test]
fn words_fields_list_lists_the_documents_fields() {
    use crate::ui::{ControlKind, FromUi, ToUi};
    let (mut engine, speech) = word_document(
        "Library Card\rSee the opening hours before you come.\r",
        0,
        crate::event::LineStructure {
            line: (0, 13),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut f5 = KeyEvent::new(Key::F(5), Modifiers::none(), true);
    f5.modifiers.insert = true;
    engine.handle_key_event(&f5);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document fields"));
    let field = |code: &str, result: &str| crate::event::DocumentField {
        code: code.to_owned(),
        result: result.to_owned(),
    };
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentFields(vec![
        field(" HYPERLINK \"https://example.org/hours\" ", "opening hours"),
        field(" FORMTEXT ", "\u{2002}\u{2002}\u{2002}\u{2002}\u{2002}"),
        field(" FORMCHECKBOX ", ""),
    ]));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "List Fields");
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 }
            if items == &["HYPERLINK opening hours", "FORMTEXT", "FORMCHECKBOX"]
    )));
    // OK on the second: the caret to the field.
    let id = dialog.id;
    let list = dialog
        .controls
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: list,
        value: crate::ui::Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "field 1"));
    // None: Word's words for it.
    engine.open_list(crate::ui::lists::ListKind::FormFields);
    speech.take();
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentFields(Vec::new()));
    assert_eq!(speech.take().last().map(String::as_str), Some("no fields"));
}

/// Ctrl+Shift+Apostrophe and Insert+Shift+F in Word list the document's comments and footnotes,
/// which the platform reads: "Reviewers' Comments dialog | List1 List view | Check the summer
/// hours. Lab Fri, Jan 02, 26", "Footnotes dialog | List1 List view | 1 Holidays excepted."
/// [obs, 1082/s1, 1083/s1], then OK and Cancel [obs, 1087/s2-s5]. OK puts the caret on the
/// note's mark and says "Edit", after the footnote for a footnote [obs, 1087/s6, 1088/s6].
#[test]
fn words_notes_lists_list_comments_and_footnotes() {
    use crate::event::{DocumentNote, NoteKind};
    use crate::ui::{ControlKind, FromUi, ToUi};
    let text = "Library Notes\rThe library opens at nine in the morning.\rIt is open every day1.\r";
    let (mut engine, speech) = word_document(
        text,
        0,
        crate::event::LineStructure {
            line: (0, 14),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut key = KeyEvent::new(Key::Apostrophe, Modifiers::none(), true);
    key.modifiers.ctrl = true;
    key.modifiers.shift = true;
    assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document comments"));
    let comment = DocumentNote {
        text: "Check the summer hours.".into(),
        author: "Lab".into(),
        date: "Fri, Jan 02, 26".into(),
        ..Default::default()
    };
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentNotes(
        NoteKind::Comment,
        vec![comment],
    ));
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Reviewers' Comments");
    let labels: Vec<&str> = dialog.controls.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["List1", "OK", "Cancel"]);
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 }
            if items == &["Check the summer hours. Lab Fri, Jan 02, 26"]
    )));
    let edit = crate::linearize::role_marker("Edit");
    let moved = |engine: &mut ScreenReaderEngine, caret| {
        engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
            crate::event::TextContext {
                text: text.to_owned(),
                caret,
                selection: None,
                font: None,
                word: None,
                misspelled: Vec::new(),
                structure: Some(crate::event::LineStructure {
                    line: (14, 55),
                    ..Default::default()
                }),
            },
        ));
    };
    // OK: the caret to the comment's mark, and "Edit" alone.
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "ok".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "Comment 0"));
    speech.take();
    moved(&mut engine, 54);
    assert_eq!(speech.take().last(), Some(&edit));
    // Insert+Shift+F: the footnotes; OK says the footnote, then "Edit".
    let mut key = KeyEvent::new(Key::Alpha('F'), Modifiers::none(), true);
    key.modifiers.insert = true;
    key.modifiers.shift = true;
    engine.handle_key_event(&key);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document footnotes"));
    let footnote = DocumentNote {
        text: "Holidays excepted.".into(),
        number: Some(1),
        ..Default::default()
    };
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentNotes(
        NoteKind::Footnote,
        vec![footnote],
    ));
    let shown = ui.take().into_iter().find_map(|m| match m {
        ToUi::DialogShow { dialog } => Some(dialog),
        _ => None,
    });
    let Some(dialog) = shown else {
        panic_free_fail("no footnotes dialog shown");
        return;
    };
    assert_eq!(dialog.title, "Footnotes");
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 } if items == &["1 Holidays excepted."]
    )));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "ok".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "Footnote 0"));
    speech.take();
    moved(&mut engine, 76);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(format!("footnote reference: 1 Holidays excepted.\n{edit}").as_str())
    );
    // None: said so.
    engine.handle_key_event(&key);
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentNotes(
        NoteKind::Footnote,
        Vec::new(),
    ));
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("There are no footnotes in the document.")
    );
}

/// Insert+Alt+B in Word lists the document's bookmarks in its order, each by its name and the
/// text it marks: "Bookmark List dialog | List1 List view | Opening nine in the morning" [obs,
/// 1091/s1]; OK on one says its text, then "Edit" [obs, 1091/s9]. Insert+Shift+E lists the
/// endnotes, and OK on one says it by its mark: "end note reference: i Renewals are free."
/// [obs, 1090/s1, s6].
#[test]
fn words_bookmarks_and_endnotes_lists() {
    use crate::event::{DocumentNote, NoteKind};
    use crate::ui::{ControlKind, FromUi, ToUi};
    let text = "Library Marks\rThe library opens at nine in the morning.\rMembers borrow books for three weeks.\r";
    let (mut engine, speech) = word_document(
        text,
        0,
        crate::event::LineStructure {
            line: (0, 14),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let edit = crate::linearize::role_marker("Edit");
    let moved = |engine: &mut ScreenReaderEngine, caret| {
        engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
            crate::event::TextContext {
                text: text.to_owned(),
                caret,
                selection: None,
                font: None,
                word: None,
                misspelled: Vec::new(),
                structure: Some(crate::event::LineStructure {
                    line: (56, 94),
                    ..Default::default()
                }),
            },
        ));
    };
    let shown = |ui: &crate::ui::RecordingUiHost| {
        ui.take().into_iter().find_map(|m| match m {
            ToUi::DialogShow { dialog } => Some(dialog),
            _ => None,
        })
    };
    // Insert+Alt+B: the bookmarks.
    let mut key = KeyEvent::new(Key::Alpha('B'), Modifiers::none(), true);
    key.modifiers.insert = true;
    key.modifiers.alt = true;
    engine.handle_key_event(&key);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document bookmarks"));
    let bookmark = |name: &str, text: &str| DocumentNote {
        text: text.into(),
        name: name.into(),
        ..Default::default()
    };
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentNotes(
        NoteKind::Bookmark,
        vec![
            bookmark("Opening", "nine in the morning"),
            bookmark("Borrowing", "three weeks"),
        ],
    ));
    let Some(dialog) = shown(&ui) else {
        panic_free_fail("no bookmarks dialog shown");
        return;
    };
    assert_eq!(dialog.title, "Bookmark List");
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 }
            if items == &["Opening nine in the morning", "Borrowing three weeks"]
    )));
    let list = dialog
        .controls
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id: dialog.id,
        control: list,
        value: crate::ui::Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "ok".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "Bookmark 1"));
    speech.take();
    moved(&mut engine, 81);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(format!("three weeks\n{edit}").as_str())
    );
    // Insert+Shift+E: the endnotes, OK says the endnote by its mark.
    let mut key = KeyEvent::new(Key::Alpha('E'), Modifiers::none(), true);
    key.modifiers.insert = true;
    key.modifiers.shift = true;
    engine.handle_key_event(&key);
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentNotes(
        NoteKind::Endnote,
        vec![DocumentNote {
            text: "Renewals are free.".into(),
            number: Some(1),
            mark: "i".into(),
            ..Default::default()
        }],
    ));
    let Some(dialog) = shown(&ui) else {
        panic_free_fail("no endnotes dialog shown");
        return;
    };
    assert_eq!(dialog.title, "End Notes");
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 } if items == &["1 Renewals are free."]
    )));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "ok".into(),
    }));
    speech.take();
    moved(&mut engine, 92);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(format!("end note reference: i Renewals are free.\n{edit}").as_str())
    );
}

/// Windows+Semicolon in Word opens Items to Display in Virtual Viewer: "Comments", "Footnotes",
/// "Endnotes", "Revisions" [obs, 1098/s1-s4]. OK on Footnotes says what is gathered, then the
/// viewer's first line and "Edit"; Down reads its lines, a footnote after its link [obs,
/// 1097/s3-s6].
#[test]
fn windows_semicolon_shows_words_notes_in_the_viewer() {
    use crate::event::{DocumentNote, NoteKind};
    use crate::ui::{ControlKind, FromUi, ToUi};
    let (mut engine, speech) = word_document(
        "Library Notes\rIt is open every day.\r",
        0,
        crate::event::LineStructure {
            line: (0, 14),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut key = KeyEvent::new(Key::Semicolon, Modifiers::none(), true);
    key.modifiers.win = true;
    assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::DialogShow { dialog }) = sent.first() else {
        panic_free_fail(&format!("no dialog shown: {sent:?}"));
        return;
    };
    assert_eq!(dialog.title, "Items to Display in Virtual Viewer");
    assert!(dialog.controls.iter().any(|c| matches!(
        &c.kind,
        ControlKind::List { items, selected: 0 }
            if items == &["Comments", "Footnotes", "Endnotes", "Revisions"]
    )));
    let list = dialog
        .controls
        .first()
        .map(|c| c.id.clone())
        .unwrap_or_default();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id: dialog.id,
        control: list,
        value: crate::ui::Value::Selected(1),
    }));
    speech.take();
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id: dialog.id,
        control: "ok".into(),
    }));
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document footnotes"));
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("Please wait: Processing Footnotes")
    );
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentNotes(
        NoteKind::Footnote,
        vec![DocumentNote {
            text: "Holidays excepted.".into(),
            number: Some(1),
            ..Default::default()
        }],
    ));
    let edit = crate::linearize::role_marker("Edit");
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(format!("There is only one Footnote to display.\n{edit}").as_str())
    );
    let down = |engine: &mut ScreenReaderEngine| {
        engine.handle_key_event(&KeyEvent::new(Key::DownArrow, Modifiers::none(), true));
    };
    down(&mut engine);
    down(&mut engine);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("\u{1d} Link \u{1e} 1")
    );
    down(&mut engine);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("1 Holidays excepted.")
    );
}

/// Alt+Shift+Apostrophe pressed twice quickly in a comment of Word's shows it in the Virtual
/// Viewer, read whole: the comment, its author, the text it is on and how to close it; Escape
/// reads the document's line again, then "Edit" [obs, 1095/s6, s7, s10].
#[test]
fn alt_shift_apostrophe_twice_shows_the_comment() {
    let text = "Library Notes\rThe library opens at nine in the morning.\r";
    let (mut engine, speech) = word_document(
        text,
        53,
        crate::event::LineStructure {
            line: (14, 56),
            comments: vec![crate::event::DocumentComment {
                start: 35,
                end: 54,
                text: "Check the summer hours.".into(),
                author: "Lab".into(),
            }],
            ..Default::default()
        },
    );
    let press = |engine: &mut ScreenReaderEngine, time_ms| {
        let mut key = KeyEvent::new(Key::Apostrophe, Modifiers::none(), true);
        key.modifiers.alt = true;
        key.modifiers.shift = true;
        key.time_ms = time_ms;
        engine.handle_key_event(&key);
    };
    press(&mut engine, 1000);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("Check the summer hours.\nby Lab")
    );
    press(&mut engine, 1150);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(
            "Check the summer hours.\nby Lab\nDocument text:\nnine in the morning\nPress ESCAPE to close this message."
        )
    );
    // Down stays on the last line, read again.
    engine.handle_key_event(&KeyEvent::new(Key::DownArrow, Modifiers::none(), true));
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("Press ESCAPE to close this message.")
    );
    engine.handle_key_event(&KeyEvent::new(Key::Escape, Modifiers::none(), true));
    let edit = crate::linearize::role_marker("Edit");
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(
            format!("The library opens at  \u{1d}has comment \u{1e} nine in the morning .\n{edit}")
                .as_str()
        )
    );
}

/// Alt+Shift+E in Word reads the footnote or endnote whose reference the caret is on: "end note
/// reference: i Renewals are free."; elsewhere "No footnote or EndNote reference at cursor."
/// [obs, 1092/s1, s7]. Left onto the reference says it: " end note i" [obs, 1092/s6].
#[test]
fn alt_shift_e_reads_the_note_at_the_caret() {
    let text = "Library Marks\rMembers borrow books for three weeks.\r";
    let structure = |at_caret: bool| crate::event::LineStructure {
        line: (14, 52),
        endnotes: vec![crate::event::DocumentFootnote {
            at: 50,
            number: 1,
            mark: "i".into(),
            text: "Renewals are free.".into(),
            at_caret,
        }],
        ..Default::default()
    };
    let (mut engine, speech) = word_document(text, 14, structure(false));
    let mut key = KeyEvent::new(Key::Alpha('E'), Modifiers::none(), true);
    key.modifiers.alt = true;
    key.modifiers.shift = true;
    assert_eq!(engine.handle_key_event(&key), KeyAction::Consume);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("No footnote or EndNote reference at cursor.")
    );
    let context = |caret, at_caret| crate::event::TextContext {
        text: text.to_owned(),
        caret,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: Some(structure(at_caret)),
    };
    // Left from before the period onto the reference, which the text does not show.
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(50, false)));
    engine.handle_key_event(&KeyEvent::new(Key::LeftArrow, Modifiers::none(), true));
    speech.take();
    engine.handle_accessibility_event(&AccessibilityEvent::TextContext(context(50, true)));
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some(" \u{1d}end note \u{1e} i")
    );
    engine.handle_key_event(&key);
    assert_eq!(
        speech.take().last().map(String::as_str),
        Some("end note reference: i Renewals are free.")
    );
}

/// Ctrl+Shift+Apostrophe outside Word's document is the application's.
#[test]
fn ctrl_shift_apostrophe_is_the_applications_outside_word() {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(SharedSpeech::default()));
    let mut editor = FocusEvent::new(Role::Document)
        .with_name("Text Editor")
        .with_window_title("notes.txt - Notepad");
    editor.app = Some("notepad.exe".into());
    editor.text = Some(crate::event::TextContext {
        text: "Plain text".to_owned(),
        caret: 0,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
        structure: None,
    });
    engine.handle_focus_event(&editor);
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut key = KeyEvent::new(Key::Apostrophe, Modifiers::none(), true);
    key.modifiers.ctrl = true;
    key.modifiers.shift = true;
    assert_eq!(engine.handle_key_event(&key), KeyAction::PassThrough);
    assert!(!actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document comments"));
}

/// Ctrl+Alt+Right in a table of Word's moves the caret to the next cell through the platform;
/// at the row's end it says so.
#[test]
fn words_table_keys_move_to_the_next_cell() {
    let cell = |row, column| crate::event::TableCell {
        table: 1,
        row,
        column,
        rows: 2,
        columns: 2,
        column_header: ["Branch", "Hours"].get(column - 1).map(|t| t.to_string()),
        row_header: ["Branch", "Central"].get(row - 1).map(|t| t.to_string()),
    };
    let (mut engine, speech) = word_document(
        "Central\r\u{7}9 to 9\r\u{7}",
        0,
        crate::event::LineStructure {
            line: (0, 7),
            cell: Some(cell(2, 1)),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    let mut right = KeyEvent::new(Key::RightArrow, Modifiers::none(), true);
    right.modifiers.ctrl = true;
    right.modifiers.alt = true;
    assert_eq!(engine.handle_key_event(&right), KeyAction::Consume);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "cell 2 2"));
    let said = engine.handle_accessibility_event(&AccessibilityEvent::TextContext(
        crate::event::TextContext {
            text: "Central\r\u{7}9 to 9\r\u{7}".to_owned(),
            caret: 9,
            selection: None,
            font: None,
            word: None,
            misspelled: Vec::new(),
            structure: Some(crate::event::LineStructure {
                line: (9, 16),
                cell: Some(cell(2, 2)),
                ..Default::default()
            }),
        },
    ));
    assert_eq!(
        said.as_deref(),
        Some("Hours\n9 to 9\nlast cell in table\nWarning! Pressing tab here adds an extra row.")
    );
    speech.take();
    // At the row's end: the edge, and the caret stays [obs, 1035/s8].
    assert_eq!(engine.handle_key_event(&right), KeyAction::Consume);
    assert_eq!(speech.take(), ["end of row"]);
}

/// Say Window Title in Word adds the view the document is shown in, which the platform reads
/// from the status bar: "Title is | sr-word-fixture.docx - Word | Print View" [obs, 519/s1].
#[test]
fn words_title_says_the_view() {
    let (mut engine, _speech) = word_document(
        "Library Hours\r",
        0,
        crate::event::LineStructure {
            line: (0, 14),
            heading: Some(1),
            ..Default::default()
        },
    );
    let actions = crate::actions::RecordingActions::default();
    engine.set_actions(Box::new(actions.clone()));
    assert_eq!(engine.execute_command("SayWindowTitle"), None);
    assert!(actions
        .requests()
        .iter()
        .any(|(action, _)| action == "document view"));
    let said = engine.handle_accessibility_event(&AccessibilityEvent::DocumentView(Some(
        "Print Layout".to_owned(),
    )));
    assert_eq!(
        said.as_deref(),
        Some("Title is\nsr-word-fixture.docx - Word\nPrint View")
    );
}

/// Quick Settings' lifetimes: a change to last until Okur exits goes to the session's layer
/// and is not saved; one to last until the focus moves is cleared when another application
/// takes the focus, not when Okur's own window does.
#[test]
fn quick_settings_changes_last_as_their_lifetimes() {
    use crate::settings::TypingEcho;
    use crate::settings_layers::Scope;
    use crate::ui::settings::Lifetime;
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let store = crate::settings::RecordingSettingsStore::default();
    engine.set_settings_store(Box::new(store.clone()));
    engine.settings_place(Some("notepad.exe"), None);
    engine.settings_dialog_application = Some("notepad".to_owned());
    let before = engine.settings.clone();
    let mut after = before.clone();
    after.typing_echo = TypingEcho::Words;
    after.say_all_unit = crate::settings::SayAllUnit::Sentence;
    engine.apply_settings(
        &Scope::Application("notepad".to_owned()),
        &before,
        &after,
        &[
            ("typing_echo", Lifetime::UntilExit),
            ("say_all_unit", Lifetime::UntilFocusMoves),
        ],
    );
    assert_eq!(engine.settings.typing_echo, TypingEcho::Words);
    assert_eq!(
        engine.settings.say_all_unit,
        crate::settings::SayAllUnit::Sentence
    );
    // Neither is kept for Notepad.
    let saved = store.take();
    assert!(saved.iter().all(|layers| layers
        .applications
        .get("notepad")
        .is_none_or(|v| v.is_empty())));
    // Okur's own window takes the focus and gives it back: both stay.
    engine.settings_place(Some("okur-ui.exe"), None);
    engine.settings_place(Some("notepad.exe"), None);
    assert_eq!(
        engine.settings.say_all_unit,
        crate::settings::SayAllUnit::Sentence
    );
    // Another application: the until-the-focus-moves change ends, the session's stays.
    engine.settings_place(Some("chrome.exe"), None);
    assert_eq!(
        engine.settings.say_all_unit,
        crate::settings::SayAllUnit::Line
    );
    assert_eq!(engine.settings.typing_echo, TypingEcho::Words);
}

/// Language's Voice Profiles, as JAWS's: Default with the profile in use checked, For the
/// application in front with Use Default checked, No Speech, Create New...; a profile chosen
/// there is the voice, No Speech silences the synthesizer until one is chosen [obs, the walk of
/// Language's Voice Profiles].
#[test]
fn the_language_menu_chooses_voice_profiles() {
    use crate::settings::Synthesizer;
    use crate::ui::{FromUi, MenuItemSpec, ToUi};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    engine.settings_place(Some("notepad.exe"), None);
    engine.execute_command("JAWSWindow");
    let sent = ui.take();
    let Some(ToUi::MainWindowShow { window }) = sent.first() else {
        panic_free_fail(&format!("no window shown: {sent:?}"));
        return;
    };
    let language = window.menus.iter().find(|m| m.label == "&Language");
    let profiles = language.and_then(|m| m.items.first());
    let labels = |item: Option<&MenuItemSpec>| -> Vec<String> {
        item.map(|i| {
            i.items
                .iter()
                .map(|i| format!("{}{}", i.label, if i.checked { " checked" } else { "" }))
                .collect()
        })
        .unwrap_or_default()
    };
    assert_eq!(profiles.map(|p| p.label.as_str()), Some("Voice Profiles"));
    assert_eq!(
        labels(profiles),
        ["Default", "For notepad", "No Speech", "Create New..."]
    );
    let default = profiles.and_then(|p| p.items.first());
    assert!(labels(default).contains(&"Automatic - (factory) checked".to_owned()));
    let for_notepad = profiles.and_then(|p| p.items.get(1));
    assert_eq!(
        labels(for_notepad).first().map(String::as_str),
        Some("Use Default checked")
    );
    let command = |engine: &mut ScreenReaderEngine, id: &str| {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::MainMenuCommand {
            id: id.into(),
        }));
    };
    command(&mut engine, "profile_default:OneCore - (factory)");
    assert_eq!(engine.settings.voice.synthesizer, Synthesizer::OneCore);
    command(&mut engine, "no_speech");
    assert_eq!(engine.settings.voice.synthesizer, Synthesizer::NoSpeech);
    command(&mut engine, "profile_application:SAPI 5 - (factory)");
    assert_eq!(engine.settings.voice.synthesizer, Synthesizer::Sapi5);
    assert!(engine.settings_layers.session.is_empty());
}

/// Insert+J shows Okur's window with JAWS's menus, of their items those Okur has; Options,
/// Basics... opens "Basic Settings", and Utilities, Settings Center opens Settings Center.
#[test]
fn insert_j_shows_okurs_window_and_its_menus() {
    use crate::ui::{FromUi, ToUi};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let mut j = KeyEvent::new(Key::Alpha('J'), Modifiers::none(), true);
    j.modifiers.insert = true;
    assert_eq!(engine.handle_key_event(&j), KeyAction::Consume);
    let sent = ui.take();
    let Some(ToUi::MainWindowShow { window }) = sent.first() else {
        panic_free_fail(&format!("no window shown: {sent:?}"));
        return;
    };
    assert_eq!(window.title, "Okur");
    let menus: Vec<&str> = window.menus.iter().map(|m| m.label.as_str()).collect();
    assert_eq!(menus, ["&Options", "&Utilities", "&Language", "&Help"]);
    let options: Vec<&str> = window
        .menus
        .first()
        .map(|m| m.items.iter().map(|i| i.label.as_str()).collect())
        .unwrap_or_default();
    assert_eq!(
        options,
        [
            "&Basics...",
            "&Voices",
            "Manage Application Settings...",
            "Restore to Factory Settings...",
            "E&xit"
        ]
    );
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::MainMenuCommand {
        id: "basics".into(),
    }));
    assert!(matches!(
        ui.take().first(),
        Some(ToUi::DialogShow { dialog }) if dialog.title == "Basic Settings"
    ));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::MainMenuCommand {
        id: "settings_center".into(),
    }));
    assert!(matches!(
        ui.take().first(),
        Some(ToUi::DialogShow { dialog }) if dialog.title.ends_with("Okur Settings Center")
    ));
}

/// The Startup Wizard from Help: a change is in force at once and kept nowhere; Escape gives
/// the settings back, Finish keeps them for all applications (JAWS's help).
#[test]
fn the_startup_wizard_tries_then_keeps_its_changes() {
    use crate::settings::TypingEcho;
    use crate::ui::{FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let store = crate::settings::RecordingSettingsStore::default();
    engine.set_settings_store(Box::new(store.clone()));
    let echo = engine.settings.typing_echo;
    let open = |engine: &mut ScreenReaderEngine| {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::MainMenuCommand {
            id: "startup_wizard".into(),
        }));
        match ui.take().first() {
            Some(ToUi::WizardShow { wizard }) => {
                assert_eq!(wizard.page.title, "Speech Settings - Okur Startup Wizard");
                wizard.id
            }
            other => {
                panic_free_fail(&format!("no wizard: {other:?}"));
                0
            }
        }
    };
    let words = |id| {
        AccessibilityEvent::Ui(FromUi::DialogChanged {
            id,
            control: "typing_echo".into(),
            value: Value::Selected(2),
        })
    };
    let id = open(&mut engine);
    engine.handle_accessibility_event(&words(id));
    assert_eq!(engine.settings.typing_echo, TypingEcho::Words);
    assert!(store.take().is_empty());
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id }));
    assert_eq!(engine.settings.typing_echo, echo);
    assert!(engine.settings_layers.session.is_empty());
    // Again, then Finish.
    let id = open(&mut engine);
    engine.handle_accessibility_event(&words(id));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "finish".into(),
    }));
    assert_eq!(engine.settings.typing_echo, TypingEcho::Words);
    assert!(engine.settings_layers.session.is_empty());
    assert_eq!(
        store
            .take()
            .last()
            .map(|layers| layers.resolve(None, None).typing_echo),
        Some(TypingEcho::Words)
    );
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
}

/// The user's start after sign-in, kept apart from the settings.
#[derive(Clone, Default)]
struct KeptStart(std::sync::Arc<std::sync::Mutex<Option<crate::settings::UserStart>>>);

impl crate::settings::StartOptions for KeptStart {
    fn user_start(&self) -> Option<crate::settings::UserStart> {
        self.0.lock().ok().and_then(|s| *s)
    }
    fn set_user_start(&self, start: crate::settings::UserStart) {
        if let Ok(mut kept) = self.0.lock() {
            *kept = Some(start);
        }
    }
}

/// The Startup Wizard's Run Okur Settings offers this user's start after sign-in, JAWS's
/// Always when they chose nothing; Finish keeps the choice where the sign-in start reads it.
#[test]
fn the_startup_wizard_keeps_the_start_after_sign_in() {
    use crate::settings::UserStart;
    use crate::ui::{ControlKind, FromUi, ToUi, Value};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let ui = crate::ui::RecordingUiHost::default();
    engine.set_ui(Box::new(ui.clone()));
    let kept = KeptStart::default();
    engine.set_start_options(Box::new(kept.clone()));
    engine.open_startup_wizard();
    let id = match ui.take().first() {
        Some(ToUi::WizardShow { wizard }) => wizard.id,
        _ => 0,
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "next".into(),
    }));
    let start = ui.take().into_iter().find_map(|m| match m {
        ToUi::WizardPage { page, .. } => page.controls.into_iter().next(),
        _ => None,
    });
    assert!(
        matches!(
            start.as_ref().map(|c| &c.kind),
            Some(ControlKind::Choice { selected: 0, .. })
        ),
        "{start:?}"
    );
    for message in [
        FromUi::DialogChanged {
            id,
            control: "start_after_sign_in".into(),
            value: Value::Selected(1),
        },
        FromUi::DialogPressed {
            id,
            control: "finish".into(),
        },
    ] {
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(message));
    }
    assert_eq!(
        crate::settings::StartOptions::user_start(&kept),
        Some(UserStart::Never)
    );
}

/// Select a Voice Profile: a profile chosen for all applications becomes their voice, its name
/// kept; one chosen for Notepad is Notepad's own; "Use Default" gives Notepad the voice for all
/// applications again, and nothing of its own is left in its settings.
#[test]
fn voice_profiles_are_chosen_for_all_applications_and_one() {
    use crate::settings::{Synthesizer, VoiceSettings};
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    let mut layers = crate::settings_layers::SettingsLayers::default();
    let mut saved = crate::settings::Settings::default();
    saved.voice_profiles.insert(
        "Fast".to_owned(),
        VoiceSettings {
            rate: 6,
            ..VoiceSettings::default()
        },
    );
    layers.global = crate::settings_layers::changes(&saved, &crate::settings::Settings::default());
    engine.set_settings_layers(layers);
    engine.settings_place(Some("notepad.exe"), None);
    engine.choose_voice_profiles(Some("SAPI 5 - (factory)".to_owned()), None);
    let global = engine.settings_layers.resolve(None, None);
    assert_eq!(global.voice.synthesizer, Synthesizer::Sapi5);
    assert_eq!(global.voice.profile.as_deref(), Some("SAPI 5 - (factory)"));
    engine.choose_voice_profiles(None, Some(("notepad".to_owned(), Some("Fast".to_owned()))));
    assert_eq!(engine.settings.voice.rate, 6);
    assert_eq!(engine.settings.voice.profile.as_deref(), Some("Fast"));
    engine.choose_voice_profiles(None, Some(("notepad".to_owned(), None)));
    assert_eq!(engine.settings.voice.rate, 0);
    assert_eq!(engine.settings.voice.synthesizer, Synthesizer::Sapi5);
    assert!(engine
        .settings_layers
        .applications
        .get("notepad")
        .is_none_or(|values| !values.contains_key("voice")));
}
