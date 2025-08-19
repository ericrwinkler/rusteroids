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
}
