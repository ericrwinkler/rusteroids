//! Spatial audio system
//!
//! Provides camera-relative audio positioning for RTS gameplay.
//! Handles distance-based attenuation and on-screen culling.

use crate::foundation::math::Vec2;

/// Configuration for spatial audio
#[derive(Debug, Clone)]
pub struct SpatialConfig {
    /// Maximum audible distance from listener
    pub max_distance: f32,
    /// Reference distance for attenuation (no falloff)
    pub reference_distance: f32,
    /// Falloff exponent (1.0 = linear, 2.0 = inverse square)
    pub falloff_exponent: f32,
}

impl Default for SpatialConfig {
    fn default() -> Self {
        Self {
            max_distance: 1000.0,
            reference_distance: 100.0,
            falloff_exponent: 1.0,
        }
    }
}

/// Spatial audio system for camera-relative positioning
pub struct SpatialAudio {
    config: SpatialConfig,
    listener_position: Vec2,
}

impl SpatialAudio {
    /// Create a new spatial audio system
    pub fn new(config: SpatialConfig) -> Self {
        Self {
            config,
            listener_position: Vec2::new(0.0, 0.0),
        }
    }
    
    /// Update the spatial audio system
    pub fn update(&mut self) {
        // TODO: Update active spatial sounds
    }
    
    /// Set the listener (camera) position
    pub fn set_listener_position(&mut self, position: Vec2) {
        self.listener_position = position;
    }
    
    /// Calculate attenuation factor for a sound at given position
    pub fn calculate_attenuation(&self, sound_position: Vec2) -> f32 {
        let distance = (sound_position - self.listener_position).norm();
        
        // Beyond max distance, sound is inaudible
        if distance > self.config.max_distance {
            return 0.0;
        }
        
        // Within reference distance, no attenuation
        if distance <= self.config.reference_distance {
            return 1.0;
        }
        
        // Apply falloff curve
        let normalized_distance = (distance - self.config.reference_distance)
            / (self.config.max_distance - self.config.reference_distance);
        
        let attenuation: f32 = 1.0 - normalized_distance.powf(self.config.falloff_exponent);
        attenuation.max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_attenuation_at_reference_distance() {
        let spatial = SpatialAudio::new(SpatialConfig::default());
        let attenuation = spatial.calculate_attenuation(Vec2::new(100.0, 0.0));
        assert_eq!(attenuation, 1.0);
    }
    
    #[test]
    fn test_attenuation_beyond_max_distance() {
        let spatial = SpatialAudio::new(SpatialConfig::default());
        let attenuation = spatial.calculate_attenuation(Vec2::new(2000.0, 0.0));
        assert_eq!(attenuation, 0.0);
    }
}
