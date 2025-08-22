//! Pipeline configuration and type definitions
//! 
//! Defines the different types of rendering pipelines and their configurations.

use crate::render::material::PipelineType;

/// Configuration for a graphics pipeline
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    /// Type of pipeline
    pub pipeline_type: PipelineType,
    /// Vertex shader path relative to resources/shaders/
    pub vertex_shader_path: String,
    /// Fragment shader path relative to resources/shaders/
    pub fragment_shader_path: String,
    /// Enable depth testing
    pub depth_test: bool,
    /// Enable depth writing
    pub depth_write: bool,
    /// Enable alpha blending
    pub alpha_blending: bool,
    /// Cull mode for backface culling
    pub cull_mode: CullMode,
}

/// Face culling modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CullMode {
    /// No culling
    None,
    /// Cull front faces
    Front,
    /// Cull back faces
    Back,
}

impl PipelineConfig {
    /// Create configuration for Standard PBR pipeline
    pub fn standard_pbr() -> Self {
        Self {
            pipeline_type: PipelineType::StandardPBR,
            vertex_shader_path: "target/shaders/standard_pbr_vert.spv".to_string(),
            fragment_shader_path: "target/shaders/standard_pbr_frag.spv".to_string(),
            depth_test: true,
            depth_write: true,
            alpha_blending: false,
            cull_mode: CullMode::Back,
        }
    }

    /// Create configuration for Unlit pipeline
    pub fn unlit() -> Self {
        Self {
            pipeline_type: PipelineType::Unlit,
            vertex_shader_path: "target/shaders/vert_ubo.spv".to_string(),
            fragment_shader_path: "target/shaders/frag_ubo_simple.spv".to_string(),
            depth_test: true,
            depth_write: true,
            alpha_blending: false,
            cull_mode: CullMode::Back,
        }
    }

    /// Create configuration for Transparent PBR pipeline
    pub fn transparent_pbr() -> Self {
        Self {
            pipeline_type: PipelineType::TransparentPBR,
            vertex_shader_path: "target/shaders/standard_pbr_vert.spv".to_string(),
            fragment_shader_path: "target/shaders/standard_pbr_frag.spv".to_string(),
            depth_test: true,
            depth_write: false, // Don't write to depth buffer for transparency
            alpha_blending: true,
            cull_mode: CullMode::None, // Don't cull for transparency
        }
    }

    /// Create configuration for Transparent Unlit pipeline
    pub fn transparent_unlit() -> Self {
        Self {
            pipeline_type: PipelineType::TransparentUnlit,
            vertex_shader_path: "target/shaders/vert_ubo.spv".to_string(),
            fragment_shader_path: "target/shaders/unlit_frag.spv".to_string(),
            depth_test: true,
            depth_write: false, // Don't write to depth buffer for transparency
            alpha_blending: true,
            cull_mode: CullMode::None, // Don't cull for transparency
        }
    }
}
