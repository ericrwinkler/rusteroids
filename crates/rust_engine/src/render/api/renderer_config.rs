//! Renderer configuration for application-specific settings
//!
//! This module provides configuration structures that applications can use
//! to customize the renderer behavior without hardcoding values in the 
//! rendering system itself.

use crate::render::ShaderConfig;

/// Configuration for the Vulkan renderer
#[derive(Debug, Clone)]
pub struct VulkanRendererConfig {
    /// Application name for Vulkan instance creation
    pub application_name: String,
    /// Application version (major, minor, patch)
    pub application_version: (u32, u32, u32),
    /// Shader configuration
    pub shaders: ShaderConfig,
    /// Maximum frames in flight (for performance tuning)
    pub max_frames_in_flight: usize,
    /// Whether to enable Vulkan validation layers
    pub enable_validation: Option<bool>,
    /// Background clear color [R, G, B, A] (0.0-1.0 range)
    pub clear_color: [f32; 4],
}

impl VulkanRendererConfig {
    /// Create a new renderer configuration
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            application_name: app_name.into(),
            application_version: (1, 0, 0),
            shaders: ShaderConfig::default(),
            max_frames_in_flight: 2,
            enable_validation: None, // Auto-detect based on debug build
            clear_color: [0.005, 0.005, 0.005, 1.0], // Dark gray background
        }
    }
    
    /// Set application version
    pub fn with_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.application_version = (major, minor, patch);
        self
    }
    
    /// Set shader configuration
    pub fn with_shaders(mut self, shaders: ShaderConfig) -> Self {
        self.shaders = shaders;
        self
    }
    
    /// Set custom shader paths
    pub fn with_shader_paths(mut self, vertex_path: impl Into<String>, fragment_path: impl Into<String>) -> Self {
        self.shaders = ShaderConfig::new(vertex_path, fragment_path);
        self
    }
    
    /// Set maximum frames in flight
    pub fn with_max_frames_in_flight(mut self, max_frames: usize) -> Self {
        self.max_frames_in_flight = max_frames.max(1).min(8); // Clamp to reasonable range
        self
    }
    
    /// Set background clear color [R, G, B, A] (0.0-1.0 range)
    pub fn with_clear_color(mut self, color: [f32; 4]) -> Self {
        self.clear_color = color;
        self
    }
    
    /// Enable or disable Vulkan validation layers
    pub fn with_validation(mut self, enable: bool) -> Self {
        self.enable_validation = Some(enable);
        self
    }
}

impl Default for VulkanRendererConfig {
    /// Default renderer configuration for a generic 3D application
    fn default() -> Self {
        Self::new("Rust 3D Application")
    }
}
