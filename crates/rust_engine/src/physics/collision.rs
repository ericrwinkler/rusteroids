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
        distance_squared < radius_sum * radius_sum
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

/// Collision shape types
#[derive(Debug, Clone)]
pub enum CollisionShape {
    /// A spherical collision shape
    Sphere(BoundingSphere),
    // Can add more shapes later: Box, Capsule, etc.
}

impl CollisionShape {
    /// Creates a spherical collision shape
    pub fn sphere(center: Vec3, radius: f32) -> Self {
        Self::Sphere(BoundingSphere::new(center, radius))
    }

    /// Returns the center position of the collision shape
    pub fn center(&self) -> Vec3 {
        match self {
            Self::Sphere(sphere) => sphere.center,
        }
    }

    /// Sets the center position of the collision shape
    pub fn set_center(&mut self, center: Vec3) {
        match self {
            Self::Sphere(sphere) => sphere.center = center,
        }
    }

    /// Test ray intersection with this collision shape
    /// Returns distance to hit if intersected, None otherwise
    pub fn intersect_ray(&self, ray: &Ray) -> Option<f32> {
        match self {
            Self::Sphere(sphere) => sphere.intersect_ray(ray).map(|(distance, _, _)| distance),
        }
    }

    /// Test ray intersection with this collision shape (detailed)
    /// Returns (distance, hit_point, normal) if hit, None otherwise
    pub fn intersect_ray_detailed(&self, ray: &Ray) -> Option<(f32, Vec3, Vec3)> {
        match self {
            Self::Sphere(sphere) => sphere.intersect_ray(ray),
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
                    // Narrow-phase collision detection
                    let (CollisionShape::Sphere(sphere_a), CollisionShape::Sphere(sphere_b)) = (shape_a, shape_b);
                    if sphere_a.intersects(sphere_b) {
                        let penetration = sphere_a.penetration_depth(sphere_b);
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
        _octree: &Octree,
        shapes: &[(Entity, CollisionShape)],
    ) -> Vec<RayHit> {
        let mut hits = Vec::new();

        // Get all entities along the ray path (broad phase)
        // For simplicity, we check all entities in the octree
        // A more optimized approach would traverse octree nodes along the ray
        let all_entities: Vec<Entity> = shapes.iter().map(|(e, _)| *e).collect();

        for entity in all_entities {
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
}

impl Default for CollisionSystem {
    fn default() -> Self {
        Self::new()
    }
}
