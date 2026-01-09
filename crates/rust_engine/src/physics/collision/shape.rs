//! High-level collision shape abstractions
//!
//! Provides ECS-friendly collision shapes that store data in model space
//! and transform to world space on-demand during collision tests.

use crate::foundation::math::Vec3;
use super::primitives::{BoundingSphere, Ray};
use super::mesh::{CollisionMeshTemplate, WorldSpaceCollisionMesh};

/// Collision shape types (stored in MODEL SPACE)
/// GEA 13.3.4: "Store collision shapes in model space, transform on-the-fly during tests"
#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// A spherical collision shape (radius only, position from TransformComponent)
    Sphere(f32),
    /// A triangle mesh collision shape template (model space, transformed on-demand)
    Mesh(CollisionMeshTemplate),
}

impl CollisionShape {
    /// Creates a spherical collision shape with given radius
    /// Position will come from TransformComponent at collision test time
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere(radius)
    }
    
    /// Legacy sphere creation (for backward compatibility)
    /// Ignores center parameter - use sphere(radius) instead
    #[allow(dead_code)]
    pub fn sphere_legacy(_center: Vec3, radius: f32) -> Self {
        Self::Sphere(radius)
    }

    /// Creates a mesh collision shape from MODEL SPACE vertices and indices
    /// Vertices should be in local coordinates (relative to origin)
    pub fn mesh_from_model(vertices: &[Vec3], indices: &[u32]) -> Self {
        Self::Mesh(CollisionMeshTemplate::from_vertices(vertices, indices))
    }
    
    /// Legacy mesh creation (for backward compatibility)
    /// Ignores center and scale parameters - vertices should already be in model space
    pub fn mesh(vertices: &[Vec3], indices: &[u32], _center: Vec3, _scale: f32) -> Self {
        // Vertices are assumed to already be in world space from old code
        // For now, just store them as-is (will need migration)
        Self::Mesh(CollisionMeshTemplate::from_vertices(vertices, indices))
    }

    /// Get the bounding radius in model space
    pub fn local_bounding_radius(&self) -> f32 {
        match self {
            Self::Sphere(radius) => *radius,
            Self::Mesh(template) => template.local_bounding_radius,
        }
    }
    
    /// Transform this shape to world space using position, rotation, and scale
    /// Returns a temporary WorldSpaceShape for collision testing
    pub fn to_world_space(
        &self,
        position: Vec3,
        rotation: crate::foundation::math::Quat,
        scale: Vec3,
    ) -> WorldSpaceShape {
        match self {
            Self::Sphere(radius) => {
                // Sphere radius is already in world-space units (not scaled by transform)
                WorldSpaceShape::Sphere(BoundingSphere::new(position, *radius))
            }
            Self::Mesh(template) => {
                // Build transformation matrix: TRS order
                use crate::foundation::math::Mat4;
                let translation = Mat4::new_translation(&position);
                let rotation_mat = rotation.to_homogeneous();
                let scale_mat = Mat4::new_nonuniform_scaling(&scale);
                let matrix = translation * rotation_mat * scale_mat;
                
                let scale_factor = scale.x.max(scale.y).max(scale.z);
                let world_mesh = template.to_world_space(&matrix, position, scale_factor);
                WorldSpaceShape::Mesh(world_mesh)
            }
        }
    }
}

/// World-space collision shape (temporary, for testing only)
#[derive(Debug)]
pub enum WorldSpaceShape {
    /// World-space sphere
    Sphere(BoundingSphere),
    /// World-space mesh
    Mesh(WorldSpaceCollisionMesh),
}

impl WorldSpaceShape {
    /// Get center position
    pub fn center(&self) -> Vec3 {
        match self {
            Self::Sphere(sphere) => sphere.center,
            Self::Mesh(mesh) => mesh.center,
        }
    }
    
    /// Get bounding sphere
    pub fn bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Sphere(sphere) => *sphere,
            Self::Mesh(mesh) => BoundingSphere::new(mesh.center, mesh.bounding_radius),
        }
    }

    /// Test ray intersection with this collision shape
    /// Returns distance to hit if intersected, None otherwise
    pub fn intersect_ray(&self, ray: &Ray) -> Option<f32> {
        match self {
            Self::Sphere(sphere) => sphere.intersect_ray(ray).map(|(distance, _, _)| distance),
            Self::Mesh(mesh) => mesh.intersect_ray(ray).map(|(distance, _, _)| distance),
        }
    }

    /// Test ray intersection with this collision shape (detailed)
    /// Returns (distance, hit_point, normal) if hit, None otherwise
    pub fn intersect_ray_detailed(&self, ray: &Ray) -> Option<(f32, Vec3, Vec3)> {
        match self {
            Self::Sphere(sphere) => sphere.intersect_ray(ray),
            Self::Mesh(mesh) => mesh.intersect_ray(ray),
        }
    }

    /// Test if this shape intersects with another shape
    pub fn intersects(&self, other: &WorldSpaceShape) -> bool {
        match (self, other) {
            // Sphere-Sphere intersection
            (Self::Sphere(a), Self::Sphere(b)) => a.intersects(b),
            
            // Sphere-Mesh intersection
            (Self::Sphere(sphere), Self::Mesh(mesh)) |
            (Self::Mesh(mesh), Self::Sphere(sphere)) => {
                mesh.intersect_sphere(sphere).is_some()
            }
            
            // Mesh-Mesh intersection
            (Self::Mesh(a), Self::Mesh(b)) => {
                a.intersects_mesh(b)
            }
        }
    }

    /// Get penetration depth for intersecting shapes (0.0 if not intersecting)
    pub fn penetration_depth(&self, other: &WorldSpaceShape) -> f32 {
        match (self, other) {
            (Self::Sphere(a), Self::Sphere(b)) => a.penetration_depth(b),
            
            (Self::Sphere(sphere), Self::Mesh(mesh)) |
            (Self::Mesh(mesh), Self::Sphere(sphere)) => {
                mesh.intersect_sphere(sphere)
                    .map(|(_, _, depth)| depth)
                    .unwrap_or(0.0)
            }
            
            (Self::Mesh(a), Self::Mesh(b)) => {
                // Use bounding sphere penetration as approximation
                let sphere_a = BoundingSphere::new(a.center, a.bounding_radius);
                let sphere_b = BoundingSphere::new(b.center, b.bounding_radius);
                sphere_a.penetration_depth(&sphere_b)
            }
        }
    }
}
