//! Mesh representation for 3D models

use crate::assets::{Asset, AssetError};
use ash::vk;

/// 3D vertex data
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Position in 3D space
    pub position: [f32; 3],
    
    /// Normal vector
    pub normal: [f32; 3],
    
    /// Texture coordinates
    pub tex_coord: [f32; 2],
}

impl Vertex {
    /// Create a new vertex
    pub fn new(position: [f32; 3], normal: [f32; 3], tex_coord: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            tex_coord,
        }
    }
    
    /// Get vertex input binding description
    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }
    
    /// Get vertex input attribute descriptions
    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            // Position attribute (location = 0)
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 0,
            },
            // Normal attribute (location = 1)
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: 12, // 3 * sizeof(f32)
            },
            // Texture coordinate attribute (location = 2)
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32_SFLOAT,
                offset: 24, // 6 * sizeof(f32)
            },
        ]
    }
    
    /// Create vertex input state create info
    pub fn get_input_state() -> (vk::VertexInputBindingDescription, [vk::VertexInputAttributeDescription; 3]) {
        let binding_description = Self::get_binding_description();
        let attribute_descriptions = Self::get_attribute_descriptions();
        
        (binding_description, attribute_descriptions)
    }
}

/// 3D mesh containing vertices and indices
#[derive(Debug, Clone)]
pub struct Mesh {
    /// Vertex data
    pub vertices: Vec<Vertex>,
    
    /// Index data for triangles
    pub indices: Vec<u32>,
}

impl Mesh {
    /// Create a new mesh
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self { vertices, indices }
    }
    
    /// Create a cube mesh for testing
    pub fn cube() -> Self {
        let vertices = vec![
            // Front face
            Vertex::new([-1.0, -1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0]),
            Vertex::new([1.0, -1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 0.0]),
            Vertex::new([1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [1.0, 1.0]),
            Vertex::new([-1.0, 1.0, 1.0], [0.0, 0.0, 1.0], [0.0, 1.0]),
            // Back face
            Vertex::new([-1.0, -1.0, -1.0], [0.0, 0.0, -1.0], [1.0, 0.0]),
            Vertex::new([-1.0, 1.0, -1.0], [0.0, 0.0, -1.0], [1.0, 1.0]),
            Vertex::new([1.0, 1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 1.0]),
            Vertex::new([1.0, -1.0, -1.0], [0.0, 0.0, -1.0], [0.0, 0.0]),
        ];

        let indices = vec![
            // Front
            0, 1, 2, 2, 3, 0,
            // Back
            4, 5, 6, 6, 7, 4,
            // Left
            4, 0, 3, 3, 5, 4,
            // Right
            1, 7, 6, 6, 2, 1,
            // Top
            3, 2, 6, 6, 5, 3,
            // Bottom
            4, 7, 1, 1, 0, 4,
        ];

        Self::new(vertices, indices)
    }
}

impl Asset for Mesh {
    fn from_bytes(_bytes: &[u8]) -> Result<Self, AssetError> 
    where 
        Self: Sized 
    {
        // TODO: Implement OBJ loading from bytes
        Err(AssetError::NotFound("Mesh loading not implemented".to_string()))
    }
}
