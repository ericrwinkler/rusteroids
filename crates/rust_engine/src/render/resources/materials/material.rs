//! Material type definitions and enums
//!
//! This module defines the different types of materials supported by the engine
//! and provides a unified interface for material management.

use super::{StandardMaterialParams, UnlitMaterialParams, MaterialTextures, TextureHandle};

/// Enumeration of supported material types
#[derive(Debug, Clone)]
pub enum MaterialType {
    /// Standard PBR material with full physically-based rendering
    StandardPBR(StandardMaterialParams),
    /// Unlit material for simple color/texture rendering
    Unlit(UnlitMaterialParams),
    /// Transparent material with alpha blending
    Transparent {
        /// Base material to make transparent
        base_material: Box<MaterialType>,
        /// Alpha blending mode to use
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
    /// Texture bindings for this material
    pub textures: MaterialTextures,
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
            textures: MaterialTextures::new(),
            id: MaterialId(0), // Will be assigned by MaterialManager
            name: None,
        }
    }

    /// Create a new unlit material
    pub fn unlit(params: UnlitMaterialParams) -> Self {
        Self {
            material_type: MaterialType::Unlit(params),
            textures: MaterialTextures::new(),
            id: MaterialId(0), // Will be assigned by MaterialManager
            name: None,
        }
    }
    
    /// Create a new transparent PBR material with alpha blending
    pub fn transparent_pbr(params: StandardMaterialParams) -> Self {
        Self {
            material_type: MaterialType::Transparent {
                base_material: Box::new(MaterialType::StandardPBR(params)),
                alpha_mode: AlphaMode::Blend,
            },
            textures: MaterialTextures::new(),
            id: MaterialId(0),
            name: None,
        }
    }
    
    /// Create a new transparent unlit material with alpha blending
    pub fn transparent_unlit(params: UnlitMaterialParams) -> Self {
        Self {
            material_type: MaterialType::Transparent {
                base_material: Box::new(MaterialType::Unlit(params)),
                alpha_mode: AlphaMode::Blend,
            },
            textures: MaterialTextures::new(),
            id: MaterialId(0),
            name: None,
        }
    }

    /// Set the material name for debugging
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Attach a base color texture
    pub fn with_base_color_texture(mut self, texture: TextureHandle) -> Self {
        self.textures.base_color = Some(texture);
        
        // Enable base color texture in material params
        match &mut self.material_type {
            MaterialType::StandardPBR(params) => {
                params.base_color_texture_enabled = true;
            }
            MaterialType::Transparent { base_material, .. } => {
                if let MaterialType::StandardPBR(params) = &mut **base_material {
                    params.base_color_texture_enabled = true;
                }
            }
            _ => {}
        }
        
        self
    }

    /// Attach a normal map texture
    pub fn with_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.textures.normal = Some(texture);
        
        // Enable normal texture in material params
        match &mut self.material_type {
            MaterialType::StandardPBR(params) => {
                params.normal_texture_enabled = true;
            }
            MaterialType::Transparent { base_material, .. } => {
                if let MaterialType::StandardPBR(params) = &mut **base_material {
                    params.normal_texture_enabled = true;
                }
            }
            _ => {}
        }
        
        self
    }

    /// Attach a metallic-roughness texture
    pub fn with_metallic_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.textures.metallic_roughness = Some(texture);
        self
    }

    /// Attach an ambient occlusion texture
    pub fn with_ao_texture(mut self, texture: TextureHandle) -> Self {
        self.textures.ambient_occlusion = Some(texture);
        self
    }

    /// Attach an emission texture
    pub fn with_emission_texture(mut self, texture: TextureHandle) -> Self {
        self.textures.emission = Some(texture);
        self
    }

    /// Attach an opacity texture
    pub fn with_opacity_texture(mut self, texture: TextureHandle) -> Self {
        self.textures.opacity = Some(texture);
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
    
    // FIXME: Implement automatic render priority assignment when system is added
    // /// Get the render priority for this material (determines rendering order)
    // pub fn render_priority(&self) -> crate::render::pipeline::RenderPriority {
    //     use crate::render::pipeline::RenderPriority;
    //     match &self.material_type {
    //         MaterialType::StandardPBR(_) => RenderPriority::Geometry,
    //         MaterialType::Unlit(_) => RenderPriority::Geometry,
    //         MaterialType::Transparent { alpha_mode, .. } => {
    //             match alpha_mode {
    //                 AlphaMode::Mask(_) => RenderPriority::AlphaTest,
    //                 _ => RenderPriority::Transparent,
    //             }
    //         }
    //     }
    // }
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

// Temporary compatibility methods for the old Material API
// TODO: Remove these when the renderer is updated to use the new material system
impl Material {
    /// Get base color as an array for compatibility with old renderer
    /// This is a temporary method during the transition to the new material system
    pub fn get_base_color_array(&self) -> [f32; 4] {
        match &self.material_type {
            MaterialType::StandardPBR(params) => {
                [params.base_color.x, params.base_color.y, params.base_color.z, params.alpha]
            }
            MaterialType::Unlit(params) => {
                [params.color.x, params.color.y, params.color.z, params.alpha]
            }
            MaterialType::Transparent { base_material, .. } => {
                // For transparent materials, get the base color from the underlying material
                match &**base_material {
                    MaterialType::StandardPBR(params) => {
                        [params.base_color.x, params.base_color.y, params.base_color.z, params.alpha]
                    }
                    MaterialType::Unlit(params) => {
                        [params.color.x, params.color.y, params.color.z, params.alpha]
                    }
                    MaterialType::Transparent { .. } => {
                        // Nested transparency not supported, return white
                        [1.0, 1.0, 1.0, 1.0]
                    }
                }
            }
        }
    }
}
