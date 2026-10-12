pub mod bridge;

pub use bridge::{PlatformBridge, PlatformError};

pub fn init() {
    tracing::info!("sr-platform initialized");
}
