//! Renderable component for entities that can be rendered
//!
//! This component marks entities as renderable and contains all the data
//! needed to submit them to the rendering system via the batch renderer.

use crate::ecs::Component;
use crate::render::resources::materials::Material;
use crate::render::primitives::Mesh;
use crate::render::systems::dynamic::MeshType;

/// Component for entities that can be rendered
#[derive(Debug, Clone)]
pub struct RenderableComponent {
    /// Material to use for rendering (full material data, not just ID)
    pub material: Material,
    
    /// Mesh data for this renderable (for now we store the mesh directly)
    /// TODO: Replace with MeshHandle when we implement mesh management
    pub mesh: Mesh,
    
    /// The mesh type (which pool this belongs to)
    pub mesh_type: MeshType,
    
    /// Whether this object is visible
    pub visible: bool,
    
    /// Whether this material is transparent (informational flag)
    /// 
    /// **NOTE**: This flag is set based on Material.required_pipeline() and is used
    /// for ECS queries and gameplay logic (e.g., "is this object transparent?").
    /// The actual transparent rendering is determined by the Material's pipeline type,
    /// not this flag. The rendering system in pool_manager.rs categorizes objects by
    /// Material.required_pipeline() to handle transparent vs opaque rendering.
    pub is_transparent: bool,
    
    /// Rendering layer for sorting (higher values render later)
    pub render_layer: u8,
}

impl RenderableComponent {
    /// Create a new renderable component (opaque)
    pub fn new(material: Material, mesh: Mesh, mesh_type: MeshType) -> Self {
        Self {
            material,
            mesh,
            mesh_type,
            visible: true,
            is_transparent: false,
            render_layer: 0,
        }
    }
    
    /// Create a transparent renderable component
    /// 
    /// Sets is_transparent=true for transparent materials. The actual transparency
    /// rendering is handled by the Material's pipeline type (TransparentPBR/TransparentUnlit).
    pub fn new_transparent(material: Material, mesh: Mesh, mesh_type: MeshType, render_layer: u8) -> Self {
        Self {
            material,
            mesh,
            mesh_type,
            visible: true,
            is_transparent: true,
            render_layer,
        }
    }
    
    /// Set visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }
    
    /// Check if this component should be rendered
    pub fn should_render(&self) -> bool {
        self.visible
    }
    
    /// Set the material
    pub fn set_material(&mut self, material: Material) {
        self.material = material;
    }
    
    /// Set transparency
    pub fn set_transparent(&mut self, transparent: bool) {
        self.is_transparent = transparent;
    }
    
    /// Set render layer
    pub fn set_render_layer(&mut self, layer: u8) {
        self.render_layer = layer;
    }
}

impl Component for RenderableComponent {}

/// Factory for creating renderable components
pub struct RenderableFactory;

impl RenderableFactory {
    /// Create a simple opaque renderable
    pub fn create_opaque(material: Material, mesh: Mesh, mesh_type: MeshType) -> RenderableComponent {
        RenderableComponent::new(material, mesh, mesh_type)
    }
    
    /// Create a transparent renderable with specific layer
    pub fn create_transparent(
        material: Material, 
        mesh: Mesh,
        mesh_type: MeshType,
        render_layer: u8
    ) -> RenderableComponent {
        RenderableComponent::new_transparent(material, mesh, mesh_type, render_layer)
    }
    
    /// Create a hidden renderable (useful for pre-loading)
    pub fn create_hidden(material: Material, mesh: Mesh, mesh_type: MeshType) -> RenderableComponent {
        let mut renderable = RenderableComponent::new(material, mesh, mesh_type);
        renderable.set_visible(false);
        renderable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::primitives::Vertex;
    use crate::foundation::math::Vec3;

    fn create_test_mesh() -> Mesh {
        Mesh::new(
            vec![
                Vertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0]),
                Vertex::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0, 0.0]),
                Vertex::new([0.5, 1.0, 0.0], [0.0, 1.0, 0.0], [0.5, 1.0]),
            ],
            vec![0, 1, 2],
        )
    }
    
    fn create_test_material() -> Material {
        Material::unlit(crate::render::resources::materials::UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        })
    }

    #[test]
    fn test_renderable_component_creation() {
        let material = create_test_material();
        let mesh = create_test_mesh();
        let renderable = RenderableComponent::new(material, mesh);
        
        assert!(renderable.visible);
        assert!(!renderable.is_transparent);
        assert_eq!(renderable.render_layer, 0);
        assert!(renderable.should_render());
    }

    #[test]
    fn test_transparent_renderable() {
        let material = create_test_material();
        let mesh = create_test_mesh();
        let renderable = RenderableComponent::new_transparent(material, mesh, MeshType::Sphere, 5);
        
        assert!(renderable.visible);
        assert!(renderable.is_transparent);
        assert_eq!(renderable.render_layer, 5);
    }

    #[test]
    fn test_visibility_toggle() {
        let material = create_test_material();
        let mesh = create_test_mesh();
        let mut renderable = RenderableComponent::new(material, mesh, MeshType::Teapot);
        
        assert!(renderable.should_render());
        
        renderable.set_visible(false);
        assert!(!renderable.should_render());
        
        renderable.set_visible(true);
        assert!(renderable.should_render());
    }

    #[test]
    fn test_factory_methods() {
        let material = create_test_material();
        let mesh = create_test_mesh();
        
        // Test opaque creation
        let opaque = RenderableFactory::create_opaque(material.clone(), mesh.clone(), MeshType::Cube);
        assert!(!opaque.is_transparent);
        assert!(opaque.visible);
        
        // Test transparent creation
        let transparent = RenderableFactory::create_transparent(material.clone(), mesh.clone(), MeshType::Sphere, 3);
        assert!(transparent.is_transparent);
        assert_eq!(transparent.render_layer, 3);
        
        // Test hidden creation
        let hidden = RenderableFactory::create_hidden(material, mesh, MeshType::Teapot);
        assert!(!hidden.visible);
    }
}
