//! Coordinate system conversion utilities
//!
//! This module provides utilities for converting between different coordinate systems
//! when needed for specific asset formats or interoperability requirements.
//!
//! # Johannes Unterguggenberger Guide Compliance
//! 
//! For proper Vulkan rendering following academic standards, the recommended approach is:
//! 1. Keep assets in their native Y-up right-handed format during loading
//! 2. Use the X matrix (vulkan_coordinate_transform) during rendering for conversion
//! 3. Follow the complete chain: C = P × X × V
//!
//! This coordinate converter should only be used for special cases where manual
//! conversion is explicitly required, not as part of the standard rendering pipeline.

use crate::render::mesh::Vertex;

/// Coordinate system conventions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinateSystem {
    /// Standard Y-up, right-handed coordinate system (common in modeling tools, OBJ files)
    /// This is the recommended format for assets in the Johannes Unterguggenberger guide approach
    YUpRightHanded,
    /// Y-down coordinate system (used by some legacy conversion workflows)
    /// NOTE: Modern Vulkan rendering should use Y-up assets + X matrix conversion instead
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
    
    /// Convert from Y-up right-handed (OBJ) to Y-down coordinates (legacy conversion)
    /// NOTE: This should NOT be used in the standard rendering pipeline when following
    /// Johannes Unterguggenberger's guide. Use X matrix conversion during rendering instead.
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
    /// Default converter: Y-up right-handed to Y-down (legacy conversion)
    /// NOTE: For Johannes Unterguggenberger guide compliance, avoid using this during
    /// standard asset loading. Keep assets in Y-up format and use X matrix in rendering.
    fn default() -> Self {
        Self::new(CoordinateSystem::YUpRightHanded, CoordinateSystem::YDownLeftHanded)
    }
}
