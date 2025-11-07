//! Material parameter types for different material workflows

use crate::foundation::math::Vec3;

/// Standard material parameters for PBR rendering
#[derive(Debug, Clone)]
pub struct StandardMaterialParams {
    /// Base color (albedo) - RGB values
    pub base_color: Vec3,
    /// Alpha transparency value
    pub alpha: f32,
    /// Metallic factor (0.0 = dielectric, 1.0 = metallic)
    pub metallic: f32,
    /// Roughness factor (0.0 = mirror, 1.0 = completely rough)
    pub roughness: f32,
    /// Ambient occlusion factor
    pub ambient_occlusion: f32,
    /// Emission color for self-illuminated materials
    pub emission: Vec3,
    /// Emission strength
    pub emission_strength: f32,
    /// Normal map scale factor
    pub normal_scale: f32,
    /// Enable base color texture sampling
    pub base_color_texture_enabled: bool,
    /// Enable normal map texture sampling
    pub normal_texture_enabled: bool,
}

impl Default for StandardMaterialParams {
    fn default() -> Self {
        Self {
            base_color: Vec3::new(0.8, 0.8, 0.8),
            alpha: 1.0,
            metallic: 0.0,
            roughness: 0.5,
            ambient_occlusion: 1.0,
            emission: Vec3::new(0.0, 0.0, 0.0),
            emission_strength: 0.0,
            normal_scale: 1.0,
            base_color_texture_enabled: false,
            normal_texture_enabled: false,
        }
    }
}

/// Unlit material parameters for simple shading
#[derive(Debug, Clone)]
pub struct UnlitMaterialParams {
    /// Material color
    pub color: Vec3,
    /// Alpha transparency
    pub alpha: f32,
}

impl Default for UnlitMaterialParams {
    fn default() -> Self {
        Self {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        }
    }
}
