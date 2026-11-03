//! What goes wrong starting or stopping a platform's port.

#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Keyboard hook error: {0}")]
    Hook(String),
    #[error("Event source error: {0}")]
    EventSource(String),
}
