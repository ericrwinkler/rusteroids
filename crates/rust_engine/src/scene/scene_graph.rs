//! Scene graph trait and implementations
//!
//! Provides pluggable spatial data structures for scene management.
//! Following Game Engine Architecture Chapter 11.2.7.4 - Scene Graphs.

use crate::ecs::Entity;
use crate::foundation::math::{Vec3, Mat4};

/// Axis-Aligned Bounding Box for spatial queries
#[derive(Debug, Clone, Copy)]
pub struct AABB {
    /// Minimum corner of the bounding box
    pub min: Vec3,
    /// Maximum corner of the bounding box
    pub max: Vec3,
}

impl AABB {
    /// Create a new AABB from min and max points
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }
    
    /// Create an AABB centered at a point with given extents
    pub fn from_center_extents(center: Vec3, extents: Vec3) -> Self {
        Self {
            min: center - extents,
            max: center + extents,
        }
    }
    
    /// Get the center of the AABB
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    
    /// Get the extents (half-size) of the AABB
    pub fn extents(&self) -> Vec3 {
        (self.max - self.min) * 0.5
    }
    
    /// Check if this AABB contains a point
    pub fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
    
    /// Check if this AABB intersects another AABB
    pub fn intersects(&self, other: &AABB) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x &&
        self.min.y <= other.max.y && self.max.y >= other.min.y &&
        self.min.z <= other.max.z && self.max.z >= other.min.z
    }
    
    /// Test ray intersection with this AABB using slab method
    /// Returns the distance to the entry point if the ray intersects, None otherwise
    /// Based on "An Efficient and Robust Ray–Box Intersection Algorithm"
    pub fn intersect_ray(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<f32> {
        let inv_dir = Vec3::new(
            if ray_dir.x != 0.0 { 1.0 / ray_dir.x } else { f32::INFINITY },
            if ray_dir.y != 0.0 { 1.0 / ray_dir.y } else { f32::INFINITY },
            if ray_dir.z != 0.0 { 1.0 / ray_dir.z } else { f32::INFINITY },
        );
        
        let t1 = (self.min.x - ray_origin.x) * inv_dir.x;
        let t2 = (self.max.x - ray_origin.x) * inv_dir.x;
        let t3 = (self.min.y - ray_origin.y) * inv_dir.y;
        let t4 = (self.max.y - ray_origin.y) * inv_dir.y;
        let t5 = (self.min.z - ray_origin.z) * inv_dir.z;
        let t6 = (self.max.z - ray_origin.z) * inv_dir.z;
        
        let tmin = t1.min(t2).max(t3.min(t4)).max(t5.min(t6));
        let tmax = t1.max(t2).min(t3.max(t4)).min(t5.max(t6));
        
        // Ray intersects if tmax >= tmin and tmax >= 0
        if tmax >= tmin && tmax >= 0.0 {
            // Return entry point distance (or 0 if we're inside the box)
            Some(tmin.max(0.0))
        } else {
            None
        }
    }
}

/// Frustum for visibility culling
#[derive(Debug, Clone)]
pub struct Frustum {
    /// Six planes defining the frustum (left, right, top, bottom, near, far)
    pub planes: [Plane; 6],
}

impl Frustum {
    /// Create a frustum from six planes
    pub fn new(planes: [Plane; 6]) -> Self {
        Self { planes }
    }
    
    /// Extract frustum planes from a view-projection matrix
    ///
    /// This uses the Gribb-Hartmann method to extract frustum planes
    /// from the combined view-projection matrix.
    pub fn from_matrix(_vp_matrix: &Mat4) -> Self {
        // TODO: Implement Gribb-Hartmann frustum extraction
        // For now, return a frustum with degenerate planes (no culling)
        let degenerate_plane = Plane {
            normal: Vec3::new(0.0, 0.0, 0.0),
            distance: 0.0,
        };
        Self {
            planes: [degenerate_plane; 6],
        }
    }
    
    /// Check if an AABB is inside or intersects the frustum
    pub fn intersects_aabb(&self, aabb: &AABB) -> bool {
        // For each plane, check if the AABB is completely outside
        for plane in &self.planes {
            // Get the point on the AABB closest to the plane
            let mut p = aabb.min;
            if plane.normal.x >= 0.0 { p.x = aabb.max.x; }
            if plane.normal.y >= 0.0 { p.y = aabb.max.y; }
            if plane.normal.z >= 0.0 { p.z = aabb.max.z; }
            
            // If this point is outside the plane, the entire AABB is outside
            if plane.distance_to_point(p) < 0.0 {
                return false;
            }
        }
        
        // AABB is inside or intersecting the frustum
        true
    }
}

/// Plane defined by normal and distance from origin
#[derive(Debug, Clone, Copy)]
pub struct Plane {
    /// Normal vector (should be normalized)
    pub normal: Vec3,
    /// Distance from origin along the normal
    pub distance: f32,
}

impl Plane {
    /// Create a new plane from normal and distance
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal: normal.normalize(), distance }
    }
    
    /// Calculate signed distance from plane to point
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(&point) + self.distance
    }
}

/// Trait for spatial data structures used in scene management
///
/// Allows pluggable scene graph implementations (list, quadtree, octree, BSP, etc.)
/// Following Game Engine Architecture Chapter 11.2.7.4 pattern.
pub trait SceneGraph: Send + Sync {
    /// Add an entity to the scene graph with its bounding volume
    fn add(&mut self, entity: Entity, bounds: AABB);
    
    /// Remove an entity from the scene graph
    fn remove(&mut self, entity: Entity);
    
    /// Update an entity's bounding volume (after transform change)
    fn update(&mut self, entity: Entity, bounds: AABB);
    
    /// Query all entities visible within a frustum
    fn query_visible(&self, frustum: &Frustum) -> Vec<Entity>;
    
    /// Query all entities within a radius of a point
    fn query_radius(&self, center: Vec3, radius: f32) -> Vec<Entity>;
    
    /// Get the total number of entities in the scene graph
    fn entity_count(&self) -> usize;
    
    /// Clear all entities from the scene graph
    fn clear(&mut self);
}

/// Simple list-based scene graph (no spatial optimization)
///
/// This is the initial implementation - performs linear search for all queries.
/// Sufficient for small scenes (<1000 objects). Can be replaced with octree/
/// quadtree later without changing the API.
#[derive(Debug)]
pub struct SimpleListGraph {
    /// List of entities with their bounding volumes
    entities: Vec<(Entity, AABB)>,
}

impl SimpleListGraph {
    /// Create a new empty scene graph
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Default for SimpleListGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneGraph for SimpleListGraph {
    fn add(&mut self, entity: Entity, bounds: AABB) {
        self.entities.push((entity, bounds));
    }
    
    fn remove(&mut self, entity: Entity) {
        self.entities.retain(|(e, _)| *e != entity);
    }
    
    fn update(&mut self, entity: Entity, bounds: AABB) {
        if let Some(entry) = self.entities.iter_mut().find(|(e, _)| *e == entity) {
            entry.1 = bounds;
        }
    }
    
    fn query_visible(&self, frustum: &Frustum) -> Vec<Entity> {
        self.entities
            .iter()
            .filter(|(_, bounds)| frustum.intersects_aabb(bounds))
            .map(|(entity, _)| *entity)
            .collect()
    }
    
    fn query_radius(&self, center: Vec3, radius: f32) -> Vec<Entity> {
        let radius_squared = radius * radius;
        self.entities
            .iter()
            .filter(|(_, bounds)| {
                let closest_point = Vec3::new(
                    bounds.min.x.max(center.x.min(bounds.max.x)),
                    bounds.min.y.max(center.y.min(bounds.max.y)),
                    bounds.min.z.max(center.z.min(bounds.max.z)),
                );
                (closest_point - center).magnitude_squared() <= radius_squared
            })
            .map(|(entity, _)| *entity)
            .collect()
    }
    
    fn entity_count(&self) -> usize {
        self.entities.len()
    }
    
    fn clear(&mut self) {
        self.entities.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_aabb_contains_point() {
        let aabb = AABB::new(
            Vec3::new(-1.0, -1.0, -1.0),
            Vec3::new(1.0, 1.0, 1.0),
        );
        
        assert!(aabb.contains_point(Vec3::zeros()));
        assert!(aabb.contains_point(Vec3::new(0.5, 0.5, 0.5)));
        assert!(!aabb.contains_point(Vec3::new(2.0, 0.0, 0.0)));
    }
    
    #[test]
    fn test_aabb_intersects() {
        let aabb1 = AABB::new(
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 2.0, 2.0),
        );
        
        let aabb2 = AABB::new(
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(3.0, 3.0, 3.0),
        );
        
        let aabb3 = AABB::new(
            Vec3::new(5.0, 5.0, 5.0),
            Vec3::new(7.0, 7.0, 7.0),
        );
        
        assert!(aabb1.intersects(&aabb2));
        assert!(!aabb1.intersects(&aabb3));
    }
    
    #[test]
    fn test_simple_list_graph_add_remove() {
        use crate::ecs::World;
        
        let mut graph = SimpleListGraph::new();
        let mut world = World::new();
        let entity1 = world.create_entity();
        let entity2 = world.create_entity();
        
        let bounds = AABB::new(Vec3::zeros(), Vec3::new(1.0, 1.0, 1.0));
        
        graph.add(entity1, bounds);
        graph.add(entity2, bounds);
        assert_eq!(graph.entity_count(), 2);
        
        graph.remove(entity1);
        assert_eq!(graph.entity_count(), 1);
    }
}
