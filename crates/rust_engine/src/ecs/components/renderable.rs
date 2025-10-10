//! Renderable component for entities that can be rendered
//!
//! This component marks entities as renderable and contains all the data
//! needed to submit them to the rendering system via the batch renderer.

use crate::ecs::Component;
use crate::render::material::MaterialId;
use crate::render::mesh::Mesh;

/// Component for entities that can be rendered
#[derive(Debug, Clone)]
pub struct RenderableComponent {
    /// Material to use for rendering
    pub material_id: MaterialId,
    
    /// Mesh data for this renderable (for now we store the mesh directly)
    /// TODO: Replace with MeshHandle when we implement mesh management
    pub mesh: Mesh,
    
    /// Whether this object is visible
    pub visible: bool,
    
    /// Whether this material is transparent (affects render order)
    pub is_transparent: bool,
    
    /// Rendering layer for sorting (higher values render later)
    pub render_layer: u8,
}

impl RenderableComponent {
    /// Create a new renderable component
    pub fn new(material_id: MaterialId, mesh: Mesh) -> Self {
        Self {
            material_id,
            mesh,
            visible: true,
            is_transparent: false,
            render_layer: 0,
        }
    }
    
    /// Create a transparent renderable component
    pub fn new_transparent(material_id: MaterialId, mesh: Mesh, render_layer: u8) -> Self {
        Self {
            material_id,
            mesh,
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
    pub fn set_material(&mut self, material_id: MaterialId) {
        self.material_id = material_id;
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
    pub fn create_opaque(material_id: MaterialId, mesh: Mesh) -> RenderableComponent {
        RenderableComponent::new(material_id, mesh)
    }
    
    /// Create a transparent renderable with specific layer
    pub fn create_transparent(
        material_id: MaterialId, 
        mesh: Mesh, 
        render_layer: u8
    ) -> RenderableComponent {
        RenderableComponent::new_transparent(material_id, mesh, render_layer)
    }
    
    /// Create a hidden renderable (useful for pre-loading)
    pub fn create_hidden(material_id: MaterialId, mesh: Mesh) -> RenderableComponent {
        let mut renderable = RenderableComponent::new(material_id, mesh);
        renderable.set_visible(false);
        renderable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::mesh::Vertex;

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

    #[test]
    fn test_renderable_component_creation() {
        let material_id = MaterialId(1);
        let mesh = create_test_mesh();
        let renderable = RenderableComponent::new(material_id, mesh);
        
        assert_eq!(renderable.material_id, material_id);
        assert!(renderable.visible);
        assert!(!renderable.is_transparent);
        assert_eq!(renderable.render_layer, 0);
        assert!(renderable.should_render());
    }

    #[test]
    fn test_transparent_renderable() {
        let material_id = MaterialId(2);
        let mesh = create_test_mesh();
        let renderable = RenderableComponent::new_transparent(material_id, mesh, 5);
        
        assert_eq!(renderable.material_id, material_id);
        assert!(renderable.visible);
        assert!(renderable.is_transparent);
        assert_eq!(renderable.render_layer, 5);
    }

    #[test]
    fn test_visibility_toggle() {
        let material_id = MaterialId(3);
        let mesh = create_test_mesh();
        let mut renderable = RenderableComponent::new(material_id, mesh);
        
        assert!(renderable.should_render());
        
        renderable.set_visible(false);
        assert!(!renderable.should_render());
        
        renderable.set_visible(true);
        assert!(renderable.should_render());
    }

    #[test]
    fn test_factory_methods() {
        let material_id = MaterialId(4);
        let mesh = create_test_mesh();
        
        // Test opaque creation
        let opaque = RenderableFactory::create_opaque(material_id, mesh.clone());
        assert!(!opaque.is_transparent);
        assert!(opaque.visible);
        
        // Test transparent creation
        let transparent = RenderableFactory::create_transparent(material_id, mesh.clone(), 3);
        assert!(transparent.is_transparent);
        assert_eq!(transparent.render_layer, 3);
        
        // Test hidden creation
        let hidden = RenderableFactory::create_hidden(material_id, mesh);
        assert!(!hidden.visible);
    }
}
