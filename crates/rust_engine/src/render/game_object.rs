//! GameObject module for per-object rendering resources
//!
//! This module provides the GameObject struct which encapsulates per-object rendering
//! resources including transforms, uniform buffers, descriptor sets, and materials.
//! This is the foundation for efficient multiple objects rendering using the Vulkan
//! command recording pattern.

use crate::foundation::math::{Vec3, Mat4, Mat4Ext};
use crate::render::material::Material;
use crate::render::vulkan::{buffer::Buffer, VulkanContext, VulkanResult};
use ash::vk;
use std::ptr;

/// Per-object uniform buffer data structure
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct ObjectUBO {
    pub model_matrix: [[f32; 4]; 4],
    pub normal_matrix: [[f32; 3]; 3],  // For lighting calculations
    pub _padding: [f32; 3],            // Ensure proper alignment
}

impl ObjectUBO {
    pub fn new(model_matrix: Mat4) -> Self {
        let normal_matrix = Self::calculate_normal_matrix(&model_matrix);
        Self {
            model_matrix: model_matrix.into(),
            normal_matrix,
            _padding: [0.0; 3],
        }
    }

    /// Calculate normal matrix for lighting calculations
    /// This is the transpose of the inverse of the upper-left 3x3 of the model matrix
    fn calculate_normal_matrix(model_matrix: &Mat4) -> [[f32; 3]; 3] {
        // For uniform scaling, we can use the transpose of the rotation part
        // For non-uniform scaling, we would need the transpose of the inverse
        // Convert matrix to array and extract elements
        let m: [[f32; 4]; 4] = (*model_matrix).into();
        [
            [m[0][0], m[1][0], m[2][0]],  // First column elements 0,1,2
            [m[0][1], m[1][1], m[2][1]],  // Second column elements 4,5,6  
            [m[0][2], m[1][2], m[2][2]],  // Third column elements 8,9,10
        ]
    }
}

/// GameObject represents a single renderable object with its own transform and material
pub struct GameObject {
    // Transform properties
    pub position: Vec3,
    pub rotation: Vec3,  // Euler angles in radians
    pub scale: Vec3,

    // Per-object GPU resources (one per frame-in-flight)
    pub uniform_buffers: Vec<Buffer>,
    pub uniform_mapped: Vec<*mut u8>,

    // Per-object descriptor sets (one per frame-in-flight)
    pub descriptor_sets: Vec<vk::DescriptorSet>,

    // Material for this object
    pub material: Material,
}

impl GameObject {
    /// Create a new GameObject with the specified transform and material
    pub fn new(position: Vec3, rotation: Vec3, scale: Vec3, material: Material) -> Self {
        Self {
            position,
            rotation,
            scale,
            material,
            uniform_buffers: Vec::new(),
            uniform_mapped: Vec::new(),
            descriptor_sets: Vec::new(),
        }
    }

    /// Calculate the model matrix for this object
    pub fn get_model_matrix(&self) -> Mat4 {
        let translation = Mat4::new_translation(&self.position);
        let rotation_x = Mat4::rotation_x(self.rotation.x);
        let rotation_y = Mat4::rotation_y(self.rotation.y);
        let rotation_z = Mat4::rotation_z(self.rotation.z);
        let scale_matrix = Mat4::new_nonuniform_scaling(&self.scale);
        
        // Apply transforms in the order: scale -> rotation -> translation
        translation * rotation_z * rotation_y * rotation_x * scale_matrix
    }

    /// Create GPU resources for this object (uniform buffers and descriptor sets)
    pub fn create_resources(
        &mut self,
        context: &VulkanContext,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        max_frames_in_flight: usize,
    ) -> VulkanResult<()> {
        // Create uniform buffers for each frame in flight
        for _i in 0..max_frames_in_flight {
            let buffer_size = std::mem::size_of::<ObjectUBO>() as vk::DeviceSize;

            // Create uniform buffer using the existing Buffer::new method
            let buffer = Buffer::new(
                context.raw_device(),
                context.instance().clone(),
                context.physical_device().device,
                buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;

            // Map memory for updates
            let mapped = buffer.map_memory()? as *mut u8;

            self.uniform_buffers.push(buffer);
            self.uniform_mapped.push(mapped);
        }

        // Create descriptor sets
        self.create_descriptor_sets(
            context,
            descriptor_pool,
            descriptor_set_layout,
            max_frames_in_flight,
        )?;

        Ok(())
    }

    /// Update the uniform buffer for the specified frame
    pub fn update_uniform_buffer(&self, frame_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if frame_index >= self.uniform_mapped.len() {
            return Err("Invalid frame index".into());
        }

        let ubo = ObjectUBO::new(self.get_model_matrix());

        unsafe {
            let mapped = self.uniform_mapped[frame_index];
            ptr::copy_nonoverlapping(
                &ubo as *const ObjectUBO as *const u8,
                mapped,
                std::mem::size_of::<ObjectUBO>(),
            );
        }

        Ok(())
    }

    /// Get the descriptor set for the specified frame
    pub fn get_descriptor_set(&self, frame_index: usize) -> Option<vk::DescriptorSet> {
        self.descriptor_sets.get(frame_index).copied()
    }

    /// Create descriptor sets for this object
    fn create_descriptor_sets(
        &mut self,
        context: &VulkanContext,
        descriptor_pool: &vk::DescriptorPool,
        descriptor_set_layout: &vk::DescriptorSetLayout,
        max_frames_in_flight: usize,
    ) -> VulkanResult<()> {
        let layouts = vec![*descriptor_set_layout; max_frames_in_flight];
        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(*descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { 
            context.raw_device().allocate_descriptor_sets(&alloc_info)
                .map_err(crate::render::vulkan::VulkanError::Api)?
        };

        // Update descriptor sets with uniform buffer info
        for (i, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::builder()
                .buffer(self.uniform_buffers[i].handle())
                .offset(0)
                .range(std::mem::size_of::<ObjectUBO>() as u64);

            let descriptor_write = vk::WriteDescriptorSet::builder()
                .dst_set(descriptor_set)
                .dst_binding(0) // Binding 0 for object uniform buffer
                .dst_array_element(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info));

            unsafe {
                context.raw_device().update_descriptor_sets(std::slice::from_ref(&descriptor_write), &[]);
            }
        }

        self.descriptor_sets = descriptor_sets;
        Ok(())
    }
}

impl Drop for GameObject {
    fn drop(&mut self) {
        // Note: Buffer structs automatically handle cleanup in their Drop implementation
        // We just need to unmap any mapped memory pointers
        for buffer in &self.uniform_buffers {
            buffer.unmap_memory();
        }
    }
}

/// Cleanup resources for a GameObject
/// This is called explicitly before dropping to ensure proper cleanup order
pub fn cleanup_game_object(game_object: &mut GameObject) {
    // Unmap memory (handled automatically by Buffer::Drop)
    for buffer in &game_object.uniform_buffers {
        buffer.unmap_memory();
    }

    // Clear vectors (buffers will be dropped automatically)
    game_object.uniform_buffers.clear();
    game_object.uniform_mapped.clear();
    game_object.descriptor_sets.clear();
}
