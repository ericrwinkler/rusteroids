//! Vulkan texture management
//!
//! Provides basic texture and image view functionality for the renderer.

use ash::vk;
use crate::render::backends::vulkan::{VulkanResult, VulkanError};
use crate::assets::ImageData;
use std::sync::Arc;

/// Basic Vulkan texture with image, image view, and sampler
pub struct Texture {
    device: Arc<ash::Device>,
    image: vk::Image,
    image_view: vk::ImageView,
    sampler: vk::Sampler,
    memory: vk::DeviceMemory,
    #[allow(dead_code)]
    extent: vk::Extent2D,
    #[allow(dead_code)]
    format: vk::Format,
}

impl Texture {
    /// Create a default 1x1 white texture for material placeholders
    pub fn create_default_white(
        device: Arc<ash::Device>,
        instance: Arc<ash::Instance>,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> VulkanResult<Self> {
        Self::create_solid_color(
            device,
            instance,
            physical_device,
            command_pool,
            graphics_queue,
            [255, 255, 255, 255], // White
        )
    }
    
    /// Create a default 1x1 normal map texture (128, 128, 255, 255) 
    pub fn create_default_normal(
        device: Arc<ash::Device>,
        instance: Arc<ash::Instance>,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> VulkanResult<Self> {
        Self::create_solid_color(
            device,
            instance,
            physical_device,
            command_pool,
            graphics_queue,
            [128, 128, 255, 255], // Default normal (pointing up in tangent space)
        )
    }
    
    /// Create a default 1x1 metallic-roughness texture (0, 255, 0, 255)
    /// Metallic=0 (non-metallic), Roughness=255 (fully rough), unused channels=0
    pub fn create_default_metallic_roughness(
        device: Arc<ash::Device>,
        instance: Arc<ash::Instance>,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> VulkanResult<Self> {
        Self::create_solid_color(
            device,
            instance,
            physical_device,
            command_pool,
            graphics_queue,
            [0, 255, 0, 255], // Non-metallic, fully rough
        )
    }

    /// Create a texture from loaded image data
    pub fn from_image_data(
        device: Arc<ash::Device>,
        instance: Arc<ash::Instance>,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        image_data: &ImageData,
    ) -> VulkanResult<Self> {
        let extent = vk::Extent2D { 
            width: image_data.width, 
            height: image_data.height 
        };
        let format = vk::Format::R8G8B8A8_UNORM;
        
        // Calculate number of mip levels
        let mip_levels = ((extent.width.max(extent.height) as f32).log2().floor() as u32) + 1;
        log::debug!("Creating texture {}x{} with {} mip levels", extent.width, extent.height, mip_levels);
        
        // Create image with mipmap support
        let image_create_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(mip_levels)  // Enable mipmaps
            .array_layers(1)
            .format(format)
            .tiling(vk::ImageTiling::OPTIMAL)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::SAMPLED)  // Add TRANSFER_SRC for blit
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);
            
        let image = unsafe {
            device.create_image(&image_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create image: {:?}", e)))?
        };
        
        // Allocate memory for image
        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let memory_type_index = Self::find_memory_type(
            &instance,
            physical_device,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        
        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
            
        let memory = unsafe {
            device.allocate_memory(&memory_allocate_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate image memory: {:?}", e)))?
        };
        
        unsafe {
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to bind image memory: {:?}", e)))?;
        }
        
        // Upload texture data to mip level 0 (leaves it in TRANSFER_DST_OPTIMAL for mipmapping)
        Self::upload_texture_data_from_slice_no_transition(
            &device,
            &instance,
            physical_device,
            command_pool,
            graphics_queue,
            image,
            extent,
            &image_data.data,
        )?;
        
        // Generate mipmaps (expects mip 0 in TRANSFER_DST_OPTIMAL, transitions all to SHADER_READ_ONLY_OPTIMAL)
        Self::generate_mipmaps(
            &device,
            command_pool,
            graphics_queue,
            image,
            extent,
            mip_levels,
        )?;
        
        // Create image view (all mip levels)
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: mip_levels,  // Use all mip levels
                base_array_layer: 0,
                layer_count: 1,
            });
            
        let image_view = unsafe {
            device.create_image_view(&image_view_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create image view: {:?}", e)))?
        };
        
        // Create sampler with mipmapping and anisotropic filtering
        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(true)  // Enable anisotropic filtering for better quality
            .max_anisotropy(16.0)     // Maximum quality
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(vk::LOD_CLAMP_NONE);  // Allow all mip levels
            
        let sampler = unsafe {
            device.create_sampler(&sampler_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create sampler: {:?}", e)))?
        };
        
        Ok(Self {
            device,
            image,
            image_view,
            sampler,
            memory,
            extent,
            format,
        })
    }
    
    /// Create a solid color 1x1 texture
    fn create_solid_color(
        device: Arc<ash::Device>,
        instance: Arc<ash::Instance>,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        color: [u8; 4],
    ) -> VulkanResult<Self> {
        let extent = vk::Extent2D { width: 1, height: 1 };
        let format = vk::Format::R8G8B8A8_UNORM;
        
        // Create image
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
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .samples(vk::SampleCountFlags::TYPE_1);
            
        let image = unsafe {
            device.create_image(&image_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create image: {:?}", e)))?
        };
        
        // Allocate memory for image
        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };
        let memory_type_index = Self::find_memory_type(
            &instance,
            physical_device,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        
        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
            
        let memory = unsafe {
            device.allocate_memory(&memory_allocate_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate image memory: {:?}", e)))?
        };
        
        unsafe {
            device.bind_image_memory(image, memory, 0)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to bind image memory: {:?}", e)))?;
        }
        
        // Upload texture data
        Self::upload_texture_data(
            &device,
            &instance,
            physical_device,
            command_pool,
            graphics_queue,
            image,
            extent,
            &color,
        )?;
        
        // Create image view
        let image_view_create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });
            
        let image_view = unsafe {
            device.create_image_view(&image_view_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create image view: {:?}", e)))?
        };
        
        // Create sampler (solid color textures don't need anisotropic filtering, but enable for consistency)
        let sampler_create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)  // No anisotropic needed for 1x1 textures
            .max_anisotropy(1.0)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .min_lod(0.0)
            .max_lod(0.0);  // Only 1 mip level for 1x1 textures
            
        let sampler = unsafe {
            device.create_sampler(&sampler_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create sampler: {:?}", e)))?
        };
        
        Ok(Self {
            device,
            image,
            image_view,
            sampler,
            memory,
            extent,
            format,
        })
    }
    
    /// Upload texture data to GPU
    fn upload_texture_data(
        device: &ash::Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        image: vk::Image,
        extent: vk::Extent2D,
        pixel_data: &[u8; 4],
    ) -> VulkanResult<()> {
        // Create staging buffer
        let buffer_size = 4; // 4 bytes for RGBA
        
        let staging_buffer_create_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
        let staging_buffer = unsafe {
            device.create_buffer(&staging_buffer_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create staging buffer: {:?}", e)))?
        };
        
        let memory_requirements = unsafe { device.get_buffer_memory_requirements(staging_buffer) };
        let memory_type_index = Self::find_memory_type(
            instance,
            physical_device,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
            
        let staging_memory = unsafe {
            device.allocate_memory(&memory_allocate_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate staging memory: {:?}", e)))?
        };
        
        unsafe {
            device.bind_buffer_memory(staging_buffer, staging_memory, 0)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to bind staging buffer memory: {:?}", e)))?;
        }
        
        // Copy data to staging buffer
        unsafe {
            let data_ptr = device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to map staging memory: {:?}", e)))? as *mut u8;
            std::ptr::copy_nonoverlapping(pixel_data.as_ptr(), data_ptr, 4);
            device.unmap_memory(staging_memory);
        }
        
        // Copy staging buffer to image
        Self::copy_buffer_to_image(
            device,
            command_pool,
            graphics_queue,
            staging_buffer,
            image,
            extent,
        )?;
        
        // Cleanup staging resources
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        }
        
        Ok(())
    }

    /// Upload texture data from a byte slice to GPU
    #[allow(dead_code)]
    fn upload_texture_data_from_slice(
        device: &ash::Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        image: vk::Image,
        extent: vk::Extent2D,
        pixel_data: &[u8],
    ) -> VulkanResult<()> {
        // Create staging buffer
        let buffer_size = pixel_data.len() as vk::DeviceSize;
        
        let staging_buffer_create_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
        let staging_buffer = unsafe {
            device.create_buffer(&staging_buffer_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create staging buffer: {:?}", e)))?
        };
        
        let memory_requirements = unsafe { device.get_buffer_memory_requirements(staging_buffer) };
        let memory_type_index = Self::find_memory_type(
            instance,
            physical_device,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
            
        let staging_memory = unsafe {
            device.allocate_memory(&memory_allocate_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate staging memory: {:?}", e)))?
        };
        
        unsafe {
            device.bind_buffer_memory(staging_buffer, staging_memory, 0)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to bind staging buffer memory: {:?}", e)))?;
        }
        
        // Copy data to staging buffer
        unsafe {
            let data_ptr = device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to map staging memory: {:?}", e)))? as *mut u8;
            std::ptr::copy_nonoverlapping(pixel_data.as_ptr(), data_ptr, pixel_data.len());
            device.unmap_memory(staging_memory);
        }
        
        // Copy staging buffer to image
        Self::copy_buffer_to_image(
            device,
            command_pool,
            graphics_queue,
            staging_buffer,
            image,
            extent,
        )?;
        
        // Cleanup staging resources
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        }
        
        Ok(())
    }
    
    /// Upload texture data from a byte slice to GPU (leaves in TRANSFER_DST_OPTIMAL for mipmapping)
    fn upload_texture_data_from_slice_no_transition(
        device: &ash::Device,
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        image: vk::Image,
        extent: vk::Extent2D,
        pixel_data: &[u8],
    ) -> VulkanResult<()> {
        // Create staging buffer
        let buffer_size = pixel_data.len() as vk::DeviceSize;
        
        let staging_buffer_create_info = vk::BufferCreateInfo::builder()
            .size(buffer_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
        let staging_buffer = unsafe {
            device.create_buffer(&staging_buffer_create_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to create staging buffer: {:?}", e)))?
        };
        
        let memory_requirements = unsafe { device.get_buffer_memory_requirements(staging_buffer) };
        let memory_type_index = Self::find_memory_type(
            instance,
            physical_device,
            memory_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        let memory_allocate_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);
            
        let staging_memory = unsafe {
            device.allocate_memory(&memory_allocate_info, None)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate staging memory: {:?}", e)))?
        };
        
        unsafe {
            device.bind_buffer_memory(staging_buffer, staging_memory, 0)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to bind staging buffer memory: {:?}", e)))?;
        }
        
        // Copy data to staging buffer
        unsafe {
            let data_ptr = device.map_memory(staging_memory, 0, buffer_size, vk::MemoryMapFlags::empty())
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to map staging memory: {:?}", e)))? as *mut u8;
            std::ptr::copy_nonoverlapping(pixel_data.as_ptr(), data_ptr, pixel_data.len());
            device.unmap_memory(staging_memory);
        }
        
        // Copy staging buffer to image (without final transition)
        Self::copy_buffer_to_image_no_final_transition(
            device,
            command_pool,
            graphics_queue,
            staging_buffer,
            image,
            extent,
        )?;
        
        // Cleanup staging resources
        unsafe {
            device.destroy_buffer(staging_buffer, None);
            device.free_memory(staging_memory, None);
        }
        
        Ok(())
    }
    
    /// Copy buffer data to image
    fn copy_buffer_to_image(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        buffer: vk::Buffer,
        image: vk::Image,
        extent: vk::Extent2D,
    ) -> VulkanResult<()> {
        // Allocate command buffer
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);
            
        let command_buffer = unsafe {
            device.allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate command buffer: {:?}", e)))?[0]
        };
        
        // Begin command buffer
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to begin command buffer: {:?}", e)))?;
        }
        
        // Transition image layout to TRANSFER_DST_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
            
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier.build()],
            );
        }
        
        // Copy buffer to image
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            });
            
        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region.build()],
            );
        }
        
        // Transition image layout to SHADER_READ_ONLY_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ);
            
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier.build()],
            );
        }
        
        // End command buffer
        unsafe {
            device.end_command_buffer(command_buffer)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to end command buffer: {:?}", e)))?;
        }
        
        // Submit command buffer
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers);
            
        unsafe {
            device.queue_submit(graphics_queue, &[submit_info.build()], vk::Fence::null())
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to submit queue: {:?}", e)))?;
                
            device.queue_wait_idle(graphics_queue)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to wait for queue idle: {:?}", e)))?;
        }
        
        // Free command buffer
        unsafe {
            device.free_command_buffers(command_pool, &[command_buffer]);
        }
        
        Ok(())
    }
    
    /// Copy buffer data to image (without final transition to SHADER_READ_ONLY_OPTIMAL)
    /// Leaves image in TRANSFER_DST_OPTIMAL for further operations like mipmapping
    fn copy_buffer_to_image_no_final_transition(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        buffer: vk::Buffer,
        image: vk::Image,
        extent: vk::Extent2D,
    ) -> VulkanResult<()> {
        // Allocate command buffer
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);
            
        let command_buffer = unsafe {
            device.allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate command buffer: {:?}", e)))?[0]
        };
        
        // Begin command buffer
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to begin command buffer: {:?}", e)))?;
        }
        
        // Transition image layout to TRANSFER_DST_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
            
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier.build()],
            );
        }
        
        // Copy buffer to image
        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
            .image_extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            });
            
        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region.build()],
            );
        }
        
        // NOTE: No final transition - leave in TRANSFER_DST_OPTIMAL for mipmapping
        
        // End command buffer
        unsafe {
            device.end_command_buffer(command_buffer)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to end command buffer: {:?}", e)))?;
        }
        
        // Submit command buffer
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers);
            
        unsafe {
            device.queue_submit(graphics_queue, &[submit_info.build()], vk::Fence::null())
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to submit queue: {:?}", e)))?;
                
            device.queue_wait_idle(graphics_queue)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to wait for queue idle: {:?}", e)))?;
        }
        
        // Free command buffer
        unsafe {
            device.free_command_buffers(command_pool, &[command_buffer]);
        }
        
        Ok(())
    }
    
    /// Generate mipmaps using vkCmdBlitImage
    /// 
    /// This progressively downsamples each mip level from the previous level,
    /// providing proper texture filtering at all distances to eliminate flickering.
    fn generate_mipmaps(
        device: &ash::Device,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        image: vk::Image,
        extent: vk::Extent2D,
        mip_levels: u32,
    ) -> VulkanResult<()> {
        // Allocate command buffer
        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);
            
        let command_buffer = unsafe {
            device.allocate_command_buffers(&command_buffer_allocate_info)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to allocate command buffer: {:?}", e)))?[0]
        };
        
        // Begin command buffer
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            
        unsafe {
            device.begin_command_buffer(command_buffer, &begin_info)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to begin command buffer: {:?}", e)))?;
        }
        
        let mut mip_width = extent.width as i32;
        let mut mip_height = extent.height as i32;
        
        // Generate each mip level from the previous one
        for i in 1..mip_levels {
            // Transition current mip level (i) from UNDEFINED to TRANSFER_DST_OPTIMAL
            let barrier_dst = vk::ImageMemoryBarrier::builder()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: i,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
                
            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier_dst.build()],
                );
            }
            
            // Transition previous mip level to TRANSFER_SRC_OPTIMAL
            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: i - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::TRANSFER_READ);
                
            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier.build()],
                );
            }
            
            // Blit from previous mip level to current mip level
            let next_mip_width = if mip_width > 1 { mip_width / 2 } else { 1 };
            let next_mip_height = if mip_height > 1 { mip_height / 2 } else { 1 };
            
            let blit = vk::ImageBlit::builder()
                .src_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D { x: mip_width, y: mip_height, z: 1 },
                ])
                .src_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: i - 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .dst_offsets([
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D { x: next_mip_width, y: next_mip_height, z: 1 },
                ])
                .dst_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: i,
                    base_array_layer: 0,
                    layer_count: 1,
                });
                
            unsafe {
                device.cmd_blit_image(
                    command_buffer,
                    image,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[blit.build()],
                    vk::Filter::LINEAR,  // Linear filtering for smooth downsampling
                );
            }
            
            // Transition previous mip level to SHADER_READ_ONLY_OPTIMAL
            let barrier = vk::ImageMemoryBarrier::builder()
                .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: i - 1,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_READ)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);
                
            unsafe {
                device.cmd_pipeline_barrier(
                    command_buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[barrier.build()],
                );
            }
            
            mip_width = next_mip_width;
            mip_height = next_mip_height;
        }
        
        // Transition last mip level to SHADER_READ_ONLY_OPTIMAL
        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: mip_levels - 1,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            })
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::SHADER_READ);
            
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier.build()],
            );
        }
        
        // End command buffer
        unsafe {
            device.end_command_buffer(command_buffer)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to end command buffer: {:?}", e)))?;
        }
        
        // Submit command buffer
        let command_buffers = [command_buffer];
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers);
            
        unsafe {
            device.queue_submit(graphics_queue, &[submit_info.build()], vk::Fence::null())
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to submit queue: {:?}", e)))?;
                
            device.queue_wait_idle(graphics_queue)
                .map_err(|e| VulkanError::InitializationFailed(format!("Failed to wait for queue idle: {:?}", e)))?;
        }
        
        // Free command buffer
        unsafe {
            device.free_command_buffers(command_pool, &[command_buffer]);
        }
        
        Ok(())
    }
    
    /// Find memory type index
    fn find_memory_type(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
    ) -> VulkanResult<u32> {
        let memory_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        for i in 0..memory_properties.memory_type_count {
            if (type_filter & (1 << i)) != 0
                && memory_properties.memory_types[i as usize].property_flags.contains(properties)
            {
                return Ok(i);
            }
        }
        
        Err(VulkanError::InitializationFailed("Failed to find suitable memory type".to_string()))
    }
    
    /// Get the image view for descriptor set binding
    pub fn image_view(&self) -> vk::ImageView {
        self.image_view
    }
    
    /// Get the sampler for descriptor set binding
    pub fn sampler(&self) -> vk::Sampler {
        self.sampler
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_image_view(self.image_view, None);
            self.device.destroy_image(self.image, None);
            self.device.free_memory(self.memory, None);
        }
    }
}
