//! Pipeline management system for handling multiple rendering pipelines
//! 
//! This module provides a centralized system for managing multiple graphics pipelines
//! that correspond to different material types (StandardPBR, Unlit, Transparent).
//! Each pipeline can have its own shaders, rendering state, and configuration.

pub mod pipeline_manager;
pub mod pipeline_config;

pub use pipeline_manager::PipelineManager;
pub use pipeline_config::{PipelineConfig, CullMode};
