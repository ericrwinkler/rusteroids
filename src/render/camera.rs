// src/render/camera.rs
use crate::util::math::{Mat4, Vec3};

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub view_matrix: Mat4,
    pub projection_matrix: Mat4,
    pub view_projection_matrix: Mat4,
}

impl Camera {
    pub fn new(
        position: Vec3,
        target: Vec3,
        up: Vec3,
        fov: f32,
        aspect_ratio: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Self {
        let mut camera = Self {
            position,
            target,
            up,
            fov,
            aspect_ratio,
            near_plane,
            far_plane,
            view_matrix: Mat4::identity(),
            projection_matrix: Mat4::identity(),
            view_projection_matrix: Mat4::identity(),
        };
        camera.update_matrices();
        camera
    }

    pub fn perspective(
        position: Vec3,
        fov: f32,
        aspect_ratio: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Self {
        Self::new(
            position,
            Vec3::new(0.0, 0.0, 0.0), // Look at origin
            Vec3::new(0.0, 1.0, 0.0), // Up is Y axis
            fov,
            aspect_ratio,
            near_plane,
            far_plane,
        )
    }

    pub fn orthographic(
        position: Vec3,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near_plane: f32,
        far_plane: f32,
    ) -> Self {
        let mut camera = Self::new(
            position,
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            90.0, // FOV not used for orthographic
            1.0,  // Aspect ratio not used for orthographic
            near_plane,
            far_plane,
        );
        camera.projection_matrix = Mat4::orthographic(left, right, bottom, top, near_plane, far_plane);
        camera.update_view_projection();
        camera
    }

    pub fn set_position(&mut self, position: Vec3) {
        self.position = position;
        self.update_matrices();
    }

    pub fn set_target(&mut self, target: Vec3) {
        self.target = target;
        self.update_matrices();
    }

    pub fn look_at(&mut self, target: Vec3) {
        self.target = target;
        self.update_matrices(); 
    }

    pub fn set_aspect_ratio(&mut self, aspect_ratio: f32) {
        self.aspect_ratio = aspect_ratio;
        self.update_matrices();
    }

    pub fn translate(&mut self, delta: Vec3) {
        self.position = self.position + delta;
        self.target = self.target + delta;
        self.update_matrices();
    }

    pub fn rotate_around_target(&mut self, angle_x: f32, angle_y: f32) {
        // Simple orbit camera rotation
        let distance = (self.position - self.target).length();
        
        // Convert to spherical coordinates
        let current_dir = (self.position - self.target).normalize();
        let mut theta = current_dir.z.atan2(current_dir.x);
        let mut phi = current_dir.y.asin();
        
        theta += angle_y;
        phi += angle_x;
        
        // Clamp phi to avoid gimbal lock
        phi = phi.max(-std::f32::consts::FRAC_PI_2 + 0.01)
                .min(std::f32::consts::FRAC_PI_2 - 0.01);
        
        // Convert back to Cartesian
        let new_dir = Vec3::new(
            phi.cos() * theta.cos(),
            phi.sin(),
            phi.cos() * theta.sin(),
        );
        
        self.position = self.target + new_dir * distance;
        self.update_matrices();
    }

    fn update_matrices(&mut self) {
        self.update_view_matrix();
        self.update_projection_matrix();
        self.update_view_projection();
    }

    fn update_view_matrix(&mut self) {
        self.view_matrix = Mat4::look_at(self.position, self.target, self.up);
    }

    fn update_projection_matrix(&mut self) {
        self.projection_matrix = Mat4::perspective(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.near_plane,
            self.far_plane,
        );
    }

    fn update_view_projection(&mut self) {
        self.view_projection_matrix = self.projection_matrix * self.view_matrix;
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        self.view_matrix
    }

    pub fn get_projection_matrix(&self) -> Mat4 {
        self.projection_matrix
    }

    pub fn get_view_projection_matrix(&self) -> Mat4 {
        self.view_projection_matrix
    }

    pub fn get_forward(&self) -> Vec3 {
        (self.target - self.position).normalize()
    }

    pub fn get_right(&self) -> Vec3 {
        self.get_forward().cross(self.up).normalize()
    }

    pub fn get_up(&self) -> Vec3 {
        self.up
    }
}
