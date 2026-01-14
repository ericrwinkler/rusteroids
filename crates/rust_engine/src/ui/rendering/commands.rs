//! UI render commands

use super::vertex::{UIVertex, PanelVertex};
use crate::foundation::math::Vec4;

/// UI render command for a single element
#[derive(Debug, Clone)]
pub enum UIRenderCommand {
    /// Render a solid color panel (position-only vertices, 8 bytes per vertex)
    Panel {
        /// Panel vertex data
        vertices: Vec<PanelVertex>,
        /// Panel color
        color: Vec4,
    },
    /// Render text using glyph atlas (position + UV vertices, 16 bytes per vertex)
    Text {
        /// Text vertex data
        vertices: Vec<UIVertex>,
        /// Text color
        color: Vec4,
    },
}
