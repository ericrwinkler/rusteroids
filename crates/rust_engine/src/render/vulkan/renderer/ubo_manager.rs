//! UBO (Uniform Buffer Object) management for Vulkan renderer
//! 
//! Handles camera, lighting, and material uniform buffer objects.

use ash::vk;
use crate::render::vulkan::*;
use crate::render::vulkan::uniform_buffer::CameraUniformData;
use crate::render::material::{StandardMaterialUBO, UnlitMaterialUBO};
use crate::foundation::math::{Mat4, Vec3, Vec2, Vec4};

/// Simple lighting data structure for per-frame UBO updates
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct SimpleLightingUBO {
    ambient_color: [f32; 4],                // 16 bytes - RGBA ambient
    directional_light_direction: [f32; 4],  // 16 bytes - XYZ direction + W intensity
    directional_light_color: [f32; 4],      // 16 bytes - RGB color + A unused
    _padding: [f32; 4],                     // 16 bytes - padding for alignment
}

unsafe impl bytemuck::Pod for SimpleLightingUBO {}
unsafe impl bytemuck::Zeroable for SimpleLightingUBO {}

/// Manages all UBO resources and updates
pub struct UboManager {
    // UBO buffers
    camera_ubo_buffer: Buffer,
    lighting_ubo_buffers: Vec<Buffer>,
    material_ubo_buffer: Buffer,
    
    // Descriptor layouts
    frame_descriptor_set_layout: DescriptorSetLayout,
    material_descriptor_set_layout: DescriptorSetLayout,
    
    // Current frame tracking for lighting UBO updates
    current_frame: usize,
    max_frames_in_flight: usize,
}

impl UboManager {
    pub fn new(
        context: &VulkanContext, 
        _resource_manager: &super::ResourceManager,
        max_frames_in_flight: usize
    ) -> VulkanResult<Self> {
        log::debug!("Creating UboManager...");
        
        // Create descriptor set layouts
        let frame_descriptor_set_layout = DescriptorSetLayoutBuilder::new()
            .add_uniform_buffer(0, vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT) // Camera UBO
            .add_uniform_buffer(1, vk::ShaderStageFlags::FRAGMENT) // Lighting UBO
            .build(&context.raw_device())?;
            
        let material_descriptor_set_layout = DescriptorSetLayoutBuilder::new()
            .add_uniform_buffer(0, vk::ShaderStageFlags::FRAGMENT) // Material UBO
            .add_combined_image_sampler(1, vk::ShaderStageFlags::FRAGMENT) // Base color texture
            .add_combined_image_sampler(2, vk::ShaderStageFlags::FRAGMENT) // Normal texture  
            .add_combined_image_sampler(3, vk::ShaderStageFlags::FRAGMENT) // Metallic-roughness texture
            .build(&context.raw_device())?;
        
        // Create camera UBO buffer
        let camera_ubo_data = CameraUniformData::default();
        let camera_ubo_size = std::mem::size_of::<CameraUniformData>() as vk::DeviceSize;
        let camera_ubo_buffer = Buffer::new(
            context.raw_device().clone(),
            context.instance().clone(),
            context.physical_device().device,
            camera_ubo_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        // Write initial camera data
        let camera_bytes = unsafe { 
            std::slice::from_raw_parts(
                &camera_ubo_data as *const _ as *const u8,
                std::mem::size_of::<CameraUniformData>()
            )
        };
        camera_ubo_buffer.write_data(camera_bytes)?;
        
        // Create per-frame lighting UBO buffers
        let lighting_ubo_data = SimpleLightingUBO {
            ambient_color: [0.2, 0.2, 0.2, 1.0],
            directional_light_direction: [-0.5, -1.0, -0.3, 0.8],
            directional_light_color: [1.0, 0.95, 0.9, 1.0],
            _padding: [0.0; 4],
        };
        let lighting_ubo_size = std::mem::size_of::<SimpleLightingUBO>() as vk::DeviceSize;
        
        let mut lighting_ubo_buffers = Vec::with_capacity(max_frames_in_flight);
        for _ in 0..max_frames_in_flight {
            let lighting_buffer = Buffer::new(
                context.raw_device().clone(),
                context.instance().clone(),
                context.physical_device().device,
                lighting_ubo_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            
            let lighting_bytes = unsafe { 
                std::slice::from_raw_parts(
                    &lighting_ubo_data as *const _ as *const u8,
                    std::mem::size_of::<SimpleLightingUBO>()
                )
            };
            lighting_buffer.write_data(lighting_bytes)?;
            lighting_ubo_buffers.push(lighting_buffer);
        }
        
        // Create default material UBO
        use crate::render::material::{StandardMaterialUBO, StandardMaterialParams};
        
        let default_material_params = StandardMaterialParams {
            base_color: Vec3::new(0.8, 0.6, 0.4), // Warm brown color
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.8,
            ambient_occlusion: 1.0,
            normal_scale: 1.0,
            emission: Vec3::new(0.0, 0.0, 0.0),
            emission_strength: 0.0,
        };
        
        let default_material_ubo = StandardMaterialUBO::from_params(&default_material_params);
        let material_ubo_size = std::mem::size_of::<StandardMaterialUBO>() as vk::DeviceSize;
        
        let material_ubo_buffer = Buffer::new(
            context.raw_device().clone(),
            context.instance().clone(),
            context.physical_device().device,
            material_ubo_size,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        
        let material_bytes = unsafe { 
            std::slice::from_raw_parts(
                &default_material_ubo as *const _ as *const u8,
                std::mem::size_of::<StandardMaterialUBO>()
            )
        };
        material_ubo_buffer.write_data(material_bytes)?;
        
        Ok(Self {
            camera_ubo_buffer,
            lighting_ubo_buffers,
            material_ubo_buffer,
            frame_descriptor_set_layout,
            material_descriptor_set_layout,
            current_frame: 0,
            max_frames_in_flight,
        })
    }
    
    /// Initialize descriptor sets after resource manager is ready
    pub fn initialize_descriptor_sets(
        &self,
        context: &VulkanContext,
        resource_manager: &mut super::ResourceManager
    ) -> VulkanResult<()> {
        // Allocate descriptor sets
        let frame_layouts = vec![self.frame_descriptor_set_layout.handle(); self.max_frames_in_flight];
        let material_layouts = vec![self.material_descriptor_set_layout.handle(); 1];
        
        resource_manager.allocate_descriptor_sets(&frame_layouts, &material_layouts)?;
        
        // Update frame descriptor sets
        let camera_ubo_size = std::mem::size_of::<CameraUniformData>() as vk::DeviceSize;
        let lighting_ubo_size = std::mem::size_of::<SimpleLightingUBO>() as vk::DeviceSize;
        
        for (frame_index, &descriptor_set) in resource_manager.frame_descriptor_sets().iter().enumerate() {
            let camera_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.camera_ubo_buffer.handle())
                .offset(0)
                .range(camera_ubo_size)
                .build();
                
            let lighting_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.lighting_ubo_buffers[frame_index].handle())
                .offset(0)
                .range(lighting_ubo_size)
                .build();
                
            let descriptor_writes = [
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[camera_buffer_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[lighting_buffer_info])
                    .build(),
            ];
            
            unsafe {
                context.raw_device().update_descriptor_sets(&descriptor_writes, &[]);
            }
        }
        
        // Update material descriptor sets
        let material_ubo_size = std::mem::size_of::<StandardMaterialUBO>() as vk::DeviceSize;
        
        for &descriptor_set in resource_manager.material_descriptor_sets().iter() {
            let material_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.material_ubo_buffer.handle())
                .offset(0)
                .range(material_ubo_size)
                .build();
            
            let descriptor_writes = vec![
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[material_buffer_info])
                    .build(),
                // Texture bindings will be added when texture system is implemented
            ];
            
            unsafe {
                context.raw_device().update_descriptor_sets(&descriptor_writes, &[]);
            }
        }
        
        Ok(())
    }
    
    /// Update camera UBO data
    pub fn update_camera(&mut self, view_matrix: Mat4, projection_matrix: Mat4, view_projection_matrix: Mat4, camera_position: Vec3) {
        let camera_ubo_data = CameraUniformData {
            view_matrix,
            projection_matrix,
            view_projection_matrix,
            camera_position: Vec4::new(camera_position.x, camera_position.y, camera_position.z, 1.0),
            camera_direction: Vec4::new(0.0, 0.0, -1.0, 0.0),
            viewport_size: Vec2::new(800.0, 600.0), // TODO: Get from swapchain
            near_far: Vec2::new(0.1, 100.0),
        };
        
        let camera_bytes = unsafe { 
            std::slice::from_raw_parts(
                &camera_ubo_data as *const _ as *const u8,
                std::mem::size_of::<CameraUniformData>()
            )
        };
        
        if let Err(e) = self.camera_ubo_buffer.write_data(camera_bytes) {
            log::error!("Failed to update camera UBO: {:?}", e);
        }
    }
    
    /// Update lighting UBO data
    pub fn update_lighting(&mut self, direction: [f32; 3], intensity: f32, color: [f32; 3], ambient_intensity: f32) {
        let lighting_ubo_data = SimpleLightingUBO {
            ambient_color: [ambient_intensity * 0.2, ambient_intensity * 0.2, ambient_intensity * 0.2, 1.0],
            directional_light_direction: [direction[0], direction[1], direction[2], intensity],
            directional_light_color: [color[0], color[1], color[2], 1.0],
            _padding: [0.0; 4],
        };
        
        let lighting_bytes = unsafe { 
            std::slice::from_raw_parts(
                &lighting_ubo_data as *const _ as *const u8,
                std::mem::size_of::<SimpleLightingUBO>()
            )
        };
        
        if let Err(e) = self.lighting_ubo_buffers[self.current_frame].write_data(lighting_bytes) {
            log::error!("Failed to update lighting UBO: {:?}", e);
        }
    }
    
    /// Update material UBO data
    pub fn update_material(&mut self, material: &crate::render::Material) -> VulkanResult<()> {
        use crate::render::material::{MaterialType, StandardMaterialUBO, UnlitMaterialUBO};
        
        let material_ubo_data = match &material.material_type {
            MaterialType::StandardPBR(params) => {
                let ubo = StandardMaterialUBO::from_params(params);
                unsafe {
                    std::slice::from_raw_parts(
                        &ubo as *const _ as *const u8,
                        std::mem::size_of::<StandardMaterialUBO>()
                    )
                }.to_vec()
            },
            MaterialType::Unlit(params) => {
                let ubo = UnlitMaterialUBO::from_params(params);
                let mut padded = vec![0u8; std::mem::size_of::<StandardMaterialUBO>()];
                let ubo_bytes = unsafe {
                    std::slice::from_raw_parts(
                        &ubo as *const _ as *const u8,
                        std::mem::size_of::<UnlitMaterialUBO>()
                    )
                };
                padded[..ubo_bytes.len()].copy_from_slice(ubo_bytes);
                padded
            },
            _ => {
                return Err(VulkanError::InitializationFailed("Unsupported material type".to_string()));
            }
        };
        
        self.material_ubo_buffer.write_data(&material_ubo_data)?;
        Ok(())
    }
    
    /// Update material color only (legacy interface)
    pub fn update_material_color(&mut self, color: [f32; 4]) -> VulkanResult<()> {
        // For now, create a simple material with the given color
        use crate::render::material::{StandardMaterialParams, StandardMaterialUBO};
        
        let material_params = StandardMaterialParams {
            base_color: Vec3::new(color[0], color[1], color[2]),
            alpha: color[3],
            metallic: 0.1,
            roughness: 0.8,
            ambient_occlusion: 1.0,
            normal_scale: 1.0,
            emission: Vec3::new(0.0, 0.0, 0.0),
            emission_strength: 0.0,
        };
        
        let material_ubo = StandardMaterialUBO::from_params(&material_params);
        let material_bytes = unsafe {
            std::slice::from_raw_parts(
                &material_ubo as *const _ as *const u8,
                std::mem::size_of::<StandardMaterialUBO>()
            )
        };
        
        self.material_ubo_buffer.write_data(material_bytes)?;
        Ok(())
    }
    
    /// Set current frame for lighting UBO updates
    pub fn set_current_frame(&mut self, frame: usize) {
        self.current_frame = frame;
    }
    
    // Getters
    pub fn frame_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.frame_descriptor_set_layout.handle()
    }
    
    pub fn material_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.material_descriptor_set_layout.handle()
    }
}
