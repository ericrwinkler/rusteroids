//! Collision detection system using spatial partitioning
//!
//! Provides efficient broad-phase collision detection using octrees,
//! and narrow-phase detection for common shapes.
//!
//! # Architecture
//!
//! This module follows Game Engine Architecture 3rd Edition (GEA 13.3.4):
//! - **Model Space Storage**: Collision shapes stored in local coordinates
//! - **On-Demand Transformation**: Shapes transformed to world space only during tests
//! - **Coordinate Decoupling**: Shape geometry separate from Transform component
//!
//! # Module Organization
//!
//! - [`primitives`] - Basic geometric primitives (rays, spheres, triangles)
//! - [`mesh`] - Complex mesh-based collision geometry
//! - [`shape`] - High-level ECS-friendly collision shapes
//!
//! # Key Types
//!
//! - [`CollisionShape`] - Model-space shape attached to entities (ECS component data)
//! - [`WorldSpaceShape`] - Temporary world-space shape for collision testing
//! - [`Ray`], [`BoundingSphere`], [`Triangle`] - Primitive geometric types

pub mod primitives;
pub mod mesh;
pub mod shape;

// Re-export commonly used types
pub use primitives::{Ray, RayHit, BoundingSphere, Triangle};
pub use mesh::{CollisionMeshTemplate, WorldSpaceCollisionMesh};
pub use shape::{CollisionShape, WorldSpaceShape};
