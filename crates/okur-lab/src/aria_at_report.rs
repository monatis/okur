//! The measure of ARIA-AT's assertions: for each imported test plan, how many of its
//! assertions ("convey role 'radio button'", "Name 'Top' is conveyed") Okur's speech conveys,
//! beside how many JAWS's recorded speech conveys, both judged the same way: by the words JAWS
//! uses for a role or a state, the name as written, a place as "2 of 3".
//!
//! The judge is mechanical and is reviewed by hand before a figure is trusted: an assertion it
//! cannot read (a list's boundary, "the ability to edit text") is left unjudged, never counted
//! as met.

use okur_corpus::parity::StepParity;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// What an assertion asks to be conveyed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Role,
    /// A name or a text, said as written.
    Name,
    State,
    /// The item's place, "2 of 3".
    Position,
    /// How many there are, "of 3", "5 columns", "with 4 items".
    Count,
    /// A cell's row, "row 2".
    RowNumber,
    /// A cell's column, "column 1".
    ColumnNumber,
    HeadingLevel,
}

/// An assertion as the judge reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Claim {
    pub kind: Kind,
    pub value: String,
    /// "not convey numeric value": met when the value is not said.
    pub negated: bool,
    /// "convey some or all the answer text": met by its first words.
    pub partial: bool,
}

/// Reads an assertion's phrase, in either of ARIA-AT's formats ("convey state of the radio
/// button, 'checked'", "State of the combobox (collapsed) is conveyed"). None for an assertion
/// the judge cannot read.
pub fn claim(phrase: &str) -> Option<Claim> {
    let lower = phrase.to_lowercase();
    let (value, outside) = quoted_value(&lower)?;
    let negated = lower.starts_with("not convey") || lower.contains(" not conveyed");
    let has = |word: &str| outside.contains(word);
    let kind = if has("heading level") {
        Kind::HeadingLevel
    } else if has("role") {
        Kind::Role
    } else if has("position") {
        Kind::Position
    } else if has("row number") {
        Kind::RowNumber
    } else if has("column number") {
        Kind::ColumnNumber
    } else if has("number of") {
        Kind::Count
    } else if has("state") {
        Kind::State
    } else if has("name") || has("text") || has("value") {
        Kind::Name
    } else {
        return None;
    };
    Some(Claim {
        kind,
        value: value.trim_start_matches("to ").trim().to_owned(),
        negated,
        partial: outside.contains("some or all"),
    })
}

/// The value an assertion names, and its phrase without it: in quotes ("'checked'"), in
/// parentheses ("(collapsed)", "('State')"), or a number or word after the last comma ("convey
/// position of the radio button, 1", "partially checked'").
fn quoted_value(phrase: &str) -> Option<(String, String)> {
    let strip = |v: &str| v.trim().trim_matches(['\'', '"']).trim().to_owned();
    if let (Some(open), Some(close)) = (phrase.find('('), phrase.rfind(')')) {
        if open < close {
            let value = phrase.get(open + 1..close)?;
            let outside = format!("{}{}", phrase.get(..open)?, phrase.get(close + 1..)?);
            return Some((strip(value), outside)).filter(|(v, _)| !v.is_empty());
        }
    }
    // A lone quote ("partially checked'") leaves the value to the comma.
    if let Some(open) = phrase.find('\'') {
        let rest = phrase.get(open + 1..)?;
        let close = rest.find('\'').unwrap_or(rest.len());
        let value = strip(rest.get(..close)?);
        if !value.is_empty() {
            return Some((value, phrase.get(..open)?.to_owned()));
        }
    }
    let comma = phrase.rfind(',')?;
    let value = phrase.get(comma + 1..)?;
    let outside = phrase.get(..comma)?.to_owned();
    Some((strip(value), outside)).filter(|(v, _)| !v.is_empty())
}

/// Speech as the judge reads it: lower case letters and digits, an apostrophe left out
/// ("can't" is ARIA-AT's "cant"), every other mark a space.
pub fn plain(speech: &str) -> String {
    let spaced: String = speech
        .to_lowercase()
        .chars()
        .filter(|c| !matches!(c, '\'' | '’'))
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    format!(
        " {} ",
        spaced.split_whitespace().collect::<Vec<_>>().join(" ")
    )
}

/// The words JAWS uses for an ARIA-AT role name.
fn role_words(role: &str) -> Vec<&'static str> {
    match role {
        "group" => vec!["group"],
        "checkbox" => vec!["check box"],
        "button" => vec!["button"],
        "radio button" => vec!["radio button"],
        "link" => vec!["link"],
        "heading" => vec!["heading"],
        "region" => vec!["region"],
        "toggle button" => vec!["toggle button"],
        "switch" => vec!["switch"],
        "alert" => vec!["alert"],
        "tab list" => vec!["tab list"],
        "tab" => vec!["tab"],
        "tab panel" => vec!["tab panel", "tabpanel"],
        "menu button" => vec!["menu button", "button menu"],
        "menu" => vec!["menu"],
        "menu item" => vec!["menu item"],
        "dialog" => vec!["dialog"],
        "grid" => vec!["grid", "table"],
        "spin button" => vec!["spin button", "spin box"],
        "slider" => vec!["slider"],
        "banner" => vec!["banner"],
        "image" => vec!["graphic", "image"],
        "navigation" => vec!["navigation"],
        "list" => vec!["list"],
        "combobox" => vec!["combo box", "edit combo"],
        "listbox" => vec!["list box"],
        "complementary" => vec!["complementary"],
        "contentinfo" => vec!["content info", "contentinfo"],
        "form" => vec!["form"],
        "main" => vec!["main"],
        "meter" => vec!["meter"],
        _ => Vec::new(),
    }
}

/// Whether `speech` (as [`plain`] made it) says the word or words `word` whole, not as part of
/// a longer role or state: "button" is not in "radio button", "checked" not in "not checked".
fn says(speech: &str, word: &str) -> bool {
    const LONGER: [&str; 5] = ["radio ", "toggle ", "spin ", "not ", "partially "];
    let needle = format!(" {word} ");
    let mut from = 0;
    while let Some(at) = speech.get(from..).and_then(|s| s.find(&needle)) {
        let start = from + at;
        let before = speech.get(..start + 1).unwrap_or_default();
        let part_of_longer = LONGER
            .iter()
            .any(|p| before.ends_with(&format!(" {p}")) && !word.starts_with(p.trim()));
        if !part_of_longer {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Whether `speech` (as [`plain`] made it) conveys the claim; None where the judge has no words
/// for it (an unknown role).
pub fn conveys(claim: &Claim, speech: &str) -> Option<bool> {
    let value = claim.value.as_str();
    let met = match claim.kind {
        Kind::Role => {
            let words = role_words(value);
            if words.is_empty() {
                return None;
            }
            words.iter().any(|w| says(speech, w))
        }
        // "Some or all" of a long text: its first words.
        Kind::Name if claim.partial => {
            let words: Vec<String> = plain(value).split_whitespace().map(str::to_owned).collect();
            let start = words
                .get(..words.len().min(6))
                .unwrap_or_default()
                .join(" ");
            speech.contains(&format!(" {start} "))
        }
        Kind::Name => speech.contains(&format!(" {} ", plain(value).trim())),
        Kind::State => match value {
            "unchecked" | "not checked" => says(speech, "not checked"),
            "not valid" => says(speech, "invalid") || says(speech, "not valid"),
            state => says(speech, state),
        },
        Kind::Position => speech.contains(&format!(" {value} of ")),
        Kind::Count => [
            " of {v} ",
            " {v} columns ",
            " {v} rows ",
            " {v} items ",
            " with {v} ",
        ]
        .iter()
        .any(|pattern| speech.contains(&pattern.replace("{v}", value))),
        Kind::RowNumber => speech.contains(&format!(" row {value} ")),
        Kind::ColumnNumber => speech.contains(&format!(" column {value} ")),
        Kind::HeadingLevel => speech.contains(&format!(" level {value} ")),
    };
    Some(met != claim.negated)
}

/// One plan's figures.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PlanFigures {
    pub assertions: usize,
    pub judged: usize,
    pub okur: usize,
    pub jaws: usize,
    pub okur_only: usize,
    pub jaws_only: usize,
    /// The judged MUST assertions (ARIA-AT's priority 1; 2 is SHOULD, 3 MAY), and how many of
    /// them each conveys.
    pub must: usize,
    pub must_okur: usize,
    pub must_jaws: usize,
}

/// An assertion Okur's speech does not convey, for the detail listing.
#[derive(Debug, Clone)]
pub struct Unmet {
    pub scenario: String,
    pub phrase: String,
    pub priority: String,
    pub jaws_conveys: bool,
    pub okur: String,
}

/// The measure over a corpus: per plan, and the assertions Okur leaves unmet.
pub struct Measure {
    pub plans: BTreeMap<String, PlanFigures>,
    pub unmet: Vec<(String, Unmet)>,
}

/// Judges every ARIA-AT scenario of the corpus with recorded evidence: the command's steps (a
/// PC cursor scenario's closing Insert+Z left out) joined, Okur's speech from `steps` against
/// JAWS's first recorded transcript of each.
pub fn measure(corpus: &Path, steps: &[StepParity]) -> Result<Measure, String> {
    let mut by_scenario: BTreeMap<&str, Vec<&StepParity>> = BTreeMap::new();
    for step in steps {
        if let Some((scenario, _)) = step.key.split_once('/') {
            by_scenario.entry(scenario).or_default().push(step);
        }
    }
    let mut plans: BTreeMap<String, PlanFigures> = BTreeMap::new();
    let mut unmet = Vec::new();
    let dir = corpus.join("scenarios");
    let entries = std::fs::read_dir(&dir).map_err(|e| format!("{dir:?}: {e}"))?;
    let mut files: Vec<_> = entries.filter_map(Result::ok).map(|e| e.path()).collect();
    files.sort();
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !(name.contains("-aria-") && name.ends_with(".json")) {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{path:?}: {e}"))?;
        let scenario: Value = serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{path:?}: {e}"))?;
        let Some(meta) = scenario.get("aria_at") else {
            continue;
        };
        let id = scenario
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Some(recorded) = by_scenario.get(id) else {
            continue;
        };
        let pc = meta.get("setting").and_then(Value::as_str) == Some("pcCursor");
        let command: Vec<&&StepParity> = recorded
            .iter()
            .enumerate()
            .filter(|(i, s)| !(pc && *i + 1 == recorded.len() && s.command == "Insert+Z"))
            .map(|(_, s)| s)
            .collect();
        let okur = plain(
            &command
                .iter()
                .map(|s| s.engine.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let jaws = plain(
            &command
                .iter()
                .map(|s| {
                    s.jaws_variants
                        .first()
                        .map(String::as_str)
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );
        let plan = meta
            .get("plan")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let figures = plans.entry(plan.clone()).or_default();
        let assertions = meta
            .get("assertions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for assertion in assertions {
            let phrase = assertion
                .get("phrase")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if phrase.trim().is_empty() {
                continue;
            }
            figures.assertions += 1;
            let Some(claim) = claim(phrase) else {
                continue;
            };
            let (Some(by_okur), Some(by_jaws)) = (conveys(&claim, &okur), conveys(&claim, &jaws))
            else {
                continue;
            };
            figures.judged += 1;
            figures.okur += usize::from(by_okur);
            figures.jaws += usize::from(by_jaws);
            figures.okur_only += usize::from(by_okur && !by_jaws);
            figures.jaws_only += usize::from(by_jaws && !by_okur);
            if assertion.get("priority").and_then(Value::as_str) == Some("1") {
                figures.must += 1;
                figures.must_okur += usize::from(by_okur);
                figures.must_jaws += usize::from(by_jaws);
            }
            if !by_okur {
                unmet.push((
                    plan.clone(),
                    Unmet {
                        scenario: id.to_owned(),
                        phrase: phrase.to_owned(),
                        priority: assertion
                            .get("priority")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        jaws_conveys: by_jaws,
                        okur: okur.trim().to_owned(),
                    },
                ));
            }
        }
    }
    Ok(Measure { plans, unmet })
}

/// The measure as a table, a line per plan and the total.
pub fn render(measure: &Measure) -> String {
    let mut out = String::from(
        "ARIA-AT assertions conveyed, judged by JAWS's words for each role, name and state\n\
         plan                                      assertions  judged    Okur    JAWS  Okur only  JAWS only    MUST  Okur  JAWS\n",
    );
    let mut total = PlanFigures::default();
    let mut line = |name: &str, f: &PlanFigures| {
        out.push_str(&format!(
            "{name:<42}{:>10}{:>8}{:>8}{:>8}{:>11}{:>11}{:>8}{:>6}{:>6}\n",
            f.assertions,
            f.judged,
            f.okur,
            f.jaws,
            f.okur_only,
            f.jaws_only,
            f.must,
            f.must_okur,
            f.must_jaws
        ));
    };
    for (plan, f) in &measure.plans {
        line(plan, f);
        total.assertions += f.assertions;
        total.judged += f.judged;
        total.okur += f.okur;
        total.jaws += f.jaws;
        total.okur_only += f.okur_only;
        total.jaws_only += f.jaws_only;
        total.must += f.must;
        total.must_okur += f.must_okur;
        total.must_jaws += f.must_jaws;
    }
    line("total", &total);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_formats_phrases_are_read() {
        let c = |p: &str| claim(p).map(|c| (c.kind, c.value, c.negated));
        assert_eq!(
            c("convey state of the radio button, 'checked'"),
            Some((Kind::State, "checked".to_owned(), false))
        );
        assert_eq!(
            c("Name of the combobox ('State') is conveyed"),
            Some((Kind::Name, "state".to_owned(), false))
        );
        assert_eq!(
            c("convey position of the radio button, 1"),
            Some((Kind::Position, "1".to_owned(), false))
        );
        assert_eq!(
            c("convey state of the checkbox, partially checked'"),
            Some((Kind::State, "partially checked".to_owned(), false))
        );
        assert_eq!(
            c("not convey numeric value, '3'"),
            Some((Kind::Name, "3".to_owned(), true))
        );
        assert_eq!(
            c("Change in state (to expanded) is conveyed"),
            Some((Kind::State, "expanded".to_owned(), false))
        );
        assert_eq!(c("convey list boundary"), None);
    }

    #[test]
    fn a_role_or_state_is_heard_whole() {
        let speech = plain("Pizza Crust \u{1d} group \u{1e}\nRegular crust \u{1d} radio button \u{1e} \u{1d}not checked\u{1e}\n1 of 3");
        let judge = |kind, value: &str| {
            conveys(
                &Claim {
                    kind,
                    value: value.to_owned(),
                    negated: false,
                    partial: false,
                },
                &speech,
            )
        };
        assert_eq!(judge(Kind::Role, "radio button"), Some(true));
        assert_eq!(judge(Kind::Role, "button"), Some(false));
        assert_eq!(judge(Kind::Role, "group"), Some(true));
        assert_eq!(judge(Kind::State, "unchecked"), Some(true));
        assert_eq!(judge(Kind::State, "checked"), Some(false));
        assert_eq!(judge(Kind::Name, "Regular crust"), Some(true));
        assert_eq!(judge(Kind::Position, "1"), Some(true));
        assert_eq!(judge(Kind::Count, "3"), Some(true));
        let grid = plain("grid with 5 columns and 7 rows | Date | column 1 row 2");
        let on_grid = |kind, value: &str| {
            conveys(
                &Claim {
                    kind,
                    value: value.to_owned(),
                    negated: false,
                    partial: false,
                },
                &grid,
            )
        };
        assert_eq!(on_grid(Kind::Count, "5"), Some(true));
        assert_eq!(on_grid(Kind::RowNumber, "2"), Some(true));
        assert_eq!(on_grid(Kind::ColumnNumber, "2"), Some(false));
        assert_eq!(
            claim("convey row number of the cell, '1'").map(|c| c.kind),
            Some(Kind::RowNumber)
        );
        assert_eq!(judge(Kind::Role, "no such role"), None);
    }
}
