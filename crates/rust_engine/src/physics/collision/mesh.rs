//! Collision mesh representations
//!
//! Provides mesh-based collision shapes with model-space templates
//! and world-space transformations for testing.

use crate::foundation::math::Vec3;
use super::primitives::{Triangle, Ray, BoundingSphere};

/// A collision mesh template stored in MODEL SPACE (local coordinates)
/// GEA 13.3.4: "Collision shapes should be stored in model space and transformed on-the-fly"
#[derive(Debug, Clone)]
pub struct CollisionMeshTemplate {
    /// Triangles in MODEL SPACE (local coordinates, never modified)
    pub local_triangles: Vec<Triangle>,
    /// Local bounding sphere radius (model space)
    pub local_bounding_radius: f32,
}

impl CollisionMeshTemplate {
    /// Creates a new collision mesh template from MODEL SPACE vertices and indices
    /// Vertices should be in local/model coordinates relative to origin
    pub fn from_vertices(vertices: &[Vec3], indices: &[u32]) -> Self {
        let mut triangles = Vec::new();
        
        // Build triangles from indices (model space)
        for chunk in indices.chunks(3) {
            if chunk.len() == 3 {
                let v0 = vertices[chunk[0] as usize];
                let v1 = vertices[chunk[1] as usize];
                let v2 = vertices[chunk[2] as usize];
                triangles.push(Triangle::new(v0, v1, v2));
            }
        }
        
        // Calculate local bounding sphere radius from furthest vertex
        let mut max_distance_sq = 0.0f32;
        for tri in &triangles {
            for vertex in [tri.v0, tri.v1, tri.v2] {
                let dist_sq = vertex.magnitude_squared();
                max_distance_sq = max_distance_sq.max(dist_sq);
            }
        }
        
        let radius = max_distance_sq.sqrt();
        
        Self {
            local_triangles: triangles,
            local_bounding_radius: radius,
        }
    }
    
    /// Transform this template to world space using a transformation matrix
    pub fn to_world_space(&self, matrix: &crate::foundation::math::Mat4, center: Vec3, scale_factor: f32) -> WorldSpaceCollisionMesh {
        let transformed_triangles: Vec<Triangle> = self.local_triangles
            .iter()
            .map(|tri| {
                let v0 = matrix.transform_point(&nalgebra::Point3::new(tri.v0.x, tri.v0.y, tri.v0.z));
                let v1 = matrix.transform_point(&nalgebra::Point3::new(tri.v1.x, tri.v1.y, tri.v1.z));
                let v2 = matrix.transform_point(&nalgebra::Point3::new(tri.v2.x, tri.v2.y, tri.v2.z));
                Triangle::new(
                    Vec3::new(v0.x, v0.y, v0.z),
                    Vec3::new(v1.x, v1.y, v1.z),
                    Vec3::new(v2.x, v2.y, v2.z),
                )
            })
            .collect();
        
        WorldSpaceCollisionMesh {
            triangles: transformed_triangles,
            center,
            bounding_radius: self.local_bounding_radius * scale_factor,
        }
    }
}

/// World-space collision mesh (temporary, created on-demand for collision tests)
/// NOT stored permanently - recreated each collision test
#[derive(Debug)]
pub struct WorldSpaceCollisionMesh {
    /// Triangles in world space
    pub triangles: Vec<Triangle>,
    /// Center position in world space
    pub center: Vec3,
    /// Bounding sphere radius in world space
    pub bounding_radius: f32,
}

impl WorldSpaceCollisionMesh {
    /// Test ray intersection against all triangles in the mesh
    /// Returns closest hit (t, hit_point, normal) if any triangle is hit
    pub fn intersect_ray(&self, ray: &Ray) -> Option<(f32, Vec3, Vec3)> {
        // First check bounding sphere
        let bounding_sphere = BoundingSphere::new(self.center, self.bounding_radius);
        let sphere_hit = bounding_sphere.intersect_ray(ray);
        if sphere_hit.is_none() {
            return None;
        }
        
        // Test all triangles, find closest hit
        let mut closest_hit: Option<(f32, Vec3, Vec3)> = None;
        let mut closest_t = f32::MAX;
        
        for triangle in &self.triangles {
            if let Some((t, _u, _v)) = triangle.intersect_ray(ray) {
                if t < closest_t {
                    closest_t = t;
                    let hit_point = ray.point_at(t);
                    let normal = triangle.normal();
                    closest_hit = Some((t, hit_point, normal));
                }
            }
        }
        
        closest_hit
    }

    /// Test sphere intersection against the mesh
    /// Returns contact point and normal if collision detected
    pub fn intersect_sphere(&self, sphere: &BoundingSphere) -> Option<(Vec3, Vec3, f32)> {
        // First check bounding spheres
        let bounding_sphere = BoundingSphere::new(self.center, self.bounding_radius);
        if !bounding_sphere.intersects(sphere) {
            return None;
        }
        
        // Test each triangle
        for triangle in &self.triangles {
            // Check if sphere center is close to triangle plane
            let dist = triangle.distance_to_point(sphere.center);
            if dist.abs() > sphere.radius {
                continue; // Too far from plane
            }
            
            // Find closest point on triangle to sphere center
            let closest = triangle.closest_point(sphere.center);
            let dist_sq = (closest - sphere.center).magnitude_squared();
            
            if dist_sq <= sphere.radius * sphere.radius {
                // Collision! Calculate contact info
                let normal = triangle.normal();
                let penetration = sphere.radius - dist_sq.sqrt();
                return Some((closest, normal, penetration));
            }
        }
        
        None
    }

    /// Test mesh-mesh intersection
    pub fn intersects_mesh(&self, other: &WorldSpaceCollisionMesh) -> bool {
        // First check bounding spheres
        let sphere_a = BoundingSphere::new(self.center, self.bounding_radius);
        let sphere_b = BoundingSphere::new(other.center, other.bounding_radius);
        
        if !sphere_a.intersects(&sphere_b) {
            return false;
        }
        
        // Test triangle pairs
        for tri_a in &self.triangles {
            for tri_b in &other.triangles {
                if tri_a.intersects_triangle(tri_b) {
                    return true;
                }
            }
        }
        
        false
    }
}
