//! Instanced rendering implementation
//!
//! Buffer management, draw recording, and state tracking for instanced rendering.

pub mod instance_renderer;
pub mod pipeline_builder;

pub use instance_renderer::*;
pub use pipeline_builder::*;
