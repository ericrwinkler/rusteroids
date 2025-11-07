//! Core primitive types for rendering
//!
//! This module contains fundamental data structures used throughout
//! the rendering system, such as meshes, vertices, and coordinate systems.

pub mod mesh;
pub mod coordinates;

// Re-export commonly used types
pub use mesh::{Mesh, Vertex};
pub use coordinates::*;
