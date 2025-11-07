//! Modular Vulkan renderer implementation
//! 
//! This module breaks down the monolithic VulkanRenderer into focused, 
//! single-responsibility components for better maintainability.

use super::resources::{ResourceManager, UboManager};
use super::rendering::command_recorder::CommandRecorder;
use super::state::{SyncManager, SwapchainManager};

use crate::render::backends::vulkan::*;
use crate::render::primitives::Vertex;
use crate::foundation::math::{Mat4, Vec3};
use ash::vk;
use std::collections::HashMap;

/// Per-object GPU resources (internal to VulkanRenderer)
/// This replaces the descriptor_sets in GameObject
/// TODO: Repurpose for material data instead of transform data (transforms use push constants)
struct ObjectResources {
    /// Uniform buffers for object data (one per frame-in-flight)
    /// Currently unused - transforms handled by push constants (Vulkan best practice)
    /// Future: Use for material/texture data
    #[allow(dead_code)]
    uniform_buffers: Vec<Buffer>,
    /// Mapped memory pointers for uniform buffers
    #[allow(dead_code)]
    uniform_mapped: Vec<*mut u8>,
    /// Vulkan descriptor sets for binding resources (one per frame-in-flight)
    /// Currently unused - will be repurposed for material data
    #[allow(dead_code)]
    descriptor_sets: Vec<vk::DescriptorSet>,
}

/// Streamlined Vulkan renderer that coordinates specialized managers
/// 
/// This is the main orchestrator that delegates specific responsibilities
/// to focused manager components, making the codebase more maintainable
/// and testable.
pub struct VulkanRenderer {
    // Core Vulkan context
    context: VulkanContext,
    render_pass: RenderPass,
    pipeline_manager: crate::render::PipelineManager,
    
    // Specialized managers for different concerns
    resource_manager: ResourceManager,
    ubo_manager: UboManager,
    command_recorder: CommandRecorder,
    sync_manager: SyncManager,
    swapchain_manager: SwapchainManager,
    
    // Minimal state tracking
    current_frame: usize,
    max_frames_in_flight: usize,
    
    // Rendering configuration
    clear_color: [f32; 4],
    
    // State for multiple object command recording
    command_recording_state: Option<CommandRecordingState>,
    
    // Internal resource cache - hides SharedRenderingResources from applications
    mesh_cache: HashMap<u64, crate::render::SharedRenderingResources>,
    next_mesh_id: u64,
    
    // Object resource cache - hides descriptor sets and uniform buffers from GameObject
    object_resources: HashMap<u64, ObjectResources>,
    next_object_id: u64,
    
    // Dynamic object instanced rendering system
    instance_renderer: Option<crate::render::systems::dynamic::InstanceRenderer>,
}

/// State for recording multiple objects in a single command buffer
struct CommandRecordingState {
    command_buffer: vk::CommandBuffer,
    frame_index: usize,
    image_index: u32,
    acquire_semaphore_handle: vk::Semaphore,
}

impl VulkanRenderer {
    /// Create new Vulkan renderer with modular architecture
    pub fn new(window: &mut Window, config: &crate::render::VulkanRendererConfig) -> VulkanResult<Self> {
        log::debug!("Creating modular VulkanRenderer...");
        
        // Create core Vulkan resources
        let context = VulkanContext::new(window, &config.application_name)?;
        let render_pass = RenderPass::new_forward_pass(
            context.raw_device(),
            context.swapchain().format().format,
        )?;
        
    // Create pipeline manager and initialize all 4 pipelines
        let mut pipeline_manager = crate::render::PipelineManager::new();
        
        // Create specialized managers
        let mut resource_manager = ResourceManager::new(&context, config.max_frames_in_flight)?;
        let ubo_manager = UboManager::new(&context, &resource_manager, config.max_frames_in_flight)?;
        let mut command_recorder = CommandRecorder::new(&context, &context.swapchain(), config.max_frames_in_flight)?;
        let sync_manager = SyncManager::new(&context, &context.swapchain(), config.max_frames_in_flight)?;
        let swapchain_manager = SwapchainManager::new(&context, &render_pass)?;
        
        // Initialize command recorder
        command_recorder.initialize(config.max_frames_in_flight);
        
        // Initialize all 4 standard pipelines (StandardPBR, Unlit, TransparentPBR, TransparentUnlit)
        pipeline_manager.initialize_standard_pipelines(
            &context,
            render_pass.handle(),
            &[ubo_manager.frame_descriptor_set_layout(), ubo_manager.material_descriptor_set_layout()],
        )?;
        
        // Then initialize UBO descriptor sets
        ubo_manager.initialize_descriptor_sets(&context, &mut resource_manager)?;
        
        log::debug!("Modular VulkanRenderer created successfully");
        Ok(Self {
            context,
            render_pass,
            pipeline_manager,
            resource_manager,
            ubo_manager,
            command_recorder,
            sync_manager,
            swapchain_manager,
            current_frame: 0,
            max_frames_in_flight: config.max_frames_in_flight,
            clear_color: config.clear_color,
            command_recording_state: None,
            mesh_cache: HashMap::new(),
            next_mesh_id: 1,
            object_resources: HashMap::new(),
            next_object_id: 1,
            instance_renderer: None, // Will be initialized when dynamic system is enabled
        })
    }
    
    /// Update mesh data with blog-guided synchronization
    pub fn update_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> VulkanResult<()> {
        // BLOG GUIDANCE: Use targeted synchronization instead of device_wait_idle()
        // The resource manager now handles proper pipeline barriers internally
        self.resource_manager.update_mesh(&self.context, vertices, indices)
    }
    
    /// Set model matrix (delegates to command recorder)
    pub fn set_model_matrix(&mut self, model: [[f32; 4]; 4]) {
        self.command_recorder.set_model_matrix(model);
    }
    
    /// Draw a frame (coordinates all managers)
    pub fn draw_frame(&mut self) -> VulkanResult<()> {
        // Wait for previous frame BEFORE reusing its semaphore
        self.sync_manager.wait_for_frame_completion(&self.context, self.current_frame)?;
        
        // Now it's safe to acquire next image using this frame's semaphore
        let (image_index, acquire_semaphore_handle) = {
            let (idx, sem) = self.sync_manager.acquire_next_image(&self.context, self.current_frame)?;
            (idx, sem.handle())
        };
        
        // CRITICAL: Update UBO manager's current frame before recording commands
        self.ubo_manager.set_current_frame(self.current_frame);
        
        // Record basic command buffer with render pass for instanced rendering
        let command_buffer = self.record_minimal_command_buffer(
            image_index,
            self.current_frame,
        )?;
        
        // Submit and present (pass the semaphore handle)
        let _render_finished_semaphore = self.sync_manager.submit_and_present(
            &self.context,
            command_buffer,
            acquire_semaphore_handle,
            image_index,
            self.current_frame,
        )?;
        
        // Advance frame
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
        Ok(())
    }
    
    /// Recreate swapchain (delegates to swapchain manager)
    pub fn recreate_swapchain(&mut self, window: &Window) -> VulkanResult<()> {
        self.context.recreate_swapchain(window)?;
        self.swapchain_manager.recreate_framebuffers(&self.context, &self.render_pass)?;
        Ok(())
    }
    
    /// Get swapchain extent
    pub fn get_swapchain_extent(&self) -> (u32, u32) {
        let extent = self.context.swapchain().extent();
        (extent.width, extent.height)
    }
    
    /// Wait for device idle
    pub fn wait_idle(&self) -> VulkanResult<()> {
        self.sync_manager.wait_idle(&self.context)
    }
    
    // === Internal Resource Management (Hidden from Applications) ===
    
    /// Create and cache mesh resources internally
    /// This replaces the need for applications to manage SharedRenderingResources
    pub(crate) fn create_and_cache_mesh(&mut self, mesh: &crate::render::Mesh, materials: &[crate::render::Material]) -> VulkanResult<u64> {
        let mesh_id = self.next_mesh_id;
        self.next_mesh_id += 1;
        
        log::debug!("Creating internal mesh cache for mesh ID {}", mesh_id);
        
        // Create SharedRenderingResources internally (not exposed to applications)
        let shared_resources = self.create_shared_rendering_resources_internal(mesh, materials)?;
        
        // Cache it internally
        self.mesh_cache.insert(mesh_id, shared_resources);
        
        log::debug!("Cached mesh resources for ID {}", mesh_id);
        Ok(mesh_id)
    }
    
    /// Get cached mesh resources by ID
    pub(crate) fn get_cached_mesh(&self, mesh_id: u64) -> Option<&crate::render::SharedRenderingResources> {
        self.mesh_cache.get(&mesh_id)
    }
    
    /// Create SharedRenderingResources internally (replaces the public method)
    fn create_shared_rendering_resources_internal(&self, mesh: &crate::render::Mesh, _materials: &[crate::render::Material]) -> VulkanResult<crate::render::SharedRenderingResources> {
        use ash::vk;
        
        log::debug!("Creating SharedRenderingResources internally with {} vertices and {} indices", 
                   mesh.vertices.len(), mesh.indices.len());
        
        // Create shared vertex buffer
        let vertex_buffer = Buffer::new(
            self.context.raw_device(),
            self.context.instance().clone(),
            self.context.physical_device().device,
            (mesh.vertices.len() * std::mem::size_of::<crate::render::Vertex>()) as vk::DeviceSize,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Upload vertex data
        vertex_buffer.write_data(&mesh.vertices)?;
        
        // Create shared index buffer
        let index_buffer = Buffer::new(
            self.context.raw_device(),
            self.context.instance().clone(),
            self.context.physical_device().device,
            (mesh.indices.len() * std::mem::size_of::<u32>()) as vk::DeviceSize,
            vk::BufferUsageFlags::INDEX_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Upload index data
        index_buffer.write_data(&mesh.indices)?;
        
        // Get pipeline and layouts from existing VulkanRenderer methods
        let pipeline = self.get_graphics_pipeline();
        let pipeline_layout = self.get_pipeline_layout();
        let frame_descriptor_set_layout = self.get_frame_descriptor_set_layout();
        let material_descriptor_set_layout = self.get_object_descriptor_set_layout();
        
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
        
        log::debug!("SharedRenderingResources created internally successfully");
        Ok(shared_resources)
    }
}

// Backend trait implementation stays similar but delegates to managers
impl crate::render::RenderBackend for VulkanRenderer {
    fn get_swapchain_extent(&self) -> (u32, u32) {
        self.get_swapchain_extent()
    }
    
    fn update_mesh(&mut self, vertices: &[crate::render::Vertex], indices: &[u32]) -> crate::render::BackendResult<()> {
        self.update_mesh(vertices, indices)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
    }
    
    fn set_mvp_matrix(&mut self, _mvp: [[f32; 4]; 4]) {
        // MVP matrix is now calculated from camera UBOs
        log::trace!("set_mvp_matrix ignored - using UBO-based camera matrices");
    }
    
    fn set_model_matrix(&mut self, model: [[f32; 4]; 4]) {
        self.command_recorder.set_model_matrix(model);
    }
    
    fn set_material_color(&mut self, color: [f32; 4]) {
        // Update push constants for multi-light shaders
        self.command_recorder.set_material_color(color);
        // Also delegate to UBO manager for material UBO-based shaders  
        if let Err(e) = self.ubo_manager.update_material_color(color) {
            log::error!("Failed to update material color: {:?}", e);
        }
    }
    
    fn set_directional_light(&mut self, direction: [f32; 3], intensity: f32, color: [f32; 3], ambient_intensity: f32) {
        self.ubo_manager.update_lighting(direction, intensity, color, ambient_intensity);
    }
    
    fn set_multi_light_environment(&mut self, multi_light_env: &crate::render::systems::lighting::MultiLightEnvironment) {
        self.ubo_manager.update_multi_light_environment(multi_light_env);
    }
    
    fn update_camera_ubo(&mut self, view_matrix: Mat4, projection_matrix: Mat4, view_projection_matrix: Mat4, camera_position: Vec3) {
        self.ubo_manager.update_camera(view_matrix, projection_matrix, view_projection_matrix, camera_position);
    }
    
    fn bind_shared_resources(&mut self, _shared: &crate::render::SharedRenderingResources) -> crate::render::BackendResult<()> {
    // FIXME: LEGACY SYSTEM DISABLED: Instance renderer handles all resource binding internally
    // No-op method - the instance renderer manages its own pipelines and resources
    log::trace!("Legacy bind_shared_resources disabled - using instanced rendering");
        Ok(())
    }
    
    fn record_shared_object_draw(&mut self, shared: &crate::render::SharedRenderingResources, object_id: u32) -> crate::render::BackendResult<()> {
        log::trace!("Recording shared object draw for object {} with {} indices", object_id, shared.index_count);
        
        // Check if we have an active recording
        if self.command_recording_state.is_none() {
            return Err(crate::render::RenderError::BackendError(
                "No active command recording. Call begin_render_pass() first.".to_string()
            ));
        }
        
        // Note: Transform data (model matrix, normal matrix) is handled via push constants
        // This follows Vulkan best practices: push constants for small, frequently-changing data
        // UBOs/descriptor sets are reserved for larger, less frequently-changing data (materials, textures)
        
        // Record the draw command using the shared mesh resources
        if let Some(ref recording_state) = self.command_recording_state {
            unsafe {
                self.context.device.device.cmd_draw_indexed(
                    recording_state.command_buffer,
                    shared.index_count,
                    1, // instance count
                    0, // first index
                    0, // vertex offset
                    0, // first instance
                );
            }
        } else {
            return Err(crate::render::RenderError::BackendError("No active command buffer".to_string()));
        }
        
        log::trace!("Recorded object draw using push constants for transform data for object {}", object_id);
        Ok(())
    }
    
    fn draw_frame(&mut self) -> crate::render::BackendResult<()> {
        self.draw_frame()
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
    }
    
    fn wait_idle(&self) -> crate::render::BackendResult<()> {
        self.wait_idle()
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
    }
    
    fn recreate_swapchain(&mut self, window_handle: &mut dyn crate::render::WindowBackendAccess) -> crate::render::BackendResult<()> {
        // Use the new backend-agnostic window access method
        let backend_window = window_handle.get_backend_window();
        if let Some(vulkan_window) = backend_window.downcast_mut::<crate::render::backends::vulkan::Window>() {
            self.recreate_swapchain(vulkan_window)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
        } else {
            Err(crate::render::RenderError::BackendError("Backend window is not a Vulkan window".to_string()))
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn begin_render_pass(&mut self) -> crate::render::BackendResult<()> {
        log::trace!("Beginning render pass for multiple objects");
        
        // If already recording, return error
        if self.command_recording_state.is_some() {
            return Err(crate::render::RenderError::BackendError(
                "Command recording already in progress".to_string()
            ));
        }
        
        // Wait for previous frame BEFORE reusing its semaphore
        self.sync_manager.wait_for_frame_completion(&self.context, self.current_frame)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        // Now it's safe to acquire next image using this frame's semaphore
        let (image_index, acquire_semaphore_handle) = {
            let (idx, sem) = self.sync_manager.acquire_next_image(&self.context, self.current_frame)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            (idx, sem.handle())
        };
        
        // Update UBO manager's current frame
        self.ubo_manager.set_current_frame(self.current_frame);
        
        // Start recording using the PROPER Vulkan tutorial approach
        self.command_recorder.begin_multiple_object_recording(
            &self.context,
            &self.render_pass,
            &self.swapchain_manager,
            &self.resource_manager,
            &self.pipeline_manager,
            image_index,
            self.current_frame,
            self.clear_color,
        ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        // Get the actual command buffer from the command recorder
        let command_buffer = self.command_recorder.get_active_command_buffer()
            .ok_or_else(|| crate::render::RenderError::BackendError(
                "Failed to get active command buffer after starting recording".to_string()
            ))?;
        
        // NOTE: Shared resources will be bound separately via bind_shared_resources() call
        
        // Store the recording state with the actual command buffer
        self.command_recording_state = Some(CommandRecordingState {
            command_buffer,
            frame_index: self.current_frame,
            image_index,
            acquire_semaphore_handle,
        });
        
        log::trace!("Multiple object recording session started");
        Ok(())
    }

    fn draw_indexed(&mut self, index_count: u32, _instance_count: u32, _first_index: u32, _vertex_offset: i32, _first_instance: u32) -> crate::render::BackendResult<()> {
        log::trace!("Recording indexed draw with {} indices (Vulkan tutorial pattern)", index_count);
        
        // Check if we have an active recording
        if self.command_recording_state.is_none() {
            return Err(crate::render::RenderError::BackendError(
                "No active command recording. Call begin_render_pass() first.".to_string()
            ));
        }
        
        // Create push constants for this object
        let _push_constants = super::rendering::command_recorder::PushConstants {
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
            material_color: [1.0, 1.0, 1.0, 1.0],
        };
        
        // Get frame descriptor sets for this draw
        let _frame_descriptor_sets = &[self.resource_manager.frame_descriptor_sets()[self.current_frame]];
        
        // Record object draw using the tutorial pattern (vkCmdDrawIndexed INSIDE active render pass)
        // TODO: This needs to be updated to use SharedRenderingResources
        // For now, create a dummy call that doesn't crash
        log::warn!("draw_indexed called but needs SharedRenderingResources - skipping actual draw");
        
        log::trace!("Recorded draw call inside active render pass");
        Ok(())
    }

    fn end_render_pass_and_submit(&mut self) -> crate::render::BackendResult<()> {
        log::trace!("Ending render pass and submitting commands (Vulkan tutorial pattern)");
        
        // Check if we have an active recording
        let mut recording_state = self.command_recording_state.take()
            .ok_or_else(|| crate::render::RenderError::BackendError(
                "No active command recording to submit".to_string()
            ))?;
        
        // Finish recording and get the command buffer (tutorial: end render pass and submit)
        let command_buffer = self.command_recorder.finish_multiple_object_recording()
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        recording_state.command_buffer = command_buffer;
        
        // Submit the command buffer and present using the sync manager
        let acquire_semaphore_handle = recording_state.acquire_semaphore_handle;
        let image_index = recording_state.image_index;
        let frame_index = recording_state.frame_index;
        
        // Submit command buffer to GPU and present the frame
        log::debug!("Submitting command buffer: {:?}, image: {}, frame: {}", command_buffer, image_index, frame_index);
        self.sync_manager.submit_and_present(
            &self.context,
            command_buffer,
            acquire_semaphore_handle,
            image_index,
            frame_index,
        ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        log::debug!("Command buffer submitted successfully");
        
       // log::info!("Successfully recorded and submitted {} draw calls in single command buffer: {:?}", 
       //           "multiple", command_buffer);
        
        // Update frame counter
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
        Ok(())
    }
    
    fn initialize_dynamic_rendering(&mut self, max_objects: usize) -> crate::render::BackendResult<()> {
        self.initialize_instance_renderer(max_objects)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
    }
    
    fn initialize_instance_renderer(&mut self, max_instances: usize) -> crate::render::BackendResult<()> {
        self.initialize_instance_renderer(max_instances)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
    }
    
    // === NEW CLEAN API IMPLEMENTATION ===
    
    fn create_mesh_resource(&mut self, mesh: &crate::render::Mesh, materials: &[crate::render::Material]) -> crate::render::BackendResult<crate::render::api::MeshHandle> {
        let mesh_id = self.create_and_cache_mesh(mesh, materials)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        Ok(crate::render::api::MeshHandle(mesh_id))
    }
    
    fn render_objects_with_mesh(&mut self, mesh_handle: crate::render::api::MeshHandle, transforms: &[crate::foundation::math::Mat4]) -> crate::render::BackendResult<()> {
        // Check if mesh exists and get basic info first
        let (index_count, vertex_count) = {
            let mesh_resources = self.get_cached_mesh(mesh_handle.0)
                .ok_or_else(|| crate::render::RenderError::BackendError(format!("Mesh handle {:?} not found in cache", mesh_handle)))?;
            (mesh_resources.index_count, mesh_resources.vertex_count)
        };
        
        log::trace!("Rendering {} objects with cached mesh {:?}", transforms.len(), mesh_handle);
        
        // For now, use simplified implementation that updates push constants for each object
        // TODO: Implement proper batch rendering with instancing
        for (i, transform) in transforms.iter().enumerate() {
            let model_array: [[f32; 4]; 4] = (*transform).into();
            self.set_model_matrix(model_array);
            
            // In a full implementation, this would:
            // 1. Bind the mesh's vertex/index buffers (from cached mesh_resources)
            // 2. Update push constants with the transform (already done above)
            // 3. Issue vkCmdDrawIndexed with index_count
            
            // For now, just demonstrate access to the cached resources
            log::trace!("Rendered object {} with {} indices and {} vertices", 
                       i, index_count, vertex_count);
        }
        
        Ok(())
    }
    
    fn record_dynamic_draws(&mut self, object_count: usize, mesh_id: u32) -> crate::render::BackendResult<()> {
        // TODO: Implement backend-agnostic dynamic rendering
        log::trace!("Recording {} dynamic objects for mesh {}", object_count, mesh_id);
        
        // For now, this is a placeholder implementation
        // In a proper implementation, this would use the mesh_id to determine
        // which mesh type to render and batch render object_count instances
        Ok(())
    }
    
    fn record_mesh_draw(&mut self, mesh: &crate::render::Mesh, transform: &crate::foundation::math::Mat4, material: &crate::render::Material) -> crate::render::BackendResult<()> {
        // Check if we have an active recording session
        let recording_state = self.command_recording_state.as_ref()
            .ok_or_else(|| crate::render::RenderError::BackendError(
                "No active command recording. Text rendering must happen during dynamic frame.".to_string()
            ))?;
        
        let command_buffer = recording_state.command_buffer;
        let frame_index = recording_state.frame_index;
        
        // Update mesh data
        self.update_mesh(&mesh.vertices, &mesh.indices)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        // Determine pipeline type - simplified for text (always transparent unlit)
        let pipeline_type = crate::render::PipelineType::TransparentUnlit;
        
        let pipeline_handle = self.pipeline_manager.get_pipeline_by_type(pipeline_type)
            .ok_or_else(|| crate::render::RenderError::BackendError(
                format!("Pipeline {:?} not found", pipeline_type)
            ))?;
        
        // Get descriptor sets from resource manager
        let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[frame_index];
        let material_descriptor_set = self.resource_manager.material_descriptor_sets()[0]; // Use default material set
        
        // Record draw commands
        unsafe {
            let device = self.context.raw_device();
            
            // Bind pipeline
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_handle.handle(),
            );
            
            // Set dynamic viewport and scissor
            let extent = self.context.swapchain().extent();
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind vertex and index buffers
            // Note: Pipeline expects 2 bindings (vertex + instance), but we're not using instancing for text
            // We bind the same vertex buffer to both bindings as a workaround
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[
                    self.resource_manager.vertex_buffer().handle(),
                    self.resource_manager.vertex_buffer().handle(), // Dummy binding for instance data
                ],
                &[0, 0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.resource_manager.index_buffer().handle(),
                0,
                vk::IndexType::UINT32,
            );
            
            // Bind descriptor sets
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_handle.layout(),
                0,
                &[frame_descriptor_set, material_descriptor_set],
                &[],
            );
            
            // Push constants: model matrix (64 bytes) + normal matrix (48 bytes) + material color (16 bytes) = 128 bytes
            let model_array = [
                [transform[(0, 0)], transform[(1, 0)], transform[(2, 0)], transform[(3, 0)]],
                [transform[(0, 1)], transform[(1, 1)], transform[(2, 1)], transform[(3, 1)]],
                [transform[(0, 2)], transform[(1, 2)], transform[(2, 2)], transform[(3, 2)]],
                [transform[(0, 3)], transform[(1, 3)], transform[(2, 3)], transform[(3, 3)]],
            ];
            
            // Calculate normal matrix (transpose of inverse of upper 3x3)
            // For uniform scale, this simplifies to just the rotation part
            let normal_matrix = [
                [transform[(0, 0)], transform[(1, 0)], transform[(2, 0)], 0.0],
                [transform[(0, 1)], transform[(1, 1)], transform[(2, 1)], 0.0],
                [transform[(0, 2)], transform[(1, 2)], transform[(2, 2)], 0.0],
            ];
            
            // Material color (white with full alpha for text)
            let material_color = [1.0f32, 1.0, 1.0, 1.0];
            
            // Combine all push constants (128 bytes total)
            let mut push_constant_data = Vec::with_capacity(32); // 32 floats = 128 bytes
            
            // Model matrix (16 floats = 64 bytes)
            for row in &model_array {
                push_constant_data.extend_from_slice(row);
            }
            
            // Normal matrix (12 floats = 48 bytes)
            for row in &normal_matrix {
                push_constant_data.extend_from_slice(row);
            }
            
            // Material color (4 floats = 16 bytes)
            push_constant_data.extend_from_slice(&material_color);
            
            let push_constants = bytemuck::cast_slice(&push_constant_data);
            device.cmd_push_constants(
                command_buffer,
                pipeline_handle.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT, // Both stages!
                0,
                push_constants,
            );
            
            // Draw indexed
            let index_count = self.resource_manager.index_buffer().index_count();
            device.cmd_draw_indexed(command_buffer, index_count, 1, 0, 0, 0);
        }
        
        Ok(())
    }
    
    fn record_indexed_draw(&mut self, index_start: u32, index_count: u32, transform: &crate::foundation::math::Mat4, material: &crate::render::Material) -> crate::render::BackendResult<()> {
        // Check if we have an active recording session
        let recording_state = self.command_recording_state.as_ref()
            .ok_or_else(|| crate::render::RenderError::BackendError(
                "No active command recording. Text rendering must happen during dynamic frame.".to_string()
            ))?;
        
        let command_buffer = recording_state.command_buffer;
        let frame_index = recording_state.frame_index;
        
        // Use the material's required pipeline type
        let pipeline_type = material.required_pipeline();
        
        let pipeline_handle = self.pipeline_manager.get_pipeline_by_type(pipeline_type)
            .ok_or_else(|| crate::render::RenderError::BackendError(
                format!("Pipeline {:?} not found", pipeline_type)
            ))?;
        
        // Get descriptor sets from resource manager
        let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[frame_index];
        let material_descriptor_set = self.resource_manager.material_descriptor_sets()[0];
        
        // Record draw commands (mesh data already uploaded)
        unsafe {
            let device = self.context.raw_device();
            
            // Bind pipeline
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_handle.handle(),
            );
            
            // Set dynamic viewport and scissor
            let extent = self.context.swapchain().extent();
            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(command_buffer, 0, &[viewport]);
            
            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            device.cmd_set_scissor(command_buffer, 0, &[scissor]);
            
            // Bind vertex and index buffers
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                &[
                    self.resource_manager.vertex_buffer().handle(),
                    self.resource_manager.vertex_buffer().handle(),
                ],
                &[0, 0],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.resource_manager.index_buffer().handle(),
                0,
                vk::IndexType::UINT32,
            );
            
            // Bind descriptor sets
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_handle.layout(),
                0,
                &[frame_descriptor_set, material_descriptor_set],
                &[],
            );
            
            // Push constants
            let model_array = [
                [transform[(0, 0)], transform[(1, 0)], transform[(2, 0)], transform[(3, 0)]],
                [transform[(0, 1)], transform[(1, 1)], transform[(2, 1)], transform[(3, 1)]],
                [transform[(0, 2)], transform[(1, 2)], transform[(2, 2)], transform[(3, 2)]],
                [transform[(0, 3)], transform[(1, 3)], transform[(2, 3)], transform[(3, 3)]],
            ];
            
            let normal_matrix = [
                [transform[(0, 0)], transform[(1, 0)], transform[(2, 0)], 0.0],
                [transform[(0, 1)], transform[(1, 1)], transform[(2, 1)], 0.0],
                [transform[(0, 2)], transform[(1, 2)], transform[(2, 2)], 0.0],
            ];
            
            // Use material's base color
            let color = material.get_base_color_array();
            let material_color = [color[0], color[1], color[2], color[3]];
            
            let mut push_constant_data = Vec::with_capacity(32);
            for row in &model_array {
                push_constant_data.extend_from_slice(row);
            }
            for row in &normal_matrix {
                push_constant_data.extend_from_slice(row);
            }
            push_constant_data.extend_from_slice(&material_color);
            
            let push_constants = bytemuck::cast_slice(&push_constant_data);
            device.cmd_push_constants(
                command_buffer,
                pipeline_handle.layout(),
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                0,
                push_constants,
            );
            
            // Draw indexed with specific range
            println!("DRAW CMD: index_start={}, index_count={}", index_start, index_count);
            device.cmd_draw_indexed(command_buffer, index_count, 1, index_start, 0, 0);
        }
        
        Ok(())
    }
    
    fn create_object_resources(&mut self, _material: &crate::render::Material) -> crate::render::BackendResult<crate::render::api::ObjectResourceHandle> {
        let object_id = self.next_object_id;
        self.next_object_id += 1;
        
        // Create per-frame uniform buffers and descriptor sets
        let mut uniform_buffers = Vec::new();
        let mut uniform_mapped = Vec::new();
        
        for _frame_index in 0..self.max_frames_in_flight {
            // Create uniform buffer for this frame
            let buffer_size = std::mem::size_of::<crate::render::ObjectUBO>() as vk::DeviceSize;
            
            let buffer = Buffer::new(
                self.context.raw_device(),
                self.context.instance().clone(),
                self.context.physical_device().device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            
            // Map memory for updates
            let mapped = buffer.map_memory()
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))? as *mut u8;
            
            uniform_buffers.push(buffer);
            uniform_mapped.push(mapped);
        }
        
        // Create descriptor sets (one per frame-in-flight)
        let layouts = vec![self.get_object_descriptor_set_layout(); self.max_frames_in_flight];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.get_descriptor_pool())
            .set_layouts(&layouts);

        let descriptor_sets = unsafe {
            self.context.raw_device().allocate_descriptor_sets(&alloc_info)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?
        };

        // Update descriptor sets to point to uniform buffers
        for (frame_index, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(uniform_buffers[frame_index].handle())
                .offset(0)
                .range(std::mem::size_of::<crate::render::ObjectUBO>() as vk::DeviceSize);

            let descriptor_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0) // Binding 0 for object UBO
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info));

            unsafe {
                self.context.raw_device().update_descriptor_sets(
                    &[descriptor_write.build()],
                    &[],
                );
            }
        }
        
        let object_resources = ObjectResources {
            uniform_buffers,
            uniform_mapped,
            descriptor_sets,
        };
        
        self.object_resources.insert(object_id, object_resources);
        
        Ok(crate::render::api::ObjectResourceHandle(object_id))
    }
    
    fn update_object_uniforms(&mut self, handle: crate::render::api::ObjectResourceHandle, model_matrix: Mat4, _material_data: &[f32; 4]) -> crate::render::BackendResult<()> {
        let object_resources = self.object_resources.get(&handle.0)
            .ok_or_else(|| crate::render::RenderError::BackendError(format!("Object resource handle {:?} not found", handle)))?;
        
        // Update uniform buffer for current frame
        let frame_index = self.current_frame;
        if frame_index < object_resources.uniform_buffers.len() {
            let object_ubo = crate::render::ObjectUBO {
                model_matrix: model_matrix.into(),
                normal_matrix: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], // Identity for now
                _padding: [0.0; 3],
            };
            
            unsafe {
                let mapped_ptr = object_resources.uniform_mapped[frame_index] as *mut crate::render::ObjectUBO;
                std::ptr::copy_nonoverlapping(&object_ubo, mapped_ptr, 1);
            }
        }
        
        Ok(())
    }
}

impl VulkanRenderer {
    /// Record dynamic draws using a dedicated InstanceRenderer (new architecture)
    ///
    /// This method provides the Vulkan-specific parameters needed for dedicated InstanceRenderer
    /// per mesh pool, preventing state corruption between different mesh types.
    /// 
    /// MODULAR PIPELINE VERSION: Groups objects by required pipeline type and renders each group separately
    pub fn record_dynamic_draws_with_dedicated_renderer(&mut self,
                                                       instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
                                                       dynamic_objects: &std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData>,
                                                       shared_resources: &crate::render::SharedRenderingResources,
                                                       pool_material_descriptor_set: ash::vk::DescriptorSet,
                                                       camera_position: crate::foundation::math::Vec3,
                                                       camera_target: crate::foundation::math::Vec3) 
                                                       -> crate::render::BackendResult<()> {
        use std::collections::HashMap;
        use crate::render::resources::materials::PipelineType;
        
        if let Some(ref state) = self.command_recording_state {
            // Group objects by their required pipeline type
            let mut pipeline_groups: HashMap<PipelineType, Vec<(crate::render::systems::dynamic::DynamicObjectHandle, &crate::render::systems::dynamic::DynamicRenderData)>> = HashMap::new();
            
            for (handle, render_data) in dynamic_objects.iter() {
                let pipeline_type = render_data.material.required_pipeline();
                pipeline_groups.entry(pipeline_type)
                    .or_insert_with(Vec::new)
                    .push((*handle, render_data));
            }
            
            log::debug!("Rendering {} objects grouped into {} pipeline types", dynamic_objects.len(), pipeline_groups.len());
            
            // Get frame descriptor set and use pool-specific material descriptor set
            let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[self.current_frame];
            let material_descriptor_set = pool_material_descriptor_set;  // Use pool's descriptor set with its texture
            
            // Set command buffer once for all pipeline groups
            instance_renderer.set_active_command_buffer(state.command_buffer, self.current_frame);
            
            // CRITICAL FIX: Upload ALL objects ONCE to contiguous buffer regions
            // Then draw each pipeline group from its specific region
            instance_renderer.upload_instance_data(dynamic_objects)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            
            // Track which objects are at which indices in the uploaded buffer
            let mut object_indices: HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32> = HashMap::new();
            let mut current_index = 0u32;
            for (handle, _) in dynamic_objects.iter() {
                object_indices.insert(*handle, current_index);
                current_index += 1;
            }
            
            // CRITICAL: Render in correct order for transparency
            // 1. Render opaque objects front-to-back (performance - early Z rejection)
            // 2. Then render transparent objects back-to-front (correctness - proper alpha blending)
            
            let opaque_pipeline_types = [PipelineType::StandardPBR, PipelineType::Unlit];
            let transparent_pipeline_types = [PipelineType::TransparentPBR, PipelineType::TransparentUnlit];
            
            // Calculate camera forward direction for view-space Z sorting (industry standard)
            let camera_forward = (camera_target - camera_position).normalize();
            
            // Render opaque objects front-to-back
            for pipeline_type in &opaque_pipeline_types {
                if let Some(mut objects) = pipeline_groups.remove(pipeline_type) {
                    // Sort front-to-back using view-space Z (dot product with camera forward)
                    // This is more accurate than radial distance for large/elongated objects
                    objects.sort_by(|a, b| {
                        let to_object_a = a.1.position - camera_position;
                        let to_object_b = b.1.position - camera_position;
                        let depth_a = to_object_a.dot(&camera_forward); // View-space Z coordinate
                        let depth_b = to_object_b.dot(&camera_forward);
                        depth_a.partial_cmp(&depth_b).unwrap_or(std::cmp::Ordering::Equal)
                    });
                    
                    self.render_pipeline_group(
                        instance_renderer,
                        shared_resources,
                        frame_descriptor_set,
                        material_descriptor_set,
                        *pipeline_type,
                        &objects,
                        &object_indices
                    )?;
                }
            }
            
            // Render transparent objects back-to-front
            // CRITICAL: Merge all transparent pipeline types and sort together for correct depth ordering
            let mut all_transparent_objects = Vec::new();
            for pipeline_type in &transparent_pipeline_types {
                if let Some(objects) = pipeline_groups.remove(pipeline_type) {
                    all_transparent_objects.extend(objects.into_iter().map(|obj| (*pipeline_type, obj)));
                }
            }
            
            // Sort ALL transparent objects back-to-front using view-space Z
            // This ensures correct alpha blending across different material types
            all_transparent_objects.sort_by(|a, b| {
                let to_object_a = a.1.1.position - camera_position;
                let to_object_b = b.1.1.position - camera_position;
                let depth_a = to_object_a.dot(&camera_forward);
                let depth_b = to_object_b.dot(&camera_forward);
                
                // Log depth values for debugging
                log::trace!("Comparing transparent objects: depth_a={:.2} (pipeline={:?}), depth_b={:.2} (pipeline={:?})", 
                           depth_a, a.0, depth_b, b.0);
                
                depth_b.partial_cmp(&depth_a).unwrap_or(std::cmp::Ordering::Equal) // Reversed for back-to-front
            });
            
            log::debug!("Sorted {} transparent objects for rendering", all_transparent_objects.len());
            
            // Render sorted transparent objects (may require pipeline switches)
            let mut current_pipeline: Option<PipelineType> = None;
            let mut batch_objects = Vec::new();
            
            for (pipeline_type, object) in all_transparent_objects {
                // If pipeline changes, render the current batch first
                if let Some(current) = current_pipeline {
                    if current != pipeline_type {
                        self.render_pipeline_group(
                            instance_renderer,
                            shared_resources,
                            frame_descriptor_set,
                            material_descriptor_set,
                            current,
                            &batch_objects,
                            &object_indices
                        )?;
                        batch_objects.clear();
                    }
                }
                
                current_pipeline = Some(pipeline_type);
                batch_objects.push(object);
            }
            
            // Render final batch
            if let Some(pipeline_type) = current_pipeline {
                if !batch_objects.is_empty() {
                    self.render_pipeline_group(
                        instance_renderer,
                        shared_resources,
                        frame_descriptor_set,
                        material_descriptor_set,
                        pipeline_type,
                        &batch_objects,
                        &object_indices
                    )?;
                }
            }
            
            log::trace!("Recorded dynamic draws for {} total objects across {} pipelines", dynamic_objects.len(), pipeline_groups.len());
            Ok(())
        } else {
            Err(crate::render::RenderError::BackendError("No active command recording".to_string()))
        }
    }
    
    /// Upload instance data for a pool without rendering (Phase 1 of split architecture)
    ///
    /// This uploads ALL objects to the GPU buffer and returns the index mapping.
    /// The indices can then be used for multiple draw calls without re-uploading.
    pub fn upload_pool_instance_data(
        &mut self,
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        dynamic_objects: &std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData>,
    ) -> crate::render::BackendResult<std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32>> {
        if let Some(ref state) = self.command_recording_state {
            // Set command buffer for this pool
            instance_renderer.set_active_command_buffer(state.command_buffer, self.current_frame);
            
            // Upload ALL objects ONCE
            instance_renderer.upload_instance_data(dynamic_objects)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            
            // Build and return index map
            let mut object_indices = std::collections::HashMap::new();
            let mut current_index = 0u32;
            for (handle, _) in dynamic_objects.iter() {
                object_indices.insert(*handle, current_index);
                current_index += 1;
            }
            
            Ok(object_indices)
        } else {
            Err(crate::render::RenderError::BackendError("No active command recording".to_string()))
        }
    }
    
    /// Render a subset of already-uploaded objects (Phase 2 of split architecture)
    ///
    /// This renders specific objects using indices from upload_pool_instance_data().
    /// Does NOT upload data - assumes upload_pool_instance_data() was already called.
    pub fn render_uploaded_objects_subset(
        &mut self,
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        shared_resources: &crate::render::SharedRenderingResources,
        material_descriptor_set: vk::DescriptorSet,
        objects: &[(crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData)],
        object_indices: &std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32>,
    ) -> crate::render::BackendResult<()> {
        use std::collections::HashMap;
        use crate::render::resources::materials::PipelineType;
        
        if self.command_recording_state.is_none() {
            return Err(crate::render::RenderError::BackendError("No active command recording".to_string()));
        }
        
        // Get frame descriptor set
        let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[self.current_frame];
        
        // Group objects by pipeline type
        let mut pipeline_groups: HashMap<PipelineType, Vec<(crate::render::systems::dynamic::DynamicObjectHandle, &crate::render::systems::dynamic::DynamicRenderData)>> = HashMap::new();
        
        for (handle, render_data) in objects.iter() {
            let pipeline_type = render_data.material.required_pipeline();
            pipeline_groups.entry(pipeline_type)
                .or_insert_with(Vec::new)
                .push((*handle, render_data));
        }
        
        // Render each pipeline group
        for (pipeline_type, group_objects) in pipeline_groups {
            self.render_pipeline_group(
                instance_renderer,
                shared_resources,
                frame_descriptor_set,
                material_descriptor_set,
                pipeline_type,
                &group_objects,
                object_indices
            )?;
        }
        
        Ok(())
    }
    
    /// Helper method to render a single pipeline group
    fn render_pipeline_group(
        &mut self,
        instance_renderer: &mut crate::render::systems::dynamic::InstanceRenderer,
        shared_resources: &crate::render::SharedRenderingResources,
        frame_descriptor_set: vk::DescriptorSet,
        material_descriptor_set: vk::DescriptorSet,
        pipeline_type: crate::render::resources::materials::PipelineType,
        objects: &[(crate::render::systems::dynamic::DynamicObjectHandle, &crate::render::systems::dynamic::DynamicRenderData)],
        object_indices: &std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, u32>
    ) -> crate::render::BackendResult<()> {
        // Get the pipeline for this material type
        let pipeline = self.pipeline_manager.get_pipeline_by_type(pipeline_type)
            .ok_or_else(|| crate::render::RenderError::BackendError(
                format!("No pipeline found for type: {:?}", pipeline_type)
            ))?;
        
        log::trace!("Rendering {} objects with pipeline {:?}", objects.len(), pipeline_type);
        
        // Collect instance indices for this pipeline group
        let instance_indices: Vec<u32> = objects.iter()
            .filter_map(|(handle, _)| object_indices.get(handle).copied())
            .collect();
        
        // Record draw commands with the specific pipeline and instance range
        instance_renderer.record_instanced_draw_with_explicit_pipeline_and_range(
            shared_resources,
            &self.context,
            frame_descriptor_set,
            material_descriptor_set,
            pipeline.handle(),
            pipeline.layout(),
            &instance_indices
        ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        Ok(())
    }
    
    // Getter methods for GameObject creation
    /// Get access to the VulkanContext for resource creation (INTERNAL)
    pub(crate) fn get_context(&self) -> &VulkanContext {
        &self.context
    }
    
    /// Get the descriptor pool for creating descriptor sets (INTERNAL)
    pub(crate) fn get_descriptor_pool(&self) -> vk::DescriptorPool {
        self.resource_manager.get_descriptor_pool()
    }
    
    /// Get the object descriptor set layout for GameObject uniform buffers (INTERNAL)
    pub(crate) fn get_object_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        // For now, use the material descriptor set layout which has the same structure
        // TODO: Add dedicated object descriptor set layout when GameObject system is fully integrated
        self.ubo_manager.material_descriptor_set_layout()
    }
    
    /// Get max frames in flight
    pub fn get_max_frames_in_flight(&self) -> usize {
        self.max_frames_in_flight
    }
    
    /// Get the default material descriptor set
    ///
    /// Returns the first material descriptor set which serves as the default.
    /// This is used for pools that don't have their own descriptor sets yet.
    pub fn get_default_material_descriptor_set(&self) -> vk::DescriptorSet {
        self.resource_manager.material_descriptor_sets()[0]
    }
    
    /// Create a material descriptor set with a specific texture
    ///
    /// Allocates a new descriptor set and binds the specified texture to it.
    /// All other texture bindings use default white textures.
    ///
    /// # Arguments
    /// * `texture_handle` - The texture to bind as base color (binding 1)
    ///
    /// # Returns
    /// A descriptor set configured with the specified texture
    pub fn create_material_descriptor_set_with_texture(
        &mut self,
        texture_handle: crate::render::resources::materials::TextureHandle,
    ) -> Result<vk::DescriptorSet, Box<dyn std::error::Error>> {
        // Allocate a new descriptor set
        let layouts = vec![self.ubo_manager.material_descriptor_set_layout()];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.resource_manager.get_descriptor_pool())
            .set_layouts(&layouts);

        let descriptor_sets = unsafe {
            self.context.raw_device().allocate_descriptor_sets(&alloc_info)
                .map_err(|e| format!("Failed to allocate descriptor set: {:?}", e))?
        };
        
        let descriptor_set = descriptor_sets[0];
        
        // Get the texture from resource manager
        let texture = self.resource_manager.get_loaded_texture(texture_handle.0 as usize)
            .ok_or("Texture not found in resource manager")?;
        
        // Material UBO buffer info - use actual size of StandardMaterialUBO
        use crate::render::resources::materials::StandardMaterialUBO;
        let material_ubo_size = std::mem::size_of::<StandardMaterialUBO>() as vk::DeviceSize;
        let material_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(self.ubo_manager.material_ubo_buffer_handle())
            .offset(0)
            .range(material_ubo_size)
            .build();
        
        // Base color texture - use the specified texture
        let base_color_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(texture.image_view())
            .sampler(texture.sampler())
            .build();
        
        // All other textures use default white
        let default_image_info = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(self.resource_manager.default_white_texture().image_view())
            .sampler(self.resource_manager.default_white_texture().sampler())
            .build();
        
        let descriptor_writes = vec![
            // Binding 0: Material UBO
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&[material_buffer_info])
                .build(),
            // Binding 1: Base color texture (custom texture)
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[base_color_image_info])
                .build(),
            // Binding 2: Normal map (default)
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[default_image_info])
                .build(),
            // Binding 3: Metallic/roughness (default)
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[default_image_info])
                .build(),
            // Binding 4: AO (default)
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(4)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[default_image_info])
                .build(),
            // Binding 5: Emission (default)
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(5)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[default_image_info])
                .build(),
            // Binding 6: Opacity (default)
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(6)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[default_image_info])
                .build(),
        ];
        
        unsafe {
            self.context.raw_device().update_descriptor_sets(&descriptor_writes, &[]);
        }
        
        log::info!("Created material descriptor set {:?} with texture {:?}", descriptor_set, texture_handle);
        Ok(descriptor_set)
    }
    
    // FIXME: LEGACY - Remove after modular pipeline system is complete
    // This assumes a single active pipeline; new system selects pipeline per-material
    /// Get the current graphics pipeline (INTERNAL) (LEGACY - will be removed)
    pub(crate) fn get_graphics_pipeline(&self) -> vk::Pipeline {
        if let Some(pipeline) = self.pipeline_manager.get_active_pipeline() {
            pipeline.handle()
        } else {
            // Fallback to a default pipeline or return error
            // For now, return a null handle
            vk::Pipeline::null()
        }
    }
    
    // FIXME: LEGACY - Remove after modular pipeline system is complete
    /// Get the current pipeline layout (INTERNAL) (LEGACY - will be removed)
    pub(crate) fn get_pipeline_layout(&self) -> vk::PipelineLayout {
        if let Some(pipeline) = self.pipeline_manager.get_active_pipeline() {
            pipeline.layout()
        } else {
            // Fallback to a default pipeline layout or return error
            // For now, return a null handle
            vk::PipelineLayout::null()
        }
    }
    
    /// Get the frame descriptor set layout (INTERNAL)
    pub(crate) fn get_frame_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.ubo_manager.frame_descriptor_set_layout()
    }
    
    /// Get the render pass handle (INTERNAL)
    pub(crate) fn get_render_pass(&self) -> vk::RenderPass {
        self.render_pass.handle()
    }

    /// Upload texture from image data to GPU
    ///
    /// Creates a Vulkan texture from image data and returns a handle for use with materials.
    /// This method provides access to texture creation for procedurally generated textures
    /// like font atlases.
    ///
    /// # Arguments
    /// * `image_data` - Image data (RGBA format)
    /// * `texture_type` - Type of texture (BaseColor, Normal, etc.)
    ///
    /// # Returns
    /// TextureHandle for the uploaded texture
    pub fn upload_texture_from_image_data(
        &mut self,
        image_data: crate::assets::ImageData,
        texture_type: crate::render::resources::materials::TextureType,
    ) -> Result<crate::render::resources::materials::TextureHandle, Box<dyn std::error::Error>> {
        use std::sync::Arc;
        use crate::render::backends::vulkan::Texture;
        
        // Create Vulkan texture from image data
        let texture = Texture::from_image_data(
            Arc::new(self.context.raw_device().clone()),
            Arc::new(self.context.instance().clone()),
            self.context.physical_device().device,
            self.resource_manager.command_pool_handle(),
            self.context.graphics_queue(),
            &image_data,
        )?;
        
        // Store texture in resource manager and get index
        let texture_index = self.resource_manager.add_loaded_texture(texture);
        
        // Create a TextureHandle (using index as handle ID)
        // Note: This is a simplified approach - ideally TextureManager would be integrated
        let handle = crate::render::resources::materials::TextureHandle(texture_index as u32);
        
        log::debug!("Uploaded texture {:?} (type: {:?}, index: {})", handle, texture_type, texture_index);
        Ok(handle)
    }
    
    /// Record a minimal command buffer for instanced rendering only
    ///
    /// Creates an empty command buffer that does nothing but clear the screen.
    /// The instance renderer handles all actual rendering internally.
    ///
    /// # Arguments
    /// * `image_index` - Swapchain image index
    /// * `frame_index` - Current frame index
    ///
    /// # Returns
    /// Empty command buffer ready for submission
    fn record_minimal_command_buffer(
        &mut self,
        image_index: u32,
        frame_index: usize,
    ) -> VulkanResult<vk::CommandBuffer> {
    // FIXME: Create minimal command buffer using legacy recorder but without binding any pipelines
        self.command_recorder.free_command_buffer(frame_index);
        
        self.command_recorder.begin_multiple_object_recording(
            &self.context,
            &self.render_pass,
            &self.swapchain_manager,
            &self.resource_manager,
            &self.pipeline_manager,
            image_index,
            frame_index,
            self.clear_color,
        )?;

        // Finish immediately - this just creates a command buffer with render pass but no draws
        let command_buffer = self.command_recorder.finish_multiple_object_recording()?;
        Ok(command_buffer)
    }
    
    /// Initialize instanced rendering system for dynamic objects
    ///
    /// Creates and initializes the InstanceRenderer for efficient batch rendering
    /// of dynamic objects using Vulkan instanced rendering (vkCmdDrawInstanced).
    ///
    /// # Arguments
    /// * `max_instances` - Maximum number of instances that can be rendered in a single draw call
    ///
    /// # Returns  
    /// Result indicating successful initialization or Vulkan error
    pub fn initialize_instance_renderer(&mut self, max_instances: usize) -> VulkanResult<()> {
        log::info!("Initializing instance renderer with {} max instances", max_instances);
        
        let instance_renderer = crate::render::systems::dynamic::InstanceRenderer::new(&self.context, max_instances)?;
        self.instance_renderer = Some(instance_renderer);
        
        log::debug!("Instance renderer initialized successfully");
        Ok(())
    }
    
    /// Record dynamic object draws during active command recording
    ///
    /// This method integrates with the current Vulkan command recording workflow
    /// to render dynamic objects using efficient instanced rendering. It must be
    /// called during active command recording (after begin_render_pass).
    ///
    /// # Arguments  
    /// * `dynamic_objects` - HashMap of active dynamic objects to render
    /// * `shared_resources` - Shared rendering resources (mesh, textures, etc.)
    ///
    /// # Returns
    /// Result indicating successful command recording or Vulkan error
    pub fn record_dynamic_objects(
        &mut self, 
        dynamic_objects: &std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData>,
        shared_resources: &crate::render::SharedRenderingResources
    ) -> VulkanResult<()> {
        
        if let (Some(ref mut instance_renderer), Some(ref state)) = 
            (&mut self.instance_renderer, &self.command_recording_state) {
            
            // Set active command buffer directly (don't call begin_frame during recording)
            instance_renderer.set_active_command_buffer(state.command_buffer, self.current_frame);
            
            // Upload instance data from active dynamic objects
            instance_renderer.upload_instance_data(dynamic_objects)?;
            
            // Get frame descriptor set for this frame
            let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[self.current_frame];
            
            // Get material descriptor set (use first one as default for all objects)
            let material_descriptor_set = self.resource_manager.material_descriptor_sets()[0];
            
            // Record instanced draw commands
            instance_renderer.record_instanced_draw(shared_resources, &self.context, frame_descriptor_set, material_descriptor_set)?;
            
            log::trace!("Recorded dynamic draws for {} objects", dynamic_objects.len());
            Ok(())
        } else {
            Err(VulkanError::InvalidOperation {
                reason: "Instance renderer not initialized or no active command recording".to_string()
            })
        }
    }
    
    /// Record dynamic object rendering using instance renderer
    ///
    /// Public interface to render dynamic objects using the internal instance renderer.
    /// This is called from the GraphicsEngine's record_dynamic_draws method.
    ///
    /// # Arguments
    /// * `dynamic_objects` - HashMap of active dynamic objects to render
    /// * `shared_resources` - Shared rendering resources (mesh, textures, materials)
    ///
    /// # Returns
    /// Result indicating successful rendering
    pub fn record_dynamic_objects_public(
        &mut self,
        dynamic_objects: &std::collections::HashMap<crate::render::systems::dynamic::DynamicObjectHandle, crate::render::systems::dynamic::DynamicRenderData>,
        shared_resources: &crate::render::SharedRenderingResources,
    ) -> VulkanResult<()> {
        if let Some(ref mut instance_renderer) = self.instance_renderer {
            log::trace!("Recording dynamic objects using instance renderer");
            
            // Upload instance data from active dynamic objects
            instance_renderer.upload_instance_data(dynamic_objects)?;
            
            // Get frame descriptor set for this frame
            let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[self.current_frame];
            
            // Get material descriptor set (use first one as default for all objects)
            let material_descriptor_set = self.resource_manager.material_descriptor_sets()[0];
            
            // Record instanced draw commands
            instance_renderer.record_instanced_draw(shared_resources, &self.context, frame_descriptor_set, material_descriptor_set)?;
            
            log::trace!("Recorded dynamic draws for {} objects", dynamic_objects.len());
            Ok(())
        } else {
            Err(VulkanError::InvalidOperation {
                reason: "Instance renderer not initialized".to_string()
            })
        }
    }
}
