//! Asset management system

pub mod obj_loader;
pub mod image_loader;
pub mod materials;
pub mod resource_manager;

pub use obj_loader::ObjLoader;
pub use image_loader::ImageData;
pub use materials::{
    MtlParser, MtlData,
    MaterialLoader, LoadedMaterial, MaterialTexturePaths,
    MaterialBuilder,
    MaterialCache,
    MaterialFactory,
};
pub use resource_manager::{
    ResourceManager, ResourceConfig, ResourceError, PoolKey,
};

#[cfg(test)]
mod test_texture_loading;

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
    /// Material cache for MTL file materials
    material_cache: MaterialCache,
}

impl AssetManager {
    /// Create a new asset manager
    pub fn new(config: &AssetConfig) -> Result<Self, AssetError> {
        Ok(Self {
            asset_storages: HashMap::new(),
            config: config.clone(),
            material_cache: MaterialCache::new(),
        })
    }
    
    /// Update the asset manager (hot reloading, etc.)
    pub fn update(&mut self) -> Result<(), AssetError> {
        // Check for material file updates and hot-reload
        let reloaded_count = self.material_cache.check_for_updates();
        if reloaded_count > 0 {
            log::info!("Hot-reloaded {} material(s)", reloaded_count);
        }
        Ok(())
    }
    
    /// Get the material cache for loading MTL materials
    pub fn material_cache(&self) -> &MaterialCache {
        &self.material_cache
    }
    
    /// Get mutable access to the material cache
    pub fn material_cache_mut(&mut self) -> &mut MaterialCache {
        &mut self.material_cache
    }
    
    /// Load an asset from disk
    ///
    /// # Arguments
    /// * `path` - Path to the asset file (relative to search paths)
    ///
    /// # Returns
    /// A handle to the loaded asset
    ///
    /// # Example
    /// ```ignore
    /// let audio_handle = asset_manager.load::<AudioAsset>("audio/explosion.wav")?;
    /// ```
    pub fn load<T: Asset>(&mut self, path: &str) -> Result<AssetHandle<T>, AssetError> {
        use crate::foundation::collections::HandleMap;
        use std::fs;
        use std::path::PathBuf;
        
        // Try each search path
        let mut full_path = None;
        for search_path in &self.config.search_paths {
            let mut candidate = PathBuf::from(search_path);
            candidate.push(path);
            if candidate.exists() {
                full_path = Some(candidate);
                break;
            }
        }
        
        // If not found in search paths, try as absolute path
        let file_path = full_path.unwrap_or_else(|| PathBuf::from(path));
        
        if !file_path.exists() {
            return Err(AssetError::NotFound(path.to_string()));
        }
        
        // Read file contents
        let bytes = fs::read(&file_path)
            .map_err(|e| AssetError::IoError(e))?;
        
        // Parse asset from bytes
        let asset = T::from_bytes(&bytes)?;
        
        // Get or create storage for this asset type
        let type_id = TypeId::of::<T>();
        let storage = self.asset_storages
            .entry(type_id)
            .or_insert_with(|| Box::new(HandleMap::<T>::new()));
        
        // Cast to the correct type
        let typed_storage = storage
            .downcast_mut::<HandleMap<T>>()
            .ok_or_else(|| AssetError::StorageError("Failed to cast storage".to_string()))?;
        
        // Store the asset and get its handle
        let handle_key = typed_storage.insert(asset);
        
        Ok(AssetHandle::new(handle_key))
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
    
    /// Invalid asset data
    #[error("Invalid data: {0}")]
    InvalidData(String),
    
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
