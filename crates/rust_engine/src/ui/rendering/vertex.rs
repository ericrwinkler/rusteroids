//! Vertex types for UI rendering

/// Vertex data for UI rendering with texture coordinates
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UIVertex {
    /// Position in NDC coordinates
    pub position: [f32; 2],
    /// Texture coordinates (for text/textured elements)
    pub uv: [f32; 2],
}

/// Simple position-only vertex for solid color panels (no UVs)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PanelVertex {
    /// Position in NDC coordinates
    pub position: [f32; 2],
}
