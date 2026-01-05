//! Spatial partitioning data structures
//!
//! Provides efficient spatial indexing for collision detection,
//! ray casting, and proximity queries in 3D space.

mod octree;
pub mod spatial_query;

pub use octree::{Octree, OctreeNode, OctreeConfig};
pub use spatial_query::{SpatialQuery, OctreeSpatialQuery};
