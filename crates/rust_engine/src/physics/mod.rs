//! Physics system (placeholder)

/// Physics system placeholder
pub struct PhysicsSystem {
    _enabled: bool,
}

impl PhysicsSystem {
    /// Create a new physics system
    pub fn new() -> Self {
        Self { _enabled: false }
    }
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self::new()
    }
}
