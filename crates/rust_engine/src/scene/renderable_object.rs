//! Renderable object representation for the scene
//!
//! This is the cached rendering data extracted from ECS components.
//! Separates gameplay (ECS) from rendering concerns.

use crate::ecs::Entity;
use crate::foundation::math::Mat4;
use crate::render::resources::materials::Material;
use crate::render::primitives::Mesh;
use crate::render::systems::dynamic::MeshType;

/// Cached rendering data for an entity
///
/// This struct represents the rendering state of an ECS entity.
/// The Scene Manager extracts this data from ECS components and
/// maintains it in a cache for efficient rendering.
#[derive(Debug, Clone)]
pub struct RenderableObject {
    /// The ECS entity this renderable represents
    pub entity: Entity,
    
    /// Mesh data for rendering
    pub mesh: Mesh,
    
    /// The mesh type (which pool this belongs to)
    pub mesh_type: MeshType,
    
    /// Material to use for rendering (full material data, not just ID)
    pub material: Material,
    
    /// World-space transform matrix (computed from Transform component)
    pub transform: Mat4,
    
    /// Whether this object is visible
    pub visible: bool,
    
    /// Whether this object is transparent (informational flag)
    /// 
    /// This flag is copied from RenderableComponent for ECS queries. The actual
    /// transparent rendering is determined by Material.required_pipeline(), not this flag.
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
        mesh: Mesh,
        mesh_type: MeshType,
        material: Material,
        transform: Mat4,
        visible: bool,
        is_transparent: bool,
        render_layer: u8,
    ) -> Self {
        Self {
            entity,
            mesh,
            mesh_type,
            material,
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
        use crate::render::resources::materials::Material;
        use crate::foundation::math::Vec3;
        
        let mut world = World::new();
        let entity = world.create_entity();
        let material = Material::unlit(crate::render::resources::materials::UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        });
        let transform = Mat4::identity();
        let mesh = Mesh { vertices: vec![], indices: vec![] };
        
        let obj = RenderableObject::new(
            entity,
            mesh,
            MeshType::Cube,
            material,
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
        use crate::render::resources::materials::Material;
        use crate::foundation::math::Vec3;
        
        let mut world = World::new();
        let entity = world.create_entity();
        let mesh = Mesh { vertices: vec![], indices: vec![] };
        let material = Material::unlit(crate::render::resources::materials::UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        });
        let mut obj = RenderableObject::new(
            entity,
            mesh,
            MeshType::Sphere,
            material,
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
