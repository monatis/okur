use super::*;
use crate::aom::{AomNode, Role};
use crate::event::{LiveRegionEvent, Politeness};
use crate::input::Modifiers;
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
             \u{1d} Link \u{1e} List JAWS Hot Keys\nPress ESCAPE to close this message."
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
    assert_eq!(sel1, Some("selected H".to_string()));

    let sel2 = engine.execute_command("+{RIGHT}");
    assert_eq!(sel2, Some("selected e".to_string()));

    // Unselect prior character
    let unsel = engine.execute_command("+{LEFT}");
    assert_eq!(unsel, Some("unselected e".to_string()));

    // Query active selection
    let say_sel = engine.execute_command("SaySelection");
    assert_eq!(say_sel, Some("H".to_string()));

    // Moving cursor clears selection
    engine.execute_command("{RIGHT}");
    let no_sel = engine.execute_command("SaySelection");
    assert_eq!(no_sel, Some("No selection".to_string()));
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
    assert!(s1.starts_with("desktop\nFolderView    List view"));
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
    assert!(entry.starts_with("desktop\n"), "{entry:?}");
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
        first.starts_with("desktop\nFolderView    List view"),
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
    // Alt+Tab: the switcher's item takes focus through UIA.
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
    assert!(s3.contains("FolderView    List view"));
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
fn test_jaws_key_combinations_are_consumed_in_native_apps() {
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
struct SharedSpeech(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl SpeechOutput for SharedSpeech {
    fn speak(&mut self, text: &str, _interrupt: bool) {
        if let Ok(mut said) = self.0.lock() {
            said.push(text.to_owned());
        }
    }
    fn stop(&mut self) {}
}

impl SharedSpeech {
    fn take(&self) -> Vec<String> {
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
    engine.settings = crate::settings::Settings::reference_machine();
    let mut notepad = FocusEvent::new(Role::Document)
        .with_name("Text editor")
        .with_window_title("viewer-test.txt - Notepad");
    notepad.text = Some(crate::event::TextContext {
        text: "First line of the viewer test\rSecond line".to_owned(),
        caret: 2,
        selection: None,
        font: None,
        word: None,
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
fn test_caps_lock_as_the_jaws_key() {
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

    engine.settings = crate::settings::Settings::reference_machine();
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
fn test_jaws_key_in_forms_mode_reads_the_caret_line() {
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
    assert!(announcement.contains("Virtual PC Cursor"));
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
    engine.settings = crate::settings::Settings::reference_machine();
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
