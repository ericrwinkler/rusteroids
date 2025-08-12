//! High-level Vulkan renderer
//! 
//! Combines all Vulkan components into a cohesive rendering system

use ash::vk;
use crate::render::vulkan::*;
use crate::render::mesh::Vertex;
use std::path::Path;

/// Push constants structure matching the shader interface
/// GLSL mat3 has 16-byte column alignment, so we need to pad each column to vec4
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PushConstants {
    mvp_matrix: [[f32; 4]; 4],      // 64 bytes
    // GLSL mat3 is stored as 3 vec4s (each column padded to 16 bytes)
    normal_matrix: [[f32; 4]; 3],   // 48 bytes - padded to match GLSL mat3 alignment
    material_color: [f32; 4],       // 16 bytes - RGBA color
    light_direction: [f32; 4],      // 16 bytes - XYZ direction + W intensity
    light_color: [f32; 4],          // 16 bytes - RGB color + A ambient intensity
}

// Safe to implement Pod and Zeroable for PushConstants since it only contains f32 arrays
unsafe impl bytemuck::Pod for PushConstants {}
unsafe impl bytemuck::Zeroable for PushConstants {}

/// High-level Vulkan renderer
pub struct VulkanRenderer {
    context: VulkanContext,
    render_pass: RenderPass,
    pipeline: GraphicsPipeline,
    command_pool: CommandPool,
    framebuffers: Vec<Framebuffer>,
    depth_buffers: Vec<DepthBuffer>,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    // Per-image synchronization (one per swapchain image)
    image_sync_objects: Vec<FrameSync>,
    // Per-frame synchronization (for frames in flight)
    frame_sync_objects: Vec<FrameSync>,
    current_frame: usize,
    max_frames_in_flight: usize,
    push_constants: PushConstants,
}

impl VulkanRenderer {
    /// Create new Vulkan renderer
    pub fn new(window: &mut Window) -> VulkanResult<Self> {
        log::debug!("Creating VulkanContext...");
        // Create Vulkan context
        let context = VulkanContext::new(window, "Rusteroids")?;
        log::debug!("VulkanContext created successfully");
        
        log::debug!("Creating RenderPass...");
        // Create render pass
        let render_pass = RenderPass::new_forward_pass(
            context.raw_device(),
            context.swapchain().format().format,
        )?;
        log::debug!("RenderPass created successfully");
        
        log::debug!("Loading shaders...");
        // Load shaders - use paths that work from both root and teapot_app directories
        let vertex_shader_path = if Path::new("target/shaders/vert.spv").exists() {
            Path::new("target/shaders/vert.spv")
        } else {
            Path::new("../target/shaders/vert.spv")
        };
        
        let fragment_shader_path = if Path::new("target/shaders/frag.spv").exists() {
            Path::new("target/shaders/frag.spv")
        } else {
            Path::new("../target/shaders/frag.spv")
        };
        
        let vertex_shader = ShaderModule::from_file(
            context.raw_device(),
            vertex_shader_path
        ).map_err(|e| {
            log::error!("Failed to load vertex shader: {:?}", e);
            e
        })?;
        
        let fragment_shader = ShaderModule::from_file(
            context.raw_device(),
            fragment_shader_path
        ).map_err(|e| {
            log::error!("Failed to load fragment shader: {:?}", e);
            e
        })?;
        log::debug!("Shaders loaded successfully");
        
        // Get vertex input state
        let (binding_desc, attr_desc) = Vertex::get_input_state();
        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[binding_desc])
            .vertex_attribute_descriptions(&attr_desc)
            .build();
        
        log::debug!("Creating graphics pipeline...");
        // Create graphics pipeline
        let pipeline = GraphicsPipeline::new(
            context.raw_device(),
            render_pass.handle(),
            &vertex_shader,
            &fragment_shader,
            vertex_input_info,
            context.swapchain().extent(),
        )?;
        log::debug!("Graphics pipeline created successfully");
        
        // Create command pool
        let command_pool = CommandPool::new(
            context.raw_device(),
            context.graphics_queue_family(),
        )?;
        
        // Create framebuffers
        let mut framebuffers = Vec::new();
        let mut depth_buffers = Vec::new();
        
        for image_view in context.swapchain().image_views() {
            let depth_buffer = DepthBuffer::new(
                context.raw_device(),
                &context.instance(),
                context.physical_device().device,
                context.swapchain().extent(),
            )?;
            
            let framebuffer = Framebuffer::new(
                context.raw_device(),
                render_pass.handle(),
                &[*image_view, depth_buffer.image_view()],
                context.swapchain().extent(),
            )?;
            
            depth_buffers.push(depth_buffer);
            framebuffers.push(framebuffer);
        }
        
        log::debug!("Creating buffers...");
        // Create test triangle vertices with proper attributes
        let vertices = vec![
            Vertex { 
                position: [0.0, -0.5, 0.0], 
                normal: [0.0, 0.0, 1.0],
                tex_coord: [0.5, 1.0]
            },
            Vertex { 
                position: [0.5, 0.5, 0.0], 
                normal: [0.0, 0.0, 1.0],
                tex_coord: [1.0, 0.0]
            },
            Vertex { 
                position: [-0.5, 0.5, 0.0], 
                normal: [0.0, 0.0, 1.0],
                tex_coord: [0.0, 0.0]
            },
        ];
        
        let indices = vec![0, 1, 2];
        
        // Create buffers
        let vertex_buffer = VertexBuffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            &vertices,
        )?;
        
        let index_buffer = IndexBuffer::new(
            context.raw_device(),
            context.instance().clone(),
            context.physical_device().device,
            &indices,
        )?;
        log::debug!("Buffers created successfully");
        
        log::debug!("Creating synchronization objects...");
        // Create synchronization objects
        // Per-image sync objects (one per swapchain image) - for semaphores
        let image_count = context.swapchain().image_count() as usize;
        let mut image_sync_objects = Vec::new();
        for _ in 0..image_count {
            image_sync_objects.push(FrameSync::new(context.raw_device())?);
        }
        
        // Per-frame sync objects (for frames in flight) - for fences
        let max_frames_in_flight = 2;
        let mut frame_sync_objects = Vec::new();
        for _ in 0..max_frames_in_flight {
            frame_sync_objects.push(FrameSync::new(context.raw_device())?);
        }
        log::debug!("Synchronization objects created successfully");
        
        log::debug!("VulkanRenderer initialization complete");
        Ok(Self {
            context,
            render_pass,
            pipeline,
            command_pool,
            framebuffers,
            depth_buffers,
            vertex_buffer,
            index_buffer,
            image_sync_objects,
            frame_sync_objects,
            current_frame: 0,
            max_frames_in_flight,
            push_constants: PushConstants {
                mvp_matrix: [
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [0.0, 0.0, 0.0, 1.0],
                ],
                normal_matrix: [
                    [1.0, 0.0, 0.0, 0.0], // Column 0: padded to vec4
                    [0.0, 1.0, 0.0, 0.0], // Column 1: padded to vec4
                    [0.0, 0.0, 1.0, 0.0], // Column 2: padded to vec4
                ],
                material_color: [1.0, 1.0, 1.0, 1.0], // Default white
                light_direction: [-0.5, -1.0, -0.3, 0.8], // Default directional light (direction + intensity)
                light_color: [1.0, 0.95, 0.9, 0.2], // Default warm white light + ambient intensity
            },
        })
    }
    
    /// Update mesh data for rendering
    pub fn update_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> VulkanResult<()> {
        log::trace!("Updating mesh with {} vertices and {} indices", vertices.len(), indices.len());
        
        // Wait for device to be idle before updating buffers
        self.wait_idle()?;
        
        // Create new vertex buffer
        self.vertex_buffer = VertexBuffer::new(
            self.context.raw_device(),
            self.context.instance().clone(),
            self.context.physical_device().device,
            vertices,
        )?;
        
        // Create new index buffer
        self.index_buffer = IndexBuffer::new(
            self.context.raw_device(),
            self.context.instance().clone(),
            self.context.physical_device().device,
            indices,
        )?;
        
        Ok(())
    }
    
    /// Set MVP matrix and model matrix for rendering (needed to calculate normal matrix)
    pub fn set_mvp_matrix(&mut self, mvp: [[f32; 4]; 4]) {
        self.push_constants.mvp_matrix = mvp;
        log::trace!("MVP matrix updated");
    }

    /// Set model matrix and calculate normal matrix for proper lighting
    ///
    /// The normal matrix is used to transform normals from object space to world space.
    /// It must be the inverse-transpose of the model matrix to handle non-uniform scaling correctly.
    /// 
    /// # Coordinate Spaces
    /// - Input normals: Object space (from 3D model)
    /// - Output normals: World space (for lighting calculations)
    /// - Light direction: World space (fixed direction)
    /// 
    /// # Matrix Storage
    /// GLSL expects column-major matrix storage, while nalgebra uses row-major indexing.
    /// We apply an additional transpose to convert between these formats.
    pub fn set_model_matrix(&mut self, model: [[f32; 4]; 4]) {
        // Calculate the proper normal matrix as inverse-transpose of the 3x3 model matrix
        // This ensures that normals are correctly transformed from object space to world space
        
        // Extract 3x3 part of model matrix
        use nalgebra::Matrix3;
        let mat3_model = Matrix3::new(
            model[0][0], model[0][1], model[0][2],
            model[1][0], model[1][1], model[1][2],
            model[2][0], model[2][1], model[2][2],
        );
        
        // Calculate inverse-transpose for normal transformation
        // This is mathematically required to transform normals correctly,
        // especially when the model matrix contains non-uniform scaling
        let normal_matrix = if let Some(inverse) = mat3_model.try_inverse() {
            inverse.transpose()
        } else {
            // Fallback to identity if matrix is not invertible
            log::warn!("Model matrix is not invertible, using identity for normal matrix");
            Matrix3::identity()
        };
        
        // Convert to the padded format required for GLSL mat3 alignment
        // GLSL matrices are column-major, so store columns of the normal matrix
        // Additional transpose is needed to convert from nalgebra's row-major indexing
        // to GLSL's column-major storage format
        let transposed = normal_matrix.transpose();
        self.push_constants.normal_matrix = [
            [transposed[(0, 0)], transposed[(1, 0)], transposed[(2, 0)], 0.0],  // First column 
            [transposed[(0, 1)], transposed[(1, 1)], transposed[(2, 1)], 0.0],  // Second column
            [transposed[(0, 2)], transposed[(1, 2)], transposed[(2, 2)], 0.0],  // Third column
        ];
        
        log::trace!("Normal matrix calculated as inverse-transpose of model matrix 3x3 part");
    }
    
    /// Set material color for rendering
    pub fn set_material_color(&mut self, color: [f32; 4]) {
        self.push_constants.material_color = color;
        log::trace!("Material color updated to {:?}", color);
    }
    
    /// Set directional light for rendering
    pub fn set_directional_light(&mut self, direction: [f32; 3], intensity: f32, color: [f32; 3], ambient_intensity: f32) {
        self.push_constants.light_direction = [direction[0], direction[1], direction[2], intensity];
        self.push_constants.light_color = [color[0], color[1], color[2], ambient_intensity];
        log::trace!("Directional light updated: dir={:?}, intensity={}, color={:?}, ambient={}", 
                   direction, intensity, color, ambient_intensity);
    }
    
    /// Draw a frame
    pub fn draw_frame(&mut self) -> VulkanResult<()> {
        // Get the frame sync object for CPU-GPU synchronization (fence)
        let frame_sync = &self.frame_sync_objects[self.current_frame];
        
        // Wait for this frame's previous work to complete
        frame_sync.in_flight.wait(u64::MAX)?;
        frame_sync.in_flight.reset()?;
        
        // We need to handle the chicken-and-egg problem differently
        // Let's acquire the next image with the current frame's image_available semaphore
        // and use per-image semaphores for render_finished
        let acquire_semaphore = &frame_sync.image_available;
        
        // Acquire the next image - handle ERROR_OUT_OF_DATE_KHR
        let (image_index, _) = match unsafe {
            self.context.swapchain_loader().acquire_next_image(
                self.context.swapchain().handle(),
                u64::MAX,
                acquire_semaphore.handle(),
                vk::Fence::null(),
            )
        } {
            Ok(result) => result,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::warn!("Swapchain out of date during acquire_next_image - swapchain recreation needed");
                // Return a special error to indicate swapchain recreation is needed
                return Err(VulkanError::Api(vk::Result::ERROR_OUT_OF_DATE_KHR));
            }
            Err(e) => return Err(VulkanError::Api(e)),
        };
        
        // Get the image-specific sync object for render finished semaphore
        let image_sync = &self.image_sync_objects[image_index as usize];
        
        // Record command buffer
        let mut recorder = self.command_pool.begin_single_time()?;
        
        let render_area = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(self.context.swapchain().extent())
            .build();
            
        let clear_values = [
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.2, 0.3, 0.8, 1.0], // Nice blue background instead of black
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            },
        ];
        
        // Begin render pass in a scope to drop it before ending
        {
            let mut render_pass = recorder.begin_render_pass(
                self.render_pass.handle(),
                self.framebuffers[image_index as usize].handle(),
                render_area,
                &clear_values,
            )?;
            
            // Bind pipeline and draw
            render_pass.cmd_bind_pipeline(vk::PipelineBindPoint::GRAPHICS, self.pipeline.handle());
            
            // Push constants to shader (both MVP matrix and material color)
            render_pass.cmd_push_constants(
                self.pipeline.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                bytemuck::cast_slice(&[self.push_constants]),
            );
            
            render_pass.cmd_bind_vertex_buffers(0, &[self.vertex_buffer.handle()], &[0]);
            render_pass.cmd_bind_index_buffer(self.index_buffer.handle(), 0, vk::IndexType::UINT32);
            
            render_pass.cmd_draw_indexed(self.index_buffer.index_count(), 1, 0, 0, 0);
        } // _render_pass is dropped here, ending the render pass
        
        // End and submit
        let command_buffer = recorder.end()?;
        
        let wait_semaphores = [acquire_semaphore.handle()];
        let command_buffers = [command_buffer];
        let signal_semaphores = [image_sync.render_finished.handle()];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
            
        unsafe {
            self.context.device().device.queue_submit(
                self.context.graphics_queue(),
                &[submit_info.build()],
                frame_sync.in_flight.handle(), // Use frame fence for CPU sync
            ).map_err(VulkanError::Api)?;
        }
        
        // Present - handle ERROR_OUT_OF_DATE_KHR
        let wait_semaphores = [image_sync.render_finished.handle()];
        let swapchains = [self.context.swapchain().handle()];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
            
        match unsafe {
            self.context.swapchain_loader().queue_present(
                self.context.present_queue(),
                &present_info
            )
        } {
            Ok(_) => {},
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::warn!("Swapchain out of date during queue_present - swapchain recreation needed");
                // Return a special error to indicate swapchain recreation is needed
                return Err(VulkanError::Api(vk::Result::ERROR_OUT_OF_DATE_KHR));
            }
            Err(e) => return Err(VulkanError::Api(e)),
        }
        
        // Advance to next frame
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
        Ok(())
    }
    
    /// Recreate the swapchain (for window resizing)
    /// This should be called when ERROR_OUT_OF_DATE_KHR is encountered
    pub fn recreate_swapchain(&mut self, window: &crate::render::vulkan::Window) -> VulkanResult<()> {
        log::info!("Recreating swapchain for window resize...");
        
        // Wait for device to be idle before recreation
        self.wait_idle()?;
        
        // Recreate the swapchain in the context
        self.context.recreate_swapchain(window)?;
        
        // Recreate framebuffers and depth buffers for the new swapchain
        self.recreate_framebuffers()?;
        
        log::info!("Swapchain recreation complete");
        Ok(())
    }
    
    /// Recreate framebuffers and depth buffers for the current swapchain
    fn recreate_framebuffers(&mut self) -> VulkanResult<()> {
        log::debug!("Recreating framebuffers and depth buffers...");
        
        // Clear old framebuffers and depth buffers (they will be dropped automatically)
        self.framebuffers.clear();
        self.depth_buffers.clear();
        
        // Create new framebuffers and depth buffers for each swapchain image
        for image_view in self.context.swapchain().image_views() {
            let depth_buffer = DepthBuffer::new(
                self.context.raw_device(),
                &self.context.instance(),
                self.context.physical_device().device,
                self.context.swapchain().extent(),
            )?;
            
            let framebuffer = Framebuffer::new(
                self.context.raw_device(),
                self.render_pass.handle(),
                &[*image_view, depth_buffer.image_view()],
                self.context.swapchain().extent(),
            )?;
            
            self.depth_buffers.push(depth_buffer);
            self.framebuffers.push(framebuffer);
        }
        
        log::debug!("Framebuffers and depth buffers recreated successfully");
        Ok(())
    }
    
    /// Wait for device to be idle
    pub fn wait_idle(&self) -> VulkanResult<()> {
        unsafe {
            self.context.device().device.device_wait_idle()
                .map_err(VulkanError::Api)
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        log::debug!("Cleaning up VulkanRenderer...");
        
        // Wait for device to be idle before cleanup
        let _ = self.wait_idle();
        
        // Resources will be cleaned up automatically by their Drop implementations
        // in the correct order due to struct field order:
        // 1. image_sync_objects and frame_sync_objects (FrameSync objects)
        // 2. vertex_buffer and index_buffer (VulkanBuffer objects) 
        // 3. depth_buffers (DepthBuffer objects)
        // 4. framebuffers (Framebuffer objects)
        // 5. command_pool (CommandPool - this destroys command buffers automatically)
        // 6. pipeline (GraphicsPipeline)
        // 7. render_pass (RenderPass)  
        // 8. context (VulkanContext - destroys device, then instance)
        
        log::debug!("VulkanRenderer cleanup complete");
    }
}
