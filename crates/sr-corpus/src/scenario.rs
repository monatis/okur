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
    #[serde(default)]
    pub keystrokes: Option<Vec<String>>,
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
    #[serde(default)]
    pub assertions: Option<StepAssertions>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepAssertions {
    #[serde(default)]
    pub contains_speech: Option<Vec<String>>,
    #[serde(default)]
    pub target_role: Option<String>,
    #[serde(default)]
    pub target_name: Option<String>,
    #[serde(default)]
    pub forms_mode_active: Option<bool>,
    #[serde(default)]
    pub target_heading_level: Option<u32>,
    #[serde(default)]
    pub target_heading_name: Option<String>,
    #[serde(default)]
    pub target_checked: Option<bool>,
}

impl Scenario {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let clean = content.trim_start_matches('\u{feff}');
        let scenario = serde_json::from_str(clean)?;
        Ok(scenario)
    }
}
