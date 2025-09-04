#!/usr/bin/env rust-script
//! OBJ Normal Generator
//! 
//! This script takes a "classic" OBJ file without normals and generates
//! a modern OBJ file with proper vertex normals included.
//! 
//! Usage: cargo run --bin obj_normalizer input.obj output.obj
//! 
//! This converts the classic Utah Teapot (and other meshes) to modern
//! standards with smooth vertex normals for proper lighting.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
    
    fn normalize(&self) -> Self {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if len > 0.0 {
            Self::new(self.x / len, self.y / len, self.z / len)
        } else {
            Self::new(0.0, 1.0, 0.0) // Default up vector
        }
    }
    
    fn cross(&self, other: &Vec3) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }
    
    fn sub(&self, other: &Vec3) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }
    
    fn add(&self, other: &Vec3) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }
}

#[derive(Debug, Clone)]
struct Face {
    vertices: Vec<usize>, // 0-based vertex indices
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} input.obj output.obj", args[0]);
        eprintln!("Converts classic OBJ files to modern format with vertex normals");
        std::process::exit(1);
    }
    
    let input_path = &args[1];
    let output_path = &args[2];
    
    match process_obj(input_path, output_path) {
        Ok(stats) => {
            println!("✅ Successfully processed OBJ file!");
            println!("   Input:  {}", input_path);
            println!("   Output: {}", output_path);
            println!("   Stats:  {} vertices, {} faces, {} normals generated", 
                     stats.vertices, stats.faces, stats.normals);
        }
        Err(e) => {
            eprintln!("❌ Error processing OBJ file: {}", e);
            std::process::exit(1);
        }
    }
}

#[derive(Default)]
struct ProcessStats {
    vertices: usize,
    faces: usize,
    normals: usize,
}

fn process_obj(input_path: &str, output_path: &str) -> Result<ProcessStats, Box<dyn std::error::Error>> {
    let input_file = File::open(input_path)?;
    let reader = BufReader::new(input_file);
    
    let mut vertices = Vec::new();
    let mut tex_coords = Vec::new();
    let mut faces = Vec::new();
    let mut other_lines = Vec::new();
    
    println!("📖 Reading input file: {}", input_path);
    
    // Parse the input OBJ file
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        
        if line.is_empty() || line.starts_with('#') {
            other_lines.push(line.to_string());
            continue;
        }
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            other_lines.push(line.to_string());
            continue;
        }
        
        match parts[0] {
            "v" => {
                // Vertex position
                if parts.len() >= 4 {
                    let x: f32 = parts[1].parse()?;
                    let y: f32 = parts[2].parse()?;
                    let z: f32 = parts[3].parse()?;
                    vertices.push(Vec3::new(x, y, z));
                }
            }
            "vt" => {
                // Texture coordinate - preserve as-is
                tex_coords.push(line.to_string());
            }
            "vn" => {
                // Vertex normal - skip, we'll generate our own
                println!("⚠️  Skipping existing normal: {}", line);
            }
            "f" => {
                // Face - parse vertex indices
                if parts.len() >= 4 {
                    let mut face_vertices = Vec::new();
                    
                    for i in 1..parts.len() {
                        let vertex_data = parts[i];
                        let indices: Vec<&str> = vertex_data.split('/').collect();
                        
                        if !indices.is_empty() {
                            // Parse position index (1-based in OBJ)
                            let pos_idx: usize = indices[0].parse()?;
                            face_vertices.push(pos_idx - 1); // Convert to 0-based
                        }
                    }
                    
                    if face_vertices.len() >= 3 {
                        faces.push(Face { vertices: face_vertices });
                    }
                }
            }
            _ => {
                // Other lines (materials, groups, etc.) - preserve as-is
                other_lines.push(line.to_string());
            }
        }
    }
    
    println!("📊 Parsed {} vertices, {} faces", vertices.len(), faces.len());
    
    // Generate vertex normals
    println!("🔧 Generating vertex normals...");
    let vertex_normals = generate_vertex_normals(&vertices, &faces);
    
    // Write the output OBJ file
    println!("✍️  Writing output file: {}", output_path);
    let output_file = File::create(output_path)?;
    let mut writer = BufWriter::new(output_file);
    
    // Write header comment
    writeln!(writer, "# OBJ file processed by obj_normalizer")?;
    writeln!(writer, "# Original: {}", input_path)?;
    writeln!(writer, "# Generated vertex normals for modern lighting")?;
    writeln!(writer, "")?;
    
    // Write preserved lines (comments, materials, etc.) first
    for line in &other_lines {
        if !line.starts_with("f ") && !line.starts_with("vt ") {
            writeln!(writer, "{}", line)?;
        }
    }
    
    writeln!(writer, "")?;
    
    // Write vertices
    for vertex in &vertices {
        writeln!(writer, "v {} {} {}", vertex.x, vertex.y, vertex.z)?;
    }
    
    writeln!(writer, "")?;
    
    // Write texture coordinates
    for tex_coord in &tex_coords {
        writeln!(writer, "{}", tex_coord)?;
    }
    
    writeln!(writer, "")?;
    
    // Write generated normals
    for normal in &vertex_normals {
        writeln!(writer, "vn {} {} {}", normal.x, normal.y, normal.z)?;
    }
    
    writeln!(writer, "")?;
    
    // Write faces with normal indices
    for face in &faces {
        write!(writer, "f")?;
        for &vertex_idx in &face.vertices {
            // OBJ format: vertex/texture/normal (1-based)
            // For simplicity, we'll use the same index for vertex and normal
            write!(writer, " {}//{}", vertex_idx + 1, vertex_idx + 1)?;
        }
        writeln!(writer)?;
    }
    
    writer.flush()?;
    
    Ok(ProcessStats {
        vertices: vertices.len(),
        faces: faces.len(),
        normals: vertex_normals.len(),
    })
}

fn generate_vertex_normals(vertices: &[Vec3], faces: &[Face]) -> Vec<Vec3> {
    let mut vertex_normals = vec![Vec3::new(0.0, 0.0, 0.0); vertices.len()];
    
    // For each face, calculate face normal and add to vertex normals
    for face in faces {
        if face.vertices.len() < 3 {
            continue;
        }
        
        // Use first three vertices to calculate face normal
        let v0 = vertices[face.vertices[0]];
        let v1 = vertices[face.vertices[1]];
        let v2 = vertices[face.vertices[2]];
        
        // Calculate two edge vectors
        let edge1 = v1.sub(&v0);
        let edge2 = v2.sub(&v0);
        
        // Calculate face normal via cross product
        let face_normal = edge1.cross(&edge2);
        
        // Add face normal to all vertices in this face
        for &vertex_idx in &face.vertices {
            if vertex_idx < vertex_normals.len() {
                vertex_normals[vertex_idx] = vertex_normals[vertex_idx].add(&face_normal);
            }
        }
    }
    
    // Normalize all vertex normals
    for normal in &mut vertex_normals {
        *normal = normal.normalize();
    }
    
    vertex_normals
}
