//! Navigation state the engine keeps: the desktop, the taskbar and File
//! Explorer.

/// Operating mode of the screen reader based on foreground application/container context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    VirtualBuffer,
    NativeDesktop,
    NativeShell,
    NativeExplorer,
    NativeControl,
}
