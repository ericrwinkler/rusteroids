//! Billboard renderer helper
//! 
//! Converts billboard quads into renderable geometry and coordinates
//! billboard rendering through the mesh pool system.

use super::{BillboardQuad, BillboardOrientation};
use super::orientation::*;
use crate::foundation::math::{Vec3, Mat4};
use crate::render::Camera;
use crate::render::systems::dynamic::{MeshPoolManager, MeshType};
use crate::render::resources::materials::{Material, UnlitMaterialParams};
use crate::render::api::RenderBackend;

/// Billboard renderer helper
pub struct BillboardRenderer;

impl BillboardRenderer {
    /// Convert a billboard quad to a transformation matrix
    pub fn calculate_billboard_matrix(
        billboard: &BillboardQuad,
        camera: &Camera,
    ) -> Mat4 {
        let camera_position = camera.position;
        let camera_up = camera.up;
        let billboard_position = billboard.position;
        let size = Vec3::new(billboard.size.x, billboard.size.y, 1.0);
        
        // Compute camera basis vectors directly from position and target
        let forward = (camera.target - camera_position).normalize();
        let right = forward.cross(&camera_up).normalize();
        let up = right.cross(&forward).normalize();
        
        match billboard.orientation {
            BillboardOrientation::ScreenAligned => {
                calculate_screen_aligned_matrix(
                    right,
                    up,
                    forward,
                    billboard_position,
                    size,
                )
            }
            BillboardOrientation::VelocityAligned { direction } => {
                calculate_velocity_aligned_matrix(
                    camera_position,
                    billboard_position,
                    direction,
                    size,
                )
            }
            BillboardOrientation::WorldAxisAligned { axis } => {
                calculate_world_axis_aligned_matrix(
                    camera_position,
                    billboard_position,
                    axis,
                    size,
                )
            }
        }
    }
}

/// Render billboard quads from a specific billboard mesh pool type
///
/// This function allows rendering billboards using different pools/textures by
/// specifying the MeshType. This is the industry standard approach for handling
/// multiple particle effect types with different visual properties.
///
/// # Arguments
/// * `pool_manager` - Mutable reference to pool manager
/// * `_backend` - Backend renderer (unused, retained for API consistency)
/// * `billboard_quads` - Slice of billboard quads to render
/// * `camera` - Camera for billboard orientation calculations
/// * `mesh_type` - Specific billboard mesh type to use (e.g., BillboardBullet, BillboardExplosion)
///
/// # Returns
/// Result indicating successful billboard rendering
pub fn render_billboards_from_pool_with_type(
    pool_manager: &mut Option<MeshPoolManager>,
    _backend: &mut dyn RenderBackend,
    billboard_quads: &[BillboardQuad],
    camera: &Camera,
    mesh_type: MeshType,
) -> Result<(), Box<dyn std::error::Error>> {
    if billboard_quads.is_empty() {
        return Ok(());
    }
    
    let mgr = pool_manager.as_mut()
        .ok_or("Pool manager not initialized")?;
    
    // Get the texture handle from the specified billboard pool
    let texture_handle = mgr.get_pool_texture(mesh_type)?;
    
    // Allocate and render each billboard as a dynamic object
    let mut allocated_count = 0;
    for quad in billboard_quads {
        // Calculate billboard transformation matrix
        let transform = BillboardRenderer::calculate_billboard_matrix(quad, camera);
        
        // Billboards use the TransparentUnlit pipeline for proper alpha blending
        let mut material = Material::transparent_unlit(UnlitMaterialParams {
            color: Vec3::new(quad.color.x, quad.color.y, quad.color.z),
            alpha: quad.color.w,
        });
        
        // Attach the pool's texture
        if let Some(tex) = texture_handle {
            material = material.with_base_color_texture(tex);
        }
        
        // Allocate from specified billboard pool
        match mgr.allocate_instance(mesh_type, transform, material) {
            Ok(_handle) => {
                allocated_count += 1;
            }
            Err(e) => {
                log::warn!("Failed to allocate billboard from {:?} pool ({}), stopping rendering", mesh_type, e);
                break;
            }
        }
    }
    
    log::trace!("Allocated {} billboards from {:?} pool for rendering", allocated_count, mesh_type);
    
    Ok(())
}

/// Render billboard quads from the billboard mesh pool (generic Billboard type)
pub fn render_billboards_from_pool(
    pool_manager: &mut Option<MeshPoolManager>,
    backend: &mut dyn RenderBackend,
    billboard_quads: &[BillboardQuad],
    camera: &Camera,
) -> Result<(), Box<dyn std::error::Error>> {
    render_billboards_from_pool_with_type(pool_manager, backend, billboard_quads, camera, MeshType::Billboard)
}
