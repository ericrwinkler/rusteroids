//! Texture Management for Material System
//!
//! This module handles texture loading, GPU upload, and binding
//! for the material system.

use std::collections::HashMap;
use std::sync::Arc;
use ash::vk;
use crate::render::vulkan::{VulkanResult, VulkanError};
use crate::render::vulkan::core::Texture;
use crate::assets::ImageData;

/// Handle for a GPU texture resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

/// Types of textures supported by the material system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureType {
    /// Base color/albedo texture
    BaseColor,
    /// Normal map texture
    Normal,
    /// Metallic-roughness texture (metallic in B channel, roughness in G channel)
    MetallicRoughness,
    /// Ambient occlusion texture
    AmbientOcclusion,
    /// Emission texture
    Emission,
}

/// Texture filtering modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    /// Nearest neighbor filtering
    Nearest,
    /// Linear filtering
    Linear,
}

/// Texture wrapping modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// Repeat the texture
    Repeat,
    /// Mirror the texture
    MirroredRepeat,
    /// Clamp to edge
    ClampToEdge,
}

/// Texture creation parameters
#[derive(Debug, Clone)]
pub struct TextureParams {
    /// Texture filtering mode
    pub filter_mode: FilterMode,
    /// Texture wrapping mode
    pub wrap_mode: WrapMode,
    /// Generate mipmaps
    pub generate_mipmaps: bool,
}

impl Default for TextureParams {
    fn default() -> Self {
        Self {
            filter_mode: FilterMode::Linear,
            wrap_mode: WrapMode::Repeat,
            generate_mipmaps: true,
        }
    }
}

/// Texture manager that handles loading and GPU management
/// 
/// Stores Vulkan textures and provides texture handle mapping
pub struct TextureManager {
    /// Loaded Vulkan textures mapped by handle
    textures: HashMap<TextureHandle, Arc<Texture>>,
    /// Texture metadata
    texture_info: HashMap<TextureHandle, TextureInfo>,
    /// Next available texture handle
    next_handle: u32,
    /// Vulkan context for texture creation
    device: Arc<ash::Device>,
    instance: Arc<ash::Instance>,
    physical_device: vk::PhysicalDevice,
    command_pool: vk::CommandPool,
    graphics_queue: vk::Queue,
}

/// Information about a loaded texture
#[derive(Debug, Clone)]
pub struct TextureInfo {
    /// Texture handle
    pub handle: TextureHandle,
    /// Texture type/usage
    pub texture_type: TextureType,
    /// Creation parameters
    pub params: TextureParams,
    /// Optional name for debugging
    pub name: Option<String>,
}

impl TextureManager {
    /// Create a new texture manager with Vulkan context
    pub fn new(
        device: Arc<ash::Device>,
        instance: Arc<ash::Instance>,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> Self {
        Self {
            textures: HashMap::new(),
            texture_info: HashMap::new(),
            next_handle: 1, // Start from 1, reserve 0 for "no texture"
            device,
            instance,
            physical_device,
            command_pool,
            graphics_queue,
        }
    }

    /// Load a texture from file
    pub fn load_texture_from_file(
        &mut self,
        file_path: &str,
        texture_type: TextureType,
        params: TextureParams,
    ) -> VulkanResult<TextureHandle> {
        // Load image data from file
        let image_data = ImageData::from_file(file_path)
            .map_err(|e| VulkanError::InitializationFailed(format!("Failed to load image: {}", e)))?;

        // Create Vulkan texture from image data
        let texture = Texture::from_image_data(
            self.device.clone(),
            self.instance.clone(),
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            &image_data,
        )?;

        let handle = TextureHandle(self.next_handle);
        self.next_handle += 1;

        let texture_info = TextureInfo {
            handle,
            texture_type,
            params,
            name: Some(file_path.to_string()),
        };

        self.textures.insert(handle, Arc::new(texture));
        self.texture_info.insert(handle, texture_info);

        log::debug!("Loaded texture {:?} from {} of type {:?}", handle, file_path, texture_type);
        Ok(handle)
    }

    /// Create a solid color texture (useful for testing)
    pub fn create_solid_color_texture(
        &mut self,
        color: [u8; 4], // RGBA
        texture_type: TextureType,
    ) -> VulkanResult<TextureHandle> {
        // Create 1x1 image data
        let image_data = ImageData::solid_color(1, 1, color);

        // Create Vulkan texture from image data
        let texture = Texture::from_image_data(
            self.device.clone(),
            self.instance.clone(),
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            &image_data,
        )?;

        let handle = TextureHandle(self.next_handle);
        self.next_handle += 1;

        let texture_info = TextureInfo {
            handle,
            texture_type,
            params: TextureParams::default(),
            name: Some(format!("SolidColor_{:?}", texture_type)),
        };

        self.textures.insert(handle, Arc::new(texture));
        self.texture_info.insert(handle, texture_info);

        log::debug!("Created solid color texture {:?} of type {:?}", handle, texture_type);
        Ok(handle)
    }

    /// Load texture directly from ImageData (useful for procedurally generated textures)
    pub fn load_texture_from_image_data(
        &mut self,
        image_data: ImageData,
        texture_type: TextureType,
        name: Option<String>,
    ) -> VulkanResult<TextureHandle> {
        // Create Vulkan texture from image data
        let texture = Texture::from_image_data(
            self.device.clone(),
            self.instance.clone(),
            self.physical_device,
            self.command_pool,
            self.graphics_queue,
            &image_data,
        )?;

        let handle = TextureHandle(self.next_handle);
        self.next_handle += 1;

        let texture_info = TextureInfo {
            handle,
            texture_type,
            params: TextureParams::default(),
            name: name.clone(),
        };

        self.textures.insert(handle, Arc::new(texture));
        self.texture_info.insert(handle, texture_info);

        log::debug!("Loaded texture {:?} from ImageData ({}x{}, {:?}) of type {:?}", 
                    handle, image_data.width, image_data.height, name, texture_type);
        Ok(handle)
    }

    /// Get the Vulkan texture for a handle
    pub fn get_texture(&self, handle: TextureHandle) -> Option<Arc<Texture>> {
        self.textures.get(&handle).cloned()
    }

    /// Get texture information
    pub fn get_texture_info(&self, handle: TextureHandle) -> Option<&TextureInfo> {
        self.texture_info.get(&handle)
    }

    /// Get number of loaded textures
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }

    /// Get all texture handles
    pub fn get_all_texture_handles(&self) -> Vec<TextureHandle> {
        self.textures.keys().copied().collect()
    }
}

/// Material texture bindings
/// 
/// This structure defines which textures are bound to which material slots.
#[derive(Debug, Clone, Default)]
pub struct MaterialTextures {
    /// Base color texture
    pub base_color: Option<TextureHandle>,
    /// Normal map texture
    pub normal: Option<TextureHandle>,
    /// Metallic-roughness texture
    pub metallic_roughness: Option<TextureHandle>,
    /// Ambient occlusion texture
    pub ambient_occlusion: Option<TextureHandle>,
    /// Emission texture
    pub emission: Option<TextureHandle>,
    /// Opacity texture
    pub opacity: Option<TextureHandle>,
}

impl MaterialTextures {
    /// Create new empty material textures
    pub fn new() -> Self {
        Self::default()
    }

    /// Set base color texture
    pub fn with_base_color(mut self, texture: TextureHandle) -> Self {
        self.base_color = Some(texture);
        self
    }

    /// Set normal map texture
    pub fn with_normal(mut self, texture: TextureHandle) -> Self {
        self.normal = Some(texture);
        self
    }

    /// Set metallic-roughness texture
    pub fn with_metallic_roughness(mut self, texture: TextureHandle) -> Self {
        self.metallic_roughness = Some(texture);
        self
    }

    /// Check if any textures are bound
    pub fn has_any_textures(&self) -> bool {
        self.base_color.is_some()
            || self.normal.is_some()
            || self.metallic_roughness.is_some()
            || self.ambient_occlusion.is_some()
            || self.emission.is_some()
            || self.opacity.is_some()
    }

    /// Get texture usage flags for UBO
    pub fn get_texture_flags(&self) -> [u32; 4] {
        [
            if self.base_color.is_some() { 1 } else { 0 },
            if self.normal.is_some() { 1 } else { 0 },
            if self.metallic_roughness.is_some() { 1 } else { 0 },
            if self.ambient_occlusion.is_some() { 1 } else { 0 },
        ]
    }

    /// Check if emission texture is present
    pub fn has_emission_texture(&self) -> bool {
        self.emission.is_some()
    }

    /// Check if opacity texture is present
    pub fn has_opacity_texture(&self) -> bool {
        self.opacity.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_material_textures() {
        let textures = MaterialTextures::new()
            .with_base_color(TextureHandle(1))
            .with_normal(TextureHandle(2));

        assert!(textures.has_any_textures());
        assert_eq!(textures.base_color, Some(TextureHandle(1)));
        assert_eq!(textures.normal, Some(TextureHandle(2)));

        let flags = textures.get_texture_flags();
        assert_eq!(flags, [1, 1, 0, 0]); // base_color and normal are set
    }
}
