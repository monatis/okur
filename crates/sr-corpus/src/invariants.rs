//! Universal virtual-buffer invariants.
//!
//! Properties every page must satisfy regardless of JAWS phrasing, checked through the
//! user-facing cursor API on real captured trees. They catch whole classes of reading bugs
//! (skipped text, duplicated lines, merged words, missing links) that a handful of recorded
//! JAWS steps from the top of a page never reach.

use crate::parity::normalize;
use sr_core::{AomNode, Role, VirtualBuffer, VirtualCursor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub invariant: &'static str,
    pub detail: String,
}

/// Upper bound on navigation steps, so a cursor that never reaches the end cannot hang a test.
const MAX_STEPS: usize = 100_000;

fn walk<'a>(node: &'a AomNode, out: &mut Vec<&'a AomNode>) {
    out.push(node);
    for child in &node.children {
        walk(child, out);
    }
}

/// Every non-empty `Text` node's text must be spoken when reading the page line by line
/// with Down Arrow from the top.
pub const TEXT_REACHABLE: &str = "all text is reachable with Down Arrow";
/// Buffer lines must be in document order and must not overlap.
pub const LINES_ORDERED: &str = "lines are in document order and do not overlap";
/// No line may be spoken twice when reading top to bottom.
pub const NO_REPEATED_LINES: &str = "Down Arrow never reads the same line twice";
/// Every word reached with Ctrl+Right must come from a single node's text.
pub const WORDS_WITHIN_NODES: &str = "words never merge text from different elements";
/// Every link in the tree must appear in the buffer's link list.
pub const LINKS_LISTED: &str = "every link is in the links list";

pub fn check(aom: &AomNode) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut nodes = Vec::new();
    walk(aom, &mut nodes);
    let buffer = VirtualBuffer::from_aom(aom);

    for pair in buffer.lines.windows(2) {
        if let [a, b] = pair {
            if b.start_offset < a.end_offset {
                violations.push(Violation {
                    invariant: LINES_ORDERED,
                    detail: format!(
                        "line {} {:?} [{}..{}) precedes line {} {:?} [{}..{})",
                        a.line_index,
                        a.text,
                        a.start_offset,
                        a.end_offset,
                        b.line_index,
                        b.text,
                        b.start_offset,
                        b.end_offset
                    ),
                });
            }
        }
    }

    let link_nodes = nodes
        .iter()
        .filter(|n| matches!(n.role, Role::Link { .. }))
        .count();
    if link_nodes != buffer.links.len() {
        violations.push(Violation {
            invariant: LINKS_LISTED,
            detail: format!(
                "{link_nodes} links in the tree, {} in the links list",
                buffer.links.len()
            ),
        });
    }

    // Read the page with Down Arrow from the top.
    let mut cursor = VirtualCursor::new(buffer.clone());
    let mut spoken_lines = vec![normalize(&cursor.move_to_start())];
    for _ in 0..MAX_STEPS {
        match cursor.move_next_line() {
            Some(line) => spoken_lines.push(normalize(&line)),
            None => break,
        }
    }
    let spoken = spoken_lines.join(" ");
    for node in nodes.iter().filter(|n| n.role == Role::Text) {
        let text = normalize(node.text.as_deref().unwrap_or_default());
        if !text.is_empty() && !spoken.contains(&text) {
            violations.push(Violation {
                invariant: TEXT_REACHABLE,
                detail: format!("never spoken: {text:?}"),
            });
        }
    }
    let mut seen = std::collections::HashSet::new();
    for line in spoken_lines.iter().filter(|l| !l.is_empty()) {
        if !seen.insert(line.as_str()) {
            violations.push(Violation {
                invariant: NO_REPEATED_LINES,
                detail: format!("spoken twice: {line:?}"),
            });
        }
    }

    // Read the page word by word with Ctrl+Right from the top.
    let sources: Vec<String> = nodes
        .iter()
        .flat_map(|n| [n.text.as_deref(), n.name.as_deref(), n.value.as_deref()])
        .flatten()
        .map(str::to_owned)
        .collect();
    let mut cursor = VirtualCursor::new(buffer);
    let _ = cursor.move_to_start();
    for _ in 0..MAX_STEPS {
        let Some(word) = cursor.move_next_word() else {
            break;
        };
        let word = normalize(&word);
        if !word.is_empty() && !sources.iter().any(|s| s.contains(&word)) {
            violations.push(Violation {
                invariant: WORDS_WITHIN_NODES,
                detail: format!("word {word:?} spans elements"),
            });
        }
    }

    violations
}

pub const BASELINE_FILE: &str = "invariants-baseline.json";

/// Violations per snapshot file name, for every snapshot in `corpus_dir/snapshots`.
pub fn run_all(
    corpus_dir: &std::path::Path,
) -> Result<std::collections::BTreeMap<String, Vec<String>>, String> {
    let mut paths: Vec<_> = std::fs::read_dir(corpus_dir.join("snapshots"))
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    paths.sort();
    let mut results = std::collections::BTreeMap::new();
    for path in paths {
        let snapshot = sr_platform::uia_snapshot::UiaSnapshot::load(&path)?;
        let aom = sr_platform::uia_snapshot::to_aom(&snapshot.root);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let violations = check(&aom)
            .into_iter()
            .map(|v| format!("[{}] {}", v.invariant, v.detail))
            .collect();
        results.insert(name, violations);
    }
    Ok(results)
}

pub fn render_baseline(
    results: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<String, String> {
    let total: usize = results.values().map(Vec::len).sum();
    let doc = serde_json::json!({
        "notice": "GENERATED FILE - do not edit. Regenerate: cargo run -p sr-cli -- invariants --write-baseline",
        "total_violations": total,
        "violations": results,
    });
    serde_json::to_string_pretty(&doc)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

/// Fails on any violation not in the committed baseline, and whenever the baseline is stale.
pub fn check_baseline(
    corpus_dir: &std::path::Path,
    results: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    let committed_text =
        std::fs::read_to_string(corpus_dir.join(BASELINE_FILE)).unwrap_or_default();
    let committed: serde_json::Value =
        serde_json::from_str(&committed_text).unwrap_or(serde_json::Value::Null);
    let mut new_violations = Vec::new();
    for (snapshot, violations) in results {
        let known: Vec<&str> = committed
            .pointer(&format!("/violations/{snapshot}"))
            .and_then(serde_json::Value::as_array)
            .map(|a| a.iter().filter_map(serde_json::Value::as_str).collect())
            .unwrap_or_default();
        for v in violations {
            if !known.contains(&v.as_str()) {
                new_violations.push(format!("{snapshot}: {v}"));
            }
        }
    }
    if !new_violations.is_empty() {
        return Err(format!(
            "new buffer invariant violations:\n{}",
            new_violations.join("\n")
        ));
    }
    if committed_text.replace("\r\n", "\n") != render_baseline(results)? {
        return Err(format!(
            "{BASELINE_FILE} is stale (violations were fixed). Regenerate with: \
             cargo run -p sr-cli -- invariants --write-baseline"
        ));
    }
    Ok(())
}

pub fn write_baseline(
    corpus_dir: &std::path::Path,
    results: &std::collections::BTreeMap<String, Vec<String>>,
) -> Result<(), String> {
    std::fs::write(corpus_dir.join(BASELINE_FILE), render_baseline(results)?)
        .map_err(|e| e.to_string())
}
