//! Framebuffer management
//! 
//! Handles Vulkan framebuffer creation and management following RAII principles

use ash::{vk, Device};
use crate::render::vulkan::{VulkanResult, VulkanError};

/// Framebuffer wrapper with RAII cleanup
pub struct Framebuffer {
    device: Device,
    framebuffer: vk::Framebuffer,
}

impl Framebuffer {
    /// Create a new framebuffer
    pub fn new(
        device: Device,
        render_pass: vk::RenderPass,
        attachments: &[vk::ImageView],
        extent: vk::Extent2D,
    ) -> VulkanResult<Self> {
        let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(attachments)
            .width(extent.width)
            .height(extent.height)
            .layers(1);
        
        let framebuffer = unsafe {
            device.create_framebuffer(&framebuffer_create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        Ok(Self {
            device,
            framebuffer,
        })
    }
    
    /// Get the framebuffer handle
    pub fn handle(&self) -> vk::Framebuffer {
        self.framebuffer
    }
}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_framebuffer(self.framebuffer, None);
        }
    }
}

/// Depth buffer wrapper with RAII cleanup
pub struct DepthBuffer {
    device: Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    image_view: vk::ImageView,
}

impl DepthBuffer {
    /// Create a new depth buffer
    pub fn new(
        device: Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        extent: vk::Extent2D,
    ) -> VulkanResult<Self> {
        let format = vk::Format::D32_SFLOAT;
        
        // Create depth image
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);
        
        let image = unsafe {
            device.create_image(&image_create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        // Get memory requirements
        let memory_requirements = unsafe {
            device.get_image_memory_requirements(image)
        };
        
        // Find memory type
        let memory_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        let memory_type_index = Self::find_memory_type(
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &memory_properties,
        )?;
        
        // Allocate memory
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
        
        let memory = unsafe {
            device.allocate_memory(&alloc_info, None)
                .map_err(VulkanError::Api)?
        };
        
        // Bind memory
        unsafe {
            device.bind_image_memory(image, memory, 0)
                .map_err(VulkanError::Api)?;
        }
        
        // Create image view
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::DEPTH,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
        
        let image_view = unsafe {
            device.create_image_view(&image_view_create_info, None)
                .map_err(VulkanError::Api)?
        };
        
        Ok(Self {
            device,
            image,
            memory,
            image_view,
        })
    }
    
    /// Find suitable memory type
    fn find_memory_type(
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> VulkanResult<u32> {
        for i in 0..memory_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && memory_properties.memory_types[i as usize]
                    .property_flags
                    .contains(properties)
            {
                return Ok(i);
            }
        }
        
        Err(VulkanError::InvalidOperation {
            reason: "Failed to find suitable memory type".to_string(),
        })
    }
    
    /// Get the image view handle
    pub fn image_view(&self) -> vk::ImageView {
        self.image_view
    }
}

impl Drop for DepthBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
