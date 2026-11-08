//! Where and how a page's snapshot was captured, whatever the accessibility API it was read
//! through: UI Automation and IAccessible2 on Windows, AT-SPI on Linux. The corpus's snapshots
//! (`corpus/snapshots/<fixture>.chrome.<api>.json`) carry it, and parity checks the fixture's hash
//! against the fixture's bytes.

use serde::{Deserialize, Serialize};

/// Bumped whenever the captured property set or its meaning changes.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// Where and how a snapshot was captured.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotProvenance {
    pub format_version: u32,
    /// The capturing crate and its version (`okur-windows`, `okur-linux 0.1.0`).
    pub tool_version: String,
    pub host: String,
    pub os_version: String,
    pub window_title: String,
    pub window_class: String,
    /// Executable file version of the captured process (e.g. Chrome's version).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_version: Option<String>,
    /// Fixture the page was loaded from, when the capture was made for the corpus.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_sha256: Option<String>,
    pub captured_at_unix_ms: u128,
    pub node_count: usize,
    pub capture_ms: u128,
}
