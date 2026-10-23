//! Times the keys that walk a whole page, on a big, deep one: each chord through
//! `handle_key_event`, as the keyboard hook runs it (a scratch benchmark, run by hand:
//! `cargo run -p okur-core --release --example big_page_keys [rows]`).
//!
//! The page is the performance reviews' (2026-10-19, 2026-10-20): per row a division holding
//! a link, a paragraph, a check box and a time, under 30 wrapper divisions, every node with a
//! `tag`, an `id` and an IA2 uniqueID. 5,000 rows give 25,032 nodes.
use okur_core::{
    AccessibilityEvent, AomNode, Key, KeyEvent, MockSpeechOutput, Modifiers, Role,
    ScreenReaderEngine, VirtualBuffer, VirtualCursor,
};

fn tagged(id: &mut usize, role: Role, tag: &str) -> AomNode {
    *id += 1;
    let mut node = AomNode::new(*id, role);
    node.attributes.insert("tag".to_owned(), tag.to_owned());
    node.attributes.insert("id".to_owned(), format!("n{id}"));
    node.attributes
        .insert("object".to_owned(), format!("-{id}"));
    node
}

fn page(rows: usize) -> AomNode {
    let mut id = 0;
    let rows = (0..rows)
        .map(|row| {
            let link =
                tagged(&mut id, Role::Link { href: None }, "a").with_name(format!("Sender {row}"));
            let text = tagged(&mut id, Role::Paragraph, "p")
                .with_text(format!("Message number {row} in a long chat log."));
            let check = tagged(&mut id, Role::CheckBox, "input").with_name("Read");
            let time = tagged(&mut id, Role::Text, "span").with_text("10:41");
            tagged(&mut id, Role::Group, "div").with_children(vec![link, text, check, time])
        })
        .collect();
    let mut root = tagged(&mut id, Role::Group, "div").with_children(rows);
    for _ in 0..30 {
        root = tagged(&mut id, Role::Group, "div").with_children(vec![root]);
    }
    AomNode::new(0, Role::Document)
        .with_name("Big page")
        .with_children(vec![root])
}

fn chord(key: Key, shift: bool, ctrl: bool, insert: bool) -> KeyEvent {
    let mut event = KeyEvent::new(key, Modifiers::none(), true);
    event.modifiers.shift = shift;
    event.modifiers.ctrl = ctrl;
    event.modifiers.insert = insert;
    event
}

fn main() {
    let rows = std::env::args()
        .nth(1)
        .and_then(|r| r.parse().ok())
        .unwrap_or(5_000);
    let root = page(rows);
    let chords = [
        ("Z", chord(Key::Alpha('Z'), false, false, false)),
        ("Shift+Z", chord(Key::Alpha('Z'), true, false, false)),
        ("Ctrl+Insert+Z", chord(Key::Alpha('Z'), false, true, true)),
        ("Ctrl+Insert+P", chord(Key::Alpha('P'), false, true, true)),
        ("Ctrl+Insert+X", chord(Key::Alpha('X'), false, true, true)),
        ("Insert+F7", chord(Key::F(7), false, false, true)),
        ("Insert+F5", chord(Key::F(5), false, false, true)),
    ];
    for (name, key) in chords {
        let mut times = Vec::new();
        for _ in 0..3 {
            let cursor =
                VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
            let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
            engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
                title: "Big page".to_owned(),
                root: root.clone(),
            });
            let started = std::time::Instant::now();
            engine.handle_key_event(&key);
            times.push(started.elapsed());
        }
        times.sort();
        println!(
            "{name:>14}: {:?} (median of {}, {} rows)",
            times.get(times.len() / 2).copied().unwrap_or_default(),
            times.len(),
            rows
        );
    }
    // Every everyday page key, pressed from the page's middle on one engine, as a user would
    // press them one after another: a sweep for keys slow enough to matter in the hook
    // (performance review 2026-10-23, T1). Printed when over 1 ms.
    {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
            title: "Big page".to_owned(),
            root: root.clone(),
        });
        let plain = |key| chord(key, false, false, false);
        let shift = |key| chord(key, true, false, false);
        let ctrl = |key| chord(key, false, true, false);
        let insert = |key| chord(key, false, false, true);
        let ctrl_insert = |key| chord(key, false, true, true);
        let mut sweep: Vec<(String, KeyEvent)> = Vec::new();
        for c in "HKUTLIBEFXCRQGSDNMOAJWVY".chars() {
            sweep.push((c.to_string(), plain(Key::Alpha(c))));
            sweep.push((format!("Shift+{c}"), shift(Key::Alpha(c))));
        }
        for d in 1..=6u8 {
            sweep.push((d.to_string(), plain(Key::Digit(d))));
        }
        for (name, key) in [
            ("Down", plain(Key::DownArrow)),
            ("Up", plain(Key::UpArrow)),
            ("Ctrl+Down", ctrl(Key::DownArrow)),
            ("Ctrl+Up", ctrl(Key::UpArrow)),
            ("Ctrl+Right", ctrl(Key::RightArrow)),
            ("PageDown", plain(Key::PageDown)),
            ("Ctrl+End", ctrl(Key::End)),
            ("Ctrl+Home", ctrl(Key::Home)),
            ("Semicolon", plain(Key::Semicolon)),
            ("Slash", plain(Key::Slash)),
            ("Minus", plain(Key::Minus)),
            ("Insert+Up", insert(Key::UpArrow)),
            ("Insert+T", insert(Key::Alpha('T'))),
            ("Insert+F6", insert(Key::F(6))),
            ("Insert+F7", insert(Key::F(7))),
            ("Insert+F5", insert(Key::F(5))),
            ("Insert+F3", insert(Key::F(3))),
            ("Ctrl+Insert+Down", ctrl_insert(Key::DownArrow)),
            ("Insert+Down", insert(Key::DownArrow)),
            ("Ctrl", plain(Key::Control)),
            ("Insert+Enter", insert(Key::Enter)),
            ("NumpadPlus", plain(Key::NumpadPlus)),
            ("Ctrl+Insert+F", ctrl_insert(Key::Alpha('F'))),
        ] {
            sweep.push((name.to_owned(), key));
        }
        let middle = format!("Sender {}", rows / 2);
        for (name, key) in sweep {
            // Each key from the middle of the page.
            let _ = engine.execute_command("TopOfFile");
            let _ = engine.cursor.find_text(&middle, true);
            if let Some(escape) = Some(plain(Key::Escape)) {
                engine.handle_key_event(&escape);
            }
            let started = std::time::Instant::now();
            engine.handle_key_event(&key);
            let took = started.elapsed();
            if took > std::time::Duration::from_millis(1) {
                println!("{name:>16}: {took:?} (from the middle, {rows} rows)");
            }
        }
        println!("{:>16}: done", "sweep");
    }
    // A page focus the buffer finds by role and name only (no uniqueID or DOM id), as while
    // the page is the quick UIA read: the last row's check box.
    let mut times = Vec::new();
    for _ in 0..3 {
        let cursor = VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
        let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
        engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
            title: "Big page".to_owned(),
            root: root.clone(),
        });
        let mut focus = okur_core::FocusEvent::new(Role::Link { href: None })
            .with_name(format!("Sender {}", rows.saturating_sub(1)));
        focus.in_document = true;
        let started = std::time::Instant::now();
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus.clone()));
        let first = started.elapsed();
        focus.name = Some(format!("Sender {}", rows / 2));
        let started = std::time::Instant::now();
        engine.handle_accessibility_event(&AccessibilityEvent::Focus(focus));
        times.push((first, started.elapsed()));
    }
    times.sort();
    println!(
        "{:>14}: {:?} (first, then the next; median of {}, {} rows)",
        "page focus",
        times.get(times.len() / 2).copied().unwrap_or_default(),
        times.len(),
        rows
    );
    // A check box toggled (the middle row's): the page built again at once, or left to the
    // host to build off the engine, as the live screen reader does (performance review P3).
    for (name, by_host) in [("toggle, here", false), ("toggle, host", true)] {
        let mut times = Vec::new();
        for _ in 0..3 {
            let cursor =
                VirtualCursor::new(VirtualBuffer::from_aom(&AomNode::new(0, Role::Document)));
            let mut engine = ScreenReaderEngine::new(cursor, Box::new(MockSpeechOutput::new()));
            engine.handle_accessibility_event(&AccessibilityEvent::DocumentLoaded {
                title: "Big page".to_owned(),
                root: root.clone(),
            });
            if by_host {
                engine.set_page_builds_elsewhere(0);
            }
            let mut control = okur_core::FocusEvent::new(Role::CheckBox)
                .with_node_id(5 * (rows / 2) + 3)
                .with_name("Read");
            control.in_document = true;
            control.states.checked = Some(true);
            let started = std::time::Instant::now();
            engine.handle_accessibility_event(&AccessibilityEvent::Change {
                kind: okur_core::ChangeKind::State,
                control,
            });
            times.push(started.elapsed());
        }
        times.sort();
        println!(
            "{name:>14}: {:?} (median of {}, {} rows)",
            times.get(times.len() / 2).copied().unwrap_or_default(),
            times.len(),
            rows
        );
    }
}
