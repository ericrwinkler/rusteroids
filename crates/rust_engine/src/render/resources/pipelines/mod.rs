//! Pipeline management
//!
//! Pipeline configuration and selection logic.

pub mod pipeline_manager;
pub mod pipeline_config;

// Re-export commonly used types
pub use pipeline_manager::PipelineManager;
pub use pipeline_config::{PipelineConfig, CullMode, BlendMode, RenderPriority, PolygonMode, DepthBiasConfig};
