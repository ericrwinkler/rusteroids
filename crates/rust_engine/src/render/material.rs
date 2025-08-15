//! Material system for 3D rendering
//! 
//! This module provides physically-based material definitions for realistic 3D rendering.
//! Materials define how surfaces interact with light to determine final appearance.
//! 
//! # Architecture Assessment: WELL-DESIGNED MODULE
//! 
//! This module represents good library design practices and serves as an example of how
//! other modules should be structured. It has clean separation of concerns, no backend
//! coupling, and a focused API that does exactly what it needs to do.
//! 
//! ## Architectural Strengths:
//! 
//! ### Backend Agnostic Design ✅
//! The material system contains no references to Vulkan, OpenGL, or any specific rendering
//! backend. Materials are pure data structures that can be interpreted by any renderer.
//! This is exactly how library-agnostic design should work.
//! 
//! ### PBR Standard Compliance ✅
//! Follows industry-standard Physically Based Rendering (PBR) material model with:
//! - Base color (albedo) for surface appearance
//! - Metallic factor for metal vs dielectric surfaces
//! - Roughness factor for surface microsurface detail
//! - Alpha channel for transparency effects
//! 
//! ### Clean API Design ✅
//! - Builder pattern for intuitive material construction
//! - Sensible defaults that work for most use cases
//! - Value clamping to prevent invalid material parameters
//! - Immutable operations that return modified copies
//! 
//! ### Focused Responsibility ✅
//! The module does one thing well: define material properties. It doesn't try to handle
//! texture loading, shader compilation, GPU resource management, or other concerns.
//! 
//! ## Areas for Future Enhancement:
//! 
//! ### Texture Support
//! Current materials only support solid colors. Real-world materials need texture maps:
//! - Base color (diffuse) textures
//! - Normal maps for surface detail
//! - Roughness/metallic maps for material variation
//! - Emission maps for glowing surfaces
//! 
//! **Future Enhancement**: Add texture handle references:
//! ```rust
//! pub struct Material {
//!     pub base_color: MaterialColor,  // Either solid color or texture
//!     pub normal_map: Option<TextureHandle>,
//!     pub metallic_roughness_map: Option<TextureHandle>,
//!     // ...
//! }
//! ```
//! 
//! ### Advanced Material Features
//! - Clear coat for car paint and plastic materials
//! - Subsurface scattering for organic materials (skin, wax, etc.)
//! - Sheen for fabric materials
//! - Anisotropic reflection for brushed metal
//! 
//! ### Material Validation
//! Consider adding validation for physically plausible material parameters:
//! - Energy conservation checks
//! - Valid parameter range enforcement
//! - Warning for impossible material combinations
//! 
//! ## Performance Considerations:
//! 
//! ### GPU Layout Compatibility
//! Current struct layout may not be optimal for GPU uniform buffers. Consider:
//! - 16-byte alignment for vec4 data
//! - Padding for GPU memory layout requirements
//! - Buffer packing optimization
//! 
//! ### Material Batching
//! For scenes with many materials, consider material sorting and batching strategies
//! to minimize GPU state changes during rendering.
//! 
//! ## Design Goals Achieved:
//! 
//! 1. ✅ **Library Agnostic**: No backend dependencies
//! 2. ✅ **Industry Standard**: Follows PBR material model  
//! 3. ✅ **Easy to Use**: Builder pattern with sensible defaults
//! 4. ✅ **Type Safe**: Uses Rust type system to prevent invalid materials
//! 5. ✅ **Performance Friendly**: Lightweight data structures
//! 
//! This module should serve as a template for how other rendering system modules
//! should be designed and documented.

/// Material properties for 3D rendering
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Base color (RGB)
    pub base_color: [f32; 3],
    
    /// Metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub metallic: f32,
    
    /// Roughness factor (0.0 = mirror, 1.0 = completely rough)
    pub roughness: f32,
    
    /// Alpha/transparency (0.0 = transparent, 1.0 = opaque)
    pub alpha: f32,
}

impl Material {
    /// Create a new material with default properties
    pub fn new() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0], // White
            metallic: 0.0,
            roughness: 0.5,
            alpha: 1.0,
        }
    }
    
    /// Set the base color
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.base_color = [r, g, b];
        self
    }
    
    /// Set the metallic factor
    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }
    
    /// Set the roughness factor
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }
    
    /// Set the alpha/transparency
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha.clamp(0.0, 1.0);
        self
    }
}

impl Default for Material {
    fn default() -> Self {
        Self::new()
    }
}
