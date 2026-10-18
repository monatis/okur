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

use crate::parity::{jaws_variants, normalize};
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
    /// Whether the runner heard Okur fall silent before the step's time ran out. An
    /// unsettled step may have been cut short.
    pub settled: bool,
}

/// What a live run is, by its provenance.
fn reader(file: &Value) -> Option<&str> {
    file.pointer("/provenance/reader").and_then(Value::as_str)
}

/// Compares the first successful trial of Okur's live `run` with JAWS's `evidence` of the same
/// scenario, step by step, as `okur parity` compares (whitespace normalized, any recorded
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
            let ours = normalize(&okur);
            Ok(LiveStep {
                key: format!("{scenario}/{id}"),
                command: step
                    .pointer("/action/command")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                matched: jaws.iter().any(|v| normalize(v) == ours),
                settled: step
                    .pointer("/observation/settled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                okur,
                jaws_variants: jaws,
            })
        })
        .collect()
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

    /// Evidence is never taken for a run, nor a run for evidence.
    #[test]
    fn only_an_okur_run_against_jaws_evidence() {
        let evidence = file(None, &[("s1", "x")]);
        assert!(compare(&evidence, &evidence).is_err());
        let run = file(Some("okur"), &[("s1", "x")]);
        assert!(compare(&run, &run).is_err());
    }
}
