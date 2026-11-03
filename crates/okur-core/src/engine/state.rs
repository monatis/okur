//! Who reads the focus: the virtual cursor, a native control, the system's shell or an
//! application module.

/// Operating mode of the screen reader based on foreground application/container context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppMode {
    #[default]
    VirtualBuffer,
    NativeControl,
    /// The system's shell reads the focus (the desktop, the taskbar): the system moves and the
    /// focus is read, and the page commands have nothing to act on. The application module
    /// reading it names itself, so it knows whether the last focus was its own too.
    Shell(&'static str),
    /// An application module reads the focus its own way (File Explorer's files), under its
    /// own name.
    Application(&'static str),
}
