//! [`sr_core::actions::ApplicationActions`] for Chromium pages, through MSAA/IA2.
//!
//! Buffer nodes mapped from IA2 carry the object's IA2 `uniqueID`
//! ([`UNIQUE_ID_ATTR`]). An action resolves it with `accChild(-uniqueID)` on the
//! foreground window's document and calls `accSelect(SELFLAG_TAKEFOCUS)` or
//! `accDoDefaultAction`. The calls cross into the browser process, so they run on a worker
//! thread: the engine calls actions from the keyboard hook, which must return at once.

/// AOM attribute holding a node's IA2 `uniqueID`.
pub const UNIQUE_ID_ATTR: &str = "ia2_unique_id";

/// The IA2 `uniqueID` recorded on a node, if any.
pub fn unique_id(node: &sr_core::AomNode) -> Option<i32> {
    node.attributes.get(UNIQUE_ID_ATTR)?.parse().ok()
}

#[cfg(windows)]
pub use chromium::ChromiumActions;

#[cfg(windows)]
mod chromium {
    use super::unique_id;
    use std::sync::mpsc::{channel, Sender};
    use std::sync::Mutex;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    use xa11y_windows::ia2::{perform_chromium_action, Ia2Action};

    /// Performs actions on the foreground Chromium window's page, on a worker thread.
    pub struct ChromiumActions {
        requests: Mutex<Sender<(i32, Ia2Action)>>,
    }

    impl ChromiumActions {
        pub fn start() -> Self {
            let (tx, rx) = channel::<(i32, Ia2Action)>();
            std::thread::spawn(move || {
                // SAFETY: COM initialization for this worker thread.
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
                for (id, action) in rx {
                    // SAFETY: plain Win32 call with no preconditions.
                    let window = unsafe { GetForegroundWindow() };
                    if let Err(e) = perform_chromium_action(window, id, action) {
                        tracing::warn!("{action:?} on IA2 object {id} failed: {e}");
                    }
                }
            });
            Self {
                requests: Mutex::new(tx),
            }
        }

        fn request(&self, node: &sr_core::AomNode, action: Ia2Action) -> bool {
            let Some(id) = unique_id(node) else {
                return false;
            };
            self.requests
                .lock()
                .is_ok_and(|tx| tx.send((id, action)).is_ok())
        }
    }

    impl sr_core::actions::ApplicationActions for ChromiumActions {
        fn focus(&self, node: &sr_core::AomNode) -> bool {
            self.request(node, Ia2Action::Focus)
        }

        fn activate(&self, node: &sr_core::AomNode) -> bool {
            self.request(node, Ia2Action::DefaultAction)
        }
    }
}

/// Captures the foreground Chromium window's page through IA2 and maps it with
/// [`crate::ia2_snapshot::ia2_to_aom`]: the same pipeline the parity harness verifies.
#[cfg(windows)]
pub fn capture_foreground_page() -> Result<sr_core::AomNode, crate::PlatformError> {
    let snapshot = crate::ia2_snapshot::capture_foreground_document(None, None)?;
    Ok(crate::ia2_snapshot::ia2_to_aom(&snapshot.root))
}
