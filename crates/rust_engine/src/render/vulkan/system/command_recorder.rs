//! Command recording for Vulkan renderer
//! 
//! Handles render pass recording, push constants, and draw commands.

use ash::vk;
use crate::render::vulkan::*;

/// Push constants structure for per-draw data
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PushConstants {
    /// Model transformation matrix (4×4 column-major)
    pub model_matrix: [[f32; 4]; 4],    // 64 bytes - model transformation
    /// Normal transformation matrix (3×4 padded to 4×4 for alignment)
    pub normal_matrix: [[f32; 4]; 3],   // 48 bytes - normal transformation (3×4 padded)
    /// Base material color with alpha channel
    pub material_color: [f32; 4],       // 16 bytes - material base color
}

unsafe impl bytemuck::Pod for PushConstants {}
unsafe impl bytemuck::Zeroable for PushConstants {}

/// Manages command buffer recording and rendering operations
pub struct CommandRecorder {
    command_pool: CommandPool,
    push_constants: PushConstants,
    pending_command_buffers: Vec<Option<vk::CommandBuffer>>,
    // State for multiple object recording (Vulkan tutorial pattern)
    active_recording: Option<ActiveRecording>,
}

/// Active recording session for multiple objects (following Vulkan tutorial exactly)
struct ActiveRecording {
    command_buffer: vk::CommandBuffer,
    device: ash::Device,
    frame_index: usize,
    render_pass_active: bool,
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
                material_color: [1.0, 1.0, 1.0, 1.0], // Default white material
            },
            pending_command_buffers: Vec::new(),
            active_recording: None,
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
    
    /// Set material color for push constants
    pub fn set_material_color(&mut self, color: [f32; 4]) {
        self.push_constants.material_color = color;
        log::trace!("CommandRecorder: Set material color in push constants to: {:?}", color);
    }
    
    /// Get the currently active command buffer, if any
    pub fn get_active_command_buffer(&self) -> Option<vk::CommandBuffer> {
        self.active_recording.as_ref().map(|recording| recording.command_buffer)
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
            
            // FIXME: LEGACY - Remove after modular pipeline system is complete
            // Bind pipeline (assumes single active pipeline for all objects)
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
            
            // FIXME: LEGACY - Remove after modular pipeline system is complete
            // Bind descriptor sets (assumes single active pipeline)
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
    
    /// Begin recording multiple objects (PROPER Vulkan tutorial implementation)
    /// This starts a command buffer recording session and keeps render pass active
    pub fn begin_multiple_object_recording(
        &mut self,
        context: &VulkanContext,
        render_pass: &RenderPass,
        swapchain_manager: &super::SwapchainManager,
        _resource_manager: &super::ResourceManager,
        _pipeline_manager: &crate::render::PipelineManager,
        image_index: u32,
        frame_index: usize,
        clear_color: [f32; 4],
    ) -> VulkanResult<()> {
        // Free previous command buffer for this frame
        if let Some(prev_command_buffer) = self.pending_command_buffers[frame_index].take() {
            self.command_pool.free_command_buffer(prev_command_buffer);
        }
        
        // Begin recording command buffer
        let command_buffers = self.command_pool.allocate_command_buffers(1)?;
        let command_buffer = command_buffers[0];
        
        // Begin command buffer recording
        let device = context.raw_device();
        unsafe {
            device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build(),
            ).map_err(|e| VulkanError::Api(e))?;
        }
        
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(context.swapchain().extent())
            .build();
            
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: clear_color, // Use clear color from config
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        
        // Begin render pass
        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(render_pass.handle())
                    .framebuffer(swapchain_manager.get_framebuffer(image_index as usize))
                    .render_area(render_area)
                    .clear_values(&clear_values)
                    .build(),
                vk::SubpassContents::INLINE,
            );
        }
        
        // Store active recording state
        self.active_recording = Some(ActiveRecording {
            command_buffer,
            device: device.clone(),
            frame_index,
            render_pass_active: true,
        });
        
        log::trace!("Started multiple object recording session for frame {}", frame_index);
        Ok(())
    }
    
    /// Setup shared resources (called once after beginning recording)
    /// This implements the tutorial's "bind shared resources once" pattern
    pub fn bind_shared_resources(
        &mut self,
        shared_resources: &crate::render::SharedRenderingResources,
        context: &VulkanContext,
        frame_descriptor_set: vk::DescriptorSet,
    ) -> VulkanResult<()> {
        let active_recording = self.active_recording.as_mut()
            .ok_or_else(|| VulkanError::InvalidOperation {
                reason: "No active recording session".to_string()
            })?;
        
        if !active_recording.render_pass_active {
            return Err(VulkanError::InvalidOperation {
                reason: "Render pass not active".to_string()
            });
        }
        
        let command_buffer = active_recording.command_buffer;
        let device = &active_recording.device;
        
        unsafe {
            // Bind the graphics pipeline
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                shared_resources.pipeline,
            );
            
            // Set dynamic viewport and scissor (CRITICAL for rendering)
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
                
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind frame descriptor set (Set 0: camera + lighting data)
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                shared_resources.pipeline_layout,
                0, // Set 0: frame data
                &[frame_descriptor_set],
                &[], // no dynamic offsets
            );
            
            // Bind vertex buffer
            let vertex_buffers = [shared_resources.vertex_buffer.handle()];
            let offsets = [0];
            device.cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);
            
            // Bind index buffer
            device.cmd_bind_index_buffer(
                command_buffer,
                shared_resources.index_buffer.handle(),
                0,
                vk::IndexType::UINT32,
            );
        }
        
        log::trace!("Bound shared resources for multiple object rendering: pipeline, viewport, scissor, frame descriptor set, vertex buffer, index buffer");
        Ok(())
    }
    
    /// Record a draw call for a single object (tutorial pattern: multiple vkCmdDrawIndexed in same command buffer)
    pub fn record_object_draw(
        &mut self,
        shared_resources: &crate::render::SharedRenderingResources,
        object_descriptor_sets: &[vk::DescriptorSet],
    ) -> VulkanResult<()> {
        let active_recording = self.active_recording.as_mut()
            .ok_or_else(|| VulkanError::InvalidOperation {
                reason: "No active recording session".to_string()
            })?;
        
        if !active_recording.render_pass_active {
            return Err(VulkanError::InvalidOperation {
                reason: "Render pass not active".to_string()
            });
        }
        
        let command_buffer = active_recording.command_buffer;
        let device = &active_recording.device;
        
        unsafe {
            // Bind per-object descriptor sets (material data for this specific object)
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                shared_resources.pipeline_layout,
                1, // Set 1: material descriptor sets (per-object)
                object_descriptor_sets,
                &[], // no dynamic offsets
            );
            
            // CRITICAL FIX: Push constants for model matrix (required by shader)
            device.cmd_push_constants(
                command_buffer,
                shared_resources.pipeline_layout,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&[self.push_constants]),
            );
            
            // Draw the indexed geometry for this object
            device.cmd_draw_indexed(
                command_buffer,
                shared_resources.index_count,
                1, // instance count
                0, // first index
                0, // vertex offset
                0, // first instance
            );
        }
        
        log::trace!("Recorded draw call for object: {} indices", shared_resources.index_count);
        Ok(())
    }
    
    /// Finish multiple object recording and submit (tutorial pattern: end render pass and submit all commands)
    pub fn finish_multiple_object_recording(&mut self) -> VulkanResult<vk::CommandBuffer> {
        let mut active_recording = self.active_recording.take()
            .ok_or_else(|| VulkanError::InvalidOperation {
                reason: "No active recording session to finish".to_string()
            })?;
        
        // End render pass
        if active_recording.render_pass_active {
            unsafe {
                active_recording.device.cmd_end_render_pass(active_recording.command_buffer);
            }
            active_recording.render_pass_active = false;
        }
        
        // End command buffer recording
        unsafe {
            active_recording.device.end_command_buffer(active_recording.command_buffer)
                .map_err(|e| VulkanError::Api(e))?;
        }
        
        // Store for cleanup later
        self.pending_command_buffers[active_recording.frame_index] = Some(active_recording.command_buffer);
        
        log::trace!("Finished multiple object recording session");
        Ok(active_recording.command_buffer)
    }
}

/// Session for recording multiple objects with shared geometry
/// This follows the Vulkan Multiple Objects tutorial pattern exactly
pub struct MultipleObjectSession<'a> {
    render_pass_recorder: crate::render::vulkan::core::commands::ActiveRenderPass<'a>,
    resource_manager: &'a super::ResourceManager,
    pipeline_manager: &'a crate::render::PipelineManager,
    frame_index: usize,
}

impl<'a> MultipleObjectSession<'a> {
    /// Record a draw command for a single object
    /// This uses the shared geometry but different per-object uniform buffer
    pub fn record_object_draw(
        &mut self,
        object_descriptor_set: vk::DescriptorSet,
        model_matrix: [[f32; 4]; 4],
        material_color: [f32; 4],
    ) -> VulkanResult<()> {
        // Create push constants for this object
        let push_constants = PushConstants {
            model_matrix,
            normal_matrix: Self::calculate_normal_matrix(model_matrix),
            material_color,
        };
        
        // FIXME: LEGACY - Remove after modular pipeline system is complete
        // This uses the single active pipeline for all objects
        if let Some(active_pipeline) = self.pipeline_manager.get_active_pipeline() {
            // Bind frame descriptor sets (Set 0: camera + lighting)
            self.render_pass_recorder.cmd_bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                active_pipeline.layout(),
                0, // First set
                &[self.resource_manager.frame_descriptor_sets()[self.frame_index]],
                &[],
            );
            
            // Bind object-specific descriptor set (Set 1: per-object material)
            self.render_pass_recorder.cmd_bind_descriptor_sets(
                vk::PipelineBindPoint::GRAPHICS,
                active_pipeline.layout(),
                1, // Second set  
                &[object_descriptor_set],
                &[],
            );
            
            // Push per-object constants
            self.render_pass_recorder.cmd_push_constants(
                active_pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&[push_constants]),
            );
            
            // Draw this object (shared geometry, different uniforms)
            self.render_pass_recorder.cmd_draw_indexed(
                self.resource_manager.index_buffer().index_count(), 
                1, // instance_count
                0, // first_index
                0, // vertex_offset
                0  // first_instance
            );
        }
        
        Ok(())
    }
    
    /// Complete recording and submit all draw commands
    pub fn finish(self) -> VulkanResult<vk::CommandBuffer> {
        // Drop render_pass_recorder which ends the render pass
        drop(self.render_pass_recorder);
        
        // Get the command buffer from the recorder - this is a bit tricky since we consumed it
        // For now, return a dummy command buffer; the real implementation would need refactoring
        
        // TODO: Properly handle command buffer completion
        // This is a temporary implementation that needs architectural improvement
        Err(VulkanError::InvalidOperation { 
            reason: "Multiple object recording completion needs architectural refactoring".to_string() 
        })
    }
    
    /// Calculate normal matrix from model matrix
    fn calculate_normal_matrix(model: [[f32; 4]; 4]) -> [[f32; 4]; 3] {
        use nalgebra::Matrix3;
        let mat3_model = Matrix3::new(
            model[0][0], model[0][1], model[0][2],
            model[1][0], model[1][1], model[1][2],
            model[2][0], model[2][1], model[2][2],
        );
        
        let normal_matrix = if let Some(inverse) = mat3_model.try_inverse() {
            inverse.transpose()
        } else {
            Matrix3::identity()
        };
        
        // Convert to GLSL column-major format with padding
        let transposed = normal_matrix.transpose();
        [
            [transposed[(0, 0)], transposed[(1, 0)], transposed[(2, 0)], 0.0],
            [transposed[(0, 1)], transposed[(1, 1)], transposed[(2, 1)], 0.0],
            [transposed[(0, 2)], transposed[(1, 2)], transposed[(2, 2)], 0.0],
        ]
    }
}
