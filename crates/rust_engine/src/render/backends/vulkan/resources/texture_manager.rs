//! Texture management and descriptor set creation
//! 
//! Handles texture uploading and material descriptor set creation
//! for the Vulkan renderer.

use crate::render::backends::vulkan::*;
use crate::render::backends::vulkan::Texture;
use crate::render::resources::materials::{TextureHandle, TextureType, StandardMaterialUBO};
use ash::vk;
use std::sync::Arc;

/// Manages texture operations and material descriptor sets
pub struct TextureManager;

impl TextureManager {
    /// Upload texture from image data to GPU
    ///
    /// Creates a Vulkan texture from image data and returns a handle for use with materials.
    /// This method provides access to texture creation for procedurally generated textures
    /// like font atlases.
    ///
    /// # Arguments
    /// * `context` - Vulkan context for device access
    /// * `resource_manager` - Resource manager to store the texture
    /// * `image_data` - Image data (RGBA format)
    /// * `texture_type` - Type of texture (BaseColor, Normal, etc.)
    ///
    /// # Returns
    /// TextureHandle for the uploaded texture
    pub fn upload_texture_from_image_data(
        context: &VulkanContext,
        resource_manager: &mut super::ResourceManager,
        image_data: crate::assets::ImageData,
        texture_type: TextureType,
    ) -> Result<TextureHandle, Box<dyn std::error::Error>> {
        // Create Vulkan texture from image data
        let texture = Texture::from_image_data(
            Arc::new(context.raw_device().clone()),
            Arc::new(context.instance().clone()),
            context.physical_device().device,
            resource_manager.command_pool_handle(),
            context.graphics_queue(),
            &image_data,
        )?;
        
        // Store texture in resource manager and get index
        let texture_index = resource_manager.add_loaded_texture(texture);
        
        // Create a TextureHandle (using index as handle ID)
        let handle = TextureHandle(texture_index as u32);
        
        log::debug!("Uploaded texture {:?} (type: {:?}, index: {})", handle, texture_type, texture_index);
        Ok(handle)
    }
    
    /// Create a material descriptor set with a specific texture
    ///
    /// Allocates a new descriptor set and binds the specified texture to it.
    /// All other texture bindings use default white textures.
    ///
    /// # Arguments
    /// * `context` - Vulkan context for device access
    /// * `resource_manager` - Resource manager for descriptor pool and textures
    /// * `ubo_manager` - UBO manager for material UBO and descriptor set layout
    /// * `texture_handle` - The texture to bind as base color (binding 1)
    ///
    /// # Returns
    /// A descriptor set configured with the specified texture
    pub fn create_material_descriptor_set_with_texture(
        context: &VulkanContext,
        resource_manager: &super::ResourceManager,
        ubo_manager: &super::UboManager,
        texture_handle: TextureHandle,
    ) -> Result<vk::DescriptorSet, Box<dyn std::error::Error>> {
        // Allocate a new descriptor set
        let layouts = vec![ubo_manager.material_descriptor_set_layout()];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(resource_manager.get_descriptor_pool())
            .set_layouts(&layouts);

        let descriptor_sets = unsafe {
            context.raw_device().allocate_descriptor_sets(&alloc_info)
                .map_err(|e| format!("Failed to allocate descriptor set: {:?}", e))?
        };
        
        let descriptor_set = descriptor_sets[0];
        
        // Get the texture from resource manager
        let texture = resource_manager.get_loaded_texture(texture_handle.0 as usize)
            .ok_or("Texture not found in resource manager")?;
        
        // Material UBO buffer info - use actual size of StandardMaterialUBO
        let material_ubo_size = std::mem::size_of::<StandardMaterialUBO>() as vk::DeviceSize;
        let material_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(ubo_manager.material_ubo_buffer_handle())
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
            .image_view(resource_manager.default_white_texture().image_view())
            .sampler(resource_manager.default_white_texture().sampler())
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
            context.raw_device().update_descriptor_sets(&descriptor_writes, &[]);
        }
        
        log::info!("Created material descriptor set {:?} with texture {:?}", descriptor_set, texture_handle);
        Ok(descriptor_set)
    }
}
