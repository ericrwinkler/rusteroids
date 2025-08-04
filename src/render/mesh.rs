// src/render/mesh.rs
use crate::util::math::Vec3;
use ash::vk;
use std::path::Path;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn new(position: [f32; 3], normal: [f32; 3], tex_coord: [f32; 2]) -> Self {
        Self {
            position,
            normal,
            tex_coord,
        }
    }

    pub fn get_binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(std::mem::size_of::<Vertex>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn get_attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            // Position
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(0)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(0)
                .build(),
            // Normal
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(1)
                .format(vk::Format::R32G32B32_SFLOAT)
                .offset(12)
                .build(),
            // Texture Coordinate
            vk::VertexInputAttributeDescription::builder()
                .binding(0)
                .location(2)
                .format(vk::Format::R32G32_SFLOAT)
                .offset(24)
                .build(),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u32>) -> Self {
        Self {
            vertices,
            indices,
        }
    }

    // Load mesh from OBJ file - simplified version for now
    pub fn from_obj_file<P: AsRef<Path>>(_path: P) -> Result<Self, Box<dyn std::error::Error>> {
        // For now, return a simple sphere-like shape that resembles a teapot
        Ok(Self::teapot_like())
    }
    
    // Create a simple teapot-like shape using basic geometry
    pub fn teapot_like() -> Self {
        // Create a simple rounded shape that looks teapot-ish
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let rings = 8; // Number of horizontal rings
        let segments = 12; // Number of vertical segments
        
        // Generate vertices for a sphere-like shape
        for ring in 0..=rings {
            let v = ring as f32 / rings as f32;
            let phi = v * std::f32::consts::PI;
            
            for segment in 0..segments {
                let u = segment as f32 / segments as f32;
                let theta = u * 2.0 * std::f32::consts::PI;
                
                // Sphere coordinates
                let x = phi.sin() * theta.cos();
                let y = phi.cos();
                let z = phi.sin() * theta.sin();
                
                // Scale and offset to make it more teapot-like
                let scale_x = 1.2; // Wider
                let scale_y = 0.8; // Shorter
                let scale_z = 1.2; // Wider
                
                let position = [x * scale_x, y * scale_y, z * scale_z];
                let normal = [x, y, z]; // Unit sphere normal
                let tex_coord = [u, v];
                
                vertices.push(Vertex::new(position, normal, tex_coord));
            }
        }
        
        // Generate indices for triangles
        for ring in 0..rings {
            for segment in 0..segments {
                let current = ring * segments + segment;
                let next = ring * segments + (segment + 1) % segments;
                let below = (ring + 1) * segments + segment;
                let below_next = (ring + 1) * segments + (segment + 1) % segments;
                
                if ring < rings {
                    // First triangle
                    indices.push(current as u32);
                    indices.push(below as u32);
                    indices.push(next as u32);
                    
                    // Second triangle
                    indices.push(next as u32);
                    indices.push(below as u32);
                    indices.push(below_next as u32);
                }
            }
        }
        
        Self::new(vertices, indices)
    }

    // Calculate normals for vertices when they're missing from the OBJ file
    fn calculate_normals(vertices: &mut [Vertex], indices: &[u32]) {
        // Reset all normals to zero
        for vertex in vertices.iter_mut() {
            vertex.normal = [0.0, 0.0, 0.0];
        }

        // Calculate face normals and accumulate into vertex normals
        for triangle in indices.chunks(3) {
            if triangle.len() == 3 {
                let i0 = triangle[0] as usize;
                let i1 = triangle[1] as usize;
                let i2 = triangle[2] as usize;

                if i0 < vertices.len() && i1 < vertices.len() && i2 < vertices.len() {
                    let v0 = Vec3::new(
                        vertices[i0].position[0],
                        vertices[i0].position[1],
                        vertices[i0].position[2],
                    );
                    let v1 = Vec3::new(
                        vertices[i1].position[0],
                        vertices[i1].position[1],
                        vertices[i1].position[2],
                    );
                    let v2 = Vec3::new(
                        vertices[i2].position[0],
                        vertices[i2].position[1],
                        vertices[i2].position[2],
                    );

                    // Calculate face normal using cross product
                    let edge1 = v1 - v0;
                    let edge2 = v2 - v0;
                    let face_normal = edge1.cross(edge2).normalized();

                    // Add face normal to each vertex normal
                    for &idx in triangle {
                        let vertex = &mut vertices[idx as usize];
                        vertex.normal[0] += face_normal.x;
                        vertex.normal[1] += face_normal.y;
                        vertex.normal[2] += face_normal.z;
                    }
                }
            }
        }

        // Normalize all vertex normals
        for vertex in vertices.iter_mut() {
            let normal = Vec3::new(vertex.normal[0], vertex.normal[1], vertex.normal[2]);
            let normalized = normal.normalized();
            vertex.normal = [normalized.x, normalized.y, normalized.z];
        }
    }

    // Create a simple cube mesh for testing
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
