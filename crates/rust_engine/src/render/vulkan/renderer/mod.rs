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
        self.set_model_matrix(model);
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
}
