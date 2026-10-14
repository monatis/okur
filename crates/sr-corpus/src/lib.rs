pub mod evidence;
pub mod manifest;
pub mod runner;
pub mod scenario;

pub use evidence::EvidenceFile;
pub use runner::{DifferentialRunner, ScenarioExecutionResult, StepExecutionResult};
pub use scenario::{Scenario, ScenarioReset, ScenarioStep, StepAssertions};

pub fn init() {
    tracing::info!("sr-corpus initialized");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn corpus_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
    }

    /// Runs every scenario under `dir` and returns a failure report (empty when all pass).
    fn run_all_and_report(dir: &Path, expected_count: usize) -> Vec<String> {
        let results = DifferentialRunner::new(dir).run_range(0, u32::MAX);
        let mut failures = Vec::new();
        if results.len() != expected_count {
            failures.push(format!(
                "expected {expected_count} scenarios under {dir:?}, ran {}",
                results.len()
            ));
        }
        for res in results {
            match res {
                Err(e) => failures.push(format!("scenario failed to load/run: {e}")),
                Ok(r) => {
                    for step in r.steps.iter().filter(|s| !s.success) {
                        failures.push(format!(
                            "{} {}: speech={:?} failures={:?}",
                            r.scenario_id,
                            step.step_id,
                            step.emitted_speech,
                            step.failed_assertions
                        ));
                    }
                }
            }
        }
        failures
    }

    /// Scenarios 01–30 have genuine 5-trial JAWS recordings in `corpus/evidence/`.
    ///
    /// NOTE: this checks the agent-authored `assertions` in each scenario file against
    /// the hand-written `*.aom.json` fixtures. It is an engine regression check, NOT a
    /// parity comparison against the recorded JAWS speech.
    #[test]
    fn recorded_scenarios_meet_authored_assertions() {
        let failures = run_all_and_report(&corpus_dir(), 30);
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// `corpus/CORPUS_MANIFEST.json` must be exactly what the generator derives from disk.
    #[test]
    fn manifest_is_current() {
        assert_eq!(manifest::check(&corpus_dir()), Ok(()));
    }

    /// Scenarios 31–65 are quarantined in `corpus/unverified/`: their "evidence" was not
    /// produced by the capture tooling. Kept only as engine regression checks.
    #[test]
    fn unverified_scenarios_meet_authored_assertions() {
        let failures = run_all_and_report(&corpus_dir().join("unverified"), 35);
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
