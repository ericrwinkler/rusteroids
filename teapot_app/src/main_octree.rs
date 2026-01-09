//! Octree Visualization Demo
//!
//! Visualizes a 3D octree spatial partitioning structure with:
//! - Transparent wireframe cubes representing octree nodes
//! - Ships moving around triggering dynamic octree restructuring
//! - Camera positioned at 30-degree angle for clear view
//! - Real-time octree subdivision based on entity density

use rust_engine::spatial::{Octree, OctreeConfig, OctreeSpatialQuery};
use rust_engine::physics::{CollisionShape, CollisionLayers};
use rust_engine::scene::{AABB, SceneManager};
use rust_engine::assets::ObjLoader;
use rust_engine::ecs::{World, Entity, LightingSystem as EcsLightingSystem};
use rust_engine::ecs::components::{TransformComponent, PickableComponent, SelectionComponent, ColliderComponent, CollisionStateComponent};
use rust_engine::ecs::systems::{PickingSystem, EcsCollisionSystem};
use rust_engine::render::{
    Camera, Mesh, GraphicsEngine, VulkanRendererConfig,
    WindowHandle, systems::dynamic::MeshType, Vertex, RenderFrameData,
    FontAtlas,
};
use rust_engine::render::resources::materials::{Material, UnlitMaterialParams, StandardMaterialParams};
use rust_engine::foundation::math::Vec3;
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;
use std::f32::consts::PI;

// Octree visualization settings
const OCTREE_SIZE: f32 = 100.0;  // Octree bounds: -50 to +50 on each axis

// Entity counts
const NUM_SMALL_MESH: usize = 15;
const NUM_SMALL_SPHERE: usize = 10;
const NUM_LARGE_MESH: usize = 3;
const NUM_LARGE_SPHERE: usize = 6;
const NUM_MONKEY: usize = 5;

// Movement speeds
const SMALL_SHIP_SPEED: f32 = 6.0;
const LARGE_SHIP_SPEED: f32 = 3.0;
const SMALL_SHIP_SIZE: f32 = 0.8;
const LARGE_SHIP_SIZE: f32 = 2.0;
const MONKEY_SIZE: f32 = 1.5;
const MONKEY_SPEED: f32 = 4.0;

/// Type of collision shape used for a ship
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CollisionShapeType {
    /// Simple sphere collision (fast)
    Sphere,
    /// Per-vertex mesh collision (accurate)
    Mesh,
}

struct Ship {
    entity: Entity,
    velocity: Vec3,
    size: f32,  // Visual scale
    collision_radius: f32,  // Actual collision bounding sphere radius
    original_color: Vec3,  // Store original color to restore after collision
    collision_shape_type: CollisionShapeType,  // Track what kind of collision to use
    mesh_type: MeshType,  // Track which mesh this ship uses (for collision matching)
}

pub struct OctreeVisualizationApp {
    window: WindowHandle,
    graphics_engine: GraphicsEngine,
    camera: Camera,
    scene_manager: SceneManager,
    world: World,
    lighting_system: EcsLightingSystem,
    
    // ECS Collision system (owns the octree)
    ecs_collision_system: EcsCollisionSystem,
    
    // Ships
    ships: Vec<Ship>,
    
    // Visualization meshes
    small_ship_mesh: Option<Mesh>,
    large_ship_mesh: Option<Mesh>,
    monkey_mesh: Option<Mesh>,
    sphere_mesh: Option<Mesh>,
    wireframe_cube_mesh: Mesh,
    
    // Visualization entities (octree node cubes)
    visualization_entities: Vec<Entity>,
    
    // Collision radius visualization sphere (for selected entity's query radius)
    collision_radius_sphere: Option<Entity>,
    
    // Bounding sphere visualization for nearby entities (cyan entities)
    nearby_bounding_spheres: std::collections::HashMap<Entity, Entity>,
    
    // Picking system (replaces old manual mouse handling)
    picking_system: PickingSystem,
    
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
            120  // For ships (doubled)
        )?;
        
        // 2. Spaceship pool for small ships
        let spaceship_mesh = ObjLoader::load_obj("resources/models/spaceship_simple.obj")?;
        let spaceship_material = Material::standard_pbr(StandardMaterialParams {
            base_color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        graphics_engine.create_mesh_pool(
            MeshType::Spaceship,
            &spaceship_mesh,
            &[spaceship_material],
            90  // For small ships (doubled)
        )?;
        
        // 3. Frigate pool for large ships
        let frigate_mesh = ObjLoader::load_obj("resources/models/frigate.obj")?;
        let frigate_material = Material::standard_pbr(StandardMaterialParams {
            base_color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        graphics_engine.create_mesh_pool(
            MeshType::Frigate,
            &frigate_mesh,
            &[frigate_material],
            30  // For large ships (doubled)
        )?;
        
        // 4. Monkey pool for collision objects
        let monkey_mesh = ObjLoader::load_obj("resources/models/monkey.obj")?;
        let monkey_material = Material::standard_pbr(StandardMaterialParams {
            base_color: Vec3::new(0.6, 0.4, 0.2), // Brown color
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        graphics_engine.create_mesh_pool(
            MeshType::Monkey,
            &monkey_mesh,
            &[monkey_material],
            20  // For monkeys
        )?;
        
        // 5. Cube pool for octree visualization
        let cube_mesh = Mesh::cube();
        let cube_material = Material::transparent_unlit(UnlitMaterialParams {
            color: Vec3::new(0.3, 0.7, 1.0),
            alpha: 0.1,  // Very transparent
        });
        graphics_engine.create_mesh_pool(
            MeshType::Cube,
            &cube_mesh,
            &[cube_material],
            1000  // Increased for deeper octree with more ships
        )?;
        
        // Set up camera - positioned to see the octree cube clearly
        // Position it at an angle (front-right-above) looking at center
        let camera_pos = Vec3::new(300.0, 160.0, -300.0);
        let mut camera = Camera::perspective(
            camera_pos,
            60.0,
            1280.0 / 720.0,
            0.1,
            2000.0,
        );
        camera.set_position(camera_pos);
        camera.look_at(Vec3::zeros(), Vec3::new(0.0, 1.0, 0.0));  // Look at origin
        
        // Calculate initial yaw and pitch from the camera's look direction
        // Direction from camera to origin: (0,0,0) - (300, 160, -300) = (-300, -160, 300)
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
            max_depth: 6,  // Increased for larger space
            min_node_size: 10.0,  // Adjusted for 200x200x200 space
        };
        
        // Create ECS collision system with its own octree
        let octree = Octree::new(octree_bounds, octree_config);
        let octree_query = Box::new(OctreeSpatialQuery::new(octree));
        let mut ecs_collision_system = EcsCollisionSystem::new(octree_query);
        ecs_collision_system.enable_debug(true); // Enable debug visualization
        
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
        
        // Initialize picking system
        let (window_width, window_height) = window.get_size();
        let picking_system = PickingSystem::new(window_width as u32, window_height as u32);
        
        let now = Instant::now();
        
        Ok(Self {
            window,
            graphics_engine,
            camera,
            scene_manager,
            world,
            lighting_system,
            ecs_collision_system,
            ships: Vec::new(),
            small_ship_mesh: None,
            large_ship_mesh: None,
            monkey_mesh: None,
            sphere_mesh: None,
            wireframe_cube_mesh,
            visualization_entities: Vec::new(),
            collision_radius_sphere: None,
            nearby_bounding_spheres: std::collections::HashMap::new(),
            picking_system,
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
        
        // Load monkey mesh
        match ObjLoader::load_obj("resources/models/monkey.obj") {
            Ok(mesh) => {
                println!("Loaded monkey mesh: {} vertices", mesh.vertices.len());
                self.monkey_mesh = Some(mesh);
            }
            Err(e) => {
                eprintln!("Failed to load monkey mesh: {}", e);
                self.monkey_mesh = Some(Mesh::cube());
            }
        }
        
        // Load sphere mesh
        match ObjLoader::load_obj("resources/models/sphere.obj") {
            Ok(mesh) => {
                println!("Loaded sphere mesh: {} vertices", mesh.vertices.len());
                self.sphere_mesh = Some(mesh);
            }
            Err(e) => {
                eprintln!("Failed to load sphere mesh: {}", e);
                self.sphere_mesh = Some(Mesh::cube());
            }
        }
        
        // Create entities with for loops
        for _ in 0..NUM_SMALL_MESH {
            self.create_small_mesh();
        }
        
        for _ in 0..NUM_SMALL_SPHERE {
            self.create_small_sphere();
        }
        
        for _ in 0..NUM_LARGE_MESH {
            self.create_large_mesh();
        }
        
        for _ in 0..NUM_LARGE_SPHERE {
            self.create_large_sphere();
        }
        
        for _ in 0..NUM_MONKEY {
            self.create_monkey();
        }
        
        let total = NUM_SMALL_MESH + NUM_SMALL_SPHERE + NUM_LARGE_MESH + NUM_LARGE_SPHERE + NUM_MONKEY;
        println!("Created {} entities:", total);
        println!("  {} small mesh (spaceship) with mesh collision (blue)", NUM_SMALL_MESH);
        println!("  {} small sphere with sphere collision (green)", NUM_SMALL_SPHERE);
        println!("  {} large mesh (frigate) with mesh collision (purple)", NUM_LARGE_MESH);
        println!("  {} large sphere with sphere collision (red)", NUM_LARGE_SPHERE);
        println!("  {} monkey with mesh collision (yellow)", NUM_MONKEY);
        
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
        
        // Add PickableComponent to all ships for new picking system
        for ship in &self.ships {
            self.world.add_component(ship.entity, PickableComponent::new()
                .with_layer("world")
                .with_collision_radius(ship.collision_radius)
            );
        }
        
        Ok(())
    }
    
    /// Calculate bounding radius for a centered mesh
    /// Returns the distance to the farthest vertex from origin
    fn calculate_mesh_bounding_radius(mesh: &rust_engine::render::Mesh) -> f32 {
        let mut max_dist_sq = 0.0f32;
        
        for vertex in &mesh.vertices {
            let x = vertex.position[0];
            let y = vertex.position[1];
            let z = vertex.position[2];
            let dist_sq = x * x + y * y + z * z;
            max_dist_sq = max_dist_sq.max(dist_sq);
        }
        
        max_dist_sq.sqrt()
    }
    
    fn create_small_mesh(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bounds = OCTREE_SIZE / 2.0 - 10.0;
        
        let position = Vec3::new(
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
        );
        
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize() * SMALL_SHIP_SPEED;
        
        let render_mesh = self.small_ship_mesh.as_ref().unwrap().clone();
        let color = Vec3::new(0.2, 0.4, 0.9); // Blue
        
        use rust_engine::render::resources::materials::StandardMaterialParams;
        let material = Material::standard_pbr(StandardMaterialParams {
            base_color: color,
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        
        use rust_engine::foundation::math::Transform;
        let transform = Transform {
            position,
            rotation: Self::rotation_from_velocity(velocity),
            scale: Vec3::new(SMALL_SHIP_SIZE, SMALL_SHIP_SIZE, SMALL_SHIP_SIZE),
        };
        
        // Calculate bounding radius: distance to farthest vertex from center (origin)
        // Mesh is already centered by obj_loader, so we find max distance from (0,0,0)
        let collision_radius = Self::calculate_mesh_bounding_radius(&render_mesh) * SMALL_SHIP_SIZE;
        
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            render_mesh.clone(),
            MeshType::Spaceship,
            material,
            transform
        );
        
        // Add collision components for new ECS collision system
        // Extract MODEL SPACE vertices (no rotation/position applied)
        let model_vertices: Vec<Vec3> = render_mesh.vertices.iter()
            .map(|v| Vec3::new(v.position[0], v.position[1], v.position[2]))
            .collect();
        
        let mesh_shape = CollisionShape::mesh_from_model(&model_vertices, &render_mesh.indices);
        let mut collider = ColliderComponent::new(mesh_shape);
        // Override bounding_radius to match the manually calculated collision_radius
        collider.bounding_radius = collision_radius;
        let collider = collider
            .with_layers(CollisionLayers::VEHICLE, CollisionLayers::VEHICLE | CollisionLayers::ENVIRONMENT)
            .with_debug_draw(false); // Don't show individual ship shapes
        
        self.world.add_component(entity, collider.clone());
        self.world.add_component(entity, CollisionStateComponent::default());
        
        // Register with ECS collision system
        self.ecs_collision_system.register_collider(entity, &collider, &self.world);
        
        self.ships.push(Ship {
            entity,
            velocity,
            size: SMALL_SHIP_SIZE,
            collision_radius,
            original_color: color,
            collision_shape_type: CollisionShapeType::Mesh,
            mesh_type: MeshType::Spaceship,
        });
    }
    
    fn create_small_sphere(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bounds = OCTREE_SIZE / 2.0 - 10.0;
        
        let position = Vec3::new(
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
        );
        
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize() * SMALL_SHIP_SPEED;
        
        let render_mesh = self.sphere_mesh.as_ref().unwrap().clone();
        let color = Vec3::new(0.2, 0.8, 0.2); // Green
        
        use rust_engine::render::resources::materials::StandardMaterialParams;
        let material = Material::standard_pbr(StandardMaterialParams {
            base_color: color,
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        
        use rust_engine::foundation::math::Transform;
        let transform = Transform {
            position,
            rotation: Self::rotation_from_velocity(velocity),
            scale: Vec3::new(SMALL_SHIP_SIZE, SMALL_SHIP_SIZE, SMALL_SHIP_SIZE),
        };
        
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            render_mesh,
            MeshType::Sphere,
            material,
            transform
        );
        
        // Add collision components for new ECS collision system
        // Sphere: just radius (position from TransformComponent)
        let sphere_shape = CollisionShape::sphere(SMALL_SHIP_SIZE);
        let collider = ColliderComponent::new(sphere_shape)
            .with_layers(CollisionLayers::VEHICLE, CollisionLayers::VEHICLE | CollisionLayers::ENVIRONMENT)
            .with_debug_draw(false);
        
        self.world.add_component(entity, collider.clone());
        self.world.add_component(entity, CollisionStateComponent::default());
        self.ecs_collision_system.register_collider(entity, &collider, &self.world);
        
        self.ships.push(Ship {
            entity,
            velocity,
            size: SMALL_SHIP_SIZE,
            collision_radius: SMALL_SHIP_SIZE,
            original_color: color,
            collision_shape_type: CollisionShapeType::Sphere,
            mesh_type: MeshType::Sphere,
        });
    }
    
    fn create_large_mesh(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bounds = OCTREE_SIZE / 2.0 - 10.0;
        
        let position = Vec3::new(
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
        );
        
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize() * LARGE_SHIP_SPEED;
        
        let render_mesh = self.large_ship_mesh.as_ref().unwrap().clone();
        let color = Vec3::new(0.7, 0.2, 0.9); // Purple
        
        use rust_engine::render::resources::materials::StandardMaterialParams;
        let material = Material::standard_pbr(StandardMaterialParams {
            base_color: color,
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        
        use rust_engine::foundation::math::Transform;
        let transform = Transform {
            position,
            rotation: Self::rotation_from_velocity(velocity),
            scale: Vec3::new(LARGE_SHIP_SIZE, LARGE_SHIP_SIZE, LARGE_SHIP_SIZE),
        };
        
        // Calculate bounding radius: distance to farthest vertex from center (origin)
        // Mesh is already centered by obj_loader, so we find max distance from (0,0,0)
        let collision_radius = Self::calculate_mesh_bounding_radius(&render_mesh) * LARGE_SHIP_SIZE;
        
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            render_mesh.clone(),
            MeshType::Frigate,
            material,
            transform
        );
        
        // Add collision components for new ECS collision system
        // Extract MODEL SPACE vertices (no rotation/position applied)
        let model_vertices: Vec<Vec3> = render_mesh.vertices.iter()
            .map(|v| Vec3::new(v.position[0], v.position[1], v.position[2]))
            .collect();
        
        let mesh_shape = CollisionShape::mesh_from_model(&model_vertices, &render_mesh.indices);
        let mut collider = ColliderComponent::new(mesh_shape);
        // Override bounding_radius to match the manually calculated collision_radius
        collider.bounding_radius = collision_radius;
        let collider = collider
            .with_layers(CollisionLayers::VEHICLE, CollisionLayers::VEHICLE | CollisionLayers::ENVIRONMENT)
            .with_debug_draw(false);
        
        self.world.add_component(entity, collider.clone());
        self.world.add_component(entity, CollisionStateComponent::default());
        self.ecs_collision_system.register_collider(entity, &collider, &self.world);
        
        self.ships.push(Ship {
            entity,
            velocity,
            size: LARGE_SHIP_SIZE,
            collision_radius,
            original_color: color,
            collision_shape_type: CollisionShapeType::Mesh,
            mesh_type: MeshType::Frigate,
        });
    }
    
    fn create_large_sphere(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bounds = OCTREE_SIZE / 2.0 - 10.0;
        
        let position = Vec3::new(
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
        );
        
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize() * LARGE_SHIP_SPEED;
        
        let render_mesh = self.sphere_mesh.as_ref().unwrap().clone();
        let color = Vec3::new(0.8, 0.2, 0.2); // Red
        
        use rust_engine::render::resources::materials::StandardMaterialParams;
        let material = Material::standard_pbr(StandardMaterialParams {
            base_color: color,
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        
        use rust_engine::foundation::math::Transform;
        let transform = Transform {
            position,
            rotation: Self::rotation_from_velocity(velocity),
            scale: Vec3::new(LARGE_SHIP_SIZE, LARGE_SHIP_SIZE, LARGE_SHIP_SIZE),
        };
        
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            render_mesh,
            MeshType::Sphere,
            material,
            transform
        );
        
        // Add collision components for new ECS collision system  
        // Sphere: just radius (position from TransformComponent)
        let sphere_shape = CollisionShape::sphere(LARGE_SHIP_SIZE);
        let collider = ColliderComponent::new(sphere_shape)
            .with_layers(CollisionLayers::VEHICLE, CollisionLayers::VEHICLE | CollisionLayers::ENVIRONMENT)
            .with_debug_draw(false);
        
        self.world.add_component(entity, collider.clone());
        self.world.add_component(entity, CollisionStateComponent::default());
        self.ecs_collision_system.register_collider(entity, &collider, &self.world);
        
        self.ships.push(Ship {
            entity,
            velocity,
            size: LARGE_SHIP_SIZE,
            collision_radius: LARGE_SHIP_SIZE,
            original_color: color,
            collision_shape_type: CollisionShapeType::Sphere,
            mesh_type: MeshType::Sphere,
        });
    }
    
    fn create_monkey(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bounds = OCTREE_SIZE / 2.0 - 10.0;
        
        let position = Vec3::new(
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
            rng.gen_range(-bounds..bounds),
        );
        
        let velocity = Vec3::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        ).normalize() * MONKEY_SPEED;
        
        let color = Vec3::new(1.0, 1.0, 0.0); // Yellow
        
        // Use cached monkey mesh
        let render_mesh = self.monkey_mesh.as_ref().unwrap().clone();
        
        let material = Material::standard_pbr(StandardMaterialParams {
            base_color: color,
            alpha: 1.0,
            metallic: 0.1,
            roughness: 0.7,
            ..Default::default()
        });
        
        use rust_engine::foundation::math::Transform;
        let transform = Transform {
            position,
            rotation: Self::rotation_from_velocity(velocity),
            scale: Vec3::new(MONKEY_SIZE, MONKEY_SIZE, MONKEY_SIZE),
        };
        
        // Calculate bounding radius: distance to farthest vertex from center (origin)
        // Mesh is already centered by obj_loader, so we find max distance from (0,0,0)
        let collision_radius = Self::calculate_mesh_bounding_radius(&render_mesh) * MONKEY_SIZE;
        
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            render_mesh.clone(),
            MeshType::Monkey,
            material,
            transform
        );
        
        // Add collision components for new ECS collision system
        // Extract MODEL SPACE vertices (no rotation/position applied)
        let model_vertices: Vec<Vec3> = render_mesh.vertices.iter()
            .map(|v| Vec3::new(v.position[0], v.position[1], v.position[2]))
            .collect();
        
        let mesh_shape = CollisionShape::mesh_from_model(&model_vertices, &render_mesh.indices);
        let mut collider = ColliderComponent::new(mesh_shape);
        // Override bounding_radius to match the manually calculated collision_radius
        collider.bounding_radius = collision_radius;
        let collider = collider
            .with_layers(CollisionLayers::VEHICLE, CollisionLayers::VEHICLE | CollisionLayers::ENVIRONMENT)
            .with_debug_draw(false);
        
        self.world.add_component(entity, collider.clone());
        self.world.add_component(entity, CollisionStateComponent::default());
        self.ecs_collision_system.register_collider(entity, &collider, &self.world);
        
        self.ships.push(Ship {
            entity,
            velocity,
            size: MONKEY_SIZE,
            collision_radius,
            original_color: color,
            collision_shape_type: CollisionShapeType::Mesh,
            mesh_type: MeshType::Monkey,
        });
    }
    
    /// Calculate rotation quaternion to make ship point in direction of velocity
    /// Assumes ship model's forward direction is +Z axis
    fn rotation_from_velocity(velocity: Vec3) -> rust_engine::foundation::math::Quat {
        if velocity.magnitude() < 0.001 {
            return rust_engine::foundation::math::Quat::identity();
        }
        
        let forward = velocity.normalize();
        let default_forward = Vec3::new(0.0, 0.0, 1.0);
        
        // Calculate rotation axis and angle
        let axis = default_forward.cross(&forward);
        let axis_length = axis.magnitude();
        
        if axis_length < 0.001 {
            // Vectors are parallel or anti-parallel
            if default_forward.dot(&forward) < 0.0 {
                // Anti-parallel: rotate 180 degrees around up axis
                rust_engine::foundation::math::Quat::from_axis_angle(&Vec3::y_axis(), std::f32::consts::PI)
            } else {
                // Parallel: no rotation needed
                rust_engine::foundation::math::Quat::identity()
            }
        } else {
            let axis_normalized = rust_engine::foundation::math::Unit::new_normalize(axis);
            let angle = default_forward.dot(&forward).acos();
            rust_engine::foundation::math::Quat::from_axis_angle(&axis_normalized, angle)
        }
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
                    // Update rotation to point in direction of travel
                    transform.rotation = Self::rotation_from_velocity(ship.velocity);
                    
                    // Collision shape automatically uses new transform - no manual update needed!
                }
            }
            
            // Update octree EVERY frame with current ship positions
            // This ensures collision detection has accurate spatial data
            self.rebuild_octree();
        }
        
        // Use new ECS collision system (replaces old detect_and_handle_collisions)
        let delta_time = 1.0 / 60.0; // Approximate frame time
        self.ecs_collision_system.update(&mut self.world, delta_time);
        
        // Update ship colors based on collision state from ECS collision system
        self.update_ship_colors_from_collision_state();
        
        // Update collision radius visualization using debug visualizer
        self.update_collision_debug_visualization();
        
        // Custom picking with proper collision shapes
        let frame_count = (self.start_time.elapsed().as_secs_f32() * 60.0) as u64;
        self.update_picking_with_mesh_collision(frame_count);
        
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
        // Clear collision system's octree via SpatialQuery interface
        let spatial_query = self.ecs_collision_system.spatial_query_mut();
        spatial_query.clear();
        
        // Reinsert all ships with consistent collision radius
        for ship in &self.ships {
            if let Some(transform) = self.world.get_component::<TransformComponent>(ship.entity) {
                spatial_query.insert(ship.entity, transform.position, ship.collision_radius);
            }
        }
        
        // Update visualization
        self.update_octree_visualization();
        
        // CRITICAL: Sync after creating new visualization entities
        self.scene_manager.sync_from_world(&mut self.world);
    }
    
    /// Custom picking that uses proper mesh/sphere collision shapes
    fn update_picking_with_mesh_collision(&mut self, current_frame: u64) {
        // Pre-collect all shapes since we need world access
        let mut shape_map = std::collections::HashMap::new();
        
        for ship in &self.ships {
            if let Some(collider) = self.world.get_component::<ColliderComponent>(ship.entity) {
                // Shapes are now stored in model-space in ColliderComponent
                shape_map.insert(ship.entity, collider.shape.clone());
            }
        }
        
        // Create shape provider closure that uses the pre-computed map
        let shape_provider = |entity: Entity| -> Option<CollisionShape> {
            shape_map.get(&entity).cloned()
        };
        
        // Use picking system's internal update to handle mouse state
        // Downcast to OctreeSpatialQuery to get the octree
        let spatial_query = self.ecs_collision_system.spatial_query();
        let octree_query = spatial_query.as_any()
            .downcast_ref::<rust_engine::spatial::OctreeSpatialQuery>()
            .expect("SpatialQuery must be OctreeSpatialQuery");
        let octree = octree_query.octree();
        
        self.picking_system.update(
            &mut self.world,
            &self.camera,
            octree,
            &shape_provider,
            current_frame,
        );
    }
    
    /// Update ship colors based on ECS collision state
    fn update_ship_colors_from_collision_state(&mut self) {
        use rust_engine::ecs::components::RenderableComponent;
        
        // Get selected entity from picking system
        let selected = self.picking_system.get_selected();
        
        // Get nearby entities for the selected ship (for cyan coloring)
        let selected_nearby_entities: std::collections::HashSet<Entity> = if let Some(selected_entity) = selected {
            if let Some(collision_state) = self.world.get_component::<CollisionStateComponent>(selected_entity) {
                collision_state.nearby_entities.clone()
            } else {
                std::collections::HashSet::new()
            }
        } else {
            std::collections::HashSet::new()
        };
        
        // Update material colors based on collision state
        // Priority: Selected ship (blue) > Nearby ships to selected (cyan) > Colliding (white) > Original
        for ship in &self.ships {
            // Check selection state
            let is_selected = self.world.get_component::<SelectionComponent>(ship.entity)
                .map_or(false, |sel| sel.selected);
            
            // Get collision state
            let is_colliding = self.world.get_component::<CollisionStateComponent>(ship.entity)
                .map_or(false, |state| state.is_colliding());
            
            // Determine new color based on state
            let new_color = if is_selected {
                Vec3::new(0.0, 0.0, 1.0)  // Selected: BLUE
            } else if selected_nearby_entities.contains(&ship.entity) {
                Vec3::new(0.0, 1.0, 1.0)  // Near selected: CYAN
            } else if is_colliding {
                Vec3::new(1.0, 1.0, 1.0)  // Colliding: WHITE
            } else {
                ship.original_color  // Default color
            };
            
            // Update renderable material
            if let Some(renderable) = self.world.get_component_mut::<RenderableComponent>(ship.entity) {
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
        
        // Update bounding sphere visualization for nearby entities
        self.update_nearby_bounding_spheres(selected, &selected_nearby_entities);
    }
    
    /// Update collision debug visualization using the new debug system
    fn update_collision_debug_visualization(&mut self) {
        // Get selected entity from picking system
        let selected = self.picking_system.get_selected();
        
        // Tell collision system which entity to visualize
        self.ecs_collision_system.set_debug_entity(selected);
    }
    
    /// Render debug collision spheres from the collision system's debug visualizer
    fn render_debug_collision_spheres(&mut self) {
        // Get debug shapes from the collision system's visualizer
        if let Some(viz) = self.ecs_collision_system.debug_visualizer() {
            let shapes = viz.get_shapes();
            
            // Find the broad-phase sphere (if any)
            let mut broad_phase_sphere: Option<(Vec3, f32)> = None;
            for shape in shapes {
                if let rust_engine::debug::DebugShape::Sphere { center, radius, .. } = shape {
                    broad_phase_sphere = Some((*center, *radius));
                    break;
                }
            }
            
            // Update or create the visualization sphere
            if let Some((position, radius)) = broad_phase_sphere {
                if let Some(existing_sphere) = self.collision_radius_sphere {
                    // Update existing sphere position and scale
                    if let Some(sphere_transform) = self.world.get_component_mut::<TransformComponent>(existing_sphere) {
                        sphere_transform.position = position;
                        sphere_transform.scale = Vec3::new(radius, radius, radius);
                    }
                } else {
                    // Create new visualization sphere
                    let sphere_mesh = rust_engine::assets::ObjLoader::load_obj("resources/models/sphere.obj").unwrap();
                    
                    use rust_engine::render::resources::materials::UnlitMaterialParams;
                    let material = Material::transparent_unlit(UnlitMaterialParams {
                        color: Vec3::new(0.5, 0.8, 1.0), // Light blue
                        alpha: 0.15, // Highly transparent
                    });
                    
                    let mut sphere_transform = rust_engine::foundation::math::Transform::from_position_rotation(
                        position,
                        rust_engine::foundation::math::Quat::identity(),
                    );
                    sphere_transform.scale = Vec3::new(radius, radius, radius);
                    
                    let entity = self.scene_manager.create_renderable_entity(
                        &mut self.world,
                        sphere_mesh,
                        rust_engine::render::systems::dynamic::MeshType::Sphere,
                        material,
                        sphere_transform,
                    );
                    
                    self.collision_radius_sphere = Some(entity);
                }
            } else {
                // No debug sphere needed, destroy if it exists
                if let Some(sphere_entity) = self.collision_radius_sphere.take() {
                    self.scene_manager.destroy_entity(
                        &mut self.world,
                        &mut self.graphics_engine,
                        sphere_entity,
                    );
                }
            }
        } else {
            println!("DEBUG: No visualizer found!");
        }
    }
    
    fn update_nearby_bounding_spheres(&mut self, selected: Option<Entity>, nearby_entities: &std::collections::HashSet<Entity>) {
        // Remove spheres for entities that are no longer nearby
        let mut to_remove = Vec::new();
        for (&entity, &sphere_entity) in &self.nearby_bounding_spheres {
            // Don't render sphere for selected entity (it has the query radius sphere)
            // Remove if not in nearby set anymore or is the selected entity
            if !nearby_entities.contains(&entity) || Some(entity) == selected {
                to_remove.push(entity);
                self.scene_manager.destroy_entity(
                    &mut self.world,
                    &mut self.graphics_engine,
                    sphere_entity,
                );
            }
        }
        for entity in to_remove {
            self.nearby_bounding_spheres.remove(&entity);
        }
        
        // Create or update spheres for nearby entities
        for &entity in nearby_entities {
            // Skip selected entity (it has its own query radius sphere)
            if Some(entity) == selected {
                continue;
            }
            
            // Get entity's transform and collider
            let (position, bounding_radius) = if let Some(transform) = self.world.get_component::<TransformComponent>(entity) {
                if let Some(collider) = self.world.get_component::<ColliderComponent>(entity) {
                    (transform.position, collider.bounding_radius)
                } else {
                    continue;
                }
            } else {
                continue;
            };
            
            // Update or create sphere
            if let Some(&sphere_entity) = self.nearby_bounding_spheres.get(&entity) {
                // Update existing sphere
                if let Some(sphere_transform) = self.world.get_component_mut::<TransformComponent>(sphere_entity) {
                    sphere_transform.position = position;
                    sphere_transform.scale = Vec3::new(bounding_radius, bounding_radius, bounding_radius);
                }
            } else {
                // Create new sphere
                let sphere_mesh = rust_engine::assets::ObjLoader::load_obj("resources/models/sphere.obj").unwrap();
                
                use rust_engine::render::resources::materials::UnlitMaterialParams;
                let material = Material::transparent_unlit(UnlitMaterialParams {
                    color: Vec3::new(0.0, 1.0, 1.0), // Cyan
                    alpha: 0.2, // Semi-transparent
                });
                
                let mut sphere_transform = rust_engine::foundation::math::Transform::from_position_rotation(
                    position,
                    rust_engine::foundation::math::Quat::identity(),
                );
                sphere_transform.scale = Vec3::new(bounding_radius, bounding_radius, bounding_radius);
                
                let sphere_entity = self.scene_manager.create_renderable_entity(
                    &mut self.world,
                    sphere_mesh,
                    rust_engine::render::systems::dynamic::MeshType::Sphere,
                    material,
                    sphere_transform,
                );
                
                self.nearby_bounding_spheres.insert(entity, sphere_entity);
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
        
        // Get all leaf nodes from collision system's octree
        // Downcast to OctreeSpatialQuery to access octree
        let spatial_query = self.ecs_collision_system.spatial_query();
        let octree_query = spatial_query.as_any()
            .downcast_ref::<rust_engine::spatial::OctreeSpatialQuery>()
            .expect("SpatialQuery must be OctreeSpatialQuery");
        let octree = octree_query.octree();
        let leaves = octree.get_all_leaves();
        
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
        
        // Render debug collision spheres if visualizer is enabled
        self.render_debug_collision_spheres();
        
        // Build render queue (scene_manager already tracks entities via create_renderable_entity)
        let render_queue = self.scene_manager.build_render_queue();
        
        // DEBUG: Print render queue info
        use std::sync::atomic::{AtomicU32, Ordering};
        static FRAME_COUNT: AtomicU32 = AtomicU32::new(0);
        let _frame = FRAME_COUNT.fetch_add(1, Ordering::Relaxed);
        
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
                    // Update picking system with mouse position
                    self.picking_system.update_mouse(x, y, false);
                }
                WindowEvent::MouseButton(glfw::MouseButton::Button1, Action::Press, _) => {
                    // Left mouse button clicked - use current mouse position and set click flag
                    // Don't update position, just set the click flag using the last known position
                    let (current_x, current_y) = self.picking_system.get_mouse_position();
                    self.picking_system.update_mouse(current_x, current_y, true);
                    
                    println!("Mouse clicked at ({:.2}, {:.2})", current_x, current_y);
                }
                WindowEvent::FramebufferSize(width, height) => {
                    framebuffer_resized = true;
                    if width > 0 && height > 0 {
                        self.camera.set_aspect_ratio(width as f32 / height as f32);
                        // Update UI screen size
                        self.ui_manager.set_screen_size(width as f32, height as f32);
                        // Update picking system window size
                        self.picking_system.update_window_size(width as u32, height as u32);
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
