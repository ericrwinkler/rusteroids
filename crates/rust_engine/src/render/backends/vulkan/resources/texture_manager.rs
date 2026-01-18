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
    
    /// Create a material descriptor set with optional custom textures
    ///
    /// Creates a descriptor set for materials with flexibility to specify which
    /// textures to use custom values for, while using sensible defaults for others.
    ///
    /// # Arguments
    /// * `context` - Vulkan context for device access
    /// * `resource_manager` - Resource manager for descriptor pool and textures
    /// * `ubo_manager` - UBO manager for material UBO and descriptor set layout
    /// * `base_color` - Optional custom base color texture (uses default white if None)
    /// * `normal` - Optional custom normal map texture (uses default normal if None)
    /// * `metallic_roughness` - Optional metallic/roughness texture (uses default white if None)
    /// * `ao` - Optional ambient occlusion texture (uses default white if None)
    /// * `emission` - Optional emission texture (uses default white if None)
    /// * `opacity` - Optional opacity texture (uses default white if None)
    ///
    /// # Returns
    /// A descriptor set configured with the specified textures
    pub fn create_material_descriptor_set(
        context: &VulkanContext,
        resource_manager: &super::ResourceManager,
        ubo_manager: &super::UboManager,
        base_color: Option<TextureHandle>,
        normal: Option<TextureHandle>,
        metallic_roughness: Option<TextureHandle>,
        ao: Option<TextureHandle>,
        emission: Option<TextureHandle>,
        opacity: Option<TextureHandle>,
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
        
        // Helper function to get texture info for a handle or default
        let get_texture_info = |handle: Option<TextureHandle>, use_normal_default: bool| -> Result<vk::DescriptorImageInfo, Box<dyn std::error::Error>> {
            if let Some(handle) = handle {
                let texture = resource_manager.get_loaded_texture(handle.0 as usize)
                    .ok_or("Texture not found in resource manager")?;
                Ok(vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(texture.image_view())
                    .sampler(texture.sampler())
                    .build())
            } else {
                let default_texture = if use_normal_default {
                    resource_manager.default_normal_texture()
                } else {
                    resource_manager.default_white_texture()
                };
                Ok(vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(default_texture.image_view())
                    .sampler(default_texture.sampler())
                    .build())
            }
        };
        
        // Material UBO buffer info
        let material_ubo_size = std::mem::size_of::<StandardMaterialUBO>() as vk::DeviceSize;
        let material_buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(ubo_manager.material_ubo_buffer_handle())
            .offset(0)
            .range(material_ubo_size)
            .build();
        
        // Get texture infos for each binding
        let base_color_info = get_texture_info(base_color, false)?;
        let normal_info = get_texture_info(normal, true)?; // Use normal default for binding 2
        let metallic_roughness_info = get_texture_info(metallic_roughness, false)?;
        let ao_info = get_texture_info(ao, false)?;
        let emission_info = get_texture_info(emission, false)?;
        let opacity_info = get_texture_info(opacity, false)?;

        // Keep arrays alive - CRITICAL for avoiding dangling pointers!
        let material_buffer_infos = [material_buffer_info];
        let base_color_infos = [base_color_info];
        let normal_infos = [normal_info];
        let metallic_roughness_infos = [metallic_roughness_info];
        let ao_infos = [ao_info];
        let emission_infos = [emission_info];
        let opacity_infos = [opacity_info];

        let descriptor_writes = vec![
            // Binding 0: Material UBO
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(&material_buffer_infos)
                .build(),
            // Binding 1: Base color texture
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(1)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&base_color_infos)
                .build(),
            // Binding 2: Normal map
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(2)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&normal_infos)
                .build(),
            // Binding 3: Metallic/roughness
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(3)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&metallic_roughness_infos)
                .build(),
            // Binding 4: Ambient occlusion
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(4)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&ao_infos)
                .build(),
            // Binding 5: Emission
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(5)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&emission_infos)
                .build(),
            // Binding 6: Opacity
            vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(6)
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&opacity_infos)
                .build(),
        ];
        
        unsafe {
            context.raw_device().update_descriptor_sets(&descriptor_writes, &[]);
        }
        
        log::info!("Created material descriptor set with textures: base_color={:?}, normal={:?}, metallic_roughness={:?}, ao={:?}, emission={:?}, opacity={:?}", 
                   base_color, normal, metallic_roughness, ao, emission, opacity);
        Ok(descriptor_set)
    }

    // Convenience wrappers for common material configurations
    
    /// Create a simple material with just a base color texture
    pub fn create_simple_material(
        context: &VulkanContext,
        resource_manager: &super::ResourceManager,
        ubo_manager: &super::UboManager,
        base_color: TextureHandle,
    ) -> Result<vk::DescriptorSet, Box<dyn std::error::Error>> {
        Self::create_material_descriptor_set(
            context, resource_manager, ubo_manager,
            Some(base_color), None, None, None, None, None
        )
    }
    
    /// Create a material with base color and emission (like the frigate)
    pub fn create_emissive_material(
        context: &VulkanContext,
        resource_manager: &super::ResourceManager,
        ubo_manager: &super::UboManager,
        base_color: TextureHandle,
        emission: TextureHandle,
    ) -> Result<vk::DescriptorSet, Box<dyn std::error::Error>> {
        Self::create_material_descriptor_set(
            context, resource_manager, ubo_manager,
            Some(base_color), None, None, None, Some(emission), None
        )
    }
    
    /// Create a full PBR material with base color, normal, and optional emission
    pub fn create_pbr_material(
        context: &VulkanContext,
        resource_manager: &super::ResourceManager,
        ubo_manager: &super::UboManager,
        base_color: TextureHandle,
        normal: TextureHandle,
        emission: Option<TextureHandle>,
    ) -> Result<vk::DescriptorSet, Box<dyn std::error::Error>> {
        Self::create_material_descriptor_set(
            context, resource_manager, ubo_manager,
            Some(base_color), Some(normal), None, None, emission, None
        )
    }
}
