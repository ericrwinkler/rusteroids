//! Material system for rendering

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
