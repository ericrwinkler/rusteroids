//! Shared rendering resources
//!
//! Resources that are shared between multiple rendering objects,
//! including mesh resources, game objects, and render queues.

pub mod shared_mesh_resources;
pub mod game_object;
pub mod render_queue;

// Re-export commonly used types
pub use shared_mesh_resources::{SharedRenderingResources, SharedRenderingResourcesBuilder};
pub use game_object::{GameObject, ObjectUBO, cleanup_game_object};
pub use render_queue::{RenderQueue, RenderCommand, CommandType, RenderCommandId};
