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
    let lines = |engine: &ScreenReaderEngine| {
        engine
            .cursor
            .buffer
            .lines
            .iter()
            .map(|l| l.text.clone())
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
/// and Down and reads them without their indentation; Insert+Up reads its line, Left and the
/// word key say nothing, and Numpad Plus goes back to the caret's line [obs, 690].
#[test]
fn the_okur_cursor_reads_a_text_fields_lines() {
    let speech = SharedSpeech::default();
    let mut engine = ScreenReaderEngine::new(
        VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document))),
        Box::new(speech.clone()),
    );
    engine.set_settings(crate::settings::Settings::reference_machine());
    let mut field = FocusEvent::new(Role::Edit).with_name("Text Editor");
    field.text = Some(crate::event::TextContext {
        text: "fn main() {\r\n    let total = 1;\r\n    println!(\"{total}\");\r\n}".into(),
        caret: 0,
        selection: None,
        font: None,
        word: None,
        misspelled: Vec::new(),
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

/// Insert+6 and Insert+V open the settings dialog; OK applies what was changed, keeps it in
/// the settings store, and closes the dialog. Okur says nothing itself: it reads the dialog
/// when the dialog takes the focus.
#[test]
fn the_settings_dialog_applies_and_keeps_changes() {
    use crate::ui::{FromUi, ToUi, Value};
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
    assert_eq!(dialog.title, "Okur Settings");
    let id = dialog.id;
    // Insert+V brings the same dialog forward.
    assert_eq!(engine.execute_command("QuickSettings"), None);
    assert!(matches!(ui.take().first(), Some(ToUi::DialogShow { dialog }) if dialog.id == id));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "typing_echo".into(),
        value: Value::Selected(0),
    }));
    assert_eq!(
        engine.settings.typing_echo,
        crate::settings::TypingEcho::Characters
    );
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogPressed {
        id,
        control: "ok".into(),
    }));
    assert_eq!(
        engine.settings.typing_echo,
        crate::settings::TypingEcho::None
    );
    // Kept for all applications: only what the dialog changed.
    assert_eq!(
        store.take(),
        [crate::settings_layers::SettingsLayers::with_global(
            &engine.settings
        )]
    );
    assert_eq!(ui.take(), [ToUi::DialogClose { id }]);
    // Cancelled, a new dialog changes nothing.
    engine.execute_command("SettingsCenter");
    let id = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) => dialog.id,
        _ => 0,
    };
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogChanged {
        id,
        control: "typing_echo".into(),
        value: Value::Selected(1),
    }));
    engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id }));
    assert_eq!(
        engine.settings.typing_echo,
        crate::settings::TypingEcho::None
    );
    assert!(store.take().is_empty());
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
    assert_eq!(line, Some((3, "Fruit edit combo collapsed".to_owned())));
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
    let notification = |text: &str| {
        let mut live = LiveRegionEvent::new(text, Politeness::Polite);
        live.region_id = Some("uia-notification".to_owned());
        live.atomic = true;
        AccessibilityEvent::LiveRegion(live)
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

    // Escape: the dialog goes and JAWS names the page.
    engine.handle_key_event(&f5);
    let id = match ui.take().first() {
        Some(ToUi::DialogShow { dialog }) => dialog.id,
        other => {
            panic_free_fail(&format!("no dialog shown: {other:?}"));
            return;
        }
    };
    let closed =
        engine.handle_accessibility_event(&AccessibilityEvent::Ui(FromUi::DialogClosed { id }));
    assert_eq!(closed.as_deref(), Some("Web Menus \u{1d} document \u{1e} "));
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
        changed.auto_forms_mode = false;
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
    changed.auto_forms_mode = false;
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
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(crate::event::MenuEvent::Start));
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
    changed.auto_forms_mode = false;
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
    changed.auto_forms_mode = false;
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
    changed.auto_forms_mode = false;
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
    engine.handle_accessibility_event(&AccessibilityEvent::Menu(crate::event::MenuEvent::Start));
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
