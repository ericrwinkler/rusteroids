//! Pipeline configuration and type definitions
//! 
//! Defines the different types of rendering pipelines and their configurations.

use crate::render::material::PipelineType;

// FIXME: Implement BlendMode variants when particle system is added
/// Blending modes for different rendering effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard alpha blending (current implementation)
    Alpha,
    /// Additive blending for particles and lights (FIXME: implement)
    #[allow(dead_code)]
    Additive,
    /// Multiplicative blending for shadows (FIXME: implement)
    #[allow(dead_code)]
    Multiplicative,
    /// Pre-multiplied alpha (FIXME: implement)
    #[allow(dead_code)]
    Premultiplied,
}

// FIXME: Implement RenderQueue system for automatic ordering
/// Render queue/priority determines when objects are rendered relative to others
/// (Renamed from RenderQueue to avoid conflict with existing render_queue module)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(dead_code)]
pub enum RenderPriority {
    /// Skybox, far background
    Background = 1000,
    /// Opaque solid objects
    Geometry = 2000,
    /// Cutout materials (alpha testing)
    AlphaTest = 2450,
    /// Transparent objects (back-to-front sorted)
    Transparent = 3000,
    /// UI, screen-space effects
    Overlay = 4000,
}

// FIXME: Implement when debug visualization is needed
/// Polygon rendering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum PolygonMode {
    /// Normal solid rendering (current)
    Fill,
    /// Wireframe mode
    Line,
    /// Point cloud mode
    Point,
}

// FIXME: Implement when decal system is added
/// Depth bias configuration to prevent z-fighting
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct DepthBiasConfig {
    /// Enable depth bias
    pub enabled: bool,
    /// Constant depth bias factor
    pub constant_factor: f32,
    /// Slope-scaled depth bias factor
    pub slope_factor: f32,
    /// Maximum depth bias clamp value
    pub clamp: f32,
}

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
    
    // FIXME: Uncomment and implement when blend modes are needed
    // pub blend_mode: BlendMode,
    
    // FIXME: Uncomment and implement when render priority system is needed
    // pub render_priority: RenderPriority,
    
    // FIXME: Uncomment and implement for two-sided materials
    // pub two_sided: bool,
    
    // FIXME: Uncomment and implement when wireframe debug is needed
    // pub polygon_mode: PolygonMode,
    
    // FIXME: Uncomment and implement when decal system is added
    // pub depth_bias: Option<DepthBiasConfig>,
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
            // Use instanced-compatible shaders (read transforms from instance buffer)
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
            // Use instanced vertex shader with unlit fragment shader
            vertex_shader_path: "target/shaders/standard_pbr_vert.spv".to_string(),
            fragment_shader_path: "target/shaders/unlit_frag.spv".to_string(),
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
            // Use the same instanced shaders as StandardPBR
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
            // Use instanced vertex shader with unlit fragment shader
            vertex_shader_path: "target/shaders/standard_pbr_vert.spv".to_string(),
            fragment_shader_path: "target/shaders/unlit_frag.spv".to_string(),
            depth_test: true,
            depth_write: false, // Don't write to depth buffer for transparency
            alpha_blending: true,
            cull_mode: CullMode::None, // Don't cull for transparency
        }
    }
}
