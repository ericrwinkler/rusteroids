//! Batch rendering system
//!
//! Batches draw calls for improved performance.

pub mod batch_renderer;

// Re-export all batching types
pub use batch_renderer::*;
