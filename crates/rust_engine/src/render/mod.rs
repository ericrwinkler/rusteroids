//! Rendering system

pub mod mesh;
pub mod material;
pub mod lighting;
pub mod window;
pub mod vulkan;

pub use mesh::{Mesh, Vertex};
pub use material::Material;
pub use lighting::{Light, LightType, LightingEnvironment};
pub use window::WindowHandle;

use thiserror::Error;
use crate::engine::{RendererConfig, WindowConfig};
use crate::ecs::World;
use crate::foundation::math::{Vec3, Mat4, Mat4Ext, utils};

/// Main renderer with Vulkan backend
pub struct Renderer {
    vulkan_renderer: vulkan::VulkanRenderer,
    current_camera: Option<Camera>,
    current_lighting: Option<LightingEnvironment>,
    frame_count: u64,
}

impl Renderer {
    /// Create a new renderer from a window handle
    pub fn new_from_window(window_handle: &mut WindowHandle) -> Self {
        log::info!("Initializing Vulkan renderer");
        
        let vulkan_renderer = vulkan::VulkanRenderer::new(window_handle.vulkan_window_mut())
            .expect("Failed to create Vulkan renderer");
        
        Self {
            vulkan_renderer,
            current_camera: None,
            current_lighting: None,
            frame_count: 0,
        }
    }
    
    /// Create a new renderer (legacy method for engine integration)
    pub fn new(_config: &RendererConfig, window_config: &WindowConfig) -> Result<Self, RenderError> {
        log::info!("Initializing Vulkan renderer: {}", window_config.title);
        
        // Create window handle
        let mut window_handle = WindowHandle::new(
            window_config.width,
            window_config.height,
            &window_config.title,
        );

        Ok(Self::new_from_window(&mut window_handle))
    }
    
    /// Begin a new frame
    pub fn begin_frame(&mut self) {
        // Frame setup handled by Vulkan renderer
        log::trace!("Begin frame {}", self.frame_count);
    }
    
    /// Set the camera for rendering
    pub fn set_camera(&mut self, camera: &Camera) {
        self.current_camera = Some(camera.clone());
        log::trace!("Camera set: pos={:?}, target={:?}", camera.position, camera.target);
    }
    
    /// Set the lighting environment
    pub fn set_lighting(&mut self, lighting: &LightingEnvironment) {
        self.current_lighting = Some(lighting.clone());
        
        // Update the Vulkan renderer with lighting data
        // LIMITATION: Current push constants only support ONE directional light
        // Multiple lights require expanding push constants (limited to ~128 bytes) or uniform buffers
        if let Some(first_light) = lighting.lights.first() {
            match first_light.light_type {
                LightType::Directional => {
                    let direction = [first_light.direction.x, first_light.direction.y, first_light.direction.z];
                    let color = [first_light.color.x, first_light.color.y, first_light.color.z];
                    self.vulkan_renderer.set_directional_light(
                        direction,
                        first_light.intensity,
                        color,
                        lighting.ambient_intensity,
                    );
                },
                _ => {
                    // Point lights and other types not supported with current push constants
                    log::warn!("Only directional lights supported in push constants. Found: {:?}", first_light.light_type);
                    log::info!("To use multiple lights, need to implement uniform buffer lighting system");
                }
            }
        } else {
            // No lights in environment, use default lighting
            self.vulkan_renderer.set_directional_light(
                [0.0, 1.0, 0.0],  // Default light pointing downward (+Y in right-handed system)
                0.3,              // Mild default intensity
                [1.0, 1.0, 1.0],  // White light
                lighting.ambient_intensity,
            );
            log::trace!("No directional lights found, using default downward light");
        }
        
        // Log limitation for multiple lights
        if lighting.lights.len() > 1 {
            log::warn!("LightingEnvironment contains {} lights, but only the first directional light is used", lighting.lights.len());
            log::info!("Multiple light support requires expanding push constants or implementing uniform buffer system");
        }
        
        log::trace!("Lighting set with {} lights", lighting.lights.len());
    }
    
    /// Draw a 3D mesh with transform and material
    pub fn draw_mesh_3d(&mut self, mesh: &Mesh, model_matrix: &Mat4, material: &Material) -> Result<(), Box<dyn std::error::Error>> {
        // Update mesh data in Vulkan renderer
        self.vulkan_renderer.update_mesh(&mesh.vertices, &mesh.indices)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Set material color in Vulkan renderer
        let material_color = [material.base_color[0], material.base_color[1], material.base_color[2], material.alpha];
        self.vulkan_renderer.set_material_color(material_color);
        
        // Calculate MVP matrix if we have a camera
        if let Some(ref camera) = self.current_camera {
            let view_matrix = camera.get_view_matrix();
            let coord_transform = Mat4::vulkan_coordinate_transform();
            let projection_matrix = camera.get_projection_matrix();
            
            // Follow Johannes Unterguggenberger's guide: C = P * X * V * M
            // where P=projection, X=coordinate transform, V=view, M=model
            let mvp = projection_matrix * coord_transform * view_matrix * *model_matrix;
            
            // Convert Mat4 to array format for Vulkan renderer
            // nalgebra matrices are column-major, we need to transpose for row access
            let mvp_array = [
                [mvp[(0, 0)], mvp[(1, 0)], mvp[(2, 0)], mvp[(3, 0)]],
                [mvp[(0, 1)], mvp[(1, 1)], mvp[(2, 1)], mvp[(3, 1)]],
                [mvp[(0, 2)], mvp[(1, 2)], mvp[(2, 2)], mvp[(3, 2)]],
                [mvp[(0, 3)], mvp[(1, 3)], mvp[(2, 3)], mvp[(3, 3)]],
            ];
            
            // Convert model matrix to array format
            let model_array = [
                [model_matrix[(0, 0)], model_matrix[(1, 0)], model_matrix[(2, 0)], model_matrix[(3, 0)]],
                [model_matrix[(0, 1)], model_matrix[(1, 1)], model_matrix[(2, 1)], model_matrix[(3, 1)]],
                [model_matrix[(0, 2)], model_matrix[(1, 2)], model_matrix[(2, 2)], model_matrix[(3, 2)]],
                [model_matrix[(0, 3)], model_matrix[(1, 3)], model_matrix[(2, 3)], model_matrix[(3, 3)]],
            ];

            self.vulkan_renderer.set_mvp_matrix(mvp_array);
            self.vulkan_renderer.set_model_matrix(model_array);
            
            // Set lighting in world space
            if let Some(ref lighting) = self.current_lighting {
                if let Some(first_light) = lighting.lights.first() {
                    match first_light.light_type {
                        LightType::Directional => {
                            // Keep light direction in world space (don't transform it)
                            // The normal matrix will transform normals from object space to world space
                            let direction = [first_light.direction.x, first_light.direction.y, first_light.direction.z];
                            let color = [first_light.color.x, first_light.color.y, first_light.color.z];
                            self.vulkan_renderer.set_directional_light(
                                direction,
                                first_light.intensity,
                                color,
                                lighting.ambient_intensity,
                            );
                            log::trace!("Set world-space directional light: dir={:?}, intensity={}", 
                                       direction, first_light.intensity);
                        },
                        _ => {
                            log::warn!("Only directional lights are currently supported in push constants");
                        }
                    }
                }
            }
        }
        
        // Draw the frame
        match self.vulkan_renderer.draw_frame() {
            Ok(()) => {},
            Err(vulkan::VulkanError::Api(ash::vk::Result::ERROR_OUT_OF_DATE_KHR)) => {
                log::warn!("Swapchain out of date during render - requesting recreation");
                // The swapchain is out of date, we should recreate it
                // For now, we'll just log this and continue
                // The application should handle window resize events properly
                return Ok(()); // Skip this frame
            }
            Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error>),
        }
        
        Ok(())
    }
    
    /// End the current frame and present
    pub fn end_frame(&mut self, _window_handle: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        log::trace!("End frame {}", self.frame_count);
        self.frame_count += 1;
        
        // Frame is already drawn by draw_mesh_3d, just increment counter
        Ok(())
    }
    
    /// Recreate swapchain (for window resizing)
    pub fn recreate_swapchain(&mut self, window_handle: &mut WindowHandle) {
        log::info!("Recreating swapchain");
        
        // Get the Vulkan window from the handle and pass it to Vulkan renderer
        let vulkan_window = window_handle.vulkan_window_mut();
        if let Err(e) = self.vulkan_renderer.recreate_swapchain(vulkan_window) {
            log::error!("Failed to recreate swapchain: {:?}", e);
        }
    }
    
    /// Get the current swapchain extent (for aspect ratio calculations)
    pub fn get_swapchain_extent(&self) -> (u32, u32) {
        self.vulkan_renderer.get_swapchain_extent()
    }
    
    /// Render the world (legacy method for engine integration)
    pub fn render(&mut self, _world: &World) -> Result<(), RenderError> {
        // Legacy rendering path - just count frames
        self.frame_count += 1;
        Ok(())
    }
    
    /// Resize the renderer
    pub fn resize(&mut self, _width: u32, _height: u32) -> Result<(), RenderError> {
        // TODO: Implement resize
        Ok(())
    }
}

/// Camera for 3D rendering
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position
    pub position: Vec3,
    
    /// Camera target
    pub target: Vec3,
    
    /// Up vector
    pub up: Vec3,
    
    /// Field of view in radians
    pub fov: f32,
    
    /// Aspect ratio
    pub aspect: f32,
    
    /// Near clip plane
    pub near: f32,
    
    /// Far clip plane
    pub far: f32,
}

impl Camera {
    /// Create a new perspective camera (using Vulkan coordinate system)
    pub fn perspective(position: Vec3, fov_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position,
            target: Vec3::zeros(),
            up: Vec3::new(0.0, 1.0, 0.0),  // Standard Y-up in view space, coordinate transform handles Vulkan conventions
            fov: utils::deg_to_rad(fov_degrees),
            aspect,
            near,
            far,
        }
    }
    
    /// Set camera position
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
    }
    
    /// Set camera target (look-at point)
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
    }
    
    /// Look at a specific point
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.target = target;
        self.up = up;
    }
    
    /// Set aspect ratio
    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        if (self.aspect - aspect).abs() > f32::EPSILON {
            log::info!("Camera aspect ratio changed: {} → {}", self.aspect, aspect);
            self.aspect = aspect;
        }
    }
    
    /// Get view matrix
    pub fn get_view_matrix(&self) -> Mat4 {
        Mat4::look_at(self.position, self.target, self.up)
    }
    
    /// Get projection matrix
    pub fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective(self.fov, self.aspect, self.near, self.far)
    }
    
    /// Get view-projection matrix following Johannes Unterguggenberger's guide
    /// This implements C = P * X * V where:
    /// - V is the view matrix (world to view space)
    /// - X is the coordinate system transform for Vulkan
    /// - P is the perspective projection matrix
    pub fn get_view_projection_matrix(&self) -> Mat4 {
        let view_matrix = self.get_view_matrix();
        let coord_transform = Mat4::vulkan_coordinate_transform();
        let projection_matrix = self.get_projection_matrix();
        
        // Combine matrices in proper order: P * X * V
        projection_matrix * coord_transform * view_matrix
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 3.0, 3.0),  // Standard Y-up: positive Y = above, positive Z = away from origin
            target: Vec3::zeros(),
            up: Vec3::new(0.0, 1.0, 0.0),  // Standard Y-up convention, coordinate transform handles Vulkan
            fov: std::f32::consts::FRAC_PI_4,
            aspect: 16.0 / 9.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

/// Rendering errors
#[derive(Error, Debug)]
pub enum RenderError {
    /// Renderer initialization failed
    #[error("Renderer initialization failed: {0}")]
    InitializationFailed(String),
    
    /// Rendering operation failed
    #[error("Rendering failed: {0}")]
    RenderingFailed(String),
    
    /// Resource creation failed
    #[error("Resource creation failed: {0}")]
    ResourceCreationFailed(String),
}
