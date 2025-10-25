//! Test texture loading functionality
//! 
//! This test verifies that we can load PNG files from disk

#[cfg(test)]
mod tests {
    use crate::assets::ImageData;
    use std::path::PathBuf;

    fn get_test_texture_path(filename: &str) -> PathBuf {
        // Get workspace root (2 levels up from crates/rust_engine)
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.pop(); // Remove rust_engine
        path.pop(); // Remove crates
        path.push("resources");
        path.push("textures");
        path.push(filename);
        path
    }

    #[test]
    fn test_load_texture_png() {
        let path = get_test_texture_path("texture.png");
        let result = ImageData::from_file(path.to_str().unwrap());
        
        assert!(result.is_ok(), "Failed to load texture.png from {:?}: {:?}", path, result.err());
        
        let image = result.unwrap();
        assert_eq!(image.channels, 4, "Expected RGBA format");
        assert!(image.width > 0, "Width should be greater than 0");
        assert!(image.height > 0, "Height should be greater than 0");
        assert_eq!(image.data.len(), (image.width * image.height * 4) as usize);
        
        println!("✓ Loaded texture.png: {}x{} RGBA", image.width, image.height);
    }

    #[test]
    fn test_load_normal_map() {
        let path = get_test_texture_path("normal_map.png");
        let result = ImageData::from_file(path.to_str().unwrap());
        
        assert!(result.is_ok(), "Failed to load normal_map.png from {:?}: {:?}", path, result.err());
        
        let image = result.unwrap();
        assert_eq!(image.channels, 4, "Expected RGBA format");
        println!("✓ Loaded normal_map.png: {}x{} RGBA", image.width, image.height);
    }

    #[test]
    fn test_load_emission_map() {
        let path = get_test_texture_path("emission_map.png");
        let result = ImageData::from_file(path.to_str().unwrap());
        
        assert!(result.is_ok(), "Failed to load emission_map.png from {:?}: {:?}", path, result.err());
        
        let image = result.unwrap();
        assert_eq!(image.channels, 4, "Expected RGBA format");
        println!("✓ Loaded emission_map.png: {}x{} RGBA", image.width, image.height);
    }

    #[test]
    fn test_solid_color() {
        let white = ImageData::solid_color(16, 16, [255, 255, 255, 255]);
        
        assert_eq!(white.width, 16);
        assert_eq!(white.height, 16);
        assert_eq!(white.channels, 4);
        assert_eq!(white.data.len(), 16 * 16 * 4);
        
        // Verify all pixels are white
        for chunk in white.data.chunks(4) {
            assert_eq!(chunk, &[255, 255, 255, 255]);
        }
        
        println!("✓ Created 16x16 white solid color texture");
    }

    #[test]
    fn test_nonexistent_file() {
        let result = ImageData::from_file("nonexistent_file.png");
        assert!(result.is_err(), "Should fail to load nonexistent file");
    }
}
