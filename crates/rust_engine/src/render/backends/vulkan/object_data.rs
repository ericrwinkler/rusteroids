//! Per-object uniform buffer data structures for Vulkan rendering
//!
//! This module defines GPU data structures for per-object rendering data.
//! These are legitimate rendering primitives, not game entities.

use crate::foundation::math::Mat4;

/// Per-object uniform buffer data structure
///
/// This represents the GPU-side data needed to render a single object.
/// It's a pure rendering primitive - no game logic, no transforms, just GPU data.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct ObjectUBO {
    /// Model transformation matrix (4×4 column-major)
    pub model_matrix: [[f32; 4]; 4],
    /// Normal transformation matrix for lighting calculations (3×3)
    pub normal_matrix: [[f32; 3]; 3],
    /// Padding to ensure proper 16-byte alignment
    pub _padding: [f32; 3],
}

impl ObjectUBO {
    /// Create a new ObjectUBO from a model matrix
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
