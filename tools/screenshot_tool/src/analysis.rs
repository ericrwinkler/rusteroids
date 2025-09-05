use anyhow::Result;
use image::{ImageBuffer, RgbaImage};
use std::path::Path;

/// Analyzes a screenshot to provide basic validation information
pub fn analyze_screenshot(image_path: &Path) -> Result<ScreenshotAnalysis> {
    let img = image::open(image_path)?;
    let rgba_img = img.to_rgba8();
    
    let (width, height) = rgba_img.dimensions();
    let total_pixels = (width * height) as usize;
    
    let mut black_pixels = 0;
    let mut white_pixels = 0;
    let mut colored_pixels = 0;
    let mut total_brightness = 0u64;
    
    for pixel in rgba_img.pixels() {
        let [r, g, b, _a] = pixel.0;
        let brightness = (r as u32 + g as u32 + b as u32) / 3;
        total_brightness += brightness as u64;
        
        if r < 10 && g < 10 && b < 10 {
            black_pixels += 1;
        } else if r > 245 && g > 245 && b > 245 {
            white_pixels += 1;
        } else if r != g || g != b {
            colored_pixels += 1;
        }
    }
    
    let avg_brightness = (total_brightness / total_pixels as u64) as u8;
    let black_ratio = black_pixels as f32 / total_pixels as f32;
    let white_ratio = white_pixels as f32 / total_pixels as f32;
    let colored_ratio = colored_pixels as f32 / total_pixels as f32;
    
    Ok(ScreenshotAnalysis {
        width,
        height,
        total_pixels,
        black_ratio,
        white_ratio,
        colored_ratio,
        avg_brightness,
        likely_content: classify_content(black_ratio, white_ratio, colored_ratio, avg_brightness),
    })
}

#[derive(Debug)]
pub struct ScreenshotAnalysis {
    pub width: u32,
    pub height: u32,
    pub total_pixels: usize,
    pub black_ratio: f32,
    pub white_ratio: f32,
    pub colored_ratio: f32,
    pub avg_brightness: u8,
    pub likely_content: ContentClassification,
}

#[derive(Debug)]
pub enum ContentClassification {
    RenderedScene,      // Normal 3D rendering
    BlankOrEmpty,       // Mostly black or white
    LoadingOrError,     // Unusual patterns suggesting issues
    UnknownPattern,     // Doesn't match expected patterns
}

fn classify_content(black_ratio: f32, white_ratio: f32, colored_ratio: f32, avg_brightness: u8) -> ContentClassification {
    if black_ratio > 0.9 || white_ratio > 0.9 {
        ContentClassification::BlankOrEmpty
    } else if colored_ratio > 0.3 && avg_brightness > 30 && avg_brightness < 200 {
        ContentClassification::RenderedScene
    } else if avg_brightness < 20 {
        ContentClassification::LoadingOrError
    } else {
        ContentClassification::UnknownPattern
    }
}

impl std::fmt::Display for ScreenshotAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Screenshot Analysis:")?;
        writeln!(f, "  Dimensions: {}x{}", self.width, self.height)?;
        writeln!(f, "  Total pixels: {}", self.total_pixels)?;
        writeln!(f, "  Black pixels: {:.1}%", self.black_ratio * 100.0)?;
        writeln!(f, "  White pixels: {:.1}%", self.white_ratio * 100.0)?;
        writeln!(f, "  Colored pixels: {:.1}%", self.colored_ratio * 100.0)?;
        writeln!(f, "  Average brightness: {}/255", self.avg_brightness)?;
        writeln!(f, "  Content classification: {:?}", self.likely_content)?;
        
        match self.likely_content {
            ContentClassification::RenderedScene => {
                writeln!(f, "  ✅ Screenshot appears to show rendered 3D content")
            }
            ContentClassification::BlankOrEmpty => {
                writeln!(f, "  ⚠️  Screenshot appears blank or empty")
            }
            ContentClassification::LoadingOrError => {
                writeln!(f, "  ❌ Screenshot may show loading screen or error state")
            }
            ContentClassification::UnknownPattern => {
                writeln!(f, "  ❓ Screenshot shows unexpected content pattern")
            }
        }
    }
}
