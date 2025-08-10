//! Audio system (placeholder)

/// Audio system placeholder
pub struct AudioSystem {
    _enabled: bool,
}

impl AudioSystem {
    /// Create a new audio system
    pub fn new() -> Self {
        Self { _enabled: false }
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new()
    }
}
