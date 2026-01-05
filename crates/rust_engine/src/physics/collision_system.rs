//! Core collision detection system
//!
//! Based on Game Engine Architecture 3rd Edition, Chapter 13:
//! "The collision detection system is typically split into two phases:
//! broad-phase and narrow-phase."
//!
//! This module implements a pure collision system that doesn't depend on ECS,
//! allowing it to be used in different contexts. The ECS wrapper is in
//! ecs/systems/collision_system.rs

use crate::spatial::spatial_query::SpatialQuery;
use crate::ecs::Entity;
use crate::physics::collision::CollisionShape;
use crate::physics::collision_layers::CollisionLayers;
use std::collections::{HashMap, HashSet};

/// Collision pair representing two entities that are colliding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CollisionPair {
    pub entity_a: Entity,
    pub entity_b: Entity,
}

impl CollisionPair {
    /// Create a new collision pair (always stores smaller entity ID first for consistency)
    pub fn new(entity_a: Entity, entity_b: Entity) -> Self {
        if entity_a.id() < entity_b.id() {
            Self { entity_a, entity_b }
        } else {
            Self { entity_a: entity_b, entity_b: entity_a }
        }
    }
}

/// Collider data stored by the collision system
#[derive(Clone)]
struct ColliderData {
    shape: CollisionShape,
    layer: u32,
    mask: u32,
    is_trigger: bool,
}

/// Core collision detection system (GEA 13.3)
/// 
/// This system manages collision detection between entities using a two-phase
/// approach: broad-phase (spatial query) and narrow-phase (shape intersection).
pub struct PhysicsCollisionSystem {
    /// Spatial partitioning structure for broad-phase
    spatial_query: Box<dyn SpatialQuery>,
    
    /// Collider data for each entity
    colliders: HashMap<Entity, ColliderData>,
    
    /// Collision pairs from the current frame
    current_pairs: HashSet<CollisionPair>,
    
    /// Collision pairs from the previous frame
    previous_pairs: HashSet<CollisionPair>,
    
    /// Enable debug mode
    pub debug_enabled: bool,
}

impl PhysicsCollisionSystem {
    /// Create a new collision system with the given spatial query implementation
    pub fn new(spatial_query: Box<dyn SpatialQuery>) -> Self {
        Self {
            spatial_query,
            colliders: HashMap::new(),
            current_pairs: HashSet::new(),
            previous_pairs: HashSet::new(),
            debug_enabled: false,
        }
    }
    
    /// Register a collider (called when ColliderComponent is added to an entity)
    pub fn register_collider(
        &mut self,
        entity: Entity,
        shape: CollisionShape,
        layer: u32,
        mask: u32,
        is_trigger: bool,
        bounding_radius: f32,
    ) {
        // Get position from shape center
        let position = shape.center();
        
        // Add to spatial structure
        self.spatial_query.insert(entity, position, bounding_radius);
        
        // Store collider data
        self.colliders.insert(entity, ColliderData {
            shape,
            layer,
            mask,
            is_trigger,
        });
    }
    
    /// Unregister a collider (called when ColliderComponent is removed)
    pub fn unregister_collider(&mut self, entity: Entity) {
        self.spatial_query.remove(entity);
        self.colliders.remove(&entity);
    }
    
    /// Update a collider's position (called when TransformComponent changes)
    pub fn update_collider_position(
        &mut self,
        entity: Entity,
        shape: CollisionShape,
        bounding_radius: f32,
    ) {
        let position = shape.center();
        self.spatial_query.update(entity, position, bounding_radius);
        
        // Update stored shape
        if let Some(collider) = self.colliders.get_mut(&entity) {
            collider.shape = shape;
        }
    }
    
    /// Perform collision detection update (broad-phase + narrow-phase)
    /// Returns collision pairs for this frame
    pub fn detect_collisions(&mut self) -> &HashSet<CollisionPair> {
        // Move current pairs to previous
        std::mem::swap(&mut self.current_pairs, &mut self.previous_pairs);
        self.current_pairs.clear();
        
        // Phase 1: Broad-phase - get potential collision pairs from spatial query
        let potential_pairs = self.broad_phase();
        
        // Phase 2: Narrow-phase - test actual shape intersections
        self.narrow_phase(potential_pairs);
        
        &self.current_pairs
    }
    
    /// Broad-phase: Use spatial query to find potential collision pairs
    /// GEA 13.3.1: "The broad phase quickly identifies pairs of objects that
    /// might be colliding using some kind of spatial partitioning scheme."
    fn broad_phase(&self) -> Vec<CollisionPair> {
        let mut potential_pairs = HashSet::new();
        
        // For each registered collider
        for (&entity, collider) in &self.colliders {
            // Query nearby entities from spatial structure
            let nearby = self.spatial_query.query_nearby(entity);
            
            // Create pairs with layer filtering
            for nearby_entity in nearby {
                // Skip self
                if entity == nearby_entity {
                    continue;
                }
                
                // Check if the other entity has a collider
                if let Some(other_collider) = self.colliders.get(&nearby_entity) {
                    // Apply layer filtering (GEA 13.3.8)
                    if !CollisionLayers::should_collide(
                        collider.layer,
                        collider.mask,
                        other_collider.layer,
                        other_collider.mask,
                    ) {
                        continue;
                    }
                    
                    // Add to potential pairs (automatically handles duplicates via HashSet)
                    potential_pairs.insert(CollisionPair::new(entity, nearby_entity));
                }
            }
        }
        
        potential_pairs.into_iter().collect()
    }
    
    /// Narrow-phase: Test actual shape intersections
    /// GEA 13.3.4: "The narrow phase performs detailed shape-to-shape tests"
    fn narrow_phase(&mut self, potential_pairs: Vec<CollisionPair>) {
        for pair in potential_pairs {
            // Get collision shapes
            let shape_a = match self.colliders.get(&pair.entity_a) {
                Some(collider) => &collider.shape,
                None => continue,
            };
            
            let shape_b = match self.colliders.get(&pair.entity_b) {
                Some(collider) => &collider.shape,
                None => continue,
            };
            
            // Test intersection
            if shape_a.intersects(shape_b) {
                self.current_pairs.insert(pair);
            }
        }
    }
    
    /// Get entities that entered collision this frame
    pub fn get_collision_entered(&self) -> Vec<CollisionPair> {
        self.current_pairs
            .difference(&self.previous_pairs)
            .copied()
            .collect()
    }
    
    /// Get entities that exited collision this frame
    pub fn get_collision_exited(&self) -> Vec<CollisionPair> {
        self.previous_pairs
            .difference(&self.current_pairs)
            .copied()
            .collect()
    }
    
    /// Get all current collision pairs
    pub fn get_current_collisions(&self) -> &HashSet<CollisionPair> {
        &self.current_pairs
    }
    
    /// Query nearby entities for a specific entity
    pub fn query_nearby(&self, entity: Entity) -> Vec<Entity> {
        self.spatial_query.query_nearby(entity)
    }
    
    /// Get spatial query for direct access (e.g., for visualization)
    pub fn spatial_query(&self) -> &dyn SpatialQuery {
        self.spatial_query.as_ref()
    }
    
    /// Get collider shape for an entity
    pub fn get_collider_shape(&self, entity: Entity) -> Option<&CollisionShape> {
        self.colliders.get(&entity).map(|c| &c.shape)
    }
    
    /// Check if an entity is registered
    pub fn has_collider(&self, entity: Entity) -> bool {
        self.colliders.contains_key(&entity)
    }
    
    /// Get the number of registered colliders
    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }
    
    /// Clear all collision data
    pub fn clear(&mut self) {
        self.spatial_query.clear();
        self.colliders.clear();
        self.current_pairs.clear();
        self.previous_pairs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::{Octree, OctreeConfig, OctreeSpatialQuery};
    use crate::scene::AABB;
    use crate::physics::collision::BoundingSphere;
    use crate::foundation::math::Vec3;
    
    fn create_test_system() -> PhysicsCollisionSystem {
        let bounds = AABB::new(
            Vec3::new(-100.0, -100.0, -100.0),
            Vec3::new(100.0, 100.0, 100.0),
        );
        let config = OctreeConfig {
            max_entities_per_node: 4,
            max_depth: 5,
            min_node_size: 5.0,
        };
        let octree = Octree::new(bounds, config);
        let spatial = Box::new(OctreeSpatialQuery::new(octree));
        
        PhysicsCollisionSystem::new(spatial)
    }
    
    #[test]
    fn test_collision_detection() {
        let mut system = create_test_system();
        
        // Create two overlapping spheres
        let entity_a = Entity::new(1, 0);
        let entity_b = Entity::new(2, 0);
        
        let shape_a = CollisionShape::Sphere(BoundingSphere::new(
            Vec3::new(0.0, 0.0, 0.0),
            5.0,
        ));
        let shape_b = CollisionShape::Sphere(BoundingSphere::new(
            Vec3::new(8.0, 0.0, 0.0),
            5.0,
        ));
        
        system.register_collider(
            entity_a,
            shape_a,
            CollisionLayers::ALL,
            CollisionLayers::ALL,
            false,
            5.0,
        );
        
        system.register_collider(
            entity_b,
            shape_b,
            CollisionLayers::ALL,
            CollisionLayers::ALL,
            false,
            5.0,
        );
        
        // Detect collisions
        let collisions = system.detect_collisions();
        
        // Should detect collision
        assert_eq!(collisions.len(), 1);
        let pair = collisions.iter().next().unwrap();
        assert!(
            (pair.entity_a == entity_a && pair.entity_b == entity_b)
                || (pair.entity_a == entity_b && pair.entity_b == entity_a)
        );
    }
    
    #[test]
    fn test_layer_filtering() {
        let mut system = create_test_system();
        
        // Create two overlapping spheres on different layers
        let entity_a = Entity::new(1, 0);
        let entity_b = Entity::new(2, 0);
        
        let shape_a = CollisionShape::Sphere(BoundingSphere::new(
            Vec3::new(0.0, 0.0, 0.0),
            5.0,
        ));
        let shape_b = CollisionShape::Sphere(BoundingSphere::new(
            Vec3::new(8.0, 0.0, 0.0),
            5.0,
        ));
        
        // Entity A is on PLAYER layer, only collides with ENEMY
        system.register_collider(
            entity_a,
            shape_a,
            CollisionLayers::PLAYER,
            CollisionLayers::ENEMY,
            false,
            5.0,
        );
        
        // Entity B is on ENVIRONMENT layer (not ENEMY)
        system.register_collider(
            entity_b,
            shape_b,
            CollisionLayers::ENVIRONMENT,
            CollisionLayers::ALL,
            false,
            5.0,
        );
        
        // Detect collisions
        let collisions = system.detect_collisions();
        
        // Should NOT detect collision due to layer filtering
        assert_eq!(collisions.len(), 0);
    }
}
