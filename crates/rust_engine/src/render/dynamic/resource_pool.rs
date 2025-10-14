//! Dynamic Resource Pool System
//!
//! Provides pre-allocated pools of GPU resources for dynamic objects to eliminate
//! runtime allocation overhead. This system manages uniform buffers, descriptor sets,
//! and material instances with handle-based access patterns.
//!
//! # Architecture
//!
//! ```text
//! DynamicResourcePool
//!         ├── BufferPool (uniform buffers)
//!         ├── DescriptorSetPool (GPU bindings)
//!         └── MaterialInstancePool (material variations)
//!                     ↓
//!              Handle-based Access
//!                     ↓
//!           O(1) Allocation/Deallocation
//! ```
//!
//! # Usage
//!
//! ```rust
//! let mut pool = DynamicResourcePool::new(context, 100)?;
//! 
//! // Allocate resources for a dynamic object
//! let buffer_handle = pool.allocate_buffer()?;
//! let descriptor_handle = pool.allocate_descriptor_set(material_type)?;
//! let material_handle = pool.allocate_material_instance(base_material)?;
//! 
//! // Use resources...
//! 
//! // Automatic cleanup when handles are dropped
//! pool.deallocate_buffer(buffer_handle);
//! pool.deallocate_descriptor_set(descriptor_handle);
//! pool.deallocate_material_instance(material_handle);
//! ```

use crate::render::vulkan::{VulkanContext, VulkanResult, VulkanError, buffer::Buffer};
use crate::render::material::Material;
use ash::vk;
use std::collections::VecDeque;

/// Maximum number of resources per pool type
pub const DEFAULT_POOL_SIZE: usize = 100;

/// Number of frames in flight for double buffering
pub const FRAMES_IN_FLIGHT: usize = 2;

/// Handle for a pooled uniform buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferHandle {
    /// Index in the buffer pool
    pub index: u32,
    /// Generation counter for safety
    pub generation: u32,
    /// Frame index for double buffering
    pub frame: usize,
}

impl BufferHandle {
    /// Create a new buffer handle
    pub fn new(index: u32, generation: u32, frame: usize) -> Self {
        Self { index, generation, frame }
    }
    
    /// Check if this handle is valid
    pub fn is_valid(&self) -> bool {
        self.frame < FRAMES_IN_FLIGHT
    }
}

/// Handle for a pooled descriptor set
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DescriptorSetHandle {
    /// Index in the descriptor set pool
    pub index: u32,
    /// Generation counter for safety
    pub generation: u32,
    /// Material type this descriptor set is for
    pub material_type: u32,
}

impl DescriptorSetHandle {
    /// Create a new descriptor set handle
    pub fn new(index: u32, generation: u32, material_type: u32) -> Self {
        Self { index, generation, material_type }
    }
}

/// Handle for a pooled material instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialInstanceHandle {
    /// Index in the material instance pool
    pub index: u32,
    /// Generation counter for safety
    pub generation: u32,
}

impl MaterialInstanceHandle {
    /// Create a new material instance handle
    pub fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
}

/// Pool entry for uniform buffers
struct BufferPoolEntry {
    /// The actual uniform buffer
    buffer: Buffer,
    /// Mapped memory pointer for direct access
    mapped_memory: *mut u8,
    /// Current generation counter
    generation: u32,
    /// Whether this entry is currently allocated
    allocated: bool,
}

impl BufferPoolEntry {
    /// Create a new buffer pool entry
    fn new(context: &VulkanContext, buffer_size: u64) -> VulkanResult<Self> {
        let buffer = Buffer::new(
            context.raw_device().clone(),
            context.instance().clone(),
            context.physical_device().device,
            buffer_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        let mapped_memory = buffer.map_memory()? as *mut u8;
        
        Ok(Self {
            buffer,
            mapped_memory,
            generation: 0,
            allocated: false,
        })
    }
    
    /// Get the buffer handle
    fn handle(&self) -> vk::Buffer {
        self.buffer.handle()
    }
    
    /// Write data to the buffer
    unsafe fn write_data(&mut self, data: &[u8], offset: usize) -> VulkanResult<()> {
        if offset + data.len() > self.buffer.size() as usize {
            return Err(VulkanError::InvalidOperation {
                reason: "Buffer write would overflow".to_string(),
            });
        }
        
        let dst_ptr = self.mapped_memory.add(offset);
        std::ptr::copy_nonoverlapping(data.as_ptr(), dst_ptr, data.len());
        
        Ok(())
    }
}

/// Pool entry for descriptor sets
#[derive(Debug)]
struct DescriptorSetPoolEntry {
    /// The descriptor set
    descriptor_set: vk::DescriptorSet,
    /// Material type this descriptor set is configured for
    material_type: u32,
    /// Current generation counter
    generation: u32,
    /// Whether this entry is currently allocated
    allocated: bool,
}

/// Pool entry for material instances
#[derive(Debug)]
struct MaterialInstancePoolEntry {
    /// Base material this instance is derived from
    base_material: Material,
    /// Runtime material properties override
    material_properties: MaterialProperties,
    /// Current generation counter
    generation: u32,
    /// Whether this entry is currently allocated
    allocated: bool,
}

/// Runtime material properties that can be modified per instance
#[derive(Debug, Clone)]
pub struct MaterialProperties {
    /// Base color multiplier
    pub base_color: [f32; 4],
    /// Metallic factor (PBR materials only)
    pub metallic: f32,
    /// Roughness factor (PBR materials only)
    pub roughness: f32,
    /// Emission color
    pub emission: [f32; 3],
    /// Alpha transparency
    pub alpha: f32,
}

impl Default for MaterialProperties {
    fn default() -> Self {
        Self {
            base_color: [1.0, 1.0, 1.0, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emission: [0.0, 0.0, 0.0],
            alpha: 1.0,
        }
    }
}

/// Pool for uniform buffers with per-frame allocation
pub struct BufferPool {
    /// Buffer entries for each frame
    entries: Vec<Vec<BufferPoolEntry>>,
    /// Free list for each frame
    free_lists: Vec<VecDeque<u32>>,
    /// Pool size per frame
    pool_size: usize,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(context: &VulkanContext, pool_size: usize, buffer_size: u64) -> VulkanResult<Self> {
        let mut entries = Vec::with_capacity(FRAMES_IN_FLIGHT);
        let mut free_lists = Vec::with_capacity(FRAMES_IN_FLIGHT);
        
        // Create buffer pools for each frame in flight
        for _frame in 0..FRAMES_IN_FLIGHT {
            let mut frame_entries = Vec::with_capacity(pool_size);
            let mut frame_free_list = VecDeque::with_capacity(pool_size);
            
            // Pre-allocate all buffers for this frame
            for index in 0..pool_size {
                let entry = BufferPoolEntry::new(context, buffer_size)?;
                frame_entries.push(entry);
                frame_free_list.push_back(index as u32);
            }
            
            entries.push(frame_entries);
            free_lists.push(frame_free_list);
        }
        
        log::info!("Created BufferPool with {} buffers per frame ({} frames) = {} total buffers", 
                   pool_size, FRAMES_IN_FLIGHT, pool_size * FRAMES_IN_FLIGHT);
        
        Ok(Self {
            entries,
            free_lists,
            pool_size,
        })
    }
    
    /// Allocate a buffer handle
    pub fn allocate(&mut self, frame_index: usize) -> Option<BufferHandle> {
        if frame_index >= FRAMES_IN_FLIGHT {
            return None;
        }
        
        if let Some(index) = self.free_lists[frame_index].pop_front() {
            let entry = &mut self.entries[frame_index][index as usize];
            entry.allocated = true;
            entry.generation += 1;
            
            Some(BufferHandle::new(index, entry.generation, frame_index))
        } else {
            None
        }
    }
    
    /// Deallocate a buffer handle
    pub fn deallocate(&mut self, handle: BufferHandle) -> Result<(), &'static str> {
        if handle.frame >= FRAMES_IN_FLIGHT || handle.index as usize >= self.pool_size {
            return Err("Invalid buffer handle");
        }
        
        let entry = &mut self.entries[handle.frame][handle.index as usize];
        
        if !entry.allocated || entry.generation != handle.generation {
            return Err("Buffer handle is stale or not allocated");
        }
        
        entry.allocated = false;
        self.free_lists[handle.frame].push_back(handle.index);
        
        Ok(())
    }
    
    /// Get buffer by handle
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<vk::Buffer> {
        if handle.frame >= FRAMES_IN_FLIGHT || handle.index as usize >= self.pool_size {
            return None;
        }
        
        let entry = &self.entries[handle.frame][handle.index as usize];
        
        if entry.allocated && entry.generation == handle.generation {
            Some(entry.handle())
        } else {
            None
        }
    }
    
    /// Write data to a buffer
    pub unsafe fn write_buffer_data(&mut self, handle: BufferHandle, data: &[u8], offset: usize) -> VulkanResult<()> {
        if handle.frame >= FRAMES_IN_FLIGHT || handle.index as usize >= self.pool_size {
            return Err(VulkanError::InvalidOperation {
                reason: "Invalid buffer handle".to_string(),
            });
        }
        
        let entry = &mut self.entries[handle.frame][handle.index as usize];
        
        if !entry.allocated || entry.generation != handle.generation {
            return Err(VulkanError::InvalidOperation {
                reason: "Buffer handle is stale or not allocated".to_string(),
            });
        }
        
        entry.write_data(data, offset)
    }
    
    /// Get available buffer count for a frame
    pub fn available_count(&self, frame_index: usize) -> usize {
        if frame_index < FRAMES_IN_FLIGHT {
            self.free_lists[frame_index].len()
        } else {
            0
        }
    }
    
    /// Get total buffer count for a frame
    pub fn total_count(&self) -> usize {
        self.pool_size
    }
}

/// Pool for descriptor sets
#[derive(Debug)]
pub struct DescriptorSetPool {
    /// Descriptor set entries
    entries: Vec<DescriptorSetPoolEntry>,
    /// Free list of available indices
    free_list: VecDeque<u32>,
    /// Pool size
    pool_size: usize,
}

impl DescriptorSetPool {
    /// Create a new descriptor set pool
    pub fn new(pool_size: usize) -> Self {
        let mut entries = Vec::with_capacity(pool_size);
        let mut free_list = VecDeque::with_capacity(pool_size);
        
        // Create placeholder entries (descriptor sets will be allocated as needed)
        for index in 0..pool_size {
            entries.push(DescriptorSetPoolEntry {
                descriptor_set: vk::DescriptorSet::null(),
                material_type: 0,
                generation: 0,
                allocated: false,
            });
            free_list.push_back(index as u32);
        }
        
        log::info!("Created DescriptorSetPool with {} descriptor sets", pool_size);
        
        Self {
            entries,
            free_list,
            pool_size,
        }
    }
    
    /// Allocate a descriptor set handle
    pub fn allocate(&mut self, material_type: u32, descriptor_set: vk::DescriptorSet) -> Option<DescriptorSetHandle> {
        if let Some(index) = self.free_list.pop_front() {
            let entry = &mut self.entries[index as usize];
            entry.descriptor_set = descriptor_set;
            entry.material_type = material_type;
            entry.allocated = true;
            entry.generation += 1;
            
            Some(DescriptorSetHandle::new(index, entry.generation, material_type))
        } else {
            None
        }
    }
    
    /// Deallocate a descriptor set handle
    pub fn deallocate(&mut self, handle: DescriptorSetHandle) -> Result<(), &'static str> {
        if handle.index as usize >= self.pool_size {
            return Err("Invalid descriptor set handle");
        }
        
        let entry = &mut self.entries[handle.index as usize];
        
        if !entry.allocated || entry.generation != handle.generation {
            return Err("Descriptor set handle is stale or not allocated");
        }
        
        entry.allocated = false;
        entry.descriptor_set = vk::DescriptorSet::null();
        self.free_list.push_back(handle.index);
        
        Ok(())
    }
    
    /// Get descriptor set by handle
    pub fn get_descriptor_set(&self, handle: DescriptorSetHandle) -> Option<vk::DescriptorSet> {
        if handle.index as usize >= self.pool_size {
            return None;
        }
        
        let entry = &self.entries[handle.index as usize];
        
        if entry.allocated && entry.generation == handle.generation {
            Some(entry.descriptor_set)
        } else {
            None
        }
    }
    
    /// Get available descriptor set count
    pub fn available_count(&self) -> usize {
        self.free_list.len()
    }
    
    /// Get total descriptor set count
    pub fn total_count(&self) -> usize {
        self.pool_size
    }
}

/// Pool for material instances
#[derive(Debug)]
pub struct MaterialInstancePool {
    /// Material instance entries
    entries: Vec<MaterialInstancePoolEntry>,
    /// Free list of available indices
    free_list: VecDeque<u32>,
    /// Pool size
    pool_size: usize,
}

impl MaterialInstancePool {
    /// Create a new material instance pool
    pub fn new(pool_size: usize) -> Self {
        let mut entries = Vec::with_capacity(pool_size);
        let mut free_list = VecDeque::with_capacity(pool_size);
        
        // Create placeholder entries
        for index in 0..pool_size {
            entries.push(MaterialInstancePoolEntry {
                base_material: Material::unlit(crate::render::material::UnlitMaterialParams {
                    color: crate::foundation::math::Vec3::new(1.0, 1.0, 1.0),
                    alpha: 1.0,
                }),
                material_properties: MaterialProperties::default(),
                generation: 0,
                allocated: false,
            });
            free_list.push_back(index as u32);
        }
        
        log::info!("Created MaterialInstancePool with {} material instances", pool_size);
        
        Self {
            entries,
            free_list,
            pool_size,
        }
    }
    
    /// Allocate a material instance handle
    pub fn allocate(&mut self, base_material: Material, properties: MaterialProperties) -> Option<MaterialInstanceHandle> {
        if let Some(index) = self.free_list.pop_front() {
            let entry = &mut self.entries[index as usize];
            entry.base_material = base_material;
            entry.material_properties = properties;
            entry.allocated = true;
            entry.generation += 1;
            
            Some(MaterialInstanceHandle::new(index, entry.generation))
        } else {
            None
        }
    }
    
    /// Deallocate a material instance handle
    pub fn deallocate(&mut self, handle: MaterialInstanceHandle) -> Result<(), &'static str> {
        if handle.index as usize >= self.pool_size {
            return Err("Invalid material instance handle");
        }
        
        let entry = &mut self.entries[handle.index as usize];
        
        if !entry.allocated || entry.generation != handle.generation {
            return Err("Material instance handle is stale or not allocated");
        }
        
        entry.allocated = false;
        self.free_list.push_back(handle.index);
        
        Ok(())
    }
    
    /// Get material instance by handle
    pub fn get_material_instance(&self, handle: MaterialInstanceHandle) -> Option<(&Material, &MaterialProperties)> {
        if handle.index as usize >= self.pool_size {
            return None;
        }
        
        let entry = &self.entries[handle.index as usize];
        
        if entry.allocated && entry.generation == handle.generation {
            Some((&entry.base_material, &entry.material_properties))
        } else {
            None
        }
    }
    
    /// Update material properties for an instance
    pub fn update_properties(&mut self, handle: MaterialInstanceHandle, properties: MaterialProperties) -> Result<(), &'static str> {
        if handle.index as usize >= self.pool_size {
            return Err("Invalid material instance handle");
        }
        
        let entry = &mut self.entries[handle.index as usize];
        
        if !entry.allocated || entry.generation != handle.generation {
            return Err("Material instance handle is stale or not allocated");
        }
        
        entry.material_properties = properties;
        Ok(())
    }
    
    /// Get available material instance count
    pub fn available_count(&self) -> usize {
        self.free_list.len()
    }
    
    /// Get total material instance count
    pub fn total_count(&self) -> usize {
        self.pool_size
    }
}

/// Combined resource pool for dynamic objects
pub struct DynamicResourcePool {
    /// Buffer pool for uniform buffers
    buffer_pool: BufferPool,
    /// Descriptor set pool
    descriptor_set_pool: DescriptorSetPool,
    /// Material instance pool
    material_instance_pool: MaterialInstancePool,
    /// Pool configuration
    pool_size: usize,
    /// Statistics
    stats: PoolStats,
}

/// Statistics for resource pool usage
#[derive(Debug, Default)]
pub struct PoolStats {
    /// Number of buffers currently allocated
    pub buffers_allocated: usize,
    /// Number of descriptor sets currently allocated
    pub descriptor_sets_allocated: usize,
    /// Number of material instances currently allocated
    pub material_instances_allocated: usize,
    /// Peak usage counts
    pub peak_buffers: usize,
    /// Maximum number of descriptor sets allocated simultaneously
    pub peak_descriptor_sets: usize,
    /// Maximum number of material instances allocated simultaneously
    pub peak_material_instances: usize,
}

impl DynamicResourcePool {
    /// Create a new dynamic resource pool
    pub fn new(context: &VulkanContext, pool_size: usize) -> VulkanResult<Self> {
        // Calculate uniform buffer size (enough for model matrix + material data)
        let uniform_buffer_size = 256; // 256 bytes per object (model matrix + material properties)
        
        let buffer_pool = BufferPool::new(context, pool_size, uniform_buffer_size)?;
        let descriptor_set_pool = DescriptorSetPool::new(pool_size);
        let material_instance_pool = MaterialInstancePool::new(pool_size);
        
        log::info!("Created DynamicResourcePool with {} resources per pool", pool_size);
        
        Ok(Self {
            buffer_pool,
            descriptor_set_pool,
            material_instance_pool,
            pool_size,
            stats: PoolStats::default(),
        })
    }
    
    /// Allocate a uniform buffer
    pub fn allocate_buffer(&mut self, frame_index: usize) -> Option<BufferHandle> {
        let handle = self.buffer_pool.allocate(frame_index)?;
        self.stats.buffers_allocated += 1;
        self.stats.peak_buffers = self.stats.peak_buffers.max(self.stats.buffers_allocated);
        Some(handle)
    }
    
    /// Deallocate a uniform buffer
    pub fn deallocate_buffer(&mut self, handle: BufferHandle) -> Result<(), &'static str> {
        self.buffer_pool.deallocate(handle)?;
        self.stats.buffers_allocated = self.stats.buffers_allocated.saturating_sub(1);
        Ok(())
    }
    
    /// Allocate a descriptor set
    pub fn allocate_descriptor_set(&mut self, material_type: u32, descriptor_set: vk::DescriptorSet) -> Option<DescriptorSetHandle> {
        let handle = self.descriptor_set_pool.allocate(material_type, descriptor_set)?;
        self.stats.descriptor_sets_allocated += 1;
        self.stats.peak_descriptor_sets = self.stats.peak_descriptor_sets.max(self.stats.descriptor_sets_allocated);
        Some(handle)
    }
    
    /// Deallocate a descriptor set
    pub fn deallocate_descriptor_set(&mut self, handle: DescriptorSetHandle) -> Result<(), &'static str> {
        self.descriptor_set_pool.deallocate(handle)?;
        self.stats.descriptor_sets_allocated = self.stats.descriptor_sets_allocated.saturating_sub(1);
        Ok(())
    }
    
    /// Allocate a material instance
    pub fn allocate_material_instance(&mut self, base_material: Material, properties: MaterialProperties) -> Option<MaterialInstanceHandle> {
        let handle = self.material_instance_pool.allocate(base_material, properties)?;
        self.stats.material_instances_allocated += 1;
        self.stats.peak_material_instances = self.stats.peak_material_instances.max(self.stats.material_instances_allocated);
        Some(handle)
    }
    
    /// Deallocate a material instance
    pub fn deallocate_material_instance(&mut self, handle: MaterialInstanceHandle) -> Result<(), &'static str> {
        self.material_instance_pool.deallocate(handle)?;
        self.stats.material_instances_allocated = self.stats.material_instances_allocated.saturating_sub(1);
        Ok(())
    }
    
    /// Get buffer by handle
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<vk::Buffer> {
        self.buffer_pool.get_buffer(handle)
    }
    
    /// Write data to a buffer
    pub unsafe fn write_buffer_data(&mut self, handle: BufferHandle, data: &[u8], offset: usize) -> VulkanResult<()> {
        self.buffer_pool.write_buffer_data(handle, data, offset)
    }
    
    /// Get descriptor set by handle
    pub fn get_descriptor_set(&self, handle: DescriptorSetHandle) -> Option<vk::DescriptorSet> {
        self.descriptor_set_pool.get_descriptor_set(handle)
    }
    
    /// Get material instance by handle
    pub fn get_material_instance(&self, handle: MaterialInstanceHandle) -> Option<(&Material, &MaterialProperties)> {
        self.material_instance_pool.get_material_instance(handle)
    }
    
    /// Update material properties
    pub fn update_material_properties(&mut self, handle: MaterialInstanceHandle, properties: MaterialProperties) -> Result<(), &'static str> {
        self.material_instance_pool.update_properties(handle, properties)
    }
    
    /// Get pool statistics
    pub fn get_stats(&self) -> &PoolStats {
        &self.stats
    }
    
    /// Get available resource counts
    pub fn get_availability(&self, frame_index: usize) -> (usize, usize, usize) {
        (
            self.buffer_pool.available_count(frame_index),
            self.descriptor_set_pool.available_count(),
            self.material_instance_pool.available_count(),
        )
    }
    
    /// Check if pools are running low on resources
    pub fn is_low_on_resources(&self, frame_index: usize, threshold_percent: f32) -> (bool, bool, bool) {
        let threshold = (self.pool_size as f32 * threshold_percent / 100.0) as usize;
        
        (
            self.buffer_pool.available_count(frame_index) < threshold,
            self.descriptor_set_pool.available_count() < threshold,
            self.material_instance_pool.available_count() < threshold,
        )
    }
    
    /// Initialize all resources (called during engine startup)
    pub fn initialize_resources(&mut self, _context: &VulkanContext) -> VulkanResult<()> {
        // Resources are already initialized in constructors
        log::debug!("DynamicResourcePool resources initialized");
        Ok(())
    }
    
    /// Cleanup all resources
    pub fn cleanup(&mut self) {
        log::debug!("Cleaning up DynamicResourcePool resources");
        // Cleanup happens automatically when pools are dropped
    }
}

impl Drop for DynamicResourcePool {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::material::{Material, UnlitMaterialParams};
    use crate::foundation::math::Vec3;
    
    #[test]
    fn test_buffer_handle_validity() {
        let handle = BufferHandle::new(0, 1, 0);
        assert!(handle.is_valid());
        
        let invalid_handle = BufferHandle::new(0, 1, 10); // Invalid frame
        assert!(!invalid_handle.is_valid());
    }
    
    #[test]
    fn test_material_properties_default() {
        let props = MaterialProperties::default();
        assert_eq!(props.base_color, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(props.metallic, 0.0);
        assert_eq!(props.roughness, 0.5);
        assert_eq!(props.emission, [0.0, 0.0, 0.0]);
        assert_eq!(props.alpha, 1.0);
    }
    
    #[test]
    fn test_descriptor_set_pool_allocation() {
        let mut pool = DescriptorSetPool::new(10);
        assert_eq!(pool.available_count(), 10);
        assert_eq!(pool.total_count(), 10);
        
        // Allocate descriptor set
        let handle = pool.allocate(0, vk::DescriptorSet::null()).expect("Should allocate");
        assert_eq!(pool.available_count(), 9);
        
        // Deallocate
        pool.deallocate(handle).expect("Should deallocate");
        assert_eq!(pool.available_count(), 10);
    }
    
    #[test]
    fn test_material_instance_pool_allocation() {
        let mut pool = MaterialInstancePool::new(10);
        assert_eq!(pool.available_count(), 10);
        
        let material = Material::unlit(UnlitMaterialParams {
            color: Vec3::new(1.0, 0.0, 0.0),
            alpha: 1.0,
        });
        let properties = MaterialProperties::default();
        
        // Allocate material instance
        let handle = pool.allocate(material, properties).expect("Should allocate");
        assert_eq!(pool.available_count(), 9);
        
        // Test getting the material instance
        let (_base_mat, props) = pool.get_material_instance(handle).expect("Should get material");
        assert_eq!(props.base_color, [1.0, 1.0, 1.0, 1.0]);
        
        // Deallocate
        pool.deallocate(handle).expect("Should deallocate");
        assert_eq!(pool.available_count(), 10);
    }
    
    #[test]
    fn test_handle_generation_validation() {
        let mut pool = DescriptorSetPool::new(5);
        
        // Allocate and deallocate to increment generation
        let handle1 = pool.allocate(0, vk::DescriptorSet::null()).unwrap();
        pool.deallocate(handle1).unwrap();
        
        // Allocate again, should get new generation
        let handle2 = pool.allocate(0, vk::DescriptorSet::null()).unwrap();
        
        // Old handle should be invalid
        assert!(pool.get_descriptor_set(handle1).is_none());
        // New handle should be valid
        assert!(pool.get_descriptor_set(handle2).is_some());
    }
}