//! Frame Rendering Data Structures
//!
//! Following Game Engine Architecture Chapter 8.2 - The Game Loop
//! 
//! This module defines the top-level frame rendering data structure for
//! the unified render_frame() API.
//!
//! Proposal #6: Simplified Frame Rendering API

use crate::render::primitives::Camera;
use crate::render::systems::lighting::MultiLightEnvironment;
use crate::render::systems::ui::UIRenderData;
use crate::render::systems::billboard::BillboardQuad;

/// Complete frame rendering data
/// 
/// Application provides this once per frame; renderer handles complete
/// frame lifecycle internally.
#[derive(Debug)]
pub struct RenderFrameData<'a> {
    /// Camera parameters (view/projection matrices)
    pub camera: &'a Camera,
    
    /// Lighting environment (all lights in scene)
    pub lights: &'a MultiLightEnvironment,
    
    /// UI overlay data (quads, text)
    pub ui: &'a UIRenderData,
    
    /// Billboard quads for trails and effects
    pub billboards: &'a [BillboardQuad],
}
