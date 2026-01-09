//! Abstract spatial query interface for broad-phase collision detection
//!
//! Based on Game Engine Architecture 3rd Edition, Section 13.3.2:
//! "Spatial partitioning schemes... allow us to quickly cull out pairs of
//! objects that cannot possibly be colliding."
//!
//! This abstraction allows swapping different spatial partitioning schemes
//! (octree, grid, BVH, etc.) without changing the collision system.

use crate::ecs::Entity;
use crate::foundation::math::Vec3;
use crate::scene::AABB;
use crate::spatial::Octree;
use std::collections::HashMap;
use std::any::Any;

/// Abstract interface for spatial partitioning used in broad-phase collision detection
/// 
/// GEA 13.3.2: "The broad phase quickly identifies pairs of objects that might
/// be colliding using some kind of spatial partitioning scheme."
pub trait SpatialQuery: Send + Sync {
    /// Insert an entity at a position with a bounding radius
    fn insert(&mut self, entity: Entity, position: Vec3, radius: f32);
    
    /// Remove an entity from the spatial structure
    fn remove(&mut self, entity: Entity);
    
    /// Update an entity's position in the spatial structure
    fn update(&mut self, entity: Entity, position: Vec3, radius: f32);
    
    /// Query entities near a specific entity (for collision detection)
    /// Returns entities that are potential collision candidates
    fn query_nearby(&self, entity: Entity) -> Vec<Entity>;
    
    /// Query entities within a sphere
    fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<Entity>;
    
    /// Query entities within an AABB
    fn query_aabb(&self, aabb: &AABB) -> Vec<Entity>;
    
    /// Get entity's current position and radius (if it exists)
    fn get_entity_data(&self, entity: Entity) -> Option<(Vec3, f32)>;
    
    /// Clear all entities from the spatial structure
    fn clear(&mut self);
    
    /// Get the number of entities in the structure
    fn entity_count(&self) -> usize;
    
    /// Downcast to Any for type-specific access (e.g., OctreeSpatialQuery)
    fn as_any(&self) -> &dyn Any;
    
    /// Downcast to Any for mutable type-specific access
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Octree-based implementation of SpatialQuery
/// 
/// Wraps the existing Octree implementation to provide the SpatialQuery interface
pub struct OctreeSpatialQuery {
    octree: Octree,
    /// Cache of entity data for quick lookups
    entity_cache: HashMap<Entity, (Vec3, f32)>,
}

impl OctreeSpatialQuery {
    /// Create a new octree-based spatial query system
    pub fn new(octree: Octree) -> Self {
        Self {
            octree,
            entity_cache: HashMap::new(),
        }
    }
    
    /// Get a reference to the underlying octree (for visualization, etc.)
    pub fn octree(&self) -> &Octree {
        &self.octree
    }
    
    /// Get a mutable reference to the underlying octree
    pub fn octree_mut(&mut self) -> &mut Octree {
        &mut self.octree
    }
}

impl SpatialQuery for OctreeSpatialQuery {
    fn insert(&mut self, entity: Entity, position: Vec3, radius: f32) {
        self.octree.insert(entity, position, radius);
        self.entity_cache.insert(entity, (position, radius));
    }
    
    fn remove(&mut self, entity: Entity) {
        self.octree.remove(entity);
        self.entity_cache.remove(&entity);
    }
    
    fn update(&mut self, entity: Entity, position: Vec3, radius: f32) {
        // Octree requires remove + re-insert for updates
        self.octree.remove(entity);
        self.octree.insert(entity, position, radius);
        self.entity_cache.insert(entity, (position, radius));
    }
    
    fn query_nearby(&self, entity: Entity) -> Vec<Entity> {
        self.octree.query_nearby(entity)
    }
    
    fn query_sphere(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        self.octree.query_radius(center, radius)
            .into_iter()
            .map(|octree_entity| octree_entity.id)
            .collect()
    }
    
    fn query_aabb(&self, aabb: &AABB) -> Vec<Entity> {
        // For octree, approximate AABB query with sphere query
        let center = aabb.center();
        let extents = aabb.extents();
        let radius = extents.magnitude(); // Conservative sphere encompassing the AABB
        
        self.query_sphere(center, radius)
            .into_iter()
            .filter(|&entity| {
                // Further filter by actual AABB intersection
                if let Some((pos, r)) = self.entity_cache.get(&entity) {
                    let entity_aabb = AABB::new(
                        *pos - Vec3::new(*r, *r, *r),
                        *pos + Vec3::new(*r, *r, *r),
                    );
                    aabb.intersects(&entity_aabb)
                } else {
                    false
                }
            })
            .collect()
    }
    
    fn get_entity_data(&self, entity: Entity) -> Option<(Vec3, f32)> {
        self.entity_cache.get(&entity).copied()
    }
    
    fn clear(&mut self) {
        self.octree.clear();
        self.entity_cache.clear();
    }
    
    fn entity_count(&self) -> usize {
        self.octree.entity_count()
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::OctreeConfig;
    
    #[test]
    fn test_spatial_query_insert_remove() {
        let bounds = AABB::new(
            Vec3::new(-50.0, -50.0, -50.0),
            Vec3::new(50.0, 50.0, 50.0),
        );
        let config = OctreeConfig {
            max_entities_per_node: 4,
            max_depth: 5,
            min_node_size: 5.0,
        };
        let octree = Octree::new(bounds, config);
        let mut spatial = OctreeSpatialQuery::new(octree);
        
        let entity = Entity::new(1, 0);
        let position = Vec3::zeros();
        let radius = 5.0;
        
        spatial.insert(entity, position, radius);
        assert_eq!(spatial.entity_count(), 1);
        
        spatial.remove(entity);
        assert_eq!(spatial.entity_count(), 0);
    }
}
