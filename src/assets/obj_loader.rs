// src/assets/obj_loader.rs
use crate::render::mesh::{Mesh, Vertex};
use crate::util::math::Vec3;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub struct ObjLoader;

impl ObjLoader {
    pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<Mesh, Box<dyn std::error::Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut positions: Vec<Vec3> = Vec::new();
        let mut normals: Vec<Vec3> = Vec::new();
        let mut uvs: Vec<(f32, f32)> = Vec::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            
            match parts[0] {
                "v" => {
                    // Vertex position
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse()?;
                        let y: f32 = parts[2].parse()?;
                        let z: f32 = parts[3].parse()?;
                        positions.push(Vec3::new(x, y, z));
                    }
                }
                "vn" => {
                    // Vertex normal
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse()?;
                        let y: f32 = parts[2].parse()?;
                        let z: f32 = parts[3].parse()?;
                        normals.push(Vec3::new(x, y, z));
                    }
                }
                "vt" => {
                    // Texture coordinate
                    if parts.len() >= 3 {
                        let u: f32 = parts[1].parse()?;
                        let v: f32 = parts[2].parse()?;
                        uvs.push((u, v));
                    }
                }
                "f" => {
                    // Face - we'll triangulate if needed
                    if parts.len() >= 4 {
                        let face_vertices = Self::parse_face(&parts[1..], &positions, &normals, &uvs)?;
                        
                        // Triangulate the face (assuming it's a polygon)
                        for i in 1..face_vertices.len() - 1 {
                            let base_index = vertices.len() as u32;
                            
                            vertices.push(face_vertices[0].clone());
                            vertices.push(face_vertices[i].clone());
                            vertices.push(face_vertices[i + 1].clone());
                            
                            indices.push(base_index);
                            indices.push(base_index + 1);
                            indices.push(base_index + 2);
                        }
                    }
                }
                _ => {
                    // Ignore other directives
                }
            }
        }
        
        // If no normals were provided, calculate them
        if vertices.iter().all(|v| v.normal == [0.0, 0.0, 0.0]) {
            Self::calculate_normals(&mut vertices, &indices);
        }
        
        // DEBUG: Print coordinate bounds to see if they're reasonable
        if !vertices.is_empty() {
            let mut min_x = vertices[0].position[0];
            let mut max_x = vertices[0].position[0];
            let mut min_y = vertices[0].position[1];
            let mut max_y = vertices[0].position[1];
            let mut min_z = vertices[0].position[2];
            let mut max_z = vertices[0].position[2];
            
            for vertex in &vertices {
                min_x = min_x.min(vertex.position[0]);
                max_x = max_x.max(vertex.position[0]);
                min_y = min_y.min(vertex.position[1]);
                max_y = max_y.max(vertex.position[1]);
                min_z = min_z.min(vertex.position[2]);
                max_z = max_z.max(vertex.position[2]);
            }
            
            println!("Model bounds: X({:.3} to {:.3}), Y({:.3} to {:.3}), Z({:.3} to {:.3})", 
                     min_x, max_x, min_y, max_y, min_z, max_z);
        }

        println!("Loaded OBJ: {} vertices, {} triangles", vertices.len(), indices.len() / 3);
        
        Ok(Mesh { vertices, indices })
    }
    
    fn parse_face(
        face_parts: &[&str],
        positions: &[Vec3],
        normals: &[Vec3],
        uvs: &[(f32, f32)],
    ) -> Result<Vec<Vertex>, Box<dyn std::error::Error>> {
        let mut face_vertices = Vec::new();
        
        for part in face_parts {
            let indices: Vec<&str> = part.split('/').collect();
            
            // Parse vertex index (1-based in OBJ format)
            let pos_index: usize = indices[0].parse::<usize>()? - 1;
            let position = positions.get(pos_index)
                .ok_or("Invalid vertex position index")?;
            
            // Parse UV index if present
            let uv = if indices.len() > 1 && !indices[1].is_empty() {
                let uv_index: usize = indices[1].parse::<usize>()? - 1;
                uvs.get(uv_index).copied().unwrap_or((0.0, 0.0))
            } else {
                (0.0, 0.0)
            };
            
            // Parse normal index if present
            let normal = if indices.len() > 2 && !indices[2].is_empty() {
                let normal_index: usize = indices[2].parse::<usize>()? - 1;
                normals.get(normal_index).copied().unwrap_or(Vec3::new(0.0, 1.0, 0.0))
            } else {
                Vec3::new(0.0, 1.0, 0.0) // Default normal
            };
            
            face_vertices.push(Vertex {
                position: [position.x, position.y, position.z],
                normal: [normal.x, normal.y, normal.z],
                tex_coord: [uv.0, uv.1],
            });
        }
        
        Ok(face_vertices)
    }
    
    fn calculate_normals(vertices: &mut [Vertex], indices: &[u32]) {
        // Reset all normals
        for vertex in vertices.iter_mut() {
            vertex.normal = [0.0, 0.0, 0.0];
        }
        
        // Calculate face normals and accumulate
        for triangle in indices.chunks(3) {
            if triangle.len() != 3 {
                continue;
            }
            
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;
            
            if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                continue;
            }
            
            let v0 = vertices[i0].position;
            let v1 = vertices[i1].position;
            let v2 = vertices[i2].position;
            
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            
            // Cross product for face normal
            let face_normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            
            // Accumulate normals
            vertices[i0].normal[0] += face_normal[0];
            vertices[i0].normal[1] += face_normal[1];
            vertices[i0].normal[2] += face_normal[2];
            
            vertices[i1].normal[0] += face_normal[0];
            vertices[i1].normal[1] += face_normal[1];
            vertices[i1].normal[2] += face_normal[2];
            
            vertices[i2].normal[0] += face_normal[0];
            vertices[i2].normal[1] += face_normal[1];
            vertices[i2].normal[2] += face_normal[2];
        }
        
        // Normalize all normals
        for vertex in vertices.iter_mut() {
            let length = (vertex.normal[0] * vertex.normal[0] +
                         vertex.normal[1] * vertex.normal[1] +
                         vertex.normal[2] * vertex.normal[2]).sqrt();
            
            if length > 0.0 {
                vertex.normal[0] /= length;
                vertex.normal[1] /= length;
                vertex.normal[2] /= length;
            } else {
                vertex.normal = [0.0, 1.0, 0.0];
            }
        }
    }
}
