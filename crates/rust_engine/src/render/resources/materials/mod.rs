//! Material system
//!
//! Material definitions, texture management, and shader configuration.

pub mod material;
pub mod material_params;
pub mod material_registry;
pub mod material_ubo;
pub mod texture_registry;
pub mod shader_config;

// Re-export commonly used types
pub use material::{Material, MaterialType, MaterialId, PipelineType, AlphaMode};
pub use material_params::{StandardMaterialParams, UnlitMaterialParams, BillboardMaterialParams, BillboardBlendMode};
pub use material_registry::MaterialManager;
pub use material_ubo::{MaterialUBO, StandardMaterialUBO, UnlitMaterialUBO};
pub use texture_registry::{TextureManager, TextureHandle, TextureType, MaterialTextures, TextureInfo, TextureParams, FilterMode, WrapMode};
pub use shader_config::ShaderConfig;
