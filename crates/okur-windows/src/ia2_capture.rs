//! IAccessible2 read from the live screen: the document of the Chromium window in front, as an
//! `okur_windows_model::ia2_snapshot::Ia2Snapshot`.

use crate::uia_capture::{os_version, window_text};
use okur_windows_model::ia2_snapshot::Ia2Snapshot;
use okur_windows_model::uia_snapshot::{SnapshotProvenance, SNAPSHOT_FORMAT_VERSION};

/// Captures the document of the foreground Chromium window through MSAA + IA2.
pub fn capture_foreground_document(
    fixture_sha256: Option<String>,
    process_version: Option<String>,
) -> Result<Ia2Snapshot, crate::PlatformError> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    let started = std::time::Instant::now();
    crate::com::init_mta();
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
