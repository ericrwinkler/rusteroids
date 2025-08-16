//! Asset management system

pub mod obj_loader;

pub use obj_loader::ObjLoader;

use thiserror::Error;
use crate::engine::AssetConfig;
use crate::foundation::collections::TypedHandle;
use std::any::{TypeId, Any};
use std::collections::HashMap;

/// Asset handle type
pub type AssetHandle<T> = TypedHandle<T>;

/// Asset management system
pub struct AssetManager {
    asset_storages: HashMap<TypeId, Box<dyn Any>>,
    #[allow(dead_code)] // Will be used for asset loading configuration
    config: AssetConfig,
}

impl AssetManager {
    /// Create a new asset manager
    pub fn new(config: &AssetConfig) -> Result<Self, AssetError> {
        Ok(Self {
            asset_storages: HashMap::new(),
            config: config.clone(),
        })
    }
    
    /// Update the asset manager (hot reloading, etc.)
    pub fn update(&mut self) -> Result<(), AssetError> {
        // TODO: Implement hot reloading
        Ok(())
    }
    
    /// Load an asset from disk
    pub fn load<T: Asset>(&mut self, _path: &str) -> Result<AssetHandle<T>, AssetError> {
        // TODO: Implement asset loading
        todo!("Asset loading not yet implemented")
    }
    
    /// Create a mesh asset from runtime data
    pub fn create_mesh_from_data(&mut self, mesh: crate::render::Mesh) -> Result<AssetHandle<crate::render::Mesh>, AssetError> {
        use crate::foundation::collections::HandleMap;
        
        // Get or create storage for Mesh assets
        let type_id = TypeId::of::<crate::render::Mesh>();
        let storage = self.asset_storages
            .entry(type_id)
            .or_insert_with(|| Box::new(HandleMap::<crate::render::Mesh>::new()));
        
        // Cast to the correct type
        let mesh_storage = storage
            .downcast_mut::<HandleMap<crate::render::Mesh>>()
            .ok_or(AssetError::StorageError("Failed to cast mesh storage".to_string()))?;
        
        // Store the mesh and get its handle
        let handle_key = mesh_storage.insert(mesh);
        
        // Create and return handle
        Ok(AssetHandle::new(handle_key))
    }
    
    /// Get an asset by handle
    pub fn get<T: Asset>(&self, handle: AssetHandle<T>) -> Option<&T> {
        use crate::foundation::collections::HandleMap;
        
        let type_id = TypeId::of::<T>();
        let storage = self.asset_storages.get(&type_id)?;
        
        // Cast to the correct type
        let typed_storage = storage
            .downcast_ref::<HandleMap<T>>()?;
        
        // Get the asset by handle key
        typed_storage.get(handle.key())
    }
}

/// Asset trait for loadable resources
pub trait Asset: Send + Sync + 'static {
    /// Load asset from raw bytes
    fn from_bytes(bytes: &[u8]) -> Result<Self, AssetError> where Self: Sized;
}

/// Asset loading errors
#[derive(Error, Debug)]
pub enum AssetError {
    /// Asset not found
    #[error("Asset not found: {0}")]
    NotFound(String),
    
    /// Failed to load asset
    #[error("Failed to load asset: {0}")]
    LoadFailed(String),
    
    /// Unsupported asset format
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    
    /// Storage system error
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// IO error during asset loading
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
