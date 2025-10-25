//! Image loading utilities for texture data
//!
//! Provides PNG, JPEG, and other image format loading for use with the texture system.

use std::path::Path;
use image;
use crate::assets::AssetError;

/// Loaded image data ready for GPU upload
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Raw RGBA pixel data
    pub data: Vec<u8>,
    /// Image width in pixels
    pub width: u32,
    /// Image height in pixels
    pub height: u32,
    /// Number of color channels (typically 4 for RGBA)
    pub channels: u8,
}

impl ImageData {
    /// Load an image from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, AssetError> {
        let path_ref = path.as_ref();
        
        log::debug!("Loading image from: {:?}", path_ref);
        
        // Load image using the image crate
        let img = image::open(path_ref)
            .map_err(|e| AssetError::LoadFailed(format!("Failed to load image: {}", e)))?;
        
        // Convert to RGBA8 format (standard for GPU upload)
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();
        
        log::info!("Loaded image {}x{} from {:?}", width, height, path_ref);
        
        Ok(Self {
            data: rgba_img.into_raw(),
            width,
            height,
            channels: 4, // RGBA
        })
    }
    
    /// Load image from memory (useful for embedded resources)
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AssetError> {
        let img = image::load_from_memory(bytes)
            .map_err(|e| AssetError::LoadFailed(format!("Failed to load image from bytes: {}", e)))?;
        
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();
        
        log::debug!("Loaded image {}x{} from memory", width, height);
        
        Ok(Self {
            data: rgba_img.into_raw(),
            width,
            height,
            channels: 4,
        })
    }
    
    /// Create a solid color image (useful for testing and defaults)
    pub fn solid_color(width: u32, height: u32, color: [u8; 4]) -> Self {
        let pixel_count = (width * height) as usize;
        let mut data = Vec::with_capacity(pixel_count * 4);
        
        for _ in 0..pixel_count {
            data.extend_from_slice(&color);
        }
        
        Self {
            data,
            width,
            height,
            channels: 4,
        }
    }
    
    /// Get the size of the image data in bytes
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
    
    /// Check if image dimensions are power of two (useful for mipmaps)
    pub fn is_power_of_two(&self) -> bool {
        self.width.is_power_of_two() && self.height.is_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_color_image() {
        let img = ImageData::solid_color(4, 4, [255, 0, 0, 255]);
        assert_eq!(img.width, 4);
        assert_eq!(img.height, 4);
        assert_eq!(img.channels, 4);
        assert_eq!(img.size_bytes(), 4 * 4 * 4); // 4x4 pixels, 4 bytes each
        
        // Check first pixel is red
        assert_eq!(&img.data[0..4], &[255, 0, 0, 255]);
    }
    
    #[test]
    fn test_power_of_two() {
        let img1 = ImageData::solid_color(256, 256, [0, 0, 0, 255]);
        assert!(img1.is_power_of_two());
        
        let img2 = ImageData::solid_color(100, 100, [0, 0, 0, 255]);
        assert!(!img2.is_power_of_two());
    }
}
