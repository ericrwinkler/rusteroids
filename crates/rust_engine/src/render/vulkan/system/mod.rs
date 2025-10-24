//! Modular Vulkan renderer implementation
//! 
//! This module breaks down the monolithic VulkanRenderer into focused, 
//! single-responsibility components for better maintainability.

pub mod resource_manager;
pub mod ubo_manager;
pub mod command_recorder;
pub mod sync_manager;
pub mod swapchain_manager;

pub use resource_manager::ResourceManager;
pub use ubo_manager::UboManager;
pub use command_recorder::CommandRecorder;
pub use sync_manager::SyncManager;
pub use swapchain_manager::SwapchainManager;

use crate::render::vulkan::*;
use crate::render::vulkan::buffer::Buffer;
use crate::render::mesh::Vertex;
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
    instance_renderer: Option<crate::render::dynamic::InstanceRenderer>,
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
        
    // FIXME: LEGACY: Create pipeline manager (disabled for unified instanced rendering)
        let pipeline_manager = crate::render::PipelineManager::new();
        
        // Create specialized managers
        let mut resource_manager = ResourceManager::new(&context, config.max_frames_in_flight)?;
        let ubo_manager = UboManager::new(&context, &resource_manager, config.max_frames_in_flight)?;
        let mut command_recorder = CommandRecorder::new(&context, &context.swapchain(), config.max_frames_in_flight)?;
        let sync_manager = SyncManager::new(&context, &context.swapchain(), config.max_frames_in_flight)?;
        let swapchain_manager = SwapchainManager::new(&context, &render_pass)?;
        
        // Initialize command recorder
        command_recorder.initialize(config.max_frames_in_flight);
        
    // FIXME: LEGACY PIPELINE SYSTEM DISABLED - Using unified instanced rendering only
        // pipeline_manager.initialize_standard_pipelines(
        //     &context,
        //     render_pass.handle(),
        //     &[ubo_manager.frame_descriptor_set_layout(), ubo_manager.material_descriptor_set_layout()],
        // )?;
        
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
    
    /// Set material (delegates to UBO manager and pipeline manager)
    pub fn set_material(&mut self, material: &crate::render::Material) -> VulkanResult<()> {
        // Update material UBO
        self.ubo_manager.update_material(&material)?;
        
        // Set appropriate pipeline
        self.pipeline_manager.set_active_pipeline(&material.material_type);
        
        Ok(())
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
    
    fn set_multi_light_environment(&mut self, multi_light_env: &crate::render::lighting::MultiLightEnvironment) {
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
        if let Some(vulkan_window) = backend_window.downcast_mut::<crate::render::vulkan::Window>() {
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
        let _push_constants = command_recorder::PushConstants {
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
    
    fn create_mesh_resource(&mut self, mesh: &crate::render::Mesh, materials: &[crate::render::Material]) -> crate::render::BackendResult<crate::render::backend::MeshHandle> {
        let mesh_id = self.create_and_cache_mesh(mesh, materials)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        Ok(crate::render::backend::MeshHandle(mesh_id))
    }
    
    fn render_objects_with_mesh(&mut self, mesh_handle: crate::render::backend::MeshHandle, transforms: &[crate::foundation::math::Mat4]) -> crate::render::BackendResult<()> {
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
    
    fn create_object_resources(&mut self, _material: &crate::render::Material) -> crate::render::BackendResult<crate::render::backend::ObjectResourceHandle> {
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
        
        Ok(crate::render::backend::ObjectResourceHandle(object_id))
    }
    
    fn update_object_uniforms(&mut self, handle: crate::render::backend::ObjectResourceHandle, model_matrix: Mat4, _material_data: &[f32; 4]) -> crate::render::BackendResult<()> {
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
    pub fn record_dynamic_draws_with_dedicated_renderer(&mut self,
                                                       instance_renderer: &mut crate::render::dynamic::InstanceRenderer,
                                                       dynamic_objects: &std::collections::HashMap<crate::render::dynamic::DynamicObjectHandle, crate::render::dynamic::DynamicRenderData>,
                                                       shared_resources: &crate::render::SharedRenderingResources) 
                                                       -> crate::render::BackendResult<()> {
        if let Some(ref state) = self.command_recording_state {
            // Use the provided dedicated InstanceRenderer instead of the shared one
            instance_renderer.set_active_command_buffer(state.command_buffer, self.current_frame);
            
            // Upload instance data from active dynamic objects
            instance_renderer.upload_instance_data(dynamic_objects)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            
            // Get frame descriptor set for this frame
            let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[self.current_frame];
            
            // Get material descriptor set (use first one as default for all objects)
            let material_descriptor_set = self.resource_manager.material_descriptor_sets()[0];
            
            // Record instanced draw commands
            instance_renderer.record_instanced_draw(shared_resources, &self.context, frame_descriptor_set, material_descriptor_set)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            
            log::trace!("Recorded dynamic draws for {} objects with dedicated renderer", dynamic_objects.len());
            Ok(())
        } else {
            Err(crate::render::RenderError::BackendError("No active command recording".to_string()))
        }
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
    
    /// Get the current graphics pipeline (INTERNAL)
    pub(crate) fn get_graphics_pipeline(&self) -> vk::Pipeline {
        if let Some(pipeline) = self.pipeline_manager.get_active_pipeline() {
            pipeline.handle()
        } else {
            // Fallback to a default pipeline or return error
            // For now, return a null handle
            vk::Pipeline::null()
        }
    }
    
    /// Get the current pipeline layout (INTERNAL)
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
        
        let instance_renderer = crate::render::dynamic::InstanceRenderer::new(&self.context, max_instances)?;
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
        dynamic_objects: &std::collections::HashMap<crate::render::dynamic::DynamicObjectHandle, crate::render::dynamic::DynamicRenderData>,
        shared_resources: &crate::render::shared_resources::SharedRenderingResources
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
        dynamic_objects: &std::collections::HashMap<crate::render::dynamic::DynamicObjectHandle, crate::render::dynamic::DynamicRenderData>,
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
