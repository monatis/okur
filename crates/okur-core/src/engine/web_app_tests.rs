//! Web applications that build their pages as they are used, as WhatsApp Web does (dev box,
//! 2026-10-17): content added after the page was read, plain elements that take the focus,
//! items the page's own arrows move through, and popup menus.

use super::*;
use crate::actions::RecordingActions;
use crate::aom::{AomNode, Role, UNIQUE_ID_ATTR};
use crate::input::Modifiers;
use crate::speech::MockSpeechOutput;

const URL: &str = "https://web.whatsapp.com/";

/// `node` as the live object `object` (its IA2 uniqueID).
fn object(mut node: AomNode, object: i64) -> AomNode {
    node.attributes
        .insert(UNIQUE_ID_ATTR.to_owned(), object.to_string());
    node
}

fn focusable(mut node: AomNode, can: bool) -> AomNode {
    node.states.focusable = Some(can);
    node
}

/// One of the user's messages, numbered from `id`: a row whose focusable group holds the
/// sender's label ("Siz:", on the first of a run), the text, and its time, a button nothing
/// focuses. Objects are the negated ids.
fn message(id: usize, text: &str, time: &str, labelled: bool) -> AomNode {
    let live = |node: AomNode| {
        let id = node.id;
        object(node, -(id as i64))
    };
    let mut parts = Vec::new();
    if labelled {
        parts.push(live(AomNode::new(id + 2, Role::Text).with_text("Siz:")));
    }
    parts.push(live(AomNode::new(id + 3, Role::Group).with_children(vec![
        live(AomNode::new(id + 4, Role::Text).with_text(text)),
    ])));
    parts.push(live(focusable(
        AomNode::new(id + 5, Role::Button).with_name(format!("{time} Okundu")),
        false,
    )));
    live(focusable(
        AomNode::new(id, Role::TableRow).with_children(vec![live(focusable(
            AomNode::new(id + 1, Role::Group).with_children(parts),
            true,
        ))]),
        true,
    ))
}

/// The chat with oneself, its newest message last, and the message field below.
fn chat(title: &str, messages: Vec<AomNode>) -> AomNode {
    let mut composer = AomNode::new(900, Role::Edit)
        .with_name("+90 500 000 00 00 sohbetine bir mesaj yazın")
        .with_value("\n");
    composer
        .attributes
        .insert("multiline".to_owned(), "true".to_owned());
    let mut page = AomNode::new(0, Role::Document)
        .with_name(title)
        .with_children(vec![
            object(AomNode::new(1, Role::Group).with_children(messages), -1),
            object(focusable(composer, true), -900),
        ]);
    page.attributes.insert("url".to_owned(), URL.to_owned());
    page
}

fn messages() -> Vec<AomNode> {
    vec![
        message(10, "hello", "13:14", true),
        message(20, "E", "13:57", false),
        message(30, "test", "14:13", false),
        message(40, "okur 1", "14:46", true),
    ]
}

fn engine_on(page: &AomNode, actions: &RecordingActions) -> ScreenReaderEngine {
    let cursor = VirtualCursor::new(VirtualBuffer::from_aom(page));
    let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
    engine.set_actions(Box::new(actions.clone()));
    // The page was entered already: its message field had the focus.
    engine.last_page_focus = Some(900);
    engine
}

/// Puts the virtual cursor on the line reading `text`, the first or the last one.
fn cursor_to(engine: &mut ScreenReaderEngine, text: &str, last: bool) {
    let mut lines = engine.cursor.buffer.lines.iter().filter(|l| l.text == text);
    let line = if last {
        lines.next_back()
    } else {
        lines.next()
    };
    let start = line.map(|l| l.start_offset);
    assert!(start.is_some(), "no line {text:?}");
    engine.cursor.offset = start.unwrap_or_default();
}

fn key(key: Key, shift: bool) -> KeyEvent {
    KeyEvent::new(
        key,
        Modifiers {
            shift,
            ..Modifiers::none()
        },
        true,
    )
}

fn focused(actions: &RecordingActions) -> Vec<usize> {
    actions
        .requests()
        .into_iter()
        .filter(|(action, _)| action == "focus")
        .map(|(_, id)| id)
        .collect()
}

fn page_requests(actions: &RecordingActions) -> Vec<String> {
    actions
        .requests()
        .into_iter()
        .map(|(action, _)| action)
        .filter(|a| a.starts_with("page") || a.starts_with("refresh"))
        .collect()
}

/// The focus WhatsApp reports for a message: its group, renamed as it takes the focus.
fn message_focus(group: usize, name: &str) -> FocusEvent {
    let mut focus = FocusEvent::new(Role::Group).with_name(name);
    focus.in_document = true;
    focus.unique_id = Some(format!("-{group}"));
    focus
}

/// Shift+Tab from a message's text goes to the message before it, as JAWS's did: the
/// element that takes the focus, a plain group, not the time button nothing focuses.
#[test]
fn tab_from_the_virtual_cursor_goes_to_a_focusable_message() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    cursor_to(&mut engine, "E", false);
    assert_eq!(
        engine.handle_key_event(&key(Key::Tab, true)),
        KeyAction::Consume
    );
    // hello's group, past its time button (25 would be E's own time).
    assert_eq!(focused(&actions), vec![11]);
}

/// Tab onto a message turns forms mode on, so the arrows are WhatsApp's: it moves through
/// its messages, and Okur says each one by the name the page gives it then (the user asked
/// for list-box arrows; JAWS 2025 stays in browse mode there).
#[test]
fn tab_onto_a_message_gives_the_page_its_arrows() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    cursor_to(&mut engine, "E", false);
    engine.handle_key_event(&key(Key::Tab, true));
    let name = "Siz hello 13:14 Okundu Daha fazla seçenek için sol veya sağ ok tuşuna basarak \
                bağlam menüsüne erişin";
    let speech =
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(message_focus(11, name)));
    assert_eq!(speech.as_deref(), Some(name));
    assert!(engine.forms_mode);
    assert_eq!(
        engine.handle_key_event(&key(Key::DownArrow, false)),
        KeyAction::PassThrough
    );
    let next = engine.handle_accessibility_event(&AccessibilityEvent::Focus(message_focus(
        21,
        "Siz E 13:57 Okundu",
    )));
    assert_eq!(next.as_deref(), Some("Siz E 13:57 Okundu"));
    assert!(engine.forms_mode);
}

/// A message the page focuses by itself, with no Tab, leaves the mode as it was.
#[test]
fn a_message_focused_without_tab_keeps_browse_mode() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    engine.handle_key_event(&key(Key::DownArrow, false));
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(message_focus(
        11,
        "Siz hello 13:14 Okundu",
    )));
    assert!(!engine.forms_mode);
}

/// The page changed outside its live regions (a chat's messages rendered as it opens): in
/// browse mode it is captured again once the changes settle; in forms mode (typing in the
/// message field changes the page with each key) only when forms mode ends, at once.
#[test]
fn a_changed_page_is_captured_again_outside_forms_mode() {
    let actions = RecordingActions::refreshing_pages();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    engine.handle_accessibility_event(&AccessibilityEvent::PageChanged);
    assert_eq!(page_requests(&actions), ["page changed"]);

    let mut composer = FocusEvent::new(Role::Edit)
        .with_node_id(900)
        .with_name("+90 500 000 00 00 sohbetine bir mesaj yazın");
    composer.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(composer));
    assert!(engine.forms_mode);
    engine.handle_accessibility_event(&AccessibilityEvent::PageChanged);
    engine.handle_accessibility_event(&AccessibilityEvent::PageChanged);
    assert_eq!(page_requests(&actions), ["page changed"]);

    engine.handle_key_event(&key(Key::Escape, false));
    assert!(!engine.forms_mode);
    assert_eq!(
        page_requests(&actions),
        ["page changed", "page changed, now"]
    );
}

/// WhatsApp's title counts unread chats ("(8) WhatsApp"): a capture with another title at
/// the same address is the same page, read again where the cursor is, not a new page.
#[test]
fn a_new_title_at_the_same_address_is_the_same_page() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    cursor_to(&mut engine, "test", false);
    let speech = engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "(8) WhatsApp".to_owned(),
        root: chat("(8) WhatsApp", messages()),
    });
    assert_eq!(speech, None);
    engine.handle_key_event(&key(Key::Control, false));
    assert_eq!(engine.cursor.current_line(), "test");
}

/// Older messages load above as a chat is read up, and WhatsApp repeats lines ("Siz:"):
/// the cursor stays on its own line, found by its object, not on the copy now nearest its
/// old offset.
#[test]
fn the_cursor_keeps_its_line_when_messages_load_above() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    cursor_to(&mut engine, "Siz:", true);
    let line_object = |engine: &ScreenReaderEngine| {
        let line = engine.cursor.buffer.line_at_offset(engine.cursor.offset)?;
        engine
            .cursor
            .buffer
            .node(line.node_id)?
            .attributes
            .get(UNIQUE_ID_ATTR)
            .cloned()
    };
    let before = line_object(&engine);
    assert_eq!(before.as_deref(), Some("-42"));

    let mut loaded = vec![
        message(50, "older one", "09:00", true),
        message(60, "older two", "09:05", true),
    ];
    loaded.extend(messages());
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "(7) WhatsApp".to_owned(),
        root: chat("(7) WhatsApp", loaded),
    });
    engine.handle_key_event(&key(Key::Control, false));
    assert_eq!(line_object(&engine), before);
}

/// The empty message field holds a line break: it is blank, as JAWS says it.
#[test]
fn a_field_holding_a_line_break_is_blank() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let speech = engine.execute_command("E").unwrap_or_default();
    assert!(speech.ends_with("blank"), "{speech:?}");
    assert!(!speech.contains("Contains text"), "{speech:?}");
}

/// WhatsApp's message menu (JAWS on the dev box, 2026-10-17): a popup of its own, whose
/// items say their positions, with reaction buttons above them. A reaction is still in the
/// menu: no "Leaving menus", the keys stay the page's, and the item the focus comes back to
/// is said again.
#[test]
fn a_popup_menu_keeps_its_reactions() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let item = |name: &str, index: usize| {
        let mut focus = FocusEvent::new(Role::MenuItem).with_name(name);
        focus.in_document = true;
        focus.item_index = Some(index);
        focus.item_count = Some(9);
        AccessibilityEvent::Focus(focus)
    };
    let first = engine.handle_accessibility_event(&item("Mesaj bilgisi", 1));
    assert_eq!(
        first.as_deref(),
        Some("Mesaj bilgisi\n1 of 9\nTo move through items press up or down arrow.")
    );
    assert_eq!(
        engine
            .handle_accessibility_event(&item("Cevapla", 2))
            .as_deref(),
        Some("Cevapla\n2 of 9")
    );

    let mut reaction = FocusEvent::new(Role::Button).with_name("👍");
    reaction.in_document = true;
    reaction.in_menu = true;
    reaction.states.checked = Some(false);
    let speech = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(reaction))
        .unwrap_or_default();
    assert_eq!(
        speech,
        "👍  \u{1d} Toggle button \u{1e}  \u{1d}Not pressed\u{1e} "
    );
    assert!(engine.forms_mode);
    assert_eq!(
        engine.handle_key_event(&key(Key::Tab, false)),
        KeyAction::PassThrough
    );
    assert_eq!(
        engine
            .handle_accessibility_event(&item("Cevapla", 2))
            .as_deref(),
        Some("Cevapla\n2 of 9")
    );
}

/// A control the page shows only once its message has the focus (WhatsApp's message
/// options button) is said from the focus event as the buffer would say it.
#[test]
fn a_control_the_buffer_lacks_is_said_from_its_event() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let mut button = FocusEvent::new(Role::Button).with_name("Mesaj seçeneklerini aç");
    button.in_document = true;
    button.states.expanded = Some(false);
    let speech = engine
        .handle_accessibility_event(&AccessibilityEvent::Focus(button))
        .unwrap_or_default();
    // Words and marks, whatever the spaces between them.
    let words = speech.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        words.starts_with("Mesaj seçeneklerini aç \u{1d} Button \u{1e} \u{1d}collapsed\u{1e}"),
        "{speech:?}"
    );
    assert!(speech.ends_with("To activate press Enter."), "{speech:?}");
}

/// WhatsApp's search (dev box, 2026-10-17): Down from the search box into its results, which
/// the page built as the box was typed in, keeps forms mode; the result the buffer lacks is
/// captured again and the virtual cursor goes to it. Enter opens its chat, whose message
/// field the page focuses: the virtual cursor reads on from there in browse mode, for the
/// user's Ctrl+End, Shift+E and Enter.
#[test]
fn a_search_result_opens_its_chat_in_browse_mode() {
    let search = |focused: bool| {
        let mut field = AomNode::new(5, Role::Edit).with_name("Aratın veya yeni sohbet başlatın");
        field.states.focused = focused;
        object(focusable(field, true), -5)
    };
    let page = |children: Vec<AomNode>| {
        let mut page = AomNode::new(0, Role::Document)
            .with_name("(7) WhatsApp")
            .with_children(children);
        page.attributes.insert("url".to_owned(), URL.to_owned());
        page
    };
    let actions = RecordingActions::refreshing_pages();
    let mut engine = engine_on(&page(vec![search(true)]), &actions);
    let mut field = FocusEvent::new(Role::Edit)
        .with_node_id(5)
        .with_name("Aratın veya yeni sohbet başlatın");
    field.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(field));
    assert!(engine.forms_mode);
    assert_eq!(
        engine.handle_key_event(&key(Key::DownArrow, false)),
        KeyAction::PassThrough
    );

    let name = "+90 500 000 00 00 (Siz) 14:46 okur 1";
    let mut result = FocusEvent::new(Role::TableCell).with_name(name);
    result.in_document = true;
    result.unique_id = Some("-71".to_owned());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(result));
    assert!(engine.forms_mode);
    assert_eq!(page_requests(&actions), ["refresh page"]);

    let mut cell = AomNode::new(71, Role::TableCell).with_name(name);
    cell.states.focused = true;
    let results = AomNode::new(60, Role::Table)
        .with_name("Arama sonuçları.")
        .with_children(vec![AomNode::new(70, Role::TableRow)
            .with_children(vec![object(focusable(cell, true), -71)])]);
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "(7) WhatsApp".to_owned(),
        root: page(vec![search(false), results]),
    });
    assert_eq!(
        engine.handle_key_event(&key(Key::Enter, false)),
        KeyAction::PassThrough
    );
    assert_eq!(engine.cursor.current_node_id, Some(71));

    let mut composer =
        FocusEvent::new(Role::Edit).with_name("+90 500 000 00 00 sohbetine bir mesaj yazın");
    composer.in_document = true;
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(composer));
    assert!(!engine.forms_mode);
}

/// Pass Key Through (JAWS key+3): "Enter key to pass through", and the next key goes to the
/// page in browse mode too, with the modifiers held for it (JAWS on the dev box, 2026-10-17).
#[test]
fn the_next_key_passes_through() {
    let actions = RecordingActions::default();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let speech = super::tests::SharedSpeech::default();
    engine.speech = Box::new(speech.clone());
    let jaws_3 = KeyEvent::new(
        Key::Digit(3),
        Modifiers {
            insert: true,
            ..Modifiers::none()
        },
        true,
    );
    assert_eq!(engine.handle_key_event(&jaws_3), KeyAction::Consume);
    assert_eq!(speech.take(), ["Enter key to pass through"]);
    assert_eq!(
        engine.handle_key_event(&key(Key::Control, false)),
        KeyAction::PassThrough
    );
    let ctrl_end = KeyEvent::new(
        Key::End,
        Modifiers {
            ctrl: true,
            ..Modifiers::none()
        },
        true,
    );
    assert_eq!(engine.handle_key_event(&ctrl_end), KeyAction::PassThrough);
    assert_eq!(engine.handle_key_event(&ctrl_end), KeyAction::Consume);
}

/// A viewer the page opens over itself (WhatsApp's PDF preview) takes the focus on an
/// element whose lines are its children's: once the page is captured again, the virtual
/// cursor is on its first line, and the arrows read the document from there.
#[test]
fn the_cursor_goes_to_a_focus_whose_lines_are_its_childrens() {
    let actions = RecordingActions::refreshing_pages();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let mut focus = FocusEvent::new(Role::Group).with_name("PDF content");
    focus.in_document = true;
    focus.unique_id = Some("-800".to_owned());
    engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
    assert_eq!(page_requests(&actions), ["refresh page"]);

    let mut content = AomNode::new(800, Role::Group)
        .with_name("PDF content")
        .with_children(vec![
            AomNode::new(801, Role::Paragraph).with_text("Page 1"),
            AomNode::new(802, Role::Paragraph).with_text("First line of the document"),
        ]);
    content.states.focused = true;
    let mut page = chat("(7) WhatsApp", messages());
    page.children.push(object(content, -800));
    engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
        title: "(7) WhatsApp".to_owned(),
        root: page,
    });
    engine.handle_key_event(&key(Key::Control, false));
    assert_eq!(engine.cursor.current_line(), "Page 1");
    engine.handle_key_event(&key(Key::DownArrow, false));
    assert_eq!(engine.cursor.current_line(), "First line of the document");
}

/// A capture of the page in front, built outside the engine, is applied as a capture of it is;
/// one of another page (loaded while it was built) is dropped, never announced.
#[test]
fn a_refresh_built_outside_the_engine_is_taken_for_the_same_page_only() {
    let actions = RecordingActions::refreshing_pages();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let mut list = messages();
    list.push(message(70, "a new one", "15:00", false));
    let refreshed = VirtualBuffer::from_aom(&chat("(8) WhatsApp", list.clone()));
    let title = refreshed.document_title().unwrap_or_default().to_owned();
    engine.handle_page_refreshed(&title, refreshed);
    engine.handle_key_event(&key(Key::Control, false));
    let has_new = |engine: &ScreenReaderEngine| {
        engine
            .cursor
            .buffer
            .lines
            .iter()
            .any(|l| l.text == "a newer one")
    };
    assert!(engine
        .cursor
        .buffer
        .lines
        .iter()
        .any(|l| l.text == "a new one"));

    let mut other = AomNode::new(0, Role::Document).with_name("Another page");
    other
        .attributes
        .insert("url".into(), "https://example.org/another".into());
    list.push(message(71, "a newer one", "15:01", false));
    let other = other.with_children(list);
    engine.handle_page_refreshed("Another page", VirtualBuffer::from_aom(&other));
    engine.handle_key_event(&key(Key::Control, false));
    assert!(!has_new(&engine));
}

/// Parts spliced and built outside the engine (the live screen reader does, so as not to hold
/// it) give the page that splicing them inside gives; once the page is another, the buffer
/// built from the old one is refused.
#[test]
fn a_page_built_outside_the_engine_is_taken_while_the_page_is_the_same() {
    let actions = RecordingActions::refreshing_pages();
    let mut inside = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let mut outside = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let mut list = messages();
    list.push(message(70, "a new one", "15:00", false));
    let part = object(AomNode::new(1, Role::Group).with_children(list), -1);

    inside.handle_accessibility_event(&AccessibilityEvent::PagePartsLoaded(vec![part.clone()]));
    let base = outside.page_for_parts();
    assert!(base.is_some());
    let Some(base) = base else { return };
    let built = VirtualBuffer::spliced(&base, std::slice::from_ref(&part))
        .map(VirtualBuffer::from_aom_owned);
    assert!(built.is_some());
    let Some(built) = built else { return };
    assert!(outside.install_page_parts(&base, built.clone()));
    for engine in [&mut inside, &mut outside] {
        engine.handle_key_event(&key(Key::Control, false));
    }
    let lines = |engine: &ScreenReaderEngine| {
        engine
            .cursor
            .buffer
            .lines
            .iter()
            .map(|l| l.text.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(lines(&outside), lines(&inside));

    // The page is another now: a buffer built from the old one is not taken.
    assert!(!outside.install_page_parts(&base, built));
}

/// A part of the page read again (the message list, after a message arrived) is spliced in
/// where the same object is: the new message is there, and the cursor stays on its line.
/// A part the page does not hold makes the whole page read again.
#[test]
fn a_part_read_again_is_spliced_into_the_page() {
    let actions = RecordingActions::refreshing_pages();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    cursor_to(&mut engine, "test", false);

    let mut list = messages();
    list.push(message(70, "a new one", "15:00", false));
    let part = object(AomNode::new(1, Role::Group).with_children(list), -1);
    engine.handle_accessibility_event(&AccessibilityEvent::PagePartsLoaded(vec![part]));
    engine.handle_key_event(&key(Key::Control, false));
    assert_eq!(engine.cursor.current_line(), "test");
    assert!(
        engine
            .cursor
            .buffer
            .lines
            .iter()
            .any(|l| l.text == "a new one"),
        "the new message is in the buffer"
    );
    assert!(page_requests(&actions).is_empty());

    let unknown = object(AomNode::new(1, Role::Group), -12345);
    engine.handle_accessibility_event(&AccessibilityEvent::PagePartsLoaded(vec![unknown]));
    assert_eq!(page_requests(&actions), ["page changed, now"]);
}

/// A part spliced without the engine and not found in the page makes the whole page read
/// again, as one spliced inside it does; once the page is another, nothing is asked, and the
/// host splices the part into that one.
#[test]
fn a_part_not_found_without_the_engine_reads_the_page_again_while_it_is_the_same() {
    let actions = RecordingActions::refreshing_pages();
    let mut engine = engine_on(&chat("(7) WhatsApp", messages()), &actions);
    let unknown = object(AomNode::new(1, Role::Group), -12345);
    let base = engine.page_for_parts();
    assert!(base.is_some());
    let Some(base) = base else { return };
    assert!(VirtualBuffer::spliced(&base, std::slice::from_ref(&unknown)).is_none());
    assert!(engine.page_parts_not_found(&base));
    assert_eq!(page_requests(&actions), ["page changed, now"]);

    let mut list = messages();
    list.push(message(70, "a new one", "15:00", false));
    engine.handle_page_refreshed(
        "(7) WhatsApp",
        VirtualBuffer::from_aom(&chat("(7) WhatsApp", list)),
    );
    assert!(!engine.page_parts_not_found(&base));
    assert_eq!(page_requests(&actions), ["page changed, now"]);
}
