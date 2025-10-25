//! Resource management for Vulkan renderer
//! 
//! Handles vertex buffers, index buffers, descriptor resources, and default textures.

use ash::vk;
use crate::render::vulkan::*;
use crate::render::mesh::Vertex;

/// Manages GPU resources like buffers, descriptor sets, and default textures
pub struct ResourceManager {
    // Buffer resources
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    
    // Descriptor management
    descriptor_pool: DescriptorPool,
    frame_descriptor_sets: Vec<vk::DescriptorSet>,
    material_descriptor_sets: Vec<vk::DescriptorSet>,
    
    // Default textures for material placeholders
    default_white_texture: Texture,
    default_normal_texture: Texture,
    default_metallic_roughness_texture: Texture,
    
    // Command pool for transfers
    command_pool: CommandPool,
    /// Extra dynamically loaded textures kept alive here so descriptor sets can reference them
    loaded_textures: Vec<Texture>,
}

impl ResourceManager {
    /// Create a new resource manager for handling vertex/index buffers and models
    pub fn new(context: &VulkanContext, max_frames_in_flight: usize) -> VulkanResult<Self> {
        log::debug!("Creating ResourceManager...");
        
        // Create initial empty buffers
        let empty_vertices = vec![Vertex { 
            position: [0.0; 3], 
            normal: [0.0; 3],
            tex_coord: [0.0; 2]
        }];
        let empty_indices = vec![0u32];
        
        let vertex_buffer = VertexBuffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            &empty_vertices,
        )?;
        
        let index_buffer = IndexBuffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            &empty_indices,
        )?;
        
        // Create descriptor pool
        // Need enough sets for: frame sets (max_frames_in_flight) + material sets (50+ objects)
        let max_descriptor_sets = (max_frames_in_flight + 100) as u32; // Increased from 10 to 100 to support many objects
        let descriptor_pool = DescriptorPool::new(context.raw_device().clone(), max_descriptor_sets)?;
        
        // Create command pool for transfers
        let command_pool = CommandPool::new(
            context.raw_device(),
            context.graphics_queue_family(),
        )?;
        
        // Create default textures for material placeholders
        let default_white_texture = Texture::create_default_white(
            std::sync::Arc::new(context.raw_device()),
            std::sync::Arc::new(context.instance().clone()),
            context.physical_device().device,
            command_pool.handle(),
            context.graphics_queue(),
        )?;
        
        let default_normal_texture = Texture::create_default_normal(
            std::sync::Arc::new(context.raw_device()),
            std::sync::Arc::new(context.instance().clone()),
            context.physical_device().device,
            command_pool.handle(),
            context.graphics_queue(),
        )?;
        
        let default_metallic_roughness_texture = Texture::create_default_metallic_roughness(
            std::sync::Arc::new(context.raw_device()),
            std::sync::Arc::new(context.instance().clone()),
            context.physical_device().device,
            command_pool.handle(),
            context.graphics_queue(),
        )?;
        
        Ok(Self {
            vertex_buffer,
            index_buffer,
            descriptor_pool,
            frame_descriptor_sets: Vec::new(),
            material_descriptor_sets: Vec::new(),
            default_white_texture,
            default_normal_texture,
            default_metallic_roughness_texture,
            command_pool,
            loaded_textures: Vec::new(),
        })
    }

    /// Add a loaded texture to the resource manager to keep it alive and return its index
    pub fn add_loaded_texture(&mut self, texture: Texture) -> usize {
        let idx = self.loaded_textures.len();
        self.loaded_textures.push(texture);
        idx
    }

    /// Get a reference to a previously loaded texture
    pub fn get_loaded_texture(&self, idx: usize) -> Option<&Texture> {
        self.loaded_textures.get(idx)
    }
    
    /// Allocate descriptor sets for frame and material data
    pub fn allocate_descriptor_sets(
        &mut self, 
        frame_layouts: &[vk::DescriptorSetLayout],
        material_layouts: &[vk::DescriptorSetLayout]
    ) -> VulkanResult<()> {
        self.frame_descriptor_sets = self.descriptor_pool.allocate_descriptor_sets(frame_layouts)?;
        self.material_descriptor_sets = self.descriptor_pool.allocate_descriptor_sets(material_layouts)?;
        Ok(())
    }
    
    /// Update mesh data with proper staging buffers
    pub fn update_mesh(&mut self, context: &VulkanContext, vertices: &[Vertex], indices: &[u32]) -> VulkanResult<()> {
        log::trace!("Updating mesh with {} vertices and {} indices", vertices.len(), indices.len());
        
        let vertex_buffer_size = (vertices.len() * std::mem::size_of::<Vertex>()) as u64;
        let index_buffer_size = (indices.len() * std::mem::size_of::<u32>()) as u64;
        
        // Recreate buffers if needed (when size increases)
        if vertex_buffer_size > self.vertex_buffer.size() {
            self.vertex_buffer = VertexBuffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                vertices,
            )?;
        } else {
            self.update_vertex_buffer_with_staging(context, vertices)?;
        }
        
        if index_buffer_size > self.index_buffer.size() {
            self.index_buffer = IndexBuffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                indices,
            )?;
        } else {
            self.update_index_buffer_with_staging(context, indices)?;
        }
        
        Ok(())
    }
    
    /// Update vertex buffer using staging buffer with blog-guided synchronization
    fn update_vertex_buffer_with_staging(&mut self, context: &VulkanContext, vertices: &[Vertex]) -> VulkanResult<()> {
        let staging_buffer = StagingBuffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            bytemuck::cast_slice(vertices),
        )?;
        
        let mut recorder = self.command_pool.begin_single_time()?;
        
        // BLOG GUIDANCE: Execution barrier first - wait for vertex input before transfer
        recorder.cmd_pipeline_barrier(
            vk::PipelineStageFlags::VERTEX_INPUT,     // srcStageMask: wait for vertex reads
            vk::PipelineStageFlags::TRANSFER,         // dstStageMask: unblock transfer writes
            vk::DependencyFlags::empty(),
            &[vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)  // Make vertex reads available
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)         // Make visible to transfer
                .build()],
            &[],
            &[],
        );
        
        // Memory barriers - Host write to transfer read
        let host_to_transfer_barrier = MemoryBarrierBuilder::buffer_host_write_to_transfer_read();
        recorder.cmd_pipeline_barrier(
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[host_to_transfer_barrier],
            &[],
            &[],
        );
        
        // Copy buffer
        let copy_region = vk::BufferCopy::builder()
            .src_offset(0)
            .dst_offset(0)
            .size((vertices.len() * std::mem::size_of::<Vertex>()) as u64)
            .build();
            
        recorder.cmd_copy_buffer(
            staging_buffer.handle(),
            self.vertex_buffer.handle(),
            &[copy_region],
        );
        
        let transfer_to_vertex_barrier = MemoryBarrierBuilder::buffer_transfer_to_vertex_read();
        recorder.cmd_pipeline_barrier(
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::VERTEX_INPUT,
            vk::DependencyFlags::empty(),
            &[transfer_to_vertex_barrier],
            &[],
            &[],
        );
        
        // Submit and wait
        let command_buffer = recorder.end()?;
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build();
            
        unsafe {
            context.device().device.queue_submit(
                context.graphics_queue(),
                &[submit_info],
                vk::Fence::null(),
            ).map_err(VulkanError::Api)?;
            
            context.device().device.queue_wait_idle(context.graphics_queue())
                .map_err(VulkanError::Api)?;
        }
        
        self.command_pool.free_command_buffer(command_buffer);
        Ok(())
    }
    
    /// Update index buffer using staging buffer
    fn update_index_buffer_with_staging(&mut self, context: &VulkanContext, indices: &[u32]) -> VulkanResult<()> {
        let staging_buffer = StagingBuffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            bytemuck::cast_slice(indices),
        )?;
        
        let mut recorder = self.command_pool.begin_single_time()?;
        
        let host_to_transfer_barrier = MemoryBarrierBuilder::buffer_host_write_to_transfer_read();
        recorder.cmd_pipeline_barrier(
            vk::PipelineStageFlags::HOST,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[host_to_transfer_barrier],
            &[],
            &[],
        );
        
        let copy_region = vk::BufferCopy::builder()
            .src_offset(0)
            .dst_offset(0)
            .size((indices.len() * std::mem::size_of::<u32>()) as u64)
            .build();
            
        recorder.cmd_copy_buffer(
            staging_buffer.handle(),
            self.index_buffer.handle(),
            &[copy_region],
        );
        
        let transfer_to_index_barrier = MemoryBarrierBuilder::buffer_transfer_to_index_read();
        recorder.cmd_pipeline_barrier(
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::VERTEX_INPUT,
            vk::DependencyFlags::empty(),
            &[transfer_to_index_barrier],
            &[],
            &[],
        );
        
        let command_buffer = recorder.end()?;
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build();
            
        unsafe {
            context.device().device.queue_submit(
                context.graphics_queue(),
                &[submit_info],
                vk::Fence::null(),
            ).map_err(VulkanError::Api)?;
            
            context.device().device.queue_wait_idle(context.graphics_queue())
                .map_err(VulkanError::Api)?;
        }
        
        self.command_pool.free_command_buffer(command_buffer);
        Ok(())
    }
    
    // Getters for other managers
    /// Get a reference to the vertex buffer
    pub fn vertex_buffer(&self) -> &VertexBuffer { &self.vertex_buffer }
    
    /// Get a reference to the index buffer
    pub fn index_buffer(&self) -> &IndexBuffer { &self.index_buffer }
    
    /// Get a reference to the frame descriptor sets
    pub fn frame_descriptor_sets(&self) -> &[vk::DescriptorSet] { &self.frame_descriptor_sets }
    
    /// Get a reference to the material descriptor sets
    pub fn material_descriptor_sets(&self) -> &[vk::DescriptorSet] { &self.material_descriptor_sets }
    
    /// Get the descriptor pool for creating additional descriptor sets
    pub fn get_descriptor_pool(&self) -> vk::DescriptorPool {
        self.descriptor_pool.handle()
    }

    /// Get the raw command pool handle for transfer operations
    pub fn command_pool_handle(&self) -> vk::CommandPool {
        self.command_pool.handle()
    }
    
    /// Get the default white texture (base color placeholder)
    pub fn default_white_texture(&self) -> &Texture {
        &self.default_white_texture
    }
    
    /// Get the default normal texture (normal map placeholder)
    pub fn default_normal_texture(&self) -> &Texture {
        &self.default_normal_texture
    }
    
    /// Get the default metallic-roughness texture (material properties placeholder)
    pub fn default_metallic_roughness_texture(&self) -> &Texture {
        &self.default_metallic_roughness_texture
    }
}
