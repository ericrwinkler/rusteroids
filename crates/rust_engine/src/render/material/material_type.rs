//! Material type definitions and enums
//!
//! This module defines the different types of materials supported by the engine
//! and provides a unified interface for material management.

use super::{StandardMaterialParams, UnlitMaterialParams};

/// Enumeration of supported material types
#[derive(Debug, Clone)]
pub enum MaterialType {
    /// Standard PBR material with full physically-based rendering
    StandardPBR(StandardMaterialParams),
    /// Unlit material for simple color/texture rendering
    Unlit(UnlitMaterialParams),
    /// Transparent material with alpha blending
    Transparent {
        base_material: Box<MaterialType>,
        alpha_mode: AlphaMode,
    },
}

/// Alpha blending modes for transparent materials
#[derive(Debug, Clone, Copy)]
pub enum AlphaMode {
    /// No transparency
    Opaque,
    /// Alpha testing with cutoff value
    Mask(f32),
    /// Alpha blending
    Blend,
}

impl Default for AlphaMode {
    fn default() -> Self {
        AlphaMode::Opaque
    }
}

/// Material resource containing type and GPU data
#[derive(Debug, Clone)]
pub struct Material {
    /// Material type and parameters
    pub material_type: MaterialType,
    /// Unique identifier for this material
    pub id: MaterialId,
    /// Optional name for debugging
    pub name: Option<String>,
}

/// Unique identifier for materials
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(pub u32);

impl Material {
    /// Create a new standard PBR material
    pub fn standard_pbr(params: StandardMaterialParams) -> Self {
        Self {
            material_type: MaterialType::StandardPBR(params),
            id: MaterialId(0), // Will be assigned by MaterialManager
            name: None,
        }
    }

    /// Create a new unlit material
    pub fn unlit(params: UnlitMaterialParams) -> Self {
        Self {
            material_type: MaterialType::Unlit(params),
            id: MaterialId(0), // Will be assigned by MaterialManager
            name: None,
        }
    }

    /// Set the material name for debugging
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Get the pipeline type required for this material
    pub fn required_pipeline(&self) -> PipelineType {
        match &self.material_type {
            MaterialType::StandardPBR(_) => PipelineType::StandardPBR,
            MaterialType::Unlit(_) => PipelineType::Unlit,
            MaterialType::Transparent { base_material, .. } => {
                // Transparent materials use the same pipeline as base but with blending
                match **base_material {
                    MaterialType::StandardPBR(_) => PipelineType::TransparentPBR,
                    MaterialType::Unlit(_) => PipelineType::TransparentUnlit,
                    MaterialType::Transparent { .. } => {
                        // Nested transparent materials not supported, default to transparent PBR
                        PipelineType::TransparentPBR
                    }
                }
            }
        }
    }
}

/// Pipeline types required for different materials
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineType {
    /// Standard opaque PBR rendering
    StandardPBR,
    /// Simple unlit rendering
    Unlit,
    /// Transparent PBR with alpha blending
    TransparentPBR,
    /// Transparent unlit with alpha blending
    TransparentUnlit,
}

impl PipelineType {
    /// Get the shader variant name for this pipeline type
    pub fn shader_variant(&self) -> &'static str {
        match self {
            PipelineType::StandardPBR => "standard_pbr",
            PipelineType::Unlit => "unlit",
            PipelineType::TransparentPBR => "transparent_pbr",
            PipelineType::TransparentUnlit => "transparent_unlit",
        }
    }

    /// Check if this pipeline type requires alpha blending
    pub fn requires_blending(&self) -> bool {
        match self {
            PipelineType::TransparentPBR | PipelineType::TransparentUnlit => true,
            _ => false,
        }
    }
}

impl Material {
    /// Get base color for backward compatibility (TEMPORARY)
    /// TODO: Remove this when the old API is fully replaced
    pub fn get_base_color_array(&self) -> [f32; 4] {
        match &self.material_type {
            MaterialType::StandardPBR(params) => [
                params.base_color.x,
                params.base_color.y,
                params.base_color.z,
                params.alpha,
            ],
            MaterialType::Unlit(params) => [
                params.color.x,
                params.color.y,
                params.color.z,
                params.alpha,
            ],
            MaterialType::Transparent { base_material, .. } => {
                // Get color from base material
                match &**base_material {
                    MaterialType::StandardPBR(params) => [
                        params.base_color.x,
                        params.base_color.y,
                        params.base_color.z,
                        params.alpha,
                    ],
                    MaterialType::Unlit(params) => [
                        params.color.x,
                        params.color.y,
                        params.color.z,
                        params.alpha,
                    ],
                    MaterialType::Transparent { .. } => [1.0, 1.0, 1.0, 1.0], // Default
                }
            }
        }
    }
}
