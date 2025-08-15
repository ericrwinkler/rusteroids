//! # Unified Configuration System
//!
//! This module consolidates all configuration structures and utilities into a single,
//! coherent system. It provides configuration for rendering, assets, engine behavior,
//! and application-specific settings.
//!
//! ## Design Goals
//!
//! - **Centralized**: All configuration types in one place for easy discovery
//! - **Modular**: Clear separation between different subsystem configurations
//! - **Serializable**: Support for multiple config file formats (TOML, RON)
//! - **Type Safe**: Strong typing with validation and defaults
//!
//! ## Configuration Categories
//!
//! - **Render Config**: Graphics backend, shaders, performance settings
//! - **Engine Config**: Core engine behavior, logging, debug features
//! - **Asset Config**: Asset loading, caching, resource management
//! - **Application Config**: User-defined application-specific settings

use serde::{Serialize, Deserialize};
use std::path::Path;

// Re-export from the old config module for compatibility
pub use crate::config::{Config, ConfigError};

/// # Shader Configuration
///
/// Defines shader loading parameters and paths for the rendering system.
/// Supports path resolution and validation for development environments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderConfig {
    /// Path to the vertex shader SPIR-V file
    pub vertex_shader_path: String,
    /// Path to the fragment shader SPIR-V file  
    pub fragment_shader_path: String,
}

impl ShaderConfig {
    /// Create a new shader configuration
    pub fn new(vertex_path: impl Into<String>, fragment_path: impl Into<String>) -> Self {
        Self {
            vertex_shader_path: vertex_path.into(),
            fragment_shader_path: fragment_path.into(),
        }
    }
    
    /// Create shader config with automatic path resolution
    /// 
    /// This tries multiple common locations for shaders, useful for applications
    /// that might be run from different working directories.
    pub fn with_path_resolution(base_vertex: &str, base_fragment: &str) -> Self {
        // Try common shader locations
        let shader_dirs = [
            "target/shaders/",
            "shaders/", 
            "resources/shaders/",
            "../shaders/",
            "./",
        ];
        
        let mut vertex_path = None;
        let mut fragment_path = None;
        
        for dir in &shader_dirs {
            let vertex_test = format!("{}{}", dir, base_vertex);
            let fragment_test = format!("{}{}", dir, base_fragment);
            
            if Path::new(&vertex_test).exists() && vertex_path.is_none() {
                vertex_path = Some(vertex_test);
            }
            if Path::new(&fragment_test).exists() && fragment_path.is_none() {
                fragment_path = Some(fragment_test);
            }
            
            if vertex_path.is_some() && fragment_path.is_some() {
                break;
            }
        }
        
        Self {
            vertex_shader_path: vertex_path.unwrap_or_else(|| format!("shaders/{}", base_vertex)),
            fragment_shader_path: fragment_path.unwrap_or_else(|| format!("shaders/{}", base_fragment)),
        }
    }
    
    /// Validate that shader files exist
    pub fn validate(&self) -> Result<(), String> {
        if !Path::new(&self.vertex_shader_path).exists() {
            return Err(format!("Vertex shader not found: {}", self.vertex_shader_path));
        }
        if !Path::new(&self.fragment_shader_path).exists() {
            return Err(format!("Fragment shader not found: {}", self.fragment_shader_path));
        }
        Ok(())
    }
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self::with_path_resolution("vert.spv", "frag.spv")
    }
}

/// # Vulkan Renderer Configuration
///
/// Configuration specific to the Vulkan rendering backend, including
/// application metadata, performance tuning, and debug features.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

impl VulkanRendererConfig {
    /// Create a new renderer configuration
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            application_name: app_name.into(),
            application_version: (1, 0, 0),
            shaders: ShaderConfig::default(),
            max_frames_in_flight: 2,
            enable_validation: None, // Auto-detect based on build type
        }
    }
    
    /// Set application version
    pub fn with_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.application_version = (major, minor, patch);
        self
    }
    
    /// Set custom shader configuration
    pub fn with_shaders(mut self, shaders: ShaderConfig) -> Self {
        self.shaders = shaders;
        self
    }
    
    /// Set maximum frames in flight
    pub fn with_max_frames_in_flight(mut self, frames: usize) -> Self {
        self.max_frames_in_flight = frames;
        self
    }
    
    /// Enable or disable validation layers
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.enable_validation = Some(enabled);
        self
    }
    
    /// Auto-detect appropriate validation settings
    /// 
    /// Enables validation in debug builds and disables in release builds
    pub fn with_auto_validation(mut self) -> Self {
        self.enable_validation = Some(cfg!(debug_assertions));
        self
    }
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.application_name.is_empty() {
            return Err("Application name cannot be empty".to_string());
        }
        
        if self.max_frames_in_flight == 0 {
            return Err("Max frames in flight must be at least 1".to_string());
        }
        
        if self.max_frames_in_flight > 8 {
            return Err("Max frames in flight should not exceed 8 for performance reasons".to_string());
        }
        
        self.shaders.validate()?;
        
        Ok(())
    }
}

impl Default for VulkanRendererConfig {
    fn default() -> Self {
        Self::new("Rust Engine Application").with_auto_validation()
    }
}

/// # Engine Configuration
///
/// Core engine behavior configuration including logging, debug features,
/// and performance settings that affect the entire engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Log level for the engine
    pub log_level: String,
    /// Whether to enable debug features
    pub debug_mode: bool,
    /// Target FPS for frame rate limiting
    pub target_fps: Option<u32>,
    /// Whether to enable performance profiling
    pub enable_profiling: bool,
}

impl EngineConfig {
    /// Create a new engine configuration
    pub fn new() -> Self {
        Self {
            log_level: "info".to_string(),
            debug_mode: cfg!(debug_assertions),
            target_fps: None, // Unlimited by default
            enable_profiling: false,
        }
    }
    
    /// Set log level
    pub fn with_log_level(mut self, level: impl Into<String>) -> Self {
        self.log_level = level.into();
        self
    }
    
    /// Enable debug mode
    pub fn with_debug(mut self, enabled: bool) -> Self {
        self.debug_mode = enabled;
        self
    }
    
    /// Set target FPS
    pub fn with_target_fps(mut self, fps: u32) -> Self {
        self.target_fps = Some(fps);
        self
    }
    
    /// Enable profiling
    pub fn with_profiling(mut self, enabled: bool) -> Self {
        self.enable_profiling = enabled;
        self
    }
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// # Asset Configuration  
///
/// Configuration for asset loading, caching, and resource management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetConfig {
    /// Base directory for assets
    pub assets_dir: String,
    /// Whether to enable asset caching
    pub enable_caching: bool,
    /// Maximum cache size in MB
    pub max_cache_size_mb: u64,
    /// Supported asset formats
    pub supported_formats: Vec<String>,
}

impl AssetConfig {
    /// Create a new asset configuration
    pub fn new() -> Self {
        Self {
            assets_dir: "resources".to_string(),
            enable_caching: true,
            max_cache_size_mb: 256,
            supported_formats: vec![
                "obj".to_string(),
                "png".to_string(), 
                "jpg".to_string(),
                "wav".to_string(),
            ],
        }
    }
    
    /// Set assets directory
    pub fn with_assets_dir(mut self, dir: impl Into<String>) -> Self {
        self.assets_dir = dir.into();
        self
    }
    
    /// Configure caching
    pub fn with_caching(mut self, enabled: bool, max_size_mb: u64) -> Self {
        self.enable_caching = enabled;
        self.max_cache_size_mb = max_size_mb;
        self
    }
}

impl Default for AssetConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// # Complete Application Configuration
///
/// Top-level configuration that encompasses all engine subsystems.
/// This is the main configuration structure applications should use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Engine core configuration
    pub engine: EngineConfig,
    /// Rendering system configuration  
    pub renderer: VulkanRendererConfig,
    /// Asset system configuration
    pub assets: AssetConfig,
}

impl ApplicationConfig {
    /// Create a new application configuration with defaults
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            engine: EngineConfig::default(),
            renderer: VulkanRendererConfig::new(app_name),
            assets: AssetConfig::default(),
        }
    }
    
    /// Validate the entire configuration
    pub fn validate(&self) -> Result<(), String> {
        self.renderer.validate()?;
        // Add other validations as needed
        Ok(())
    }
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self::new("Rust Engine Application")
    }
}

impl Config for ApplicationConfig {}
