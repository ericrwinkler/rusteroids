//! Billboard orientation matrix calculations

use crate::foundation::math::{Vec3, Mat4};

/// Calculate transformation matrix for screen-aligned billboard
/// 
/// The billboard always faces the camera, perpendicular to the view direction.
/// This is the simplest billboard type, used for particles and generic effects.
pub fn calculate_screen_aligned_matrix(
    camera_right: Vec3,
    camera_up: Vec3,
    camera_forward: Vec3,
    billboard_position: Vec3,
    size: Vec3,
) -> Mat4 {
    // Billboard basis = camera basis (always perpendicular to camera)
    let right = camera_right * size.x;
    let up = camera_up * size.y;
    let forward = camera_forward * size.z;
    
    // Construct transformation matrix (nalgebra uses column-major storage)
    // Mat4::new takes 16 args: (col0_r0, col1_r0, col2_r0, col3_r0, col0_r1, col1_r1, ...)
    // Each column is: [basis_x, basis_y, basis_z, 0] and position column is [pos_x, pos_y, pos_z, 1]
    // This creates the matrix: [right | up | forward | position]
    Mat4::new(
        right.x, up.x, forward.x, billboard_position.x,
        right.y, up.y, forward.y, billboard_position.y,
        right.z, up.z, forward.z, billboard_position.z,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// Calculate transformation matrix for velocity-aligned billboard
/// 
/// The billboard's "right" axis is aligned with the velocity direction,
/// creating a ribbon/streak effect. The billboard faces the camera
/// while being stretched along the movement path.
pub fn calculate_velocity_aligned_matrix(
    camera_position: Vec3,
    billboard_position: Vec3,
    velocity_direction: Vec3,
    size: Vec3,
) -> Mat4 {
    // Normalize velocity direction - this becomes the "right" (length) axis of the trail
    let trail_direction = velocity_direction.normalize();
    
    // Calculate vector from billboard to camera
    let to_camera = (camera_position - billboard_position).normalize();
    
    // Calculate up vector perpendicular to both trail direction and camera direction
    // This makes the billboard face the camera
    let up = to_camera.cross(&trail_direction).normalize();
    
    // Recalculate forward to ensure orthogonal basis
    let forward = trail_direction.cross(&up).normalize();
    
    // Apply size scaling
    // size.x controls trail length, size.y controls width
    let right_scaled = trail_direction * size.x;
    let up_scaled = up * size.y;
    let forward_scaled = forward * size.z;
    
    // Construct transformation matrix (nalgebra uses column-major storage)
    // Mat4::new takes 16 args: (col0_r0, col1_r0, col2_r0, col3_r0, col0_r1, col1_r1, ...)
    // Each column is: [basis_x, basis_y, basis_z, 0] and position column is [pos_x, pos_y, pos_z, 1]
    // This creates the matrix: [right | up | forward | position]
    Mat4::new(
        right_scaled.x, up_scaled.x, forward_scaled.x, billboard_position.x,
        right_scaled.y, up_scaled.y, forward_scaled.y, billboard_position.y,
        right_scaled.z, up_scaled.z, forward_scaled.z, billboard_position.z,
        0.0, 0.0, 0.0, 1.0,
    )
}

/// Calculate transformation matrix for world-axis-aligned billboard
/// 
/// The billboard's up axis is locked to a world axis (typically Y-up),
/// but it rotates around that axis to face the camera. Good for
/// vertical elements like trees, lamp posts, or columns.
pub fn calculate_world_axis_aligned_matrix(
    camera_position: Vec3,
    billboard_position: Vec3,
    world_up_axis: Vec3,
    size: Vec3,
) -> Mat4 {
    // Normalize world up axis
    let up = world_up_axis.normalize();
    
    // Calculate vector from billboard to camera (in world space)
    let to_camera = camera_position - billboard_position;
    
    // Project to_camera onto plane perpendicular to up axis
    let to_camera_projected = to_camera - up * up.dot(&to_camera);
    let forward = to_camera_projected.normalize();
    
    // Calculate right vector
    let right = up.cross(&forward).normalize();
    
    // Apply size scaling
    let right_scaled = right * size.x;
    let up_scaled = up * size.y;
    let forward_scaled = forward * size.z;
    
    // Construct transformation matrix (nalgebra uses column-major storage)
    // Mat4::new takes 16 args: (col0_r0, col1_r0, col2_r0, col3_r0, col0_r1, col1_r1, ...)
    // This creates the matrix: [right | up | forward | position]
    Mat4::new(
        right_scaled.x, right_scaled.y, right_scaled.z, 0.0,
        up_scaled.x, up_scaled.y, up_scaled.z, 0.0,
        forward_scaled.x, forward_scaled.y, forward_scaled.z, 0.0,
        billboard_position.x, billboard_position.y, billboard_position.z, 1.0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_screen_aligned_perpendicular_to_camera() {
        let camera_pos = Vec3::new(0.0, 0.0, 5.0);
        let camera_rot = Quat::identity();
        let billboard_pos = Vec3::new(0.0, 0.0, 0.0);
        let size = Vec3::new(1.0, 1.0, 1.0);
        
        let matrix = calculate_screen_aligned_matrix(camera_pos, camera_rot, billboard_pos, size);
        
        // Extract forward direction from matrix (3rd column)
        let forward = Vec3::new(matrix.x_axis.z, matrix.y_axis.z, matrix.z_axis.z);
        let expected_forward = Vec3::new(0.0, 0.0, -1.0);
        
        assert!((forward - expected_forward).length() < 0.001);
    }
    
    #[test]
    fn test_velocity_aligned_maintains_direction() {
        let camera_pos = Vec3::new(5.0, 0.0, 0.0);
        let billboard_pos = Vec3::new(0.0, 0.0, 0.0);
        let velocity_dir = Vec3::new(0.0, 0.0, 1.0); // Moving forward
        let size = Vec3::new(1.0, 1.0, 1.0);
        
        let matrix = calculate_velocity_aligned_matrix(camera_pos, billboard_pos, velocity_dir, size);
        
        // Extract forward direction from matrix (3rd column)
        let forward = Vec3::new(matrix.x_axis.z, matrix.y_axis.z, matrix.z_axis.z);
        
        // Should maintain velocity direction
        assert!((forward - velocity_dir).length() < 0.001);
    }
}
