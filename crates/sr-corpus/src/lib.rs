pub mod evidence;
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
    use std::path::Path;

    fn get_corpus_dir() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus")
    }

    #[test]
    fn test_milestone_1_scenarios_01_through_10() {
        let corpus_dir = get_corpus_dir();
        assert!(
            corpus_dir.exists(),
            "Corpus directory must exist at {:?}",
            corpus_dir
        );

        let runner = DifferentialRunner::new(&corpus_dir);
        let results = runner.run_range(1, 10);

        assert_eq!(results.len(), 10, "Should have executed 10 scenarios");

        for (idx, res) in results.into_iter().enumerate() {
            let scenario_res =
                res.unwrap_or_else(|e| panic!("Scenario {} failed to load/run: {}", idx + 1, e));
            println!(
                "Scenario [{}] {}: {}",
                scenario_res.scenario_id,
                scenario_res.scenario_name,
                if scenario_res.passed {
                    "PASSED"
                } else {
                    "FAILED"
                }
            );

            for step in &scenario_res.steps {
                println!(
                    "  Step [{}]: command='{}', speech='{}', passed={}",
                    step.step_id, step.command, step.emitted_speech, step.success
                );
                if !step.failed_assertions.is_empty() {
                    for fail in &step.failed_assertions {
                        println!("    ASSERTION FAILURE: {}", fail);
                    }
                }
            }

            assert!(
                scenario_res.passed,
                "Scenario {} ({}) failed assertion checks!",
                scenario_res.scenario_id, scenario_res.scenario_name
            );
        }
    }

    #[test]
    fn test_milestone_2_scenarios_11_through_15() {
        let corpus_dir = get_corpus_dir();
        assert!(
            corpus_dir.exists(),
            "Corpus directory must exist at {:?}",
            corpus_dir
        );

        let runner = DifferentialRunner::new(&corpus_dir);
        let results = runner.run_range(11, 15);

        assert_eq!(results.len(), 5, "Should have executed 5 scenarios");

        for (idx, res) in results.into_iter().enumerate() {
            let scenario_res =
                res.unwrap_or_else(|e| panic!("Scenario {} failed to load/run: {}", idx + 11, e));
            println!(
                "Scenario [{}] {}: {}",
                scenario_res.scenario_id,
                scenario_res.scenario_name,
                if scenario_res.passed {
                    "PASSED"
                } else {
                    "FAILED"
                }
            );

            for step in &scenario_res.steps {
                println!(
                    "  Step [{}]: command='{}', speech='{}', passed={}",
                    step.step_id, step.command, step.emitted_speech, step.success
                );
                if !step.failed_assertions.is_empty() {
                    for fail in &step.failed_assertions {
                        println!("    ASSERTION FAILURE: {}", fail);
                    }
                }
            }

            assert!(
                scenario_res.passed,
                "Scenario {} ({}) failed assertion checks!",
                scenario_res.scenario_id, scenario_res.scenario_name
            );
        }
    }
}
