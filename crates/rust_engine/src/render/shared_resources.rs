//! SharedRenderingResources module for managing resources shared between multiple objects
//!
//! This module provides the SharedRenderingResources struct which holds Vulkan resources
//! that can be shared between multiple game objects, such as vertex buffers, index buffers,
//! pipelines, and descriptor set layouts. This is key to the efficient multiple objects
//! rendering architecture.

use crate::render::vulkan::buffer::Buffer;
use ash::vk;

/// Resources that can be shared between multiple game objects
pub struct SharedRenderingResources {
    /// Shared vertex buffer containing mesh geometry
    pub vertex_buffer: Buffer,
    
    /// Shared index buffer for indexed drawing
    pub index_buffer: Buffer,
    
    /// Graphics pipeline object (keeps the VkPipeline alive)
    pub graphics_pipeline: Option<crate::render::vulkan::shader::GraphicsPipeline>,
    
    /// Graphics pipeline handle for rendering
    pub pipeline: vk::Pipeline,
    
    /// Pipeline layout for binding descriptor sets
    pub pipeline_layout: vk::PipelineLayout,
    
    /// Descriptor set layout for set 1: per-object data (material + textures, 5 descriptors)
    pub material_descriptor_set_layout: vk::DescriptorSetLayout,
    
    /// Descriptor set layout for set 0: per-frame data (camera + lighting, 2 descriptors)
    pub frame_descriptor_set_layout: vk::DescriptorSetLayout,
    
    /// Number of indices to draw
    pub index_count: u32,
    
    /// Number of vertices in the buffer
    pub vertex_count: u32,
}

impl SharedRenderingResources {
    /// Create new shared rendering resources for a specific mesh and pipeline
    pub fn new(
        vertex_buffer: Buffer,
        index_buffer: Buffer,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        material_descriptor_set_layout: vk::DescriptorSetLayout,
        frame_descriptor_set_layout: vk::DescriptorSetLayout,
        index_count: u32,
        vertex_count: u32,
    ) -> Self {
        // Debug log the pipeline handle being stored
        log::info!("SharedRenderingResources storing pipeline: {:?}", pipeline);
        
        Self {
            vertex_buffer,
            index_buffer,
            graphics_pipeline: None, // For legacy paths that don't provide GraphicsPipeline object
            pipeline,
            pipeline_layout,
            material_descriptor_set_layout,
            frame_descriptor_set_layout,
            index_count,
            vertex_count,
        }
    }
    
    /// Create new shared rendering resources with a GraphicsPipeline object to keep it alive
    pub fn new_with_graphics_pipeline(
        vertex_buffer: Buffer,
        index_buffer: Buffer,
        graphics_pipeline: crate::render::vulkan::shader::GraphicsPipeline,
        pipeline_layout: vk::PipelineLayout,
        material_descriptor_set_layout: vk::DescriptorSetLayout,
        frame_descriptor_set_layout: vk::DescriptorSetLayout,
        index_count: u32,
        vertex_count: u32,
    ) -> Self {
        let pipeline_handle = graphics_pipeline.handle();
        
        // Debug log the pipeline handle being stored
        log::debug!("SharedRenderingResources storing pipeline object with handle: {:?}", pipeline_handle);
        
        Self {
            vertex_buffer,
            index_buffer,
            graphics_pipeline: Some(graphics_pipeline),
            pipeline: pipeline_handle,
            pipeline_layout,
            material_descriptor_set_layout,
            frame_descriptor_set_layout,
            index_count,
            vertex_count,
        }
    }
    
    /// Bind the shared vertex and index buffers to the command buffer
    pub fn bind_buffers(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            // Bind vertex buffer
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[self.vertex_buffer.handle()],
                &[0],
            );
            
            // Bind index buffer
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.handle(),
                0,
                vk::IndexType::UINT32,
            );
        }
    }
    
    /// Bind the graphics pipeline to the command buffer
    pub fn bind_pipeline(&self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
        }
    }
    
    /// Get the pipeline layout for binding descriptor sets
    pub fn get_pipeline_layout(&self) -> vk::PipelineLayout {
        self.pipeline_layout
    }
    
    /// Get the material descriptor set layout (set 1: material + textures)
    pub fn get_material_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.material_descriptor_set_layout
    }
    
    /// Get the frame descriptor set layout
    pub fn get_frame_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.frame_descriptor_set_layout
    }
    
    /// Get the number of indices to draw
    pub fn get_index_count(&self) -> u32 {
        self.index_count
    }
    
    /// Get the number of vertices
    pub fn get_vertex_count(&self) -> u32 {
        self.vertex_count
    }
}

/// Builder for creating SharedRenderingResources
pub struct SharedRenderingResourcesBuilder {
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    pipeline: Option<vk::Pipeline>,
    pipeline_layout: Option<vk::PipelineLayout>,
    material_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    frame_descriptor_set_layout: Option<vk::DescriptorSetLayout>,
    index_count: u32,
    vertex_count: u32,
}

impl SharedRenderingResourcesBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            vertex_buffer: None,
            index_buffer: None,
            pipeline: None,
            pipeline_layout: None,
            material_descriptor_set_layout: None,
            frame_descriptor_set_layout: None,
            index_count: 0,
            vertex_count: 0,
        }
    }
    
    /// Set the vertex buffer
    pub fn vertex_buffer(mut self, buffer: Buffer, vertex_count: u32) -> Self {
        self.vertex_buffer = Some(buffer);
        self.vertex_count = vertex_count;
        self
    }
    
    /// Set the index buffer
    pub fn index_buffer(mut self, buffer: Buffer, index_count: u32) -> Self {
        self.index_buffer = Some(buffer);
        self.index_count = index_count;
        self
    }
    
    /// Set the pipeline and layout
    pub fn pipeline(mut self, pipeline: vk::Pipeline, layout: vk::PipelineLayout) -> Self {
        self.pipeline = Some(pipeline);
        self.pipeline_layout = Some(layout);
        self
    }
    
    /// Set the descriptor set layouts
    pub fn descriptor_set_layouts(
        mut self,
        material_layout: vk::DescriptorSetLayout,
        frame_layout: vk::DescriptorSetLayout,
    ) -> Self {
        self.material_descriptor_set_layout = Some(material_layout);
        self.frame_descriptor_set_layout = Some(frame_layout);
        self
    }
    
    /// Build the SharedRenderingResources
    pub fn build(self) -> Result<SharedRenderingResources, &'static str> {
        Ok(SharedRenderingResources::new(
            self.vertex_buffer.ok_or("Vertex buffer not set")?,
            self.index_buffer.ok_or("Index buffer not set")?,
            self.pipeline.ok_or("Pipeline not set")?,
            self.pipeline_layout.ok_or("Pipeline layout not set")?,
            self.material_descriptor_set_layout.ok_or("Material descriptor set layout not set")?,
            self.frame_descriptor_set_layout.ok_or("Frame descriptor set layout not set")?,
            self.index_count,
            self.vertex_count,
        ))
    }
}

impl Default for SharedRenderingResourcesBuilder {
    fn default() -> Self {
        Self::new()
    }
}
