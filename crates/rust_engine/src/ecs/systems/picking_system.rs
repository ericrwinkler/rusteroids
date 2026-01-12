//! High-level picking system for mouse-based entity selection
//!
//! Orchestrates the picking pipeline: input → camera → physics → ECS

use crate::ecs::{World, Entity};
use crate::input::picking::MouseState;
use crate::render::Camera;
use crate::physics::CollisionShape;
use crate::spatial::Octree;
use crate::ecs::components::{PickableComponent, SelectionComponent, TransformComponent};

/// High-level picking system that handles mouse-to-entity selection
///
/// This system bridges the input, rendering, physics, and ECS layers to
/// provide a complete picking solution.
///
/// # Usage
/// ```no_run
/// # use rust_engine::ecs::systems::PickingSystem;
/// # use rust_engine::ecs::World;
/// # use rust_engine::render::Camera;
/// # use rust_engine::physics::CollisionSystem;
/// # use rust_engine::spatial::Octree;
/// # use rust_engine::foundation::math::Vec3;
/// # use rust_engine::scene::AABB;
/// # let mut world = World::new();
/// # let camera = Camera::default();
/// # let collision_system = CollisionSystem::new();
/// # let octree = Octree::new(AABB::new(Vec3::zeros(), Vec3::new(100.0, 100.0, 100.0)), Default::default());
/// let mut picking_system = PickingSystem::new(1920, 1080);
///
/// // In your event loop:
/// picking_system.update_mouse(100.0, 200.0, false);
///
/// // In your update loop:
/// picking_system.update(&mut world, &camera, &octree, &collision_system, 0);
///
/// // Query selection:
/// if let Some(selected) = picking_system.get_selected() {
///     println!("Selected entity: {:?}", selected);
/// }
/// ```
pub struct PickingSystem {
    /// Mouse input state
    mouse_state: MouseState,
    
    /// Currently selected entity (if any)
    selected_entity: Option<Entity>,
    
    /// Currently hovered entity (if any)
    hovered_entity: Option<Entity>,
    
    /// Layer mask for filtering pickable entities
    layer_mask: u32,
    
    /// Whether to perform hover detection (can be disabled for performance)
    hover_enabled: bool,
    
    /// Use simple sphere collision for picking (much faster than triangle meshes)
    use_simple_picking: bool,
}

impl PickingSystem {
    /// Create a new picking system
    ///
    /// # Arguments
    /// * `window_width` - Initial window width in pixels
    /// * `window_height` - Initial window height in pixels
    pub fn new(window_width: u32, window_height: u32) -> Self {
        Self {
            mouse_state: MouseState::new(window_width, window_height),
            selected_entity: None,
            hovered_entity: None,
            layer_mask: 0xFFFFFFFF, // All layers by default
            hover_enabled: true,
            use_simple_picking: true, // Use sphere collision by default (faster)
        }
    }

    /// Update mouse state from window events
    ///
    /// Call this in your event handling code.
    ///
    /// # Arguments
    /// * `screen_x` - Mouse X position in screen space (pixels)
    /// * `screen_y` - Mouse Y position in screen space (pixels)
    /// * `clicked` - Whether left mouse button was clicked this frame
    pub fn update_mouse(&mut self, screen_x: f64, screen_y: f64, clicked: bool) {
        self.mouse_state.update_position(screen_x, screen_y);
        self.mouse_state.set_left_click(clicked);
    }

    /// Update window size for NDC conversion
    ///
    /// Call this when the window is resized.
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.mouse_state.update_window_size(width, height);
    }

    /// Set the layer mask for filtering pickable entities
    ///
    /// Only entities with matching layer bits will be considered for picking.
    pub fn set_layer_mask(&mut self, mask: u32) {
        self.layer_mask = mask;
    }

    /// Enable or disable hover detection
    pub fn set_hover_enabled(&mut self, enabled: bool) {
        self.hover_enabled = enabled;
    }
    
    /// Enable or disable simple picking (sphere-only collision, much faster)
    /// 
    /// When enabled, uses bounding spheres for picking instead of accurate triangle meshes.
    /// This is much faster but less precise.
    pub fn set_simple_picking(&mut self, enabled: bool) {
        self.use_simple_picking = enabled;
    }

    /// Perform picking operation
    ///
    /// Call this every frame (or just when needed for click-only picking).
    ///
    /// # Arguments
    /// * `world` - The ECS world
    /// * `camera` - The active camera
    /// * `octree` - Spatial partitioning structure for broad-phase
    /// * `collision_system` - Physics collision system
    /// * `shape_provider` - Function to get collision shape for an entity
    /// * `current_frame` - Current frame number (for selection tracking)
    pub fn update(
        &mut self,
        world: &mut World,
        camera: &Camera,
        octree: &Octree,
        shape_provider: &dyn Fn(Entity) -> Option<CollisionShape>,
        current_frame: u64,
    ) {
        // 1. Convert mouse to NDC
        let (ndc_x, ndc_y) = self.mouse_state.screen_to_ndc();

        // 2. Handle click (selection) - ONLY ray cast on click
        if self.mouse_state.left_click {
            // Generate world-space ray from camera
            let ray = camera.screen_to_world_ray(ndc_x, ndc_y);

            // Use octree to filter entities along the ray path (broad phase)
            let candidates = self.collect_ray_candidates(world, octree, &ray, shape_provider);

            // Ray cast against filtered candidates only (narrow phase)
            let hit = self.ray_cast_shapes(world, &ray, &candidates);
            
            println!("PickingSystem: Click at ({:.2}, {:.2})", 
                     self.mouse_state.screen_x, self.mouse_state.screen_y);
            
            if let Some(hit) = &hit {
                println!("PickingSystem: ✓ Hit entity {:?} at distance {:.2}", hit.entity, hit.distance);
                self.select_entity(world, hit.entity, current_frame);
            } else {
                println!("PickingSystem: ✗ No hit detected, deselecting all");
                self.deselect_all(world, current_frame);
            }
            
            // Clear click state after processing
            self.mouse_state.clear_clicks();
        }
    }

    /// Get the current mouse position in screen space
    pub fn get_mouse_position(&self) -> (f64, f64) {
        (self.mouse_state.screen_x, self.mouse_state.screen_y)
    }

    /// Get currently selected entity
    pub fn get_selected(&self) -> Option<Entity> {
        self.selected_entity
    }

    /// Get currently hovered entity
    pub fn get_hovered(&self) -> Option<Entity> {
        self.hovered_entity
    }

    /// Manually select an entity
    pub fn select_entity(&mut self, world: &mut World, entity: Entity, current_frame: u64) {
        // Deselect previous
        if let Some(prev) = self.selected_entity {
            self.deselect_entity_internal(world, prev, current_frame);
        }

        // Select new
        self.select_entity_internal(world, entity, current_frame);
        self.selected_entity = Some(entity);
    }

    /// Manually deselect current selection
    pub fn deselect_all(&mut self, world: &mut World, current_frame: u64) {
        if let Some(prev) = self.selected_entity {
            self.deselect_entity_internal(world, prev, current_frame);
        }
        self.selected_entity = None;
    }

    /// Helper: Collect entities along ray path using octree (broad phase)
    /// This dramatically reduces the number of expensive triangle tests
    fn collect_ray_candidates(
        &self,
        world: &World,
        _octree: &Octree,
        ray: &crate::physics::collision::Ray,
        shape_provider: &dyn Fn(Entity) -> Option<CollisionShape>,
    ) -> Vec<(Entity, CollisionShape)> {
        use crate::physics::collision::primitives::BoundingSphere;
        
        let mut candidates = Vec::new();

        // Quick sphere-ray intersection test for all entities
        // This is much cheaper than octree traversal and gives us good culling
        for entity in world.entities() {
            // Check if entity has required components
            if let (Some(pickable), Some(transform)) = (
                world.get_component::<PickableComponent>(*entity),
                world.get_component::<TransformComponent>(*entity),
            ) {
                // Skip if not enabled
                if !pickable.enabled {
                    continue;
                }

                // Skip if doesn't match layer mask
                if !pickable.matches_layer_mask(self.layer_mask) {
                    continue;
                }

                // Early-out: Test bounding sphere against ray (if radius is provided)
                if let Some(radius) = pickable.collision_radius {
                    let bounding_sphere = BoundingSphere::new(transform.position, radius);
                    if bounding_sphere.intersect_ray(ray).is_none() {
                        continue; // Ray doesn't hit this entity's bounding sphere
                    }
                }

                // Get collision shape from provider
                if let Some(shape) = shape_provider(*entity) {
                    // If simple picking is enabled, convert mesh shapes to spheres
                    if self.use_simple_picking {
                        match shape {
                            CollisionShape::Mesh(_) => {
                                // Use bounding sphere instead of mesh for picking (if available)
                                if let Some(radius) = pickable.collision_radius {
                                    let sphere_shape = CollisionShape::Sphere(radius);
                                    candidates.push((*entity, sphere_shape));
                                } else {
                                    // No radius provided, use the mesh shape
                                    candidates.push((*entity, shape));
                                }
                            }
                            CollisionShape::Sphere(_) => {
                                candidates.push((*entity, shape));
                            }
                        }
                    } else {
                        candidates.push((*entity, shape));
                    }
                }
            }
        }

        candidates
    }

    /// Helper: Ray cast against collected shapes, transforming from model-space to world-space
    fn ray_cast_shapes(
        &self,
        world: &World,
        ray: &crate::physics::collision::Ray,
        shapes: &[(Entity, CollisionShape)],
    ) -> Option<crate::physics::collision::RayHit> {
        use crate::physics::collision::RayHit;
        
        let mut closest_hit: Option<RayHit> = None;
        let mut closest_distance = f32::MAX;

        for (entity, model_shape) in shapes {
            // Get transform to convert model-space shape to world-space
            if let Some(transform) = world.get_component::<TransformComponent>(*entity) {
                // Transform shape to world-space
                let world_shape = model_shape.to_world_space(
                    transform.position,
                    transform.rotation,
                    transform.scale,
                );

                // Test ray intersection with world-space shape
                if let Some((distance, point, normal)) = world_shape.intersect_ray_detailed(ray) {
                    if distance < closest_distance {
                        closest_distance = distance;
                        closest_hit = Some(RayHit {
                            entity: *entity,
                            distance,
                            point,
                            normal,
                        });
                    }
                }
            }
        }

        closest_hit
    }

    /// Helper: Update hover state
    #[allow(dead_code)]
    fn update_hover_state(
        &mut self,
        world: &mut World,
        new_hovered: Option<Entity>,
        current_frame: u64,
    ) {
        if self.hovered_entity != new_hovered {
            // Clear old hover
            if let Some(old) = self.hovered_entity {
                if let Some(selection) = world.get_component_mut::<SelectionComponent>(old) {
                    selection.unhover(current_frame);
                }
            }

            // Set new hover
            if let Some(new) = new_hovered {
                // Add SelectionComponent if not present
                if world.get_component::<SelectionComponent>(new).is_none() {
                    world.add_component(new, SelectionComponent::default());
                }

                if let Some(selection) = world.get_component_mut::<SelectionComponent>(new) {
                    selection.hover(current_frame);
                }
            }

            self.hovered_entity = new_hovered;
        }
    }

    /// Helper: Mark entity as selected
    fn select_entity_internal(&self, world: &mut World, entity: Entity, current_frame: u64) {
        // Add SelectionComponent if not present
        if world.get_component::<SelectionComponent>(entity).is_none() {
            world.add_component(entity, SelectionComponent::default());
        }

        if let Some(selection) = world.get_component_mut::<SelectionComponent>(entity) {
            selection.select(current_frame);
        }
    }

    /// Helper: Mark entity as deselected
    fn deselect_entity_internal(&self, world: &mut World, entity: Entity, current_frame: u64) {
        if let Some(selection) = world.get_component_mut::<SelectionComponent>(entity) {
            selection.deselect(current_frame);
        }
    }
}

impl Default for PickingSystem {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}
