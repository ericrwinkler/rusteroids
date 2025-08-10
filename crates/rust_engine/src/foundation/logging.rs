//! Logging utilities and structured logging support

pub use log::{debug, info, warn, error, trace};

/// Initialize the logging system
pub fn init() {
    env_logger::init();
}
