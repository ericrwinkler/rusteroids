//! Mesh representation for 3D models
//! 
//! This module provides 3D geometry data structures and management for rendering.
//! Contains vertex definitions and mesh primitives for use with the rendering system.
//! 
//! # Architecture Issues and Design Notes
//! 
//! ## Current State: MIXED ABSTRACTION LEVELS
//! This module mixes high-level mesh concepts with low-level Vulkan implementation details.
//! While the core `Mesh` and vertex data structures are backend-agnostic, the `Vertex` 
//! implementation contains Vulkan-specific methods that violate library design principles.
//! 
//! ## Library-Agnostic Design Violations:
//! 
//! ### CRITICAL ISSUE: Vulkan Coupling in Vertex Structure
//! The `Vertex` struct contains methods like `get_binding_description()` and 
//! `get_attribute_descriptions()` that return Vulkan-specific types. This creates
//! a direct dependency on the Vulkan backend at the high-level API layer.
//! 
//! **FIXME: Move Vulkan-specific vertex layout information to backend module**
//! - `Vertex` should be a pure data structure with no backend dependencies
//! - Vulkan vertex input state should be handled in `vulkan/` module
//! - Consider trait-based approach: `impl VulkanVertex for Vertex` in backend
//! 
//! ### Asset Loading Integration
//! The `Mesh` struct implements the `Asset` trait but with incomplete functionality.
//! This suggests an intention for proper asset pipeline integration that needs completion.
//! 
//! **FIXME: Complete asset loading implementation**
//! - Implement OBJ/glTF file format support
//! - Add proper error handling for malformed mesh data
//! - Consider streaming/LOD support for large meshes
//! 
//! ## Performance Considerations:
//! 
//! ### Memory Layout
//! The current vertex structure uses separate arrays for positions, normals, and
//! texture coordinates. While this is standard, it may not be optimal for all
//! rendering backends or usage patterns.
//! 
//! ### Mesh Storage
//! Meshes currently store vertex/index data directly in vectors. For performance-
//! critical applications, consider:
//! - GPU-resident mesh storage with handles
//! - Compressed vertex formats where appropriate
//! - Mesh instancing support for repeated geometry
//! 
//! ## Recommended Refactoring:
//! 
//! 1. **Separate Backend Concerns**: Move all Vulkan-specific code to backend module
//! 2. **Complete Asset Integration**: Implement proper mesh loading from files  
//! 3. **Add Validation**: Ensure mesh data integrity (valid indices, normalized normals)
//! 4. **Performance Optimization**: Consider GPU-resident storage and instancing
//! 
//! ## Design Goals for Library Version:
//! - Pure data structures with no backend dependencies
//! - Complete asset loading pipeline integration
//! - Flexible vertex attribute system supporting various backends
//! - Efficient memory usage and GPU resource management

use crate::assets::{Asset, AssetError};

/// 3D vertex data structure for rendering
/// 
/// Represents a single vertex with position, normal, and texture coordinate data.
/// This is a standard vertex layout suitable for most 3D rendering applications.
/// 
/// # Memory Layout
/// The `#[repr(C)]` attribute ensures consistent memory layout across platforms,
/// which is essential for GPU buffer uploads and cross-language compatibility.
/// 
/// # Backend Integration Issues
/// **ARCHITECTURE VIOLATION**: This struct contains Vulkan-specific methods that
/// break library-agnostic design principles. The vertex data should be pure,
/// with backend-specific layout information handled in the appropriate backend modules.
/// 
/// Vertex data structure for 3D rendering with position, normal, and texture coordinates.
/// 
/// This struct implements Hash and Eq to enable efficient vertex deduplication during
/// model loading, which can reduce memory usage by 5-6x for typical meshes.
/// 
/// **FIXME**: Remove Vulkan-specific methods and move to `vulkan/vertex_layout.rs`
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Position in 3D space
    pub position: [f32; 3],
    
    /// Normal vector
    pub normal: [f32; 3],
    
    /// Texture coordinates
    pub tex_coord: [f32; 2],
    
    /// Tangent vector for normal mapping
    pub tangent: [f32; 3],
    
    /// Padding for alignment (brings total to 44 bytes, or 48 with padding)
    pub _padding: f32,
}

// Safe to implement Pod and Zeroable for Vertex since it only contains f32 arrays
unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}

// Manual implementation of Eq and Hash for f32 arrays to enable vertex deduplication
// This is safe for our use case as we don't expect NaN values in mesh vertices
// Using bit representation ensures consistent hashing across identical float values
impl Eq for Vertex {}

impl std::hash::Hash for Vertex {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // Hash the bit representation of floats for consistent hashing across identical values
        self.position[0].to_bits().hash(state);
        self.position[1].to_bits().hash(state);
        self.position[2].to_bits().hash(state);
        self.normal[0].to_bits().hash(state);
        self.normal[1].to_bits().hash(state);
        self.normal[2].to_bits().hash(state);
        self.tex_coord[0].to_bits().hash(state);
        self.tex_coord[1].to_bits().hash(state);
        self.tangent[0].to_bits().hash(state);
        self.tangent[1].to_bits().hash(state);
        self.tangent[2].to_bits().hash(state);
    }
}

impl Vertex {
    /// Create a new vertex
    pub fn new(position: [f32; 3], normal: [f32; 3], tex_coord: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            tex_coord,
            tangent: [0.0, 0.0, 0.0], // Will be calculated later
            _padding: 0.0,
        }
    }
    
    /// Create a new vertex with tangent
    pub fn new_with_tangent(position: [f32; 3], normal: [f32; 3], tex_coord: [f32; 2], tangent: [f32; 3]) -> Self {
        Self {
            position,
            normal,
            tex_coord,
            tangent,
            _padding: 0.0,
        }
    }
}

/// 3D mesh containing vertices and indices for rendering
/// 
/// Represents a complete 3D model with vertex data and triangle indices. This is the
/// primary geometry container used by the rendering system for drawing 3D objects.
/// 
/// # Design Philosophy
/// The `Mesh` struct follows good library design principles by being backend-agnostic
/// and focusing purely on geometry data storage. Unlike the `Vertex` struct, this
/// container doesn't expose any rendering backend implementation details.
/// 
/// # Memory Management
/// Currently uses `Vec<Vertex>` and `Vec<u32>` for dynamic storage. While flexible,
/// this approach may not be optimal for performance-critical applications where
/// GPU-resident storage and mesh handles might be more appropriate.
/// 
/// # Asset Integration
/// Implements the `Asset` trait for integration with the asset loading system,
/// though the implementation is currently incomplete (returns NotFound error).
/// 
/// **FIXME**: Complete asset loading implementation for OBJ, glTF, and other 3D formats
/// 
/// # Future Enhancements
/// - GPU-resident mesh storage with handle-based access
/// - Level-of-detail (LOD) support for distance-based mesh switching  
/// - Mesh instancing data for rendering multiple copies efficiently
/// - Bounding box/sphere calculation for frustum culling
/// - Mesh compression for reduced memory usage
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Vertex data
    pub vertices: Vec<Vertex>,
    
    /// Index data for triangles
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Create a new mesh
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
    
    /// Create a test cube mesh with proper normals and texture coordinates
    /// 
    /// Generates a unit cube centered at the origin with vertices at ±1.0 on each axis.
    /// Each face has proper outward-facing normals and texture coordinates from 0.0 to 1.0.
    /// 
    /// # Coordinate System
    /// Uses right-handed coordinate system with Y-up orientation:
    /// - X+ = Right, X- = Left
    /// - Y+ = Up, Y- = Down  
    /// - Z+ = Forward (towards viewer), Z- = Back
    /// 
    /// # Vertex Layout
    /// Each face consists of 4 vertices arranged as 2 triangles (6 indices per face).
    /// Total: 8 vertices, 36 indices (12 triangles × 3 vertices each).
    /// 
    /// # Usage
    /// Primarily intended for testing and debugging. For production applications,
    /// consider loading meshes from asset files or using a more sophisticated
    /// primitive generation system.
    /// 
    /// **FIXME**: Consider moving primitive generation to a separate `primitives` module
    /// as the mesh system grows to include spheres, cylinders, and other basic shapes.
    pub fn cube() -> Self {
        let vertices = vec![
            // Front face
            Vertex::new([-1.0, -1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            Vertex::new([1.0, -1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            Vertex::new([1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            Vertex::new([-1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // Back face
            Vertex::new([-1.0, -1.0, -1.0], [0.0, 0.0, -1.0], [1.0, 0.0]),
            Vertex::new([-1.0, 1.0, -1.0], [0.0, 0.0, -1.0], [1.0, 1.0]),
            Vertex::new([1.0, 1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 1.0]),
            Vertex::new([1.0, -1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0]),
        ];

        let indices = vec![
            // Front
            0, 1, 2, 2, 3, 0,
            // Back
            4, 5, 6, 6, 7, 4,
            // Left
            4, 0, 3, 3, 5, 4,
            // Right
            1, 7, 6, 6, 2, 1,
            // Top
            3, 2, 6, 6, 5, 3,
            // Bottom
            4, 7, 1, 1, 0, 4,
        ];

        Self::new(vertices, indices)
    }
    
    /// Create a skybox cube mesh with proper UV mapping for cubemap textures
    /// 
    /// Generates a large cube (size 500.0) centered at the origin designed to be rendered
    /// as a skybox. The cube should be rendered LAST (use render_layer = 255) with depth
    /// testing configured appropriately for skybox rendering.
    /// 
    /// # UV Mapping Layout
    /// 
    /// The UV coordinates are mapped to a cross-layout skybox texture:
    /// ```text
    ///     [TOP]
    /// [L] [F] [R] [BK]
    ///     [BOT]
    /// ```
    /// 
    /// Layout breakdown:
    /// - **Top face**: Uses top 1/3 height, 1/4 width (starting at x=0.25)
    /// - **Middle faces** (Front, Right, Back, Left): Use middle 1/3 height
    ///   - Front: x=0.25-0.50
    ///   - Right: x=0.50-0.75  
    ///   - Back: x=0.75-1.00
    ///   - Left: x=0.00-0.25
    /// - **Bottom face**: Uses bottom 1/3 height, 1/4 width (starting at x=0.25)
    /// 
    /// # Coordinate System
    /// - Interior faces visible (normals point inward)
    /// - Y-up orientation
    /// - Size: 500.0 units (large enough to encompass typical scenes)
    /// 
    /// # Usage
    /// ```rust
    /// use rust_engine::render::primitives::Mesh;
    /// use rust_engine::render::resources::materials::{Material, UnlitMaterialParams};
    /// use rust_engine::foundation::math::Vec3;
    /// use rust_engine::render::systems::dynamic::MeshType;
    /// 
    /// // Create the skybox mesh
    /// let skybox_mesh = Mesh::skybox();
    /// 
    /// // Load your skybox texture (should be a cross-layout cubemap)
    /// // You can use resources/textures/skybox.png as an example
    /// let skybox_texture = texture_manager.load("resources/textures/skybox.png")?;
    /// 
    /// // Create an unlit material for the skybox
    /// // Skyboxes should be unlit since they represent distant environment lighting
    /// let skybox_material = Material::unlit(UnlitMaterialParams {
    ///     color: Vec3::new(1.0, 1.0, 1.0), // White to show texture colors
    ///     alpha: 1.0,
    /// })
    /// .with_albedo_texture(skybox_texture)
    /// .with_name("Skybox");
    /// 
    /// // Optional: Add emission texture if your skybox has glowing elements
    /// // let skybox_material = skybox_material.with_emission_texture(emission_texture);
    /// 
    /// // Create a renderable component with render_layer = 255 to render LAST
    /// // This ensures the skybox is drawn after all other objects
    /// let skybox_renderable = RenderableComponent::new(
    ///     skybox_material,
    ///     skybox_mesh,
    ///     MeshType::Cube, // Or create a MeshType::Skybox if needed
    /// );
    /// skybox_renderable.set_render_layer(255); // Maximum value = renders last
    /// 
    /// // Add to your ECS world
    /// let skybox_entity = world.create_entity();
    /// world.add_component(skybox_entity, skybox_renderable);
    /// world.add_component(skybox_entity, TransformComponent::identity());
    /// 
    /// // The skybox will now render as the background of your scene
    /// ```
    /// 
    /// # Important Rendering Notes
    /// 
    /// For proper skybox rendering, you may need to configure your renderer:
    /// 
    /// 1. **Depth Testing**: Skyboxes should use `LESS_OR_EQUAL` depth test to ensure
    ///    they render behind all geometry but don't Z-fight with the far plane.
    /// 
    /// 2. **Camera Transform**: When rendering the skybox, remove camera translation
    ///    (only rotation should apply). This keeps the skybox infinitely distant.
    ///    ```rust
    ///    // Pseudo-code for skybox rendering:
    ///    let view_no_translation = view_matrix;
    ///    view_no_translation.set_translation(Vec3::zero());
    ///    ```
    /// 
    /// 3. **Render Order**: Using `render_layer = 255` ensures the skybox renders last,
    ///    taking advantage of early-Z rejection for optimal performance.
    /// 
    /// 4. **Culling**: Disable backface culling or render interior faces since the camera
    ///    is inside the skybox cube.
    /// 
    /// # Rendering Notes
    /// - Should be rendered with depth test set to LESS_OR_EQUAL
    /// - Camera translation should be removed (only rotation applies)
    /// - Typically uses unlit material with skybox texture
    pub fn skybox() -> Self {
        let size = 500.0;
        
        // UV mapping for cross-layout skybox (4x3 grid)
        // Horizontal sections: Left(0.00-0.25), Front(0.25-0.50), Right(0.50-0.75), Back(0.75-1.00)
        // Vertical sections: Top(2/3-1.00), Middle(1/3-2/3), Bottom(0.00-1/3)
        // Using precise fractions: 1/3 = 0.333333..., 2/3 = 0.666666...
        // UV inset to prevent texture filtering from sampling adjacent faces
        
        const ONE_THIRD: f32 = 1.0 / 3.0;
        const TWO_THIRDS: f32 = 2.0 / 3.0;
        const UV_INSET: f32 = 0.002; // Small inset to avoid edge bleeding
        
        let vertices = vec![
            // Front face (+Z) - Middle section, x=0.25-0.50, y=1/3-2/3
            Vertex::new([-size, -size, size], [0.0, 0.0, -1.0], [0.25 + UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([size, -size, size], [0.0, 0.0, -1.0], [0.50 - UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([size, size, size], [0.0, 0.0, -1.0], [0.50 - UV_INSET, TWO_THIRDS - UV_INSET]),
            Vertex::new([-size, size, size], [0.0, 0.0, -1.0], [0.25 + UV_INSET, TWO_THIRDS - UV_INSET]),
            
            // Back face (-Z) - Middle section, x=0.75-1.00, y=1/3-2/3
            Vertex::new([size, -size, -size], [0.0, 0.0, 1.0], [0.75 + UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([-size, -size, -size], [0.0, 0.0, 1.0], [1.00 - UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([-size, size, -size], [0.0, 0.0, 1.0], [1.00 - UV_INSET, TWO_THIRDS - UV_INSET]),
            Vertex::new([size, size, -size], [0.0, 0.0, 1.0], [0.75 + UV_INSET, TWO_THIRDS - UV_INSET]),
            
            // Left face (-X) - Middle section, x=0.00-0.25, y=1/3-2/3
            Vertex::new([-size, -size, -size], [1.0, 0.0, 0.0], [0.00 + UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([-size, -size, size], [1.0, 0.0, 0.0], [0.25 - UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([-size, size, size], [1.0, 0.0, 0.0], [0.25 - UV_INSET, TWO_THIRDS - UV_INSET]),
            Vertex::new([-size, size, -size], [1.0, 0.0, 0.0], [0.00 + UV_INSET, TWO_THIRDS - UV_INSET]),
            
            // Right face (+X) - Middle section, x=0.50-0.75, y=1/3-2/3
            Vertex::new([size, -size, size], [-1.0, 0.0, 0.0], [0.50 + UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([size, -size, -size], [-1.0, 0.0, 0.0], [0.75 - UV_INSET, ONE_THIRD + UV_INSET]),
            Vertex::new([size, size, -size], [-1.0, 0.0, 0.0], [0.75 - UV_INSET, TWO_THIRDS - UV_INSET]),
            Vertex::new([size, size, size], [-1.0, 0.0, 0.0], [0.50 + UV_INSET, TWO_THIRDS - UV_INSET]),
            
            // Top face (+Y) - Top section, x=0.25-0.50, y=2/3-1.00
            Vertex::new([-size, size, size], [0.0, -1.0, 0.0], [0.25 + UV_INSET, TWO_THIRDS + UV_INSET]),
            Vertex::new([size, size, size], [0.0, -1.0, 0.0], [0.50 - UV_INSET, TWO_THIRDS + UV_INSET]),
            Vertex::new([size, size, -size], [0.0, -1.0, 0.0], [0.50 - UV_INSET, 1.00 - UV_INSET]),
            Vertex::new([-size, size, -size], [0.0, -1.0, 0.0], [0.25 + UV_INSET, 1.00 - UV_INSET]),
            
            // Bottom face (-Y) - Bottom section, x=0.25-0.50, y=0.00-1/3
            Vertex::new([-size, -size, -size], [0.0, 1.0, 0.0], [0.25 + UV_INSET, 0.00 + UV_INSET]),
            Vertex::new([size, -size, -size], [0.0, 1.0, 0.0], [0.50 - UV_INSET, 0.00 + UV_INSET]),
            Vertex::new([size, -size, size], [0.0, 1.0, 0.0], [0.50 - UV_INSET, ONE_THIRD - UV_INSET]),
            Vertex::new([-size, -size, size], [0.0, 1.0, 0.0], [0.25 + UV_INSET, ONE_THIRD - UV_INSET]),
        ];

        // REVERSED winding order for viewing from INSIDE the cube
        // Counter-clockwise when viewed from inside = clockwise from outside
        let indices = vec![
            // Front face - REVERSED
            0, 3, 2, 2, 1, 0,
            // Back face - REVERSED
            4, 7, 6, 6, 5, 4,
            // Left face - REVERSED
            8, 11, 10, 10, 9, 8,
            // Right face - REVERSED
            12, 15, 14, 14, 13, 12,
            // Top face - REVERSED
            16, 19, 18, 18, 17, 16,
            // Bottom face - REVERSED
            20, 23, 22, 22, 21, 20,
        ];

        Self::new(vertices, indices)
    }
}

impl Asset for Mesh {
    /// Load mesh from binary data (currently unimplemented)
    /// 
    /// **IMPLEMENTATION STATUS**: Placeholder returning NotFound error.
    /// 
    /// This method should parse common 3D file formats and convert them to internal
    /// mesh representation. The asset loading system expects this to handle binary
    /// data from various file formats.
    /// 
    /// # Planned Implementation
    /// Should support multiple 3D file formats:
    /// - **OBJ**: Simple text-based format, good for basic models
    /// - **glTF 2.0**: Industry standard for real-time rendering
    /// - **FBX**: Common format from 3D modeling tools (complex)
    /// - **Custom Binary**: Optimized format for runtime performance
    /// 
    /// # Error Handling
    /// Should provide detailed error information for malformed files:
    /// - Invalid vertex data (NaN positions, zero-length normals)
    /// - Missing required attributes (positions at minimum)
    /// - Unsupported format features
    /// - File corruption or truncation
    /// 
    /// # Performance Considerations
    /// - Large mesh files should support streaming/chunked loading
    /// - Consider vertex optimization (deduplication, cache-friendly ordering)
    /// - Validate and fix mesh data (ensure proper winding order, normalize normals)
    /// 
    /// **FIXME**: Implement actual mesh loading with proper format detection and parsing
    fn from_bytes(_bytes: &[u8]) -> Result<Self, AssetError> 
    where 
        Self: Sized 
    {
        // TODO: Implement OBJ loading from bytes
        Err(AssetError::NotFound("Mesh loading not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skybox_mesh_structure() {
        let skybox = Mesh::skybox();
        
        // Skybox should have 24 vertices (4 per face * 6 faces)
        assert_eq!(skybox.vertices.len(), 24, "Skybox should have 24 vertices");
        
        // Skybox should have 36 indices (2 triangles per face * 3 vertices per triangle * 6 faces)
        assert_eq!(skybox.indices.len(), 36, "Skybox should have 36 indices");
        
        // Verify all indices are valid (less than vertex count)
        for &idx in &skybox.indices {
            assert!(idx < skybox.vertices.len() as u32, "Index {} is out of bounds", idx);
        }
    }

    #[test]
    fn test_skybox_uv_mapping() {
        let skybox = Mesh::skybox();
        
        // Verify UV coordinates are within valid range [0.0, 1.0]
        for (i, vertex) in skybox.vertices.iter().enumerate() {
            assert!(vertex.tex_coord[0] >= 0.0 && vertex.tex_coord[0] <= 1.0,
                "Vertex {} U coordinate {} is out of range [0.0, 1.0]", i, vertex.tex_coord[0]);
            assert!(vertex.tex_coord[1] >= 0.0 && vertex.tex_coord[1] <= 1.0,
                "Vertex {} V coordinate {} is out of range [0.0, 1.0]", i, vertex.tex_coord[1]);
        }
        
        // Test specific UV mapping for front face (first 4 vertices)
        // Front face should use x=0.25-0.50, y=1/3-2/3
        let front_verts = &skybox.vertices[0..4];
        let one_third = 1.0 / 3.0;
        let two_thirds = 2.0 / 3.0;
        for vertex in front_verts {
            assert!(vertex.tex_coord[0] >= 0.25 && vertex.tex_coord[0] <= 0.50,
                "Front face U coordinate should be in range [0.25, 0.50]");
            assert!(vertex.tex_coord[1] >= one_third - 0.01 && vertex.tex_coord[1] <= two_thirds + 0.01,
                "Front face V coordinate should be in range [1/3, 2/3]");
        }
        
        // Test back face (vertices 4-7) should use x=0.75-1.00
        let back_verts = &skybox.vertices[4..8];
        for vertex in back_verts {
            assert!(vertex.tex_coord[0] >= 0.75 && vertex.tex_coord[0] <= 1.00,
                "Back face U coordinate should be in range [0.75, 1.00]");
        }
    }

    #[test]
    fn test_skybox_size() {
        let skybox = Mesh::skybox();
        let expected_size = 500.0;
        
        // Check that vertices are at the expected distance
        for vertex in &skybox.vertices {
            let pos = vertex.position;
            // Each component should be ±expected_size
            assert!(pos[0].abs() == expected_size, "X coordinate should be ±{}", expected_size);
            assert!(pos[1].abs() == expected_size, "Y coordinate should be ±{}", expected_size);
            assert!(pos[2].abs() == expected_size, "Z coordinate should be ±{}", expected_size);
        }
    }

    #[test]
    fn test_skybox_normals_point_inward() {
        let skybox = Mesh::skybox();
        
        // Skybox normals should point inward (opposite of standard cube)
        // Front face (+Z) should have normals pointing in -Z direction
        let front_normal = skybox.vertices[0].normal;
        assert!(front_normal[2] < 0.0, "Front face normal should point inward (-Z)");
        
        // Back face (-Z) should have normals pointing in +Z direction
        let back_normal = skybox.vertices[4].normal;
        assert!(back_normal[2] > 0.0, "Back face normal should point inward (+Z)");
        
        // Left face (-X) should have normals pointing in +X direction
        let left_normal = skybox.vertices[8].normal;
        assert!(left_normal[0] > 0.0, "Left face normal should point inward (+X)");
        
        // Right face (+X) should have normals pointing in -X direction
        let right_normal = skybox.vertices[12].normal;
        assert!(right_normal[0] < 0.0, "Right face normal should point inward (-X)");
    }

    #[test]
    fn test_cube_mesh_structure() {
        let cube = Mesh::cube();
        
        // Cube should have 8 vertices
        assert_eq!(cube.vertices.len(), 8, "Cube should have 8 vertices");
        
        // Cube should have 36 indices (6 faces * 2 triangles * 3 vertices)
        assert_eq!(cube.indices.len(), 36, "Cube should have 36 indices");
    }
}
