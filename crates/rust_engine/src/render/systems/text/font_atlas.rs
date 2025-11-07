//! Font atlas system for text rendering
//!
//! This module provides font loading, glyph rasterization, and GPU texture atlas
//! management using the `fontdue` library for pure Rust font rendering.

use std::collections::HashMap;
use std::sync::Arc;
use fontdue::{Font, FontSettings};
use crate::render::resources::materials::{TextureHandle, TextureType};
use crate::render::GraphicsEngine;
use crate::assets::ImageData;
use nalgebra::Vector2;

/// Result type for font operations
pub type FontResult<T> = Result<T, FontError>;

/// Errors that can occur during font operations
#[derive(Debug, thiserror::Error)]
pub enum FontError {
    /// Failed to load font from file or data
    #[error("Failed to load font: {0}")]
    LoadError(String),
    
    /// Failed to rasterize a specific glyph character
    #[error("Failed to rasterize glyph '{0}': {1}")]
    RasterizeError(char, String),
    
    /// Failed to create or upload the atlas texture
    #[error("Failed to create atlas texture: {0}")]
    AtlasCreationError(String),
    
    /// Requested character was not found in the font atlas
    #[error("Character '{0}' not found in atlas")]
    GlyphNotFound(char),
}

/// Information about a single glyph in the atlas
#[derive(Debug, Clone)]
pub struct GlyphInfo {
    /// UV coordinates in atlas texture (normalized 0.0-1.0) - top-left corner
    pub uv_min: Vector2<f32>,
    /// UV coordinates in atlas texture (normalized 0.0-1.0) - bottom-right corner
    pub uv_max: Vector2<f32>,
    
    /// Glyph size in pixels
    pub size: Vector2<f32>,
    
    /// Horizontal advance for cursor positioning
    pub advance: f32,
    
    /// Bearing offset from baseline (x = left, y = top)
    pub bearing: Vector2<f32>,
}

/// Font atlas that manages a font texture and glyph metadata
///
/// The atlas rasterizes glyphs from a TrueType/OpenType font using `fontdue`
/// and packs them into a single GPU texture for efficient rendering.
pub struct FontAtlas {
    /// Underlying fontdue font
    font: Font,
    
    /// Font size in pixels
    font_size: f32,
    
    /// Handle to GPU texture atlas
    texture_handle: Option<TextureHandle>,
    
    /// Glyph information lookup
    glyph_cache: HashMap<char, GlyphInfo>,
    
    /// Atlas texture dimensions
    atlas_width: u32,
    atlas_height: u32,
    
    /// Shared reference for cloning
    _inner: Arc<FontAtlasInner>,
}

/// Inner data for Arc sharing
struct FontAtlasInner {
    // Currently unused, reserved for future shared state
}

impl FontAtlas {
    /// Create a new font atlas from TrueType/OpenType font data
    ///
    /// # Arguments
    ///
    /// * `font_data` - Raw font file bytes (TTF or OTF format)
    /// * `font_size` - Size in pixels to rasterize glyphs
    ///
    /// # Example
    ///
    /// ```no_run
    /// let font_bytes = std::fs::read("resources/fonts/default.ttf")?;
    /// let atlas = FontAtlas::new(&font_bytes, 48.0)?;
    /// ```
    pub fn new(font_data: &[u8], font_size: f32) -> FontResult<Self> {
        // Load font with fontdue
        let font = Font::from_bytes(font_data, FontSettings::default())
            .map_err(|e| FontError::LoadError(format!("fontdue error: {}", e)))?;
        
        log::info!("Loaded font at {}px size", font_size);
        
        // Start with empty atlas (will be built with upload_to_gpu)
        Ok(Self {
            font,
            font_size,
            texture_handle: None,
            glyph_cache: HashMap::new(),
            atlas_width: 1024,
            atlas_height: 1024,
            _inner: Arc::new(FontAtlasInner {}),
        })
    }
    
    /// Rasterize all ASCII glyphs and populate the glyph cache
    ///
    /// This prepares the font atlas for text layout without uploading to GPU.
    /// Useful for testing or when GPU upload will be done separately.
    ///
    /// # Example
    ///
    /// ```no_run
    /// let mut atlas = FontAtlas::new(&font_data, 48.0)?;
    /// atlas.rasterize_glyphs()?; // Populate cache
    /// let layout = TextLayout::new(atlas);
    /// let (vertices, indices) = layout.layout_text("Hello");
    /// ```
    pub fn rasterize_glyphs(&mut self) -> FontResult<()> {
        const ASCII_START: u32 = 32;  // Space character
        const ASCII_END: u32 = 126;    // Tilde character
        const GLYPH_COUNT: usize = (ASCII_END - ASCII_START + 1) as usize;
        
        log::info!("Rasterizing {} glyphs at {}px for cache", GLYPH_COUNT, self.font_size);
        
        // Rasterize all glyphs first to determine required atlas size
        let mut rasterized_glyphs = Vec::with_capacity(GLYPH_COUNT);
        let mut max_glyph_height = 0;
        
        for code_point in ASCII_START..=ASCII_END {
            let ch = char::from_u32(code_point).unwrap();
            let (metrics, _bitmap) = self.font.rasterize(ch, self.font_size);
            
            max_glyph_height = max_glyph_height.max(metrics.height);
            rasterized_glyphs.push((ch, metrics));
        }
        
        // Pack glyphs into atlas using simple grid layout
        let glyphs_per_row = 16; // 16x6 grid for 95 glyphs
        let cell_width = self.atlas_width / glyphs_per_row;
        let cell_height = (max_glyph_height as u32 * 3) / 2; // Add padding
        
        let mut current_x = 0u32;
        let mut current_y = 0u32;
        
        for (ch, metrics) in rasterized_glyphs {
            // Store glyph info (UV coordinates normalized to 0-1)
            let uv_min = Vector2::new(
                current_x as f32 / self.atlas_width as f32,
                current_y as f32 / self.atlas_height as f32,
            );
            let uv_max = Vector2::new(
                (current_x + metrics.width as u32) as f32 / self.atlas_width as f32,
                (current_y + metrics.height as u32) as f32 / self.atlas_height as f32,
            );
            
            let glyph_info = GlyphInfo {
                uv_min,
                uv_max,
                size: Vector2::new(metrics.width as f32, metrics.height as f32),
                advance: metrics.advance_width,
                bearing: Vector2::new(
                    metrics.xmin as f32,
                    metrics.ymin as f32,
                ),
            };
            
            self.glyph_cache.insert(ch, glyph_info);
            
            // Move to next cell
            current_x += cell_width;
            if current_x + cell_width > self.atlas_width {
                current_x = 0;
                current_y += cell_height;
            }
        }
        
        log::info!("Glyph cache populated: {} glyphs", self.glyph_cache.len());
        Ok(())
    }
    
    /// Upload font atlas to GPU
    ///
    /// Rasterizes all printable ASCII characters (32-126) and packs them into
    /// a texture atlas, then uploads to GPU via the graphics engine.
    ///
    /// # Arguments
    ///
    /// * `graphics_engine` - Graphics engine for texture upload
    ///
    /// # Returns
    ///
    /// Handle to the uploaded GPU texture
    pub fn upload_to_gpu(
        &mut self,
        graphics_engine: &mut GraphicsEngine,
    ) -> FontResult<TextureHandle> {
        const ASCII_START: u32 = 32;  // Space character
        const ASCII_END: u32 = 126;    // Tilde character
        const GLYPH_COUNT: usize = (ASCII_END - ASCII_START + 1) as usize;
        
        log::info!("Rasterizing {} glyphs at {}px", GLYPH_COUNT, self.font_size);
        
        // Rasterize all glyphs first to determine required atlas size
        let mut rasterized_glyphs = Vec::with_capacity(GLYPH_COUNT);
        let mut max_glyph_height = 0;
        
        for code_point in ASCII_START..=ASCII_END {
            let ch = char::from_u32(code_point).unwrap();
            let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);
            
            max_glyph_height = max_glyph_height.max(metrics.height);
            rasterized_glyphs.push((ch, metrics, bitmap));
        }
        
        // Pack glyphs into atlas using simple grid layout
        let glyphs_per_row = 16; // 16x6 grid for 95 glyphs
        let cell_width = self.atlas_width / glyphs_per_row;
        let cell_height = (max_glyph_height as u32 * 3) / 2; // Add padding
        
        // Create atlas texture (R8 format for alpha channel)
        let mut atlas_data = vec![0u8; (self.atlas_width * self.atlas_height) as usize];
        
        let mut current_x = 0u32;
        let mut current_y = 0u32;
        
        for (ch, metrics, bitmap) in rasterized_glyphs {
            // Copy glyph bitmap into atlas
            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let src_idx = y * metrics.width + x;
                    let dst_x = current_x + x as u32;
                    let dst_y = current_y + y as u32;
                    let dst_idx = (dst_y * self.atlas_width + dst_x) as usize;
                    
                    if dst_idx < atlas_data.len() && src_idx < bitmap.len() {
                        atlas_data[dst_idx] = bitmap[src_idx];
                    }
                }
            }
            
            // Store glyph info (UV coordinates normalized to 0-1)
            let uv_min = Vector2::new(
                current_x as f32 / self.atlas_width as f32,
                current_y as f32 / self.atlas_height as f32,
            );
            let uv_max = Vector2::new(
                (current_x + metrics.width as u32) as f32 / self.atlas_width as f32,
                (current_y + metrics.height as u32) as f32 / self.atlas_height as f32,
            );
            
            let glyph_info = GlyphInfo {
                uv_min,
                uv_max,
                size: Vector2::new(metrics.width as f32, metrics.height as f32),
                advance: metrics.advance_width,
                bearing: Vector2::new(
                    metrics.xmin as f32,
                    metrics.ymin as f32,
                ),
            };
            
            self.glyph_cache.insert(ch, glyph_info);
            
            // Move to next cell
            current_x += cell_width;
            if current_x + cell_width > self.atlas_width {
                current_x = 0;
                current_y += cell_height;
            }
        }
        
        log::info!("Atlas packed: {}x{}, {} glyphs cached", 
                   self.atlas_width, self.atlas_height, self.glyph_cache.len());
        
        // Convert single-channel atlas to RGBA format expected by GPU
        // Use grayscale data in alpha channel for text rendering
        let mut rgba_data = Vec::with_capacity((self.atlas_width * self.atlas_height * 4) as usize);
        for alpha in atlas_data {
            rgba_data.push(255); // R
            rgba_data.push(255); // G
            rgba_data.push(255); // B
            rgba_data.push(alpha); // A - glyph coverage
        }
        
        // Upload to GPU
        let image_data = ImageData {
            width: self.atlas_width,
            height: self.atlas_height,
            data: rgba_data,
            channels: 4, // RGBA format
        };
        
        let handle = graphics_engine.upload_texture_from_image_data(
            image_data,
            TextureType::BaseColor, // Use BaseColor type for font atlas
        ).map_err(|e| FontError::AtlasCreationError(e.to_string()))?;
        
        self.texture_handle = Some(handle);
        
        log::info!("Font atlas uploaded to GPU: TextureHandle({:?})", handle);
        
        Ok(handle)
    }
    
    /// Get glyph information for a character
    pub fn get_glyph(&self, ch: char) -> FontResult<&GlyphInfo> {
        self.glyph_cache
            .get(&ch)
            .ok_or(FontError::GlyphNotFound(ch))
    }
    
    /// Get the GPU texture handle for this atlas
    pub fn texture_handle(&self) -> Option<TextureHandle> {
        self.texture_handle
    }
    
    /// Get the font size in pixels
    pub fn font_size(&self) -> f32 {
        self.font_size
    }
    
    /// Get atlas dimensions
    pub fn atlas_dimensions(&self) -> (u32, u32) {
        (self.atlas_width, self.atlas_height)
    }
    
    /// Calculate line height for this font (distance between baselines)
    pub fn line_height(&self) -> f32 {
        // Use font metrics for accurate line height
        self.font_size * 1.2 // 120% of font size is a reasonable default
    }
    
    /// Save atlas texture to PNG file for debugging
    #[cfg(debug_assertions)]
    pub fn save_debug_image(&self, _path: &str) -> Result<(), String> {
        // TODO: Implement debug image export
        log::warn!("Debug image export not yet implemented");
        Ok(())
    }
}

impl Clone for FontAtlas {
    fn clone(&self) -> Self {
        Self {
            font: self.font.clone(),
            font_size: self.font_size,
            texture_handle: self.texture_handle,
            glyph_cache: self.glyph_cache.clone(),
            atlas_width: self.atlas_width,
            atlas_height: self.atlas_height,
            _inner: Arc::clone(&self._inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_font_atlas_creation() {
        // This test requires a real font file
        // In actual tests, we'd load from resources/fonts/
        // For now, just verify the API compiles
    }
    
    #[test]
    fn test_glyph_info() {
        let info = GlyphInfo {
            uv_min: Vector2::new(0.0, 0.0),
            uv_max: Vector2::new(0.1, 0.1),
            size: Vector2::new(16.0, 24.0),
            advance: 12.0,
            bearing: Vector2::new(1.0, 20.0),
        };
        
        assert_eq!(info.size.x, 16.0);
        assert_eq!(info.advance, 12.0);
    }
}
