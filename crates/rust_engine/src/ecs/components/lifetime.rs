//! Lifetime Component
//!
//! Tracks entity lifetime for automatic despawning.
//! Moved from render/systems/dynamic/spawner.rs as this is game logic.

/// Lifetime component for entities that should despawn after a duration
#[derive(Debug, Clone, Copy)]
pub struct Lifetime {
    /// Time when the entity was created (in seconds since game start)
    pub created_at: f32,
    /// How long the entity should live (in seconds)
    pub duration: f32,
}

impl Lifetime {
    /// Create a new lifetime component
    pub fn new(created_at: f32, duration: f32) -> Self {
        Self {
            created_at,
            duration,
        }
    }

    /// Check if this entity's lifetime has expired
    pub fn is_expired(&self, current_time: f32) -> bool {
        if self.duration <= 0.0 {
            false // Infinite lifetime
        } else {
            current_time >= self.created_at + self.duration
        }
    }

    /// Get remaining lifetime in seconds
    pub fn remaining(&self, current_time: f32) -> f32 {
        if self.duration <= 0.0 {
            f32::INFINITY
        } else {
            (self.created_at + self.duration - current_time).max(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifetime_not_expired() {
        let lifetime = Lifetime::new(0.0, 5.0);
        assert!(!lifetime.is_expired(2.0));
    }

    #[test]
    fn test_lifetime_expired() {
        let lifetime = Lifetime::new(0.0, 5.0);
        assert!(lifetime.is_expired(6.0));
    }

    #[test]
    fn test_infinite_lifetime() {
        let lifetime = Lifetime::new(0.0, 0.0);
        assert!(!lifetime.is_expired(1000.0));
    }

    #[test]
    fn test_remaining_time() {
        let lifetime = Lifetime::new(0.0, 10.0);
        assert_eq!(lifetime.remaining(3.0), 7.0);
        assert_eq!(lifetime.remaining(12.0), 0.0);
    }
}
