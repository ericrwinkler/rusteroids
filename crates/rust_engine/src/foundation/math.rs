//! Math utilities and types
//!
//! Provides fundamental math types for 3D graphics and game development.

pub use nalgebra::{
    Vector2, Vector3, Vector4,
    Matrix3, Matrix4,
    Quaternion,
    Unit,
};

/// 2D vector type
pub type Vec2 = Vector2<f32>;

/// 3D vector type
pub type Vec3 = Vector3<f32>;

/// 4D vector type
pub type Vec4 = Vector4<f32>;

/// 3x3 matrix type
pub type Mat3 = Matrix3<f32>;

/// 4x4 matrix type
pub type Mat4 = Matrix4<f32>;

/// 2D point type
pub type Point2 = nalgebra::Point2<f32>;

/// 3D point type
pub type Point3 = nalgebra::Point3<f32>;

/// Quaternion type for rotations
pub type Quat = Unit<Quaternion<f32>>;

/// Transform representing position, rotation, and scale
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    /// Position in 3D space
    pub position: Vec3,
    
    /// Rotation quaternion
    pub rotation: Quat,
    
    /// Scale factors
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec3::zeros(),
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    /// Create a new identity transform
    pub fn identity() -> Self {
        Self::default()
    }
    
    /// Create a transform with only position
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
    
    /// Create a transform with position and rotation
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Default::default()
        }
    }
    
    /// Convert to a transformation matrix
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::new_translation(&self.position)
            * self.rotation.to_homogeneous()
            * Mat4::new_nonuniform_scaling(&self.scale)
    }
    
    /// Apply this transform to a point
    pub fn transform_point(&self, point: Point3) -> Point3 {
        let matrix = self.to_matrix();
        matrix.transform_point(&point)
    }
    
    /// Apply this transform to a vector
    pub fn transform_vector(&self, vector: Vec3) -> Vec3 {
        let matrix = self.to_matrix();
        matrix.transform_vector(&vector)
    }
    
    /// Create a transform from a translation vector
    pub fn from_translation(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
    
    /// Create a transform from a transformation matrix
    pub fn from_matrix(matrix: Mat4) -> Self {
        // Extract position
        let position = Vec3::new(matrix.m14, matrix.m24, matrix.m34);
        
        // Extract scale from the matrix columns
        let scale_x = Vec3::new(matrix.m11, matrix.m21, matrix.m31).magnitude();
        let scale_y = Vec3::new(matrix.m12, matrix.m22, matrix.m32).magnitude();
        let scale_z = Vec3::new(matrix.m13, matrix.m23, matrix.m33).magnitude();
        let scale = Vec3::new(scale_x, scale_y, scale_z);
        
        // Extract rotation by removing scale from the rotation matrix
        let rotation_matrix = Matrix3::new(
            matrix.m11 / scale_x, matrix.m12 / scale_y, matrix.m13 / scale_z,
            matrix.m21 / scale_x, matrix.m22 / scale_y, matrix.m23 / scale_z,
            matrix.m31 / scale_x, matrix.m32 / scale_y, matrix.m33 / scale_z,
        );
        let rotation = Quat::from_matrix(&rotation_matrix);
        
        Self {
            position,
            rotation,
            scale,
        }
    }
    
    /// Combine this transform with another
    pub fn combine(&self, other: &Transform) -> Transform {
        Transform {
            position: self.position + self.rotation * (self.scale.component_mul(&other.position)),
            rotation: self.rotation * other.rotation,
            scale: self.scale.component_mul(&other.scale),
        }
    }
    
    /// Get the inverse transform
    pub fn inverse(&self) -> Transform {
        let inv_scale = Vec3::new(1.0 / self.scale.x, 1.0 / self.scale.y, 1.0 / self.scale.z);
        let inv_rotation = self.rotation.inverse();
        let inv_position = inv_rotation * (-self.position.component_mul(&inv_scale));
        
        Transform {
            position: inv_position,
            rotation: inv_rotation,
            scale: inv_scale,
        }
    }
}

/// Math constants
pub mod constants {
    /// Pi constant
    pub const PI: f32 = std::f32::consts::PI;
    
    /// 2 * Pi
    pub const TAU: f32 = 2.0 * PI;
    
    /// Pi / 2
    pub const HALF_PI: f32 = PI * 0.5;
    
    /// Pi / 4
    pub const QUARTER_PI: f32 = PI * 0.25;
    
    /// Degrees to radians conversion factor
    pub const DEG_TO_RAD: f32 = PI / 180.0;
    
    /// Radians to degrees conversion factor
    pub const RAD_TO_DEG: f32 = 180.0 / PI;
}

/// Math utility functions
pub mod utils {
    use super::*;
    
    /// Convert degrees to radians
    pub fn deg_to_rad(degrees: f32) -> f32 {
        degrees * constants::DEG_TO_RAD
    }
    
    /// Convert radians to degrees
    pub fn rad_to_deg(radians: f32) -> f32 {
        radians * constants::RAD_TO_DEG
    }
    
    /// Clamp a value between min and max
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        if value < min { min } else if value > max { max } else { value }
    }
    
    /// Linear interpolation
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}

/// Extension trait for Mat4 with additional convenience methods
pub trait Mat4Ext {
    /// Create a rotation matrix around the X axis
    fn rotation_x(angle: f32) -> Mat4;
    
    /// Create a rotation matrix around the Y axis  
    fn rotation_y(angle: f32) -> Mat4;
    
    /// Create a rotation matrix around the Z axis
    fn rotation_z(angle: f32) -> Mat4;
    
    /// Create a perspective projection matrix
    fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4;
    
    /// Create a look-at view matrix
    fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4;
    
    /// Create the intermediate coordinate system transformation for Vulkan
    /// This implements the X matrix from the guide to prepare coordinates for Vulkan's conventions
    fn vulkan_coordinate_transform() -> Mat4;
}

impl Mat4Ext for Mat4 {
    fn rotation_x(angle: f32) -> Mat4 {
        Mat4::from_axis_angle(&Vec3::x_axis(), angle)
    }
    
    fn rotation_y(angle: f32) -> Mat4 {
        Mat4::from_axis_angle(&Vec3::y_axis(), angle)
    }
    
    fn rotation_z(angle: f32) -> Mat4 {
        Mat4::from_axis_angle(&Vec3::z_axis(), angle)
    }
    
    fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
        // Proper Vulkan perspective matrix following Johannes Unterguggenberger's guide
        // https://johannesugb.github.io/gpu-programming/setting-up-a-proper-vulkan-projection-matrix/
        // This implements the P matrix from Equation 3 in the guide
        let tan_half_fovy = (fov_y * 0.5).tan();
        
        let mut result = Mat4::zeros();
        
        // Following the guide's formula exactly:
        // P = [a⁻¹/tan(φ/2)    0              0                    0           ]
        //     [0               1/tan(φ/2)     0                    0           ]
        //     [0               0              f/(f-n)              -nf/(f-n)   ]
        //     [0               0              1                    0           ]
        
        result[(0, 0)] = 1.0 / (aspect * tan_half_fovy);  // a⁻¹/tan(φ/2) where a⁻¹ is aspect ratio
        result[(1, 1)] = 1.0 / tan_half_fovy;             // 1/tan(φ/2) - positive, no Y-flip here
        result[(2, 2)] = far / (far - near);               // f/(f-n) - depth mapping to [0,1]
        result[(2, 3)] = -(near * far) / (far - near);     // -nf/(f-n) - depth offset
        result[(3, 2)] = 1.0;                              // Perspective divide trigger
        
        result
    }
    
    fn look_at(eye: Vec3, target: Vec3, up: Vec3) -> Mat4 {
        // Right-handed look-at matrix for Vulkan coordinate system
        // Vulkan uses Y-down, Z-into-screen convention
        let forward = (target - eye).normalize();
        let right = forward.cross(&up).normalize();
        let camera_up = right.cross(&forward);
        
        let translation = Mat4::new(
            1.0, 0.0, 0.0, -eye.x,
            0.0, 1.0, 0.0, -eye.y,
            0.0, 0.0, 1.0, -eye.z,
            0.0, 0.0, 0.0, 1.0,
        );
        
        let rotation = Mat4::new(
            right.x, right.y, right.z, 0.0,
            camera_up.x, camera_up.y, camera_up.z, 0.0,
            -forward.x, -forward.y, -forward.z, 0.0,  // Negative forward for right-handed
            0.0, 0.0, 0.0, 1.0,
        );
        
        rotation * translation
    }
    
    fn vulkan_coordinate_transform() -> Mat4 {
        // Intermediate coordinate system transformation from Johannes Unterguggenberger's guide
        // This implements the X matrix from Equation 2 in the guide:
        // X = [1   0   0  0]⁻¹
        //     [0  -1   0  0]
        //     [0   0  -1  0] 
        //     [0   0   0  1]
        // 
        // The inverse flips Y and Z axes to align with Vulkan's expectations:
        // - X axis remains unchanged (right is right)  
        // - Y axis flipped (up becomes down for Vulkan Y-down)
        // - Z axis flipped (forward becomes into screen for Vulkan Z-forward)
        Mat4::new(
            1.0,  0.0,  0.0, 0.0,
            0.0, -1.0,  0.0, 0.0,
            0.0,  0.0, -1.0, 0.0,
            0.0,  0.0,  0.0, 1.0,
        )
    }
}
