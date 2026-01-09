//! Primitive collision shapes and intersection algorithms
//!
//! Provides basic geometric primitives (rays, spheres, triangles) with
//! efficient intersection testing algorithms.

use crate::foundation::math::Vec3;
use crate::ecs::Entity;

/// A ray for ray casting and picking
#[derive(Debug, Clone, Copy)]
pub struct Ray {
    /// The origin point of the ray in world space
    pub origin: Vec3,
    /// The direction of the ray (should be normalized)
    pub direction: Vec3,
}

impl Ray {
    /// Creates a new ray with the given origin and direction
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        Self {
            origin,
            direction: direction.normalize(),
        }
    }

    /// Get a point along the ray at distance t
    pub fn point_at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

/// Result of a ray intersection test
#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    /// The entity that was hit
    pub entity: Entity,
    /// The distance from the ray origin to the hit point
    pub distance: f32,
    /// The point of intersection in world space
    pub point: Vec3,
    /// The surface normal at the intersection point
    pub normal: Vec3,
}

/// A bounding sphere for collision detection
#[derive(Debug, Clone, Copy)]
pub struct BoundingSphere {
    /// The center position of the sphere in world space
    pub center: Vec3,
    /// The radius of the sphere
    pub radius: f32,
}

impl BoundingSphere {
    /// Creates a new bounding sphere with the given center and radius
    pub fn new(center: Vec3, radius: f32) -> Self {
        Self { center, radius }
    }

    /// Check if this sphere intersects with another
    pub fn intersects(&self, other: &BoundingSphere) -> bool {
        let distance_squared = (self.center - other.center).magnitude_squared();
        let radius_sum = self.radius + other.radius;
        distance_squared <= radius_sum * radius_sum  // Changed < to <= for edge cases
    }

    /// Get the penetration depth if intersecting (0.0 if not intersecting)
    pub fn penetration_depth(&self, other: &BoundingSphere) -> f32 {
        let distance = (self.center - other.center).magnitude();
        let radius_sum = self.radius + other.radius;
        if distance < radius_sum {
            radius_sum - distance
        } else {
            0.0
        }
    }

    /// Test ray intersection with this sphere
    /// Returns (distance, hit_point, normal) if hit, None otherwise
    pub fn intersect_ray(&self, ray: &Ray) -> Option<(f32, Vec3, Vec3)> {
        // Vector from ray origin to sphere center
        let oc = ray.origin - self.center;
        
        // Quadratic formula coefficients for ray-sphere intersection
        // Solve: |origin + t*direction - center|^2 = radius^2
        let a = ray.direction.dot(&ray.direction); // Should be 1.0 if normalized
        let b = 2.0 * oc.dot(&ray.direction);
        let c = oc.dot(&oc) - self.radius * self.radius;
        
        let discriminant = b * b - 4.0 * a * c;
        
        if discriminant < 0.0 {
            return None; // No intersection
        }
        
        // Calculate both intersection points
        let sqrt_discriminant = discriminant.sqrt();
        let t1 = (-b - sqrt_discriminant) / (2.0 * a);
        let t2 = (-b + sqrt_discriminant) / (2.0 * a);
        
        // Use the closest positive intersection
        let t = if t1 > 0.0 {
            t1
        } else if t2 > 0.0 {
            t2
        } else {
            return None; // Ray pointing away from sphere
        };
        
        // Calculate hit point and normal
        let hit_point = ray.point_at(t);
        let normal = (hit_point - self.center).normalize();
        
        Some((t, hit_point, normal))
    }
}

/// A triangle for collision detection
#[derive(Debug, Clone, Copy)]
pub struct Triangle {
    /// Triangle vertices in world space
    pub v0: Vec3,
    /// Second vertex
    pub v1: Vec3,
    /// Third vertex
    pub v2: Vec3,
}

impl Triangle {
    /// Creates a new triangle
    pub fn new(v0: Vec3, v1: Vec3, v2: Vec3) -> Self {
        Self { v0, v1, v2 }
    }

    /// Calculates the normal of the triangle (right-hand rule)
    pub fn normal(&self) -> Vec3 {
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        edge1.cross(&edge2).normalize()
    }

    /// Calculates the centroid (center point) of the triangle
    pub fn centroid(&self) -> Vec3 {
        (self.v0 + self.v1 + self.v2) / 3.0
    }

    /// Möller-Trumbore ray-triangle intersection algorithm
    /// Returns (t, u, v) barycentric coordinates if hit, None otherwise
    /// 
    /// This is one of the fastest ray-triangle intersection algorithms.
    /// See: "Fast, Minimum Storage Ray/Triangle Intersection" by Möller & Trumbore
    pub fn intersect_ray(&self, ray: &Ray) -> Option<(f32, f32, f32)> {
        const EPSILON: f32 = 0.000001;  // Very small value for numerical stability
        
        // Calculate edges from v0
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        
        // Calculate determinant
        let h = ray.direction.cross(&edge2);
        let a = edge1.dot(&h);
        
        // Ray parallel to triangle?
        if a.abs() < EPSILON {
            return None;
        }
        
        let f = 1.0 / a;
        let s = ray.origin - self.v0;
        let u = f * s.dot(&h);
        
        // Hit outside triangle on u axis?
        if u < 0.0 || u > 1.0 {
            return None;
        }
        
        let q = s.cross(&edge1);
        let v = f * ray.direction.dot(&q);
        
        // Hit outside triangle on v axis?
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        
        // Calculate t (distance along ray)
        let t = f * edge2.dot(&q);
        
        // Accept any positive distance (including very small values)
        if t >= 0.0 {
            Some((t, u, v)) // Hit!
        } else {
            None // Behind ray origin
        }
    }

    /// Get the closest point on the triangle to a given point
    pub fn closest_point(&self, point: Vec3) -> Vec3 {
        // Project point onto triangle plane
        let edge1 = self.v1 - self.v0;
        let edge2 = self.v2 - self.v0;
        let v0_to_point = point - self.v0;
        
        let d1 = edge1.dot(&v0_to_point);
        let d2 = edge2.dot(&v0_to_point);
        
        // Check if point is in vertex region outside v0
        if d1 <= 0.0 && d2 <= 0.0 {
            return self.v0;
        }
        
        // Check if point is in vertex region outside v1
        let v1_to_point = point - self.v1;
        let d3 = edge1.dot(&v1_to_point);
        let d4 = edge2.dot(&v1_to_point);
        if d3 >= 0.0 && d4 <= d3 {
            return self.v1;
        }
        
        // Check if point is in vertex region outside v2
        let v2_to_point = point - self.v2;
        let d5 = edge1.dot(&v2_to_point);
        let d6 = edge2.dot(&v2_to_point);
        if d6 >= 0.0 && d5 <= d6 {
            return self.v2;
        }
        
        // Check if point is in edge region
        let vc = d1 * d4 - d3 * d2;
        if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
            let v_val = d1 / (d1 - d3);
            return self.v0 + edge1 * v_val;
        }
        
        let vb = d5 * d2 - d1 * d6;
        if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
            let w = d2 / (d2 - d6);
            return self.v0 + edge2 * w;
        }
        
        let va = d3 * d6 - d5 * d4;
        if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
            let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
            return self.v1 + (self.v2 - self.v1) * w;
        }
        
        // Point is inside triangle
        let denom = 1.0 / (va + vb + vc);
        let v_val = vb * denom;
        let w = vc * denom;
        self.v0 + edge1 * v_val + edge2 * w
    }

    /// Distance from a point to the triangle plane (signed)
    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        let normal = self.normal();
        let v0_to_point = point - self.v0;
        normal.dot(&v0_to_point)
    }
    
    /// Test if this triangle intersects another triangle
    /// Uses proper Separating Axis Theorem (SAT)
    /// Tests 11 potential separating axes:
    /// - 2 face normals (one per triangle)
    /// - 9 edge-edge cross products
    pub fn intersects_triangle(&self, other: &Triangle) -> bool {
        const EPSILON: f32 = 0.000001;
        
        // Helper to project a triangle onto an axis and get min/max
        fn project_triangle(tri: &Triangle, axis: Vec3) -> (f32, f32) {
            let p0 = axis.dot(&tri.v0);
            let p1 = axis.dot(&tri.v1);
            let p2 = axis.dot(&tri.v2);
            (p0.min(p1).min(p2), p0.max(p1).max(p2))
        }
        
        // Helper to check if two intervals overlap
        fn intervals_overlap(min1: f32, max1: f32, min2: f32, max2: f32) -> bool {
            max1 >= min2 && max2 >= min1
        }
        
        // Test axis (returns false if it's a separating axis)
        fn test_axis(tri1: &Triangle, tri2: &Triangle, axis: Vec3) -> bool {
            let axis_len_sq = axis.magnitude_squared();
            if axis_len_sq < EPSILON {
                return true; // Degenerate axis, skip
            }
            
            let normalized_axis = axis * (1.0 / axis_len_sq.sqrt());
            let (min1, max1) = project_triangle(tri1, normalized_axis);
            let (min2, max2) = project_triangle(tri2, normalized_axis);
            
            intervals_overlap(min1, max1, min2, max2)
        }
        
        // Get edges for both triangles
        let edges1 = [
            self.v1 - self.v0,
            self.v2 - self.v1,
            self.v0 - self.v2,
        ];
        
        let edges2 = [
            other.v1 - other.v0,
            other.v2 - other.v1,
            other.v0 - other.v2,
        ];
        
        // Test 1: Triangle 1's face normal
        let n1 = self.normal();
        if !test_axis(self, other, n1) {
            return false;
        }
        
        // Test 2: Triangle 2's face normal
        let n2 = other.normal();
        if !test_axis(self, other, n2) {
            return false;
        }
        
        // Test 3-11: All 9 edge-edge cross products
        for edge1 in &edges1 {
            for edge2 in &edges2 {
                let axis = edge1.cross(edge2);
                if !test_axis(self, other, axis) {
                    return false;
                }
            }
        }
        
        // No separating axis found = triangles intersect
        true
    }
}
