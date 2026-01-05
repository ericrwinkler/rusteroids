//! Collision detection components for ECS
//!
//! Based on Game Engine Architecture 3rd Edition:
//! - Section 13.3: Collision Detection System
//! - Section 16.2: Component-based architecture

use crate::physics::collision::CollisionShape;
use crate::ecs::Entity;
use std::collections::HashSet;

/// Component that marks an entity as having collision detection enabled
/// 
/// Following GEA Section 16.2's component-based architecture, this component
/// stores all configuration data for an entity's collision behavior.
#[derive(Clone)]
pub struct ColliderComponent {
    /// The collision shape (sphere, mesh, etc.)
    pub shape: CollisionShape,
    
    /// Collision layer bitmask (what layer is this entity on?)
    /// See GEA 13.3.8: Collision filtering via layers
    pub layer: u32,
    
    /// Collision mask (what layers can this entity collide with?)
    pub mask: u32,
    
    /// Is this a trigger volume (generates events but no physical response)?
    pub is_trigger: bool,
    
    /// Cached bounding sphere radius for broad-phase culling
    /// GEA 13.3.2: "Hierarchical bounding volumes" for broad-phase optimization
    pub bounding_radius: f32,
    
    /// Should this collider be visualized in debug mode?
    pub debug_draw: bool,
}

impl ColliderComponent {
    /// Create a new collider with default settings
    pub fn new(shape: CollisionShape) -> Self {
        let bounding_radius = match &shape {
            CollisionShape::Sphere(sphere) => sphere.radius,
            CollisionShape::Mesh(mesh) => mesh.bounding_sphere.radius,
        };
        
        Self {
            shape,
            layer: 0xFFFFFFFF,  // Default: on all layers
            mask: 0xFFFFFFFF,   // Default: collides with all layers
            is_trigger: false,
            bounding_radius,
            debug_draw: false,
        }
    }
    
    /// Create a collider with specific layer and mask
    pub fn with_layers(mut self, layer: u32, mask: u32) -> Self {
        self.layer = layer;
        self.mask = mask;
        self
    }
    
    /// Mark this as a trigger volume
    pub fn as_trigger(mut self) -> Self {
        self.is_trigger = true;
        self
    }
    
    /// Enable debug visualization
    pub fn with_debug_draw(mut self, enabled: bool) -> Self {
        self.debug_draw = enabled;
        self
    }
}

/// Component that tracks the current collision state of an entity
/// 
/// Following GEA 13.3.10: "Collision event callbacks"
/// This component is updated each frame by the collision system to reflect
/// which entities are currently colliding with this entity.
#[derive(Default, Clone)]
pub struct CollisionStateComponent {
    /// All entities we're currently colliding with
    pub colliding_with: HashSet<Entity>,
    
    /// Entities we started colliding with this frame (entered collision)
    pub collision_entered: Vec<Entity>,
    
    /// Entities we stopped colliding with this frame (exited collision)
    pub collision_exited: Vec<Entity>,
    
    /// Entities within broad-phase query range (potential collision candidates)
    /// Useful for debugging and for AI "awareness" systems
    pub nearby_entities: HashSet<Entity>,
}

impl CollisionStateComponent {
    /// Check if we're currently colliding with any entity
    pub fn is_colliding(&self) -> bool {
        !self.colliding_with.is_empty()
    }
    
    /// Check if we're colliding with a specific entity
    pub fn is_colliding_with(&self, entity: Entity) -> bool {
        self.colliding_with.contains(&entity)
    }
    
    /// Get the number of entities we're colliding with
    pub fn collision_count(&self) -> usize {
        self.colliding_with.len()
    }
    
    /// Check if we just started colliding with a specific entity this frame
    pub fn just_collided_with(&self, entity: Entity) -> bool {
        self.collision_entered.iter().any(|&e| e == entity)
    }
    
    /// Check if we just stopped colliding with a specific entity this frame
    pub fn just_stopped_colliding_with(&self, entity: Entity) -> bool {
        self.collision_exited.iter().any(|&e| e == entity)
    }
    
    /// Clear per-frame data (called by collision system at start of update)
    pub(crate) fn clear_frame_data(&mut self) {
        self.collision_entered.clear();
        self.collision_exited.clear();
    }
}
