//! Generated corpus manifest.
//!
//! `corpus/CORPUS_MANIFEST.json` is derived entirely from the files on disk and must never
//! be edited by hand. Regenerate it with `cargo run -p sr-cli -- corpus-manifest --write`;
//! the `manifest_is_current` test fails whenever the committed copy is stale, so every
//! statistic in it is verifiable rather than claimed.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub const MANIFEST_FILE: &str = "CORPUS_MANIFEST.json";

#[derive(Debug, Serialize)]
pub struct CorpusManifest {
    pub notice: &'static str,
    pub fixtures: BTreeMap<String, FileEntry>,
    pub scenarios: Vec<ScenarioEntry>,
    pub statistics: Statistics,
}

#[derive(Debug, Serialize)]
pub struct FileEntry {
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ScenarioEntry {
    pub scenario_id: String,
    pub fixture: String,
    pub evidence: Option<EvidenceEntry>,
}

#[derive(Debug, Serialize)]
pub struct EvidenceEntry {
    pub sha256: String,
    pub os_version: Option<String>,
    pub jaws_version: Option<String>,
    pub trials: usize,
    pub successful_trials: usize,
    /// True when every successful trial produced identical speech for every step.
    pub deterministic: bool,
    /// The `summary` block written alongside the trials, if any, agrees with the trials.
    pub summary_consistent: bool,
}

#[derive(Debug, Default, Serialize)]
pub struct Statistics {
    pub scenarios: usize,
    pub scenarios_with_evidence: usize,
    pub total_trials: usize,
    pub successful_trials: usize,
    pub deterministic_scenarios: usize,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
    serde_json::from_str(text.trim_start_matches('\u{feff}')).map_err(|e| format!("{path:?}: {e}"))
}

fn sorted_files(dir: &Path, suffix: &str) -> Result<Vec<std::path::PathBuf>, String> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .map_err(|e| format!("{dir:?}: {e}"))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.to_string_lossy().ends_with(suffix))
        .collect();
    files.sort();
    Ok(files)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn evidence_entry(path: &Path) -> Result<EvidenceEntry, String> {
    let bytes = fs::read(path).map_err(|e| format!("{path:?}: {e}"))?;
    let json = read_json(path)?;
    let env = json.get("environment");
    let env_str = |key: &str| {
        env.and_then(|e| e.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };

    let trials = json
        .get("trials")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let successful: Vec<&Value> = trials
        .iter()
        .filter(|t| t.get("trial_success").and_then(Value::as_bool) == Some(true))
        .collect();

    let speech_of = |trial: &Value| -> Vec<Option<String>> {
        trial
            .get("step_results")
            .and_then(Value::as_array)
            .map(|steps| {
                steps
                    .iter()
                    .map(|s| {
                        s.pointer("/observation/speech_raw")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let deterministic = match successful.split_first() {
        Some((first, rest)) => {
            let reference = speech_of(first);
            rest.iter().all(|t| speech_of(t) == reference)
        }
        None => false,
    };

    let summary = json.get("summary");
    let summary_count = |key: &str| summary.and_then(|s| s.get(key)).and_then(Value::as_u64);
    let summary_consistent = summary.is_none()
        || (summary_count("total_trials") == Some(trials.len() as u64)
            && summary_count("successful_trials") == Some(successful.len() as u64));

    Ok(EvidenceEntry {
        sha256: sha256_hex(&bytes),
        os_version: env_str("os_version"),
        jaws_version: env_str("jaws_version"),
        trials: trials.len(),
        successful_trials: successful.len(),
        deterministic,
        summary_consistent,
    })
}

/// Builds the manifest for the corpus rooted at `corpus_dir` from the files on disk.
pub fn generate(corpus_dir: &Path) -> Result<CorpusManifest, String> {
    let mut fixtures = BTreeMap::new();
    for path in sorted_files(&corpus_dir.join("fixtures"), "")? {
        let bytes = fs::read(&path).map_err(|e| format!("{path:?}: {e}"))?;
        fixtures.insert(
            file_name(&path),
            FileEntry {
                sha256: sha256_hex(&bytes),
                size_bytes: bytes.len() as u64,
            },
        );
    }

    let mut scenarios = Vec::new();
    let mut stats = Statistics::default();
    for path in sorted_files(&corpus_dir.join("scenarios"), ".json")? {
        let json = read_json(&path)?;
        let scenario_id = json
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("{path:?}: missing \"id\""))?;
        let fixture = json
            .get("fixture")
            .and_then(Value::as_str)
            .and_then(|uri| uri.rsplit('/').next())
            .unwrap_or_default()
            .to_owned();
        if !fixtures.contains_key(&fixture) {
            return Err(format!(
                "{scenario_id}: fixture {fixture:?} not found in fixtures/"
            ));
        }

        let evidence_path = corpus_dir
            .join("evidence")
            .join(format!("{scenario_id}-evidence.json"));
        let evidence = if evidence_path.exists() {
            Some(evidence_entry(&evidence_path)?)
        } else {
            None
        };

        stats.scenarios += 1;
        if let Some(ev) = &evidence {
            stats.scenarios_with_evidence += 1;
            stats.total_trials += ev.trials;
            stats.successful_trials += ev.successful_trials;
            if ev.deterministic {
                stats.deterministic_scenarios += 1;
            }
        }
        scenarios.push(ScenarioEntry {
            scenario_id,
            fixture,
            evidence,
        });
    }

    Ok(CorpusManifest {
        notice: "GENERATED FILE - do not edit. Regenerate: cargo run -p sr-cli -- corpus-manifest --write",
        fixtures,
        scenarios,
        statistics: stats,
    })
}

/// Renders the manifest exactly as it is stored on disk.
pub fn render(manifest: &CorpusManifest) -> Result<String, String> {
    serde_json::to_string_pretty(manifest)
        .map(|s| s + "\n")
        .map_err(|e| e.to_string())
}

/// Returns `Ok(())` when the committed manifest matches the files on disk.
pub fn check(corpus_dir: &Path) -> Result<(), String> {
    let expected = render(&generate(corpus_dir)?)?;
    let actual = fs::read_to_string(corpus_dir.join(MANIFEST_FILE)).unwrap_or_default();
    if actual.replace("\r\n", "\n") == expected {
        Ok(())
    } else {
        Err(format!(
            "{MANIFEST_FILE} is stale or hand-edited. Regenerate it with: \
             cargo run -p sr-cli -- corpus-manifest --write"
        ))
    }
}

/// Regenerates and writes the manifest.
pub fn write(corpus_dir: &Path) -> Result<(), String> {
    let rendered = render(&generate(corpus_dir)?)?;
    fs::write(corpus_dir.join(MANIFEST_FILE), rendered).map_err(|e| e.to_string())
}
