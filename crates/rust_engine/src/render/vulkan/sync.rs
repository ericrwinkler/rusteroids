//! Synchronization primitives
//! 
//! Vulkan synchronization objects following RAII patterns

use ash::{vk, Device};
use crate::render::vulkan::{VulkanResult, VulkanError};

/// Semaphore wrapper with RAII cleanup
pub struct Semaphore {
    device: Device,
    semaphore: vk::Semaphore,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(device: Device) -> VulkanResult<Self> {
        let create_info = vk::SemaphoreCreateInfo::builder();
        
        let semaphore = unsafe {
            device.create_semaphore(&create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        Ok(Self { device, semaphore })
    }
    
    /// Get the semaphore handle
    pub fn handle(&self) -> vk::Semaphore {
        self.semaphore
    }
}

impl Drop for Semaphore {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_semaphore(self.semaphore, None);
        }
    }
}

/// Fence wrapper with RAII cleanup
pub struct Fence {
    device: Device,
    fence: vk::Fence,
}

impl Fence {
    /// Create a new fence
    pub fn new(device: Device, signaled: bool) -> VulkanResult<Self> {
        let flags = if signaled {
            vk::FenceCreateFlags::SIGNALED
        } else {
            vk::FenceCreateFlags::empty()
        };
        
        let create_info = vk::FenceCreateInfo::builder().flags(flags);
        
        let fence = unsafe {
            device.create_fence(&create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        Ok(Self { device, fence })
    }
    
    /// Wait for fence
    pub fn wait(&self, timeout: u64) -> VulkanResult<()> {
        unsafe {
            self.device.wait_for_fences(&[self.fence], true, timeout)
                .map_err(VulkanError::Api)
        }
    }
    
    /// Reset fence
    pub fn reset(&self) -> VulkanResult<()> {
        unsafe {
            self.device.reset_fences(&[self.fence])
                .map_err(VulkanError::Api)
        }
    }
    
    /// Get the fence handle
    pub fn handle(&self) -> vk::Fence {
        self.fence
    }
}

impl Drop for Fence {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_fence(self.fence, None);
        }
    }
}

/// Frame synchronization objects for in-flight frame management
pub struct FrameSync {
    pub image_available: Semaphore,
    pub render_finished: Semaphore,
    pub in_flight: Fence,
}

impl FrameSync {
    /// Create frame synchronization objects
    pub fn new(device: Device) -> VulkanResult<Self> {
        let image_available = Semaphore::new(device.clone())?;
        let render_finished = Semaphore::new(device.clone())?;
        let in_flight = Fence::new(device, true)?;
        
        Ok(Self {
            image_available,
            render_finished,
            in_flight,
        })
    }
}
