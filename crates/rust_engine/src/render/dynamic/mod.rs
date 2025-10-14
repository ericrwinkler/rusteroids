//! Dynamic rendering system
//!
//! This module provides the dynamic object rendering system for temporary objects
//! with bounded lifetimes. It uses resource pooling and batch rendering for
//! efficient memory usage and performance.
//!
//! # Single-Mesh Pool Architecture
//! 
//! The system uses single-mesh pools for optimal performance. Each pool manages
//! objects of exactly one mesh type, enabling perfect batching and minimal state
//! changes during rendering.

pub mod object_manager;
pub mod instance_renderer;
pub mod resource_pool;
pub mod data_structures;
pub mod spawner;
pub mod mesh_type;
pub mod pool_manager;

pub use object_manager::*;
pub use instance_renderer::*;
pub use mesh_type::*;
pub use pool_manager::*;
pub use resource_pool::*;
pub use data_structures::*;
pub use spawner::*;