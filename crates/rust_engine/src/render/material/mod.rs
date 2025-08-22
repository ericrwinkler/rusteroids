//! Material system for the rendering engine
//!
//! This module provides a comprehensive material system supporting different
//! material types, PBR workflows, and efficient GPU data management.
//!
//! # Architecture
//!
//! The material system is built around descriptor sets and uniform buffers:
//! - Set 0: Per-frame data (camera, lighting)
//! - Set 1: Per-material data (material properties, textures)
//! - Set 2: Per-object data (reserved for future use)
//!
//! # Material Types
//!
//! - **Standard PBR**: Full physically-based rendering with metallic-roughness workflow
//! - **Unlit**: Simple unshaded materials for UI, effects, etc.
//! - **Transparent**: Materials with alpha blending support
//! - **Custom**: User-defined materials with custom shaders

pub mod material_type;
pub mod material_ubo;
pub mod material_manager;
pub mod texture_manager;

pub use material_type::*;
pub use material_ubo::*;
pub use material_manager::*;
pub use texture_manager::*;

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
