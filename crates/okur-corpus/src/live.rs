//! Okur's own speech in a live run on the reference machine, compared with JAWS's evidence for
//! the same scenario.
//!
//! Replay drives the engine with recorded events, so it cannot measure what Okur draws and
//! reads itself: its dialogs (the Links List), which only exist when Okur runs. A live run
//! sends the scenario's keys to Okur, with JAWS stopped, through the same runner
//! (`run-scenario.ps1 -Reader okur`). Its file has the evidence format, with
//! `provenance.reader` = `"okur"` and each step's `speech_raw` taken from Okur's speech log.
//! It is never evidence: it lives outside `corpus/evidence/`, and evidence is never read as a
//! run.

use crate::divergences::{compare as compare_step, Comparison};
use crate::parity::{jaws_spoken_variants, jaws_variants, step_spoken, SpokenParity};
use serde_json::Value;

/// One step of a live run against JAWS's transcripts for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStep {
    /// `<scenario_id>/<step_id>`.
    pub key: String,
    pub command: String,
    /// What Okur said, as its speech log has it.
    pub okur: String,
    /// Distinct transcripts JAWS produced for this step across successful trials.
    pub jaws_variants: Vec<String>,
    pub matched: bool,
    /// An accepted divergence covers the whole step: not counted.
    pub accepted: bool,
    /// Whether the runner heard Okur fall silent before the step's time ran out. An
    /// unsettled step may have been cut short.
    pub settled: bool,
    /// What Okur sent its synthesizer against what JAWS sent its own, when both were recorded
    /// through the speech recorder voice.
    pub spoken: Option<SpokenParity>,
}

/// What a live run is, by its provenance.
fn reader(file: &Value) -> Option<&str> {
    file.pointer("/provenance/reader").and_then(Value::as_str)
}

/// Compares the first successful trial of Okur's live `run` with JAWS's `evidence` of the same
/// scenario, step by step, as `okur-lab parity` compares (whitespace normalized, any recorded
/// variant). A run without one is compared by its first trial: a step that did not settle (a
/// long reading at Okur's slower rate) fails the trial but still says what Okur said.
pub fn compare(run: &Value, evidence: &Value) -> Result<Vec<LiveStep>, String> {
    if reader(run) != Some("okur") {
        return Err("the run is not Okur's (provenance.reader is not \"okur\")".to_owned());
    }
    if reader(evidence).is_some_and(|r| r != "jaws") {
        return Err("the evidence is not JAWS's".to_owned());
    }
    let scenario = run
        .get("scenario_id")
        .and_then(Value::as_str)
        .ok_or("the run has no scenario_id")?;
    if evidence.get("scenario_id").and_then(Value::as_str) != Some(scenario) {
        return Err(format!("the evidence is not of scenario {scenario}"));
    }
    let trial = run
        .get("trials")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|t| t.get("trial_success").and_then(Value::as_bool) == Some(true))
        .or_else(|| run.pointer("/trials/0"))
        .ok_or("the run has no trial")?;
    let variants = jaws_variants(evidence);
    let spoken_variants = jaws_spoken_variants(evidence);
    let steps = trial
        .get("step_results")
        .and_then(Value::as_array)
        .ok_or("the run's trial has no steps")?;
    steps
        .iter()
        .map(|step| {
            let id = step
                .get("step_id")
                .and_then(Value::as_str)
                .ok_or("a step has no step_id")?;
            let okur = step
                .pointer("/observation/speech_raw")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let jaws = variants.get(id).cloned().unwrap_or_default();
            if jaws.is_empty() {
                return Err(format!("no JAWS transcript recorded for step {id}"));
            }
            let key = format!("{scenario}/{id}");
            let comparison = compare_step(&key, &okur, &jaws);
            let recorded = step
                .pointer("/observation/spoken")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty());
            let spoken = spoken_variants.as_ref().filter(|_| recorded).map(|all| {
                let jaws_variants = all.get(id).cloned().unwrap_or_default();
                let okur = step_spoken(step);
                SpokenParity {
                    matched: compare_step(&key, &okur, &jaws_variants) == Comparison::Matched,
                    engine: okur,
                    jaws_variants,
                }
            });
            Ok(LiveStep {
                key,
                command: step
                    .pointer("/action/command")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                matched: comparison == Comparison::Matched,
                accepted: comparison == Comparison::Accepted,
                settled: step
                    .pointer("/observation/settled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                okur,
                jaws_variants: jaws,
                spoken,
            })
        })
        .collect()
}

/// Where a platform's live runs are kept: `corpus/live/<platform>/<scenario_id>.json`, each as
/// the platform's runner wrote it (the Linux lab's `run-scenario-linux.py`), never edited.
pub const LIVE_DIR: &str = "live";

/// A platform's live runs compared with JAWS's evidence: the conformance a port shows live
/// (docs/plans/2026-11-05-linux-first.md §2.2).
#[derive(Debug, Default)]
pub struct LiveReport {
    pub runs: usize,
    pub steps: Vec<LiveStep>,
    pub errors: Vec<String>,
}

impl LiveReport {
    pub fn matched(&self) -> usize {
        self.steps.iter().filter(|s| s.matched).count()
    }

    pub fn accepted(&self) -> usize {
        self.steps.iter().filter(|s| s.accepted).count()
    }

    /// The steps compared: every step but the accepted divergences.
    pub fn compared(&self) -> usize {
        self.steps.len() - self.accepted()
    }
}

fn read_json(path: &std::path::Path) -> Result<Value, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("{path:?}: {e}"))
}

/// Compares every live run of `platform` kept in the corpus with JAWS's evidence of its
/// scenario, in the runs' order by name.
pub fn run_platform(corpus_dir: &std::path::Path, platform: &str) -> LiveReport {
    let mut report = LiveReport::default();
    let dir = corpus_dir.join(LIVE_DIR).join(platform);
    let mut paths: Vec<_> = match std::fs::read_dir(&dir) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect(),
        Err(e) => {
            report.errors.push(format!("{dir:?}: {e}"));
            return report;
        }
    };
    paths.sort();
    for path in paths {
        let compared = read_json(&path).and_then(|run| {
            let scenario = run
                .get("scenario_id")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{path:?}: no scenario_id"))?
                .to_owned();
            let evidence = read_json(
                &corpus_dir
                    .join("evidence")
                    .join(format!("{scenario}-evidence.json")),
            )?;
            compare(&run, &evidence).map_err(|e| format!("{scenario}: {e}"))
        });
        match compared {
            Ok(steps) => {
                report.runs += 1;
                report.steps.extend(steps);
            }
            Err(e) => report.errors.push(e),
        }
    }
    report
}

/// The baseline of `platform`'s live runs: `corpus/live/<platform>-baseline.json`.
fn baseline_path(corpus_dir: &std::path::Path, platform: &str) -> std::path::PathBuf {
    corpus_dir
        .join(LIVE_DIR)
        .join(format!("{platform}-baseline.json"))
}

/// The command that writes `platform`'s baseline: a replay's (`linux-replay`) by
/// `live-replay`, a platform's live runs' by `live-baseline`.
fn regenerate(platform: &str) -> String {
    match platform.strip_suffix("-replay") {
        Some(replayed) => format!("live-replay {replayed} --write"),
        None => format!("live-baseline {platform} --write"),
    }
}

/// Renders the baseline exactly as stored on disk.
pub fn render_baseline(platform: &str, report: &LiveReport) -> Result<String, String> {
    let steps: std::collections::BTreeMap<&str, &str> = report
        .steps
        .iter()
        .map(|s| {
            let status = match (s.matched, s.accepted) {
                (true, _) => "match",
                (false, true) => "accepted divergence",
                (false, false) => "mismatch",
            };
            (s.key.as_str(), status)
        })
        .collect();
    let baseline = serde_json::json!({
        "notice": format!(
            "GENERATED FILE - do not edit. Regenerate: cargo run --bin okur-lab -- {}",
            regenerate(platform)
        ),
        "platform": platform,
        "runs": report.runs,
        "matched_steps": report.matched(),
        "total_steps": report.compared(),
        "accepted_divergences": report.accepted(),
        "errors": report.errors,
        "steps": steps,
    });
    serde_json::to_string_pretty(&baseline)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

/// The steps that matched JAWS in the committed baseline and no longer do, or are gone.
fn regressions(corpus_dir: &std::path::Path, platform: &str, report: &LiveReport) -> Vec<String> {
    let committed = std::fs::read_to_string(baseline_path(corpus_dir, platform))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or(Value::Null);
    let Some(steps) = committed.get("steps").and_then(Value::as_object) else {
        return Vec::new();
    };
    steps
        .iter()
        .filter(|(_, status)| status.as_str() == Some("match"))
        .filter(|(key, _)| !report.steps.iter().any(|s| &s.key == *key && s.matched))
        .map(|(key, _)| key.clone())
        .collect()
}

/// Compares `report` with the committed baseline: a step that matched JAWS and no longer does
/// is a regression; any other difference means the baseline is stale.
pub fn check_baseline(
    corpus_dir: &std::path::Path,
    platform: &str,
    report: &LiveReport,
) -> Result<(), String> {
    let regressed = regressions(corpus_dir, platform, report);
    if !regressed.is_empty() {
        return Err(format!(
            "Okur live on {platform} regressed for: {}",
            regressed.join(", ")
        ));
    }
    let committed = std::fs::read_to_string(baseline_path(corpus_dir, platform))
        .unwrap_or_default()
        .replace("\r\n", "\n");
    if committed != render_baseline(platform, report)? {
        return Err(format!(
            "corpus/live/{platform}-baseline.json is stale: review and regenerate with \
             `cargo run --bin okur-lab -- {}`",
            regenerate(platform)
        ));
    }
    Ok(())
}

/// Writes the baseline. It may only improve: a regression is refused unless `allow_regression`
/// (a step that matched by chance, recorded so in the commit that lets it go).
pub fn write_baseline(
    corpus_dir: &std::path::Path,
    platform: &str,
    report: &LiveReport,
    allow_regression: bool,
) -> Result<(), String> {
    let regressed = regressions(corpus_dir, platform, report);
    if !regressed.is_empty() && !allow_regression {
        return Err(format!(
            "not written: Okur live on {platform} regressed for {}",
            regressed.join(", ")
        ));
    }
    std::fs::write(
        baseline_path(corpus_dir, platform),
        render_baseline(platform, report)?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn file(reader: Option<&str>, steps: &[(&str, &str)]) -> Value {
        let provenance = match reader {
            Some(reader) => json!({ "reader": reader }),
            None => json!({}),
        };
        json!({
            "scenario_id": "222-web-links-list",
            "provenance": provenance,
            "trials": [{
                "trial_success": true,
                "step_results": steps.iter().map(|(id, speech)| json!({
                    "step_id": id,
                    "action": { "command": "Insert+F7" },
                    "observation": { "speech_raw": speech }
                })).collect::<Vec<_>>()
            }]
        })
    }

    #[test]
    fn a_live_run_is_compared_step_by_step() {
        let evidence = file(
            None,
            &[("s1", "Links List dialog"), ("s2", "Cancel Button")],
        );
        let run = file(
            Some("okur"),
            &[("s1", "Links List  dialog"), ("s2", "Cancel")],
        );
        let steps = compare(&run, &evidence).unwrap_or_default();
        let matched: Vec<bool> = steps.iter().map(|s| s.matched).collect();
        assert_eq!(matched, vec![true, false]);
        assert_eq!(
            steps.first().map(|s| s.key.as_str()),
            Some("222-web-links-list/s1")
        );
    }

    /// A run whose only trial failed (a step ran out of time) is still compared, with the
    /// unsettled step marked.
    #[test]
    fn a_failed_trial_is_compared_with_its_unsettled_steps_marked() {
        let evidence = file(None, &[("s1", "Count Button"), ("s2", "a long log")]);
        let mut run = file(Some("okur"), &[("s1", "Count Button"), ("s2", "a long")]);
        if let Some(success) = run.pointer_mut("/trials/0/trial_success") {
            *success = json!(false);
        }
        if let Some(observation) = run.pointer_mut("/trials/0/step_results/1/observation") {
            *observation = json!({ "speech_raw": "a long", "settled": false });
        }
        let steps = compare(&run, &evidence).unwrap_or_default();
        let found: Vec<(bool, bool)> = steps.iter().map(|s| (s.matched, s.settled)).collect();
        assert_eq!(found, vec![(true, true), (false, false)]);
    }

    /// What Okur sent its synthesizer is compared with what JAWS sent its own, when both were
    /// recorded through the speech recorder voice.
    #[test]
    fn the_spoken_layer_is_compared_too() {
        let mut evidence = file(None, &[("s1", "Save... Button")]);
        if let Some(observation) = evidence.pointer_mut("/trials/0/step_results/0/observation") {
            *observation = json!({
                "speech_raw": "Save... Button",
                "spoken": ["Save dot dot dot Button", "Copy speech history to clipboard"]
            });
        }
        let mut run = file(Some("okur"), &[("s1", "Save... Button")]);
        let steps = compare(&run, &evidence).unwrap_or_default();
        assert_eq!(steps.first().map(|s| s.spoken.is_none()), Some(true));
        if let Some(observation) = run.pointer_mut("/trials/0/step_results/0/observation") {
            *observation = json!({
                "speech_raw": "Save... Button",
                "spoken": ["Save dot dot dot Button"]
            });
        }
        let steps = compare(&run, &evidence).unwrap_or_default();
        assert_eq!(
            steps
                .first()
                .and_then(|s| s.spoken.as_ref())
                .map(|s| s.matched),
            Some(true)
        );
    }

    /// Evidence is never taken for a run, nor a run for evidence.
    #[test]
    fn only_an_okur_run_against_jaws_evidence() {
        let evidence = file(None, &[("s1", "x")]);
        assert!(compare(&evidence, &evidence).is_err());
        let run = file(Some("okur"), &[("s1", "x")]);
        assert!(compare(&run, &run).is_err());
    }

    fn step(key: &str, matched: bool) -> LiveStep {
        LiveStep {
            key: key.to_owned(),
            command: "Down".to_owned(),
            okur: String::new(),
            jaws_variants: Vec::new(),
            matched,
            accepted: false,
            settled: true,
            spoken: None,
        }
    }

    /// A platform's live baseline may only improve: a step that matched and no longer does is
    /// a regression, which the check reports and the write refuses unless allowed.
    #[test]
    fn a_live_baseline_may_only_improve() {
        let dir = std::env::temp_dir().join(format!("okur-live-baseline-{}", std::process::id()));
        let _ = std::fs::create_dir_all(dir.join(LIVE_DIR));
        let report = |first: bool| LiveReport {
            runs: 1,
            steps: vec![step("02-doc/s1", first), step("02-doc/s2", false)],
            errors: Vec::new(),
        };
        assert_eq!(write_baseline(&dir, "linux", &report(true), false), Ok(()));
        assert_eq!(check_baseline(&dir, "linux", &report(true)), Ok(()));
        let worse = report(false);
        assert!(check_baseline(&dir, "linux", &worse).is_err_and(|e| e.contains("02-doc/s1")));
        assert!(write_baseline(&dir, "linux", &worse, false).is_err());
        assert_eq!(write_baseline(&dir, "linux", &worse, true), Ok(()));
        let better = LiveReport {
            runs: 1,
            steps: vec![step("02-doc/s1", false), step("02-doc/s2", true)],
            errors: Vec::new(),
        };
        // An improvement is not a regression, but leaves the baseline stale until written.
        assert!(check_baseline(&dir, "linux", &better).is_err_and(|e| e.contains("stale")));
        assert_eq!(write_baseline(&dir, "linux", &better, false), Ok(()));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
