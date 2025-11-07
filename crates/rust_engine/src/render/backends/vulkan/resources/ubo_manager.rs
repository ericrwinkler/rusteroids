//! UBO (Uniform Buffer Object) management for Vulkan renderer
//! 
//! Handles camera, lighting, and material uniform buffer objects.

use ash::vk;
use crate::render::backends::vulkan::{VulkanContext, VulkanResult};
use crate::render::backends::vulkan::Buffer;
use crate::render::backends::vulkan::{DescriptorSetLayoutBuilder, DescriptorSetLayout};
use crate::render::backends::vulkan::CameraUniformData;
use crate::render::resources::materials::StandardMaterialUBO;
use crate::foundation::math::{Mat4, Vec3, Vec2, Vec4};
use crate::render::systems::lighting::MultiLightEnvironment;
use crate::assets::ImageData;
use crate::render::backends::vulkan::Texture;
use std::sync::Arc;

/// Multi-light UBO structure that matches shader expectations
/// This matches the MultiLightUBO in the multi_light_frag.frag shader
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct MultiLightingUBO {
    ambient_color: [f32; 4],                // 16 bytes - RGBA ambient
    directional_light_count: u32,           // 4 bytes
    point_light_count: u32,                 // 4 bytes  
    spot_light_count: u32,                  // 4 bytes
    _padding: u32,                          // 4 bytes - total 16 bytes
    directional_lights: [DirectionalLightData; 4], // 4 * 32 = 128 bytes
    point_lights: [PointLightData; 8],              // 8 * 48 = 384 bytes
    spot_lights: [SpotLightData; 4],                // 4 * 64 = 256 bytes
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct DirectionalLightData {
    direction: [f32; 4],    // 16 bytes - xyz + intensity
    color: [f32; 4],        // 16 bytes - rgb + padding
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct PointLightData {
    position: [f32; 4],     // 16 bytes - xyz + range
    color: [f32; 4],        // 16 bytes - rgb + intensity
    attenuation: [f32; 4],  // 16 bytes - constant, linear, quadratic, padding
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
struct SpotLightData {
    position: [f32; 4],     // 16 bytes - xyz + range
    direction: [f32; 4],    // 16 bytes - xyz + intensity
    color: [f32; 4],        // 16 bytes - rgb + padding
    cone_angles: [f32; 4],  // 16 bytes - inner, outer, unused, unused
}

unsafe impl bytemuck::Pod for MultiLightingUBO {}
unsafe impl bytemuck::Zeroable for MultiLightingUBO {}

unsafe impl bytemuck::Pod for DirectionalLightData {}
unsafe impl bytemuck::Zeroable for DirectionalLightData {}

unsafe impl bytemuck::Pod for PointLightData {}
unsafe impl bytemuck::Zeroable for PointLightData {}

unsafe impl bytemuck::Pod for SpotLightData {}
unsafe impl bytemuck::Zeroable for SpotLightData {}

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
    /// Create a new UBO manager with camera, lighting, and material buffers
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
            .add_combined_image_sampler(4, vk::ShaderStageFlags::FRAGMENT) // AO texture
            .add_combined_image_sampler(5, vk::ShaderStageFlags::FRAGMENT) // Emission texture
            .add_combined_image_sampler(6, vk::ShaderStageFlags::FRAGMENT) // Opacity texture
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
        let lighting_ubo_data = MultiLightingUBO {
            ambient_color: [0.2, 0.2, 0.2, 0.2], // Match old ambient intensity (0.2, not 1.0)
            directional_light_count: 1,
            point_light_count: 0,
            spot_light_count: 0,
            _padding: 0,
            directional_lights: [
                DirectionalLightData {
                    direction: [-0.5, -1.0, -0.3, 1.0], // Normalize intensity to 1.0 for consistency
                    color: [0.8, 0.76, 0.72, 1.0], // Tone down the bright warm color
                },
                DirectionalLightData {
                    direction: [0.0; 4],
                    color: [0.0; 4],
                },
                DirectionalLightData {
                    direction: [0.0; 4],
                    color: [0.0; 4],
                },
                DirectionalLightData {
                    direction: [0.0; 4],
                    color: [0.0; 4],
                },
            ],
            point_lights: [PointLightData {
                position: [0.0; 4],
                color: [0.0; 4],
                attenuation: [1.0, 0.0, 0.0, 0.0],
            }; 8],
            spot_lights: [SpotLightData {
                position: [0.0; 4],
                direction: [0.0; 4],
                color: [0.0; 4],
                cone_angles: [0.0; 4],
            }; 4],
        };
        let lighting_ubo_size = std::mem::size_of::<MultiLightingUBO>() as vk::DeviceSize;
        
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
                    std::mem::size_of::<MultiLightingUBO>()
                )
            };
            lighting_buffer.write_data(lighting_bytes)?;
            lighting_ubo_buffers.push(lighting_buffer);
        }
        
        // Create default material UBO
        use crate::render::resources::materials::{StandardMaterialUBO, StandardMaterialParams};
        
        let default_material_params = StandardMaterialParams {
            base_color: Vec3::new(0.8, 0.6, 0.4), // Warm brown default
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.8,
            ambient_occlusion: 1.0,
            normal_scale: 1.0,
            emission: Vec3::new(0.0, 0.0, 0.0), // No global emission - per-instance now
            emission_strength: 0.0,
            base_color_texture_enabled: false,
            normal_texture_enabled: false,
        };
        
        let default_material_ubo = StandardMaterialUBO::from_params(&default_material_params)
            .with_texture_flags(true, false, false, false) // Enable base_color texture
            .with_emission_texture(true); // Enable emission texture sampling
        
        log::info!("Default Material UBO - base_color: {:?}, emission: {:?}, texture_flags: {:?}, additional_params: {:?}",
                   default_material_ubo.base_color,
                   default_material_ubo.emission,
                   default_material_ubo.texture_flags,
                   default_material_ubo.additional_params);
        
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
        let lighting_ubo_size = std::mem::size_of::<MultiLightingUBO>() as vk::DeviceSize;
        
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
        
        // Load base color texture (texture.png) BEFORE the descriptor set loop
        let base_color_texture_idx = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.pop(); // rust_engine
            path.pop(); // crates
            path.push("resources");
            path.push("textures");
            path.push("texture.png");

            match ImageData::from_file(path.to_str().unwrap_or("resources/textures/texture.png")) {
                Ok(img) => {
                    log::info!("Found texture.png at {:?}, creating GPU texture", path);
                    match Texture::from_image_data(
                        Arc::new(context.raw_device().clone()),
                        Arc::new(context.instance().clone()),
                        context.physical_device().device,
                        resource_manager.command_pool_handle(),
                        context.graphics_queue(),
                        &img,
                    ) {
                        Ok(tex) => {
                            let idx = resource_manager.add_loaded_texture(tex);
                            log::info!("Successfully loaded and stored texture.png as texture #{}", idx);
                            Some(idx)
                        }
                        Err(e) => {
                            log::warn!("Failed to create base color texture from image data: {:?}, falling back to default", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::info!("No texture.png found or failed to load: {:?}, using default white", e);
                    None
                }
            }
        };
        
        // Load emission texture BEFORE the descriptor set loop to avoid borrow conflicts
        let emission_texture_idx = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.pop(); // rust_engine
            path.pop(); // crates
            path.push("resources");
            path.push("textures");
            path.push("emission_map.png");

            match ImageData::from_file(path.to_str().unwrap_or("resources/textures/emission_map.png")) {
                Ok(img) => {
                    log::info!("Found emission_map.png at {:?}, creating GPU texture", path);
                    match Texture::from_image_data(
                        Arc::new(context.raw_device().clone()),
                        Arc::new(context.instance().clone()),
                        context.physical_device().device,
                        resource_manager.command_pool_handle(),
                        context.graphics_queue(),
                        &img,
                    ) {
                        Ok(tex) => {
                            let idx = resource_manager.add_loaded_texture(tex);
                            log::info!("Successfully loaded and stored emission_map.png as texture #{}", idx);
                            Some(idx)
                        }
                        Err(e) => {
                            log::warn!("Failed to create emission texture from image data: {:?}, falling back to default", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::info!("No emission_map.png found or failed to load: {:?}, using default white", e);
                    None
                }
            }
        };
        
        // Load normal map texture BEFORE the descriptor set loop
        let normal_map_texture_idx = {
            let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.pop(); // rust_engine
            path.pop(); // crates
            path.push("resources");
            path.push("textures");
            path.push("normal_map.png");

            match ImageData::from_file(path.to_str().unwrap_or("resources/textures/normal_map.png")) {
                Ok(img) => {
                    log::info!("Found normal_map.png at {:?}, creating GPU texture", path);
                    match Texture::from_image_data(
                        Arc::new(context.raw_device().clone()),
                        Arc::new(context.instance().clone()),
                        context.physical_device().device,
                        resource_manager.command_pool_handle(),
                        context.graphics_queue(),
                        &img,
                    ) {
                        Ok(tex) => {
                            let idx = resource_manager.add_loaded_texture(tex);
                            log::info!("Successfully loaded and stored normal_map.png as texture #{}", idx);
                            Some(idx)
                        }
                        Err(e) => {
                            log::warn!("Failed to create normal texture from image data: {:?}, falling back to default", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    log::info!("No normal_map.png found or failed to load: {:?}, using default normal", e);
                    None
                }
            }
        };
        
        for &descriptor_set in resource_manager.material_descriptor_sets().iter() {
            let material_buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.material_ubo_buffer.handle())
                .offset(0)
                .range(material_ubo_size)
                .build();
            
            // Build emission image info from loaded texture or default
            let emission_image_info = if let Some(idx) = emission_texture_idx {
                if let Some(stored) = resource_manager.get_loaded_texture(idx) {
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(stored.image_view())
                        .sampler(stored.sampler())
                        .build()
                } else {
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(resource_manager.default_white_texture().image_view())
                        .sampler(resource_manager.default_white_texture().sampler())
                        .build()
                }
            } else {
                vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(resource_manager.default_white_texture().image_view())
                    .sampler(resource_manager.default_white_texture().sampler())
                    .build()
            };

            // Create image info for base color texture (use loaded texture.png or default)
            let base_color_image_info = if let Some(idx) = base_color_texture_idx {
                if let Some(stored) = resource_manager.get_loaded_texture(idx) {
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(stored.image_view())
                        .sampler(stored.sampler())
                        .build()
                } else {
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(resource_manager.default_white_texture().image_view())
                        .sampler(resource_manager.default_white_texture().sampler())
                        .build()
                }
            } else {
                vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(resource_manager.default_white_texture().image_view())
                    .sampler(resource_manager.default_white_texture().sampler())
                    .build()
            };
                
            // Build normal image info from loaded texture or default
            let normal_image_info = if let Some(idx) = normal_map_texture_idx {
                if let Some(stored) = resource_manager.get_loaded_texture(idx) {
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(stored.image_view())
                        .sampler(stored.sampler())
                        .build()
                } else {
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image_view(resource_manager.default_normal_texture().image_view())
                        .sampler(resource_manager.default_normal_texture().sampler())
                        .build()
                }
            } else {
                vk::DescriptorImageInfo::builder()
                    .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .image_view(resource_manager.default_normal_texture().image_view())
                    .sampler(resource_manager.default_normal_texture().sampler())
                    .build()
            };
                
            let metallic_roughness_image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(resource_manager.default_metallic_roughness_texture().image_view())
                .sampler(resource_manager.default_metallic_roughness_texture().sampler())
                .build();
                
            // AO texture uses the same as metallic-roughness for now
            let ao_image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(resource_manager.default_white_texture().image_view())
                .sampler(resource_manager.default_white_texture().sampler())
                .build();
            
            // (emission_image_info already computed above)
            
            // Opacity texture - default white (fully opaque)
            let opacity_image_info = vk::DescriptorImageInfo::builder()
                .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .image_view(resource_manager.default_white_texture().image_view())
                .sampler(resource_manager.default_white_texture().sampler())
                .build();
            
            let descriptor_writes = vec![
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(0)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[material_buffer_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(1)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[base_color_image_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(2)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[normal_image_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(3)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[metallic_roughness_image_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(4)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[ao_image_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(5)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[emission_image_info])
                    .build(),
                vk::WriteDescriptorSet::builder()
                    .dst_set(descriptor_set)
                    .dst_binding(6)
                    .dst_array_element(0)
                    .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                    .image_info(&[opacity_image_info])
                    .build(),
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
    
    /// Update lighting UBO data from MultiLightEnvironment
    pub fn update_multi_light_environment(&mut self, multi_light_env: &MultiLightEnvironment) {
        // Convert MultiLightEnvironment to shader-compatible UBO format
        
        // Convert directional lights
        let mut directional_lights = [DirectionalLightData {
            direction: [0.0; 4],
            color: [0.0; 4],
        }; 4];
        for i in 0..multi_light_env.header.directional_light_count.min(4) as usize {
            let src = &multi_light_env.directional_lights[i];
            directional_lights[i] = DirectionalLightData {
                direction: src.direction,
                color: src.color,
            };
        }
        
        // Convert point lights
        let mut point_lights = [PointLightData {
            position: [0.0; 4],
            color: [0.0; 4],
            attenuation: [1.0, 0.0, 0.0, 0.0],
        }; 8];
        for i in 0..multi_light_env.header.point_light_count.min(8) as usize {
            let src = &multi_light_env.point_lights[i];
            point_lights[i] = PointLightData {
                position: src.position,
                color: src.color,
                attenuation: src.attenuation,
            };
        }
        
        // Convert spot lights
        let mut spot_lights = [SpotLightData {
            position: [0.0; 4],
            direction: [0.0; 4],
            color: [0.0; 4],
            cone_angles: [0.0; 4],
        }; 4];
        for i in 0..multi_light_env.header.spot_light_count.min(4) as usize {
            let src = &multi_light_env.spot_lights[i];
            spot_lights[i] = SpotLightData {
                position: src.position,
                direction: src.direction,
                color: src.color,
                cone_angles: src.cone_angles,
            };
        }
        
        let lighting_ubo_data = MultiLightingUBO {
            ambient_color: multi_light_env.header.ambient_color,
            directional_light_count: multi_light_env.header.directional_light_count,
            point_light_count: multi_light_env.header.point_light_count,
            spot_light_count: multi_light_env.header.spot_light_count,
            _padding: 0,
            directional_lights,
            point_lights,
            spot_lights,
        };
        
        let lighting_bytes = unsafe { 
            std::slice::from_raw_parts(
                &lighting_ubo_data as *const _ as *const u8,
                std::mem::size_of::<MultiLightingUBO>()
            )
        };
        
        if let Err(e) = self.lighting_ubo_buffers[self.current_frame].write_data(lighting_bytes) {
            log::error!("Failed to update lighting UBO: {:?}", e);
        }
        
        // Update frame counter for next update
        self.current_frame = (self.current_frame + 1) % self.max_frames_in_flight;
    }
    
    /// FIXME: Legacy update lighting method for backwards compatibility
    /// This will be deprecated once all callers use update_multi_light_environment
    pub fn update_lighting(&mut self, direction: [f32; 3], intensity: f32, color: [f32; 3], ambient_intensity: f32) {
        let lighting_ubo_data = MultiLightingUBO {
            ambient_color: [ambient_intensity * 0.2, ambient_intensity * 0.2, ambient_intensity * 0.2, ambient_intensity], // Use ambient_intensity as alpha
            directional_light_count: 1,
            point_light_count: 0,
            spot_light_count: 0,
            _padding: 0,
            directional_lights: [
                DirectionalLightData {
                    direction: [direction[0], direction[1], direction[2], intensity],
                    color: [color[0] * intensity, color[1] * intensity, color[2] * intensity, 1.0], // Pre-apply intensity to color
                },
                DirectionalLightData {
                    direction: [0.0; 4],
                    color: [0.0; 4],
                },
                DirectionalLightData {
                    direction: [0.0; 4],
                    color: [0.0; 4],
                },
                DirectionalLightData {
                    direction: [0.0; 4],
                    color: [0.0; 4],
                },
            ],
            point_lights: [PointLightData {
                position: [0.0; 4],
                color: [0.0; 4],
                attenuation: [1.0, 0.0, 0.0, 0.0],
            }; 8],
            spot_lights: [SpotLightData {
                position: [0.0; 4],
                direction: [0.0; 4],
                color: [0.0; 4],
                cone_angles: [0.0; 4],
            }; 4],
        };
        
        let lighting_bytes = unsafe { 
            std::slice::from_raw_parts(
                &lighting_ubo_data as *const _ as *const u8,
                std::mem::size_of::<MultiLightingUBO>()
            )
        };
        
        if let Err(e) = self.lighting_ubo_buffers[self.current_frame].write_data(lighting_bytes) {
            log::error!("Failed to update lighting UBO: {:?}", e);
        }
    }
    
    /// Update material color only (legacy interface)
    pub fn update_material_color(&mut self, color: [f32; 4]) -> VulkanResult<()> {
        // For now, create a simple material with the given color
        use crate::render::resources::materials::{StandardMaterialParams, StandardMaterialUBO};
        
        let material_params = StandardMaterialParams {
            base_color: Vec3::new(color[0], color[1], color[2]),
            alpha: color[3],
            metallic: 0.1,
            roughness: 0.8,
            ambient_occlusion: 1.0,
            normal_scale: 1.0,
            emission: Vec3::new(0.0, 0.0, 0.0),
            emission_strength: 0.0,
            base_color_texture_enabled: false,
            normal_texture_enabled: false,
        };
        
        let material_ubo = StandardMaterialUBO::from_params(&material_params)
            .with_texture_flags(true, false, false, false) // Enable base_color texture
            .with_emission_texture(true); // Enable emission texture sampling
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
    /// Get the frame descriptor set layout for camera and lighting UBOs
    pub fn frame_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.frame_descriptor_set_layout.handle()
    }

    /// Get the material descriptor set layout for material UBO and textures  
    pub fn material_descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.material_descriptor_set_layout.handle()
    }
    
    /// Get the material UBO buffer handle for descriptor set binding
    pub fn material_ubo_buffer_handle(&self) -> vk::Buffer {
        self.material_ubo_buffer.handle()
    }
}
