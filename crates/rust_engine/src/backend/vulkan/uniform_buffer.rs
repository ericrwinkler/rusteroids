/// Uniform Buffer Object management for Vulkan renderer
/// 
/// This module provides safe, high-level abstractions for uniform buffers,
/// descriptor sets, and data uploads in the Vulkan graphics pipeline.

use ash::{vk, Device};
use crate::foundation::math::{Mat3, Mat4, Vec2, Vec4};
use std::mem;
use crate::backend::vulkan::{VulkanError, VulkanResult};

/// Camera uniform data - updated once per frame
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct CameraUniformData {
    /// View matrix (world to camera space)
    pub view_matrix: Mat4,
    /// Projection matrix (camera to clip space)  
    pub projection_matrix: Mat4,
    /// Pre-computed view-projection matrix for efficiency
    pub view_projection_matrix: Mat4,
    /// Camera position in world space (Vec3 + padding)
    pub camera_position: Vec4,
    /// Camera forward direction (Vec3 + padding)
    pub camera_direction: Vec4,
    /// Viewport size (width, height)
    pub viewport_size: Vec2,
    /// Near and far clip planes
    pub near_far: Vec2,
}

/// Directional light data
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct DirectionalLightData {
    /// Light direction (normalized, Vec3 + padding)
    pub direction: Vec4,
    /// Light color and intensity (RGB + intensity)
    pub color: Vec4,
}

/// Point light data
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct PointLightData {
    /// Light position (Vec3 + radius)
    pub position: Vec4,
    /// Light color and intensity (RGB + intensity)
    pub color: Vec4,
    /// Attenuation factors (constant, linear, quadratic, _padding)
    pub attenuation: Vec4,
}

/// Spot light data
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SpotLightData {
    /// Light position (Vec3 + padding)
    pub position: Vec4,
    /// Light direction (Vec3 + padding)
    pub direction: Vec4,
    /// Light color and intensity (RGB + intensity)
    pub color: Vec4,
    /// Spot light parameters (inner_angle, outer_angle, attenuation, intensity)
    pub params: Vec4,
}

/// Lighting uniform data - updated when lights change
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct LightingUniformData {
    /// Ambient light color and intensity
    pub ambient_color: Vec4,
    /// Directional lights (up to 4)
    pub directional_lights: [DirectionalLightData; 4],
    /// Point lights (up to 8)  
    pub point_lights: [PointLightData; 8],
    /// Spot lights (up to 4)
    pub spot_lights: [SpotLightData; 4],
    /// Number of active directional lights
    pub num_dir_lights: u32,
    /// Number of active point lights  
    pub num_point_lights: u32,
    /// Number of active spot lights
    pub num_spot_lights: u32,
    /// Padding for alignment
    pub _padding: u32,
}

/// Material uniform data
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct MaterialUniformData {
    /// Base color (RGBA)
    pub base_color: Vec4,
    /// Metallic, roughness, ambient occlusion, _padding
    pub metallic_roughness: Vec4,
    /// Emission color and strength (RGB + strength)
    pub emission: Vec4,
    /// Normal map scale
    pub normal_scale: f32,
    /// Texture usage flags (bitfield)
    pub texture_flags: u32,
    /// Padding to maintain alignment
    pub _padding: [u32; 2],
}

/// Push constants - minimal per-draw data
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct PushConstants {
    /// Model matrix (object to world space)
    pub model_matrix: Mat4,
    /// Normal matrix (for transforming normals)
    pub normal_matrix: Mat3,
    /// Material ID (index into material buffer)
    pub material_id: u32,
    /// Padding for alignment
    pub _padding: [u32; 3],
}

// Implement Pod and Zeroable for all uniform data types
unsafe impl bytemuck::Pod for CameraUniformData {}
unsafe impl bytemuck::Zeroable for CameraUniformData {}

unsafe impl bytemuck::Pod for DirectionalLightData {}
unsafe impl bytemuck::Zeroable for DirectionalLightData {}

unsafe impl bytemuck::Pod for PointLightData {}
unsafe impl bytemuck::Zeroable for PointLightData {}

unsafe impl bytemuck::Pod for SpotLightData {}
unsafe impl bytemuck::Zeroable for SpotLightData {}

unsafe impl bytemuck::Pod for LightingUniformData {}
unsafe impl bytemuck::Zeroable for LightingUniformData {}

unsafe impl bytemuck::Pod for MaterialUniformData {}
unsafe impl bytemuck::Zeroable for MaterialUniformData {}

unsafe impl bytemuck::Pod for PushConstants {}
unsafe impl bytemuck::Zeroable for PushConstants {}

impl Default for CameraUniformData {
    fn default() -> Self {
        Self {
            view_matrix: Mat4::identity(),
            projection_matrix: Mat4::identity(),
            view_projection_matrix: Mat4::identity(),
            camera_position: Vec4::new(0.0, 0.0, 0.0, 1.0),
            camera_direction: Vec4::new(0.0, 0.0, -1.0, 0.0),
            viewport_size: Vec2::new(800.0, 600.0),
            near_far: Vec2::new(0.1, 100.0),
        }
    }
}

impl Default for LightingUniformData {
    fn default() -> Self {
        Self {
            ambient_color: Vec4::new(0.1, 0.1, 0.1, 1.0),
            directional_lights: [DirectionalLightData::default(); 4],
            point_lights: [PointLightData::default(); 8],
            spot_lights: [SpotLightData::default(); 4],
            num_dir_lights: 0,
            num_point_lights: 0,
            num_spot_lights: 0,
            _padding: 0,
        }
    }
}

impl Default for DirectionalLightData {
    fn default() -> Self {
        Self {
            direction: Vec4::new(0.0, -1.0, 0.0, 0.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
        }
    }
}

impl Default for PointLightData {
    fn default() -> Self {
        Self {
            position: Vec4::new(0.0, 0.0, 0.0, 1.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            attenuation: Vec4::new(1.0, 0.09, 0.032, 0.0),
        }
    }
}

impl Default for SpotLightData {
    fn default() -> Self {
        Self {
            position: Vec4::new(0.0, 0.0, 0.0, 1.0),
            direction: Vec4::new(0.0, -1.0, 0.0, 0.0),
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            params: Vec4::new(12.5_f32.to_radians(), 17.5_f32.to_radians(), 1.0, 1.0),
        }
    }
}

impl Default for MaterialUniformData {
    fn default() -> Self {
        Self {
            base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            metallic_roughness: Vec4::new(0.0, 0.5, 1.0, 0.0),
            emission: Vec4::new(0.0, 0.0, 0.0, 0.0),
            normal_scale: 1.0,
            texture_flags: 0,
            _padding: [0; 2],
        }
    }
}

impl Default for PushConstants {
    fn default() -> Self {
        Self {
            model_matrix: Mat4::identity(),
            normal_matrix: Mat3::identity(),
            material_id: 0,
            _padding: [0; 3],
        }
    }
}

/// Uniform buffer wrapper with automatic memory management
pub struct UniformBuffer<T> {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    mapped_ptr: *mut T,
    device: Device,
    size: usize,
}

impl<T> UniformBuffer<T>
where
    T: bytemuck::Pod + bytemuck::Zeroable,
{
    /// Create a new uniform buffer
    pub fn new(device: Device, memory_properties: vk::PhysicalDeviceMemoryProperties) -> VulkanResult<Self> {
        let size = mem::size_of::<T>();
        
        // Create buffer
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size as vk::DeviceSize)
            .usage(vk::BufferUsageFlags::UNIFORM_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
            
        let buffer = unsafe { device.create_buffer(&buffer_info, None) }
            .map_err(VulkanError::Api)?;
            
        // Get memory requirements
        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        
        // Find suitable memory type (host visible and coherent)
        let memory_type = find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            memory_properties,
        )?;
        
        // Allocate memory
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type);
            
        let memory = unsafe { device.allocate_memory(&alloc_info, None) }
            .map_err(VulkanError::Api)?;
            
        // Bind buffer to memory
        unsafe { device.bind_buffer_memory(buffer, memory, 0) }
            .map_err(VulkanError::Api)?;
            
        // Map memory
        let mapped_ptr = unsafe {
            device.map_memory(memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty())
                .map_err(VulkanError::Api)?
        } as *mut T;
        
        Ok(Self {
            buffer,
            memory,
            mapped_ptr,
            device,
            size,
        })
    }
    
    /// Update the uniform buffer data
    pub fn update(&mut self, data: &T) -> VulkanResult<()> {
        unsafe {
            std::ptr::copy_nonoverlapping(data, self.mapped_ptr, 1);
        }
        Ok(())
    }
    
    /// Get the buffer handle
    pub fn buffer(&self) -> vk::Buffer {
        self.buffer
    }
    
    /// Get the buffer size
    pub fn size(&self) -> usize {
        self.size
    }
}

impl<T> Drop for UniformBuffer<T> {
    fn drop(&mut self) {
        unsafe {
            self.device.unmap_memory(self.memory);
            self.device.destroy_buffer(self.buffer, None);
            self.device.free_memory(self.memory, None);
        }
    }
}

/// Find a suitable memory type
fn find_memory_type(
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
    mem_properties: vk::PhysicalDeviceMemoryProperties,
) -> VulkanResult<u32> {
    for (i, memory_type) in mem_properties.memory_types.iter().enumerate() {
        if (type_filter & (1 << i)) != 0 && memory_type.property_flags.contains(properties) {
            return Ok(i as u32);
        }
    }
    Err(VulkanError::NoSuitableMemoryType)
}
