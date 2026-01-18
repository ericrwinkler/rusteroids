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
    /// Text quad mesh - for rendering text strings
    /// Each text string generates a unique mesh with positioned quads
    TextQuad,
    /// Spaceship mesh - player or enemy ships
    Spaceship,
    /// Frigate mesh - large capital ships
    Frigate,
    /// Monkey mesh - Blender's iconic test mesh
    Monkey,
    /// Billboard quad - for particles, trails, effects (generic)
    Billboard,
    /// Billboard quad - for bullet projectiles
    BillboardBullet,
    /// Billboard quad - for explosion effects
    BillboardExplosion,
    /// Turret base mesh - rotating base of turret
    TurretBase,
    /// Turret barrel mesh - recoiling barrel of turret
    TurretBarrel,
}

impl MeshType {
    /// Get all available mesh types
    pub fn all() -> &'static [MeshType] {
        &[MeshType::Teapot, MeshType::Sphere, MeshType::Cube, MeshType::TextQuad, MeshType::Spaceship, MeshType::Frigate, MeshType::Monkey, MeshType::Billboard, MeshType::BillboardBullet, MeshType::BillboardExplosion, MeshType::TurretBase, MeshType::TurretBarrel]
    }
    
    /// Get the default mesh model path for this mesh type
    pub fn default_model_path(&self) -> &'static str {
        match self {
            MeshType::Teapot => "models/teapot.obj",
            MeshType::Sphere => "models/sphere.obj",
            MeshType::Cube => "models/cube.obj",
            MeshType::TextQuad => "", // Text quads are generated procedurally, not loaded
            MeshType::Spaceship => "models/spaceship_simple.obj",
            MeshType::Frigate => "models/frigate.obj",
            MeshType::Monkey => "models/monkey.obj",
            MeshType::Billboard => "", // Billboard quads are generated procedurally
            MeshType::BillboardBullet => "", // Billboard quads are generated procedurally
            MeshType::BillboardExplosion => "", // Billboard quads are generated procedurally
            MeshType::TurretBase => "models/simple_turret_base.obj",
            MeshType::TurretBarrel => "models/simple_turret_barrel.obj",
        }
    }
    
    /// Get the human-readable name for this mesh type
    pub fn name(&self) -> &'static str {
        match self {
            MeshType::Teapot => "Teapot",
            MeshType::Sphere => "Sphere", 
            MeshType::Cube => "Cube",
            MeshType::TextQuad => "TextQuad",
            MeshType::Spaceship => "Spaceship",
            MeshType::Frigate => "Frigate",
            MeshType::Monkey => "Monkey",
            MeshType::Billboard => "Billboard",
            MeshType::BillboardBullet => "BillboardBullet",
            MeshType::BillboardExplosion => "BillboardExplosion",
            MeshType::TurretBase => "TurretBase",
            MeshType::TurretBarrel => "TurretBarrel",
        }
    }
    
    /// Check if this mesh type is a billboard (transient particle effect)
    pub fn is_billboard(&self) -> bool {
        matches!(self, MeshType::Billboard | MeshType::BillboardBullet | MeshType::BillboardExplosion)
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