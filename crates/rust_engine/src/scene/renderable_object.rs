//! Renderable object representation for the scene
//!
//! This is the cached rendering data extracted from ECS components.
//! Separates gameplay (ECS) from rendering concerns.

use crate::ecs::Entity;
use crate::foundation::math::Mat4;
use crate::render::resources::materials::MaterialId;

/// Cached rendering data for an entity
///
/// This struct represents the rendering state of an ECS entity.
/// The Scene Manager extracts this data from ECS components and
/// maintains it in a cache for efficient rendering.
#[derive(Debug, Clone)]
pub struct RenderableObject {
    /// The ECS entity this renderable represents
    pub entity: Entity,
    
    /// Material to use for rendering
    pub material_id: MaterialId,
    
    /// World-space transform matrix (computed from Transform component)
    pub transform: Mat4,
    
    /// Whether this object is visible
    pub visible: bool,
    
    /// Whether this material is transparent (affects render order)
    pub is_transparent: bool,
    
    /// Rendering layer for sorting (higher values render later)
    pub render_layer: u8,
    
    /// Dirty flag - true if ECS components changed since last sync
    pub dirty: bool,
}

impl RenderableObject {
    /// Create a new renderable object
    pub fn new(
        entity: Entity,
        material_id: MaterialId,
        transform: Mat4,
        visible: bool,
        is_transparent: bool,
        render_layer: u8,
    ) -> Self {
        Self {
            entity,
            material_id,
            transform,
            visible,
            is_transparent,
            render_layer,
            dirty: true, // Start dirty to force initial upload
        }
    }
    
    /// Mark this object as dirty (needs GPU update)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Clear the dirty flag (after GPU update)
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }
    
    /// Check if this object should be rendered
    pub fn should_render(&self) -> bool {
        self.visible
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_renderable_object_creation() {
        use crate::ecs::World;
        
        let mut world = World::new();
        let entity = world.create_entity();
        let material_id = MaterialId(0);
        let transform = Mat4::identity();
        
        let obj = RenderableObject::new(
            entity,
            material_id,
            transform,
            true,
            false,
            0,
        );
        
        assert!(obj.should_render());
        assert!(obj.dirty);
    }
    
    #[test]
    fn test_dirty_flag() {
        use crate::ecs::World;
        
        let mut world = World::new();
        let entity = world.create_entity();
        let mut obj = RenderableObject::new(
            entity,
            MaterialId(0),
            Mat4::identity(),
            true,
            false,
            0,
        );
        
        obj.clear_dirty();
        assert!(!obj.dirty);
        
        obj.mark_dirty();
        assert!(obj.dirty);
    }
}
