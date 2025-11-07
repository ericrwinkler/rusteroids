//! Text layout engine
//!
//! Converts text strings into positioned quads (vertices and indices)
//! for rendering. Handles glyph positioning, advance calculation, and
//! baseline alignment.

use super::{FontAtlas, GlyphInfo};
use crate::render::primitives::{Mesh, Vertex};
use nalgebra::{Vector2, Vector3};

/// Vertex data for text rendering
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct TextVertex {
    /// Position in local space
    pub position: Vector3<f32>,
    /// UV texture coordinates
    pub uv: Vector2<f32>,
}

/// Text layout engine that converts strings to mesh geometry
pub struct TextLayout {
    /// Font atlas for glyph lookup
    font_atlas: FontAtlas,
}

/// Bounding box for text layout
#[derive(Debug, Clone, Copy)]
pub struct TextBounds {
    /// Minimum X coordinate
    pub min_x: f32,
    /// Minimum Y coordinate
    pub min_y: f32,
    /// Maximum X coordinate
    pub max_x: f32,
    /// Maximum Y coordinate
    pub max_y: f32,
}

impl TextBounds {
    /// Calculate width of bounding box
    pub fn width(&self) -> f32 {
        self.max_x - self.min_x
    }
    
    /// Calculate height of bounding box
    pub fn height(&self) -> f32 {
        self.max_y - self.min_y
    }
}

impl TextLayout {
    /// Create a new text layout engine
    pub fn new(font_atlas: FontAtlas) -> Self {
        Self { font_atlas }
    }
    
    /// Convert a text string into positioned quads
    ///
    /// Returns (vertices, indices) for rendering as triangles.
    /// Each character generates 4 vertices and 6 indices (2 triangles).
    ///
    /// # Layout Coordinate System
    ///
    /// - Origin (0, 0) is at the baseline of the first character
    /// - +X axis points right
    /// - +Y axis points up (OpenGL convention)
    ///
    /// # Example
    ///
    /// ```no_run
    /// let (vertices, indices) = layout.layout_text("Hello");
    /// assert_eq!(vertices.len(), 20); // 5 chars × 4 vertices
    /// assert_eq!(indices.len(), 30);  // 5 chars × 6 indices
    /// ```
    pub fn layout_text(&self, text: &str) -> (Vec<TextVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        
        let mut cursor_x = 0.0f32;
        let baseline_y = 0.0f32;
        
        for ch in text.chars() {
            // Skip characters not in atlas (e.g., newlines, unprintable)
            let glyph = match self.font_atlas.get_glyph(ch) {
                Ok(g) => g,
                Err(_) => {
                    // Use space character advance for unknown chars
                    if let Ok(space_glyph) = self.font_atlas.get_glyph(' ') {
                        cursor_x += space_glyph.advance;
                    }
                    continue;
                }
            };
            
            // Create quad for this glyph
            let quad_verts = self.create_glyph_quad(cursor_x, baseline_y, glyph);
            let base_index = vertices.len() as u32;
            
            vertices.extend_from_slice(&quad_verts);
            indices.extend_from_slice(&[
                base_index, base_index + 2, base_index + 1,  // First triangle (CCW from -Z)
                base_index, base_index + 3, base_index + 2,  // Second triangle (CCW from -Z)
            ]);
            
            // Advance cursor for next character
            cursor_x += glyph.advance;
        }
        
        (vertices, indices)
    }
    
    /// Create a single quad for a glyph
    fn create_glyph_quad(
        &self,
        cursor_x: f32,
        baseline_y: f32,
        glyph: &GlyphInfo,
    ) -> [TextVertex; 4] {
        // Calculate quad corners based on glyph metrics
        let x_min = cursor_x + glyph.bearing.x;
        let y_min = baseline_y + glyph.bearing.y;
        let x_max = x_min + glyph.size.x;
        let y_max = y_min + glyph.size.y;
        
        // Create quad vertices (counter-clockwise winding when viewed from -Z)
        // Camera is at +Z looking toward -Z, so quad must face toward +Z (back toward camera)
        [
            // Bottom-left
            TextVertex {
                position: Vector3::new(x_min, y_min, 0.0),
                uv: Vector2::new(glyph.uv_min.x, glyph.uv_max.y),
            },
            // Top-left
            TextVertex {
                position: Vector3::new(x_min, y_max, 0.0),
                uv: Vector2::new(glyph.uv_min.x, glyph.uv_min.y),
            },
            // Top-right
            TextVertex {
                position: Vector3::new(x_max, y_max, 0.0),
                uv: Vector2::new(glyph.uv_max.x, glyph.uv_min.y),
            },
            // Bottom-right
            TextVertex {
                position: Vector3::new(x_max, y_min, 0.0),
                uv: Vector2::new(glyph.uv_max.x, glyph.uv_max.y),
            },
        ]
    }
    
    /// Calculate bounding box for a text string
    ///
    /// Useful for alignment, hit testing, and UI layout.
    pub fn calculate_bounds(&self, text: &str) -> TextBounds {
        let mut min_x = 0.0f32;
        let mut min_y = 0.0f32;
        let mut max_x = 0.0f32;
        let mut max_y = 0.0f32;
        
        let mut cursor_x = 0.0f32;
        let baseline_y = 0.0f32;
        
        for ch in text.chars() {
            let glyph = match self.font_atlas.get_glyph(ch) {
                Ok(g) => g,
                Err(_) => {
                    if let Ok(space_glyph) = self.font_atlas.get_glyph(' ') {
                        cursor_x += space_glyph.advance;
                    }
                    continue;
                }
            };
            
            let x_min = cursor_x + glyph.bearing.x;
            let y_min = baseline_y + glyph.bearing.y;
            let x_max = x_min + glyph.size.x;
            let y_max = y_min + glyph.size.y;
            
            min_x = min_x.min(x_min);
            min_y = min_y.min(y_min);
            max_x = max_x.max(x_max);
            max_y = max_y.max(y_max);
            
            cursor_x += glyph.advance;
        }
        
        TextBounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
    
    /// Get the font atlas used by this layout engine
    pub fn font_atlas(&self) -> &FontAtlas {
        &self.font_atlas
    }
}

/// Convert text vertices to a standard mesh for rendering
///
/// Converts TextVertex (position + UV) to the engine's Vertex format
/// (position + normal + tex_coord + tangent). Text quads use Z+ normal
/// since they're billboards that face the camera.
///
/// # Arguments
/// * `text_vertices` - The text vertex data from TextLayout
/// * `text_indices` - The index data from TextLayout
///
/// # Returns
/// A Mesh ready for rendering with the dynamic object system
pub fn text_vertices_to_mesh(text_vertices: Vec<TextVertex>, text_indices: Vec<u32>) -> Mesh {
    let vertices: Vec<Vertex> = text_vertices
        .iter()
        .map(|tv| {
            Vertex::new(
                [tv.position.x, tv.position.y, tv.position.z],
                [0.0, 0.0, 1.0], // Normal facing camera (Z+)
                [tv.uv.x, tv.uv.y],
            )
        })
        .collect();
    
    Mesh::new(vertices, text_indices)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_text_vertex_size() {
        // Verify TextVertex is compatible with bytemuck for GPU upload
        assert_eq!(
            std::mem::size_of::<TextVertex>(),
            std::mem::size_of::<f32>() * 5 // 3 position + 2 uv
        );
    }
    
    #[test]
    fn test_bounds_calculation() {
        let bounds = TextBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 100.0,
            max_y: 50.0,
        };
        
        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 50.0);
    }
}
