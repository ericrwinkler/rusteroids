//! Pickable component for mouse selection
//!
//! Marks entities as pickable (selectable) via mouse ray casting.

use crate::ecs::Component;

/// Component marking an entity as pickable (selectable via mouse)
///
/// # Examples
/// ```
/// # use rust_engine::ecs::components::PickableComponent;
/// let pickable = PickableComponent::new()
///     .with_layer("world")
///     .with_layer_bits(0b0001);
/// ```
#[derive(Debug, Clone)]
pub struct PickableComponent {
    /// Whether this entity is currently pickable
    pub enabled: bool,
    
    /// Optional layer name for logical grouping (e.g., "ui", "world", "gizmo")
    pub layer: Option<String>,
    
    /// Layer bits for efficient filtering (bit mask)
    /// 
    /// Inspired by Unity and Unreal's layer systems. This allows for
    /// efficient broad-phase filtering using bitwise operations.
    /// 
    /// Example layer assignments:
    /// - 0b0001 (1) = World objects
    /// - 0b0010 (2) = UI elements
    /// - 0b0100 (4) = Gizmos/debug objects
    /// - 0b1000 (8) = Effects
    pub layer_bits: Option<u32>,
    
    /// Collision radius override (if None, uses component from collision shape)
    pub collision_radius: Option<f32>,
}

impl PickableComponent {
    /// Create a new pickable component with default settings
    pub fn new() -> Self {
        Self {
            enabled: true,
            layer: None,
            layer_bits: None,
            collision_radius: None,
        }
    }

    /// Set the enabled state
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set the layer name
    pub fn with_layer(mut self, layer: impl Into<String>) -> Self {
        self.layer = Some(layer.into());
        self
    }

    /// Set the layer bits for filtering
    pub fn with_layer_bits(mut self, bits: u32) -> Self {
        self.layer_bits = Some(bits);
        self
    }

    /// Set custom collision radius for picking
    pub fn with_collision_radius(mut self, radius: f32) -> Self {
        self.collision_radius = Some(radius);
        self
    }

    /// Enable this entity for picking
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable this entity for picking
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if entity matches a layer mask
    pub fn matches_layer_mask(&self, mask: u32) -> bool {
        if let Some(bits) = self.layer_bits {
            (mask & bits) != 0
        } else {
            true // No layer bits = matches all masks
        }
    }
}

impl Component for PickableComponent {}

impl Default for PickableComponent {
    fn default() -> Self {
        Self::new()
    }
}

/// Standard layer bit definitions
pub mod layers {
    /// World objects layer (bit 0)
    pub const WORLD: u32 = 0b0001;
    
    /// UI elements layer (bit 1)
    pub const UI: u32 = 0b0010;
    
    /// Debug gizmos layer (bit 2)
    pub const GIZMO: u32 = 0b0100;
    
    /// Effects layer (bit 3)
    pub const EFFECTS: u32 = 0b1000;
    
    /// All layers mask
    pub const ALL: u32 = 0xFFFFFFFF;
    
    /// None layers mask
    pub const NONE: u32 = 0x00000000;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_mask_matching() {
        let pickable = PickableComponent::new()
            .with_layer_bits(layers::WORLD | layers::GIZMO);

        // Should match world layer
        assert!(pickable.matches_layer_mask(layers::WORLD));
        
        // Should match gizmo layer
        assert!(pickable.matches_layer_mask(layers::GIZMO));
        
        // Should not match UI layer only
        assert!(!pickable.matches_layer_mask(layers::UI));
        
        // Should match combined mask
        assert!(pickable.matches_layer_mask(layers::WORLD | layers::UI));
    }

    #[test]
    fn test_default_matches_all() {
        let pickable = PickableComponent::new();
        
        // No layer bits means matches all masks
        assert!(pickable.matches_layer_mask(layers::WORLD));
        assert!(pickable.matches_layer_mask(layers::UI));
        assert!(pickable.matches_layer_mask(layers::ALL));
    }
}
