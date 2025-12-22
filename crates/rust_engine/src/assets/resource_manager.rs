//! Resource Manager - CPU-Side Resource Tracking and Lifecycle Management
//!
//! **Implements**: Game Engine Architecture Chapter 7.2 (Resource Manager)
//!
//! **ARCHITECTURE PRINCIPLE** (from Game Engine Architecture, Section 1.6.7):
//! "The resource manager provides a unified interface for accessing any and all
//! types of game assets... Present in every game engine in some form..."
//!
//! **SEPARATION OF CONCERNS**:
//! - ResourceManager: CPU-side asset tracking, reference counting, caching
//! - GraphicsEngine: GPU resource creation (pools, descriptors, buffers)
//! - SceneManager: Gameplay-level entity/component management
//!
//! This follows the book's layered architecture (Figure 1.20):
//! ```
//! Game Assets (Meshes, Materials) 
//!        ↓
//! Resource Manager (THIS MODULE - tracks what exists, refcounts)
//!        ↓  
//! Rendering Engine (GraphicsEngine - creates GPU resources)
//! ```
//!
//! ResourceManager does NOT create GPU resources. Instead, it:
//! - Tracks which (mesh_type, materials) combinations have pools
//! - Maintains reference counts for CPU-side assets
//! - Notifies when pools need to be created (via PoolCreationRequest)
//! - Maps entities to their pool handles for cleanup
//!
//! **Ownership**: GraphicsEngine owns ResourceManager, shares with SceneManager
//! via Arc<Mutex<ResourceManager>>. Follows Chapter 7.2: "The resource manager
//! is typically owned by the engine's main loop."

use crate::ecs::Entity;
use crate::foundation::math::Mat4;
use crate::render::primitives::Mesh;
use crate::render::resources::materials::Material;
use crate::render::systems::dynamic::{
    DynamicObjectError, DynamicObjectHandle, MeshPoolManager, MeshType, PoolManagerError,
    SpawnerError,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use thiserror::Error;

/// Resource Manager errors
#[derive(Debug, Error)]
pub enum ResourceError {
    #[error("Pool not found for key: {0:?}")]
    PoolNotFound(PoolKey),

    #[error("Pool exhausted: {0}")]
    PoolExhausted(String),

    #[error("Memory budget exceeded: requested {requested} bytes, available {available} bytes")]
    MemoryBudgetExceeded { requested: usize, available: usize },

    #[error("Max pool size reached: {max_size}")]
    MaxPoolSizeReached { max_size: usize },

    #[error("Allocation failed: {0}")]
    AllocationFailed(String),

    #[error("Spawner error: {0}")]
    SpawnerError(#[from] SpawnerError),

    #[error("Pool manager error: {0}")]
    PoolManagerError(#[from] PoolManagerError),

    #[error("Dynamic object error: {0}")]
    DynamicObjectError(#[from] DynamicObjectError),
}

/// Configuration for Resource Manager
#[derive(Debug, Clone)]
pub struct ResourceConfig {
    /// Initial size for newly created pools
    pub initial_pool_size: usize,

    /// Growth factor when pool needs to expand (e.g., 1.5 = grow by 50%)
    pub growth_factor: f32,

    /// Maximum size a pool can grow to
    pub max_pool_size: usize,

    /// Total memory budget in bytes (0 = unlimited)
    pub memory_budget: usize,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            initial_pool_size: 16,
            growth_factor: 1.5,
            max_pool_size: 1024,
            memory_budget: 512 * 1024 * 1024, // 512 MB
        }
    }
}

/// Pool key for caching - identifies unique mesh type + materials combination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PoolKey {
    /// Mesh type (Teapot, Sphere, etc.)
    pub mesh_type: MeshType,

    /// Hash of materials (for cache lookup)
    pub materials_hash: u64,
}

/// Resource Manager - Central resource tracking and automatic pool management
///
/// **Architecture**: Follows Game Engine Architecture Chapter 7.2
/// - Transparent to game code (applications don't see pools)
/// - Automatic pool creation and caching
/// - Reference counting for meshes and materials
/// - Memory budget enforcement
pub struct ResourceManager {
    /// Cached pools by (mesh_type, materials_hash)
    pool_cache: HashMap<PoolKey, MeshPoolManager>,

    /// Reference counts for meshes (by mesh ID)
    mesh_refs: HashMap<usize, usize>,

    /// Reference counts for materials (by material ID)
    material_refs: HashMap<usize, usize>,

    /// Entity to (PoolKey, Handle) mapping for cleanup
    entity_handles: HashMap<Entity, (PoolKey, DynamicObjectHandle)>,

    /// Configuration
    config: ResourceConfig,

    /// Current memory usage in bytes
    memory_used: usize,
    
    /// Pending pool creation requests (pools registered but not yet created on GPU)
    /// GraphicsEngine reads this to know which pools need GPU resources
    pending_pools: HashMap<PoolKey, (Mesh, Vec<Material>)>,
}

impl ResourceManager {
    /// Create new Resource Manager
    ///
    /// **OWNERSHIP**: GraphicsEngine owns ResourceManager, shares with
    /// SceneManager via Arc<Mutex<ResourceManager>>. This follows
    /// Game Engine Architecture Chapter 7.2 guidance: "The resource
    /// manager is typically owned by the engine's main loop."
    ///
    /// # Arguments
    /// * `config` - Configuration parameters (cache sizes, memory budget)
    pub fn new(config: ResourceConfig) -> Self {
        log::info!("Creating ResourceManager with config: {:?}", config);
        Self {
            pool_cache: HashMap::new(),
            mesh_refs: HashMap::new(),
            material_refs: HashMap::new(),
            entity_handles: HashMap::new(),
            config,
            memory_used: 0,
            pending_pools: HashMap::new(),
        }
    }

    /// Request mesh instance - transparent pooling
    ///
    /// This method automatically creates pools on-demand and manages
    /// reference counting for meshes and materials. Applications never
    /// see pools directly.
    ///
    /// # Arguments
    /// * `entity` - ECS entity requesting the instance
    /// * `mesh_type` - Type of mesh (determines pool)
    /// * `mesh` - Mesh geometry data
    /// * `materials` - Material array (typically length 1)
    /// * `transform` - Initial world transform
    ///
    /// # Returns
    /// * `Ok(Handle)` - Successfully allocated instance
    /// * `Err(ResourceError)` - Allocation failed (budget exceeded, etc.)
    pub fn request_mesh_instance(
        &mut self,
        entity: Entity,
        mesh_type: MeshType,
        mesh: &Mesh,
        materials: &[Material],
        transform: Mat4,
    ) -> Result<DynamicObjectHandle, ResourceError> {
        // 1. Calculate pool key (mesh type + materials hash)
        let materials_hash = self.hash_materials(materials);
        let pool_key = PoolKey {
            mesh_type,
            materials_hash,
        };

        // 2. Get or create pool
        if !self.pool_cache.contains_key(&pool_key) {
            self.create_pool_internal(pool_key, mesh, materials)?;
        }

        // 3. Allocate from pool
        let pool = self
            .pool_cache
            .get_mut(&pool_key)
            .ok_or(ResourceError::PoolNotFound(pool_key))?;

        let handle = pool
            .allocate_instance(mesh_type, transform, materials[0].clone())
            .map_err(|e| ResourceError::PoolExhausted(e.to_string()))?;

        // 4. Track entity→handle mapping
        self.entity_handles.insert(entity, (pool_key, handle));

        // 5. Increment reference counts
        self.increment_refs(mesh, materials);

        log::debug!(
            "Allocated instance for entity {:?} from pool {:?} (handle: {:?})",
            entity,
            pool_key,
            handle
        );
        Ok(handle)
    }

    /// Release mesh instance - automatic cleanup
    ///
    /// Decrements reference counts and frees the instance from its pool.
    /// If reference counts reach zero, resources may be unloaded.
    ///
    /// # Arguments
    /// * `entity` - ECS entity to release
    ///
    /// # Returns
    /// * `Ok(())` - Successfully released
    /// * `Err(ResourceError)` - Release failed (entity not found, etc.)
    pub fn release_mesh_instance(&mut self, entity: Entity) -> Result<(), ResourceError> {
        if let Some((pool_key, handle)) = self.entity_handles.remove(&entity) {
            // Free from pool
            let pool = self
                .pool_cache
                .get_mut(&pool_key)
                .ok_or(ResourceError::PoolNotFound(pool_key))?;

            pool.free_instance(pool_key.mesh_type, handle)?;

            // Decrement reference counts (TODO: implement unloading)
            self.decrement_refs(pool_key)?;

            log::debug!(
                "Released instance for entity {:?} from pool {:?}",
                entity,
                pool_key
            );
        }
        Ok(())
    }

    /// Update instance if transform/material changed (dirty tracking integration)
    ///
    /// **ECS Integration**: Scene Manager calls this during sync_from_world()
    /// when cached RenderableObject is marked dirty. Avoids unnecessary GPU writes.
    ///
    /// # Arguments
    /// * `entity` - Entity to update
    /// * `transform` - New transform matrix
    /// * `material` - New material
    ///
    /// # Returns
    /// * `Ok(())` - Successfully updated
    /// * `Err(ResourceError)` - Update failed
    pub fn update_instance_if_dirty(
        &mut self,
        entity: Entity,
        transform: Mat4,
        material: Material,
    ) -> Result<(), ResourceError> {
        if let Some((pool_key, handle)) = self.entity_handles.get(&entity) {
            let pool = self
                .pool_cache
                .get_mut(pool_key)
                .ok_or(ResourceError::PoolNotFound(*pool_key))?;

            // Update transform and material via existing infrastructure
            pool.update_instance_transform(pool_key.mesh_type, *handle, transform)?;
            pool.update_instance_material(pool_key.mesh_type, *handle, material)?;

            log::trace!("Updated instance for entity {:?} (dirty tracking)", entity);
        }
        Ok(())
    }

    /// Get pool statistics for debugging
    pub fn get_pool_stats(&self, pool_key: PoolKey) -> Option<(usize, usize)> {
        self.pool_cache
            .get(&pool_key)
            .and_then(|pool| pool.get_pool_stats(pool_key.mesh_type))
    }

    /// Get total number of pools
    pub fn pool_count(&self) -> usize {
        self.pool_cache.len()
    }

    /// Get current memory usage
    pub fn memory_used(&self) -> usize {
        self.memory_used
    }

    /// Get memory budget
    pub fn memory_budget(&self) -> usize {
        self.config.memory_budget
    }
    
    /// Get resource configuration
    ///
    /// Returns reference to the configuration used by this ResourceManager.
    /// Useful for GraphicsEngine to know pool sizing parameters.
    pub fn config(&self) -> &ResourceConfig {
        &self.config
    }
    
    /// Get pending pool creation requests
    ///
    /// Returns pools that have been registered but not yet created on GPU.
    /// GraphicsEngine should call this to create GPU resources for these pools.
    ///
    /// # Returns
    /// Iterator of (PoolKey, Mesh, Materials) tuples
    pub fn pending_pools(&self) -> impl Iterator<Item = (&PoolKey, &Mesh, &Vec<Material>)> {
        self.pending_pools.iter().map(|(key, (mesh, mats))| (key, mesh, mats))
    }
    
    /// Mark a pool as created (GPU resources now exist)
    ///
    /// Called by GraphicsEngine after successfully creating GPU pool.
    /// Removes the pool from pending list.
    ///
    /// # Arguments
    /// * `pool_key` - Key identifying the pool that was created
    pub fn mark_pool_created(&mut self, pool_key: PoolKey) {
        if self.pending_pools.remove(&pool_key).is_some() {
            log::info!("Pool {:?} GPU resources created successfully", pool_key);
        }
    }

    // ========================================================================
    // PRIVATE HELPER METHODS
    // ========================================================================

    /// Create pool internally (transparent to caller)
    fn create_pool_internal(
        &mut self,
        pool_key: PoolKey,
        mesh: &Mesh,
        materials: &[Material],
    ) -> Result<(), ResourceError> {
        // Check memory budget
        let estimated_size = self.estimate_pool_size(mesh, self.config.initial_pool_size);
        if self.config.memory_budget > 0
            && self.memory_used + estimated_size > self.config.memory_budget
        {
            return Err(ResourceError::MemoryBudgetExceeded {
                requested: estimated_size,
                available: self.config.memory_budget - self.memory_used,
            });
        }

        // NOTE: This method should NOT create GPU resources!
        // That violates separation of concerns (see module docs).
        // 
        // Instead, register this pool as pending so GraphicsEngine can create it.
        
        // Store mesh and materials data for GPU pool creation
        self.pending_pools.insert(pool_key, (mesh.clone(), materials.to_vec()));
        
        // Create empty pool manager placeholder
        let pool_manager = MeshPoolManager::new();
        self.pool_cache.insert(pool_key, pool_manager);
        self.memory_used += estimated_size;

        log::info!(
            "Registered pending pool {:?} (waiting for GraphicsEngine to create GPU resources)",
            pool_key
        );
        Ok(())
    }

    /// Hash materials for cache key
    fn hash_materials(&self, materials: &[Material]) -> u64 {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        for material in materials {
            material.id.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Increment reference counts for mesh and materials
    fn increment_refs(&mut self, mesh: &Mesh, materials: &[Material]) {
        // Use pointer address as mesh ID (simple but works for Phase 1)
        let mesh_id = mesh as *const Mesh as usize;
        *self.mesh_refs.entry(mesh_id).or_insert(0) += 1;
        
        for material in materials {
            *self.material_refs.entry(material.id.0 as usize).or_insert(0) += 1;
        }

        log::trace!(
            "Incremented refs: mesh={}, materials={}",
            self.mesh_refs.get(&mesh_id).unwrap_or(&0),
            materials.len()
        );
    }

    /// Decrement reference counts, unload if zero
    fn decrement_refs(&mut self, _pool_key: PoolKey) -> Result<(), ResourceError> {
        // TODO: Track mesh/material IDs per pool_key
        // For now, we keep pools alive (cleanup will be manual in Phase 5)
        // Future: Implement automatic unloading when refs reach zero
        Ok(())
    }

    /// Estimate memory size for a pool
    fn estimate_pool_size(&self, mesh: &Mesh, pool_size: usize) -> usize {
        // Rough estimate: vertex data + index data + instance data
        let vertex_size = mesh.vertices.len() * std::mem::size_of::<f32>() * 3; // positions
        let index_size = mesh.indices.len() * std::mem::size_of::<u32>();
        let instance_size = pool_size * 128; // ~128 bytes per instance (transform + material params)

        vertex_size + index_size + instance_size
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;
    use crate::foundation::math::Mat4;

    /// Helper to create a test mesh
    fn create_test_mesh() -> Mesh {
        use crate::render::primitives::Vertex;
        Mesh {
            vertices: vec![Vertex::new([0.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0])],
            indices: vec![0, 1, 2],
        }
    }

    /// Helper to create a test material
    fn create_test_material(_id: usize) -> Material {
        use crate::foundation::math::Vec3;
        use crate::render::resources::materials::UnlitMaterialParams;
        Material::unlit(UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        })
    }

    #[test]
    fn test_resource_manager_creation() {
        let rm = ResourceManager::new(ResourceConfig::default());
        assert_eq!(rm.pool_count(), 0);
        assert_eq!(rm.memory_used(), 0);
    }

    #[test]
    fn test_pool_caching() {
        let mut world = World::new();
        let mut rm = ResourceManager::new(ResourceConfig::default());
        let mesh = create_test_mesh();
        let material = create_test_material(1);

        let entity1 = world.create_entity();
        let entity2 = world.create_entity();

        // Allocate two entities with same mesh+materials
        let result1 = rm.request_mesh_instance(
            entity1,
            MeshType::Teapot,
            &mesh,
            &[material.clone()],
            Mat4::identity(),
        );
        assert!(result1.is_ok());

        let result2 = rm.request_mesh_instance(
            entity2,
            MeshType::Teapot,
            &mesh,
            &[material.clone()],
            Mat4::identity(),
        );
        assert!(result2.is_ok());

        // Should only have created 1 pool (caching works)
        assert_eq!(rm.pool_count(), 1);
    }

    #[test]
    fn test_reference_counting() {
        let mut world = World::new();
        let mut rm = ResourceManager::new(ResourceConfig::default());
        let mesh = create_test_mesh();
        let material = create_test_material(1);
        let entity = world.create_entity();

        // Allocate
        let result = rm.request_mesh_instance(
            entity,
            MeshType::Sphere,
            &mesh,
            &[material.clone()],
            Mat4::identity(),
        );
        assert!(result.is_ok());

        // Check ref counts incremented (use pointer address as ID)
        let mesh_id = &mesh as *const Mesh as usize;
        assert_eq!(rm.mesh_refs.get(&mesh_id), Some(&1));
        assert_eq!(rm.material_refs.get(&(material.id.0 as usize)), Some(&1));

        // Release
        let result = rm.release_mesh_instance(entity);
        assert!(result.is_ok());

        // Entity handle should be removed
        assert!(!rm.entity_handles.contains_key(&entity));
    }

    #[test]
    fn test_memory_budget_enforcement() {
        let mut world = World::new();
        let config = ResourceConfig {
            initial_pool_size: 16,
            memory_budget: 100, // Very small budget
            ..Default::default()
        };
        let mut rm = ResourceManager::new(config);
        let mesh = create_test_mesh();
        let material = create_test_material(1);
        let entity = world.create_entity();

        // Try to allocate - should exceed budget
        let result = rm.request_mesh_instance(
            entity,
            MeshType::Teapot,
            &mesh,
            &[material],
            Mat4::identity(),
        );

        match result {
            Err(ResourceError::MemoryBudgetExceeded { .. }) => {
                // Expected
            }
            _ => panic!("Expected MemoryBudgetExceeded error"),
        }
    }

    #[test]
    fn test_different_mesh_types_create_different_pools() {
        let mut world = World::new();
        let mut rm = ResourceManager::new(ResourceConfig::default());
        let mesh = create_test_mesh();
        let material = create_test_material(1);

        let entity1 = world.create_entity();
        let entity2 = world.create_entity();

        // Allocate with different mesh types
        rm.request_mesh_instance(
            entity1,
            MeshType::Teapot,
            &mesh,
            &[material.clone()],
            Mat4::identity(),
        )
        .unwrap();

        rm.request_mesh_instance(
            entity2,
            MeshType::Sphere,
            &mesh,
            &[material],
            Mat4::identity(),
        )
        .unwrap();

        // Should have 2 pools (different mesh types)
        assert_eq!(rm.pool_count(), 2);
    }

    #[test]
    fn test_hash_materials_same_material() {
        let rm = ResourceManager::new(ResourceConfig::default());
        let material = create_test_material(1);

        let hash1 = rm.hash_materials(&[material.clone()]);
        let hash2 = rm.hash_materials(&[material.clone()]);

        // Same material should produce same hash
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_materials_different_materials() {
        let rm = ResourceManager::new(ResourceConfig::default());
        let material1 = create_test_material(1);
        let material2 = create_test_material(2);

        let hash1 = rm.hash_materials(&[material1]);
        let hash2 = rm.hash_materials(&[material2]);

        // Different materials should (likely) produce different hashes
        // Note: Technically hash collisions are possible, but unlikely
        assert_ne!(hash1, hash2);
    }
}
