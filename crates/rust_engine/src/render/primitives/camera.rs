//! # 3D Camera System
//!
//! Provides camera abstractions for 3D rendering with proper matrix mathematics
//! following Johannes Unterguggenberger's academically correct approach.
//!
//! ## Design Principles
//! - **Library-agnostic**: No direct Vulkan dependencies in camera math
//! - **Immutable operation**: Methods don't modify camera state unexpectedly
//! - **Mathematical correctness**: Follows established computer graphics conventions
//! - **Flexible API**: Supports various camera creation and manipulation patterns

use crate::foundation::math::{Vec3, Mat4, Mat4Ext, utils};

/// 3D Camera for perspective and orthographic projections
///
/// Represents a camera in 3D space with position, orientation, and projection parameters.
/// Supports both perspective and orthographic projections with proper matrix generation
/// following Johannes Unterguggenberger's mathematical approach.
///
/// # Coordinate System
/// Uses standard right-handed Y-up coordinate system in view space:
/// - X+ = Right
/// - Y+ = Up  
/// - Z+ = Forward (towards viewer, opposite of OpenGL convention)
///
/// The Vulkan coordinate transformation is applied separately to convert to Vulkan's
/// Y-down NDC convention while preserving standard view space mathematics.
///
/// # Performance Notes
/// Matrix calculations are performed on-demand rather than cached. For performance-
/// critical applications with static cameras, consider caching the computed matrices.
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position in world space
    pub position: Vec3,
    
    /// Point the camera is looking at in world space
    pub target: Vec3,
    
    /// Up vector for camera orientation (typically [0, 1, 0])
    pub up: Vec3,
    
    /// Field of view angle in radians (for perspective projection)
    pub fov: f32,
    
    /// Aspect ratio (width / height) for projection calculations
    pub aspect: f32,
    
    /// Distance to near clipping plane
    pub near: f32,
    
    /// Distance to far clipping plane  
    pub far: f32,
}

impl Camera {
    /// Create a new perspective camera with standard Y-up orientation
    ///
    /// Creates a perspective camera suitable for 3D rendering with the specified
    /// field of view, aspect ratio, and clipping plane distances.
    ///
    /// # Arguments
    /// * `position` - Camera position in world space
    /// * `fov_degrees` - Field of view angle in degrees (converted to radians internally)
    /// * `aspect` - Aspect ratio (width / height) of the viewport
    /// * `near` - Distance to near clipping plane (must be > 0)
    /// * `far` - Distance to far clipping plane (must be > near)
    ///
    /// # Returns
    /// New Camera instance configured for perspective projection
    ///
    /// # Example
    /// ```rust
    /// use rust_engine::foundation::math::Vec3;
    /// use rust_engine::render::primitives::Camera;
    /// 
    /// let camera = Camera::perspective(
    ///     Vec3::new(0.0, 2.0, 5.0),  // Position 5 units back, 2 up
    ///     75.0,                       // 75-degree field of view
    ///     16.0 / 9.0,                // Widescreen aspect ratio
    ///     0.1,                        // Near plane at 10cm
    ///     100.0                       // Far plane at 100 meters
    /// );
    /// ```
    ///
    /// # Design Notes
    /// The default target is origin [0,0,0] and up vector is +Y [0,1,0], providing
    /// a sensible starting configuration that can be customized after creation.
    pub fn perspective(position: Vec3, fov_degrees: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            position,
            target: Vec3::zeros(), // Default to looking at origin
            up: Vec3::new(0.0, 1.0, 0.0), // Standard Y-up orientation
            fov: utils::deg_to_rad(fov_degrees), // Convert degrees to radians for internal use
            aspect,
            near,
            far,
        }
    }
    
    /// Update camera position in world space
    ///
    /// Sets the camera position to a new location while preserving the current
    /// target and orientation.
    ///
    /// # Arguments
    /// * `position` - New camera position in world coordinates
    ///
    /// # Performance Notes
    /// This method is lightweight and doesn't trigger any matrix recalculations
    /// until the next call to get_view_matrix() or get_projection_matrix().
    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        log::trace!("Camera position updated to: {:?}", position);
    }
    
    /// Update camera target (look-at point)
    ///
    /// Changes where the camera is pointing without moving the camera position.
    ///
    /// # Arguments
    /// * `target` - Point in world space to look at
    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
        log::trace!("Camera target updated to: {:?}", target);
    }
    
    /// Configure camera to look at a specific point with custom up vector
    ///
    /// Simultaneously sets the target point and up vector for camera orientation.
    /// This is useful for creating specific camera orientations or for orbit-style
    /// camera controls.
    ///
    /// # Arguments
    /// * `target` - Point in world space to look at
    /// * `up` - Up vector for camera orientation (should be normalized)
    ///
    /// # Mathematical Notes
    /// The up vector doesn't need to be perpendicular to the view direction.
    /// The view matrix calculation will automatically orthonormalize the vectors
    /// using the Gram-Schmidt process.
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.target = target;
        self.up = up;
        log::trace!("Camera look_at updated - target: {:?}, up: {:?}", target, up);
    }
    
    /// Update camera aspect ratio for viewport changes
    ///
    /// Updates the aspect ratio used for projection matrix calculations.
    /// This is typically called when the window/viewport is resized.
    ///
    /// # Arguments
    /// * `aspect` - New aspect ratio (width / height)
    ///
    /// # Automatic Change Detection
    /// Only logs aspect ratio changes when the difference is significant
    /// (> 0.01) to reduce log noise during window resize events.
    pub fn set_aspect_ratio(&mut self, aspect: f32) {
        // Use a larger threshold to prevent spam during window resize
        if (self.aspect - aspect).abs() > 0.01 {
            log::info!("Camera aspect ratio changed: {:.3} -> {:.3}", self.aspect, aspect);
            self.aspect = aspect;
        } else {
            // Still update the aspect ratio, just don't log it
            self.aspect = aspect;
        }
    }
    
    /// Generate view matrix for world-to-camera space transformation
    ///
    /// Creates the view matrix that transforms vertices from world space to
    /// camera space using the camera's position, target, and up vector.
    ///
    /// # Returns
    /// 4x4 view transformation matrix
    ///
    /// # Mathematical Implementation
    /// Uses the standard look-at matrix calculation following right-handed
    /// coordinate system conventions. The transformation:
    /// 1. Translates world by -camera_position
    /// 2. Rotates to align world axes with camera axes
    ///
    /// # Coordinate System Notes
    /// Generates matrices for standard Y-up view space. The separate Vulkan
    /// coordinate transformation matrix handles conversion to Vulkan conventions.
    pub fn get_view_matrix(&self) -> Mat4 {
        // Use standard look-at matrix generation (Y-up, right-handed)
        // Vulkan coordinate transformation is applied separately
        Mat4::look_at(self.position, self.target, self.up)
    }
    
    /// Generate perspective projection matrix  
    ///
    /// Creates the projection matrix for perspective rendering using the camera's
    /// field of view, aspect ratio, and clipping planes.
    ///
    /// # Returns
    /// 4x4 perspective projection matrix
    ///
    /// # Mathematical Implementation
    /// Follows Johannes Unterguggenberger's mathematically correct approach for
    /// perspective projection. Uses standard right-handed coordinate conventions
    /// in view space before Vulkan coordinate transformation.
    ///
    /// # Aspect Ratio Handling
    /// Automatically uses the current aspect ratio. For dynamic viewports,
    /// ensure set_aspect_ratio() is called when window dimensions change.
    pub fn get_projection_matrix(&self) -> Mat4 {
        // Use standard perspective projection (Y-up view space)
        // Vulkan coordinate transformation applied separately in rendering pipeline
        Mat4::perspective(self.fov, self.aspect, self.near, self.far)
    }
    
    /// Generate combined view-projection matrix
    ///
    /// Creates the complete view-projection transformation following Johannes
    /// Unterguggenberger's mathematically correct approach: P × X × V
    ///
    /// # Matrix Chain Components
    /// - P = Perspective projection matrix
    /// - X = Vulkan coordinate transformation matrix  
    /// - V = View matrix (world to camera space)
    ///
    /// # Returns
    /// 4x4 combined view-projection matrix ready for rendering
    ///
    /// # Usage Notes
    /// This method provides the complete camera transformation. For rendering
    /// individual objects, multiply this result by the model matrix:
    /// `Final = ViewProjection × Model × Vertex`
    ///
    /// # Performance Consideration
    /// This method recalculates matrices on every call. For static cameras,
    /// consider caching the result until camera parameters change.
    pub fn get_view_projection_matrix(&self) -> Mat4 {
        // Calculate component matrices
        let view_matrix = self.get_view_matrix();
        let coord_transform = Mat4::vulkan_coordinate_transform();
        let projection_matrix = self.get_projection_matrix();
        
        // Combine in mathematically correct order: P × X × V
        // This follows Johannes Unterguggenberger's academic approach
        projection_matrix * coord_transform * view_matrix
    }

    /// Convert screen coordinates to a world-space ray
    ///
    /// Takes normalized device coordinates (NDC) and generates a ray in world space
    /// that originates at the camera position and points through the screen position.
    ///
    /// # Arguments
    /// * `screen_x` - Screen X coordinate in NDC (-1 to 1, left to right)
    /// * `screen_y` - Screen Y coordinate in NDC (-1 to 1, bottom to top)
    ///
    /// # Returns
    /// Ray in world space originating at camera position
    ///
    /// # Mathematical Process
    /// 1. Convert NDC to clip space (add Z = -1 for near plane, W = 1)
    /// 2. Inverse projection: clip → view space
    /// 3. Inverse view: view → world space
    /// 4. Normalize direction vector
    ///
    /// # Usage Example
    /// ```rust
    /// // Convert mouse position to NDC first (range -1 to 1)
    /// let ndc_x = (mouse_x / window_width) * 2.0 - 1.0;
    /// let ndc_y = 1.0 - (mouse_y / window_height) * 2.0; // Flip Y
    /// let ray = camera.screen_to_world_ray(ndc_x, ndc_y);
    /// ```
    pub fn screen_to_world_ray(&self, screen_x: f32, screen_y: f32) -> crate::physics::collision::Ray {
        use crate::foundation::math::Vec4;
        
        // Get the combined view-projection matrix and invert it
        // Matrix order: P × X × V, so inverse is: inv(V) × inv(X) × inv(P)
        let view_proj = self.get_view_projection_matrix();
        let inv_view_proj = view_proj.try_inverse().expect("Failed to invert view-projection matrix");
        
        // Create NDC points at near and far planes
        let ndc_near = Vec4::new(screen_x, screen_y, -1.0, 1.0); // Near plane (Vulkan: Z=-1)
        let ndc_far = Vec4::new(screen_x, screen_y, 1.0, 1.0);   // Far plane (Vulkan: Z=+1)
        
        // Unproject both points from NDC to world space
        let world_near_h = inv_view_proj * ndc_near;
        let world_far_h = inv_view_proj * ndc_far;
        
        // Perspective divide to get 3D world coordinates
        let world_near = Vec3::new(
            world_near_h.x / world_near_h.w,
            world_near_h.y / world_near_h.w,
            world_near_h.z / world_near_h.w,
        );
        let world_far = Vec3::new(
            world_far_h.x / world_far_h.w,
            world_far_h.y / world_far_h.w,
            world_far_h.z / world_far_h.w,
        );
        
        // Ray direction from near to far
        let ray_direction = (world_far - world_near).normalize();
        
        // For perspective camera, origin is camera position
        // Direction points from camera through the clicked point into the scene
        let ray_origin = self.position;
        
        crate::physics::collision::Ray::new(ray_origin, ray_direction)
    }
}

impl Default for Camera {
    /// Create a default perspective camera with sensible settings
    ///
    /// Provides a reasonable starting camera configuration suitable for most
    /// 3D applications. Positioned above and behind the origin, looking down
    /// at the scene center.
    ///
    /// # Default Configuration
    /// - Position: (0, 3, 3) - Above and behind origin
    /// - Target: (0, 0, 0) - Looking at origin
    /// - Up: (0, 1, 0) - Standard Y-up orientation
    /// - FOV: 45 degrees - Natural perspective view
    /// - Aspect: 16:9 - Common widescreen ratio
    /// - Near: 0.1 - Close near plane for detailed objects
    /// - Far: 1000.0 - Distant far plane for large scenes
    ///
    /// # Coordinate System
    /// Uses standard Y-up right-handed coordinates, compatible with common
    /// 3D modeling conventions and the engine's mathematical foundation.
    fn default() -> Self {
        Self {
            position: Vec3::new(0.0, 3.0, 3.0), // Above and behind origin - good default viewpoint
            target: Vec3::zeros(),               // Look at scene center
            up: Vec3::new(0.0, 1.0, 0.0),      // Standard Y-up orientation
            fov: std::f32::consts::FRAC_PI_4,   // 45 degrees in radians - natural FOV
            aspect: 16.0 / 9.0,                 // Widescreen aspect ratio
            near: 0.1,                          // Close near plane - good for detail
            far: 1000.0,                        // Distant far plane - good for landscapes
        }
    }
}
