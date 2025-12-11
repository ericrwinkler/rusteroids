//! Transform component for the ECS system
//! 
//! Follows Game Engine Architecture principles:
//! - Pure data component (no logic)
//! - Immutable operations through associated functions
//! - Coordinate system consistency following Johannes Unterguggenberger guide

use crate::foundation::math::{Transform as MathTransform, Vec3, Mat4, Quat};
use crate::ecs::Component;

/// ECS Transform component
/// 
/// Pure data component representing spatial transformation in world space.
/// All coordinates follow Y-up right-handed conventions consistent with 
/// Johannes Unterguggenberger's academic standards.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformComponent {
    /// World space position (Y-up right-handed)
    pub position: Vec3,
    
    /// World space rotation quaternion
    pub rotation: Quat,
    
    /// World space scale factors
    pub scale: Vec3,
}

impl Component for TransformComponent {}

impl Default for TransformComponent {
    fn default() -> Self {
        Self {
            position: Vec3::zeros(),
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl TransformComponent {
    /// Create identity transform
    pub fn identity() -> Self {
        Self::default()
    }
    
    /// Create from position only
    pub fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Default::default()
        }
    }
    
    /// Create from position and rotation
    pub fn from_position_rotation(position: Vec3, rotation: Quat) -> Self {
        Self {
            position,
            rotation,
            ..Default::default()
        }
    }
    
    /// Create from full transform specification
    pub fn from_transform(position: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
        }
    }
    
    /// Convert to foundation math Transform for calculations
    pub fn to_math_transform(&self) -> MathTransform {
        MathTransform {
            position: self.position,
            rotation: self.rotation,
            scale: self.scale,
        }
    }
    
    /// Create from foundation math Transform
    pub fn from_math_transform(transform: &MathTransform) -> Self {
        Self {
            position: transform.position,
            rotation: transform.rotation,
            scale: transform.scale,
        }
    }
    
    /// Create from a transformation matrix (decompose TRS)
    pub fn from_matrix(matrix: Mat4) -> Self {
        let math_transform = MathTransform::from_matrix(matrix);
        Self::from_math_transform(&math_transform)
    }
    
    /// Convert to transformation matrix (TRS order)
    pub fn to_matrix(&self) -> Mat4 {
        self.to_math_transform().to_matrix()
    }
    
    /// Builder pattern: Set position
    pub fn with_position(mut self, position: Vec3) -> Self {
        self.position = position;
        self
    }
    
    /// Builder pattern: Set rotation from quaternion
    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }
    
    /// Builder pattern: Set rotation from Euler angles (radians, XYZ order)
    pub fn with_rotation_euler(mut self, x: f32, y: f32, z: f32) -> Self {
        self.rotation = Quat::from_euler_angles(x, y, z);
        self
    }
    
    /// Builder pattern: Set rotation from axis-angle
    pub fn with_rotation_axis_angle(mut self, axis: Vec3, angle: f32) -> Self {
        self.rotation = Quat::from_axis_angle(&nalgebra::Unit::new_normalize(axis), angle);
        self
    }
    
    /// Builder pattern: Set scale (uniform)
    pub fn with_uniform_scale(mut self, scale: f32) -> Self {
        self.scale = Vec3::new(scale, scale, scale);
        self
    }
    
    /// Builder pattern: Set scale (non-uniform)
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }
}

/// Transform factory for creating common transform configurations
/// 
/// Following Game Engine Architecture: Logic separated from data components
pub struct TransformFactory;

impl TransformFactory {
    /// Create directional light transform pointing in specific direction
    /// 
    /// # Arguments
    /// * `direction` - World space direction vector (Y-up right-handed)
    /// 
    /// # Coordinate System
    /// - Input direction in Y-up right-handed world space
    /// - Output transform rotation aligns -Z axis with direction
    /// - Consistent with Johannes Unterguggenberger coordinate standards
    pub fn directional_light(direction: Vec3) -> TransformComponent {
        let normalized_direction = direction.normalize();
        
        // Default forward direction is -Z (away from viewer in right-handed Y-up)
        let default_forward = Vec3::new(0.0, 0.0, -1.0);
        
        // Calculate rotation to align default forward with desired direction
        let rotation = if (normalized_direction + default_forward).magnitude() < 1e-6 {
            // Special case: direction is opposite to default forward
            Quat::from_axis_angle(&Vec3::y_axis(), std::f32::consts::PI)
        } else if (normalized_direction - default_forward).magnitude() < 1e-6 {
            // Direction already matches default
            Quat::identity()
        } else {
            // General case: rotate from default to target direction
            Quat::rotation_between(&default_forward, &normalized_direction).unwrap_or(Quat::identity())
        };
        
        TransformComponent {
            position: Vec3::zeros(), // Directional lights have no position
            rotation,
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
    
    /// Create point light transform at specific position
    pub fn point_light(position: Vec3) -> TransformComponent {
        TransformComponent {
            position,
            rotation: Quat::identity(), // Point lights have no orientation
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
    
    /// Create camera transform with look-at behavior
    pub fn look_at(position: Vec3, target: Vec3, up: Vec3) -> TransformComponent {
        let forward = (target - position).normalize();
        let right = forward.cross(&up.normalize()).normalize();
        let camera_up = right.cross(&forward);
        
        // Build rotation matrix and convert to quaternion
        let rotation_matrix = Mat4::new(
            right.x, camera_up.x, -forward.x, 0.0,
            right.y, camera_up.y, -forward.y, 0.0,
            right.z, camera_up.z, -forward.z, 0.0,
            0.0, 0.0, 0.0, 1.0,
        );
        
        let rotation_mat3 = rotation_matrix.fixed_view::<3, 3>(0, 0).into_owned();
        let rotation = Quat::from_matrix(&rotation_mat3);
        
        TransformComponent {
            position,
            rotation,
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::math::constants::PI;
    use approx::assert_relative_eq;
    
    const EPSILON: f32 = 1e-6;
    
    #[test]
    fn test_transform_identity() {
        let transform = TransformComponent::identity();
        
        assert_eq!(transform.position, Vec3::zeros());
        assert_relative_eq!(transform.rotation, Quat::identity(), epsilon = EPSILON);
        assert_eq!(transform.scale, Vec3::new(1.0, 1.0, 1.0));
    }
    
    #[test]
    fn test_transform_from_position() {
        let position = Vec3::new(1.0, 2.0, 3.0);
        let transform = TransformComponent::from_position(position);
        
        assert_eq!(transform.position, position);
        assert_relative_eq!(transform.rotation, Quat::identity(), epsilon = EPSILON);
        assert_eq!(transform.scale, Vec3::new(1.0, 1.0, 1.0));
    }
    
    #[test]
    fn test_coordinate_system_consistency() {
        // Test that our coordinate system matches Y-up right-handed conventions
        
        // Positive X should be "right"
        let x_pos = Vec3::new(1.0, 0.0, 0.0);
        
        // Positive Y should be "up"  
        let y_pos = Vec3::new(0.0, 1.0, 0.0);
        
        // Positive Z should be "toward viewer" (right-handed)
        let z_pos = Vec3::new(0.0, 0.0, 1.0);
        
        // Cross product test: X × Y should equal Z in right-handed system
        let cross_xy = x_pos.cross(&y_pos);
        assert_relative_eq!(cross_xy, z_pos, epsilon = EPSILON);
        
        // Y × Z should equal X
        let cross_yz = y_pos.cross(&z_pos);
        assert_relative_eq!(cross_yz, x_pos, epsilon = EPSILON);
        
        // Z × X should equal Y
        let cross_zx = z_pos.cross(&x_pos);
        assert_relative_eq!(cross_zx, y_pos, epsilon = EPSILON);
    }
    
    #[test]
    fn test_directional_light_coordinate_validation() {
        // Test the exact coordinate values from our lighting system
        // This preserves the (-0.7, -1.0, 0.3) direction validation
        let direction = Vec3::new(-0.7, -1.0, 0.3);
        let transform = TransformFactory::directional_light(direction);
        
        // Verify the rotation produces the expected direction
        let math_transform = transform.to_math_transform();
        let forward_dir = math_transform.rotation * Vec3::new(0.0, 0.0, -1.0);
        
        // Should match the normalized input direction
        let expected_direction = direction.normalize();
        assert_relative_eq!(forward_dir, expected_direction, epsilon = 1e-5);
    }
    
    #[test]
    fn test_quaternion_rotation_consistency() {
        // Test various rotation configurations for consistency
        
        // 90-degree rotation around Y axis
        let rotation_y = Quat::from_axis_angle(&Vec3::y_axis(), PI / 2.0);
        let transform = TransformComponent::from_position_rotation(Vec3::zeros(), rotation_y);
        
        // Transform X-axis vector, should become Z-axis
        let x_axis = Vec3::new(1.0, 0.0, 0.0);
        let math_transform = transform.to_math_transform();
        let rotated = math_transform.rotation * x_axis;
        
        // In right-handed Y-up, rotating X by 90° around Y should give -Z
        let expected = Vec3::new(0.0, 0.0, -1.0);
        assert_relative_eq!(rotated, expected, epsilon = EPSILON);
    }
    
    #[test]
    fn test_matrix_roundtrip_consistency() {
        // Test Transform ↔ Matrix conversion consistency
        let original_transform = TransformComponent::from_transform(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_axis_angle(&nalgebra::Unit::new_normalize(Vec3::new(1.0, 1.0, 1.0)), 0.5),
            Vec3::new(2.0, 1.5, 0.8)
        );
        
        // Convert to math transform and matrix
        let math_transform = original_transform.to_math_transform();
        let matrix = math_transform.to_matrix();
        
        // Convert back from matrix
        let reconstructed_math = MathTransform::from_matrix(matrix);
        let reconstructed_transform = TransformComponent::from_math_transform(&reconstructed_math);
        
        // Should be approximately equal (within floating-point precision)
        assert_relative_eq!(reconstructed_transform.position, original_transform.position, epsilon = 1e-5);
        assert_relative_eq!(reconstructed_transform.scale, original_transform.scale, epsilon = 1e-5);
        
        // Quaternions might flip sign but represent same rotation
        let dot = original_transform.rotation.coords.dot(&reconstructed_transform.rotation.coords);
        assert!(dot.abs() > 0.999, "Quaternion rotation mismatch: dot product = {}", dot);
    }
    
    #[test]
    fn test_coordinate_parameter_preservation() {
        // Validate that specific coordinate parameters are preserved exactly
        // This ensures our transformation chain maintains coordinate integrity
        
        let test_positions = vec![
            Vec3::new(-0.7, -1.0, 0.3),  // From our lighting validation
            Vec3::new(0.0, 1.0, 0.0),    // Pure Y-up
            Vec3::new(1.0, 0.0, 0.0),    // Pure X-right
            Vec3::new(0.0, 0.0, 1.0),    // Pure Z-forward
            Vec3::new(-1.0, -1.0, -1.0), // Negative octant
        ];
        
        for position in test_positions {
            let transform = TransformComponent::from_position(position);
            
            // Position should be preserved exactly
            assert_eq!(transform.position, position);
            
            // Apply identity matrix transformation
            let math_transform = transform.to_math_transform();
            let matrix = math_transform.to_matrix();
            
            // For identity transform, position should be preserved
            let transformed_position = matrix.transform_vector(&position);
            assert_relative_eq!(transformed_position, position, epsilon = EPSILON);
        }
    }
    
    #[test]
    fn test_look_at_transform_validation() {
        // Test camera look-at transforms follow right-handed conventions
        
        let eye = Vec3::new(0.0, 0.0, 5.0);     // Camera at positive Z
        let target = Vec3::new(0.0, 0.0, 0.0);  // Looking at origin
        let up = Vec3::new(0.0, 1.0, 0.0);      // Y-up
        
        let transform = TransformFactory::look_at(eye, target, up);
        
        // Verify position
        assert_relative_eq!(transform.position, eye, epsilon = EPSILON);
        
        // Verify forward direction (should be -Z in our coordinate system)
        let math_transform = transform.to_math_transform();
        let forward = math_transform.rotation * Vec3::new(0.0, 0.0, -1.0);
        let expected_forward = (target - eye).normalize();
        
        assert_relative_eq!(forward, expected_forward, epsilon = 1e-5);
    }
    
    #[test]
    fn test_transform_combination_validation() {
        // Test transform combination follows correct mathematical order
        
        let parent = TransformComponent::from_position_rotation(
            Vec3::new(1.0, 0.0, 0.0),
            Quat::from_axis_angle(&Vec3::y_axis(), PI / 2.0)
        );
        
        let child = TransformComponent::from_position(Vec3::new(0.0, 0.0, 1.0));
        
        // Combine transforms
        let parent_math = parent.to_math_transform();
        let child_math = child.to_math_transform();
        let combined = parent_math.combine(&child_math);
        
        // Child position (0,0,1) rotated 90° around Y and translated by (1,0,0)
        // Should result in position (1,0,-1) in world space
        let expected_position = Vec3::new(1.0, 0.0, -1.0);
        assert_relative_eq!(combined.position, expected_position, epsilon = 1e-5);
    }
    
    #[test]
    fn test_inverse_transform_validation() {
        // Test that transform inverse operations are mathematically correct
        
        let original = TransformComponent::from_transform(
            Vec3::new(2.0, 3.0, 1.0),
            Quat::from_axis_angle(&nalgebra::Unit::new_normalize(Vec3::new(0.0, 1.0, 0.0)), 0.785),  // 45 degrees
            Vec3::new(2.0, 2.0, 2.0)
        );
        
        let math_transform = original.to_math_transform();
        let inverse = math_transform.inverse();
        
        // Combining transform with its inverse should yield identity
        let should_be_identity = math_transform.combine(&inverse);
        
        assert_relative_eq!(should_be_identity.position, Vec3::zeros(), epsilon = 1e-5);
        assert_relative_eq!(should_be_identity.scale, Vec3::new(1.0, 1.0, 1.0), epsilon = 1e-5);
        
        // Rotation should be approximately identity
        let identity_quat = Quat::identity();
        let rotation_dot = should_be_identity.rotation.coords.dot(&identity_quat.coords);
        assert!(rotation_dot.abs() > 0.999, "Inverse rotation validation failed");
    }
}
