//! Vulkan-specific vertex layout definitions
//!
//! This module contains Vulkan-specific vertex input state descriptions that were
//! previously incorrectly placed in the high-level mesh module. This separation
//! ensures the core mesh types remain backend-agnostic.

use ash::vk;
use crate::render::mesh::Vertex;

/// Vulkan vertex layout implementation for the engine's Vertex type
pub struct VulkanVertexLayout;

impl VulkanVertexLayout {
    /// Get Vulkan vertex input binding description for Vertex
    /// 
    /// This describes how vertex data is organized in buffer memory for Vulkan.
    /// The binding describes the rate at which vertex attributes advance and
    /// the stride between consecutive vertices.
    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }
    
    /// Get Vulkan vertex input attribute descriptions for Vertex
    /// 
    /// This describes the layout of individual vertex attributes (position, normal, 
    /// texture coordinates) within the vertex structure for Vulkan shader input.
    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            // Position attribute (location = 0)
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0, // Start of struct
            },
            // Normal attribute (location = 1)
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 12, // 3 * sizeof(f32) after position
            },
            // Texture coordinate attribute (location = 2)
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: 24, // 6 * sizeof(f32) after position and normal
            },
        ]
    }
    
    /// Get vertex input state for pipeline creation
    /// 
    /// Convenience method that returns the complete vertex input state needed
    /// for Vulkan graphics pipeline creation.
    pub fn get_input_state() -> (vk::VertexInputBindingDescription, [vk::VertexInputAttributeDescription; 3]) {
        (Self::get_binding_description(), Self::get_attribute_descriptions())
    }
    
    /// Get instanced vertex input state for dynamic object rendering
    /// 
    /// Returns vertex input state for instanced rendering with:
    /// - Binding 0: Regular vertex data (position, normal, tex_coord)  
    /// - Binding 1: Instance data (model matrix, normal matrix, material color, emission color, texture flags, material index)
    pub fn get_instanced_input_state() -> (
        [vk::VertexInputBindingDescription; 2], 
        [vk::VertexInputAttributeDescription; 15] // 3 vertex attrs + 12 instance attrs
    ) {
        let vertex_binding = Self::get_binding_description();
        let instance_binding = Self::get_instance_binding_description();
        
        let vertex_attributes = Self::get_attribute_descriptions();
        let instance_attributes = Self::get_instance_attribute_descriptions();
        
        // Combine bindings
        let bindings = [vertex_binding, instance_binding];
        
        // Combine attributes (3 vertex + 12 instance = 15 total)
        let mut attributes = [vk::VertexInputAttributeDescription::default(); 15];
        
        // Copy vertex attributes (locations 0-2)
        attributes[0] = vertex_attributes[0];
        attributes[1] = vertex_attributes[1]; 
        attributes[2] = vertex_attributes[2];
        
        // Copy instance attributes (locations 3-14)
        for (i, attr) in instance_attributes.iter().enumerate() {
            attributes[3 + i] = *attr;
        }
        
        (bindings, attributes)
    }
    
    /// Get instance binding description for instanced rendering
    fn get_instance_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 1,
            stride: std::mem::size_of::<crate::render::dynamic::InstanceData>() as u32,
            input_rate: vk::VertexInputRate::INSTANCE,
        }
    }
    
    /// Get instance attribute descriptions for instanced rendering
    /// 
    /// Instance data layout:
    /// - Locations 3-6: Model matrix (4x vec4)
    /// - Locations 7-10: Normal matrix (4x vec4, only first 3 used)
    /// - Location 11: Material color (vec4)
    /// - Location 12: Emission color (vec4)
    /// - Location 13: Texture enable flags (uvec4)
    /// - Location 14: Material index (uint)
    fn get_instance_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 12] {
        use std::mem::size_of;
        
        [
            // Model matrix (4x4 = 4 vec4s at locations 3-6)
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 3,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: 0,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 4,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: size_of::<[f32; 4]>() as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 5,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (2 * size_of::<[f32; 4]>()) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 6,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (3 * size_of::<[f32; 4]>()) as u32,
            },
            
            // Normal matrix (4x4 = 4 vec4s at locations 7-10, padded)
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 7,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (4 * size_of::<[f32; 4]>()) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 8,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (5 * size_of::<[f32; 4]>()) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 9,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (6 * size_of::<[f32; 4]>()) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 10,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (7 * size_of::<[f32; 4]>()) as u32,
            },
            
            // Material color (vec4 at location 11)
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 11,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (8 * size_of::<[f32; 4]>()) as u32,
            },
            
            // Emission color (vec4 at location 12)
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 12,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: (9 * size_of::<[f32; 4]>()) as u32,
            },
            
            // Texture enable flags (uvec4 at location 13)
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 13,
                format: vk::Format::R32G32B32A32_UINT,
                offset: (10 * size_of::<[f32; 4]>()) as u32,
            },
            
            // Material index (uint at location 14)
            vk::VertexInputAttributeDescription {
                binding: 1,
                location: 14,
                format: vk::Format::R32_UINT,
                offset: (10 * size_of::<[f32; 4]>() + size_of::<[u32; 4]>()) as u32,
            },
        ]
    }
}
