//! What a window's class says about how it is read.

/// Whether a window of class `class` (its top-level window's), drawn by `framework` (UI
/// Automation's FrameworkId, empty when unknown), is read element by element: a modern app's
/// (UWP, WinUI, WPF), as JAWS's viewer lists its elements and its Scan Cursor walks them
/// [obs, 1002/s2]; a classic window by its screen rows.
pub fn reads_by_elements(class: &str, framework: &str) -> bool {
    matches!(
        class,
        "ApplicationFrameWindow" | "Windows.UI.Core.CoreWindow" | "WinUIDesktopWin32WindowClass"
    ) || class.starts_with("HwndWrapper[")
        || matches!(framework, "XAML" | "WPF" | "WinUI")
}

/// Whether a window of class `class` shows files to select: File Explorer's, or the desktop's
/// (its icons are the shell's `Progman` or `WorkerW`).
pub fn is_file_view(class: &str) -> bool {
    matches!(
        class,
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
    )
}
