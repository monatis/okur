use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub category: String,
    pub version: String,
    pub fixture: String,
    #[serde(default)]
    pub target_app: Option<String>,
    #[serde(default)]
    pub reset: Option<ScenarioReset>,
    pub steps: Vec<ScenarioStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioReset {
    /// Legacy SendKeys strings (scenarios 01-30).
    #[serde(default)]
    pub keystrokes: Option<Vec<String>>,
    /// Key chords in the `key` step syntax, e.g. `["Ctrl+Home"]`.
    #[serde(default)]
    pub keys: Option<Vec<String>>,
    #[serde(default)]
    pub exit_forms_mode: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStep {
    pub id: String,
    #[serde(rename = "type")]
    pub step_type: String,
    pub command: String,
    #[serde(default)]
    pub settle_ms: Option<u64>,
}

impl Scenario {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let clean = content.trim_start_matches('\u{feff}');
        let scenario = serde_json::from_str(clean)?;
        Ok(scenario)
    }
}
