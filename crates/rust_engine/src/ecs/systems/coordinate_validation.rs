//! Simple coordinate system validation tests
//! 
//! Basic tests for coordinate transformations following Johannes Unterguggenberger's guide

use crate::foundation::math::{Mat4Ext, Vec3, Mat4};

/// Simple coordinate system validator
pub struct CoordinateSystemValidator;

impl CoordinateSystemValidator {
    
    /// Validate right-handed coordinate system conventions
    pub fn validate_right_handed_system() -> Result<(), String> {
        // Standard right-handed basis vectors
        let x_axis = Vec3::new(1.0, 0.0, 0.0); // Right
        let y_axis = Vec3::new(0.0, 1.0, 0.0); // Up
        let z_axis = Vec3::new(0.0, 0.0, 1.0); // Forward (toward viewer)
        
        // Verify cross products follow right-hand rule
        let cross_xy = x_axis.cross(&y_axis);
        if (cross_xy - z_axis).magnitude() > EPSILON {
            return Err(format!("Right-hand rule violation: X × Y = {:?}, expected Z = {:?}", cross_xy, z_axis));
        }
        
        let cross_yz = y_axis.cross(&z_axis);
        if (cross_yz - x_axis).magnitude() > EPSILON {
            return Err(format!("Right-hand rule violation: Y × Z = {:?}, expected X = {:?}", cross_yz, x_axis));
        }
        
        let cross_zx = z_axis.cross(&x_axis);
        if (cross_zx - y_axis).magnitude() > EPSILON {
            return Err(format!("Right-hand rule violation: Z × X = {:?}, expected Y = {:?}", cross_zx, y_axis));
        }
        
        Ok(())
    }
    
    /// Validate Johannes Unterguggenberger's specific coordinate parameters
    pub fn validate_johannes_coordinate_parameters() -> Result<(), String> {
        // Test the exact parameters mentioned in the guide and our lighting system
        let test_direction = Vec3::new(-0.7, -1.0, 0.3);
        
        // 1. Verify normalization preserves coordinate relationships
        let normalized = test_direction.normalize();
        let original_magnitude = test_direction.magnitude();
        
        if (normalized.magnitude() - 1.0).abs() > EPSILON {
            return Err(format!("Normalization failed: magnitude = {}", normalized.magnitude()));
        }
        
        // 2. Test that coordinate ratios are preserved
        let ratio_xy = normalized.x / normalized.y;
        let expected_ratio_xy = test_direction.x / test_direction.y;
        
        if (ratio_xy - expected_ratio_xy).abs() > EPSILON {
            return Err(format!("Coordinate ratio not preserved: {} vs {}", ratio_xy, expected_ratio_xy));
        }
        
        // 3. Validate matrix transformations preserve these coordinates
        let identity_matrix = Mat4::identity();
        let point = Point3::new(test_direction.x, test_direction.y, test_direction.z);
        let transformed = identity_matrix.transform_point(&point);
        
        if (transformed.coords - test_direction).magnitude() > EPSILON {
            return Err(format!("Identity transformation failed to preserve coordinates"));
        }
        
        Ok(())
    }
    
    /// Validate perspective projection matrix according to Johannes guide
    pub fn validate_perspective_projection() -> Result<(), String> {
        // Test parameters from Johannes Unterguggenberger's guide
        let fov_y = std::f32::consts::PI / 4.0; // 45 degrees
        let aspect = 16.0 / 9.0;                // Widescreen aspect
        let near = 0.1;
        let far = 100.0;
        
        let projection = Mat4::perspective(fov_y, aspect, near, far);
        
        // Validate matrix structure according to the guide
        // P = [a⁻¹/tan(φ/2)    0              0                    0           ]
        //     [0               1/tan(φ/2)     0                    0           ]
        //     [0               0              f/(f-n)              -nf/(f-n)   ]
        //     [0               0              1                    0           ]
        
        let tan_half_fovy = (fov_y * 0.5).tan();
        let expected_00 = 1.0 / (aspect * tan_half_fovy);
        let expected_11 = 1.0 / tan_half_fovy;
        let expected_22 = far / (far - near);
        let expected_23 = -(near * far) / (far - near);
        
        // Check key matrix elements
        if (projection[(0, 0)] - expected_00).abs() > EPSILON {
            return Err(format!("Projection matrix [0,0] incorrect: {} vs {}", projection[(0, 0)], expected_00));
        }
        
        if (projection[(1, 1)] - expected_11).abs() > EPSILON {
            return Err(format!("Projection matrix [1,1] incorrect: {} vs {}", projection[(1, 1)], expected_11));
        }
        
        if (projection[(2, 2)] - expected_22).abs() > EPSILON {
            return Err(format!("Projection matrix [2,2] incorrect: {} vs {}", projection[(2, 2)], expected_22));
        }
        
        if (projection[(2, 3)] - expected_23).abs() > EPSILON {
            return Err(format!("Projection matrix [2,3] incorrect: {} vs {}", projection[(2, 3)], expected_23));
        }
        
        Ok(())
    }
    
    /// Validate Vulkan coordinate transformation matrix
    pub fn validate_vulkan_coordinate_transform() -> Result<(), String> {
        let transform = Mat4::vulkan_coordinate_transform();
        
        // According to Johannes guide, this should be:
        // X = [1   0   0  0]⁻¹
        //     [0  -1   0  0]
        //     [0   0  -1  0] 
        //     [0   0   0  1]
        
        let expected = Mat4::new(
            1.0,  0.0,  0.0, 0.0,
            0.0, -1.0,  0.0, 0.0,
            0.0,  0.0, -1.0, 0.0,
            0.0,  0.0,  0.0, 1.0,
        );
        
        // Compare matrices element by element
        for i in 0..4 {
            for j in 0..4 {
                if (transform[(i, j)] - expected[(i, j)]).abs() > EPSILON {
                    return Err(format!("Vulkan transform matrix [{},{}] incorrect: {} vs {}", 
                        i, j, transform[(i, j)], expected[(i, j)]));
                }
            }
        }
        
        Ok(())
    }
    
    /// Validate look-at matrix construction
    pub fn validate_look_at_matrix() -> Result<(), String> {
        // Test various look-at configurations
        let test_cases = vec![
            // Case 1: Camera on +Z looking at origin
            (Vec3::new(0.0, 0.0, 5.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            // Case 2: Camera on +X looking at origin  
            (Vec3::new(5.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            // Case 3: Camera on +Y looking down at origin
            (Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -1.0)),
        ];
        
        for (i, (eye, target, up)) in test_cases.iter().enumerate() {
            let view_matrix = Mat4::look_at(*eye, *target, *up);
            
            // Test that forward direction is correct
            let forward = (target - eye).normalize();
            
            // Extract forward vector from view matrix (3rd column, negated)
            let matrix_forward = Vec3::new(-view_matrix[(0, 2)], -view_matrix[(1, 2)], -view_matrix[(2, 2)]);
            
            if (matrix_forward - forward).magnitude() > 1e-5 {
                return Err(format!("Look-at test case {} failed: forward mismatch", i));
            }
        }
        
        Ok(())
    }
    
    /// Validate quaternion rotation consistency
    pub fn validate_quaternion_rotations() -> Result<(), String> {
        let test_rotations = vec![
            // 90-degree rotations around each axis
            (Vec3::x_axis(), std::f32::consts::PI / 2.0),
            (Vec3::y_axis(), std::f32::consts::PI / 2.0),
            (Vec3::z_axis(), std::f32::consts::PI / 2.0),
            // 180-degree rotations
            (Vec3::x_axis(), std::f32::consts::PI),
            (Vec3::y_axis(), std::f32::consts::PI),
            (Vec3::z_axis(), std::f32::consts::PI),
        ];
        
        for (axis, angle) in test_rotations {
            let quat = Quat::from_axis_angle(&axis, angle);
            
            // Verify quaternion is normalized
            if (quat.magnitude() - 1.0).abs() > EPSILON {
                return Err(format!("Quaternion not normalized: magnitude = {}", quat.magnitude()));
            }
            
            // Test rotation of identity vector
            let test_vector = Vec3::new(1.0, 0.0, 0.0);
            let rotated = quat * test_vector;
            
            // Verify magnitude is preserved
            if (rotated.magnitude() - test_vector.magnitude()).abs() > EPSILON {
                return Err(format!("Rotation changed vector magnitude"));
            }
        }
        
        Ok(())
    }
    
    /// Run all coordinate validation tests
    pub fn run_all_validations() -> Vec<Result<(), String>> {
        vec![
            Self::validate_right_handed_system(),
            Self::validate_johannes_coordinate_parameters(),
            Self::validate_perspective_projection(),
            Self::validate_vulkan_coordinate_transform(),
            Self::validate_look_at_matrix(),
            Self::validate_quaternion_rotations(),
            Self::validate_full_transformation_chain(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_coordinate_system_validation_suite() {
        let results = CoordinateSystemValidator::run_all_validations();
        
        for (i, result) in results.iter().enumerate() {
            match result {
                Ok(()) => println!("✓ Validation test {} passed", i + 1),
                Err(msg) => panic!("✗ Validation test {} failed: {}", i + 1, msg),
            }
        }
        
        println!("All coordinate system validation tests passed!");
    }
    
    #[test]
    fn test_specific_parameter_preservation() {
        // Test the exact (-0.7, -1.0, 0.3) parameters used in our system
        let direction = Vec3::new(-0.7, -1.0, 0.3);
        
        // Test through various transformation stages
        let normalized = direction.normalize();
        
        // Create directional light transform
        let transform = Transform::from_position_rotation(
            Vec3::zeros(),
            Quat::rotation_between(&Vec3::new(0.0, 0.0, -1.0), &normalized)
                .unwrap_or(Quat::identity())
        );
        
        // Transform default forward direction
        let result_direction = transform.rotation * Vec3::new(0.0, 0.0, -1.0);
        
        // Should match normalized input direction
        assert_relative_eq!(result_direction, normalized, epsilon = 1e-5);
        
        // Verify specific coordinate preservation
        assert!((result_direction.x - normalized.x).abs() < 1e-5);
        assert!((result_direction.y - normalized.y).abs() < 1e-5);
        assert!((result_direction.z - normalized.z).abs() < 1e-5);
    }
    
    #[test]
    fn test_camera_coordinate_consistency() {
        // Test camera coordinate transformations maintain consistency
        let camera = Camera::new(
            Vec3::new(0.0, 0.0, 5.0),   // position
            Vec3::new(0.0, 0.0, 0.0),   // target
            Vec3::new(0.0, 1.0, 0.0),   // up
            std::f32::consts::PI / 4.0, // fov
            16.0 / 9.0,                 // aspect
            0.1,                        // near
            100.0                       // far
        );
        
        let view_matrix = camera.get_view_matrix();
        let projection_matrix = camera.get_projection_matrix();
        let combined = camera.get_view_projection_matrix();
        
        // Verify manual combination matches camera's method
        let coord_transform = Mat4::vulkan_coordinate_transform();
        let manual_combined = projection_matrix * coord_transform * view_matrix;
        
        // Matrices should be identical (within floating-point precision)
        for i in 0..4 {
            for j in 0..4 {
                assert_relative_eq!(
                    combined[(i, j)], 
                    manual_combined[(i, j)], 
                    epsilon = 1e-6
                );
            }
        }
    }
    
    #[test]
    fn test_transform_matrix_decomposition() {
        // Test Transform ↔ Matrix conversions maintain coordinate integrity
        let original_position = Vec3::new(-0.7, -1.0, 0.3);
        let original_rotation = Quat::from_axis_angle(
            &Vec3::new(1.0, 1.0, 1.0).normalize(), 
            0.785 // 45 degrees
        );
        let original_scale = Vec3::new(2.0, 1.5, 0.8);
        
        let transform = Transform {
            position: original_position,
            rotation: original_rotation,
            scale: original_scale,
        };
        
        // Convert to matrix and back
        let matrix = transform.to_matrix();
        let reconstructed = Transform::from_matrix(matrix);
        
        // Position should be exact
        assert_relative_eq!(reconstructed.position, original_position, epsilon = 1e-6);
        
        // Scale should be exact
        assert_relative_eq!(reconstructed.scale, original_scale, epsilon = 1e-6);
        
        // Quaternion should represent same rotation (may be flipped in sign)
        let dot_product = original_rotation.coords.dot(&reconstructed.rotation.coords);
        assert!(dot_product.abs() > 0.999, "Quaternion rotation mismatch");
    }
}
