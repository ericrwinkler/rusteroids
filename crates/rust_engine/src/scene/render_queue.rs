//! Render queue for batched rendering
//!
//! Collects renderable objects and organizes them for efficient rendering.
//! Following Game Engine Architecture Chapter 11.3 - Render Queues.

use crate::scene::RenderableObject;
use crate::render::resources::materials::MaterialId;
use std::collections::HashMap;

/// A batch of objects sharing the same material
#[derive(Debug, Clone)]
pub struct RenderBatch {
    /// Material used by all objects in this batch
    pub material_id: MaterialId,
    
    /// Objects in this batch
    pub objects: Vec<RenderableObject>,
}

impl RenderBatch {
    /// Create a new empty batch for a material
    pub fn new(material_id: MaterialId) -> Self {
        Self {
            material_id,
            objects: Vec::new(),
        }
    }
    
    /// Add an object to this batch
    pub fn add_object(&mut self, object: RenderableObject) {
        self.objects.push(object);
    }
    
    /// Get the number of objects in this batch
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

/// Optimized render queue for a frame
///
/// Organizes renderable objects into batches by material to minimize
/// GPU state changes. Separates opaque and transparent objects.
#[derive(Debug)]
pub struct RenderQueue {
    /// Opaque object batches (rendered front-to-back for early-z)
    opaque_batches: Vec<RenderBatch>,
    
    /// Transparent object batches (rendered back-to-front for alpha blending)
    transparent_batches: Vec<RenderBatch>,
}

impl RenderQueue {
    /// Create an empty render queue
    pub fn new() -> Self {
        Self {
            opaque_batches: Vec::new(),
            transparent_batches: Vec::new(),
        }
    }
    
    /// Build a render queue from a list of renderable objects
    pub fn from_objects(objects: &[RenderableObject]) -> Self {
        let mut queue = Self::new();
        
        // Separate opaque and transparent objects
        let mut opaque_objects: Vec<&RenderableObject> = objects
            .iter()
            .filter(|obj| obj.should_render() && !obj.is_transparent)
            .collect();
        
        let mut transparent_objects: Vec<&RenderableObject> = objects
            .iter()
            .filter(|obj| obj.should_render() && obj.is_transparent)
            .collect();
        
        // Sort opaque objects front-to-back (for early-z optimization)
        // Note: This requires camera position, simplified for now
        opaque_objects.sort_by_key(|obj| obj.render_layer);
        
        // Sort transparent objects back-to-front (for correct alpha blending)
        transparent_objects.sort_by_key(|obj| std::cmp::Reverse(obj.render_layer));
        
        // Batch opaque objects by material
        queue.opaque_batches = Self::batch_by_material(&opaque_objects);
        
        // Batch transparent objects by material (maintaining back-to-front order)
        queue.transparent_batches = Self::batch_by_material(&transparent_objects);
        
        queue
    }
    
    /// Batch objects by material ID
    fn batch_by_material(objects: &[&RenderableObject]) -> Vec<RenderBatch> {
        let mut batches_map: HashMap<MaterialId, RenderBatch> = HashMap::new();
        
        for obj in objects {
            batches_map
                .entry(obj.material_id)
                .or_insert_with(|| RenderBatch::new(obj.material_id))
                .add_object((*obj).clone());
        }
        
        batches_map.into_values().collect()
    }
    
    /// Get opaque batches (front-to-back order)
    pub fn opaque_batches(&self) -> &[RenderBatch] {
        &self.opaque_batches
    }
    
    /// Get transparent batches (back-to-front order)
    pub fn transparent_batches(&self) -> &[RenderBatch] {
        &self.transparent_batches
    }
    
    /// Get total number of opaque objects
    pub fn opaque_object_count(&self) -> usize {
        self.opaque_batches.iter().map(|b| b.object_count()).sum()
    }
    
    /// Get total number of transparent objects
    pub fn transparent_object_count(&self) -> usize {
        self.transparent_batches.iter().map(|b| b.object_count()).sum()
    }
    
    /// Get total number of objects in the queue
    pub fn total_object_count(&self) -> usize {
        self.opaque_object_count() + self.transparent_object_count()
    }
    
    /// Get total number of batches
    pub fn batch_count(&self) -> usize {
        self.opaque_batches.len() + self.transparent_batches.len()
    }
}

impl Default for RenderQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::math::Mat4;
    
    #[test]
    fn test_render_batch() {
        use crate::ecs::World;
        
        let mut world = World::new();
        let mut batch = RenderBatch::new(MaterialId(1));
        assert_eq!(batch.object_count(), 0);
        
        let obj = RenderableObject::new(
            world.create_entity(),
            MaterialId(1),
            Mat4::identity(),
            true,
            false,
            0,
        );
        
        batch.add_object(obj);
        assert_eq!(batch.object_count(), 1);
    }
    
    #[test]
    fn test_render_queue_batching() {
        use crate::ecs::World;
        
        let mut world = World::new();
        let objects = vec![
            RenderableObject::new(
                world.create_entity(),
                MaterialId(0),
                Mat4::identity(),
                true,
                false,
                0,
            ),
            RenderableObject::new(
                world.create_entity(),
                MaterialId(0),
                Mat4::identity(),
                true,
                false,
                0,
            ),
            RenderableObject::new(
                world.create_entity(),
                MaterialId(1),
                Mat4::identity(),
                true,
                false,
                0,
            ),
        ];
        
        let queue = RenderQueue::from_objects(&objects);
        
        assert_eq!(queue.total_object_count(), 3);
        assert_eq!(queue.batch_count(), 2); // 2 unique materials
    }
    
    #[test]
    fn test_render_queue_transparency_separation() {
        use crate::ecs::World;
        
        let mut world = World::new();
        let objects = vec![
            RenderableObject::new(
                world.create_entity(),
                MaterialId(0),
                Mat4::identity(),
                true,
                false, // Opaque
                0,
            ),
            RenderableObject::new(
                world.create_entity(),
                MaterialId(1),
                Mat4::identity(),
                true,
                true, // Transparent
                0,
            ),
        ];
        
        let queue = RenderQueue::from_objects(&objects);
        
        assert_eq!(queue.opaque_object_count(), 1);
        assert_eq!(queue.transparent_object_count(), 1);
    }
}
