//! Shared rendering resources
//!
//! Resources that are shared between multiple rendering objects,
//! including mesh resources and render queues.

pub mod shared_mesh_resources;
pub mod render_queue;
pub mod resource_factory;

// Re-export commonly used types
pub use shared_mesh_resources::{SharedRenderingResources, SharedRenderingResourcesBuilder};
pub use render_queue::{RenderQueue, RenderCommand, CommandType, RenderCommandId};
pub use resource_factory::*;

// Re-export ObjectUBO from its original location for backwards compatibility
pub use crate::render::backends::vulkan::object_data::ObjectUBO;
