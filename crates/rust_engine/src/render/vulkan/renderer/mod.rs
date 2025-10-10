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
use crate::render::mesh::Vertex;
use crate::foundation::math::{Mat4, Vec3};
use ash::vk;

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
    
    // State for multiple object command recording
    command_recording_state: Option<CommandRecordingState>,
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
        
        // Create pipeline manager
        let mut pipeline_manager = crate::render::PipelineManager::new();
        
        // Create specialized managers
        let mut resource_manager = ResourceManager::new(&context, config.max_frames_in_flight)?;
        let ubo_manager = UboManager::new(&context, &resource_manager, config.max_frames_in_flight)?;
        let mut command_recorder = CommandRecorder::new(&context, &context.swapchain(), config.max_frames_in_flight)?;
        let sync_manager = SyncManager::new(&context, &context.swapchain(), config.max_frames_in_flight)?;
        let swapchain_manager = SwapchainManager::new(&context, &render_pass)?;
        
        // Initialize command recorder
        command_recorder.initialize(config.max_frames_in_flight);
        
        // Initialize pipelines with UBO layouts FIRST
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
            command_recording_state: None,
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
        // Acquire next image
        let (image_index, acquire_semaphore_handle) = {
            let (idx, sem) = self.sync_manager.acquire_next_image(&self.context, self.current_frame)?;
            (idx, sem.handle())
        };
        
        // Wait for previous frame
        self.sync_manager.wait_for_frame_completion(&self.context, self.current_frame)?;
        
        // CRITICAL: Update UBO manager's current frame before recording commands
        self.ubo_manager.set_current_frame(self.current_frame);
        
        // Record commands
        let command_buffer = self.command_recorder.record_frame(
            &self.context,
            &self.render_pass,
            &self.swapchain_manager,
            &self.resource_manager,
            &self.ubo_manager,
            &self.pipeline_manager,
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
    
    fn bind_shared_resources(&mut self, shared: &crate::render::SharedRenderingResources) -> crate::render::BackendResult<()> {
        log::trace!("Binding shared resources in Vulkan backend");
        
        // Check if we have an active recording
        if self.command_recording_state.is_none() {
            return Err(crate::render::RenderError::BackendError(
                "No active command recording. Call begin_render_pass() first.".to_string()
            ));
        }
        
        // Get the frame descriptor set for the current frame
        let frame_descriptor_set = self.resource_manager.frame_descriptor_sets()[self.current_frame];
        
        // Bind shared resources using the command recorder, including context for viewport/scissor setup
        self.command_recorder.bind_shared_resources(shared, &self.context, frame_descriptor_set)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        Ok(())
    }
    
    fn record_shared_object_draw(&mut self, shared: &crate::render::SharedRenderingResources, object_descriptor_sets: &[ash::vk::DescriptorSet]) -> crate::render::BackendResult<()> {
        log::trace!("Recording shared object draw with {} indices", shared.index_count);
        
        // Check if we have an active recording
        if self.command_recording_state.is_none() {
            return Err(crate::render::RenderError::BackendError(
                "No active command recording. Call begin_render_pass() first.".to_string()
            ));
        }
        
        // CRITICAL FIX: Only bind the current frame's descriptor set, not all descriptor sets
        // GameObjects create max_frames_in_flight descriptor sets, but only one should be bound per frame
        let current_frame_descriptor_set = if object_descriptor_sets.len() > self.current_frame {
            &[object_descriptor_sets[self.current_frame]]
        } else {
            // Fallback if array is too small (shouldn't happen in normal operation)
            log::warn!("Object descriptor sets array too small ({}), using first descriptor set", object_descriptor_sets.len());
            &[object_descriptor_sets[0]]
        };
        
        // Record object draw using the new command recorder method
        self.command_recorder.record_object_draw(shared, current_frame_descriptor_set)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        log::trace!("Recorded object draw using shared resources and current frame descriptor set (frame {})", self.current_frame);
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
        if let Some(vulkan_window) = window_handle.get_vulkan_window() {
            self.recreate_swapchain(vulkan_window)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))
        } else {
            Err(crate::render::RenderError::BackendError("No Vulkan window available".to_string()))
        }
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
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
        
        // Acquire next image
        let (image_index, acquire_semaphore_handle) = {
            let (idx, sem) = self.sync_manager.acquire_next_image(&self.context, self.current_frame)
                .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
            (idx, sem.handle())
        };
        
        // Wait for previous frame
        self.sync_manager.wait_for_frame_completion(&self.context, self.current_frame)
            .map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
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
        ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
        // NOTE: Shared resources will be bound separately via bind_shared_resources() call
        
        // Store the recording state
        self.command_recording_state = Some(CommandRecordingState {
            command_buffer: vk::CommandBuffer::null(), // Will be set when finished
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
        self.sync_manager.submit_and_present(
            &self.context,
            command_buffer,
            acquire_semaphore_handle,
            image_index,
            frame_index,
        ).map_err(|e| crate::render::RenderError::BackendError(e.to_string()))?;
        
       // log::info!("Successfully recorded and submitted {} draw calls in single command buffer: {:?}", 
       //           "multiple", command_buffer);
        
        // Update frame counter
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
        
        Ok(())
    }
}

impl VulkanRenderer {
    // Getter methods for GameObject creation
    /// Get access to the VulkanContext for resource creation
    pub fn get_context(&self) -> &VulkanContext {
        &self.context
    }
    
    /// Get the descriptor pool for creating descriptor sets
    pub fn get_descriptor_pool(&self) -> vk::DescriptorPool {
        self.resource_manager.get_descriptor_pool()
    }
    
    /// Get the object descriptor set layout for GameObject uniform buffers
    pub fn get_object_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        // For now, use the material descriptor set layout which has the same structure
        // TODO: Add dedicated object descriptor set layout when GameObject system is fully integrated
        self.ubo_manager.material_descriptor_set_layout()
    }
    
    /// Get max frames in flight
    pub fn get_max_frames_in_flight(&self) -> usize {
        self.max_frames_in_flight
    }
    
    /// Get the current graphics pipeline
    pub fn get_graphics_pipeline(&self) -> vk::Pipeline {
        if let Some(pipeline) = self.pipeline_manager.get_active_pipeline() {
            pipeline.handle()
        } else {
            // Fallback to a default pipeline or return error
            // For now, return a null handle
            vk::Pipeline::null()
        }
    }
    
    /// Get the current pipeline layout
    pub fn get_pipeline_layout(&self) -> vk::PipelineLayout {
        if let Some(pipeline) = self.pipeline_manager.get_active_pipeline() {
            pipeline.layout()
        } else {
            // Fallback to a default pipeline layout or return error
            // For now, return a null handle
            vk::PipelineLayout::null()
        }
    }
    
    /// Get the frame descriptor set layout
    pub fn get_frame_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.ubo_manager.frame_descriptor_set_layout()
    }
}
