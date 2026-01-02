//! Octree Visualization Demo
//!
//! Visualizes a 3D octree spatial partitioning structure with:
//! - Transparent wireframe cubes representing octree nodes
//! - Ships moving around triggering dynamic octree restructuring
//! - Camera positioned at 30-degree angle for clear view
//! - Real-time octree subdivision based on entity density

#![allow(dead_code)]

use rust_engine::spatial::{Octree, OctreeConfig};
use rust_engine::physics::{CollisionSystem, CollisionShape};
use rust_engine::scene::{AABB, SceneManager};
use rust_engine::assets::ObjLoader;
use rust_engine::ecs::{World, Entity, LightingSystem as EcsLightingSystem};
use rust_engine::ecs::components::TransformComponent;
use rust_engine::render::{
    Camera, Mesh, GraphicsEngine, VulkanRendererConfig,
    WindowHandle, systems::dynamic::MeshType, Vertex, RenderFrameData,
    FontAtlas,
};
use rust_engine::render::resources::materials::{Material, UnlitMaterialParams};
use rust_engine::foundation::math::Vec3;
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;
use std::f32::consts::PI;

// Octree visualization settings
const OCTREE_SIZE: f32 = 100.0;  // Octree bounds: -50 to +50 on each axis
const CAMERA_DISTANCE: f32 = 200.0;
const CAMERA_ANGLE: f32 = 30.0 * PI / 180.0;  // 30 degrees

// Ship movement settings
const NUM_SMALL_SHIPS: usize = 15;  // Small ships
const NUM_LARGE_SHIPS: usize = 5;   // Large ships
const SMALL_SHIP_SPEED: f32 = 10.0;
const LARGE_SHIP_SPEED: f32 = 5.0;
const SMALL_SHIP_SIZE: f32 = 2.0;  // Visual and collision size
const LARGE_SHIP_SIZE: f32 = 8.0;  // Visual and collision size

// Visualization colors
const OCTREE_LINE_COLOR: [f32; 4] = [0.0, 0.8, 1.0, 0.3];  // Cyan with transparency
const ACTIVE_NODE_COLOR: [f32; 4] = [1.0, 0.5, 0.0, 0.5];  // Orange for nodes with entities

struct Ship {
    entity: Entity,
    velocity: Vec3,
    size: f32,  // Visual scale AND collision radius (same value!)
    bounds_extents: Vec3,
    original_color: Vec3,  // Store original color to restore after collision
    is_colliding: bool,     // Track collision state
}

pub struct OctreeVisualizationApp {
    window: WindowHandle,
    graphics_engine: GraphicsEngine,
    camera: Camera,
    scene_manager: SceneManager,
    world: World,
    lighting_system: EcsLightingSystem,
    
    // Octree
    octree: Octree,
    
    // Collision system
    collision_system: CollisionSystem,
    
    // Ships
    ships: Vec<Ship>,
    
    // Visualization meshes
    small_ship_mesh: Option<Mesh>,
    large_ship_mesh: Option<Mesh>,
    wireframe_cube_mesh: Mesh,
    
    // Visualization entities (octree node cubes)
    visualization_entities: Vec<Entity>,
    
    // Ray visualization entities (for mouse picking debug)
    ray_visualization_entities: Vec<Entity>,
    
    // Camera control state
    camera_move_forward: bool,
    camera_move_backward: bool,
    camera_move_left: bool,
    camera_move_right: bool,
    camera_yaw_left: bool,
    camera_yaw_right: bool,
    camera_pitch_up: bool,
    camera_pitch_down: bool,
    camera_yaw: f32,
    camera_pitch: f32,
    
    // Simulation control
    paused: bool,
    
    // FPS tracking
    fps_update_timer: Instant,
    frame_count: u32,
    current_fps: f32,
    
    // UI
    ui_manager: rust_engine::ui::UIManager,
    fps_label_id: rust_engine::ui::UINodeId,
    
    // Time
    start_time: Instant,
    
    // Selection state for mouse picking
    selected_ship: Option<Entity>,
    mouse_x: f64,
    mouse_y: f64,
}

impl OctreeVisualizationApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Initialize window using WindowHandle::new (smaller size)
        let mut window = WindowHandle::new(1280, 720, "Octree Visualization Demo");
        
        // Initialize graphics engine
        let renderer_config = VulkanRendererConfig::new("Octree Visualization")
            .with_version(1, 0, 0)
            .with_shader_paths(
                "target/shaders/standard_pbr_vert.spv".to_string(),
                "target/shaders/standard_pbr_frag.spv".to_string()
            );
        
        let mut graphics_engine = GraphicsEngine::new_from_window(&mut window, &renderer_config)?;
        
        // Initialize UI text rendering system
        graphics_engine.initialize_ui_text_system()?;
        
        // Initialize dynamic object system with proper mesh type pools
        graphics_engine.initialize_dynamic_system(200)?;
        
        // Create mesh pools for the types we'll use
        // 1. Sphere pool for ships
        let sphere_mesh = ObjLoader::load_obj("resources/models/sphere.obj")?;
        let sphere_material = Material::transparent_unlit(UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 0.9,
        });
        graphics_engine.create_mesh_pool(
            MeshType::Sphere,
            &sphere_mesh,
            &[sphere_material],
            20  // For ships
        )?;
        
        // 2. Cube pool for octree visualization
        let cube_mesh = Mesh::cube();
        let cube_material = Material::transparent_unlit(UnlitMaterialParams {
            color: Vec3::new(0.3, 0.7, 1.0),
            alpha: 0.1,  // Very transparent
        });
        graphics_engine.create_mesh_pool(
            MeshType::Cube,
            &cube_mesh,
            &[cube_material],
            300  // Increased for deeper octree subdivisions
        )?;
        
        // Set up camera - positioned to see the octree cube clearly
        // Position it at an angle (front-right-above) looking at center
        let camera_pos = Vec3::new(150.0, 80.0, -150.0);
        let mut camera = Camera::perspective(
            camera_pos,
            60.0,
            1280.0 / 720.0,
            0.1,
            1000.0,
        );
        camera.set_position(camera_pos);
        camera.look_at(Vec3::zeros(), Vec3::new(0.0, 1.0, 0.0));  // Look at origin
        
        // Calculate initial yaw and pitch from the camera's look direction
        // Direction from camera to origin: (0,0,0) - (150, 80, -150) = (-150, -80, 150)
        let initial_direction = Vec3::zeros() - camera_pos;
        let initial_direction_normalized = initial_direction.normalize();
        
        // yaw = atan2(z, x) where z is forward component, x is right component
        let initial_yaw = initial_direction_normalized.z.atan2(initial_direction_normalized.x);
        // pitch = asin(y)
        let initial_pitch = initial_direction_normalized.y.asin();
        
        println!("Camera initialized at {:?}, looking at origin", camera_pos);
        println!("Initial yaw: {:.2} rad, pitch: {:.2} rad", initial_yaw, initial_pitch);
        println!("Camera view matrix: {:?}", camera.get_view_matrix());
        println!("Camera projection matrix: {:?}", camera.get_projection_matrix());
        
        // Create ECS world and scene manager
        let world = World::new();
        let scene_manager = SceneManager::new();
        let lighting_system = EcsLightingSystem::new();
        
        // Create octree with fixed bounds
        let octree_bounds = AABB::new(
            Vec3::new(-OCTREE_SIZE / 2.0, -OCTREE_SIZE / 2.0, -OCTREE_SIZE / 2.0),
            Vec3::new(OCTREE_SIZE / 2.0, OCTREE_SIZE / 2.0, OCTREE_SIZE / 2.0),
        );
        
        let octree_config = OctreeConfig {
            max_entities_per_node: 3,  // Low threshold for visible subdivision
            max_depth: 4,
            min_node_size: 5.0,
        };
        
        let octree = Octree::new(octree_bounds, octree_config);
        
        // Create collision system
        let collision_system = CollisionSystem::new();
        
        // Create wireframe cube mesh for octree visualization
        let wireframe_cube_mesh = Self::create_wireframe_cube();
        
        // Create UI Manager with FPS counter
        let mut ui_manager = rust_engine::ui::UIManager::new();
        
        // Create font atlas for UI
        let font_path = "resources/fonts/default.ttf";
        let font_data = std::fs::read(font_path)
            .expect(&format!("Failed to read font file: {}", font_path));
        let mut font_atlas = FontAtlas::new(&font_data, 24.0)
            .expect("Failed to create font atlas");
        font_atlas.rasterize_glyphs()
            .expect("Failed to rasterize glyphs");
        ui_manager.initialize_font_atlas(font_atlas);
        
        // FPS counter
        use rust_engine::ui::{UIText, UIElement, Anchor, HorizontalAlign, VerticalAlign};
        use rust_engine::foundation::math::Vec4;
        
        let fps_text = UIText {
            element: UIElement {
                position: (10.0, 30.0),
                size: (0.0, 0.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "FPS: 0.0".to_string(),
            font_size: 24.0,
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            h_align: HorizontalAlign::Left,
            v_align: VerticalAlign::Top,
        };
        let fps_label_id = ui_manager.add_text(fps_text);
        
        let now = Instant::now();
        
        Ok(Self {
            window,
            graphics_engine,
            camera,
            scene_manager,
            world,
            lighting_system,
            octree,
            collision_system,
            ships: Vec::new(),
            small_ship_mesh: None,
            large_ship_mesh: None,
            wireframe_cube_mesh,
            visualization_entities: Vec::new(),
            ray_visualization_entities: Vec::new(),
            camera_move_forward: false,
            camera_move_backward: false,
            camera_move_left: false,
            camera_move_right: false,
            camera_yaw_left: false,
            camera_yaw_right: false,
            camera_pitch_up: false,
            camera_pitch_down: false,
            camera_yaw: initial_yaw,
            camera_pitch: initial_pitch,
            paused: false,
            fps_update_timer: now,
            frame_count: 0,
            current_fps: 60.0,
            ui_manager,
            fps_label_id,
            start_time: now,
            selected_ship: None,
            mouse_x: 0.0,
            mouse_y: 0.0,
        })
    }
    
    /// Create a wireframe cube mesh
    fn create_wireframe_cube() -> Mesh {
        // 8 vertices for cube corners - no texture coordinates needed for wireframe
        let vertices = vec![
            // Bottom face
            Vertex::new([-0.5, -0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::new([0.5, -0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::new([0.5, -0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::new([-0.5, -0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            // Top face
            Vertex::new([-0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::new([0.5, 0.5, -0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::new([0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
            Vertex::new([-0.5, 0.5, 0.5], [0.0, 1.0, 0.0], [0.0, 0.0]),
        ];
        
        // Line list indices (each pair forms a line)
        let indices = vec![
            // Bottom face edges
            0, 1,  1, 2,  2, 3,  3, 0,
            // Top face edges
            4, 5,  5, 6,  6, 7,  7, 4,
            // Vertical edges
            0, 4,  1, 5,  2, 6,  3, 7,
        ];
        
        Mesh { vertices, indices }
    }
    
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Initializing Octree Visualization Demo...");
        
        // Load ship meshes
        let spaceship_path = "resources/models/spaceship_simple.obj";
        let frigate_path = "resources/models/frigate.obj";
        
        match ObjLoader::load_obj(spaceship_path) {
            Ok(mesh) => {
                println!("Loaded small ship mesh: {} vertices", mesh.vertices.len());
                self.small_ship_mesh = Some(mesh);
            }
            Err(e) => {
                eprintln!("Failed to load small ship: {}", e);
                self.small_ship_mesh = Some(Mesh::cube());
            }
        }
        
        match ObjLoader::load_obj(frigate_path) {
            Ok(mesh) => {
                println!("Loaded large ship mesh: {} vertices", mesh.vertices.len());
                self.large_ship_mesh = Some(mesh);
            }
            Err(e) => {
                eprintln!("Failed to load large ship: {}", e);
                self.large_ship_mesh = Some(Mesh::cube());
            }
        }
        
        // Create small ships
        for i in 0..NUM_SMALL_SHIPS {
            self.create_ship(i as f32, true);
        }
        
        // Create large ships
        for i in 0..NUM_LARGE_SHIPS {
            self.create_ship(i as f32 + NUM_SMALL_SHIPS as f32, false);
        }
        
        println!("Created {} small ships and {} large ships", NUM_SMALL_SHIPS, NUM_LARGE_SHIPS);
        println!("Octree bounds: {:?}", self.octree.root.bounds);
        
        // Add directional sunlight
        use rust_engine::ecs::components::lighting::LightFactory;
        let sun_entity = self.world.create_entity();
        self.world.add_component(
            sun_entity,
            LightFactory::directional(
                Vec3::new(-0.3, -1.0, -0.5).normalize(),  // Light coming from upper-front-right
                Vec3::new(1.0, 0.95, 0.9),  // Warm sunlight color
                1.5,  // Intensity
            ),
        );
        println!("Added directional sunlight");
        
        // Build initial octree visualization
        self.rebuild_octree();
        
        Ok(())
    }
    
    fn create_ship(&mut self, _index: f32, is_small: bool) {
        // Random starting position within octree bounds
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bounds = OCTREE_SIZE / 2.0 - 10.0;
        
        let position = Vec3::new(
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
        );
        
        // Random velocity
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize() * if is_small { SMALL_SHIP_SPEED } else { LARGE_SHIP_SPEED };
        
        let size = if is_small { SMALL_SHIP_SIZE } else { LARGE_SHIP_SIZE };
        let mesh = if is_small {
            self.small_ship_mesh.as_ref().unwrap().clone()
        } else {
            self.large_ship_mesh.as_ref().unwrap().clone()
        };
        
        // Create material with ship color
        let color = if is_small {
            Vec3::new(0.2, 0.8, 0.2)  // Green for small ships
        } else {
            Vec3::new(0.8, 0.2, 0.2)  // Red for large ships
        };
        
        use rust_engine::render::resources::materials::StandardMaterialParams;
        let material = Material::standard_pbr(StandardMaterialParams {
            base_color: color,
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        
        // Use scene_manager to create entity (automatically registers it)
        // Scale the mesh to match the octree size expectations
        use rust_engine::foundation::math::Transform;
        let scale_vec = Vec3::new(size, size, size);  // Uniform scale
        let transform = Transform {
            position,
            rotation: rust_engine::foundation::math::Quat::identity(),
            scale: scale_vec,
        };
        
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            mesh,
            MeshType::Sphere,
            material,
            transform
        );
        
        // Insert into octree using the SAME size for collision radius
        // Sphere.obj has radius 1.0, so when scaled by 'size', final radius = size
        let collision_radius = size;  // NOT size/2.0 because sphere.obj already has radius 1.0!
        self.octree.insert(entity, position, collision_radius);
        
        self.ships.push(Ship {
            entity,
            velocity,
            size,
            bounds_extents: Vec3::new(size, size, size),
            original_color: color,
            is_colliding: false,
        });
    }
    
    pub fn update(&mut self, delta_time: f32) {
        // Check if any camera controls are active
        let has_movement = self.camera_move_forward || self.camera_move_backward ||
                          self.camera_move_left || self.camera_move_right ||
                          self.camera_yaw_left || self.camera_yaw_right ||
                          self.camera_pitch_up || self.camera_pitch_down;
        
        if has_movement {
            // Apply camera movement
            let camera_speed = 50.0; // Units per second
            let camera_rotation_speed = 1.5; // Radians per second
            
            // Get camera forward and right vectors
            let forward = Vec3::new(
                self.camera_yaw.cos() * self.camera_pitch.cos(),
                self.camera_pitch.sin(),
                self.camera_yaw.sin() * self.camera_pitch.cos(),
            ).normalize();
            let right = Vec3::new(
                (self.camera_yaw + PI / 2.0).cos(),
                0.0,
                (self.camera_yaw + PI / 2.0).sin(),
            ).normalize();
            
            let mut camera_pos = self.camera.position;
            
            // WASD movement
            if self.camera_move_forward {
                camera_pos = camera_pos + forward * camera_speed * delta_time;
            }
            if self.camera_move_backward {
                camera_pos = camera_pos - forward * camera_speed * delta_time;
            }
            if self.camera_move_left {
                camera_pos = camera_pos - right * camera_speed * delta_time;
            }
            if self.camera_move_right {
                camera_pos = camera_pos + right * camera_speed * delta_time;
            }
            
            // QE yaw rotation
            if self.camera_yaw_left {
                self.camera_yaw -= camera_rotation_speed * delta_time;
            }
            if self.camera_yaw_right {
                self.camera_yaw += camera_rotation_speed * delta_time;
            }
            
            // CX pitch rotation
            if self.camera_pitch_up {
                self.camera_pitch += camera_rotation_speed * delta_time;
            }
            if self.camera_pitch_down {
                self.camera_pitch -= camera_rotation_speed * delta_time;
            }
            
            // Clamp pitch to avoid gimbal lock
            self.camera_pitch = self.camera_pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
            
            // Update camera position
            self.camera.set_position(camera_pos);
            
            // Calculate new target point based on look direction
            let look_direction = Vec3::new(
                self.camera_yaw.cos() * self.camera_pitch.cos(),
                self.camera_pitch.sin(),
                self.camera_yaw.sin() * self.camera_pitch.cos(),
            ).normalize();
            
            // Set target to a point in front of the camera
            let new_target = camera_pos + look_direction;
            self.camera.set_target(new_target);
        }
        
        // Only update simulation if not paused
        if !self.paused {
            // Update ship positions
            let half_bounds = OCTREE_SIZE / 2.0;
            
            for ship in &mut self.ships {
                if let Some(transform) = self.world.get_component_mut::<TransformComponent>(ship.entity) {
                    let mut pos = transform.position;
                    pos += ship.velocity * delta_time;
                    
                    // Bounce off walls
                    if pos.x < -half_bounds || pos.x > half_bounds {
                        ship.velocity.x = -ship.velocity.x;
                        pos.x = pos.x.clamp(-half_bounds, half_bounds);
                    }
                    if pos.y < -half_bounds || pos.y > half_bounds {
                        ship.velocity.y = -ship.velocity.y;
                        pos.y = pos.y.clamp(-half_bounds, half_bounds);
                    }
                    if pos.z < -half_bounds || pos.z > half_bounds {
                        ship.velocity.z = -ship.velocity.z;
                        pos.z = pos.z.clamp(-half_bounds, half_bounds);
                    }
                    
                    transform.position = pos;
                }
            }
            
            // Update octree EVERY frame with current ship positions
            // This ensures collision detection has accurate spatial data
            self.rebuild_octree();
        }
        
        // ALWAYS detect and handle collisions (handles coloring based on selection)
        // This runs even when paused so that clicking on a ball updates colors immediately
        self.detect_and_handle_collisions();
        
        // Update FPS counter
        self.frame_count += 1;
        let fps_elapsed = self.fps_update_timer.elapsed();
        if fps_elapsed.as_secs_f32() >= 0.5 {
            self.current_fps = self.frame_count as f32 / fps_elapsed.as_secs_f32();
            self.frame_count = 0;
            self.fps_update_timer = Instant::now();
        }
        
        let fps_text = format!("FPS: {:.1}", self.current_fps);
        self.ui_manager.update_text(self.fps_label_id, fps_text);
    }
    
    fn rebuild_octree(&mut self) {
        // Clear octree
        self.octree.clear();
        
        // Reinsert all ships with consistent collision radius
        // Sphere.obj has radius 1.0, so when scaled by 'size', final radius = size
        for ship in &self.ships {
            if let Some(transform) = self.world.get_component::<TransformComponent>(ship.entity) {
                self.octree.insert(ship.entity, transform.position, ship.size);
            }
        }
        
        // Update visualization
        self.update_octree_visualization();
        
        // CRITICAL: Sync after creating new visualization entities
        self.scene_manager.sync_from_world(&mut self.world);
        
        // Debug logging
        let leaves = self.octree.get_all_leaves();
        /*println!("Octree rebuilt: {} leaf nodes, {} visualization entities", 
                 leaves.len(), self.visualization_entities.len());*/
    }
    
    fn detect_and_handle_collisions(&mut self) {
        use rust_engine::ecs::components::RenderableComponent;
        
        // Use octree for efficient collision detection (broad phase)
        // For each ship, query nearby entities from the octree and check collisions
        let mut colliding_entities = std::collections::HashSet::new();
        
        // Track entities that are near the selected ship (for cyan coloring)
        let mut selected_nearby_entities = std::collections::HashSet::new();
        
        // Track entities that are collision targets of the selected ship (for collision detection visualization)
        let mut selected_collision_targets = std::collections::HashSet::new();
        if let Some(selected) = self.selected_ship {
            // Get selected ship's collision shape
            if let Some(selected_ship) = self.ships.iter().find(|s| s.entity == selected) {
                if let Some(selected_transform) = self.world.get_component::<TransformComponent>(selected) {
                    let selected_shape = CollisionShape::sphere(selected_transform.position, selected_ship.size);
                    
                    // Query all nearby entities from octree (these are candidates for collision detection)
                    let nearby = self.octree.query_nearby(selected);
                    for nearby_entity in nearby {
                        if nearby_entity == selected {
                            continue;
                        }
                        
                        // Mark as nearby (for cyan coloring)
                        selected_nearby_entities.insert(nearby_entity);
                        
                        // Check collision for actual intersections
                        if let Some(nearby_ship) = self.ships.iter().find(|s| s.entity == nearby_entity) {
                            if let Some(nearby_transform) = self.world.get_component::<TransformComponent>(nearby_entity) {
                                let nearby_shape = CollisionShape::sphere(nearby_transform.position, nearby_ship.size);
                                
                                if let (CollisionShape::Sphere(s1), CollisionShape::Sphere(s2)) = (&selected_shape, &nearby_shape) {
                                    if s1.intersects(s2) {
                                        selected_collision_targets.insert(nearby_entity);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        for ship in &self.ships {
            // Get this ship's current transform
            let ship_transform = match self.world.get_component::<TransformComponent>(ship.entity) {
                Some(t) => t,
                None => continue,
            };
            
            // Query nearby entities using the octree (broad phase)
            let nearby = self.octree.query_nearby(ship.entity);
            
            // For each nearby entity, perform detailed collision check (narrow phase)
            for nearby_entity in nearby {
                if nearby_entity.id() == ship.entity.id() {
                    continue; // Skip self
                }
                
                // Find the nearby ship to get its size
                let nearby_ship = match self.ships.iter().find(|s| s.entity.id() == nearby_entity.id()) {
                    Some(s) => s,
                    None => continue,
                };
                
                // Get nearby entity's transform
                let nearby_transform = match self.world.get_component::<TransformComponent>(nearby_entity) {
                    Some(t) => t,
                    None => continue,
                };
                
                // Create bounding spheres with current positions
                // Sphere.obj has radius 1.0, scaled by ship.size → final radius = ship.size
                let sphere_a = CollisionShape::sphere(ship_transform.position, ship.size);
                let sphere_b = CollisionShape::sphere(nearby_transform.position, nearby_ship.size);
                
                // Check if spheres intersect (narrow phase)
                let (CollisionShape::Sphere(sphere_a), CollisionShape::Sphere(sphere_b)) = (&sphere_a, &sphere_b);
                if sphere_a.intersects(sphere_b) {
                    colliding_entities.insert(ship.entity);
                    colliding_entities.insert(nearby_entity);
                }
            }
        }
        
        // Update ship collision states
        for ship in &mut self.ships {
            ship.is_colliding = colliding_entities.contains(&ship.entity);
        }
        
        // Update material colors based on collision state
        // Priority: Selected ship (blue) > Nearby ships to selected (cyan) > Colliding (white) > Original
        for ship in &self.ships {
            if let Some(renderable) = self.world.get_component_mut::<RenderableComponent>(ship.entity) {
                let new_color = if Some(ship.entity) == self.selected_ship {
                    // Selected ship is BLUE
                    Vec3::new(0.0, 0.0, 1.0)
                } else if selected_nearby_entities.contains(&ship.entity) {
                    // Ships near the selected ship are CYAN (octree collision detection candidates)
                    Vec3::new(0.0, 1.0, 1.0)
                } else if ship.is_colliding {
                    // Other colliding ships are WHITE
                    Vec3::new(1.0, 1.0, 1.0)
                } else {
                    // Non-colliding, non-selected ships keep original color
                    ship.original_color
                };
                
                // Update the material color using PBR material
                use rust_engine::render::resources::materials::StandardMaterialParams;
                renderable.material = Material::standard_pbr(StandardMaterialParams {
                    base_color: new_color,
                    alpha: 1.0,
                    metallic: 0.1,
                    roughness: 0.7,
                    ..Default::default()
                });
            }
        }
    }
    
    fn update_octree_visualization(&mut self) {
        // Properly despawn old visualization entities to free GPU resources
        for entity in self.visualization_entities.drain(..) {
            self.scene_manager.destroy_entity(
                &mut self.world,
                &mut self.graphics_engine,
                entity,
            );
        }
        
        // Get all leaf nodes
        let leaves = self.octree.get_all_leaves();
        
        // Create visualization cube for each leaf node
        for leaf in leaves {
            let center = leaf.bounds.center();
            let extents = leaf.bounds.extents();
            // extents = half-size from center to edge
            // Mesh::cube() goes from -1.0 to 1.0 (full size = 2.0)
            // So to scale cube to match node: scale = extents (which is half the node size)
            // This makes the cube go from -extents to +extents, matching the AABB
            // Then * 0.95 to create small gaps between adjacent cubes
            let scale = extents * 0.95;
            
            // Color based on entity count
            let color = if leaf.entities.is_empty() {
                Vec3::new(0.0, 0.8, 1.0)  // Cyan for empty
            } else {
                Vec3::new(1.0, 0.5, 0.0)  // Orange for occupied
            };
            
            let material = Material::transparent_unlit(UnlitMaterialParams {
                color,
                alpha: 0.05,  // Very transparent to see through octree structure
            });
            
            // Use scene_manager to create entity (automatically registers it)
            use rust_engine::foundation::math::Transform;
            let transform = Transform {
                position: center,
                rotation: rust_engine::foundation::math::Quat::identity(),
                scale,
            };
            
            let entity = self.scene_manager.create_renderable_entity(
                &mut self.world,
                self.wireframe_cube_mesh.clone(),
                MeshType::Cube,
                material,
                transform
            );
            
            self.visualization_entities.push(entity);
        }
    }
    
    pub fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Sync transforms from ECS to scene manager before rendering
        // This ensures camera movement and ship movement are reflected
        self.scene_manager.sync_from_world(&mut self.world);
        
        // Build render queue (scene_manager already tracks entities via create_renderable_entity)
        let render_queue = self.scene_manager.build_render_queue();
        
        // DEBUG: Print render queue info
        use std::sync::atomic::{AtomicU32, Ordering};
        static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
        let frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        
        self.graphics_engine.render_entities_from_queue(&render_queue)?;
        
        // Build lighting environment
        let multi_light_env = self.lighting_system.build_multi_light_environment(
            &self.world,
            Vec3::new(0.1, 0.1, 0.1),
            0.1,
        ).clone();
        
        // Get UI render data
        let ui_data = self.ui_manager.get_render_data();
        
        // Render frame
        self.graphics_engine.render_frame(
            &RenderFrameData {
                camera: &self.camera,
                lights: &multi_light_env,
                ui: &ui_data,
            },
            &mut self.window,
        )?;
        
        Ok(())
    }
    
    /// Handle mouse click for ray casting and entity selection
    fn handle_mouse_click(&mut self) {
        // Get window dimensions
        let (width, height) = self.window.get_size();
        
        // Convert mouse position to NDC (-1 to 1)
        // Following ARCHITECTURE.md: Standard formula for screen-to-NDC conversion
        // Screen space: (0,0) at top-left, Y-down
        // NDC: (-1,-1) at top-left with positive viewport
        let ndc_x = (self.mouse_x / width as f64) as f32 * 2.0 - 1.0;
        let ndc_y = (self.mouse_y / height as f64) as f32 * 2.0 - 1.0;  // NO FLIP with positive viewport
        
        // Generate ray from camera
        let ray = self.camera.screen_to_world_ray(ndc_x, ndc_y);
        
        println!("Mouse click at ({:.2}, {:.2}), window: {}x{}, NDC: ({:.3}, {:.3})", 
                 self.mouse_x, self.mouse_y, width, height, ndc_x, ndc_y);
        println!("Ray: origin={:?}, direction={:?}", ray.origin, ray.direction);
        
        // ====== EFFICIENT OCTREE RAY QUERY ======
        // Instead of testing ALL ships, use octree to only get candidates in ray path
        let octree_candidates = self.octree.query_ray(ray.origin, ray.direction);
        
        println!("Octree filtering: {} candidates (from {} total ships)", 
                 octree_candidates.len(), self.ships.len());
        
        // Build collision shapes ONLY for candidates identified by octree
        let shapes: Vec<(Entity, CollisionShape)> = octree_candidates.iter()
            .filter_map(|octree_entity| {
                // Find the corresponding ship
                self.ships.iter().find(|ship| ship.entity == octree_entity.id)
                    .and_then(|ship| {
                        // Get transform and create collision shape
                        self.world.get_component::<TransformComponent>(ship.entity)
                            .map(|transform| {
                                let shape = CollisionShape::sphere(transform.position, ship.size);
                                (ship.entity, shape)
                            })
                    })
            })
            .collect();
        
        println!("Testing ray against {} candidate ships (filtered by octree)", shapes.len());
        
        // DEBUG: Print candidate ship positions
        if !shapes.is_empty() {
            println!("  Octree candidates:");
            for (i, (_entity, shape)) in shapes.iter().enumerate() {
                match shape {
                    CollisionShape::Sphere(sphere) => {
                        // Calculate closest point on ray to sphere center
                        let oc = sphere.center - ray.origin;
                        let t = oc.dot(&ray.direction).max(0.0);
                        let closest_point = ray.point_at(t);
                        let dist_to_ray = (closest_point - sphere.center).magnitude();
                        
                        println!("    Candidate {}: center={:?}, radius={:.2}, dist_to_ray={:.2}", 
                                 i, sphere.center, sphere.radius, dist_to_ray);
                    }
                }
            }
        }
        
        // Perform ray cast on filtered candidates
        if let Some(hit) = self.collision_system.ray_cast_first(&ray, &self.octree, &shapes) {
            println!("✓ Hit entity {:?} at distance {:.2}", hit.entity, hit.distance);
            
            // Clear previous selection (restore original color)
            if let Some(prev_selected) = self.selected_ship {
                if let Some(prev_ship) = self.ships.iter().find(|s| s.entity == prev_selected) {
                    self.update_ship_color(prev_selected, prev_ship.original_color);
                }
            }
            
            // Set new selection to BLUE
            self.selected_ship = Some(hit.entity);
            
            // DEBUG: Print which ship we're coloring
            if let Some(ship_index) = self.ships.iter().position(|s| s.entity == hit.entity) {
                if let Some(transform) = self.world.get_component::<TransformComponent>(hit.entity) {
                    println!("✓✓✓ Selected ship at array index {}, entity {:?}, position {:?}", 
                             ship_index, hit.entity, transform.position);
                }
            }
            
            // Note: Coloring is handled by detect_and_handle_collisions() every frame
        } else {
            println!("✗ No hit detected");
            // No hit - deselect
            if let Some(prev_selected) = self.selected_ship {
                if let Some(prev_ship) = self.ships.iter().find(|s| s.entity == prev_selected) {
                    self.update_ship_color(prev_selected, prev_ship.original_color);
                }
            }
            self.selected_ship = None;
        }
    }
    
    /// Highlight all ships that would collide with the selected ship in CYAN
    fn highlight_collision_targets(&mut self, selected_entity: Entity) {
        // Get the collision shape for the selected entity
        let selected_shape = self.ships.iter()
            .find(|s| s.entity == selected_entity)
            .and_then(|ship| {
                self.world.get_component::<TransformComponent>(ship.entity)
                    .map(|t| CollisionShape::sphere(t.position, ship.size))
            });
        
        if let Some(selected_shape) = selected_shape {
            // Query octree for nearby entities
            let nearby = self.octree.query_nearby(selected_entity);
            
            // Collect ship data to avoid borrowing issues
            let ship_data: Vec<(Entity, Vec3)> = self.ships.iter()
                .map(|s| (s.entity, s.original_color))
                .collect();
            
            // Check collision with each nearby ship
            for (entity, original_color) in ship_data {
                if entity == selected_entity {
                    continue; // Skip the selected ship itself
                }
                
                // Restore original color first
                self.update_ship_color(entity, original_color);
                
                // Check if nearby
                if nearby.contains(&entity) {
                    // Get collision shape and check intersection
                    if let Some(transform) = self.world.get_component::<TransformComponent>(entity) {
                        if let Some(ship) = self.ships.iter().find(|s| s.entity == entity) {
                            let ship_shape = CollisionShape::sphere(transform.position, ship.size);
                            
                            // Check collision using BoundingSphere intersection
                            let (CollisionShape::Sphere(s1), CollisionShape::Sphere(s2)) = (&selected_shape, &ship_shape);
                            if s1.intersects(s2) {
                                // Set to CYAN
                                self.update_ship_color(entity, Vec3::new(0.0, 1.0, 1.0));
                            }
                        }
                    }
                }
            }
        }
    }
    
    /// Update a ship's color in the ECS
    fn update_ship_color(&mut self, entity: Entity, color: Vec3) {
        use rust_engine::ecs::components::RenderableComponent;
        use rust_engine::render::resources::materials::StandardMaterialParams;
        
        if let Some(renderable) = self.world.get_component_mut::<RenderableComponent>(entity) {
            // Update material color using PBR material (same as ships are created with)
            renderable.material = Material::standard_pbr(StandardMaterialParams {
                base_color: color,
                alpha: 1.0,
                metallic: 0.1,
                roughness: 0.7,
                ..Default::default()
            });
        }
    }
    
    /// Visualize a ray as a line in 3D space for debugging
    fn visualize_ray(&mut self, ray: &rust_engine::physics::collision::Ray) {
        use rust_engine::ecs::components::{TransformComponent, RenderableComponent};
        use rust_engine::render::resources::materials::{Material, UnlitMaterialParams};
        use rust_engine::render::systems::dynamic::MeshType;
        
        // Clean up previous ray visualization spheres by removing their components
        for entity in self.ray_visualization_entities.drain(..) {
            self.world.remove_component::<TransformComponent>(entity);
            self.world.remove_component::<RenderableComponent>(entity);
        }
        
        // Create several small spheres along the ray path to visualize it
        let sphere_mesh = self.small_ship_mesh.clone().unwrap();
        let yellow_material = Material::transparent_unlit(UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 0.0), // Bright yellow
            alpha: 0.8,
        });
        
        // Place spheres at intervals along the ray
        let num_spheres = 1;
        let max_distance = 50.0; // Max ray visualization length
        
        for i in 0..num_spheres {
            let distance = (i as f32 / num_spheres as f32) * max_distance;
            let position = ray.origin + ray.direction * distance;
            
            // Create entity for this sphere
            let sphere_entity = self.world.create_entity();
            
            // Add transform
            let transform = TransformComponent::from_position(position)
                .with_scale(Vec3::new(0.5, 0.5, 0.5)); // Small spheres
            self.world.add_component(sphere_entity, transform);
            
            // Add renderable
            let renderable = RenderableComponent::new(
                yellow_material.clone(),
                sphere_mesh.clone(),
                MeshType::Sphere,
            );
            self.world.add_component(sphere_entity, renderable);
            
            // Mark for sync and track for cleanup
            self.scene_manager.mark_entity_dirty(sphere_entity);
            self.ray_visualization_entities.push(sphere_entity);
        }
        
        println!("✓ Ray visualization created: {} spheres from {:?} direction {:?}", 
                 num_spheres, ray.origin, ray.direction);
    }
    
    pub fn handle_events(&mut self) -> (bool, bool) {
        self.window.poll_events();
        
        let events: Vec<_> = self.window.event_iter().collect();
        let mut framebuffer_resized = false;
        
        for (_, event) in events {
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    self.window.set_should_close(true);
                    return (false, false);
                }
                WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                    self.paused = !self.paused;
                    println!("Simulation {}", if self.paused { "PAUSED" } else { "RESUMED" });
                }
                WindowEvent::Key(Key::W, _, action, _) => {
                    self.camera_move_forward = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::S, _, action, _) => {
                    self.camera_move_backward = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::A, _, action, _) => {
                    self.camera_move_left = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::D, _, action, _) => {
                    self.camera_move_right = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::Q, _, action, _) => {
                    self.camera_yaw_left = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::E, _, action, _) => {
                    self.camera_yaw_right = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::C, _, action, _) => {
                    self.camera_pitch_up = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::Key(Key::X, _, action, _) => {
                    self.camera_pitch_down = action == Action::Press || action == Action::Repeat;
                }
                WindowEvent::CursorPos(x, y) => {
                    self.mouse_x = x;
                    self.mouse_y = y;
                }
                WindowEvent::MouseButton(glfw::MouseButton::Button1, Action::Press, _) => {
                    // Left mouse button clicked - perform ray cast and selection
                    self.handle_mouse_click();
                }
                WindowEvent::FramebufferSize(width, height) => {
                    framebuffer_resized = true;
                    if width > 0 && height > 0 {
                        self.camera.set_aspect_ratio(width as f32 / height as f32);
                        // Update UI screen size
                        self.ui_manager.set_screen_size(width as f32, height as f32);
                    }
                }
                _ => {}
            }
        }
        
        (!self.window.should_close(), framebuffer_resized)
    }
    
    pub fn run(mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize()?;
        
        // CRITICAL: Sync entities from ECS to scene manager's renderable cache
        // This populates the cache so build_render_queue() can return objects
        self.scene_manager.sync_from_world(&mut self.world);
        println!("After sync: scene_manager has {} entities", self.scene_manager.entity_count());
        
        let mut last_frame = Instant::now();
        
        loop {
            let (continue_running, framebuffer_resized) = self.handle_events();
            if !continue_running {
                break;
            }
            
            // Handle framebuffer resize
            if framebuffer_resized {
                log::info!("Framebuffer resized detected, recreating swapchain...");
                self.graphics_engine.recreate_swapchain(&mut self.window);
                let (width, height) = self.graphics_engine.get_swapchain_extent();
                if width > 0 && height > 0 {
                    self.camera.set_aspect_ratio(width as f32 / height as f32);
                }
            }
            
            let now = Instant::now();
            let delta_time = now.duration_since(last_frame).as_secs_f32();
            last_frame = now;
            
            self.update(delta_time);
            self.render()?;
        }
        
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    println!("=== Octree Visualization Demo ===");
    println!("Controls:");
    println!("  WASD - Move camera forward/back/left/right");
    println!("  Q/E  - Rotate camera left/right (yaw)");
    println!("  C/X  - Rotate camera up/down (pitch)");
    println!("  SPACE - Pause/Resume simulation");
    println!("  ESC  - Exit");
    println!();
    println!("Watch the octree dynamically subdivide as ships move!");
    println!("  Cyan cubes = empty octree nodes");
    println!("  Orange cubes = nodes containing ships");
    println!();
    
    let app = OctreeVisualizationApp::new()?;
    app.run()
}
