use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceFile {
    pub scenario_id: String,
    pub scenario_version: String,
    #[serde(default)]
    pub trials: Vec<Trial>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trial {
    pub trial_index: u32,
    pub step_results: Vec<StepResultRecord>,
    pub trial_success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResultRecord {
    pub step_id: String,
    pub observation: Observation,
    pub assertions_passed: bool,
    pub step_success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub speech_raw: Option<String>,
    #[serde(default)]
    pub dispatch_ok: bool,
}

impl EvidenceFile {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let clean = content.trim_start_matches('\u{feff}');
        let evidence = serde_json::from_str(clean)?;
        Ok(evidence)
    }

    /// Finds canonical observation speech for a given step ID from the first successful trial.
    pub fn canonical_speech_for_step(&self, step_id: &str) -> Option<String> {
        for trial in &self.trials {
            if trial.trial_success {
                for res in &trial.step_results {
                    if res.step_id == step_id {
                        return res.observation.speech_raw.clone();
                    }
                }
            }
        }
        None
    }
}
