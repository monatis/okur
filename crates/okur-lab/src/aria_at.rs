//! `okur-lab aria-at-import`: one of W3C ARIA-AT's test plans
//! (<https://github.com/w3c/aria-at>, `tests/<plan>/`) made into pending scenarios for the
//! reference machine (plan `docs/plans/2026-10-31-aria-at-web-round-1.md`).
//!
//! A plan's test page is copied as it is, with its scripts and styles, into
//! `<out>/aria-at/<plan>/`, which goes to `corpus/fixtures/aria-at/<plan>/` when recorded. Each
//! of its tests has a page per setup script (`checkbox-mixed.moveFocusBeforeFirstCheckbox.html`)
//! whose "Run Test Setup" button, the page's last element, runs the setup when activated, as
//! ARIA-AT's testers do. Each JAWS command of a test (`data/jaws-commands.csv`) becomes a
//! scenario: its reset goes to the button (Ctrl+End) and presses it (Enter), then turns the
//! virtual cursor off (Insert+Z) for a command in the PC cursor; its steps are the command's
//! keys, one per step ("down down" is two), and Insert+Z after a PC cursor's command turns the
//! virtual cursor on again for the scenarios after it. The test's assertions go with the
//! scenario (`aria_at`), for the measure of what is conveyed.
//!
//! Only the plans in ARIA-AT's second format (`jaws-commands.csv`, `assertions.csv`) are read.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Where the reference machine keeps the corpus's fixtures, as a scenario names its page.
const LAB_FIXTURES: &str = "file:///C:/Users/ailabs/corpus/fixtures/aria-at";

/// What an import wrote.
pub struct Imported {
    pub scenarios: usize,
    pub files: usize,
    pub last_id: u32,
}

/// One CSV file's rows, each by its header's names.
fn rows(path: &Path) -> Result<Vec<BTreeMap<String, String>>, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|e| format!("{path:?}: {e}"))?;
    let headers = reader
        .headers()
        .map_err(|e| format!("{path:?}: {e}"))?
        .clone();
    reader
        .records()
        .map(|record| {
            let record = record.map_err(|e| format!("{path:?}: {e}"))?;
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
                .collect())
        })
        .collect()
}

fn field<'a>(row: &'a BTreeMap<String, String>, name: &str) -> &'a str {
    row.get(name).map(String::as_str).unwrap_or_default()
}

/// One of ARIA-AT's key tokens ("ins", "down", "pageUp", "x", "nine") as the recorder names
/// the key ("Insert", "Down", "PageUp", "X", "9").
fn key_name(token: &str) -> Option<String> {
    let named = match token {
        "ins" => "Insert",
        "shift" => "Shift",
        "ctrl" => "Ctrl",
        "alt" => "Alt",
        "win" => "Win",
        "up" => "Up",
        "down" => "Down",
        "left" => "Left",
        "right" => "Right",
        "home" => "Home",
        "end" => "End",
        "pageUp" => "PageUp",
        "pageDown" => "PageDown",
        "tab" => "Tab",
        "enter" => "Enter",
        "esc" => "Escape",
        "space" => "Space",
        "backspace" => "Backspace",
        "del" => "Delete",
        "zero" | "cero" => "0",
        "one" => "1",
        "two" => "2",
        "three" => "3",
        "four" => "4",
        "five" => "5",
        "six" => "6",
        "seven" => "7",
        "eight" => "8",
        "nine" => "9",
        _ => "",
    };
    if !named.is_empty() {
        return Some(named.to_owned());
    }
    let mut chars = token.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) if c.is_ascii_alphanumeric() => Some(c.to_ascii_uppercase().to_string()),
        _ => {
            let n = token.strip_prefix('f')?.parse::<u8>().ok()?;
            (1..=12).contains(&n).then(|| format!("F{n}"))
        }
    }
}

/// A command ("ins+up", "down down") as the recorder's chords, one per key press.
fn chords(command: &str) -> Result<Vec<String>, String> {
    command
        .split_whitespace()
        .map(|press| {
            press
                .split('+')
                .map(|token| {
                    key_name(token).ok_or_else(|| format!("unknown key {token:?} in {command:?}"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|keys| keys.join("+"))
        })
        .collect()
}

/// "navForwardsToMixedCheckbox" as "nav-forwards-to-mixed-checkbox".
fn kebab(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_uppercase() && !out.is_empty() {
            out.push('-');
        }
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// Copies the folder `from` into `to`, every file and folder in it; returns how many files.
fn copy_folder(from: &Path, to: &Path) -> Result<usize, String> {
    fs::create_dir_all(to).map_err(|e| format!("{to:?}: {e}"))?;
    let mut files = 0;
    for entry in fs::read_dir(from).map_err(|e| format!("{from:?}: {e}"))? {
        let path = entry.map_err(|e| format!("{from:?}: {e}"))?.path();
        let Some(name) = path.file_name() else {
            continue;
        };
        let target = to.join(name);
        if path.is_dir() {
            files += copy_folder(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("{path:?}: {e}"))?;
            files += 1;
        }
    }
    Ok(files)
}

/// The notice the copied pages carry, as the W3C's licenses ask.
fn notice(source: &str) -> String {
    format!(
        "# ARIA-AT's test pages\n\n\
         The folders here are copies of test plans' pages from W3C's ARIA and Assistive \
         Technologies project, <https://github.com/w3c/aria-at>, at commit `{source}`: each \
         plan's `tests/apg/<plan>/reference/<date>/` folder, unchanged.\n\n\
         Copyright (c) World Wide Web Consortium. The ARIA-AT repository's documents are \
         licensed under the W3C Document License, \
         <https://www.w3.org/copyright/document-license-2023/>; the pages are the ARIA \
         Authoring Practices Guide's examples, <https://github.com/w3c/aria-practices>, under \
         the W3C Software and Document License, \
         <https://www.w3.org/copyright/software-license-2023/>.\n\n\
         They are used here to record how a screen reader reads them, as ARIA-AT's own \
         testers do (`crates/okur-lab/src/aria_at.rs`).\n"
    )
}

/// Imports the test plan `plan` ("apg/checkbox-tri-state") of the ARIA-AT `tests` folder
/// `tests` into `out`, its scenarios numbered from `first_id`; `source` is the checkout's
/// commit.
pub fn import(
    tests: &Path,
    plan: &str,
    first_id: u32,
    source: &str,
    out: &Path,
) -> Result<Imported, String> {
    let plan_dir = tests.join(plan);
    let data = plan_dir.join("data");
    let name = plan.rsplit('/').next().unwrap_or(plan);
    let references: BTreeMap<String, String> = rows(&data.join("references.csv"))?
        .into_iter()
        .map(|row| {
            (
                field(&row, "refId").to_owned(),
                field(&row, "value").to_owned(),
            )
        })
        .collect();
    let reference = references
        .get("reference")
        .ok_or_else(|| format!("{plan}: no reference page in references.csv"))?;
    let title = references.get("title").cloned().unwrap_or_default();
    let reference = Path::new(reference);
    let page_stem = reference
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("{plan}: reference {reference:?} names no page"))?;
    let reference_dir = plan_dir.join(reference.parent().unwrap_or(Path::new("")));

    let commands_path = data.join("jaws-commands.csv");
    if !commands_path.exists() {
        return Err(format!(
            "{plan}: no jaws-commands.csv (ARIA-AT's first format, not read yet)"
        ));
    }
    let tests_by_id: BTreeMap<String, BTreeMap<String, String>> = rows(&data.join("tests.csv"))?
        .into_iter()
        .map(|row| (field(&row, "testId").to_owned(), row))
        .collect();
    let assertions: BTreeMap<String, BTreeMap<String, String>> =
        rows(&data.join("assertions.csv"))?
            .into_iter()
            .map(|row| (field(&row, "assertionId").to_owned(), row))
            .collect();

    let pages = out.join("aria-at").join(name);
    let files = copy_folder(&reference_dir, &pages)?;
    fs::write(out.join("aria-at").join("NOTICE.md"), notice(source))
        .map_err(|e| format!("NOTICE.md: {e}"))?;

    let mut id = first_id;
    let mut scenarios = 0;
    for command in rows(&commands_path)? {
        let test_id = field(&command, "testId");
        let test = tests_by_id
            .get(test_id)
            .ok_or_else(|| format!("{plan}: command for unknown test {test_id:?}"))?;
        let setup = field(test, "setupScript");
        let page = if setup.is_empty() {
            format!("{page_stem}.html")
        } else {
            format!("{page_stem}.{setup}.html")
        };
        if !pages.join(&page).exists() {
            return Err(format!("{plan}: {test_id} has no page {page:?}"));
        }
        let keys = field(&command, "command");
        let setting = field(&command, "settings");
        let mut steps = chords(keys)?;
        let mut reset = if setup.is_empty() {
            vec!["Ctrl+Home".to_owned()]
        } else {
            vec!["Ctrl+End".to_owned(), "Enter".to_owned()]
        };
        let cursor = match setting {
            "virtualCursor" => "vc",
            "pcCursor" => {
                reset.push("Insert+Z".to_owned());
                steps.push("Insert+Z".to_owned());
                "pc"
            }
            other => return Err(format!("{plan}: {test_id}: unknown setting {other:?}")),
        };
        let scenario_id = format!(
            "{id}-aria-{name}-{}-{}-{cursor}",
            kebab(test_id),
            kebab(&keys.replace('+', " "))
        );
        let asserted: Vec<serde_json::Value> = field(test, "assertions")
            .split_whitespace()
            .map(|assertion| {
                let row = assertions.get(assertion);
                serde_json::json!({
                    "id": assertion,
                    "priority": row.map(|r| field(r, "priority")).unwrap_or_default(),
                    "phrase": row.map(|r| field(r, "assertionPhrase")).unwrap_or_default(),
                })
            })
            .collect();
        let cursor_name = if cursor == "pc" {
            "the PC cursor"
        } else {
            "the virtual cursor"
        };
        let scenario = serde_json::json!({
            "id": scenario_id,
            "name": format!(
                "ARIA-AT, {title}: {}; {} in {cursor_name}",
                field(test, "title"),
                steps.first().map(String::as_str).unwrap_or_default()
            ),
            "category": "aria-at",
            "version": "1.0.0",
            "fixture": format!("{LAB_FIXTURES}/{name}/{page}"),
            "target_app": "chrome",
            "reset": { "keys": reset },
            "steps": steps
                .iter()
                .enumerate()
                .map(|(at, chord)| serde_json::json!({
                    "id": format!("s{}", at + 1),
                    "type": "key",
                    "command": chord,
                }))
                .collect::<Vec<_>>(),
            "aria_at": {
                "source": format!("w3c/aria-at@{source}"),
                "plan": plan,
                "test": test_id,
                "instructions": field(test, "instructions"),
                "command": keys,
                "setting": setting,
                "presentation_number": field(&command, "presentationNumber"),
                "assertions": asserted,
                "assertion_exceptions": field(&command, "assertionExceptions"),
            },
        });
        let text = serde_json::to_string_pretty(&scenario).map_err(|e| e.to_string())?;
        let path: PathBuf = out.join(format!("{scenario_id}.json"));
        fs::write(&path, format!("{text}\n")).map_err(|e| format!("{path:?}: {e}"))?;
        scenarios += 1;
        id += 1;
    }
    Ok(Imported {
        scenarios,
        files,
        last_id: id.saturating_sub(1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ARIA-AT's commands as the recorder's chords [`tests/commands.json`'s names].
    #[test]
    fn commands_become_the_recorders_chords() {
        assert_eq!(chords("ins+up"), Ok(vec!["Insert+Up".to_owned()]));
        assert_eq!(
            chords("down down"),
            Ok(vec!["Down".to_owned(), "Down".to_owned()])
        );
        assert_eq!(chords("shift+x"), Ok(vec!["Shift+X".to_owned()]));
        assert_eq!(
            chords("ctrl+alt+pageDown"),
            Ok(vec!["Ctrl+Alt+PageDown".to_owned()])
        );
        assert_eq!(chords("nine"), Ok(vec!["9".to_owned()]));
        assert_eq!(chords("f12"), Ok(vec!["F12".to_owned()]));
        assert!(chords("ins+bogus").is_err());
    }

    #[test]
    fn test_ids_become_kebab_case() {
        assert_eq!(
            kebab("navForwardsToMixedCheckbox"),
            "nav-forwards-to-mixed-checkbox"
        );
        assert_eq!(kebab("ins up"), "ins-up");
    }
}
