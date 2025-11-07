//! Vulkan surface management
//! 
//! Handles window surface creation and management for presentation

use ash::{vk, extensions::khr};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use crate::render::backends::vulkan::{VulkanContext, VulkanResult, VulkanError};

/// Vulkan surface wrapper for presentation
pub struct Surface {
    /// Vulkan instance for surface operations
    instance: ash::Instance,
    surface_loader: khr::Surface,
    surface: vk::SurfaceKHR,
}

impl Surface {
    /// Create a new surface from a window
    pub fn new<W>(
        context: &VulkanContext,
        window: &W,
    ) -> VulkanResult<Self> 
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let surface_loader = khr::Surface::new(context.entry(), context.instance());
        
        let surface = unsafe {
            ash_window::create_surface(
                context.entry(),
                context.instance(),
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
            .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create surface: {:?}", e)))?
        };

        Ok(Self {
            instance: context.instance().clone(),
            surface_loader,
            surface,
        })
    }

    /// Get the underlying surface handle
    pub fn handle(&self) -> vk::SurfaceKHR {
        self.surface
    }

    /// Get the surface loader
    pub fn loader(&self) -> &khr::Surface {
        &self.surface_loader
    }

    /// Get surface capabilities for a physical device
    pub fn capabilities(&self, physical_device: vk::PhysicalDevice) -> VulkanResult<vk::SurfaceCapabilitiesKHR> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(physical_device, self.surface)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to get surface capabilities: {:?}", e)))
        }
    }

    /// Get surface formats for a physical device
    pub fn formats(&self, physical_device: vk::PhysicalDevice) -> VulkanResult<Vec<vk::SurfaceFormatKHR>> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(physical_device, self.surface)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to get surface formats: {:?}", e)))
        }
    }

    /// Get surface present modes for a physical device
    pub fn present_modes(&self, physical_device: vk::PhysicalDevice) -> VulkanResult<Vec<vk::PresentModeKHR>> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(physical_device, self.surface)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to get present modes: {:?}", e)))
        }
    }

    /// Check if a queue family supports presentation to this surface
    pub fn supports_present(&self, physical_device: vk::PhysicalDevice, queue_family_index: u32) -> VulkanResult<bool> {
        unsafe {
            self.surface_loader
                .get_physical_device_surface_support(physical_device, queue_family_index, self.surface)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to check surface support: {:?}", e)))
        }
    }
    
    /// Get the Vulkan instance associated with this surface
    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            self.surface_loader.destroy_surface(self.surface, None);
        }
    }
}
