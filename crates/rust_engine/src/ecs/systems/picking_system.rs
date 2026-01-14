//! High-level picking system for mouse-based entity selection
//!
//! Orchestrates the picking pipeline: input → camera → physics → ECS

use crate::ecs::{World, Entity};
use crate::input::picking::MouseState;
use crate::render::Camera;
use crate::physics::CollisionShape;
use crate::physics::collision::WorldSpaceShape;
use crate::spatial::Octree;
use crate::ecs::components::{PickableComponent, SelectionComponent, TransformComponent};
use crate::foundation::math::Vec3;
use std::collections::HashSet;

/// A plane defined by a normal and distance from origin
#[derive(Debug, Clone, Copy)]
struct Plane {
    normal: Vec3,
    distance: f32,
}

impl Plane {
    fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal: normal.normalize(), distance }
    }
    
    fn from_point_normal(point: Vec3, normal: Vec3) -> Self {
        let normal = normal.normalize();
        let distance = -normal.dot(&point);
        Self { normal, distance }
    }
    
    /// Test if a point is on the positive side of the plane (inside frustum)
    fn is_point_inside(&self, point: Vec3) -> bool {
        self.distance_to_point(point) >= 0.0
    }
    
    /// Get signed distance from point to plane (positive = in front)
    fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(&point) + self.distance
    }
}

/// View frustum for culling
#[derive(Debug, Clone)]
struct Frustum {
    planes: [Plane; 6], // left, right, top, bottom, near, far
}

impl Frustum {
    /// Test if a sphere intersects the frustum
    fn intersects_sphere(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false; // Sphere is completely outside this plane
            }
        }
        true // Sphere intersects or is inside frustum
    }
    
    /// Test if a point is inside the frustum
    fn contains_point(&self, point: Vec3) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(point) < 0.0 {
                return false; // Point is outside this plane
            }
        }
        true // Point is inside frustum
    }
}

/// Selection mode for picking system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Replace current selection (default)
    Replace,
    /// Add to current selection (Ctrl)
    Add,
    /// Remove from current selection (Shift)
    Remove,
}

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
    
    /// Currently selected entities
    selected_entities: HashSet<Entity>,
    
    /// Currently hovered entity (if any)
    hovered_entity: Option<Entity>,
    
    /// Layer mask for filtering pickable entities
    layer_mask: u32,
    
    /// Whether to perform hover detection (can be disabled for performance)
    hover_enabled: bool,
    
    /// Use simple sphere collision for picking (much faster than triangle meshes)
    use_simple_picking: bool,
    
    /// Enable box selection (drag to select multiple)
    box_selection_enabled: bool,
    
    /// Modifier key states for selection modes
    ctrl_pressed: bool,
    shift_pressed: bool,
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
            selected_entities: HashSet::new(),
            hovered_entity: None,
            layer_mask: 0xFFFFFFFF, // All layers by default
            hover_enabled: true,
            use_simple_picking: true, // Use sphere collision by default (faster)
            box_selection_enabled: true,
            ctrl_pressed: false,
            shift_pressed: false,
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

    /// Handle mouse button press event
    pub fn on_mouse_press(&mut self, screen_x: f64, screen_y: f64) {
        self.mouse_state.update_position(screen_x, screen_y);
        self.mouse_state.start_drag();
    }

    /// Handle mouse button release event  
    pub fn on_mouse_release(&mut self) {
        self.mouse_state.end_drag();
    }

    /// Handle mouse move event
    pub fn on_mouse_move(&mut self, screen_x: f64, screen_y: f64) {
        self.mouse_state.update_position(screen_x, screen_y);
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

    /// Enable or disable box selection
    pub fn set_box_selection_enabled(&mut self, enabled: bool) {
        self.box_selection_enabled = enabled;
    }

    /// Update modifier key states (Ctrl for add, Shift for remove)
    pub fn update_modifiers(&mut self, ctrl: bool, shift: bool) {
        self.ctrl_pressed = ctrl;
        self.shift_pressed = shift;
    }

    /// Get current selection mode based on modifier keys
    fn get_selection_mode(&self) -> SelectionMode {
        if self.ctrl_pressed {
            SelectionMode::Add
        } else if self.shift_pressed {
            SelectionMode::Remove
        } else {
            SelectionMode::Replace
        }
    }

    /// Perform picking operation
    ///
    /// Call this every frame (or just when needed for click-only picking).
    ///
    /// # Arguments
    /// * `world` - The ECS world
    /// * `camera` - The active camera
    /// * `octree` - Spatial partitioning structure for broad-phase
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
        // For now, only handle single-click selection
        // Box selection will be handled when mouse is released after dragging
        
        // Handle click (selection) - ONLY ray cast on click
        if self.mouse_state.left_click {
            let (ndc_x, ndc_y) = self.mouse_state.screen_to_ndc();
            
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
                
                // Apply selection mode based on modifiers
                let mode = self.get_selection_mode();
                match mode {
                    SelectionMode::Replace => {
                        self.select_entity(world, hit.entity, current_frame);
                    }
                    SelectionMode::Add => {
                        self.add_to_selection(world, hit.entity, current_frame);
                    }
                    SelectionMode::Remove => {
                        self.remove_from_selection(world, hit.entity, current_frame);
                    }
                }
            } else {
                println!("PickingSystem: ✗ No hit detected");
                // Only deselect if in replace mode
                if self.get_selection_mode() == SelectionMode::Replace {
                    self.deselect_all(world, current_frame);
                }
            }
            
            // Clear click state after processing
            self.mouse_state.clear_clicks();
        }
    }

    /// Get the current mouse position in screen space
    pub fn get_mouse_position(&self) -> (f64, f64) {
        (self.mouse_state.screen_x, self.mouse_state.screen_y)
    }

    /// Get currently selected entity (returns first selected for backward compatibility)
    pub fn get_selected(&self) -> Option<Entity> {
        self.selected_entities.iter().next().copied()
    }

    /// Get all currently selected entities
    pub fn get_all_selected(&self) -> &HashSet<Entity> {
        &self.selected_entities
    }

    /// Check if currently dragging (box selection)
    pub fn is_dragging(&self) -> bool {
        self.box_selection_enabled && self.mouse_state.is_dragging()
    }

    /// Get selection box in screen space for rendering
    pub fn get_selection_box(&self) -> Option<((f64, f64), (f64, f64))> {
        if self.box_selection_enabled {
            self.mouse_state.get_drag_box()
        } else {
            None
        }
    }

    /// Get currently hovered entity
    pub fn get_hovered(&self) -> Option<Entity> {
        self.hovered_entity
    }

    /// Manually select an entity (replaces current selection)
    pub fn select_entity(&mut self, world: &mut World, entity: Entity, current_frame: u64) {
        // Deselect all previous
        for prev_entity in self.selected_entities.iter() {
            self.deselect_entity_internal(world, *prev_entity, current_frame);
        }
        self.selected_entities.clear();

        // Select new
        self.select_entity_internal(world, entity, current_frame);
        self.selected_entities.insert(entity);
    }

    /// Add entity to selection
    pub fn add_to_selection(&mut self, world: &mut World, entity: Entity, current_frame: u64) {
        if self.selected_entities.insert(entity) {
            self.select_entity_internal(world, entity, current_frame);
        }
    }

    /// Remove entity from selection
    pub fn remove_from_selection(&mut self, world: &mut World, entity: Entity, current_frame: u64) {
        if self.selected_entities.remove(&entity) {
            self.deselect_entity_internal(world, entity, current_frame);
        }
    }

    /// Toggle entity selection
    pub fn toggle_selection(&mut self, world: &mut World, entity: Entity, current_frame: u64) {
        if self.selected_entities.contains(&entity) {
            self.remove_from_selection(world, entity, current_frame);
        } else {
            self.add_to_selection(world, entity, current_frame);
        }
    }

    /// Manually deselect current selection
    pub fn deselect_all(&mut self, world: &mut World, current_frame: u64) {
        self.deselect_all_internal(world, current_frame);
        self.selected_entities.clear();
    }

    /// Internal helper to deselect all entities
    fn deselect_all_internal(&self, world: &mut World, current_frame: u64) {
        for entity in &self.selected_entities {
            self.deselect_entity_internal(world, *entity, current_frame);
        }
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
    
    /// Build a view frustum from a 2D selection box
    /// Returns None if box is too small (likely a click, not a drag)
    fn build_frustum_from_box(
        &self,
        camera: &Camera,
        box_min: (f64, f64),
        box_max: (f64, f64),
    ) -> Option<Frustum> {
        // Convert screen corners to NDC
        let to_ndc = |x: f64, y: f64| {
            let ndc_x = (x / self.mouse_state.window_width as f64) as f32 * 2.0 - 1.0;
            let ndc_y = (y / self.mouse_state.window_height as f64) as f32 * 2.0 - 1.0;
            (ndc_x, ndc_y)
        };
        
        // Get 4 corner rays
        let (min_x_ndc, min_y_ndc) = to_ndc(box_min.0, box_min.1);
        let (max_x_ndc, max_y_ndc) = to_ndc(box_max.0, box_max.1);
        
        // Generate rays from camera through box corners
        let ray_tl = camera.screen_to_world_ray(min_x_ndc, min_y_ndc); // top-left
        let ray_tr = camera.screen_to_world_ray(max_x_ndc, min_y_ndc); // top-right
        let ray_bl = camera.screen_to_world_ray(min_x_ndc, max_y_ndc); // bottom-left
        let ray_br = camera.screen_to_world_ray(max_x_ndc, max_y_ndc); // bottom-right
        
        let cam_pos = camera.position;
        
        // Build 4 side planes from adjacent rays
        // Normals must point INWARD (toward the frustum volume)
        // Cross product order determines normal direction
        let left_normal = ray_bl.direction.cross(&ray_tl.direction);   // Inward from left
        let right_normal = ray_tr.direction.cross(&ray_br.direction);  // Inward from right
        let top_normal = ray_tl.direction.cross(&ray_tr.direction);    // Inward from top
        let bottom_normal = ray_br.direction.cross(&ray_bl.direction); // Inward from bottom
        
        // Create planes
        let left_plane = Plane::from_point_normal(cam_pos, left_normal);
        let right_plane = Plane::from_point_normal(cam_pos, right_normal);
        let top_plane = Plane::from_point_normal(cam_pos, top_normal);
        let bottom_plane = Plane::from_point_normal(cam_pos, bottom_normal);
        
        // Near and far planes (use camera's near/far)
        let forward = (camera.target - camera.position).normalize();
        let near_plane = Plane::from_point_normal(cam_pos + forward * 0.1, forward);
        let far_plane = Plane::from_point_normal(cam_pos + forward * 1000.0, -forward);
        
        let frustum = Frustum {
            planes: [left_plane, right_plane, top_plane, bottom_plane, near_plane, far_plane],
        };
        
        // Debug: print frustum info
        println!("Frustum planes:");
        println!("  Camera pos: {:?}", cam_pos);
        println!("  Forward: {:?}", forward);
        println!("  Left normal: {:?}, dist: {:.2}", left_plane.normal, left_plane.distance);
        println!("  Right normal: {:?}, dist: {:.2}", right_plane.normal, right_plane.distance);
        
        Some(frustum)
    }
    
    /// Perform box selection using frustum culling (like RTS games)
    pub fn perform_box_selection(
        &mut self,
        world: &mut World,
        camera: &Camera,
        octree: &Octree,
        shape_provider: &dyn Fn(Entity) -> Option<CollisionShape>,
        current_frame: u64,
    ) {
        // Get drag box
        let Some(((min_x, min_y), (max_x, max_y))) = self.mouse_state.get_drag_box() else {
            return;
        };
        
        // Build frustum from box
        let Some(frustum) = self.build_frustum_from_box(camera, (min_x, min_y), (max_x, max_y)) else {
            return;
        };
        
        println!("PickingSystem: Box selection from ({:.0}, {:.0}) to ({:.0}, {:.0})", min_x, min_y, max_x, max_y);
        
        // Collect entities inside frustum
        let mut selected_in_box = HashSet::new();
        
        let mut total_checked = 0;
        let mut inside_frustum = 0;
        
        for entity in world.entities() {
            total_checked += 1;
            
            // Check if entity has required components
            if let (Some(pickable), Some(transform)) = (
                world.get_component::<PickableComponent>(*entity),
                world.get_component::<TransformComponent>(*entity),
            ) {
                // Skip if not enabled or doesn't match layer
                if !pickable.enabled || !pickable.matches_layer_mask(self.layer_mask) {
                    continue;
                }
                
                // If simple picking is disabled, use detailed collision shape testing
                let selected = if self.use_simple_picking {
                    // Fast path: Test bounding sphere against frustum
                    let radius = pickable.collision_radius.unwrap_or(1.0);
                    frustum.intersects_sphere(transform.position, radius)
                } else {
                    // Accurate path: Test actual collision shape against frustum
                    if let Some(shape) = shape_provider(*entity) {
                        let world_shape = shape.to_world_space(
                            transform.position,
                            transform.rotation,
                            transform.scale,
                        );
                        self.frustum_intersects_shape(&frustum, &world_shape)
                    } else {
                        // Fallback to sphere if no shape
                        let radius = pickable.collision_radius.unwrap_or(1.0);
                        frustum.intersects_sphere(transform.position, radius)
                    }
                };
                
                if selected {
                    inside_frustum += 1;
                    selected_in_box.insert(*entity);
                    
                    if inside_frustum <= 5 {
                        println!("  Selected entity at {:?}", transform.position);
                    }
                }
            }
        }
        
        println!("PickingSystem: Checked {} entities, found {} inside frustum", total_checked, inside_frustum);
        
        // Apply selection based on mode
        let mode = self.get_selection_mode();
        match mode {
            SelectionMode::Replace => {
                self.deselect_all_internal(world, current_frame);
                self.selected_entities.clear();
                for entity in selected_in_box {
                    self.select_entity_internal(world, entity, current_frame);
                    self.selected_entities.insert(entity);
                }
            }
            SelectionMode::Add => {
                for entity in selected_in_box {
                    if self.selected_entities.insert(entity) {
                        self.select_entity_internal(world, entity, current_frame);
                    }
                }
            }
            SelectionMode::Remove => {
                for entity in selected_in_box {
                    if self.selected_entities.remove(&entity) {
                        self.deselect_entity_internal(world, entity, current_frame);
                    }
                }
            }
        }
    }
    
    /// Test if a collision shape intersects with the view frustum
    /// Used for accurate box selection with detailed collision shapes
    fn frustum_intersects_shape(&self, frustum: &Frustum, world_shape: &WorldSpaceShape) -> bool {
        use crate::physics::collision::WorldSpaceShape;
        
        match world_shape {
            WorldSpaceShape::Sphere(sphere) => {
                // Test sphere against all frustum planes
                frustum.intersects_sphere(sphere.center, sphere.radius)
            }
            WorldSpaceShape::Mesh(mesh) => {
                // First check bounding sphere for early rejection
                if !frustum.intersects_sphere(mesh.center, mesh.bounding_radius) {
                    return false;
                }
                
                // For detailed test, check if any triangle vertex is inside the frustum
                // This is conservative - if any part of the mesh is inside, we select it
                for triangle in &mesh.triangles {
                    // Check all three vertices
                    if frustum.contains_point(triangle.v0) ||
                       frustum.contains_point(triangle.v1) ||
                       frustum.contains_point(triangle.v2) {
                        return true;
                    }
                }
                
                // Also check if triangle centroid is inside (catches small triangles)
                for triangle in &mesh.triangles {
                    if frustum.contains_point(triangle.centroid()) {
                        return true;
                    }
                }
                
                false
            }
        }
    }
}

impl Default for PickingSystem {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}
