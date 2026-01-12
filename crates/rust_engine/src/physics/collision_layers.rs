//! Collision layer system for filtering collision detection
//! 
//! Based on Game Engine Architecture 3rd Edition, Section 13.3.8:
//! "Most games need to filter collisions... This is typically done via 
//! collision layers or groups."

/// Collision layer definitions using bitflags for efficient filtering
pub struct CollisionLayers;

impl CollisionLayers {
    /// No collision layer
    pub const NONE: u32 = 0;
    
    /// All collision layers
    pub const ALL: u32 = 0xFFFFFFFF;
    
    // Standard game entity layers (bits 0-7)
    /// Player character layer
    pub const PLAYER: u32 = 1 << 0;
    
    /// Enemy character layer
    pub const ENEMY: u32 = 1 << 1;
    
    /// Projectiles (bullets, missiles, etc.)
    pub const PROJECTILE: u32 = 1 << 2;
    
    /// Static environment geometry
    pub const ENVIRONMENT: u32 = 1 << 3;
    
    /// Trigger volumes (no physical response)
    pub const TRIGGER: u32 = 1 << 4;
    
    /// Debris and small physics objects
    pub const DEBRIS: u32 = 1 << 5;
    
    /// Vehicles
    pub const VEHICLE: u32 = 1 << 6;
    
    /// Pickups and collectibles
    pub const PICKUP: u32 = 1 << 7;
    
    // User-defined custom layers (bits 8-31)
    /// Custom collision layer 8 - user-defined
    pub const CUSTOM_8: u32 = 1 << 8;
    /// Custom collision layer 9 - user-defined
    pub const CUSTOM_9: u32 = 1 << 9;
    /// Custom collision layer 10 - user-defined
    pub const CUSTOM_10: u32 = 1 << 10;
    /// Custom collision layer 11 - user-defined
    pub const CUSTOM_11: u32 = 1 << 11;
    /// Custom collision layer 12 - user-defined
    pub const CUSTOM_12: u32 = 1 << 12;
    /// Custom collision layer 13 - user-defined
    pub const CUSTOM_13: u32 = 1 << 13;
    /// Custom collision layer 14 - user-defined
    pub const CUSTOM_14: u32 = 1 << 14;
    /// Custom collision layer 15 - user-defined
    pub const CUSTOM_15: u32 = 1 << 15;
    /// Custom collision layer 16 - user-defined
    pub const CUSTOM_16: u32 = 1 << 16;
    /// Custom collision layer 17 - user-defined
    pub const CUSTOM_17: u32 = 1 << 17;
    /// Custom collision layer 18 - user-defined
    pub const CUSTOM_18: u32 = 1 << 18;
    /// Custom collision layer 19 - user-defined
    pub const CUSTOM_19: u32 = 1 << 19;
    /// Custom collision layer 20 - user-defined
    pub const CUSTOM_20: u32 = 1 << 20;
    /// Custom collision layer 21 - user-defined
    pub const CUSTOM_21: u32 = 1 << 21;
    /// Custom collision layer 22 - user-defined
    pub const CUSTOM_22: u32 = 1 << 22;
    /// Custom collision layer 23 - user-defined
    pub const CUSTOM_23: u32 = 1 << 23;
    /// Custom collision layer 24 - user-defined
    pub const CUSTOM_24: u32 = 1 << 24;
    /// Custom collision layer 25 - user-defined
    pub const CUSTOM_25: u32 = 1 << 25;
    /// Custom collision layer 26 - user-defined
    pub const CUSTOM_26: u32 = 1 << 26;
    /// Custom collision layer 27 - user-defined
    pub const CUSTOM_27: u32 = 1 << 27;
    /// Custom collision layer 28 - user-defined
    pub const CUSTOM_28: u32 = 1 << 28;
    /// Custom collision layer 29 - user-defined
    pub const CUSTOM_29: u32 = 1 << 29;
    /// Custom collision layer 30 - user-defined
    pub const CUSTOM_30: u32 = 1 << 30;
    /// Custom collision layer 31 - user-defined
    pub const CUSTOM_31: u32 = 1 << 31;
    
    /// Check if two entities should collide based on their layers and masks
    /// 
    /// # Arguments
    /// * `layer_a` - Entity A's collision layer
    /// * `mask_a` - Entity A's collision mask (what it collides with)
    /// * `layer_b` - Entity B's collision layer
    /// * `mask_b` - Entity B's collision mask (what it collides with)
    /// 
    /// # Returns
    /// `true` if the entities should collide, `false` otherwise
    /// 
    /// # Example
    /// ```
    /// // Player collides with enemies and environment
    /// let player_layer = CollisionLayers::PLAYER;
    /// let player_mask = CollisionLayers::ENEMY | CollisionLayers::ENVIRONMENT;
    /// 
    /// // Enemy collides with player and projectiles
    /// let enemy_layer = CollisionLayers::ENEMY;
    /// let enemy_mask = CollisionLayers::PLAYER | CollisionLayers::PROJECTILE;
    /// 
    /// // Check if they should collide
    /// let should_collide = CollisionLayers::should_collide(
    ///     player_layer, player_mask,
    ///     enemy_layer, enemy_mask
    /// ); // Returns true
    /// ```
    pub fn should_collide(layer_a: u32, mask_a: u32, layer_b: u32, mask_b: u32) -> bool {
        // A's layer must be in B's mask AND B's layer must be in A's mask
        (layer_a & mask_b) != 0 && (layer_b & mask_a) != 0
    }
    
    /// Helper to create a mask from multiple layers
    /// 
    /// # Example
    /// ```
    /// let mask = CollisionLayers::mask(&[
    ///     CollisionLayers::PLAYER,
    ///     CollisionLayers::ENEMY,
    ///     CollisionLayers::ENVIRONMENT
    /// ]);
    /// ```
    pub fn mask(layers: &[u32]) -> u32 {
        layers.iter().fold(0, |acc, &layer| acc | layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_should_collide_mutual() {
        // Player and enemy should collide
        let player_layer = CollisionLayers::PLAYER;
        let player_mask = CollisionLayers::ENEMY;
        
        let enemy_layer = CollisionLayers::ENEMY;
        let enemy_mask = CollisionLayers::PLAYER;
        
        assert!(CollisionLayers::should_collide(
            player_layer, player_mask,
            enemy_layer, enemy_mask
        ));
    }
    
    #[test]
    fn test_should_not_collide_one_way() {
        // Player wants to collide with enemy, but enemy doesn't want to collide with player
        let player_layer = CollisionLayers::PLAYER;
        let player_mask = CollisionLayers::ENEMY;
        
        let enemy_layer = CollisionLayers::ENEMY;
        let enemy_mask = CollisionLayers::PROJECTILE; // Not player
        
        assert!(!CollisionLayers::should_collide(
            player_layer, player_mask,
            enemy_layer, enemy_mask
        ));
    }
    
    #[test]
    fn test_mask_creation() {
        let mask = CollisionLayers::mask(&[
            CollisionLayers::PLAYER,
            CollisionLayers::ENEMY,
            CollisionLayers::ENVIRONMENT
        ]);
        
        assert_eq!(
            mask,
            CollisionLayers::PLAYER | CollisionLayers::ENEMY | CollisionLayers::ENVIRONMENT
        );
    }
}
