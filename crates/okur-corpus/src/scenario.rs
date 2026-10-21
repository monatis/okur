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
    /// Desktop scenarios: a fixture app launched fresh per trial, found by window title.
    #[serde(default)]
    pub launch: Option<serde_json::Value>,
    #[serde(default)]
    pub reset: Option<ScenarioReset>,
    /// The scenario recorded just before it, whose saved state it starts from (a Flexible Web
    /// rule saved there): replay runs that one first, and keeps what Okur saved.
    #[serde(default)]
    pub continues: Option<String>,
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
    /// A `key` step's chord pressed this many times quickly ("twice quickly"); 1 when absent.
    #[serde(default)]
    pub repeat: Option<u32>,
    /// Chords pressed while the step's speech goes on (Ctrl or Right during Say All), each
    /// `after_ms` after the one before.
    #[serde(default)]
    pub during: Option<Vec<DuringKey>>,
}

/// A chord a step presses while its speech goes on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuringKey {
    pub after_ms: u64,
    pub command: String,
}

impl Scenario {
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let clean = content.trim_start_matches('\u{feff}');
        let scenario = serde_json::from_str(clean)?;
        Ok(scenario)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_reads_the_keys_pressed_during_its_speech() -> Result<(), serde_json::Error> {
        let step: ScenarioStep = serde_json::from_str(
            r#"{ "id": "s1", "type": "key", "command": "Insert+Down",
                 "during": [{ "after_ms": 3000, "command": "Right" }, { "after_ms": 2500, "command": "Ctrl" }] }"#,
        )?;
        let during = step.during.unwrap_or_default();
        let keys: Vec<(u64, &str)> = during
            .iter()
            .map(|key| (key.after_ms, key.command.as_str()))
            .collect();
        assert_eq!(keys, vec![(3000, "Right"), (2500, "Ctrl")]);
        Ok(())
    }
}
