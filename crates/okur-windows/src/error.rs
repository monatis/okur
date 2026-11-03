//! What goes wrong in a call to Windows.

#[derive(Debug, thiserror::Error)]
pub enum PlatformError {
    #[error("xa11y error: {0}")]
    Xa11y(#[from] xa11y::Error),
    #[error("No accessible window found")]
    NoWindowFound,
    #[error("No accessible document found in application")]
    NoDocumentFound,
    #[error("Event listener error: {0}")]
    EventListenerError(String),
    #[error("Audio error: {0}")]
    AudioError(String),
    #[error("Unsupported platform: {0}")]
    Unsupported(String),
    #[error("Recognition error: {0}")]
    Recognition(String),
}
