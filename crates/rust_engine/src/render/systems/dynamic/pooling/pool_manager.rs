//! Mesh Pool Manager
//!
//! Coordinates multiple single-mesh dynamic object pools for optimal rendering performance.
//! Each mesh type gets its own dedicated pool to enable perfect batching with minimal
//! state changes.

use crate::foundation::math::{Vec3, Mat4};
use crate::render::backends::vulkan::{VulkanContext, VulkanError};
use crate::render::resources::materials::{Material, TextureHandle};
use crate::render::systems::dynamic::{
    MeshType, DynamicObjectManager, DynamicSpawnParams, DynamicObjectHandle, 
    DynamicRenderData, DynamicObjectError, resource_pool::MaterialProperties,
    InstanceRenderer
};
use crate::render::SharedRenderingResources;
use ash::vk;
use std::collections::HashMap;

/// Metadata for a transparent object (used for global depth sorting)
struct TransparentObjectInfo {
    mesh_type: MeshType,
    handle: DynamicObjectHandle,
    render_data: DynamicRenderData,
    depth: f32,
    render_layer: u8,
}

/// Resources associated with a single mesh pool
///
/// Each mesh pool has its own set of rendering resources including
/// a dedicated descriptor set with texture binding.
pub struct PoolResources {
    /// Dynamic object manager for this mesh type
    pub manager: DynamicObjectManager,
    /// Shared rendering resources (vertex/index buffers, pipeline)
    pub shared_resources: SharedRenderingResources,
    /// Dedicated instance renderer for this pool
    pub instance_renderer: Option<InstanceRenderer>,
    /// Material descriptor set with this pool's texture binding
    pub material_descriptor_set: vk::DescriptorSet,
    /// Texture handle for this pool (all instances share this texture)
    pub texture_handle: Option<TextureHandle>,
}

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
    /// Individual single-mesh pools with their rendering resources
    pools: HashMap<MeshType, PoolResources>,
    
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
    /// * `material_descriptor_set` - Descriptor set with texture binding for this pool
    /// * `texture_handle` - Optional texture handle for this pool
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
        material_descriptor_set: vk::DescriptorSet,
        texture_handle: Option<TextureHandle>,
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
            
        let pool_resources = PoolResources {
            manager: pool,
            shared_resources,
            instance_renderer: Some(instance_renderer),
            material_descriptor_set,
            texture_handle,
        };
        
        self.pools.insert(mesh_type, pool_resources);
        self.stats.active_pools = self.pools.len();
        
        log::info!("Created single-mesh pool for {} with capacity {}, descriptor set {:?}, texture {:?}", 
                   mesh_type, max_objects, material_descriptor_set, texture_handle);
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
        
        for (mesh_type, pool_resources) in &self.pools {
            if pool_resources.manager.capacity() > 0 && pool_resources.instance_renderer.is_none() {
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
    /// Allocate an instance from the pool with a pre-computed transform
    ///
    /// # Arguments
    ///
    /// * `mesh_type` - Which mesh type pool to allocate from
    /// * `transform` - Pre-computed model-to-world transformation matrix
    /// * `material` - Material properties for rendering
    ///
    /// # Returns
    ///
    /// * `Ok(DynamicObjectHandle)` - Handle to the allocated instance
    /// * `Err(PoolManagerError)` - Allocation failed
    pub fn allocate_instance(
        &mut self,
        mesh_type: MeshType,
        transform: Mat4,
        material: Material,
    ) -> Result<DynamicObjectHandle, PoolManagerError> {
        let pool_resources = self.pools.get_mut(&mesh_type)
            .ok_or(PoolManagerError::PoolNotFound { mesh_type })?;
        
        let spawn_params = DynamicSpawnParams::from_matrix(
            transform,
            material,
        );
        
        let handle = pool_resources.manager.spawn_object(spawn_params)
            .map_err(|err| PoolManagerError::PoolOperationFailed {
                mesh_type,
                operation: "allocate_instance".to_string(),
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
    pub fn free_instance(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
    ) -> Result<(), PoolManagerError> {
        let pool_resources = self.pools.get_mut(&mesh_type)
            .ok_or(PoolManagerError::PoolNotFound { mesh_type })?;
        
        pool_resources.manager.despawn_object(handle)
            .map_err(|err| PoolManagerError::PoolOperationFailed {
                mesh_type,
                operation: "free_instance".to_string(),
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
            .and_then(|pool_resources| pool_resources.manager.get_object(handle).ok())
    }
    
    /// Update all pools (handle object lifecycles)
    ///
    /// This should be called once per frame to handle automatic despawning
    /// of objects that have exceeded their lifetime.
    ///
    /// # Arguments
    ///
    /// * `delta_time` - Time elapsed since last update (seconds)
    /// Update all pools - processes cleanup in each pool
    ///
    /// This method updates each pool to process cleanup of manually despawned objects.
    /// Frame timing is not needed - this just manages GPU resources.
    pub fn update_all_pools(&mut self) {
        for pool_resources in self.pools.values_mut() {
            pool_resources.manager.update();
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
            .map(|pool_resources| pool_resources.manager.get_active_objects_map())
    }
    
    /// Get all active pools with their mesh types and resources
    ///
    /// Returns an iterator over (MeshType, &DynamicObjectManager, &SharedRenderingResources) tuples
    /// for pools that have active objects.
    pub fn active_pools_with_resources(&self) -> impl Iterator<Item = (MeshType, &DynamicObjectManager, &SharedRenderingResources)> {
        self.pools.iter()
            .filter(|(_, pool_resources)| pool_resources.manager.active_count() > 0)
            .map(|(mesh_type, pool_resources)| (*mesh_type, &pool_resources.manager, &pool_resources.shared_resources))
    }

    /// Render all active pools using backend interface (with dedicated renderers)
    /// 
    /// This method provides the same interface as the old backend.record_dynamic_draws
    /// but uses each pool's dedicated InstanceRenderer to prevent state corruption.
    pub fn render_all_pools_via_backend(
        &mut self, 
        backend: &mut dyn crate::render::api::RenderBackend,
        camera_position: crate::foundation::math::Vec3,
        camera_target: crate::foundation::math::Vec3
    ) -> Result<(), String> {
        log::debug!("Rendering all active pools via backend with global transparent sorting");
        
        // Cast to VulkanRenderer to access split-phase rendering methods
        let vulkan_renderer = backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>()
            .ok_or("Backend is not a VulkanRenderer")?;
        
        // ========== PHASE 1: UPLOAD ALL POOLS (ONCE PER POOL) ==========
        // Upload instance data and build index maps for each pool
        let mut pool_indices: HashMap<MeshType, HashMap<DynamicObjectHandle, u32>> = HashMap::new();
        
        for (mesh_type, pool_resources) in &mut self.pools {
            if pool_resources.manager.active_count() > 0 {
                if let Some(instance_renderer) = &mut pool_resources.instance_renderer {
                    let active_objects = pool_resources.manager.get_active_objects_map();
                    
                    log::debug!("Uploading {} objects for {:?} pool", active_objects.len(), mesh_type);
                    
                    let indices = vulkan_renderer.upload_pool_instance_data(
                        instance_renderer,
                        &active_objects
                    ).map_err(|e| format!("Failed to upload data for {:?}: {}", mesh_type, e))?;
                    
                    pool_indices.insert(*mesh_type, indices);
                }
            }
        }
        
        // ========== PHASE 2: COLLECT AND CATEGORIZE OBJECTS ==========
        let camera_forward = (camera_target - camera_position).normalize();
        let mut opaque_by_pool: HashMap<MeshType, Vec<(DynamicObjectHandle, DynamicRenderData)>> = HashMap::new();
        let mut transparent_objects: Vec<TransparentObjectInfo> = Vec::new();
        let mut skybox_by_pool: HashMap<MeshType, Vec<(DynamicObjectHandle, DynamicRenderData)>> = HashMap::new();
        
        for (mesh_type, pool_resources) in &self.pools {
            if pool_resources.manager.active_count() > 0 {
                let active_objects = pool_resources.manager.get_active_objects_map();
                
                for (handle, render_data) in active_objects {
                    let pipeline_type = render_data.material.required_pipeline();
                    
                    // Check if skybox first
                    let is_skybox = matches!(
                        pipeline_type,
                        crate::render::resources::materials::PipelineType::Skybox
                    );
                    
                    if is_skybox {
                        // Skybox object - render last
                        skybox_by_pool.entry(*mesh_type)
                            .or_insert_with(Vec::new)
                            .push((handle, render_data.clone()));
                    } else {
                        let is_transparent = matches!(
                            pipeline_type,
                            crate::render::resources::materials::PipelineType::TransparentPBR | 
                            crate::render::resources::materials::PipelineType::TransparentUnlit
                        );
                        
                        if is_transparent {
                            // Calculate view-space depth for sorting
                            // Extract position from transform matrix
                            let position = render_data.transform.column(3).xyz();
                            let to_object = position - camera_position;
                            let depth = to_object.dot(&camera_forward);
                            
                            transparent_objects.push(TransparentObjectInfo {
                                mesh_type: *mesh_type,
                                handle,
                                render_data: render_data.clone(),
                                depth,
                                render_layer: render_data.render_layer,
                            });
                        } else {
                            // Opaque object
                            opaque_by_pool.entry(*mesh_type)
                                .or_insert_with(Vec::new)
                                .push((handle, render_data.clone()));
                        }
                    }
                }
            }
        }
        
        // ========== PHASE 3: RENDER OPAQUE OBJECTS (BATCHED BY POOL) ==========
        for (mesh_type, objects) in opaque_by_pool {
            if let Some(pool_resources) = self.pools.get_mut(&mesh_type) {
                if let Some(instance_renderer) = &mut pool_resources.instance_renderer {
                    if let Some(indices) = pool_indices.get(&mesh_type) {
                        log::debug!("Rendering {} opaque objects from {:?} pool", objects.len(), mesh_type);
                        
                        vulkan_renderer.render_uploaded_objects_subset(
                            instance_renderer,
                            &pool_resources.shared_resources,
                            pool_resources.material_descriptor_set,
                            &objects,
                            indices
                        ).map_err(|e| format!("Failed to render opaque {:?}: {}", mesh_type, e))?;
                    }
                }
            }
        }
        
        // ========== PHASE 4: RENDER SKYBOX OBJECTS (BEFORE TRANSPARENT) ==========
        // Skybox renders after opaque geometry but before transparent objects
        // Uses depth test (LESS_OR_EQUAL) but no depth write to fill background pixels
        // This ensures transparent objects in front of skybox are not occluded
        for (mesh_type, objects) in skybox_by_pool {
            if let Some(pool_resources) = self.pools.get_mut(&mesh_type) {
                if let Some(instance_renderer) = &mut pool_resources.instance_renderer {
                    if let Some(indices) = pool_indices.get(&mesh_type) {
                        log::debug!("Rendering {} skybox objects from {:?} pool (depth test on, write off)", 
                                   objects.len(), mesh_type);
                        
                        vulkan_renderer.render_uploaded_objects_subset(
                            instance_renderer,
                            &pool_resources.shared_resources,
                            pool_resources.material_descriptor_set,
                            &objects,
                            indices
                        ).map_err(|e| format!("Failed to render skybox {:?}: {}", mesh_type, e))?;
                    }
                }
            }
        }
        
        // ========== PHASE 5: RENDER TRANSPARENT OBJECTS (GLOBALLY SORTED, LAST) ==========
        if !transparent_objects.is_empty() {
            // Sort by render_layer first (higher = renders later), then by depth (back-to-front)
            // This ensures skyboxes (layer 255) render LAST regardless of depth
            transparent_objects.sort_by(|a, b| {
                // First compare by render layer (ascending - lower layers first)
                match a.render_layer.cmp(&b.render_layer) {
                    std::cmp::Ordering::Equal => {
                        // Same layer: sort by depth (back-to-front for alpha blending)
                        b.depth.partial_cmp(&a.depth).unwrap_or(std::cmp::Ordering::Equal)
                    }
                    other => other,
                }
            });
            
            log::debug!("Sorted {} transparent objects by layer+depth", transparent_objects.len());
            
            // Batch consecutive objects from same pool for efficiency
            let mut current_batch_type = transparent_objects[0].mesh_type;
            let mut current_batch: Vec<(DynamicObjectHandle, DynamicRenderData)> = Vec::new();
            
            for obj in transparent_objects {
                if obj.mesh_type != current_batch_type {
                    // Pool changed - render current batch
                    if let Some(pool_resources) = self.pools.get_mut(&current_batch_type) {
                        if let Some(instance_renderer) = &mut pool_resources.instance_renderer {
                            if let Some(indices) = pool_indices.get(&current_batch_type) {
                                log::debug!("Rendering {} transparent objects from {:?} pool", 
                                           current_batch.len(), current_batch_type);
                                
                                vulkan_renderer.render_uploaded_objects_subset(
                                    instance_renderer,
                                    &pool_resources.shared_resources,
                                    pool_resources.material_descriptor_set,
                                    &current_batch,
                                    indices
                                ).map_err(|e| format!("Failed to render transparent {:?}: {}", current_batch_type, e))?;
                            }
                        }
                    }
                    
                    // Start new batch
                    current_batch.clear();
                    current_batch_type = obj.mesh_type;
                }
                
                current_batch.push((obj.handle, obj.render_data));
            }
            
            // Render final batch
            if !current_batch.is_empty() {
                if let Some(pool_resources) = self.pools.get_mut(&current_batch_type) {
                    if let Some(instance_renderer) = &mut pool_resources.instance_renderer {
                        if let Some(indices) = pool_indices.get(&current_batch_type) {
                            log::debug!("Rendering {} transparent objects from {:?} pool (final batch)", 
                                       current_batch.len(), current_batch_type);
                            
                            vulkan_renderer.render_uploaded_objects_subset(
                                instance_renderer,
                                &pool_resources.shared_resources,
                                pool_resources.material_descriptor_set,
                                &current_batch,
                                indices
                            ).map_err(|e| format!("Failed to render transparent {:?}: {}", current_batch_type, e))?;
                        }
                    }
                }
            }
        }
        
        log::debug!("Completed rendering with global transparent sorting");
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
        for (mesh_type, pool_resources) in &mut self.pools {
            if pool_resources.manager.active_count() > 0 {
                if let Some(instance_renderer) = &mut pool_resources.instance_renderer {
                    let active_objects = pool_resources.manager.get_active_objects_map();
                    log::debug!("Rendering {} objects of type {:?} with dedicated renderer", active_objects.len(), mesh_type);
                    
                    // Use the pool's dedicated InstanceRenderer following the same pattern as VulkanRenderer
                    instance_renderer.set_active_command_buffer(command_buffer, frame_index);
                    
                    // Upload instance data from active dynamic objects
                    instance_renderer.upload_instance_data(&active_objects)
                        .map_err(|e| format!("Failed to upload instance data for {:?}: {}", mesh_type, e))?;
                    
                    // Record instanced draw commands
                    instance_renderer.record_instanced_draw(&pool_resources.shared_resources, vulkan_context, frame_descriptor_set, material_descriptor_set)
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
            .filter(|(_, pool_resources)| pool_resources.manager.active_count() > 0)
            .map(|(mesh_type, pool_resources)| (*mesh_type, &pool_resources.manager))
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
            .map(|pool_resources| (pool_resources.manager.active_count(), pool_resources.manager.capacity()))
    }
    
    /// Get total number of active objects across all pools
    pub fn total_active_objects(&self) -> usize {
        self.pools.values()
            .map(|pool_resources| pool_resources.manager.active_count())
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
            .filter(|(_, pool_resources)| pool_resources.manager.active_count() > 0)
            .count();
    }
    
    /// Update the transform of a specific dynamic object
    ///
    /// Updates the transform matrix of an object identified by its handle.
    /// The mesh type is used to locate the correct pool.
    /// Application is responsible for computing the transform matrix from components.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh (determines which pool to search)
    /// * `handle` - Unique handle identifying the object
    /// * `transform` - New transform matrix in world coordinates
    ///
    /// # Returns
    /// Result indicating success or failure with specific error details
    pub fn update_instance_transform(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        transform: Mat4,
    ) -> Result<(), DynamicObjectError> {
        if let Some(pool_resources) = self.pools.get_mut(&mesh_type) {
            pool_resources.manager.update_instance_transform(handle, transform)
        } else {
            Err(DynamicObjectError::ResourceCreationFailed(
                format!("No pool found for mesh type: {:?}", mesh_type)
            ))
        }
    }

    /// Update the material properties of a specific dynamic object
    ///
    /// Updates the material of an object identified by its handle.
    /// The mesh type is used to locate the correct pool.
    /// Material changes are automatically reflected in the next frame.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh (determines which pool to search)
    /// * `handle` - Unique handle identifying the object
    /// * `material` - New material with updated properties
    ///
    /// # Returns
    /// Result indicating success or failure with specific error details
    pub fn update_instance_material(
        &mut self,
        mesh_type: MeshType,
        handle: DynamicObjectHandle,
        material: Material,
    ) -> Result<(), DynamicObjectError> {
        if let Some(pool_resources) = self.pools.get_mut(&mesh_type) {
            pool_resources.manager.update_instance_material(handle, material)
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
    let material = Material::standard_pbr(crate::render::resources::materials::StandardMaterialParams {
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
    
    // Build transform matrix from components
    let transform = Mat4::new_translation(&position)
        * Mat4::from_euler_angles(rotation.x, rotation.y, rotation.z)
        * Mat4::new_nonuniform_scaling(&scale);
    
    DynamicSpawnParams::from_matrix(transform, material)
}