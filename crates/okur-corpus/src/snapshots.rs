//! Captured document snapshots (`corpus/snapshots/<fixture>.chrome.<api>.json`) as AOM trees.
//!
//! The same fixture load is captured through more than one accessibility API. Each maps into
//! the engine's AOM through production code (`okur_windows_model::uia_snapshot::to_aom`,
//! `okur_windows_model::ia2_snapshot::ia2_to_aom`, and for a page captured on Linux
//! `okur_linux_model::page::to_aom`), so the harness can measure each API's parity.

use okur_core::AomNode;
use okur_windows_model::ia2_snapshot::{ia2_to_aom, Ia2Snapshot};
use okur_windows_model::uia_snapshot::{to_aom, UiaSnapshot};
use std::path::{Path, PathBuf};

/// The accessibility API a snapshot was captured through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    Uia,
    Ia2,
    /// Chrome's page read through AT-SPI in the Linux lab.
    Atspi,
}

/// The API the parity ratchet runs on. JAWS reads Chromium documents through IA2.
pub const PARITY_API: Api = Api::Ia2;

impl Api {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "uia" => Some(Self::Uia),
            "ia2" => Some(Self::Ia2),
            "atspi" => Some(Self::Atspi),
            _ => None,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Uia => ".chrome.uia.json",
            Self::Ia2 => ".chrome.ia2.json",
            Self::Atspi => ".chrome.atspi.json",
        }
    }

    fn of_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        [Self::Uia, Self::Ia2, Self::Atspi]
            .into_iter()
            .find(|api| name.ends_with(api.suffix()))
    }
}

/// The snapshot of `fixture` (its key, [`crate::scenario::fixture_key`]: `news-article.html`,
/// `aria-at/checkbox-tri-state/checkbox-mixed.html`) for `api`. Snapshots lie side by side: a
/// fixture in a folder is named by its path, dots for slashes
/// (`aria-at.checkbox-tri-state.checkbox-mixed.chrome.ia2.json`).
pub fn path(corpus_dir: &Path, fixture: &str, api: Api) -> PathBuf {
    let stem = fixture
        .strip_suffix(".html")
        .unwrap_or(fixture)
        .replace('/', ".");
    corpus_dir
        .join("snapshots")
        .join(format!("{stem}{}", api.suffix()))
}

/// A loaded snapshot: the mapped AOM and the fixture hash recorded at capture time.
pub struct Loaded {
    pub aom: AomNode,
    pub fixture_sha256: Option<String>,
    /// The page's IA2 objects as captured, where the snapshot is IA2's: what a live read of
    /// them (Element Information's) would have found.
    pub ia2: Option<okur_windows_model::ia2_snapshot::Ia2Node>,
}

/// Loads and maps a snapshot, choosing the API by its file name.
pub fn load(path: &Path) -> Result<Loaded, String> {
    match Api::of_path(path) {
        Some(Api::Uia) => {
            let s = UiaSnapshot::load(path)?;
            Ok(Loaded {
                aom: to_aom(&s.root),
                fixture_sha256: s.provenance.fixture_sha256,
                ia2: None,
            })
        }
        Some(Api::Ia2) => {
            let s = Ia2Snapshot::load(path)?;
            Ok(Loaded {
                aom: ia2_to_aom(&s.root),
                fixture_sha256: s.provenance.fixture_sha256,
                ia2: Some(s.root),
            })
        }
        Some(Api::Atspi) => {
            let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
            let s: okur_linux_model::page::AtspiSnapshot =
                serde_json::from_str(text.trim_start_matches('\u{feff}'))
                    .map_err(|e| format!("{path:?}: {e}"))?;
            Ok(Loaded {
                aom: okur_linux_model::page::to_aom(&s.root),
                fixture_sha256: s.provenance.fixture_sha256,
                ia2: None,
            })
        }
        None => Err(format!(
            "{path:?}: not a .chrome.uia.json, .chrome.ia2.json or .chrome.atspi.json snapshot"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use okur_linux_model::page::{state, AtspiNode, AtspiSnapshot};
    use okur_model::snapshot::SnapshotProvenance;

    fn node(role: &str, name: &str, children: Vec<AtspiNode>) -> AtspiNode {
        AtspiNode {
            role: role.to_owned(),
            name: name.to_owned(),
            states: 1 << state::ENABLED,
            children,
            ..AtspiNode::default()
        }
    }

    /// A page captured through AT-SPI in the Linux lab is found by its name beside the
    /// Windows ones and loads as its page, with the fixture's hash it was captured from.
    #[test]
    fn an_atspi_snapshot_loads_as_its_page() -> Result<(), String> {
        let dir = std::env::temp_dir().join(format!("okur-atspi-{}", std::process::id()));
        let file = path(&dir, "aria-at/checkbox/checkbox.html", Api::Atspi);
        assert!(file.ends_with("snapshots/aria-at.checkbox.checkbox.chrome.atspi.json"));
        let mut heading = node("heading", "Title", vec![node("static", "Title", vec![])]);
        heading
            .attributes
            .insert("level".to_owned(), "1".to_owned());
        let snapshot = AtspiSnapshot {
            provenance: SnapshotProvenance {
                format_version: 1,
                tool_version: "test".to_owned(),
                host: "okur-lab".to_owned(),
                os_version: "Linux".to_owned(),
                window_title: "Page".to_owned(),
                window_class: String::new(),
                process_version: None,
                fixture_sha256: Some("abc".to_owned()),
                captured_at_unix_ms: 0,
                node_count: 3,
                capture_ms: 0,
            },
            root: node("document web", "Page", vec![heading]),
        };
        let json = serde_json::to_string(&snapshot).map_err(|e| e.to_string())?;
        let parent = file.parent().ok_or("no folder")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        std::fs::write(&file, json).map_err(|e| e.to_string())?;
        let loaded = load(&file);
        let _ = std::fs::remove_dir_all(&dir);
        let loaded = loaded?;
        assert_eq!(loaded.fixture_sha256.as_deref(), Some("abc"));
        assert_eq!(loaded.aom.role, okur_core::Role::Document);
        assert_eq!(
            loaded.aom.children.first().map(|h| h.role.clone()),
            Some(okur_core::Role::Heading { level: 1 })
        );
        Ok(())
    }
}
