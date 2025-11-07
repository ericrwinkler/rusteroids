//! Internal resource caching for VulkanRenderer
//! 
//! Manages internal caches for meshes and object resources, hiding
//! implementation details from applications.

use crate::render::backends::vulkan::*;
use crate::render::backends::vulkan::Buffer;
use ash::vk;
use std::collections::HashMap;

/// Per-object GPU resources (internal to VulkanRenderer)
/// This replaces the descriptor_sets in GameObject
/// TODO: Repurpose for material data instead of transform data (transforms use push constants)
pub struct ObjectResources {
    /// Uniform buffers for object data (one per frame-in-flight)
    /// Currently unused - transforms handled by push constants (Vulkan best practice)
    /// Future: Use for material/texture data
    #[allow(dead_code)]
    pub uniform_buffers: Vec<Buffer>,
    /// Mapped memory pointers for uniform buffers
    #[allow(dead_code)]
    pub uniform_mapped: Vec<*mut u8>,
    /// Vulkan descriptor sets for binding resources (one per frame-in-flight)
    /// Currently unused - will be repurposed for material data
    #[allow(dead_code)]
    pub descriptor_sets: Vec<vk::DescriptorSet>,
}

/// Manages internal resource caches for VulkanRenderer
/// 
/// Hides SharedRenderingResources and descriptor sets from applications,
/// providing a cleaner API that works with opaque handles.
pub struct CacheManager {
    // Internal resource cache - hides SharedRenderingResources from applications
    mesh_cache: HashMap<u64, crate::render::SharedRenderingResources>,
    next_mesh_id: u64,
    
    // Object resource cache - hides descriptor sets and uniform buffers from GameObject
    object_resources: HashMap<u64, ObjectResources>,
    next_object_id: u64,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            mesh_cache: HashMap::new(),
            next_mesh_id: 1,
            object_resources: HashMap::new(),
            next_object_id: 1,
        }
    }
    
    /// Create and cache mesh resources internally
    /// This replaces the need for applications to manage SharedRenderingResources
    pub fn create_and_cache_mesh(
        &mut self,
        context: &VulkanContext,
        mesh: &crate::render::Mesh,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        frame_descriptor_set_layout: vk::DescriptorSetLayout,
        material_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> VulkanResult<u64> {
        let mesh_id = self.next_mesh_id;
        self.next_mesh_id += 1;
        
        log::debug!("Creating internal mesh cache for mesh ID {}", mesh_id);
        
        // Create shared vertex buffer
        let vertex_buffer = Buffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            (mesh.vertices.len() * std::mem::size_of::<crate::render::Vertex>()) as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Upload vertex data
        vertex_buffer.write_data(&mesh.vertices)?;
        
        // Create shared index buffer
        let index_buffer = Buffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Upload index data
        index_buffer.write_data(&mesh.indices)?;
        
        let shared_resources = crate::render::SharedRenderingResources::new(
            vertex_buffer,
            index_buffer,
            pipeline,
            pipeline_layout,
            material_descriptor_set_layout,
            frame_descriptor_set_layout,
            mesh.indices.len() as u32,
            mesh.vertices.len() as u32,
        );
        
        // Cache it internally
        self.mesh_cache.insert(mesh_id, shared_resources);
        
        log::debug!("Cached mesh resources for ID {}", mesh_id);
        Ok(mesh_id)
    }
    
    /// Get cached mesh resources by ID
    pub fn get_cached_mesh(&self, mesh_id: u64) -> Option<&crate::render::SharedRenderingResources> {
        self.mesh_cache.get(&mesh_id)
    }
    
    /// Create object resources with uniform buffers and descriptor sets
    pub fn create_object_resources(
        &mut self,
        context: &VulkanContext,
        descriptor_pool: vk::DescriptorPool,
        object_descriptor_set_layout: vk::DescriptorSetLayout,
        max_frames_in_flight: usize,
    ) -> VulkanResult<u64> {
        let object_id = self.next_object_id;
        self.next_object_id += 1;
        
        // Create per-frame uniform buffers and descriptor sets
        let mut uniform_buffers = Vec::new();
        let mut uniform_mapped = Vec::new();
        
        for _frame_index in 0..max_frames_in_flight {
            // Create uniform buffer for this frame
            let buffer_size = std::mem::size_of::<crate::render::ObjectUBO>() as vk::DeviceSize;
            
            let buffer = Buffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            // Map memory for updates
            let mapped = buffer.map_memory()? as *mut u8;
            
            uniform_buffers.push(buffer);
            uniform_mapped.push(mapped);
        }
        
        // Create descriptor sets (one per frame-in-flight)
        let layouts = vec![object_descriptor_set_layout; max_frames_in_flight];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe {
            context.raw_device().allocate_descriptor_sets(&alloc_info)
                .map_err(VulkanError::Api)?
        };

        // Update descriptor sets to point to uniform buffers
        for (frame_index, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniform_buffers[frame_index].handle())
                .offset(0)
                .range(std::mem::size_of::<crate::render::ObjectUBO>() as vk::DeviceSize)
                .build();

            let descriptor_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[buffer_info])
                .build();

            unsafe {
                context.raw_device().update_descriptor_sets(&[descriptor_write], &[]);
            }
        }
        
        let object_resources = ObjectResources {
            uniform_buffers,
            uniform_mapped,
            descriptor_sets,
        };
        
        self.object_resources.insert(object_id, object_resources);
        
        Ok(object_id)
    }
    
    /// Get object resources by ID
    pub fn get_object_resources(&self, object_id: u64) -> Option<&ObjectResources> {
        self.object_resources.get(&object_id)
    }
    
    /// Get mutable object resources by ID
    pub fn get_object_resources_mut(&mut self, object_id: u64) -> Option<&mut ObjectResources> {
        self.object_resources.get_mut(&object_id)
    }
}
