//! High-level Vulkan renderer
//! 
//! Combines all Vulkan components into a cohesive rendering system

use ash::vk;
use crate::render::vulkan::*;
use crate::render::mesh::Vertex;
use std::path::Path;

/// Push constants structure matching the shader interface
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PushConstants {
    mvp_matrix: [[f32; 4]; 4],  // 64 bytes
    material_color: [f32; 4],   // 16 bytes - RGBA color
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
                material_color: [1.0, 1.0, 1.0, 1.0], // Default white
            },
        })
    }
    
    /// Update mesh data for rendering
    pub fn update_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> VulkanResult<()> {
        log::debug!("Updating mesh with {} vertices and {} indices", vertices.len(), indices.len());
        
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
    
    /// Set MVP matrix for rendering
    pub fn set_mvp_matrix(&mut self, mvp: [[f32; 4]; 4]) {
        self.push_constants.mvp_matrix = mvp;
        log::trace!("MVP matrix updated");
    }
    
    /// Set material color for rendering
    pub fn set_material_color(&mut self, color: [f32; 4]) {
        self.push_constants.material_color = color;
        log::trace!("Material color updated to {:?}", color);
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
        
        // Acquire the next image
        let (image_index, _) = unsafe {
            self.context.swapchain_loader().acquire_next_image(
                self.context.swapchain().handle(),
                u64::MAX,
                acquire_semaphore.handle(),
                vk::Fence::null(),
            ).map_err(VulkanError::Api)?
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
        
        // Present
        let wait_semaphores = [image_sync.render_finished.handle()];
        let swapchains = [self.context.swapchain().handle()];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
            
        unsafe {
            self.context.swapchain_loader().queue_present(
                self.context.present_queue(),
                &present_info
            ).map_err(VulkanError::Api)?;
        }
        
        // Advance to next frame
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
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
