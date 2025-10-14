//! Mesh Pool Manager
//!
//! Coordinates multiple single-mesh dynamic object pools for optimal rendering performance.
//! Each mesh type gets its own dedicated pool to enable perfect batching with minimal
//! state changes.

use crate::foundation::math::Vec3;
use crate::render::vulkan::{VulkanContext, VulkanError};
use crate::render::material::Material;
use crate::render::dynamic::{
    MeshType, DynamicObjectManager, DynamicSpawnParams, DynamicObjectHandle, 
    DynamicRenderData, DynamicObjectError, resource_pool::MaterialProperties,
    InstanceRenderer
};
use crate::render::SharedRenderingResources;
use ash::vk;
use std::collections::HashMap;

/// Errors that can occur during mesh pool management
#[derive(Debug)]
pub enum PoolManagerError {
    /// No pool exists for the specified mesh type
    PoolNotFound {
        /// The mesh type that was not found
        mesh_type: MeshType,
    },
    /// Failed to create a new pool
    PoolCreationFailed {
        /// The mesh type for which pool creation failed
        mesh_type: MeshType,
        /// Reason for the failure
        reason: String,
    },
    /// Pool-specific operation failed
    PoolOperationFailed {
        /// The mesh type of the pool that failed
        mesh_type: MeshType,
        /// The operation that was attempted
        operation: String,
        /// Reason for the failure
        reason: String,
    },
    /// Vulkan-related error
    VulkanError(VulkanError),
}

impl std::fmt::Display for PoolManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PoolManagerError::PoolNotFound { mesh_type } => {
                write!(f, "No pool found for mesh type: {}", mesh_type)
            }
            PoolManagerError::PoolCreationFailed { mesh_type, reason } => {
                write!(f, "Failed to create pool for {}: {}", mesh_type, reason)
            }
            PoolManagerError::PoolOperationFailed { mesh_type, operation, reason } => {
                write!(f, "Pool operation '{}' failed for {}: {}", operation, mesh_type, reason)
            }
            PoolManagerError::VulkanError(err) => {
                write!(f, "Vulkan error in pool manager: {}", err)
            }
        }
    }
}

impl std::error::Error for PoolManagerError {}

impl From<VulkanError> for PoolManagerError {
    fn from(err: VulkanError) -> Self {
        PoolManagerError::VulkanError(err)
    }
}

/// Manager for multiple single-mesh dynamic object pools
///
/// The MeshPoolManager coordinates separate DynamicObjectManager instances,
/// one for each mesh type. This architecture enables optimal rendering performance
/// with O(1) draw calls per mesh type and minimal state changes.
///
/// # Architecture
///
/// - Each mesh type (Teapot, Sphere, Cube) gets its own dedicated pool
/// - Each pool manages objects of exactly one mesh type
/// - Rendering batches perfectly within each pool
/// - Minimal state changes when switching between pools
///
/// # Performance Characteristics
///
/// - **Rendering**: O(1) draw call per active mesh type (not per object count)
/// - **Memory**: Bounded at startup with pre-allocated pools
/// - **Allocation**: O(1) spawning/despawning within each pool
/// - **State Changes**: Minimal - only mesh/texture binding between pools
///
/// # Usage
///
/// ```rust
/// let mut pool_manager = MeshPoolManager::new();
/// 
/// // Create pools for each mesh type
/// pool_manager.create_pool(MeshType::Teapot, teapot_resources, 100)?;
/// pool_manager.create_pool(MeshType::Sphere, sphere_resources, 200)?;
/// 
/// // Spawn objects into appropriate pools
/// let teapot = pool_manager.spawn_object(
///     MeshType::Teapot,
///     Vec3::zeros(),
///     Vec3::zeros(), 
///     Vec3::ones(),
///     material,
///     5.0
/// )?;
/// 
/// // Render all active pools efficiently
/// pool_manager.render_all_pools(context, frame_index)?;
/// ```
pub struct MeshPoolManager {
    /// Individual single-mesh pools with their rendering resources and dedicated renderers
    pools: HashMap<MeshType, (DynamicObjectManager, SharedRenderingResources, Option<InstanceRenderer>)>,
    
    /// Statistics for monitoring performance
    stats: PoolManagerStats,
}

/// Statistics for the mesh pool manager
#[derive(Debug, Clone, Default)]
pub struct PoolManagerStats {
    /// Total number of active pools
    pub active_pools: usize,
    /// Total objects across all pools
    pub total_active_objects: usize,
    /// Objects spawned since creation
    pub total_spawned: u64,
    /// Objects despawned since creation
    pub total_despawned: u64,
    /// Number of render batches per frame (should equal number of active pools)
    pub render_batches_per_frame: usize,
}

impl MeshPoolManager {
    /// Create a new mesh pool manager
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            stats: PoolManagerStats::default(),
        }
    }
    
    /// Create a new single-mesh pool for the specified mesh type
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - The mesh type for this pool
    /// * `shared_resources` - Rendering resources specific to this mesh type (stored for later use)
    /// * `max_objects` - Maximum number of objects this pool can hold
    /// * `context` - Vulkan context for InstanceRenderer creation
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Pool created successfully
    /// * `Err(PoolManagerError)` - Pool creation failed
    pub fn create_pool(
        &mut self,
        mesh_type: MeshType,
        shared_resources: SharedRenderingResources,
        max_objects: usize,
        context: &VulkanContext,
    ) -> Result<(), PoolManagerError> {
        if self.pools.contains_key(&mesh_type) {
            return Err(PoolManagerError::PoolCreationFailed {
                mesh_type,
                reason: "Pool already exists for this mesh type".to_string(),
            });
        }
        
        // Create pool with its dedicated SharedRenderingResources and InstanceRenderer
        let mut pool = DynamicObjectManager::new(max_objects);
        
        // Initialize the pool resources
        pool.initialize_resources(context)
            .map_err(|err| PoolManagerError::PoolCreationFailed {
                mesh_type,
                reason: format!("Failed to initialize pool resources: {}", err),
            })?;
            
        // Create the dedicated InstanceRenderer for this pool
        let instance_renderer = InstanceRenderer::new(context, max_objects)
            .map_err(|err| PoolManagerError::PoolCreationFailed {
                mesh_type,
                reason: format!("Failed to create InstanceRenderer: {}", err),
            })?;
            
        self.pools.insert(mesh_type, (pool, shared_resources, Some(instance_renderer)));
        self.stats.active_pools = self.pools.len();
        
        log::info!("Created single-mesh pool for {} with capacity {} and dedicated InstanceRenderer", mesh_type, max_objects);
        Ok(())
    }
    
    /// Initialize all pools with Vulkan resources (now mostly a no-op since pools self-initialize)
    ///
    /// This method now mainly serves as a compatibility function since pools 
    /// initialize themselves when created.
    ///
    /// # Arguments
    ///
    /// * `context` - Vulkan context for resource initialization
    ///
    /// # Returns
    ///
    /// * `Ok(())` - All pools initialized successfully
    /// * `Err(PoolManagerError)` - Pool initialization failed
    pub fn initialize_pools(&mut self, _context: &VulkanContext) -> Result<(), PoolManagerError> {
        // Since pools now initialize themselves when created, this is mostly a no-op
        // But we can validate that all existing pools have their renderers
        let mut missing_renderers = Vec::new();
        
        for (mesh_type, (pool, _shared_resources, instance_renderer_opt)) in &self.pools {
            if pool.capacity() > 0 && instance_renderer_opt.is_none() {
                missing_renderers.push(*mesh_type);
            }
        }
        
        if !missing_renderers.is_empty() {
            return Err(PoolManagerError::PoolOperationFailed {
                mesh_type: missing_renderers[0],
                operation: "validate_renderers".to_string(),
                reason: format!("Pools missing InstanceRenderers: {:?}", missing_renderers),
            });
        }
        
        log::info!("Validated {} mesh pools - all have dedicated renderers", self.pools.len());
        Ok(())
    }
    
    /// Spawn a dynamic object in the appropriate mesh pool
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - Which mesh type pool to spawn into
    /// * `position` - Initial world position
    /// * `rotation` - Initial rotation (Euler angles)
    /// * `scale` - Initial scale
    /// * `material` - Material properties
    /// * `lifetime` - How long the object should live (seconds)
    ///
    /// # Returns
    ///
    /// * `Ok(DynamicObjectHandle)` - Handle to the spawned object
    /// * `Err(PoolManagerError)` - Spawning failed
    pub fn spawn_object(
        &mut self,
        mesh_type: MeshType,
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
        material: Material,
    ) -> Result<DynamicObjectHandle, PoolManagerError> {
        let (pool, _shared_resources, _renderer) = self.pools.get_mut(&mesh_type)
            .ok_or(PoolManagerError::PoolNotFound { mesh_type })?;
        
        let spawn_params = DynamicSpawnParams::new(
            position,
            rotation,
            scale,
            material,
        );
        
        let handle = pool.spawn_object(spawn_params)
            .map_err(|err| PoolManagerError::PoolOperationFailed {
                mesh_type,
                operation: "spawn_object".to_string(),
                reason: err.to_string(),
            })?;
        
        // Update statistics
        self.stats.total_spawned += 1;
        self.update_stats();
        
        Ok(handle)
    }
    
    /// Despawn a dynamic object from its pool
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - Which mesh type pool the object belongs to
    /// * `handle` - Handle to the object to despawn
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Object despawned successfully
    /// * `Err(PoolManagerError)` - Despawning failed
    pub fn despawn_object(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
    ) -> Result<(), PoolManagerError> {
        let (pool, _shared_resources, _renderer) = self.pools.get_mut(&mesh_type)
            .ok_or(PoolManagerError::PoolNotFound { mesh_type })?;
        
        pool.despawn_object(handle)
            .map_err(|err| PoolManagerError::PoolOperationFailed {
                mesh_type,
                operation: "despawn_object".to_string(),
                reason: err.to_string(),
            })?;
        
        // Update statistics
        self.stats.total_despawned += 1;
        self.update_stats();
        
        Ok(())
    }
    
    /// Get object information from any pool
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - Which mesh type pool to search
    /// * `handle` - Handle to the object
    ///
    /// # Returns
    ///
    /// * `Some(&DynamicRenderData)` - Object data found
    /// * `None` - Object not found
    pub fn get_object(
        &self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
    ) -> Option<&DynamicRenderData> {
        self.pools.get(&mesh_type)
            .and_then(|(pool, _shared_resources, _renderer)| pool.get_object(handle).ok())
    }
    
    /// Update all pools (handle object lifecycles)
    ///
    /// This should be called once per frame to handle automatic despawning
    /// of objects that have exceeded their lifetime.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last update (seconds)
    pub fn update_all_pools(&mut self, delta_time: f32) {
        for (pool, _shared_resources, _renderer) in self.pools.values_mut() {
            pool.update(delta_time);
        }
        
        self.update_stats();
    }
    
    /// Get active objects from a specific pool
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - Which mesh type pool to query
    ///
    /// # Returns
    ///
    /// * `Some(HashMap)` - Active objects in the pool
    /// * `None` - Pool not found
    pub fn get_active_objects(
        &self,
        mesh_type: MeshType,
    ) -> Option<std::collections::HashMap<DynamicObjectHandle, DynamicRenderData>> {
        self.pools.get(&mesh_type)
            .map(|(pool, _shared_resources, _renderer)| pool.get_active_objects_map())
    }
    
    /// Get all active pools with their mesh types and resources
    ///
    /// Returns an iterator over (MeshType, &DynamicObjectManager, &SharedRenderingResources) tuples
    /// for pools that have active objects.
    pub fn active_pools_with_resources(&self) -> impl Iterator<Item = (MeshType, &DynamicObjectManager, &SharedRenderingResources)> {
        self.pools.iter()
            .filter(|(_, (pool, _, _))| pool.active_count() > 0)
            .map(|(mesh_type, (pool, resources, _))| (*mesh_type, pool, resources))
    }

    /// Render all active pools using backend interface (with dedicated renderers)
    /// 
    /// This method provides the same interface as the old backend.record_dynamic_draws
    /// but uses each pool's dedicated InstanceRenderer to prevent state corruption.
    pub fn render_all_pools_via_backend(
        &mut self, 
        backend: &mut dyn crate::render::backend::RenderBackend
    ) -> Result<(), String> {
        log::debug!("Rendering all active pools via backend with dedicated renderers");
        
        // Track total objects rendered for performance monitoring
        let mut total_objects = 0;
        let mut pools_rendered = 0;
        
        // Cast to VulkanRenderer to access the dedicated renderer method
        if let Some(vulkan_renderer) = backend.as_any_mut().downcast_mut::<crate::render::vulkan::renderer::VulkanRenderer>() {
            // Render each active pool separately using dedicated renderers
            for (mesh_type, (pool, shared_resources, instance_renderer_opt)) in &mut self.pools {
                if pool.active_count() > 0 {
                    if let Some(instance_renderer) = instance_renderer_opt {
                        let active_objects = pool.get_active_objects_map();
                        log::debug!("Rendering {} objects of type {:?} with dedicated renderer", active_objects.len(), mesh_type);
                        
                        // Use the VulkanRenderer's new dedicated method
                        vulkan_renderer.record_dynamic_draws_with_dedicated_renderer(instance_renderer, &active_objects, shared_resources)
                            .map_err(|e| format!("Failed to record draws with dedicated renderer for {:?}: {}", mesh_type, e))?;
                        
                        total_objects += active_objects.len();
                        pools_rendered += 1;
                    } else {
                        log::warn!("Pool for {:?} has active objects but no InstanceRenderer", mesh_type);
                    }
                }
            }
        } else {
            return Err("Backend is not a VulkanRenderer - dedicated renderer method not available".to_string());
        }
        
        log::debug!("Rendered {} total objects across {} mesh type pools via dedicated renderers", total_objects, pools_rendered);
        Ok(())
    }

    /// Render all active pools with proper state management (low-level Vulkan interface)
    /// 
    /// This method ensures each mesh type is rendered correctly without state corruption
    /// by using each pool's dedicated InstanceRenderer directly.
    pub fn render_all_pools_direct(
        &mut self,
        command_buffer: vk::CommandBuffer,
        frame_index: usize,
        vulkan_context: &VulkanContext,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet
    ) -> Result<(), String> {
        log::debug!("Rendering all active pools with dedicated renderers (direct mode)");
        
        // Track total objects rendered for performance monitoring
        let mut total_objects = 0;
        let mut pools_rendered = 0;
        
        // Render each active pool separately using its dedicated InstanceRenderer
        for (mesh_type, (pool, shared_resources, instance_renderer_opt)) in &mut self.pools {
            if pool.active_count() > 0 {
                if let Some(instance_renderer) = instance_renderer_opt {
                    let active_objects = pool.get_active_objects_map();
                    log::debug!("Rendering {} objects of type {:?} with dedicated renderer", active_objects.len(), mesh_type);
                    
                    // Use the pool's dedicated InstanceRenderer following the same pattern as VulkanRenderer
                    instance_renderer.set_active_command_buffer(command_buffer, frame_index);
                    
                    // Upload instance data from active dynamic objects
                    instance_renderer.upload_instance_data(&active_objects)
                        .map_err(|e| format!("Failed to upload instance data for {:?}: {}", mesh_type, e))?;
                    
                    // Record instanced draw commands
                    instance_renderer.record_instanced_draw(shared_resources, vulkan_context, frame_descriptor_set, material_descriptor_set)
                        .map_err(|e| format!("Failed to record draw commands for {:?}: {}", mesh_type, e))?;
                    
                    total_objects += active_objects.len();
                    pools_rendered += 1;
                } else {
                    log::warn!("Pool for {:?} has active objects but no InstanceRenderer", mesh_type);
                }
            }
        }
        
        log::debug!("Rendered {} total objects across {} mesh type pools using dedicated renderers", total_objects, pools_rendered);
        Ok(())
    }
    
    /// Get all active pools with their mesh types (legacy compatibility)
    ///
    /// Returns an iterator over (MeshType, &DynamicObjectManager) pairs
    /// for pools that have active objects.
    pub fn active_pools(&self) -> impl Iterator<Item = (MeshType, &DynamicObjectManager)> {
        self.pools.iter()
            .filter(|(_, (pool, _, _))| pool.active_count() > 0)
            .map(|(mesh_type, (pool, _, _))| (*mesh_type, pool))
    }
    
    /// Get mesh pool manager statistics
    pub fn get_stats(&self) -> PoolManagerStats {
        self.stats.clone()
    }
    
    /// Get pool-specific statistics
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - Which mesh type pool to query
    ///
    /// # Returns
    ///
    /// * `Some(PoolStats)` - Pool statistics
    /// * `None` - Pool not found
    pub fn get_pool_stats(&self, mesh_type: MeshType) -> Option<(usize, usize)> {
        self.pools.get(&mesh_type)
            .map(|(pool, _shared_resources, _renderer)| (pool.active_count(), pool.capacity()))
    }
    
    /// Get total number of active objects across all pools
    pub fn total_active_objects(&self) -> usize {
        self.pools.values()
            .map(|(pool, _shared_resources, _renderer)| pool.active_count())
            .sum()
    }
    
    /// Check if any pools exist
    pub fn has_pools(&self) -> bool {
        !self.pools.is_empty()
    }
    
    /// Check if a specific pool exists
    pub fn has_pool(&self, mesh_type: MeshType) -> bool {
        self.pools.contains_key(&mesh_type)
    }
    
    /// Update internal statistics
    fn update_stats(&mut self) {
        self.stats.active_pools = self.pools.len();
        self.stats.total_active_objects = self.total_active_objects();
        self.stats.render_batches_per_frame = self.pools.iter()
            .filter(|(_, (pool, _, _))| pool.active_count() > 0)
            .count();
    }
    
    /// Update the transform of a specific dynamic object
    ///
    /// Updates the position, rotation, and scale of an object identified by its handle.
    /// The mesh type is used to locate the correct pool.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh (determines which pool to search)
    /// * `handle` - Unique handle identifying the object
    /// * `position` - New position in world coordinates
    /// * `rotation` - New rotation angles in radians (x, y, z)
    /// * `scale` - New scale factor for each axis
    ///
    /// # Returns
    /// Result indicating success or failure with specific error details
    pub fn update_object_transform(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        position: Vec3,
        rotation: Vec3,
        scale: Vec3,
    ) -> Result<(), DynamicObjectError> {
        if let Some((pool, _shared_resources, _renderer)) = self.pools.get_mut(&mesh_type) {
            pool.update_object_transform(handle, position, rotation, scale)
        } else {
            Err(DynamicObjectError::ResourceCreationFailed(
                format!("No pool found for mesh type: {:?}", mesh_type)
            ))
        }
    }

    /// Update only the position of a dynamic object, preserving rotation and scale.
    ///
    /// The mesh type is used to locate the correct pool.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh (determines which pool to search)
    /// * `handle` - Unique handle identifying the object
    /// * `position` - New position in world coordinates
    ///
    /// # Returns
    /// Result indicating success or failure with specific error details
    pub fn update_object_position(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        position: Vec3,
    ) -> Result<(), DynamicObjectError> {
        if let Some((pool, _shared_resources, _renderer)) = self.pools.get_mut(&mesh_type) {
            pool.update_object_position(handle, position)
        } else {
            Err(DynamicObjectError::ResourceCreationFailed(
                format!("No pool found for mesh type: {:?}", mesh_type)
            ))
        }
    }
}

impl Default for MeshPoolManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to create a validated spawn request
///
/// This converts the material properties format from the spawner API
/// to the format expected by DynamicSpawnParams.
pub fn create_spawn_params(
    position: Vec3,
    rotation: Vec3,
    scale: Vec3,
    material_props: MaterialProperties,
) -> DynamicSpawnParams {
    let material = Material::standard_pbr(crate::render::material::StandardMaterialParams {
        base_color: Vec3::new(
            material_props.base_color[0],
            material_props.base_color[1],
            material_props.base_color[2]
        ),
        alpha: material_props.base_color[3],
        metallic: material_props.metallic,
        roughness: material_props.roughness,
        emission: Vec3::new(
            material_props.emission[0],
            material_props.emission[1],
            material_props.emission[2]
        ),
        ..Default::default()
    });
    
    DynamicSpawnParams::new(
        position,
        rotation,
        scale,
        material,
    )
}