//! Spatial partitioning data structures
//!
//! Provides efficient spatial indexing for collision detection,
//! ray casting, and proximity queries in 3D space.

mod octree;

pub use octree::{Octree, OctreeNode, OctreeConfig};
