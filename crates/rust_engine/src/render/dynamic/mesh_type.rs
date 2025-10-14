//! Mesh Type Definitions
//!
//! This module defines the mesh types supported by the single-mesh pool system.
//! Each mesh type gets its own dedicated pool for optimal rendering performance.

/// Enumeration of supported mesh types for dynamic object pools
///
/// Each mesh type corresponds to a separate single-mesh pool managed by
/// the MeshPoolManager. This enables optimal batching with O(1) rendering
/// per mesh type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MeshType {
    /// Teapot mesh - primarily used for testing and demos
    Teapot,
    /// Sphere mesh - for particles, projectiles, etc.
    Sphere,
    /// Cube mesh - for blocks, debris, etc.
    Cube,
}

impl MeshType {
    /// Get all available mesh types
    pub fn all() -> &'static [MeshType] {
        &[MeshType::Teapot, MeshType::Sphere, MeshType::Cube]
    }
    
    /// Get the default mesh model path for this mesh type
    pub fn default_model_path(&self) -> &'static str {
        match self {
            MeshType::Teapot => "models/teapot.obj",
            MeshType::Sphere => "models/sphere.obj",
            MeshType::Cube => "models/cube.obj",
        }
    }
    
    /// Get the human-readable name for this mesh type
    pub fn name(&self) -> &'static str {
        match self {
            MeshType::Teapot => "Teapot",
            MeshType::Sphere => "Sphere", 
            MeshType::Cube => "Cube",
        }
    }
}

impl std::fmt::Display for MeshType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for MeshType {
    fn default() -> Self {
        MeshType::Teapot
    }
}