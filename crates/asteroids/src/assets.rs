//! Game asset definitions

use rust_engine::assets::Asset;

/// Ship model asset
pub struct ShipModel {
    /// Vertex data
    pub vertices: Vec<ShipVertex>,
    
    /// Index data
    pub indices: Vec<u32>,
}

/// Ship vertex
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShipVertex {
    /// Position
    pub position: [f32; 2],
    
    /// Color
    pub color: [f32; 3],
}

impl Asset for ShipModel {
    fn from_bytes(_bytes: &[u8]) -> Result<Self, rust_engine::assets::AssetError> {
        // TODO: Implement ship model loading
        Err(rust_engine::assets::AssetError::LoadFailed("Not implemented".to_string()))
    }
}

/// Asteroid model asset
pub struct AsteroidModel {
    /// Vertex data
    pub vertices: Vec<AsteroidVertex>,
    
    /// Index data
    pub indices: Vec<u32>,
}

/// Asteroid vertex
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AsteroidVertex {
    /// Position
    pub position: [f32; 2],
    
    /// UV coordinates
    pub uv: [f32; 2],
}

impl Asset for AsteroidModel {
    fn from_bytes(_bytes: &[u8]) -> Result<Self, rust_engine::assets::AssetError> {
        // TODO: Implement asteroid model loading
        Err(rust_engine::assets::AssetError::LoadFailed("Not implemented".to_string()))
    }
}

/// Bullet model asset
pub struct BulletModel {
    /// Vertex data
    pub vertices: Vec<BulletVertex>,
    
    /// Index data
    pub indices: Vec<u32>,
}

/// Bullet vertex
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BulletVertex {
    /// Position
    pub position: [f32; 2],
    
    /// Color
    pub color: [f32; 3],
}

impl Asset for BulletModel {
    fn from_bytes(_bytes: &[u8]) -> Result<Self, rust_engine::assets::AssetError> {
        // TODO: Implement bullet model loading
        Err(rust_engine::assets::AssetError::LoadFailed("Not implemented".to_string()))
    }
}
