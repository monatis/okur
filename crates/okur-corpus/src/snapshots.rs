//! Captured document snapshots (`corpus/snapshots/<fixture>.chrome.<api>.json`) as AOM trees.
//!
//! The same fixture load is captured through more than one accessibility API. Both map into
//! the engine's AOM through production code (`okur_platform::uia_snapshot::to_aom`,
//! `okur_platform::ia2_snapshot::ia2_to_aom`), so the harness can measure each API's parity.

use okur_core::AomNode;
use okur_platform::ia2_snapshot::{ia2_to_aom, Ia2Snapshot};
use okur_platform::uia_snapshot::{to_aom, UiaSnapshot};
use std::path::{Path, PathBuf};

/// The accessibility API a snapshot was captured through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    Uia,
    Ia2,
}

/// The API the parity ratchet runs on. JAWS reads Chromium documents through IA2.
pub const PARITY_API: Api = Api::Ia2;

impl Api {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "uia" => Some(Self::Uia),
            "ia2" => Some(Self::Ia2),
            _ => None,
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            Self::Uia => ".chrome.uia.json",
            Self::Ia2 => ".chrome.ia2.json",
        }
    }

    fn of_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        [Self::Uia, Self::Ia2]
            .into_iter()
            .find(|api| name.ends_with(api.suffix()))
    }
}

/// The snapshot of `fixture` (a file name such as `news-article.html`) for `api`.
pub fn path(corpus_dir: &Path, fixture: &str, api: Api) -> PathBuf {
    let stem = fixture.strip_suffix(".html").unwrap_or(fixture);
    corpus_dir
        .join("snapshots")
        .join(format!("{stem}{}", api.suffix()))
}

/// A loaded snapshot: the mapped AOM and the fixture hash recorded at capture time.
pub struct Loaded {
    pub aom: AomNode,
    pub fixture_sha256: Option<String>,
}

/// Loads and maps a snapshot, choosing the API by its file name.
pub fn load(path: &Path) -> Result<Loaded, String> {
    match Api::of_path(path) {
        Some(Api::Uia) => {
            let s = UiaSnapshot::load(path)?;
            Ok(Loaded {
                aom: to_aom(&s.root),
                fixture_sha256: s.provenance.fixture_sha256,
            })
        }
        Some(Api::Ia2) => {
            let s = Ia2Snapshot::load(path)?;
            Ok(Loaded {
                aom: ia2_to_aom(&s.root),
                fixture_sha256: s.provenance.fixture_sha256,
            })
        }
        None => Err(format!(
            "{path:?}: not a .chrome.uia.json or .chrome.ia2.json snapshot"
        )),
    }
}
