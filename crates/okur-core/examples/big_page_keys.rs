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
}
