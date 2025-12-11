//! Material cache for deduplicating and managing loaded materials
//!
//! Provides caching layer on top of MaterialLoader to avoid reloading
//! the same MTL files multiple times.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use super::material_loader::MaterialLoader;
use crate::render::resources::materials::Material;

/// Cache entry with material and metadata
#[derive(Clone)]
struct CacheEntry {
    /// Cached material
    material: Arc<Material>,
    /// File modification time when loaded
    modified_time: Option<SystemTime>,
}

/// Thread-safe material cache for deduplicating loaded materials
pub struct MaterialCache {
    /// Cache storage: path -> material
    cache: RwLock<HashMap<PathBuf, CacheEntry>>,
}

impl MaterialCache {
    /// Create a new empty material cache
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Load a material from MTL file, using cache if available
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    /// * `material_name` - Name of the material to load
    ///
    /// # Returns
    /// A shared reference to the cached material
    pub fn load_or_get(
        &self,
        mtl_path: impl AsRef<Path>,
        material_name: &str,
    ) -> Result<Arc<Material>, String> {
        let mtl_path = mtl_path.as_ref();
        let cache_key = Self::make_cache_key(mtl_path, material_name);

        // Check if we need to reload due to file modification
        let should_reload = {
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                // Check if file has been modified since we cached it
                if let Some(cached_time) = entry.modified_time {
                    if let Ok(metadata) = std::fs::metadata(mtl_path) {
                        if let Ok(current_time) = metadata.modified() {
                            current_time > cached_time
                        } else {
                            false // Can't determine, assume not modified
                        }
                    } else {
                        false // File doesn't exist, use cache
                    }
                } else {
                    false // No timestamp, use cache
                }
            } else {
                true // Not in cache, need to load
            }
        };

        if !should_reload {
            // Return cached version
            let cache = self.cache.read().unwrap();
            if let Some(entry) = cache.get(&cache_key) {
                return Ok(Arc::clone(&entry.material));
            }
        }

        // Load from file
        let loaded = MaterialLoader::load_mtl(mtl_path, material_name)?;
        let modified_time = std::fs::metadata(mtl_path)
            .ok()
            .and_then(|m| m.modified().ok());

        // Cache it (store only the material, not the texture paths)
        let material_arc = Arc::new(loaded.material);
        let entry = CacheEntry {
            material: Arc::clone(&material_arc),
            modified_time,
        };

        let mut cache = self.cache.write().unwrap();
        cache.insert(cache_key, entry);

        Ok(material_arc)
    }

    /// Load all materials from an MTL file, using cache if available
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    ///
    /// # Returns
    /// A vector of (material_name, Arc<Material>) tuples
    pub fn load_all_or_get(
        &self,
        mtl_path: impl AsRef<Path>,
    ) -> Result<Vec<(String, Arc<Material>)>, String> {
        let mtl_path = mtl_path.as_ref();

        // Load all materials from file (not using cache for bulk load to keep it simple)
        let loaded_materials = MaterialLoader::load_all_mtl(mtl_path)?;
        let modified_time = std::fs::metadata(mtl_path)
            .ok()
            .and_then(|m| m.modified().ok());

        let mut result = Vec::new();
        let mut cache = self.cache.write().unwrap();

        for (name, loaded) in loaded_materials {
            let cache_key = Self::make_cache_key(mtl_path, &name);
            let material_arc = Arc::new(loaded.material);

            let entry = CacheEntry {
                material: Arc::clone(&material_arc),
                modified_time,
            };

            cache.insert(cache_key, entry);
            result.push((name, material_arc));
        }

        Ok(result)
    }

    /// Get a cached material without loading
    ///
    /// # Arguments
    /// * `mtl_path` - Path to the .mtl file
    /// * `material_name` - Name of the material
    ///
    /// # Returns
    /// The cached material if present, None otherwise
    pub fn get_cached(
        &self,
        mtl_path: impl AsRef<Path>,
        material_name: &str,
    ) -> Option<Arc<Material>> {
        let cache_key = Self::make_cache_key(mtl_path.as_ref(), material_name);
        let cache = self.cache.read().unwrap();
        cache.get(&cache_key).map(|entry| Arc::clone(&entry.material))
    }

    /// Check if a material is cached
    pub fn is_cached(&self, mtl_path: impl AsRef<Path>, material_name: &str) -> bool {
        let cache_key = Self::make_cache_key(mtl_path.as_ref(), material_name);
        let cache = self.cache.read().unwrap();
        cache.contains_key(&cache_key)
    }

    /// Force reload a material from disk, bypassing cache
    pub fn reload(
        &self,
        mtl_path: impl AsRef<Path>,
        material_name: &str,
    ) -> Result<Arc<Material>, String> {
        let mtl_path = mtl_path.as_ref();
        let cache_key = Self::make_cache_key(mtl_path, material_name);

        // Remove from cache
        {
            let mut cache = self.cache.write().unwrap();
            cache.remove(&cache_key);
        }

        // Load fresh
        self.load_or_get(mtl_path, material_name)
    }

    /// Clear all cached materials
    pub fn clear(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }

    /// Get the number of cached materials
    pub fn len(&self) -> usize {
        let cache = self.cache.read().unwrap();
        cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check all cached materials for file modifications and reload if needed
    ///
    /// Returns the number of materials that were reloaded
    pub fn check_for_updates(&self) -> usize {
        let mut reloaded_count = 0;
        let entries_to_reload: Vec<(PathBuf, String)> = {
            let cache = self.cache.read().unwrap();
            
            cache.iter()
                .filter_map(|(key, entry)| {
                    // Extract mtl_path and material_name from cache key
                    let mtl_path = key.parent()?;
                    let material_name = key.file_name()?.to_str()?;
                    
                    // Check if file has been modified
                    if let Some(cached_time) = entry.modified_time {
                        if let Ok(metadata) = std::fs::metadata(mtl_path) {
                            if let Ok(current_time) = metadata.modified() {
                                if current_time > cached_time {
                                    return Some((mtl_path.to_path_buf(), material_name.to_string()));
                                }
                            }
                        }
                    }
                    None
                })
                .collect()
        };
        
        // Reload each modified material
        for (mtl_path, material_name) in entries_to_reload {
            if self.reload(&mtl_path, &material_name).is_ok() {
                reloaded_count += 1;
            }
        }
        
        reloaded_count
    }

    /// Create a cache key from path and material name
    fn make_cache_key(mtl_path: &Path, material_name: &str) -> PathBuf {
        // Combine path and material name for unique key
        let mut key = mtl_path.to_path_buf();
        key.push(material_name);
        key
    }
}

impl Default for MaterialCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Write, Seek};
    use tempfile::NamedTempFile;

    #[test]
    fn test_cache_basic() {
        let cache = MaterialCache::new();
        assert!(cache.is_empty());

        // Create test MTL file
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl TestMat\nKd 1.0 0.0 0.0\n").unwrap();

        // Load material
        let mat1 = cache.load_or_get(temp_file.path(), "TestMat").unwrap();
        assert_eq!(cache.len(), 1);

        // Load again - should return cached version
        let mat2 = cache.load_or_get(temp_file.path(), "TestMat").unwrap();
        assert_eq!(cache.len(), 1);

        // Verify it's the same Arc
        assert!(Arc::ptr_eq(&mat1, &mat2));
    }

    #[test]
    fn test_get_cached() {
        let cache = MaterialCache::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl TestMat\nKd 1.0 0.0 0.0\n").unwrap();

        // Not cached yet
        assert!(!cache.is_cached(temp_file.path(), "TestMat"));
        assert!(cache.get_cached(temp_file.path(), "TestMat").is_none());

        // Load it
        cache.load_or_get(temp_file.path(), "TestMat").unwrap();

        // Now cached
        assert!(cache.is_cached(temp_file.path(), "TestMat"));
        assert!(cache.get_cached(temp_file.path(), "TestMat").is_some());
    }

    #[test]
    fn test_load_all() {
        let cache = MaterialCache::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl Mat1\nKd 1.0 0.0 0.0\n\nnewmtl Mat2\nKd 0.0 1.0 0.0\n").unwrap();

        let materials = cache.load_all_or_get(temp_file.path()).unwrap();
        assert_eq!(materials.len(), 2);
        assert_eq!(cache.len(), 2);

        // Both should be cached
        assert!(cache.is_cached(temp_file.path(), "Mat1"));
        assert!(cache.is_cached(temp_file.path(), "Mat2"));
    }

    #[test]
    fn test_clear() {
        let cache = MaterialCache::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl TestMat\nKd 1.0 0.0 0.0\n").unwrap();

        cache.load_or_get(temp_file.path(), "TestMat").unwrap();
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(!cache.is_cached(temp_file.path(), "TestMat"));
    }

    #[test]
    fn test_reload() {
        let cache = MaterialCache::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl TestMat\nKd 1.0 0.0 0.0\n").unwrap();

        let mat1 = cache.load_or_get(temp_file.path(), "TestMat").unwrap();

        // Modify file
        temp_file.seek(std::io::SeekFrom::Start(0)).unwrap();
        write!(temp_file, "newmtl TestMat\nKd 0.0 1.0 0.0\n").unwrap();
        temp_file.flush().unwrap();

        // Reload
        let mat2 = cache.reload(temp_file.path(), "TestMat").unwrap();

        // Should be different Arc instances
        assert!(!Arc::ptr_eq(&mat1, &mat2));
    }

    #[test]
    fn test_different_materials_same_file() {
        let cache = MaterialCache::new();

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "newmtl Mat1\nKd 1.0 0.0 0.0\n\nnewmtl Mat2\nKd 0.0 1.0 0.0\n").unwrap();

        let mat1 = cache.load_or_get(temp_file.path(), "Mat1").unwrap();
        let mat2 = cache.load_or_get(temp_file.path(), "Mat2").unwrap();

        assert_eq!(cache.len(), 2);
        assert!(!Arc::ptr_eq(&mat1, &mat2));
    }
}
