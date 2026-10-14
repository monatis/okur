pub mod evidence;
pub mod invariants;
pub mod keys;
pub mod manifest;
pub mod parity;
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
    fn run_all_and_report(dir: &Path, range: (u32, u32), expected_count: usize) -> Vec<String> {
        let results = DifferentialRunner::new(dir).run_range(range.0, range.1);
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
        // Only the frozen legacy set: later scenarios are verified by parity alone.
        let failures = run_all_and_report(&corpus_dir(), (1, 30), 30);
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// Engine speech vs. recorded JAWS speech, over UIA snapshots captured from Chrome on
    /// the reference machine. Fails on any regression and whenever the generated baseline
    /// is stale; run `cargo run -p sr-cli -- parity` for the per-step diff.
    #[test]
    fn jaws_parity_matches_baseline() {
        let dir = corpus_dir();
        let report = parity::run(&dir);
        assert!(
            report.errors.is_empty(),
            "scenarios could not run: {:?}",
            report.errors
        );
        assert_eq!(parity::check_baseline(&dir, &report), Ok(()));
    }

    /// Universal buffer invariants over every captured snapshot. Known violations are listed
    /// in the generated baseline; any new one fails, and fixing one requires regenerating it.
    #[test]
    fn buffer_invariants_match_baseline() {
        let dir = corpus_dir();
        let results = invariants::run_all(&dir);
        assert!(results.is_ok(), "{results:?}");
        if let Ok(results) = results {
            assert_eq!(invariants::check_baseline(&dir, &results), Ok(()));
        }
    }

    fn snapshot_aom(name: &str) -> sr_core::AomNode {
        let path = corpus_dir()
            .join("snapshots")
            .join(format!("{name}.chrome.uia.json"));
        let snapshot = sr_platform::uia_snapshot::UiaSnapshot::load(&path);
        assert!(snapshot.is_ok(), "{snapshot:?}");
        snapshot
            .map(|s| sr_platform::uia_snapshot::to_aom(&s.root))
            .unwrap_or_else(|_| sr_core::AomNode::new(0, sr_core::Role::Generic))
    }

    fn find<'a>(
        node: &'a sr_core::AomNode,
        pred: &dyn Fn(&sr_core::AomNode) -> bool,
    ) -> Vec<&'a sr_core::AomNode> {
        let mut out = Vec::new();
        if pred(node) {
            out.push(node);
        }
        for c in &node.children {
            out.extend(find(c, pred));
        }
        out
    }

    /// The UIA→AOM mapping on real Chrome data: semantics come from control types, ARIA
    /// roles and UIA properties (old-pc's localized names are Turkish and must not matter).
    #[test]
    fn uia_mapping_preserves_chrome_semantics() {
        use sr_core::Role;
        let doc = snapshot_aom("structural-navigation");
        let levels: Vec<u32> = find(&doc, &|n| matches!(n.role, Role::Heading { .. }))
            .iter()
            .filter_map(|n| match n.role {
                Role::Heading { level } => Some(level),
                _ => None,
            })
            .collect();
        assert_eq!(levels, vec![1, 2, 3, 2, 2]);
        let hrefs: Vec<Option<String>> = find(&doc, &|n| matches!(n.role, Role::Link { .. }))
            .iter()
            .map(|n| match &n.role {
                Role::Link { href } => href.clone(),
                _ => None,
            })
            .collect();
        assert_eq!(hrefs.len(), 3);
        assert!(hrefs.iter().all(|h| h
            .as_deref()
            .is_some_and(|h| h.contains("structural-navigation.html#"))));
        let landmarks: Vec<String> = find(&doc, &|n| matches!(n.role, Role::Landmark { .. }))
            .iter()
            .filter_map(|n| match &n.role {
                Role::Landmark { landmark_type } => Some(landmark_type.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            landmarks,
            vec![
                "banner",
                "navigation",
                "main",
                "complementary",
                "contentinfo"
            ]
        );

        let forms = snapshot_aom("forms-interaction");
        let checkbox = find(&forms, &|n| n.role == Role::CheckBox);
        assert_eq!(checkbox.len(), 1);
        assert_eq!(checkbox.first().and_then(|c| c.states.checked), Some(false));
        let edits = find(&forms, &|n| n.role == Role::Edit);
        assert_eq!(
            edits.iter().map(|e| e.value.as_deref()).collect::<Vec<_>>(),
            vec![Some("Jane Doe"), Some("jane@example.com")]
        );

        let tables = snapshot_aom("tables-navigation");
        let col_headers = find(&tables, &|n| {
            n.attributes.get("scope").map(String::as_str) == Some("col")
        });
        assert_eq!(col_headers.len(), 4);
        let row_headers = find(&tables, &|n| {
            n.attributes.get("scope").map(String::as_str) == Some("row")
        });
        assert_eq!(row_headers.len(), 3);
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
        let failures = run_all_and_report(&corpus_dir().join("unverified"), (31, 65), 35);
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
