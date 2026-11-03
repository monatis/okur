//! COM on the calling thread.

/// Joins the calling thread to COM's multithreaded apartment, which UI Automation, MSAA, the
/// shell and the synthesizers are called from on Okur's threads. A thread already in COM,
/// in this apartment or another, stays as it is.
pub fn init_mta() {
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    // SAFETY: no reserved pointer is passed; S_FALSE (already joined) and RPC_E_CHANGED_MODE
    // (in another apartment) leave the thread usable as it was, so the result is not needed.
    let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
}
