//! Dynamic rendering system
//!
//! This module provides the dynamic object rendering system for temporary objects
//! with bounded lifetimes. It uses resource pooling and batch rendering for
//! efficient memory usage and performance.

pub mod object_manager;
pub mod instance_renderer;
pub mod resource_pool;
pub mod data_structures;
pub mod spawner;

pub use object_manager::*;
pub use instance_renderer::*;
pub use resource_pool::*;
pub use data_structures::*;
pub use spawner::*;