//! Command recording for Vulkan renderer
//! 
//! Handles render pass recording, push constants, and draw commands.

use ash::vk;
use crate::render::vulkan::*;

/// Push constants structure for per-draw data
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PushConstants {
    model_matrix: [[f32; 4]; 4],    // 64 bytes - model transformation
    normal_matrix: [[f32; 4]; 3],   // 48 bytes - normal transformation (3×4 padded)
}

unsafe impl bytemuck::Pod for PushConstants {}
unsafe impl bytemuck::Zeroable for PushConstants {}

/// Manages command buffer recording and rendering operations
pub struct CommandRecorder {
    command_pool: CommandPool,
    push_constants: PushConstants,
    pending_command_buffers: Vec<Option<vk::CommandBuffer>>,
}

impl CommandRecorder {
    /// Create a new command recorder for handling command buffer operations
    pub fn new(
        context: &VulkanContext,
        _swapchain: &Swapchain,
        _max_frames_in_flight: usize
    ) -> VulkanResult<Self> {
        log::debug!("Creating CommandRecorder...");
        
        let command_pool = CommandPool::new(
            context.raw_device(),
            context.graphics_queue_family(),
        )?;
        
        Ok(Self {
            command_pool,
            push_constants: PushConstants {
                model_matrix: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                normal_matrix: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                ],
            },
            pending_command_buffers: Vec::new(),
        })
    }
    
    /// Initialize with frames in flight
    pub fn initialize(&mut self, max_frames_in_flight: usize) {
        self.pending_command_buffers = vec![None; max_frames_in_flight];
    }
    
    /// Set model matrix and calculate normal matrix
    pub fn set_model_matrix(&mut self, model: [[f32; 4]; 4]) {
        self.push_constants.model_matrix = model;
        
        // Calculate normal matrix as inverse-transpose of 3x3 model matrix
        use nalgebra::Matrix3;
        let mat3_model = Matrix3::new(
            model[0][0], model[0][1], model[0][2],
            model[1][0], model[1][1], model[1][2],
            model[2][0], model[2][1], model[2][2],
        );
        
        let normal_matrix = if let Some(inverse) = mat3_model.try_inverse() {
            inverse.transpose()
        } else {
            log::warn!("Model matrix is not invertible, using identity for normal matrix");
            Matrix3::identity()
        };
        
        // Convert to GLSL column-major format with padding
        let transposed = normal_matrix.transpose();
        self.push_constants.normal_matrix = [
            [transposed[(0, 0)], transposed[(1, 0)], transposed[(2, 0)], 0.0],
            [transposed[(0, 1)], transposed[(1, 1)], transposed[(2, 1)], 0.0],
            [transposed[(0, 2)], transposed[(1, 2)], transposed[(2, 2)], 0.0],
        ];
    }
    
    /// Record a complete frame
    pub fn record_frame(
        &mut self,
        context: &VulkanContext,
        render_pass: &RenderPass,
        swapchain_manager: &super::SwapchainManager,
        resource_manager: &super::ResourceManager,
        _ubo_manager: &super::UboManager,
        pipeline_manager: &crate::render::PipelineManager,
        image_index: u32,
        frame_index: usize,
    ) -> VulkanResult<vk::CommandBuffer> {
        
        // Free previous command buffer for this frame
        if let Some(prev_command_buffer) = self.pending_command_buffers[frame_index].take() {
            self.command_pool.free_command_buffer(prev_command_buffer);
        }
        
        // Begin recording
        let mut recorder = self.command_pool.begin_single_time()?;
        
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(context.swapchain().extent())
            .build();
            
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.2, 0.3, 0.8, 1.0], // Nice blue background
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        
        // Record render pass
        {
            let mut render_pass_recorder = recorder.begin_render_pass(
                render_pass.handle(),
                swapchain_manager.get_framebuffer(image_index as usize),
                render_area,
                &clear_values,
            )?;
            
            // Bind pipeline
            if let Some(active_pipeline) = pipeline_manager.get_active_pipeline() {
                render_pass_recorder.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, active_pipeline.handle());
            } else {
                return Err(VulkanError::InitializationFailed("No active pipeline".to_string()));
            }
            
            // Set dynamic viewport and scissor
            let extent = context.swapchain().extent();
            let viewport = vk::Viewport::builder()
                .x(0.0)
                .y(0.0)
                .width(extent.width as f32)
                .height(extent.height as f32)
                .min_depth(0.0)
                .max_depth(1.0)
                .build();
            
            let scissor = vk::Rect2D::builder()
                .offset(vk::Offset2D { x: 0, y: 0 })
                .extent(extent)
                .build();
                
            render_pass_recorder.set_viewport(&viewport);
            render_pass_recorder.set_scissor(&scissor);
            
            // Bind descriptor sets
            if let Some(active_pipeline) = pipeline_manager.get_active_pipeline() {
                // Bind Set 0 (frame data: camera + lighting)
                render_pass_recorder.cmd_bind_descriptor_sets(
                    vk::PipelineBindPoint::GRAPHICS,
                    active_pipeline.layout(),
                    0, // First set
                    &[resource_manager.frame_descriptor_sets()[frame_index]],
                    &[],
                );
                
                // Bind Set 1 (material data)
                render_pass_recorder.cmd_bind_descriptor_sets(
                    vk::PipelineBindPoint::GRAPHICS,
                    active_pipeline.layout(),
                    1, // Second set
                    &[resource_manager.material_descriptor_sets()[0]], // Use default material for now
                    &[],
                );
                
                // Push constants
                render_pass_recorder.cmd_push_constants(
                    active_pipeline.layout(),
                    vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                    0,
                    bytemuck::cast_slice(&[self.push_constants]),
                );
            }
            
            // Bind vertex and index buffers
            render_pass_recorder.cmd_bind_vertex_buffers(0, &[resource_manager.vertex_buffer().handle()], &[0]);
            render_pass_recorder.cmd_bind_index_buffer(resource_manager.index_buffer().handle(), 0, vk::IndexType::UINT32);
            
            // Draw indexed
            render_pass_recorder.cmd_draw_indexed(resource_manager.index_buffer().index_count(), 1, 0, 0, 0);
            
        } // Render pass ends here
        
        // End command recording
        let command_buffer = recorder.end()?;
        
        // Store for cleanup later
        self.pending_command_buffers[frame_index] = Some(command_buffer);
        
        Ok(command_buffer)
    }
    
    /// Free a command buffer when frame is complete
    pub fn free_command_buffer(&mut self, frame_index: usize) {
        if let Some(command_buffer) = self.pending_command_buffers[frame_index].take() {
            self.command_pool.free_command_buffer(command_buffer);
        }
    }
}
