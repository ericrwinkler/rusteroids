//! Command buffer management
//! 
//! Type-safe command buffer recording following RAII patterns from the design document

use ash::{vk, Device};
use crate::render::vulkan::{VulkanResult, VulkanError};

/// Command pool wrapper with RAII cleanup
pub struct CommandPool {
    device: Device,
    command_pool: vk::CommandPool,
}

impl CommandPool {
    /// Create a new command pool
    pub fn new(device: Device, queue_family_index: u32) -> VulkanResult<Self> {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);
        
        let command_pool = unsafe {
            device.create_command_pool(&pool_create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        Ok(Self {
            device,
            command_pool,
        })
    }
    
    /// Allocate command buffers
    pub fn allocate_command_buffers(&self, count: u32) -> VulkanResult<Vec<vk::CommandBuffer>> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(count);
        
        let command_buffers = unsafe {
            self.device.allocate_command_buffers(&alloc_info)
                .map_err(VulkanError::Api)?
        };
        
        Ok(command_buffers)
    }
    
    /// Get the command pool handle
    pub fn handle(&self) -> vk::CommandPool {
        self.command_pool
    }
    
    /// Begin single-time command buffer
    pub fn begin_single_time(&self) -> VulkanResult<CommandRecorder> {
        let command_buffers = self.allocate_command_buffers(1)?;
        let command_buffer = command_buffers[0];
        
        let mut recorder = CommandRecorder::new(command_buffer, self.device.clone());
        recorder.begin()?;
        Ok(recorder)
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            // Wait for device to be idle to ensure all command buffers are finished
            let _ = self.device.device_wait_idle();
            
            // Destroy command pool (automatically frees all command buffers)
            self.device.destroy_command_pool(self.command_pool, None);
        }
    }
}

/// Type-safe command buffer recorder following design document patterns
pub struct CommandRecorder {
    command_buffer: vk::CommandBuffer,
    device: Device,
    recording: bool,
}

impl CommandRecorder {
    /// Create a new command recorder
    pub fn new(command_buffer: vk::CommandBuffer, device: Device) -> Self {
        Self {
            command_buffer,
            device,
            recording: false,
        }
    }
    
    /// Begin command recording
    pub fn begin(&mut self) -> VulkanResult<&mut Self> {
        if self.recording {
            return Err(VulkanError::InvalidOperation { 
                reason: "Command buffer already recording".to_string() 
            });
        }
        
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        
        unsafe {
            self.device.begin_command_buffer(self.command_buffer, &begin_info)
                .map_err(VulkanError::Api)?;
        }
        
        self.recording = true;
        Ok(self)
    }
    
    /// Begin render pass
    pub fn begin_render_pass(
        &mut self,
        render_pass: vk::RenderPass,
        framebuffer: vk::Framebuffer,
        render_area: vk::Rect2D,
        clear_values: &[vk::ClearValue],
    ) -> VulkanResult<ActiveRenderPass<'_>> {
        if !self.recording {
            return Err(VulkanError::InvalidOperation { 
                reason: "Command buffer not recording".to_string() 
            });
        }
        
        let render_pass_begin = vk::RenderPassBeginInfo::builder()
            .render_pass(render_pass)
            .framebuffer(framebuffer)
            .render_area(render_area)
            .clear_values(clear_values);
        
        unsafe {
            self.device.cmd_begin_render_pass(
                self.command_buffer,
                &render_pass_begin,
                vk::SubpassContents::INLINE,
            );
        }
        
        Ok(ActiveRenderPass::new(self))
    }
    
    /// End command recording
    pub fn end(mut self) -> VulkanResult<vk::CommandBuffer> {
        if !self.recording {
            return Err(VulkanError::InvalidOperation { 
                reason: "Command buffer not recording".to_string() 
            });
        }
        
        unsafe {
            self.device.end_command_buffer(self.command_buffer)
                .map_err(VulkanError::Api)?;
        }
        
        self.recording = false;
        Ok(self.command_buffer)
    }
    
    /// Bind graphics pipeline
    pub fn cmd_bind_pipeline(&mut self, bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline) {
        unsafe {
            self.device.cmd_bind_pipeline(self.command_buffer, bind_point, pipeline);
        }
    }
    
    /// Bind vertex buffers
    pub fn cmd_bind_vertex_buffers(&mut self, first_binding: u32, buffers: &[vk::Buffer], offsets: &[vk::DeviceSize]) {
        unsafe {
            self.device.cmd_bind_vertex_buffers(self.command_buffer, first_binding, buffers, offsets);
        }
    }
    
    /// Bind index buffer
    pub fn cmd_bind_index_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        unsafe {
            self.device.cmd_bind_index_buffer(self.command_buffer, buffer, offset, index_type);
        }
    }
    
    /// Draw indexed
    pub fn cmd_draw_indexed(&mut self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe {
            self.device.cmd_draw_indexed(self.command_buffer, index_count, instance_count, first_index, vertex_offset, first_instance);
        }
    }
}

/// Active render pass following RAII pattern from design document
pub struct ActiveRenderPass<'a> {
    recorder: &'a mut CommandRecorder,
}

impl<'a> ActiveRenderPass<'a> {
    fn new(recorder: &'a mut CommandRecorder) -> Self {
        Self { recorder }
    }
    
    /// Set viewport
    pub fn set_viewport(&mut self, viewport: &vk::Viewport) {
        unsafe {
            self.recorder.device.cmd_set_viewport(
                self.recorder.command_buffer,
                0,
                &[*viewport],
            );
        }
    }
    
    /// Set scissor
    pub fn set_scissor(&mut self, scissor: &vk::Rect2D) {
        unsafe {
            self.recorder.device.cmd_set_scissor(
                self.recorder.command_buffer,
                0,
                &[*scissor],
            );
        }
    }
    
    /// Bind graphics pipeline
    pub fn cmd_bind_pipeline(&mut self, bind_point: vk::PipelineBindPoint, pipeline: vk::Pipeline) {
        unsafe {
            self.recorder.device.cmd_bind_pipeline(self.recorder.command_buffer, bind_point, pipeline);
        }
    }
    
    /// Bind vertex buffers
    pub fn cmd_bind_vertex_buffers(&mut self, first_binding: u32, buffers: &[vk::Buffer], offsets: &[vk::DeviceSize]) {
        unsafe {
            self.recorder.device.cmd_bind_vertex_buffers(self.recorder.command_buffer, first_binding, buffers, offsets);
        }
    }
    
    /// Bind index buffer
    pub fn cmd_bind_index_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        unsafe {
            self.recorder.device.cmd_bind_index_buffer(self.recorder.command_buffer, buffer, offset, index_type);
        }
    }
    
    /// Draw indexed
    pub fn cmd_draw_indexed(&mut self, index_count: u32, instance_count: u32, first_index: u32, vertex_offset: i32, first_instance: u32) {
        unsafe {
            self.recorder.device.cmd_draw_indexed(self.recorder.command_buffer, index_count, instance_count, first_index, vertex_offset, first_instance);
        }
    }
    
    /// Push constants to shaders
    pub fn cmd_push_constants(&mut self, pipeline_layout: vk::PipelineLayout, stage_flags: vk::ShaderStageFlags, offset: u32, data: &[u8]) {
        unsafe {
            self.recorder.device.cmd_push_constants(self.recorder.command_buffer, pipeline_layout, stage_flags, offset, data);
        }
    }
}

impl<'a> Drop for ActiveRenderPass<'a> {
    fn drop(&mut self) {
        unsafe {
            self.recorder.device.cmd_end_render_pass(self.recorder.command_buffer);
        }
    }
}
