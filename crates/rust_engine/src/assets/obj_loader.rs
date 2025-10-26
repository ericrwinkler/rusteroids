//! OBJ file loader for 3D models with vertex deduplication
//! 
//! This loader implements HashMap-based vertex deduplication to eliminate duplicate
//! vertices during mesh loading. This optimization typically reduces vertex count
//! by 5-6x, significantly improving memory usage and rendering performance.

use crate::render::{Mesh, Vertex};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

/// Errors that can occur during OBJ file loading
#[derive(Error, Debug)]
pub enum ObjError {
    /// I/O error during file operations
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Error parsing OBJ file format
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Invalid or unsupported OBJ format
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// OBJ file loader utility
pub struct ObjLoader;

impl ObjLoader {
    /// Load an OBJ file and return a mesh with vertex deduplication
    /// 
    /// This method implements efficient vertex deduplication using a HashMap to eliminate
    /// duplicate vertices. The deduplication process typically reduces vertex count by 5-6x
    /// compared to naive loading, significantly improving memory usage and rendering performance.
    pub fn load_obj<P: AsRef<Path>>(path: P) -> Result<Mesh, ObjError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut tex_coords = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        // HashMap for vertex deduplication - maps unique vertices to their indices
        let mut unique_vertices: HashMap<Vertex, usize> = HashMap::new();
        
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
                        let x: f32 = parts[1].parse().map_err(|_| ObjError::ParseError("Invalid vertex x".to_string()))?;
                        let y: f32 = parts[2].parse().map_err(|_| ObjError::ParseError("Invalid vertex y".to_string()))?;
                        let z: f32 = parts[3].parse().map_err(|_| ObjError::ParseError("Invalid vertex z".to_string()))?;
                        positions.push([x, y, z]);
                    }
                }
                "vn" => {
                    // Vertex normal
                    if parts.len() >= 4 {
                        let x: f32 = parts[1].parse().map_err(|_| ObjError::ParseError("Invalid normal x".to_string()))?;
                        let y: f32 = parts[2].parse().map_err(|_| ObjError::ParseError("Invalid normal y".to_string()))?;
                        let z: f32 = parts[3].parse().map_err(|_| ObjError::ParseError("Invalid normal z".to_string()))?;
                        normals.push([x, y, z]);
                    }
                }
                "vt" => {
                    // Texture coordinate
                    if parts.len() >= 3 {
                        let u: f32 = parts[1].parse().map_err(|_| ObjError::ParseError("Invalid tex coord u".to_string()))?;
                        let v: f32 = parts[2].parse().map_err(|_| ObjError::ParseError("Invalid tex coord v".to_string()))?;
                        tex_coords.push([u, v]);
                    }
                }
                "f" => {
                    // Face
                    if parts.len() >= 4 {
                        // Parse face indices (simplified - assumes triangulated faces)
                        let mut face_indices = Vec::new();
                        
                        for i in 1..parts.len() {
                            let vertex_data = parts[i];
                            let indices_parts: Vec<&str> = vertex_data.split('/').collect();
                            
                            if indices_parts.is_empty() {
                                continue;
                            }
                            
                            // Parse position index (1-based in OBJ)
                            let pos_idx: usize = indices_parts[0].parse()
                                .map_err(|_| ObjError::ParseError("Invalid position index".to_string()))?;
                            let pos_idx = pos_idx - 1; // Convert to 0-based
                            
                            // Parse texture coordinate index if present
                            let tex_idx = if indices_parts.len() > 1 && !indices_parts[1].is_empty() {
                                indices_parts[1].parse::<usize>()
                                    .map(|i| i - 1) // Convert to 0-based
                                    .ok()
                            } else {
                                None
                            };
                            
                            // Parse normal index if present
                            let normal_idx = if indices_parts.len() > 2 && !indices_parts[2].is_empty() {
                                indices_parts[2].parse::<usize>()
                                    .map(|i| i - 1) // Convert to 0-based
                                    .ok()
                            } else {
                                None
                            };
                            
                            // Get position
                            let position = positions.get(pos_idx)
                                .ok_or_else(|| ObjError::InvalidFormat("Position index out of bounds".to_string()))?;
                            
                            // Get texture coordinate or use default
                            let tex_coord = tex_idx
                                .and_then(|idx| tex_coords.get(idx))
                                .unwrap_or(&[0.0, 0.0]);
                            
                            // Get normal or use default (zero normal indicates missing)
                            let normal = normal_idx
                                .and_then(|idx| normals.get(idx))
                                .unwrap_or(&[0.0, 0.0, 0.0]);  // Zero normal indicates missing
                            
                            // Create vertex from parsed components
                            let vertex = Vertex {
                                position: *position,
                                normal: *normal,
                                tex_coord: *tex_coord,
                                tangent: [0.0, 0.0, 0.0], // Will be calculated later
                                _padding: 0.0,
                            };
                            
                            // Vertex deduplication: check if this exact vertex already exists
                            let vertex_index = if let Some(&existing_index) = unique_vertices.get(&vertex) {
                                // Reuse existing vertex index
                                existing_index
                            } else {
                                // New unique vertex - add to vertices and track in HashMap
                                let new_index = vertices.len();
                                vertices.push(vertex);
                                unique_vertices.insert(vertex, new_index);
                                new_index
                            };
                            
                            face_indices.push(vertex_index);
                        }
                        
                        // Triangulate face (simple fan triangulation)
                        for i in 1..(face_indices.len() - 1) {
                            indices.push(face_indices[0] as u32);
                            indices.push(face_indices[i] as u32);
                            indices.push(face_indices[i + 1] as u32);
                        }
                    }
                }
                _ => {
                    // Ignore other commands
                }
            }
        }
        
        if vertices.is_empty() {
            return Err(ObjError::InvalidFormat("No vertices found in OBJ file".to_string()));
        }
        
        // Generate normals if the mesh doesn't have them
        let needs_normals = vertices.iter().all(|v| v.normal == [0.0, 0.0, 0.0]);
        if needs_normals {
            log::info!("OBJ file has no normals, generating them automatically");
            Self::generate_normals(&mut vertices, &indices);
        }
        
        // Log deduplication statistics
        let total_vertex_references = indices.len();
        let unique_vertices = vertices.len();
        let deduplication_ratio = total_vertex_references as f32 / unique_vertices as f32;
        log::info!(
            "OBJ loading complete: {} unique vertices from {} total references ({}x deduplication)",
            unique_vertices, total_vertex_references, deduplication_ratio
        );
        
        // Calculate tangents for normal mapping
        Self::calculate_tangents(&mut vertices, &indices);
        
        Ok(Mesh::new(vertices, indices))
    }
    
    /// Generate face normals for a mesh (using Vulkan coordinate system)
    fn generate_normals(vertices: &mut [Vertex], indices: &[u32]) {
        // First, zero out all normals
        for vertex in vertices.iter_mut() {
            vertex.normal = [0.0, 0.0, 0.0];
        }
        
        // Calculate face normals and accumulate them to vertex normals
        for triangle in indices.chunks(3) {
            if triangle.len() != 3 { continue; }
            
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;
            
            if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                continue;
            }
            
            let v0 = vertices[i0].position;
            let v1 = vertices[i1].position;
            let v2 = vertices[i2].position;
            
            // Calculate two edge vectors
            let edge1 = [v1[0] - v0[0], v1[1] - v0[1], v1[2] - v0[2]];
            let edge2 = [v2[0] - v0[0], v2[1] - v0[1], v2[2] - v0[2]];
            
            // Calculate cross product for face normal (right-handed cross product)
            let face_normal = [
                edge1[1] * edge2[2] - edge1[2] * edge2[1],
                edge1[2] * edge2[0] - edge1[0] * edge2[2],
                edge1[0] * edge2[1] - edge1[1] * edge2[0],
            ];
            
            // Add face normal to each vertex normal (for smooth shading)
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
        
        // Normalize all vertex normals
        for vertex in vertices.iter_mut() {
            let normal = &mut vertex.normal;
            let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
            if length > 0.0 {
                normal[0] /= length;
                normal[1] /= length;
                normal[2] /= length;
            } else {
                // Fallback for degenerate normals (standard Y-up convention)
                *normal = [0.0, 1.0, 0.0];
            }
        }
    }
    
    /// Calculate tangent vectors for normal mapping using triangle edge vectors and UV deltas
    fn calculate_tangents(vertices: &mut [Vertex], indices: &[u32]) {
        // First, zero out all tangents
        for vertex in vertices.iter_mut() {
            vertex.tangent = [0.0, 0.0, 0.0];
        }
        
        // Calculate tangents per triangle and accumulate
        for triangle in indices.chunks(3) {
            if triangle.len() != 3 { continue; }
            
            let i0 = triangle[0] as usize;
            let i1 = triangle[1] as usize;
            let i2 = triangle[2] as usize;
            
            if i0 >= vertices.len() || i1 >= vertices.len() || i2 >= vertices.len() {
                continue;
            }
            
            let v0 = &vertices[i0];
            let v1 = &vertices[i1];
            let v2 = &vertices[i2];
            
            // Position deltas
            let edge1 = [
                v1.position[0] - v0.position[0],
                v1.position[1] - v0.position[1],
                v1.position[2] - v0.position[2],
            ];
            let edge2 = [
                v2.position[0] - v0.position[0],
                v2.position[1] - v0.position[1],
                v2.position[2] - v0.position[2],
            ];
            
            // UV deltas
            let delta_uv1 = [
                v1.tex_coord[0] - v0.tex_coord[0],
                v1.tex_coord[1] - v0.tex_coord[1],
            ];
            let delta_uv2 = [
                v2.tex_coord[0] - v0.tex_coord[0],
                v2.tex_coord[1] - v0.tex_coord[1],
            ];
            
            // Calculate tangent using the formula:
            // T = (edge1 * deltaUV2.y - edge2 * deltaUV1.y) / determinant
            let det = delta_uv1[0] * delta_uv2[1] - delta_uv1[1] * delta_uv2[0];
            
            let tangent = if det.abs() > 1e-6 {
                let r = 1.0 / det;
                [
                    r * (delta_uv2[1] * edge1[0] - delta_uv1[1] * edge2[0]),
                    r * (delta_uv2[1] * edge1[1] - delta_uv1[1] * edge2[1]),
                    r * (delta_uv2[1] * edge1[2] - delta_uv1[1] * edge2[2]),
                ]
            } else {
                // Degenerate UV case - use fallback tangent aligned with X axis
                [1.0, 0.0, 0.0]
            };
            
            // Accumulate tangent to all three vertices of the triangle
            vertices[i0].tangent[0] += tangent[0];
            vertices[i0].tangent[1] += tangent[1];
            vertices[i0].tangent[2] += tangent[2];
            
            vertices[i1].tangent[0] += tangent[0];
            vertices[i1].tangent[1] += tangent[1];
            vertices[i1].tangent[2] += tangent[2];
            
            vertices[i2].tangent[0] += tangent[0];
            vertices[i2].tangent[1] += tangent[1];
            vertices[i2].tangent[2] += tangent[2];
        }
        
        // Normalize tangents and orthogonalize with respect to normals (Gram-Schmidt)
        for vertex in vertices.iter_mut() {
            let t = vertex.tangent;
            let n = vertex.normal;
            
            // Gram-Schmidt orthogonalization: t' = t - (t·n)n
            let dot = t[0] * n[0] + t[1] * n[1] + t[2] * n[2];
            let mut tangent = [
                t[0] - dot * n[0],
                t[1] - dot * n[1],
                t[2] - dot * n[2],
            ];
            
            // Normalize
            let length = (tangent[0] * tangent[0] + tangent[1] * tangent[1] + tangent[2] * tangent[2]).sqrt();
            if length > 1e-6 {
                tangent[0] /= length;
                tangent[1] /= length;
                tangent[2] /= length;
            } else {
                // Fallback tangent if normalization fails
                tangent = [1.0, 0.0, 0.0];
            }
            
            vertex.tangent = tangent;
        }
    }
}
