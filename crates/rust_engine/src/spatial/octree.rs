//! Octree spatial partitioning structure
//!
//! Efficiently divides 3D space into hierarchical regions for fast
//! spatial queries. Each node subdivides into 8 octants when entity
//! density exceeds a threshold.

use crate::ecs::Entity;
use crate::foundation::math::Vec3;
use crate::scene::AABB;

/// Configuration for octree behavior
#[derive(Debug, Clone)]
pub struct OctreeConfig {
    /// Maximum entities per node before subdivision
    pub max_entities_per_node: usize,
    
    /// Maximum subdivision depth
    pub max_depth: u32,
    
    /// Minimum node size (prevents excessive subdivision)
    pub min_node_size: f32,
}

impl Default for OctreeConfig {
    fn default() -> Self {
        Self {
            max_entities_per_node: 8,
            max_depth: 8,
            min_node_size: 1.0,
        }
    }
}

/// Entity stored in octree with position and optional radius
#[derive(Debug, Clone, Copy)]
pub struct OctreeEntity {
    pub id: Entity,
    pub position: Vec3,
    pub radius: f32,
}

/// Single node in the octree hierarchy
#[derive(Debug, Clone)]
pub struct OctreeNode {
    /// World-space bounds of this node
    pub bounds: AABB,
    
    /// Entities contained in this node (if leaf)
    pub entities: Vec<OctreeEntity>,
    
    /// Child nodes (8 octants), None if this is a leaf
    pub children: Option<Box<[OctreeNode; 8]>>,
    
    /// Depth in the tree (0 = root)
    pub depth: u32,
}

impl OctreeNode {
    /// Create a new leaf node
    pub fn new(bounds: AABB, depth: u32) -> Self {
        Self {
            bounds,
            entities: Vec::new(),
            children: None,
            depth,
        }
    }
    
    /// Check if this node is a leaf (has no children)
    pub fn is_leaf(&self) -> bool {
        self.children.is_none()
    }
    
    /// Get the octant index (0-7) for a position within this node's bounds
    #[allow(dead_code)]
    fn get_octant_index(&self, position: Vec3) -> usize {
        let center = self.bounds.center();
        let x_bit = if position.x >= center.x { 1 } else { 0 };
        let y_bit = if position.y >= center.y { 1 } else { 0 };
        let z_bit = if position.z >= center.z { 1 } else { 0 };
        
        // Octant layout:
        // 0: -X, -Y, -Z (back-bottom-left)
        // 1: +X, -Y, -Z (back-bottom-right)
        // 2: -X, +Y, -Z (back-top-left)
        // 3: +X, +Y, -Z (back-top-right)
        // 4: -X, -Y, +Z (front-bottom-left)
        // 5: +X, -Y, +Z (front-bottom-right)
        // 6: -X, +Y, +Z (front-top-left)
        // 7: +X, +Y, +Z (front-top-right)
        (z_bit << 2) | (y_bit << 1) | x_bit
    }
    
    /// Subdivide this node into 8 children
    fn subdivide(&mut self) {
        if self.children.is_some() {
            return; // Already subdivided
        }
        
        let center = self.bounds.center();
        let half_extents = self.bounds.extents();
        let quarter_extents = half_extents * 0.5;
        
        let mut children = Vec::with_capacity(8);
        
        // Create 8 child nodes
        for octant in 0..8 {
            let x_sign = if octant & 1 != 0 { 1.0 } else { -1.0 };
            let y_sign = if octant & 2 != 0 { 1.0 } else { -1.0 };
            let z_sign = if octant & 4 != 0 { 1.0 } else { -1.0 };
            
            let child_center = Vec3::new(
                center.x + quarter_extents.x * x_sign,
                center.y + quarter_extents.y * y_sign,
                center.z + quarter_extents.z * z_sign,
            );
            
            let child_bounds = AABB::from_center_extents(child_center, quarter_extents);
            children.push(OctreeNode::new(child_bounds, self.depth + 1));
        }
        
        self.children = Some(Box::new([
            children[0].clone(),
            children[1].clone(),
            children[2].clone(),
            children[3].clone(),
            children[4].clone(),
            children[5].clone(),
            children[6].clone(),
            children[7].clone(),
        ]));
        
        // Redistribute existing entities to children
        let entities_to_distribute = std::mem::take(&mut self.entities);
        let center = self.bounds.center();  // Get center again to avoid borrow issues
        
        if let Some(ref mut children) = self.children {
            for entity in entities_to_distribute {
                // Calculate octant inline to avoid borrowing self
                let x_bit = if entity.position.x >= center.x { 1 } else { 0 };
                let y_bit = if entity.position.y >= center.y { 1 } else { 0 };
                let z_bit = if entity.position.z >= center.z { 1 } else { 0 };
                let octant = (z_bit << 2) | (y_bit << 1) | x_bit;
                
                children[octant].entities.push(entity);
            }
        }
    }
    
    /// Insert an entity into this node
    pub fn insert(&mut self, entity: OctreeEntity, config: &OctreeConfig) -> bool {
        // Check if entity is within bounds
        if !self.bounds.contains_point(entity.position) {
            return false;
        }
        
        // If this is a leaf node
        if self.is_leaf() {
            // Check if we should subdivide
            let should_subdivide = self.entities.len() >= config.max_entities_per_node
                && self.depth < config.max_depth
                && self.bounds.extents().x > config.min_node_size;
            
            if should_subdivide {
                // Subdivide and insert into appropriate child
                self.subdivide();
                
                // Calculate octant inline to avoid borrow issues
                let center = self.bounds.center();
                let x_bit = if entity.position.x >= center.x { 1 } else { 0 };
                let y_bit = if entity.position.y >= center.y { 1 } else { 0 };
                let z_bit = if entity.position.z >= center.z { 1 } else { 0 };
                let octant = (z_bit << 2) | (y_bit << 1) | x_bit;
                
                if let Some(ref mut children) = self.children {
                    return children[octant].insert(entity, config);
                }
            } else {
                // Just add to this node
                self.entities.push(entity);
                return true;
            }
        }
        
        // This is a branch node, insert into appropriate child
        let center = self.bounds.center();
        let x_bit = if entity.position.x >= center.x { 1 } else { 0 };
        let y_bit = if entity.position.y >= center.y { 1 } else { 0 };
        let z_bit = if entity.position.z >= center.z { 1 } else { 0 };
        let octant = (z_bit << 2) | (y_bit << 1) | x_bit;
        
        if let Some(ref mut children) = self.children {
            return children[octant].insert(entity, config);
        }
        
        false
    }
    
    /// Remove an entity from this node
    pub fn remove(&mut self, entity_id: Entity) -> bool {
        // Check if entity is in this node
        if let Some(index) = self.entities.iter().position(|e| e.id == entity_id) {
            self.entities.swap_remove(index);
            return true;
        }
        
        // Check children
        if let Some(ref mut children) = self.children {
            for child in children.iter_mut() {
                if child.remove(entity_id) {
                    return true;
                }
            }
        }
        
        false
    }
    
    /// Query all entities within a radius of a point
    pub fn query_radius(&self, center: Vec3, radius: f32, results: &mut Vec<OctreeEntity>) {
        // Quick AABB check - if sphere doesn't intersect node bounds, skip
        let closest_point = Vec3::new(
            center.x.clamp(self.bounds.min.x, self.bounds.max.x),
            center.y.clamp(self.bounds.min.y, self.bounds.max.y),
            center.z.clamp(self.bounds.min.z, self.bounds.max.z),
        );
        
        let distance_sq = (closest_point - center).magnitude_squared();
        if distance_sq > radius * radius {
            return; // Sphere doesn't intersect this node
        }
        
        // Check entities in this node
        for entity in &self.entities {
            let entity_distance_sq = (entity.position - center).magnitude_squared();
            let combined_radius = radius + entity.radius;
            
            if entity_distance_sq <= combined_radius * combined_radius {
                results.push(*entity);
            }
        }
        
        // Recursively check children
        if let Some(ref children) = self.children {
            for child in children.iter() {
                child.query_radius(center, radius, results);
            }
        }
    }
    
    /// Query all entities that intersect a ray, traversing the octree efficiently
    /// Returns entities in nodes the ray passes through, accounting for entity radius
    /// max_entity_radius: The maximum entity radius in the entire octree to properly expand bounds
    pub fn query_ray(&self, ray_origin: Vec3, ray_dir: Vec3, max_entity_radius: f32, results: &mut Vec<OctreeEntity>) {
        // Expand AABB by max entity radius to account for entities extending beyond their storage node
        // This ensures we find entities stored in neighboring nodes that extend into this node's space
        let expanded_bounds = if max_entity_radius > 0.0 {
            use crate::scene::AABB;
            let expansion = Vec3::new(max_entity_radius, max_entity_radius, max_entity_radius);
            AABB::new(self.bounds.min - expansion, self.bounds.max + expansion)
        } else {
            self.bounds
        };
        
        // Quick AABB check - if ray doesn't intersect expanded node bounds, skip entire subtree
        if expanded_bounds.intersect_ray(ray_origin, ray_dir).is_none() {
            return;
        }
        
        // Add all entities in this node
        results.extend_from_slice(&self.entities);
        
        // Recursively check children
        if let Some(ref children) = self.children {
            for child in children.iter() {
                child.query_ray(ray_origin, ray_dir, max_entity_radius, results);
            }
        }
    }
    
    /// Find an entity in this node or its children
    pub fn find_entity(&self, entity_id: Entity) -> Option<OctreeEntity> {
        // Check entities in this node
        for entity in &self.entities {
            if entity.id == entity_id {
                return Some(*entity);
            }
        }
        
        // Recursively check children
        if let Some(ref children) = self.children {
            for child in children.iter() {
                if let Some(entity) = child.find_entity(entity_id) {
                    return Some(entity);
                }
            }
        }
        
        None
    }
    
    /// Get all leaf nodes (for visualization)
    pub fn get_all_leaves<'a>(&'a self, leaves: &mut Vec<&'a OctreeNode>) {
        if self.is_leaf() {
            leaves.push(self);
        } else if let Some(ref children) = self.children {
            for child in children.iter() {
                child.get_all_leaves(leaves);
            }
        }
    }
    
    /// Get all nodes at a specific depth (for visualization)
    pub fn get_nodes_at_depth<'a>(&'a self, target_depth: u32, nodes: &mut Vec<&'a OctreeNode>) {
        if self.depth == target_depth {
            nodes.push(self);
        } else if let Some(ref children) = self.children {
            for child in children.iter() {
                child.get_nodes_at_depth(target_depth, nodes);
            }
        }
    }
    
    /// Count total entities in this node and all children
    pub fn count_entities(&self) -> usize {
        let mut count = self.entities.len();
        
        if let Some(ref children) = self.children {
            for child in children.iter() {
                count += child.count_entities();
            }
        }
        
        count
    }
}

/// Octree spatial partitioning structure
#[derive(Debug, Clone)]
pub struct Octree {
    /// Root node containing the entire world space
    pub root: OctreeNode,
    
    /// Configuration
    config: OctreeConfig,
    
    /// Cached maximum entity radius in the tree (updated on insert/remove)
    max_entity_radius: f32,
}

impl Octree {
    /// Create a new octree with given world bounds
    pub fn new(world_bounds: AABB, config: OctreeConfig) -> Self {
        Self {
            root: OctreeNode::new(world_bounds, 0),
            config,
            max_entity_radius: 0.0,
        }
    }
    
    /// Insert an entity into the octree
    pub fn insert(&mut self, entity_id: Entity, position: Vec3, radius: f32) -> bool {
        let entity = OctreeEntity {
            id: entity_id,
            position,
            radius,
        };
        
        // Update cached max radius if this entity has a larger radius
        if radius > self.max_entity_radius {
            self.max_entity_radius = radius;
        }
        
        self.root.insert(entity, &self.config)
    }
    
    /// Remove an entity from the octree
    pub fn remove(&mut self, entity_id: Entity) -> bool {
        self.root.remove(entity_id)
    }
    
    /// Query all entities within a radius of a point
    pub fn query_radius(&self, center: Vec3, radius: f32) -> Vec<OctreeEntity> {
        let mut results = Vec::new();
        self.root.query_radius(center, radius, &mut results);
        results
    }
    
    /// Query all entities that potentially intersect a ray
    /// Returns entities that are in octree nodes the ray passes through
    /// For actual intersection testing, you still need to test each entity individually
    pub fn query_ray(&self, ray_origin: Vec3, ray_dir: Vec3) -> Vec<OctreeEntity> {
        // Use cached max entity radius (O(1) instead of O(n))
        let mut results = Vec::new();
        self.root.query_ray(ray_origin, ray_dir, self.max_entity_radius, &mut results);
        results
    }
    
    /// Query all entities that are in the same or nearby octree nodes as the given entity
    /// This is useful for collision detection - returns potential collision candidates
    pub fn query_nearby(&self, entity_id: Entity) -> Vec<Entity> {
        // First, find the entity in the octree
        if let Some(entity_data) = self.find_entity(entity_id) {
            // Query using the entity's position and radius, plus some buffer for safety
            let query_radius = entity_data.radius * 2.0; // Query double the radius
            let results = self.query_radius(entity_data.position, query_radius);
            
            // Convert to just Entity IDs
            results.into_iter().map(|e| e.id).collect()
        } else {
            Vec::new()
        }
    }
    
    /// Find an entity in the octree and return its data
    pub fn find_entity(&self, entity_id: Entity) -> Option<OctreeEntity> {
        self.root.find_entity(entity_id)
    }
    
    /// Get all leaf nodes (for visualization)
    pub fn get_all_leaves(&self) -> Vec<&OctreeNode> {
        let mut leaves = Vec::new();
        self.root.get_all_leaves(&mut leaves);
        leaves
    }
    
    /// Get all nodes at a specific depth (for visualization)
    pub fn get_nodes_at_depth(&self, depth: u32) -> Vec<&OctreeNode> {
        let mut nodes = Vec::new();
        self.root.get_nodes_at_depth(depth, &mut nodes);
        nodes
    }
    
    /// Get total entity count
    pub fn entity_count(&self) -> usize {
        self.root.count_entities()
    }
    
    /// Clear the octree
    pub fn clear(&mut self) {
        self.root = OctreeNode::new(self.root.bounds, 0);
        self.max_entity_radius = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;
    
    #[test]
    fn test_octree_basic_insertion() {
        let bounds = AABB::new(Vec3::new(-100.0, -100.0, -100.0), Vec3::new(100.0, 100.0, 100.0));
        let mut octree = Octree::new(bounds, OctreeConfig::default());
        let mut world = World::new();
        
        let entity = world.create_entity();
        assert!(octree.insert(entity, Vec3::new(0.0, 0.0, 0.0), 1.0));
        assert_eq!(octree.entity_count(), 1);
    }
    
    #[test]
    fn test_octree_subdivision() {
        let bounds = AABB::new(Vec3::new(-100.0, -100.0, -100.0), Vec3::new(100.0, 100.0, 100.0));
        let config = OctreeConfig {
            max_entities_per_node: 4,
            max_depth: 3,
            min_node_size: 1.0,
        };
        let mut octree = Octree::new(bounds, config);
        let mut world = World::new();
        
        // Insert entities at same position to force subdivision
        for _ in 0..10 {
            let entity = world.create_entity();
            octree.insert(entity, Vec3::new(0.0, 0.0, 0.0), 1.0);
        }
        
        assert_eq!(octree.entity_count(), 10);
        assert!(octree.root.children.is_some()); // Should have subdivided
    }
    
    #[test]
    fn test_octree_radius_query() {
        let bounds = AABB::new(Vec3::new(-100.0, -100.0, -100.0), Vec3::new(100.0, 100.0, 100.0));
        let mut octree = Octree::new(bounds, OctreeConfig::default());
        let mut world = World::new();
        
        let entity1 = world.create_entity();
        let entity2 = world.create_entity();
        let entity3 = world.create_entity();
        
        octree.insert(entity1, Vec3::new(0.0, 0.0, 0.0), 1.0);
        octree.insert(entity2, Vec3::new(5.0, 0.0, 0.0), 1.0);
        octree.insert(entity3, Vec3::new(50.0, 0.0, 0.0), 1.0);
        
        let results = octree.query_radius(Vec3::new(0.0, 0.0, 0.0), 10.0);
        assert_eq!(results.len(), 2); // Should find entity1 and entity2
    }
}
