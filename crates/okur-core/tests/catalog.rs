//! Guards of the message catalog (`src/messages.rs`, `locales/en-US/*.ftl`):
//!
//! 1. **No word of Okur's is written into code.** Every string literal with words in the
//!    non-test code of `okur-core` and `okur-ui` is either listed in `tests/literals.txt` as an
//!    identifier (an attribute key, a class name, a JCF key: never said or shown), listed there
//!    as still pending its move to the catalog, or it fails this test. A new
//!    `speech.speak("Some new words")` fails, and the failure gives the line to add if the
//!    literal really is an identifier. A listed literal that no longer exists fails too, so the
//!    list stays exact.
//! 2. **Every message is used.** A message whose constructor (`msg::<id>`) no code outside tests
//!    calls fails.
//! 3. **The pseudo-locale run.** With every message wrapped in `⟦…⟧`
//!    ([`okur_core::messages::with_pseudo_locale`]), the engine reads a page and native
//!    controls; what it says holds no word that is neither wrapped nor the page's own. This
//!    finds words built at run time that the scan cannot tell from identifiers.
//!
//! The source scan reads Rust as tokens: comments, attributes (`#[serde(rename = "…")]`),
//! `tracing` macros and `eprintln!` (developer logs, never spoken) and `#[cfg(test)]` items
//! (and the files of `#[cfg(test)] mod x;`) are left out. A literal has words when, without
//! its escapes and format placeholders, two letters follow each other. One without spaces
//! holding `_` or `::` (`snake_case`, `Option::is_none`) is an identifier by its shape and
//! needs no entry.
//!
//! Why the scan is the main guard: a run finds only the words the paths it exercises say,
//! while the scan sees every literal, including those of paths no test reaches.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use okur_core::aom::{AomNode, Role};
use okur_core::event::FocusEvent;
use okur_core::messages::{with_pseudo_locale, MessageId};
use okur_core::{MockSpeechOutput, ScreenReaderEngine, VirtualBuffer, VirtualCursor};

/// The crates whose literals are checked: those that hold what Okur says and shows.
const CHECKED: &[&str] = &["okur-core", "okur-ui"];

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Punct(char),
    Str {
        text: String,
        raw: bool,
        line: usize,
    },
}

/// Rust source as tokens, without comments, character literals, lifetimes, numbers and byte
/// strings.
fn tokens(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let at = |i: usize| chars.get(i).copied().unwrap_or('\0');
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    while i < chars.len() {
        let c = at(i);
        let prev_ident = i > 0 && is_ident(at(i - 1));
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && at(i + 1) == '/' {
            while i < chars.len() && at(i) != '\n' {
                i += 1;
            }
        } else if c == '/' && at(i + 1) == '*' {
            let mut depth = 0;
            loop {
                if i >= chars.len() {
                    break;
                }
                if at(i) == '/' && at(i + 1) == '*' {
                    depth += 1;
                    i += 2;
                } else if at(i) == '*' && at(i + 1) == '/' {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if at(i) == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if !prev_ident && (c == 'r' || (c == 'b' && at(i + 1) == 'r')) && {
            let mut j = i + if c == 'b' { 2 } else { 1 };
            while at(j) == '#' {
                j += 1;
            }
            at(j) == '"'
        } {
            // A raw string: r"…", r#"…"#, br"…".
            let byte = c == 'b';
            let mut j = i + if byte { 2 } else { 1 };
            let mut hashes = 0;
            while at(j) == '#' {
                hashes += 1;
                j += 1;
            }
            j += 1;
            let start = j;
            let start_line = line;
            loop {
                if j >= chars.len() {
                    break;
                }
                if at(j) == '"' && (1..=hashes).all(|k| at(j + k) == '#') {
                    break;
                }
                if at(j) == '\n' {
                    line += 1;
                }
                j += 1;
            }
            if !byte {
                let text: String = chars.get(start..j).unwrap_or_default().iter().collect();
                out.push(Token::Str {
                    text,
                    raw: true,
                    line: start_line,
                });
            }
            i = j + 1 + hashes;
        } else if c == '"' || (!prev_ident && c == 'b' && at(i + 1) == '"') {
            let byte = c == 'b';
            let mut j = i + if byte { 2 } else { 1 };
            let start = j;
            let start_line = line;
            while j < chars.len() && at(j) != '"' {
                if at(j) == '\\' {
                    if at(j + 1) == '\n' {
                        line += 1;
                    }
                    j += 2;
                    continue;
                }
                if at(j) == '\n' {
                    line += 1;
                }
                j += 1;
            }
            if !byte {
                let text: String = chars.get(start..j).unwrap_or_default().iter().collect();
                out.push(Token::Str {
                    text,
                    raw: false,
                    line: start_line,
                });
            }
            i = j + 1;
        } else if c == '\'' || (!prev_ident && c == 'b' && at(i + 1) == '\'') {
            let j = if c == 'b' { i + 1 } else { i };
            if at(j + 1) == '\\' {
                // An escaped character: '\n', '\u{1d}', '\''.
                let mut k = j + 3;
                while k < chars.len() && at(k) != '\'' {
                    k += 1;
                }
                i = k + 1;
            } else if at(j + 2) == '\'' {
                i = j + 3;
            } else {
                // A lifetime or a label.
                i = j + 1;
                while is_ident(at(i)) {
                    i += 1;
                }
            }
        } else if c.is_ascii_digit() {
            while is_ident(at(i)) || (at(i) == '.' && at(i + 1).is_ascii_digit()) {
                i += 1;
            }
        } else if is_ident(c) {
            let start = i;
            while is_ident(at(i)) {
                i += 1;
            }
            out.push(Token::Ident(
                chars.get(start..i).unwrap_or_default().iter().collect(),
            ));
        } else {
            out.push(Token::Punct(c));
            i += 1;
        }
    }
    out
}

/// What a file's non-test code holds.
#[derive(Debug, Default)]
struct Scan {
    /// String literals as written, with their lines.
    literals: Vec<(usize, String, bool)>,
    /// The constructors called as `msg::<name>`.
    messages: BTreeSet<String>,
    /// Modules declared `#[cfg(test)] mod name;`, whose files are tests.
    test_modules: Vec<String>,
}

/// The index after the group that opens at `open` (a `(`, `[` or `{`).
fn group_end(tokens: &[Token], open: usize) -> usize {
    let mut depth = 0usize;
    let mut i = open;
    while let Some(token) = tokens.get(i) {
        match token {
            Token::Punct('(' | '[' | '{') => depth += 1,
            Token::Punct(')' | ']' | '}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    i
}

/// The index after the item starting at `start`: its `;` or its `{ … }` at depth 0.
fn item_end(tokens: &[Token], start: usize) -> usize {
    let mut i = start;
    while let Some(token) = tokens.get(i) {
        match token {
            Token::Punct(';') => return i + 1,
            Token::Punct('{') => return group_end(tokens, i),
            Token::Punct('(' | '[') => i = group_end(tokens, i),
            _ => i += 1,
        }
    }
    i
}

const LOG_MACROS: &[&str] = &["trace", "debug", "info", "warn", "error", "eprintln"];

fn scan(source: &str) -> Scan {
    let tokens = tokens(source);
    let mut scan = Scan::default();
    let ident = |i: usize| match tokens.get(i) {
        Some(Token::Ident(name)) => Some(name.as_str()),
        _ => None,
    };
    let punct = |i: usize, c: char| tokens.get(i) == Some(&Token::Punct(c));
    let mut i = 0;
    let mut test_item = false;
    while let Some(token) = tokens.get(i) {
        // An attribute: #[…] or #![…].
        if punct(i, '#') && (punct(i + 1, '[') || (punct(i + 1, '!') && punct(i + 2, '['))) {
            let open = if punct(i + 1, '[') { i + 1 } else { i + 2 };
            let end = group_end(&tokens, open);
            let inner = tokens
                .get(open + 1..end.saturating_sub(1))
                .unwrap_or_default();
            if inner
                == [
                    Token::Ident("cfg".to_owned()),
                    Token::Punct('('),
                    Token::Ident("test".to_owned()),
                    Token::Punct(')'),
                ]
            {
                test_item = true;
            }
            i = end;
            continue;
        }
        if test_item {
            test_item = false;
            if ident(i) == Some("mod") && punct(i + 2, ';') {
                if let Some(name) = ident(i + 1) {
                    scan.test_modules.push(name.to_owned());
                }
            }
            i = item_end(&tokens, i);
            continue;
        }
        match token {
            Token::Ident(name) if LOG_MACROS.contains(&name.as_str()) && punct(i + 1, '!') => {
                i = group_end(&tokens, i + 2);
                continue;
            }
            Token::Ident(name) if name == "msg" && punct(i + 1, ':') && punct(i + 2, ':') => {
                if let Some(called) = ident(i + 3) {
                    scan.messages.insert(called.to_owned());
                }
            }
            Token::Str { text, raw, line } => scan.literals.push((*line, text.clone(), *raw)),
            _ => {}
        }
        i += 1;
    }
    scan
}

/// Whether a literal holds words: two letters in a row once escapes and format placeholders
/// are gone, and not an identifier by its shape.
fn has_words(text: &str, raw: bool) -> bool {
    let mut plain = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && !raw {
            match chars.next() {
                Some('u') => {
                    for c in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                    }
                }
                Some('x') => {
                    chars.next();
                    chars.next();
                }
                _ => {}
            }
            plain.push(' ');
        } else {
            plain.push(c);
        }
    }
    let plain = plain.replace("{{", " ").replace("}}", " ");
    let mut words = String::new();
    let mut depth = 0;
    for c in plain.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => words.push(c),
            _ => {}
        }
    }
    if !words.contains(char::is_whitespace) && (words.contains('_') || words.contains("::")) {
        return false;
    }
    let letters: Vec<bool> = words.chars().map(char::is_alphabetic).collect();
    letters.windows(2).any(|pair| pair == [true, true])
}

fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Every `.rs` file under `dir`, sorted.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// The non-test code of a crate's `src`, file by file, keyed by the path from `crates/`.
fn crate_scans(name: &str) -> BTreeMap<String, Scan> {
    let crates = crates_dir();
    let src = crates.join(name).join("src");
    let mut scans = BTreeMap::new();
    let mut tests = BTreeSet::new();
    for file in rust_files(&src) {
        let source = std::fs::read_to_string(&file).unwrap_or_default();
        let scan = scan(&source);
        // `mod x;` in `a.rs`, `a/mod.rs`, `lib.rs` or `main.rs` is `x.rs` or `x/mod.rs` beside
        // it or under `a/`.
        let stem = file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        let parent = file.parent().map(Path::to_path_buf).unwrap_or_default();
        let dir = if matches!(stem, "mod" | "lib" | "main") {
            parent
        } else {
            parent.join(stem)
        };
        for module in &scan.test_modules {
            tests.insert(dir.join(format!("{module}.rs")));
            tests.insert(dir.join(module).join("mod.rs"));
        }
        let key = file
            .strip_prefix(&crates)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        scans.insert(key, (file, scan));
    }
    scans
        .into_iter()
        .filter(|(_, (file, _))| !tests.contains(file))
        .map(|(key, (_, scan))| (key, scan))
        .collect()
}

/// `tests/literals.txt`: (section, file or `*`, literal or `*`).
fn allowed() -> Vec<(String, String, String)> {
    let list = include_str!("literals.txt");
    let mut section = String::new();
    let mut entries = Vec::new();
    for line in list.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            section = name.to_owned();
            continue;
        }
        if let Some((file, literal)) = line.split_once(": ") {
            let literal = literal
                .strip_prefix('"')
                .and_then(|l| l.strip_suffix('"'))
                .unwrap_or(literal);
            entries.push((section.clone(), file.to_owned(), literal.to_owned()));
        } else {
            entries.push((section.clone(), line.to_owned(), "?".to_owned()));
        }
    }
    entries
}

#[test]
fn no_words_outside_the_catalog() {
    let allowed = allowed();
    let mut used = vec![false; allowed.len()];
    let mut failures = Vec::new();
    for name in CHECKED {
        for (file, scan) in crate_scans(name) {
            for (line, literal, raw) in &scan.literals {
                if !has_words(literal, *raw) {
                    continue;
                }
                let found = allowed
                    .iter()
                    .position(|(_, f, l)| (f == &file || f == "*") && (l == literal || l == "*"));
                match found.and_then(|i| used.get_mut(i)) {
                    Some(seen) => *seen = true,
                    None => failures.push(format!(
                        "{file}:{line}: words outside the catalog; move them into \
                         locales/en-US, or if never said or shown, list them in \
                         tests/literals.txt as\n{file}: \"{literal}\""
                    )),
                }
            }
        }
    }
    for ((section, file, literal), seen) in allowed.iter().zip(&used) {
        if !seen {
            failures.push(format!(
                "tests/literals.txt [{section}] lists {file}: \"{literal}\", which no longer \
                 exists: delete the line"
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}\n", failures.join("\n"));
}

#[test]
fn every_message_is_used() {
    let mut called = BTreeSet::new();
    let Ok(crates) = std::fs::read_dir(crates_dir()) else {
        return;
    };
    for entry in crates.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "xa11y" || !entry.path().join("src").is_dir() {
            continue;
        }
        for scan in crate_scans(&name).values() {
            called.extend(scan.messages.iter().cloned());
        }
    }
    let unused: Vec<String> = MessageId::ALL
        .iter()
        .filter(|id| !called.contains(id.constructor()))
        .map(|id| {
            format!(
                "{} ({}): no code calls msg::{}",
                id.id(),
                id.file(),
                id.constructor()
            )
        })
        .collect();
    assert!(unused.is_empty(), "\n{}\n", unused.join("\n"));
}

#[test]
fn the_scanner_sees_words_and_leaves_out_what_is_never_said() {
    let source = r##"
        // "a comment"
        /* "a block" /* "nested" */ */
        #[serde(rename = "renamed")]
        fn f(s: &mut impl Speak) {
            tracing::debug!("a log line {}", "with words");
            let c = '"';
            let e = '\'';
            let r = r#"raw "words" here"#;
            let b = b"bytes are not words";
            s.speak("Some new words");
            msg::hint_type_in_text();
        }
        #[cfg(test)]
        mod tests {
            fn t() { s.speak("test words"); }
        }
        #[cfg(test)]
        mod more_tests;
    "##;
    let scan = scan(source);
    let literals: Vec<&str> = scan.literals.iter().map(|(_, l, _)| l.as_str()).collect();
    assert_eq!(literals, ["raw \"words\" here", "Some new words"]);
    assert!(scan.messages.contains("hint_type_in_text"));
    assert_eq!(scan.test_modules, ["more_tests"]);
    assert!(has_words("Some new words", false));
    assert!(has_words("blank", false));
    assert!(!has_words("{name}  \\u{1d} {} \\u{1e} ", false));
    assert!(has_words("\\n{p} of", false));
    assert!(!has_words("snake_case", false));
    assert!(!has_words("Option::is_none", false));
    assert!(!has_words("{a}:{b}", false));
}

/// What is left of `said` without the pseudo-locale's wrapped messages and `page_words`, when
/// it still has words; `None` when only marks, spaces and punctuation are left.
fn unwrapped_words(said: &str, page_words: &[&str]) -> Option<String> {
    let mut rest = String::new();
    let mut depth = 0usize;
    for c in said.chars() {
        match c {
            '\u{27e6}' => depth += 1,
            '\u{27e7}' => depth = depth.saturating_sub(1),
            _ if depth == 0 => rest.push(c),
            _ => {}
        }
    }
    let mut words: Vec<&str> = page_words.to_vec();
    words.sort_by_key(|w| std::cmp::Reverse(w.len()));
    for word in words {
        rest = rest.replace(word, " ");
    }
    has_words(&rest, true).then_some(rest)
}

fn page() -> AomNode {
    let mut check = AomNode::new(9, Role::CheckBox).with_name("India");
    check.states.checked = Some(false);
    let mut combo = AomNode::new(11, Role::ComboBox)
        .with_name("Lima")
        .with_value("Mike");
    combo.states.expanded = Some(false);
    let item = |id, text: &str| {
        AomNode::new(id, Role::ListItem)
            .with_children(vec![AomNode::new(id + 100, Role::Text).with_text(text)])
    };
    let cell = |id, text: &str| {
        AomNode::new(id, Role::TableCell)
            .with_children(vec![AomNode::new(id + 100, Role::Text).with_text(text)])
    };
    AomNode::new(0, Role::Document)
        .with_name("Alpha")
        .with_children(vec![
            AomNode::new(1, Role::Heading { level: 2 }).with_text("Bravo"),
            AomNode::new(2, Role::Paragraph).with_text("Charlie delta"),
            AomNode::new(
                3,
                Role::Link {
                    href: Some("https://example.com/echo".to_owned()),
                },
            )
            .with_children(vec![AomNode::new(4, Role::Text).with_text("Echo")]),
            AomNode::new(5, Role::List).with_children(vec![item(6, "Foxtrot"), item(7, "Golf")]),
            AomNode::new(8, Role::Button).with_name("Hotel"),
            check,
            AomNode::new(10, Role::Edit)
                .with_name("Juliet")
                .with_value("Kilo"),
            combo,
            AomNode::new(12, Role::Table).with_children(vec![
                AomNode::new(13, Role::TableRow)
                    .with_children(vec![cell(14, "November"), cell(15, "Oscar")]),
                AomNode::new(16, Role::TableRow)
                    .with_children(vec![cell(17, "Papa"), cell(18, "Quebec")]),
            ]),
            AomNode::new(
                19,
                Role::Landmark {
                    landmark_type: "navigation".to_owned(),
                },
            )
            .with_name("Romeo")
            .with_children(vec![AomNode::new(20, Role::Paragraph).with_text("Sierra")]),
        ])
}

const PAGE_WORDS: &[&str] = &[
    "Alpha", "Bravo", "Charlie", "delta", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India",
    "Juliet", "Kilo", "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra",
    "Tango", "Uniform", "Victor", "Whiskey", "Xray", "Yankee", "Zulu", "Alt", "Ctrl",
];

/// Native controls taking the focus one after another, in one window.
fn native_focuses() -> Vec<FocusEvent> {
    let named = |role: Role, name: &str| {
        let mut focus = FocusEvent::new(role).with_name(name);
        focus.window_title = Some("Tango".to_owned());
        focus
    };
    let mut button = named(Role::Button, "Uniform");
    button.access_key = Some("Alt, v".to_owned());
    button.states.expanded = Some(false);
    let mut check = named(Role::CheckBox, "Victor");
    check.states.checked = Some(true);
    let mut radio = named(Role::RadioButton, "Whiskey");
    radio.item_index = Some(1);
    radio.item_count = Some(2);
    let mut combo = named(Role::ComboBox, "Xray");
    combo.value = Some("Yankee".to_owned());
    let mut item = named(Role::ListItem, "Zulu");
    item.container_role = Some(Role::List);
    item.container_name = Some("Alpha".to_owned());
    item.container_is_list_view = true;
    item.item_index = Some(3);
    item.item_count = Some(4);
    let mut tree = named(Role::TreeItem, "Bravo");
    tree.container_role = Some(Role::Tree);
    tree.states.expanded = Some(true);
    let mut slider = named(Role::Slider, "Charlie");
    slider.value = Some("50%".to_owned());
    let mut edit = named(Role::Edit, "Echo");
    edit.accelerator_key = Some("Ctrl+L".to_owned());
    vec![
        named(Role::Edit, "Delta"),
        button,
        check,
        radio,
        combo,
        item,
        tree,
        named(Role::Tab, "Foxtrot"),
        named(Role::SplitButton, "Golf"),
        named(Role::MenuButton, "Hotel"),
        named(Role::Link { href: None }, "India"),
        slider,
        edit,
    ]
}

/// Under the pseudo-locale every word Okur says itself is wrapped: the page's reading, quick
/// navigation, tables, the element lists and native focus speech hold no other word than the
/// page's and the controls' own.
#[test]
fn the_pseudo_locale_wraps_every_word_okur_says() {
    with_pseudo_locale(|| {
        let mut engine = ScreenReaderEngine::new(
            VirtualCursor::new(VirtualBuffer::from_aom(&page())),
            Box::new(MockSpeechOutput::new()),
        );
        let mut said = Vec::new();
        for command in [
            "SayAll",
            "TopOfFile",
            "MoveToNextHeading",
            "MoveToNextHeadingLevelOne",
            "MoveToNextLink",
            "MoveToNextList",
            "MoveToNextButton",
            "MoveToNextCheckbox",
            "MoveToNextEdit",
            "MoveToNextComboBox",
            "MoveToNextTable",
            "TableCellRight",
            "TableCellDown",
            "TableCellDown",
            "MoveToNextRegion",
            "MoveToNextFrame",
            "MoveToNextGraphic",
            "MoveToNextRadioButton",
            "MoveToNextPlaceMarker",
            "MoveToNextArticle",
            "SelectAHeading",
            "SelectALink",
            "SelectAFormField",
            "SelectARegion",
            "PassKeyThrough",
        ] {
            said.extend(engine.execute_command(command));
        }
        for focus in native_focuses() {
            said.extend(engine.handle_focus_event(&focus));
        }
        assert!(said.len() > 30, "{said:?}");
        let unwrapped: Vec<String> = said
            .iter()
            .filter_map(|s| {
                unwrapped_words(s, PAGE_WORDS).map(|rest| format!("{s:?} leaves {rest:?}"))
            })
            .collect();
        assert!(unwrapped.is_empty(), "\n{}\n", unwrapped.join("\n"));
    });
}
