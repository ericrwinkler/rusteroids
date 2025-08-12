//! Vulkan swapchain management
//! 
//! Handles swapchain creation, recreation, and management following RAII principles

use ash::{vk, Device, Instance};
use ash::extensions::khr::{Surface, Swapchain as SwapchainLoader};
use crate::render::vulkan::{VulkanResult, VulkanError, PhysicalDeviceInfo};

/// Swapchain management wrapper with RAII cleanup
pub struct Swapchain {
    device: Device,
    swapchain_loader: SwapchainLoader,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
    image_count: u32,
}

impl Swapchain {
    /// Create a new swapchain
    pub fn new(
        instance: &Instance,
        device: Device,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
        physical_device_info: &PhysicalDeviceInfo,
        window_extent: vk::Extent2D,
    ) -> VulkanResult<Self> {
        let swapchain_loader = SwapchainLoader::new(instance, &device);
        
        // Get surface capabilities
        let surface_caps = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device_info.device, surface)
                .map_err(VulkanError::Api)?
        };
        
        // Choose surface format
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device_info.device, surface)
                .map_err(VulkanError::Api)?
        };
        
        let format = surface_formats
            .iter()
            .find(|sf| sf.format == vk::Format::B8G8R8A8_SRGB && sf.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .cloned()
            .unwrap_or(surface_formats[0]);
        
        // Choose present mode
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device_info.device, surface)
                .map_err(VulkanError::Api)?
        };
        
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        
        // Choose extent
        let extent = if surface_caps.current_extent.width != u32::MAX {
            surface_caps.current_extent
        } else {
            vk::Extent2D {
                width: window_extent.width.clamp(
                    surface_caps.min_image_extent.width,
                    surface_caps.max_image_extent.width,
                ),
                height: window_extent.height.clamp(
                    surface_caps.min_image_extent.height,
                    surface_caps.max_image_extent.height,
                ),
            }
        };
        
        // Choose image count
        let image_count = (surface_caps.min_image_count + 1).min(
            if surface_caps.max_image_count > 0 {
                surface_caps.max_image_count
            } else {
                surface_caps.min_image_count + 1
            }
        );
        
        // Create swapchain
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(vk::SwapchainKHR::null());
        
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        // Get swapchain images
        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(VulkanError::Api)?
        };
        
        // Create image views
        let image_views: Result<Vec<_>, _> = images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                
                unsafe { device.create_image_view(&create_info, None) }
            })
            .collect();
        
        let image_views = image_views.map_err(VulkanError::Api)?;
        
        Ok(Self {
            device,
            swapchain_loader,
            swapchain,
            images,
            image_views,
            format,
            extent,
            image_count,
        })
    }
    
    /// Recreate the swapchain with new window dimensions
    pub fn recreate(
        instance: &Instance,
        device: Device,
        surface: vk::SurfaceKHR,
        surface_loader: &Surface,
        physical_device_info: &PhysicalDeviceInfo,
        window_extent: vk::Extent2D,
        old_swapchain: vk::SwapchainKHR,
    ) -> VulkanResult<Self> {
        let swapchain_loader = SwapchainLoader::new(instance, &device);
        
        // Get surface capabilities
        let surface_caps = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device_info.device, surface)
                .map_err(VulkanError::Api)?
        };
        
        // Choose surface format
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device_info.device, surface)
                .map_err(VulkanError::Api)?
        };
        
        let format = surface_formats
            .iter()
            .find(|sf| sf.format == vk::Format::B8G8R8A8_SRGB && sf.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR)
            .cloned()
            .unwrap_or(surface_formats[0]);
        
        // Choose present mode
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device_info.device, surface)
                .map_err(VulkanError::Api)?
        };
        
        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        
        // Choose extent
        let extent = if surface_caps.current_extent.width != u32::MAX {
            surface_caps.current_extent
        } else {
            vk::Extent2D {
                width: window_extent.width.clamp(
                    surface_caps.min_image_extent.width,
                    surface_caps.max_image_extent.width,
                ),
                height: window_extent.height.clamp(
                    surface_caps.min_image_extent.height,
                    surface_caps.max_image_extent.height,
                ),
            }
        };
        
        // Choose image count
        let image_count = (surface_caps.min_image_count + 1).min(
            if surface_caps.max_image_count > 0 {
                surface_caps.max_image_count
            } else {
                surface_caps.min_image_count + 1
            }
        );
        
        // Create swapchain with old swapchain for recreation
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain); // Use the old swapchain for recreation
        
        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&swapchain_create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        // Get swapchain images
        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(VulkanError::Api)?
        };
        
        // Create image views
        let image_views: Result<Vec<_>, _> = images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                
                unsafe { device.create_image_view(&create_info, None) }
            })
            .collect();
        
        let image_views = image_views.map_err(VulkanError::Api)?;
        
        Ok(Self {
            device,
            swapchain_loader,
            swapchain,
            images,
            image_views,
            format,
            extent,
            image_count,
        })
    }
    
    /// Get swapchain extent
    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
    
    /// Get surface format
    pub fn format(&self) -> vk::SurfaceFormatKHR {
        self.format
    }
    
    /// Get image views
    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }
    
    /// Get swapchain handle
    pub fn handle(&self) -> vk::SwapchainKHR {
        self.swapchain
    }
    
    /// Get swapchain loader
    pub fn loader(&self) -> &SwapchainLoader {
        &self.swapchain_loader
    }
    
    /// Get image count
    pub fn image_count(&self) -> u32 {
        self.image_count
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            // Cleanup image views
            for &image_view in &self.image_views {
                self.device.destroy_image_view(image_view, None);
            }
            
            // Cleanup swapchain
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}
