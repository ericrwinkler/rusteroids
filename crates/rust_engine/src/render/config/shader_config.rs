//! Shader configuration for the rendering system
//!
//! This module provides configuration structures for shader loading,
//! allowing applications to specify shader paths and parameters.

use std::path::Path;

/// Configuration for shader loading
#[derive(Debug, Clone)]
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
        let vertex_path = Self::resolve_shader_path(base_vertex);
        let fragment_path = Self::resolve_shader_path(base_fragment);
        
        Self {
            vertex_shader_path: vertex_path,
            fragment_shader_path: fragment_path,
        }
    }
    
    /// Resolve shader path by checking multiple common locations
    fn resolve_shader_path(base_path: &str) -> String {
        // Try current directory first
        if Path::new(base_path).exists() {
            return base_path.to_string();
        }
        
        // Try target/shaders directory (from project root)
        let target_path = format!("target/shaders/{}", 
            Path::new(base_path).file_name().unwrap().to_str().unwrap());
        if Path::new(&target_path).exists() {
            return target_path;
        }
        
        // Try ../target/shaders (from subdirectory)
        let parent_target_path = format!("../target/shaders/{}", 
            Path::new(base_path).file_name().unwrap().to_str().unwrap());
        if Path::new(&parent_target_path).exists() {
            return parent_target_path;
        }
        
        // Fall back to original path
        base_path.to_string()
    }
}

impl Default for ShaderConfig {
    /// Default shader configuration for basic 3D rendering
    /// 
    /// Uses standard vertex and fragment shaders with automatic path resolution.
    fn default() -> Self {
        Self::with_path_resolution("vert.spv", "frag.spv")
    }
}
