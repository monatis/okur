//! IAccessible2 snapshots of browser documents.
//!
//! Captured alongside UIA snapshots (`sr-cli snapshot --api ia2`) from the same fixture load,
//! with the same provenance block. IA2 adds what UIA lacks for web content: HTML tags and CSS
//! display (`tag`, `display` attributes), visited state, hypertext with embedded objects, and
//! the browser's rendered line ranges per text object.

use crate::uia_snapshot::SnapshotProvenance;
use serde::{Deserialize, Serialize};

pub use xa11y_windows::ia2::Ia2Node;

/// A captured IA2 document tree plus its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ia2Snapshot {
    pub provenance: SnapshotProvenance,
    pub root: Ia2Node,
}

impl Ia2Snapshot {
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, String> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        serde_json::from_str(text.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("{path:?}: {e}"))
    }
}

/// Captures the document of the foreground Chromium window through MSAA + IA2.
#[cfg(windows)]
pub fn capture_foreground_document(
    fixture_sha256: Option<String>,
    process_version: Option<String>,
) -> Result<Ia2Snapshot, crate::PlatformError> {
    use crate::uia_snapshot::capture::{os_version, window_text};
    use crate::uia_snapshot::SNAPSHOT_FORMAT_VERSION;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let started = std::time::Instant::now();
    // SAFETY: standard COM initialization; repeated initialization is tolerated.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    // SAFETY: plain Win32 call with no preconditions.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        return Err(crate::PlatformError::NoWindowFound);
    }
    let (window_title, window_class) = window_text(hwnd);
    let root = xa11y_windows::ia2::capture_chromium_document(hwnd)
        .map_err(|e| crate::PlatformError::EventListenerError(e.to_string()))?;
    Ok(Ia2Snapshot {
        provenance: SnapshotProvenance {
            format_version: SNAPSHOT_FORMAT_VERSION,
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
            host: std::env::var("COMPUTERNAME").unwrap_or_default(),
            os_version: os_version(),
            window_title,
            window_class,
            process_version,
            fixture_sha256,
            captured_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
            node_count: root.node_count(),
            capture_ms: started.elapsed().as_millis(),
        },
        root,
    })
}
