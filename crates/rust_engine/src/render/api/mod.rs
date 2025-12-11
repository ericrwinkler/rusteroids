//! Public rendering API
//!
//! This module contains the high-level public API that users interact with,
//! including the graphics engine, render backend trait, and renderer configuration.

pub mod render_backend;
pub mod renderer_config;
pub mod frame_data;

// Re-export commonly used types
pub use render_backend::{RenderBackend, WindowBackendAccess, BackendResult, MeshHandle, MaterialHandle, ObjectResourceHandle};
pub use renderer_config::VulkanRendererConfig;
pub use frame_data::RenderFrameData;

// TODO: Extract GraphicsEngine from render/mod.rs to api/graphics_engine.rs
// For now, GraphicsEngine remains in render/mod.rs due to its size (~1350 lines)
