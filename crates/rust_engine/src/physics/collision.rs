//! Collision detection system using spatial partitioning
//!
//! Provides efficient broad-phase collision detection using octrees,
//! and narrow-phase detection for common shapes.

use crate::foundation::math::Vec3;
use crate::ecs::Entity;
use crate::spatial::Octree;
use std::collections::HashSet;

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
    pub v1: Vec3,
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

/// A collision mesh composed of triangles
#[derive(Debug, Clone)]
pub struct CollisionMesh {
    /// All triangles in the mesh (world space)
    pub triangles: Vec<Triangle>,
    /// Bounding sphere for broad-phase tests
    pub bounding_sphere: BoundingSphere,
    /// AABB for mid-phase tests (optional, can be calculated from sphere)
    pub center: Vec3,
}

impl CollisionMesh {
    /// Creates a new collision mesh from vertices and indices
    pub fn from_vertices(vertices: &[Vec3], indices: &[u32], center: Vec3, scale: f32) -> Self {
        let mut triangles = Vec::new();
        
        // Build triangles from indices
        for chunk in indices.chunks(3) {
            if chunk.len() == 3 {
                let v0 = center + vertices[chunk[0] as usize] * scale;
                let v1 = center + vertices[chunk[1] as usize] * scale;
                let v2 = center + vertices[chunk[2] as usize] * scale;
                triangles.push(Triangle::new(v0, v1, v2));
            }
        }
        
        // Calculate bounding sphere from furthest vertex
        let mut max_distance_sq = 0.0f32;
        for tri in &triangles {
            for vertex in [tri.v0, tri.v1, tri.v2] {
                let dist_sq = (vertex - center).magnitude_squared();
                max_distance_sq = max_distance_sq.max(dist_sq);
            }
        }
        
        let radius = max_distance_sq.sqrt();
        let bounding_sphere = BoundingSphere::new(center, radius);
        
        Self {
            triangles,
            bounding_sphere,
            center,
        }
    }

    /// Test ray intersection against all triangles in the mesh
    /// Returns closest hit (t, hit_point, normal) if any triangle is hit
    pub fn intersect_ray(&self, ray: &Ray) -> Option<(f32, Vec3, Vec3)> {
        // First check bounding sphere
        let sphere_hit = self.bounding_sphere.intersect_ray(ray);
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
        if !self.bounding_sphere.intersects(sphere) {
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

    /// Update all triangle positions (for moving meshes)
    pub fn update_position(&mut self, new_center: Vec3) {
        let delta = new_center - self.center;
        
        for triangle in &mut self.triangles {
            triangle.v0 = triangle.v0 + delta;
            triangle.v1 = triangle.v1 + delta;
            triangle.v2 = triangle.v2 + delta;
        }
        
        self.center = new_center;
        self.bounding_sphere.center = new_center;
    }
}

/// Collision shape types
#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// A spherical collision shape (fastest)
    Sphere(BoundingSphere),
    /// A triangle mesh collision shape (most accurate)
    Mesh(CollisionMesh),
}

impl CollisionShape {
    /// Creates a spherical collision shape
    pub fn sphere(center: Vec3, radius: f32) -> Self {
        Self::Sphere(BoundingSphere::new(center, radius))
    }

    /// Creates a mesh collision shape from vertices and indices
    pub fn mesh(vertices: &[Vec3], indices: &[u32], center: Vec3, scale: f32) -> Self {
        Self::Mesh(CollisionMesh::from_vertices(vertices, indices, center, scale))
    }

    /// Returns the center position of the collision shape
    pub fn center(&self) -> Vec3 {
        match self {
            Self::Sphere(sphere) => sphere.center,
            Self::Mesh(mesh) => mesh.center,
        }
    }

    /// Sets the center position of the collision shape
    pub fn set_center(&mut self, center: Vec3) {
        match self {
            Self::Sphere(sphere) => sphere.center = center,
            Self::Mesh(mesh) => mesh.update_position(center),
        }
    }

    /// Returns the bounding sphere of the collision shape
    pub fn bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Sphere(sphere) => *sphere,
            Self::Mesh(mesh) => mesh.bounding_sphere,
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
    pub fn intersects(&self, other: &CollisionShape) -> bool {
        match (self, other) {
            // Sphere-Sphere intersection (existing)
            (Self::Sphere(a), Self::Sphere(b)) => a.intersects(b),
            
            // Sphere-Mesh intersection
            (Self::Sphere(sphere), Self::Mesh(mesh)) |
            (Self::Mesh(mesh), Self::Sphere(sphere)) => {
                mesh.intersect_sphere(sphere).is_some()
            }
            
            // Mesh-Mesh intersection - proper triangle-triangle testing
            (Self::Mesh(a), Self::Mesh(b)) => {
                // First quick check: bounding spheres
                if !a.bounding_sphere.intersects(&b.bounding_sphere) {
                    return false;
                }
                
                // Detailed check: test each triangle pair
                for tri_a in &a.triangles {
                    for tri_b in &b.triangles {
                        if tri_a.intersects_triangle(tri_b) {
                            return true;
                        }
                    }
                }
                
                false
            }
        }
    }

    /// Get penetration depth for intersecting shapes (0.0 if not intersecting)
    pub fn penetration_depth(&self, other: &CollisionShape) -> f32 {
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
                a.bounding_sphere.penetration_depth(&b.bounding_sphere)
            }
        }
    }
}

/// Collision event between two entities
#[derive(Debug, Clone, Copy)]
pub struct CollisionEvent {
    /// The first entity involved in the collision
    pub entity_a: Entity,
    /// The second entity involved in the collision
    pub entity_b: Entity,
    /// How deep the collision penetrates (positive value)
    pub penetration_depth: f32,
}

/// Collision detection system
pub struct CollisionSystem {
    /// Track which entities are currently colliding (for persistent collision detection)
    active_collisions: HashSet<(Entity, Entity)>,
}

impl CollisionSystem {
    /// Creates a new collision detection system
    pub fn new() -> Self {
        Self {
            active_collisions: HashSet::new(),
        }
    }

    /// Detect collisions using an octree for broad-phase detection
    ///
    /// Returns a list of collision events for this frame
    pub fn detect_collisions(
        &mut self,
        octree: &Octree,
        shapes: &[(Entity, CollisionShape)],
    ) -> Vec<CollisionEvent> {
        let mut collisions = Vec::new();
        let mut current_frame_collisions = HashSet::new();

        // For each entity, query the octree for nearby entities
        for (entity_a, shape_a) in shapes {
            // Get all entities in the same octree nodes
            let nearby_entities = octree.query_nearby(*entity_a);

            // Check collision with each nearby entity
            for entity_b in nearby_entities {
                // Skip self-collision
                if *entity_a == entity_b {
                    continue;
                }

                // Skip if we already checked this pair (A-B is same as B-A)
                let pair = if entity_a.id() < entity_b.id() {
                    (*entity_a, entity_b)
                } else {
                    (entity_b, *entity_a)
                };

                if current_frame_collisions.contains(&pair) {
                    continue;
                }

                // Find shape_b in the shapes list
                if let Some((_, shape_b)) = shapes.iter().find(|(e, _)| *e == entity_b) {
                    // Narrow-phase collision detection using unified intersects method
                    if shape_a.intersects(shape_b) {
                        let penetration = shape_a.penetration_depth(shape_b);
                        collisions.push(CollisionEvent {
                            entity_a: *entity_a,
                            entity_b,
                            penetration_depth: penetration,
                        });
                        current_frame_collisions.insert(pair);
                    }
                }
            }
        }

        self.active_collisions = current_frame_collisions;
        collisions
    }

    /// Check if an entity is currently colliding with any other entity
    pub fn is_colliding(&self, entity: Entity) -> bool {
        self.active_collisions.iter().any(|(a, b)| *a == entity || *b == entity)
    }

    /// Get all entities that a specific entity is colliding with
    pub fn get_collisions_for(&self, entity: Entity) -> Vec<Entity> {
        self.active_collisions
            .iter()
            .filter_map(|(a, b)| {
                if *a == entity {
                    Some(*b)
                } else if *b == entity {
                    Some(*a)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Cast a ray through the scene and return all hits, sorted by distance
    ///
    /// Uses the octree for broad-phase filtering, then does precise ray-shape tests.
    /// Returns hits sorted from closest to farthest.
    pub fn ray_cast(
        &self,
        ray: &Ray,
        octree: &Octree,
        shapes: &[(Entity, CollisionShape)],
    ) -> Vec<RayHit> {
        let mut hits = Vec::new();

        // Use octree to get candidate entities along the ray path (broad phase)
        // Query octree along the ray direction
        let ray_candidates_octree = octree.query_ray(ray.origin, ray.direction);
        
        // Also query spheres around points along the ray for better coverage
        let mut candidates = std::collections::HashSet::new();
        for ent in ray_candidates_octree {
            candidates.insert(ent.id);
        }
        
        // Query at ray origin
        let origin_nearby = octree.query_radius(ray.origin, 50.0);
        for ent in origin_nearby {
            candidates.insert(ent.id);
        }
        
        // Query at points along the ray (up to reasonable distance)
        for i in 1..=10 {
            let t = i as f32 * 50.0; // Sample every 50 units
            let point = ray.origin + ray.direction * t;
            let nearby = octree.query_radius(point, 50.0);
            for ent in nearby {
                candidates.insert(ent.id);
            }
        }
        
        let ray_candidates: Vec<Entity> = candidates.into_iter().collect();
        
        println!("Ray cast: Octree filtered to {} candidates from {} total shapes", 
                 ray_candidates.len(), shapes.len());

        // Test each candidate shape (narrow phase)
        for entity in ray_candidates {
            // Find the shape for this entity
            if let Some((_, shape)) = shapes.iter().find(|(e, _)| *e == entity) {
                // Test ray intersection
                if let Some((distance, point, normal)) = shape.intersect_ray_detailed(ray) {
                    hits.push(RayHit {
                        entity,
                        distance,
                        point,
                        normal,
                    });
                }
            }
        }

        // Sort by distance (closest first)
        hits.sort_by(|a, b| a.distance.partial_cmp(&b.distance).unwrap_or(std::cmp::Ordering::Equal));
        hits
    }

    /// Cast a ray and return only the closest hit
    pub fn ray_cast_first(
        &self,
        ray: &Ray,
        octree: &Octree,
        shapes: &[(Entity, CollisionShape)],
    ) -> Option<RayHit> {
        self.ray_cast(ray, octree, shapes).into_iter().next()
    }

    /// Cast a ray with layer filtering
    ///
    /// Uses layer masks to efficiently filter entities before performing narrow-phase
    /// intersection tests. Inspired by Unity and Unreal's layer systems.
    ///
    /// # Arguments
    /// * `ray` - The ray to cast
    /// * `octree` - Spatial partitioning structure for broad-phase
    /// * `shapes` - All collision shapes with entities
    /// * `layer_mask` - Bit mask for filtering (e.g., 0b0001 for world layer)
    /// * `layer_bits_fn` - Function to get layer bits for an entity
    ///
    /// # Returns
    /// The closest hit matching the layer mask, or None
    ///
    /// # Examples
    /// ```no_run
    /// # use rust_engine::physics::{CollisionSystem, Ray};
    /// # use rust_engine::foundation::math::Vec3;
    /// # use rust_engine::spatial::Octree;
    /// # use rust_engine::scene::AABB;
    /// # let collision_system = CollisionSystem::new();
    /// # let ray = Ray::new(Vec3::zeros(), Vec3::new(0.0, 0.0, 1.0));
    /// # let octree = Octree::new(AABB::new(Vec3::zeros(), Vec3::new(100.0, 100.0, 100.0)), Default::default());
    /// # let shapes = vec![];
    /// const WORLD_LAYER: u32 = 0b0001;
    /// const UI_LAYER: u32 = 0b0010;
    ///
    /// // Only pick world objects, ignore UI
    /// let hit = collision_system.ray_cast_filtered(
    ///     &ray,
    ///     &octree,
    ///     &shapes,
    ///     WORLD_LAYER,
    ///     |_entity| Some(WORLD_LAYER),
    /// );
    /// ```
    pub fn ray_cast_filtered<F>(
        &self,
        ray: &Ray,
        octree: &Octree,
        shapes: &[(Entity, CollisionShape)],
        layer_mask: u32,
        layer_bits_fn: F,
    ) -> Option<RayHit>
    where
        F: Fn(Entity) -> Option<u32>,
    {
        // Filter shapes by layer mask before testing
        let filtered_shapes: Vec<_> = shapes
            .iter()
            .filter(|(entity, _)| {
                if let Some(entity_layers) = layer_bits_fn(*entity) {
                    (layer_mask & entity_layers) != 0
                } else {
                    true // No layers = matches all
                }
            })
            .cloned()
            .collect();

        // Perform ray cast on filtered shapes
        self.ray_cast_first(ray, octree, &filtered_shapes)
    }
}

impl Default for CollisionSystem {
    fn default() -> Self {
        Self::new()
    }
}
