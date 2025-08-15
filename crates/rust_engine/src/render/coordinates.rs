//! Coordinate system conversion utilities
//!
//! This module provides utilities for converting between different coordinate systems
//! commonly used in 3D graphics, allowing applications to handle coordinate conversions
//! explicitly rather than having them hardcoded in asset loading.

use crate::render::mesh::Vertex;

/// Coordinate system conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateSystem {
    /// Standard Y-up, right-handed coordinate system (common in modeling tools, OBJ files)
    YUpRightHanded,
    /// Y-down, left-handed coordinate system (Vulkan, DirectX)
    YDownLeftHanded,
    /// Z-up, right-handed coordinate system (some CAD tools)
    ZUpRightHanded,
}

/// Coordinate system converter
pub struct CoordinateConverter {
    from: CoordinateSystem,
    to: CoordinateSystem,
}

impl CoordinateConverter {
    /// Create a new coordinate converter
    pub fn new(from: CoordinateSystem, to: CoordinateSystem) -> Self {
        Self { from, to }
    }
    
    /// Convert a mesh from one coordinate system to another
    pub fn convert_mesh(&self, vertices: &mut [Vertex]) {
        if self.from == self.to {
            return; // No conversion needed
        }
        
        match (self.from, self.to) {
            (CoordinateSystem::YUpRightHanded, CoordinateSystem::YDownLeftHanded) => {
                self.convert_y_up_to_vulkan(vertices);
            }
            _ => {
                log::warn!("Coordinate conversion from {:?} to {:?} not yet implemented", self.from, self.to);
            }
        }
    }
    
    /// Convert from Y-up right-handed (OBJ) to Y-down left-handed (Vulkan)
    fn convert_y_up_to_vulkan(&self, vertices: &mut [Vertex]) {
        for vertex in vertices {
            // Convert position: flip Y coordinate
            let pos = vertex.position;
            vertex.position = [pos[0], -pos[1], pos[2]];
            
            // Convert normal: match position coordinate conversion
            let normal = vertex.normal;
            vertex.normal = [normal[0], -normal[1], normal[2]];
            
            // Texture coordinates typically stay the same (UV mapping)
            // vertex.tex_coord remains unchanged
        }
        
        // Note: When we flip Y coordinates, we effectively mirror the model,
        // which automatically reverses the triangle winding order.
        // This converts CCW→CW winding correctly for Vulkan.
    }
}

impl Default for CoordinateConverter {
    /// Default converter: Y-up right-handed to Y-down left-handed (OBJ to Vulkan)
    fn default() -> Self {
        Self::new(CoordinateSystem::YUpRightHanded, CoordinateSystem::YDownLeftHanded)
    }
}
