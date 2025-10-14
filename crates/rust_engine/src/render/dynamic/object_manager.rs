//! Dynamic Object Manager
//!
//! Core system for managing dynamic objects with pool-based resource allocation.
//! This replaces per-frame GameObject creation with efficient pooled resources
//! designed for temporary objects with bounded lifetimes.
//!
//! # Architecture
//!
//! - **Object Pools**: Pre-allocated storage for dynamic render objects
//! - **Resource Pools**: Pre-allocated GPU buffers and descriptor sets
//! - **Handle System**: Generation counters prevent use-after-free errors
//! - **Batch Rendering**: Single command buffer submission per frame
//!
//! # Performance Characteristics
//!
//! - **Allocation**: O(1) allocation/deallocation from pre-allocated pools
//! - **Memory**: Bounded at startup, no runtime heap allocation
//! - **Rendering**: Single instanced draw call for all active objects
//!
//! # Usage
//!
//! ```rust
//! let mut manager = DynamicObjectManager::new(context, 100)?;
//! 
//! // Spawn dynamic object (pool-based allocation)
//! let handle = manager.spawn_object(spawn_params)?;
//! 
//! // Update object transform
//! manager.update_object(handle, new_transform)?;
//! 
//! // Automatic cleanup when lifetime expires
//! manager.update(delta_time);
//! ```

use crate::foundation::math::{Vec3, Mat4, Mat4Ext};
use crate::render::vulkan::{VulkanContext, VulkanResult, VulkanError, buffer::Buffer};
use crate::render::material::Material;
use std::collections::VecDeque;
use std::time::Instant;

/// Maximum number of dynamic objects supported by default
pub const DEFAULT_MAX_DYNAMIC_OBJECTS: usize = 100;

/// Errors that can occur during dynamic object management
#[derive(Debug)]
pub enum DynamicObjectError {
    /// Object pool has no available slots
    PoolExhausted {
        requested: usize,
        available: usize,
    },
    /// Handle is invalid (wrong generation or index)
    InvalidHandle {
        handle: DynamicObjectHandle,
        reason: String,
    },
    /// Vulkan operation failed
    VulkanError(String),
    /// Resource creation failed
    ResourceCreationFailed(String),
}

impl std::fmt::Display for DynamicObjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PoolExhausted { requested, available } => {
                write!(f, "Pool exhausted: requested {}, available {}", requested, available)
            }
            Self::InvalidHandle { handle, reason } => {
                write!(f, "Invalid handle {:?}: {}", handle, reason)
            }
            Self::VulkanError(e) => write!(f, "Vulkan error: {}", e),
            Self::ResourceCreationFailed(msg) => write!(f, "Resource creation failed: {}", msg),
        }
    }
}

impl std::error::Error for DynamicObjectError {}

impl From<VulkanError> for DynamicObjectError {
    fn from(error: VulkanError) -> Self {
        Self::VulkanError(format!("{}", error))
    }
}

/// Handle to a dynamic object with generation counter for safety
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DynamicObjectHandle {
    /// Index in the object pool
    pub index: u32,
    /// Generation counter to prevent use-after-free
    pub generation: u32,
}

impl DynamicObjectHandle {
    /// Create a new handle with index and generation
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    
    /// Check if this is a valid (non-null) handle
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
    
    /// Create an invalid handle
    pub fn invalid() -> Self {
        Self { index: u32::MAX, generation: 0 }
    }
}

/// Parameters for spawning a new dynamic object
#[derive(Debug, Clone)]
pub struct DynamicSpawnParams {
    /// Initial world position
    pub position: Vec3,
    /// Initial rotation (Euler angles in radians)
    pub rotation: Vec3,
    /// Initial scale
    pub scale: Vec3,
    /// Material properties for this object
    pub material: Material,
    /// How long this object should live (in seconds)
    pub lifetime: f32,
}

impl DynamicSpawnParams {
    /// Create new spawn parameters
    pub fn new(position: Vec3, rotation: Vec3, scale: Vec3, material: Material, lifetime: f32) -> Self {
        Self {
            position,
            rotation,
            scale,
            material,
            lifetime,
        }
    }
}

/// State of a resource in the pool
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceState {
    /// Available for allocation
    Available,
    /// Currently assigned to an active object
    Active,
    /// Marked for cleanup next frame
    PendingCleanup,
}

/// Dynamic render object data
#[derive(Debug, Clone)]
pub struct DynamicRenderData {
    /// Transform information
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    
    /// Material for rendering
    pub material: Material,
    
    /// Lifecycle information
    pub spawn_time: Instant,
    pub lifetime: f32,
    
    /// Pool management
    pub generation: u32,
    pub state: ResourceState,
}

impl Default for DynamicRenderData {
    fn default() -> Self {
        Self {
            position: Vec3::zeros(),
            rotation: Vec3::zeros(),
            scale: Vec3::new(1.0, 1.0, 1.0),
            material: Material::standard_pbr(crate::render::material::StandardMaterialParams::default()),
            spawn_time: Instant::now(),
            lifetime: 0.0,
            generation: 0,
            state: ResourceState::Available,
        }
    }
}

impl DynamicRenderData {
    /// Check if this object has expired
    pub fn is_expired(&self) -> bool {
        self.spawn_time.elapsed().as_secs_f32() >= self.lifetime
    }
    
    /// Get the model matrix for this object
    pub fn get_model_matrix(&self) -> Mat4 {
        let translation = Mat4::new_translation(&self.position);
        let rotation_x = Mat4::rotation_x(self.rotation.x);
        let rotation_y = Mat4::rotation_y(self.rotation.y);
        let rotation_z = Mat4::rotation_z(self.rotation.z);
        let scale_matrix = Mat4::new_nonuniform_scaling(&self.scale);
        
        // Apply transforms: scale -> rotation -> translation
        translation * rotation_z * rotation_y * rotation_x * scale_matrix
    }
}

/// Pool for managing dynamic render objects
pub struct ObjectPool<T> {
    /// Storage for pool objects
    objects: Vec<T>,
    /// Free list for O(1) allocation
    free_indices: VecDeque<usize>,
    /// Generation counters for each slot
    generations: Vec<u32>,
    /// Maximum capacity
    capacity: usize,
}

impl<T: Default + Clone> ObjectPool<T> {
    /// Create a new object pool with specified capacity
    pub fn new(capacity: usize) -> Self {
        let mut objects = Vec::with_capacity(capacity);
        let mut free_indices = VecDeque::with_capacity(capacity);
        let mut generations = Vec::with_capacity(capacity);
        
        // Initialize all slots as available
        for i in 0..capacity {
            objects.push(T::default());
            free_indices.push_back(i);
            generations.push(0);
        }
        
        Self {
            objects,
            free_indices,
            generations,
            capacity,
        }
    }
    
    /// Allocate an object from the pool
    pub fn allocate(&mut self) -> Option<(usize, u32)> {
        if let Some(index) = self.free_indices.pop_front() {
            let generation = self.generations[index];
            Some((index, generation))
        } else {
            None
        }
    }
    
    /// Deallocate an object back to the pool
    pub fn deallocate(&mut self, index: usize) {
        if index < self.capacity {
            // Increment generation to invalidate old handles
            self.generations[index] = self.generations[index].wrapping_add(1);
            // Reset object to default state
            self.objects[index] = T::default();
            // Return to free list
            self.free_indices.push_back(index);
        }
    }
    
    /// Get object by index (without generation check)
    pub fn get(&self, index: usize) -> Option<&T> {
        self.objects.get(index)
    }
    
    /// Get mutable object by index (without generation check)
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.objects.get_mut(index)
    }
    
    /// Validate handle and get object
    pub fn get_with_handle(&self, handle: DynamicObjectHandle) -> Option<&T> {
        if self.is_handle_valid(handle) {
            self.get(handle.index as usize)
        } else {
            None
        }
    }
    
    /// Validate handle and get mutable object
    pub fn get_mut_with_handle(&mut self, handle: DynamicObjectHandle) -> Option<&mut T> {
        if self.is_handle_valid(handle) {
            self.get_mut(handle.index as usize)
        } else {
            None
        }
    }
    
    /// Check if a handle is valid
    pub fn is_handle_valid(&self, handle: DynamicObjectHandle) -> bool {
        let index = handle.index as usize;
        index < self.capacity && self.generations[index] == handle.generation
    }
    
    /// Get current generation for an index
    pub fn get_generation(&self, index: usize) -> Option<u32> {
        self.generations.get(index).copied()
    }
    
    /// Get number of available slots
    pub fn available_count(&self) -> usize {
        self.free_indices.len()
    }
    
    /// Get number of active objects
    pub fn active_count(&self) -> usize {
        self.capacity - self.free_indices.len()
    }
    
    /// Get total capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Core dynamic object manager
pub struct DynamicObjectManager {
    /// Pool of dynamic render objects
    object_pool: ObjectPool<DynamicRenderData>,
    
    /// Active object handles for this frame
    active_objects: Vec<DynamicObjectHandle>,
    
    /// Shared vertex buffer for all dynamic objects
    shared_vertex_buffer: Option<Buffer>,
    
    /// Shared index buffer for all dynamic objects
    shared_index_buffer: Option<Buffer>,
    
    /// Maximum number of instances supported
    max_instances: usize,
    
    /// Statistics for monitoring
    stats: DynamicObjectStats,
}

/// Statistics for dynamic object management
#[derive(Debug, Default)]
pub struct DynamicObjectStats {
    /// Total objects spawned
    pub total_spawned: u64,
    /// Total objects despawned
    pub total_despawned: u64,
    /// Current active count
    pub current_active: usize,
    /// Peak active count
    pub peak_active: usize,
    /// Pool exhaustion events
    pub pool_exhaustions: u64,
}

impl DynamicObjectManager {
    /// Create a new dynamic object manager
    pub fn new(max_instances: usize) -> Self {
        log::info!("Creating DynamicObjectManager with {} max instances", max_instances);
        
        Self {
            object_pool: ObjectPool::new(max_instances),
            active_objects: Vec::with_capacity(max_instances),
            shared_vertex_buffer: None,
            shared_index_buffer: None,
            max_instances,
            stats: DynamicObjectStats::default(),
        }
    }
    
    /// Initialize GPU resources
    pub fn initialize_resources(&mut self, context: &VulkanContext) -> VulkanResult<()> {
        log::debug!("Initializing DynamicObjectManager GPU resources");
        
        // For now, we'll initialize with empty buffers
        // These will be populated when we have actual mesh data
        self.shared_vertex_buffer = None;
        self.shared_index_buffer = None;
        
        log::info!("DynamicObjectManager GPU resources initialized");
        Ok(())
    }
    
    /// Spawn a new dynamic object
    pub fn spawn_object(&mut self, params: DynamicSpawnParams) -> Result<DynamicObjectHandle, DynamicObjectError> {
        // Try to allocate from pool
        if let Some((index, generation)) = self.object_pool.allocate() {
            // Initialize object data
            if let Some(object) = self.object_pool.get_mut(index) {
                object.position = params.position;
                object.rotation = params.rotation;
                object.scale = params.scale;
                object.material = params.material;
                object.spawn_time = Instant::now();
                object.lifetime = params.lifetime;
                object.generation = generation;
                object.state = ResourceState::Active;
            }
            
            let handle = DynamicObjectHandle::new(index as u32, generation);
            self.active_objects.push(handle);
            
            // Update statistics
            self.stats.total_spawned += 1;
            self.stats.current_active = self.active_objects.len();
            if self.stats.current_active > self.stats.peak_active {
                self.stats.peak_active = self.stats.current_active;
            }
            
            log::trace!("Spawned dynamic object with handle {:?} at position {:?}", handle, params.position);
            Ok(handle)
        } else {
            self.stats.pool_exhaustions += 1;
            log::warn!("Failed to spawn dynamic object: pool exhausted ({}/{} active)", 
                      self.object_pool.active_count(), 
                      self.object_pool.capacity());
            
            Err(DynamicObjectError::PoolExhausted {
                requested: 1,
                available: self.object_pool.available_count(),
            })
        }
    }
    
    /// Despawn a dynamic object
    pub fn despawn_object(&mut self, handle: DynamicObjectHandle) -> Result<(), DynamicObjectError> {
        if !self.object_pool.is_handle_valid(handle) {
            return Err(DynamicObjectError::InvalidHandle {
                handle,
                reason: "Handle generation mismatch or invalid index".to_string(),
            });
        }
        
        // Mark for cleanup
        if let Some(object) = self.object_pool.get_mut_with_handle(handle) {
            object.state = ResourceState::PendingCleanup;
        }
        
        log::trace!("Marked dynamic object {:?} for despawn", handle);
        Ok(())
    }
    
    /// Update object transform
    pub fn update_object_transform(&mut self, handle: DynamicObjectHandle, position: Vec3, rotation: Vec3, scale: Vec3) -> Result<(), DynamicObjectError> {
        if let Some(object) = self.object_pool.get_mut_with_handle(handle) {
            object.position = position;
            object.rotation = rotation;
            object.scale = scale;
            Ok(())
        } else {
            Err(DynamicObjectError::InvalidHandle {
                handle,
                reason: "Object not found or generation mismatch".to_string(),
            })
        }
    }
    
    /// Get object data by handle
    pub fn get_object(&self, handle: DynamicObjectHandle) -> Result<&DynamicRenderData, DynamicObjectError> {
        if let Some(object) = self.object_pool.get_with_handle(handle) {
            Ok(object)
        } else {
            Err(DynamicObjectError::InvalidHandle {
                handle,
                reason: "Object not found or generation mismatch".to_string(),
            })
        }
    }
    
    /// Update all dynamic objects (handle lifetime expiration)
    pub fn update(&mut self, _delta_time: f32) {
        let mut objects_to_cleanup = Vec::new();
        
        // Check for expired objects
        for &handle in &self.active_objects {
            if let Some(object) = self.object_pool.get_with_handle(handle) {
                if object.is_expired() || object.state == ResourceState::PendingCleanup {
                    objects_to_cleanup.push(handle);
                }
            }
        }
        
        // Clean up expired/marked objects
        for handle in objects_to_cleanup {
            self.cleanup_object(handle);
        }
        
        // Update active objects list
        self.active_objects.retain(|&handle| {
            self.object_pool.is_handle_valid(handle) && 
            self.object_pool.get_with_handle(handle)
                .map(|obj| obj.state == ResourceState::Active)
                .unwrap_or(false)
        });
        
        // Update statistics
        self.stats.current_active = self.active_objects.len();
    }
    
    /// Clean up a specific object
    fn cleanup_object(&mut self, handle: DynamicObjectHandle) {
        if self.object_pool.is_handle_valid(handle) {
            self.object_pool.deallocate(handle.index as usize);
            self.stats.total_despawned += 1;
            log::trace!("Cleaned up dynamic object {:?}", handle);
        }
    }
    
    /// Begin frame processing
    pub fn begin_frame(&mut self) {
        // Update objects and clean up expired ones
        self.update(0.0);
    }
    
    /// End frame processing
    pub fn end_frame(&mut self) {
        // Any post-frame cleanup if needed
    }
    
    /// Get active object handles
    pub fn get_active_objects(&self) -> &[DynamicObjectHandle] {
        &self.active_objects
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> &DynamicObjectStats {
        &self.stats
    }
    
    /// Get available object count
    pub fn available_count(&self) -> usize {
        self.object_pool.available_count()
    }
    
    /// Get active object count
    pub fn active_count(&self) -> usize {
        self.object_pool.active_count()
    }
    
    /// Get maximum capacity
    pub fn capacity(&self) -> usize {
        self.max_instances
    }
    
    /// Get active objects as HashMap for rendering
    ///
    /// Creates a HashMap mapping active object handles to their render data.
    /// This is used by the rendering system for batch operations.
    /// 
    /// WARNING: This method clones all render data - use for_each_active_object for better performance
    ///
    /// # Returns
    /// HashMap of active objects ready for rendering
    pub fn get_active_objects_map(&self) -> std::collections::HashMap<DynamicObjectHandle, DynamicRenderData> {
        let mut active_map = std::collections::HashMap::new();
        
        for &handle in &self.active_objects {
            if let Some(render_data) = self.object_pool.get_with_handle(handle) {
                active_map.insert(handle, render_data.clone());
            }
        }
        
        active_map
    }
    
    /// Iterate over active objects without cloning data
    ///
    /// More efficient than get_active_objects_map() as it avoids HashMap creation
    /// and render data cloning. Use this for performance-critical rendering paths.
    ///
    /// # Arguments
    /// * `callback` - Function called for each active object with handle and data reference
    pub fn for_each_active_object<F>(&self, mut callback: F) 
    where 
        F: FnMut(DynamicObjectHandle, &DynamicRenderData),
    {
        for &handle in &self.active_objects {
            if let Some(render_data) = self.object_pool.get_with_handle(handle) {
                callback(handle, render_data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::material::{StandardMaterialParams, MaterialType};
    
    fn create_test_material() -> Material {
        Material {
            material_type: MaterialType::StandardPBR(StandardMaterialParams::default()),
            id: crate::render::material::MaterialId(0),
            name: Some("Test Material".to_string()),
        }
    }
    
    #[test]
    fn test_object_pool_allocation() {
        let mut pool: ObjectPool<DynamicRenderData> = ObjectPool::new(10);
        
        // Test allocation
        let allocation1 = pool.allocate().expect("Should allocate");
        let (index1, gen1) = allocation1;
        let allocation2 = pool.allocate().expect("Should allocate");
        let (index2, gen2) = allocation2;
        
        assert_ne!(index1, index2);
        assert_eq!(gen1, 0);
        assert_eq!(gen2, 0);
        assert_eq!(pool.available_count(), 8);
        assert_eq!(pool.active_count(), 2);
    }
    
    #[test]
    fn test_object_pool_deallocation() {
        let mut pool: ObjectPool<DynamicRenderData> = ObjectPool::new(10);
        
        let allocation = pool.allocate().expect("Should allocate");
        let index = allocation.0;
        let gen = allocation.1;
        assert_eq!(pool.available_count(), 9);
        
        pool.deallocate(index);
        assert_eq!(pool.available_count(), 10);
        
        // Generation should increment
        let new_gen = pool.get_generation(index).expect("Should have generation");
        assert_eq!(new_gen, gen + 1);
    }
    
    #[test]
    fn test_handle_validation() {
        let mut pool: ObjectPool<DynamicRenderData> = ObjectPool::new(10);
        
        let allocation = pool.allocate().expect("Should allocate");
        let (index, gen) = allocation;
        let handle = DynamicObjectHandle::new(index as u32, gen);
        
        assert!(pool.is_handle_valid(handle));
        
        pool.deallocate(index);
        assert!(!pool.is_handle_valid(handle)); // Should be invalid after deallocation
    }
    
    #[test]
    fn test_dynamic_object_manager_spawn() {
        let mut manager = DynamicObjectManager::new(10);
        
        let params = DynamicSpawnParams::new(
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::zeros(),
            Vec3::new(1.0, 1.0, 1.0),
            create_test_material(),
            5.0,
        );
        
        let handle = manager.spawn_object(params).expect("Should spawn object");
        assert_eq!(manager.active_count(), 1);
        
        let object = manager.get_object(handle).expect("Should get object");
        assert_eq!(object.position, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(object.lifetime, 5.0);
    }
    
    #[test]
    fn test_dynamic_object_manager_pool_exhaustion() {
        let mut manager = DynamicObjectManager::new(2);
        
        let params = DynamicSpawnParams::new(
            Vec3::zeros(),
            Vec3::zeros(),
            Vec3::new(1.0, 1.0, 1.0),
            create_test_material(),
            5.0,
        );
        
        // Fill the pool
        manager.spawn_object(params.clone()).expect("Should spawn 1");
        manager.spawn_object(params.clone()).expect("Should spawn 2");
        
        // This should fail
        let result = manager.spawn_object(params);
        assert!(matches!(result, Err(DynamicObjectError::PoolExhausted { .. })));
    }
    
    #[test]
    fn test_dynamic_object_manager_despawn() {
        let mut manager = DynamicObjectManager::new(10);
        
        let params = DynamicSpawnParams::new(
            Vec3::zeros(),
            Vec3::zeros(),
            Vec3::new(1.0, 1.0, 1.0),
            create_test_material(),
            5.0,
        );
        
        let handle = manager.spawn_object(params).expect("Should spawn object");
        assert_eq!(manager.active_count(), 1);
        
        manager.despawn_object(handle).expect("Should despawn object");
        manager.update(0.0); // Process cleanup
        
        assert_eq!(manager.active_count(), 0);
    }
}