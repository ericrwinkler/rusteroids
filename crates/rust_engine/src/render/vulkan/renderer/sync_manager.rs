//! Synchronization management for Vulkan renderer
//! 
//! Handles semaphores, fences, and frames-in-flight synchronization.

use ash::vk;
use crate::render::vulkan::*;

/// Manages all synchronization objects and frame coordination
pub struct SyncManager {
    frame_sync_objects: Vec<FrameSync>,
    image_sync_objects: Vec<FrameSync>,
    frames_submitted: usize,
}

impl SyncManager {
    /// Create a new synchronization manager with per-frame and per-image sync objects
    pub fn new(
        context: &VulkanContext,
        swapchain: &Swapchain,
        max_frames_in_flight: usize
    ) -> VulkanResult<Self> {
        log::debug!("Creating SyncManager...");
        
        // Create per-image sync objects (one per swapchain image) for semaphores
        let image_count = swapchain.image_count() as usize;
        let mut image_sync_objects = Vec::new();
        for _ in 0..image_count {
            image_sync_objects.push(FrameSync::new(context.raw_device())?);
        }
        
        // Create per-frame sync objects (for frames in flight) for fences
        let mut frame_sync_objects = Vec::new();
        for _ in 0..max_frames_in_flight {
            frame_sync_objects.push(FrameSync::new(context.raw_device())?);
        }
        
        Ok(Self {
            frame_sync_objects,
            image_sync_objects,
            frames_submitted: 0,
        })
    }
    
    /// Wait for a specific frame to complete rendering (blog-guided approach)
    pub fn wait_for_frame_completion(&mut self, context: &VulkanContext, frame_index: usize) -> VulkanResult<()> {
        // BLOG GUIDANCE: "To signal a fence, all previously submitted commands must complete"
        // Only wait if this frame has actually been submitted before
        if frame_index >= self.frames_submitted {
            return Ok(()); // Nothing to wait for - frame never submitted
        }
        
        let frame_sync = &self.frame_sync_objects[frame_index];
        
        // BLOG GUIDANCE: Use reasonable timeout instead of infinite wait
        unsafe {
            context.device().device.wait_for_fences(
                &[frame_sync.in_flight.handle()],
                true,
                1_000_000_000, // 1 second timeout instead of infinite
            ).map_err(VulkanError::Api)?;
            
            // DON'T reset fence here - it will be reset just before next submission
        }
        
        Ok(())
    }
    
    /// Acquire next swapchain image
    pub fn acquire_next_image(&self, context: &VulkanContext, current_frame: usize) -> VulkanResult<(u32, &Semaphore)> {
        // We need to acquire first to know which image, then return the correct semaphore
        // For now, use frame-based semaphore but this may need fixing
        let acquire_semaphore = &self.frame_sync_objects[current_frame].image_available;
        
        let (image_index, _) = match unsafe {
            context.swapchain_loader().acquire_next_image(
                context.swapchain().handle(),
                u64::MAX,
                acquire_semaphore.handle(),
                vk::Fence::null(),
            )
        } {
            Ok(result) => result,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::warn!("Swapchain out of date during acquire_next_image");
                return Err(VulkanError::Api(vk::Result::ERROR_OUT_OF_DATE_KHR));
            }
            Err(e) => return Err(VulkanError::Api(e)),
        };
        
        Ok((image_index, acquire_semaphore))
    }
    
    /// Submit command buffer and present
    pub fn submit_and_present(
        &mut self,
        context: &VulkanContext,
        command_buffer: vk::CommandBuffer,
        acquire_semaphore_handle: vk::Semaphore,
        image_index: u32,
        frame_index: usize,
    ) -> VulkanResult<&Semaphore> {
        
        // Get sync objects
        let frame_sync = &self.frame_sync_objects[frame_index];
        let image_sync = &self.image_sync_objects[image_index as usize];
        
        // Reset fence just before submission (fixes validation error)
        unsafe {
            context.device().device.reset_fences(&[frame_sync.in_flight.handle()])
                .map_err(VulkanError::Api)?;
        }
        
        // Submit command buffer
        let wait_semaphores = [acquire_semaphore_handle];
        let command_buffers = [command_buffer];
        let signal_semaphores = [image_sync.render_finished.handle()];
        let submit_info = vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
            
        unsafe {
            context.device().device.queue_submit(
                context.graphics_queue(),
                &[submit_info.build()],
                frame_sync.in_flight.handle(),
            ).map_err(VulkanError::Api)?;
        }
        
        // Track that this frame has been submitted
        self.frames_submitted = self.frames_submitted.max(frame_index + 1);
        
        // Present
        let wait_semaphores = [image_sync.render_finished.handle()];
        let swapchains = [context.swapchain().handle()];
        let image_indices = [image_index];
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);
            
        match unsafe {
            context.swapchain_loader().queue_present(
                context.present_queue(),
                &present_info
            )
        } {
            Ok(_) => {},
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::warn!("Swapchain out of date during queue_present");
                return Err(VulkanError::Api(vk::Result::ERROR_OUT_OF_DATE_KHR));
            }
            Err(e) => return Err(VulkanError::Api(e)),
        }
        
        Ok(&image_sync.render_finished)
    }
    
    /// Wait for device to be idle
    pub fn wait_idle(&self, context: &VulkanContext) -> VulkanResult<()> {
        unsafe {
            context.device().device.device_wait_idle()
                .map_err(VulkanError::Api)
        }
    }
}
