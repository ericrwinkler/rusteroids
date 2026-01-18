//! Turret demo showcasing hierarchical animations and targeting AI

#![allow(dead_code)]

use rust_engine::assets::{ObjLoader, MaterialBuilder, ImageData};
use rust_engine::render::resources::materials::{Material, UnlitMaterialParams, TextureType};
use rust_engine::ecs::{
    World, Entity, LightFactory, LightingSystem as EcsLightingSystem,
};
use rust_engine::ecs::components::TransformComponent;
use rust_engine::scene::SceneManager;
use rust_engine::render::{
    Camera,
    Mesh,
    GraphicsEngine,
    VulkanRendererConfig,
    WindowHandle,
    systems::dynamic::MeshType,
    FontAtlas,
};
use rust_engine::foundation::math::{Vec3, Quat};
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;

// Turret configuration
const TURRET_RECOIL_DISTANCE: f32 = 0.3;  // How far barrel moves back
const TURRET_RECOIL_SPEED: f32 = 5.0;     // Speed of recoil snap-back
const TURRET_RETURN_SPEED: f32 = 2.0;     // Speed barrel returns forward
const TURRET_ROTATION_SPEED: f32 = 1.0;   // Radians per second turret rotation
const FIRE_INTERVAL: f32 = 2.0;            // Seconds between shots
// Barrel mount point relative to base: Vec3::new(0.0, 0.5, -1.2)

struct TurretBase {
    entity: Entity,
    position: Vec3,
    rotation_y: f32,  // Current Y-axis rotation (yaw)
    target_rotation_y: f32,  // Target Y-axis rotation for smooth turning
}

struct TurretBarrel {
    entity: Entity,
    parent_base: Entity,  // Reference to parent turret base
    recoil_offset: f32,   // Current recoil position on local Z axis
    is_recoiling: bool,   // Whether actively recoiling
    pitch: f32,           // Current pitch angle (up/down aim)
    target_pitch: f32,    // Target pitch for smooth aiming
}

struct Target {
    entity: Entity,
    position: Vec3,
    velocity: Vec3,
}

pub struct TurretDemoApp {
    window: WindowHandle,
    graphics_engine: GraphicsEngine,
    camera: Camera,
    scene_manager: SceneManager,
    world: World,
    lighting_system: EcsLightingSystem,
    
    // Meshes
    turret_base_mesh: Option<Mesh>,
    turret_barrel_mesh: Option<Mesh>,
    sphere_mesh: Option<Mesh>,
    frigate_mesh: Option<Mesh>,
    monkey_mesh: Option<Mesh>,
    
    // Materials
    turret_base_material: Option<Material>,
    frigate_material: Option<Material>,
    
    // Entities
    frigate_entity: Option<Entity>,
    planet_entity: Option<Entity>,
    turret_base: Option<TurretBase>,
    turret_barrel: Option<TurretBarrel>,
    standalone_turret_base: Option<Entity>,
    standalone_turret_barrel: Option<Entity>,
    monkey_entity: Option<Entity>,
    targets: Vec<Target>,
    sunlight_entity: Option<Entity>,
    skybox_entity: Option<Entity>,
    
    // Orbit state
    orbit_angle: f32,
    orbit_radius: f32,
    orbit_speed: f32,
    
    // Targeting
    current_target_index: usize,
    fire_timer: f32,
    
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
    camera_boost: bool,  // Shift key for faster movement
    
    // Simulation control
    paused: bool,
    
    // Timing
    start_time: Instant,
    last_frame_time: Instant,
    
    // FPS tracking
    fps_update_timer: Instant,
    frame_count: u32,
    current_fps: f32,
    
    // UI System
    ui_manager: rust_engine::ui::UIManager,
    fps_label_id: rust_engine::ui::UINodeId,
    
    // Restored automatic behavior - no manual controls needed
}

impl TurretDemoApp {
    pub fn new() -> Self {
        log::info!("Creating turret demo application...");
        
        // Create window
        let mut window = WindowHandle::new(1200, 800, "Rusteroids - Turret Demo");
        
        // Create renderer
        let renderer_config = VulkanRendererConfig::new("Rusteroids - Turret Demo")
            .with_version(1, 0, 0)
            .with_shader_paths(
                "target/shaders/standard_pbr_vert.spv".to_string(),
                "target/shaders/standard_pbr_frag.spv".to_string()
            );
        let mut graphics_engine = GraphicsEngine::new_from_window(&mut window, &renderer_config)
            .expect("Failed to create graphics engine");
        
        // Initialize UI text rendering system
        graphics_engine.initialize_ui_text_system()
            .expect("Failed to initialize UI text system");
        
        // Initialize UI screen size
        graphics_engine.update_ui_screen_size(1200.0, 800.0);
        
        // Initialize dynamic object system
        graphics_engine.initialize_dynamic_system(100)
            .expect("Failed to initialize dynamic object system");
        
        // Create camera - positioned to view planet and orbiting frigate
        let camera_position = Vec3::new(40.0, 50.0, 80.0);  // Higher and further back to see orbit
        let planet_position = Vec3::new(0.0, 0.0, 0.0);
        let mut camera = Camera::perspective(
            camera_position,
            60.0,
            1200.0 / 800.0,
            0.1,
            1000.0
        );
        camera.set_position(camera_position);
        camera.look_at(planet_position, Vec3::new(0.0, 1.0, 0.0));
        
        // Initialize SceneManager
        let scene_manager = SceneManager::new();
        
        // Initialize ECS for lighting
        let mut world = World::new();
        let lighting_system = EcsLightingSystem::new();
        
        // Create sunlight
        let sunlight_entity = Some(Self::create_sunlight(&mut world));
        
        // Initialize UI Manager
        let mut ui_manager = rust_engine::ui::UIManager::new();
        ui_manager.set_screen_size(1200.0, 800.0);
        
        // Create font atlas for UI
        let font_path = "resources/fonts/default.ttf";
        let font_data = std::fs::read(font_path)
            .expect(&format!("Failed to read font file: {}", font_path));
        let mut font_atlas = FontAtlas::new(&font_data, 24.0)
            .expect("Failed to create font atlas");
        font_atlas.rasterize_glyphs()
            .expect("Failed to rasterize glyphs");
        ui_manager.initialize_font_atlas(font_atlas);
        
        // Create UI elements
        use rust_engine::ui::{UIText, Anchor};
        use rust_engine::foundation::math::Vec4;
        
        // FPS counter
        let fps_text = UIText {
            element: rust_engine::ui::UIElement {
                position: (10.0, 30.0),
                size: (0.0, 0.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "FPS: 0.0".to_string(),
            font_size: 24.0,
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            h_align: rust_engine::ui::HorizontalAlign::Left,
            v_align: rust_engine::ui::VerticalAlign::Top,
        };
        let fps_label_id = ui_manager.add_text(fps_text);
        
        let start_time = Instant::now();
        
        // Calculate camera yaw and pitch AFTER look_at to match the view direction
        // From position (15, 10, 22) looking at origin (0, 0, 0)
        // Direction vector: (0, 0, 0) - (15, 10, 22) = (-15, -10, -22)
        let look_direction = Vec3::new(-camera_position.x, -camera_position.y, -camera_position.z).normalize();
        let camera_yaw = look_direction.z.atan2(look_direction.x);
        let horizontal_dist = (look_direction.x * look_direction.x + look_direction.z * look_direction.z).sqrt();
        let camera_pitch = look_direction.y.atan2(horizontal_dist);
        
        Self {
            window,
            graphics_engine,
            camera,
            scene_manager,
            world,
            lighting_system,
            turret_base_mesh: None,
            turret_barrel_mesh: None,
            sphere_mesh: None,
            frigate_mesh: None,
            monkey_mesh: None,
            turret_base_material: None,
            frigate_material: None,
            frigate_entity: None,
            planet_entity: None,
            turret_base: None,
            turret_barrel: None,
            standalone_turret_base: None,
            standalone_turret_barrel: None,
            monkey_entity: None,
            targets: Vec::new(),
            sunlight_entity,
            skybox_entity: None,
            orbit_angle: 0.0,
            orbit_radius: 160.0,  // Further from planet
            orbit_speed: 0.05,
            current_target_index: 0,
            fire_timer: 0.0,
            fps_update_timer: Instant::now(),
            frame_count: 0,
            current_fps: 60.0,
            last_frame_time: Instant::now(),
            ui_manager,
            fps_label_id,
            start_time,
            camera_move_forward: false,
            camera_move_backward: false,
            camera_move_left: false,
            camera_move_right: false,
            camera_yaw_left: false,
            camera_yaw_right: false,
            camera_pitch_up: false,
            camera_pitch_down: false,
            camera_yaw,
            camera_pitch,
            camera_boost: false,
            paused: false,
        }
    }
    
    fn create_sunlight(world: &mut World) -> Entity {
        let sunlight_entity = world.create_entity();
        // Light coming from slightly above and to the side
        let direction = Vec3::new(-1.0, -0.9, -1.6).normalize();
        let sunlight = LightFactory::directional(
            direction,
            Vec3::new(1.0, 1.0, 1.0),  // Full white light
            1.2                         // Higher intensity to compensate for PBR energy conservation
        );
        world.add_component(sunlight_entity, sunlight);
        log::info!("Created directional sunlight");
        sunlight_entity
    }
    
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing turret demo...");
        
        // Load turret base mesh
        match ObjLoader::load_obj("resources/models/simple_turret_base.obj") {
            Ok(mesh) => {
                log::info!("Loaded turret base mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.turret_base_mesh = Some(mesh);
            }
            Err(e) => {
                log::error!("Failed to load simple_turret_base.obj: {:?}", e);
                return Err(e.into());
            }
        }
        
        // Load turret barrel mesh
        match ObjLoader::load_obj("resources/models/simple_turret_barrel.obj") {
            Ok(mesh) => {
                log::info!("Loaded turret barrel mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.turret_barrel_mesh = Some(mesh);
            }
            Err(e) => {
                log::error!("Failed to load simple_turret_barrel.obj: {:?}", e);
                return Err(e.into());
            }
        }
        
        // Load sphere mesh for targets
        match ObjLoader::load_obj("resources/models/sphere.obj") {
            Ok(mesh) => {
                log::info!("Loaded sphere mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.sphere_mesh = Some(mesh);
            }
            Err(e) => {
                log::error!("Failed to load sphere.obj: {:?}", e);
                return Err(e.into());
            }
        }

        // Load frigate mesh
        match ObjLoader::load_obj("resources/models/frigate.obj") {
            Ok(mesh) => {
                log::info!("Loaded frigate mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.frigate_mesh = Some(mesh);
            }
            Err(e) => {
                log::error!("Failed to load frigate.obj: {:?}", e);
                return Err(e.into());
            }
        }

        // Load monkey mesh for testing lighting
        match ObjLoader::load_obj("resources/models/monkey.obj") {
            Ok(mesh) => {
                log::info!("Loaded monkey mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.monkey_mesh = Some(mesh);
            }
            Err(e) => {
                log::error!("Failed to load monkey.obj: {:?}", e);
                return Err(e.into());
            }
        }
        
        // Create mesh pools with materials
        // Load specific textures for each object type
        use rust_engine::assets::ImageData;
        use rust_engine::render::resources::materials::TextureType;

        // Load frigate textures
        let frigate_base_image = ImageData::from_file("resources/textures/Frigate_texture.png")
            .map_err(|e| format!("Failed to load frigate base texture: {}", e))?;
        let frigate_emission_image = ImageData::from_file("resources/textures/Frigate_emission.png")
            .map_err(|e| format!("Failed to load frigate emission texture: {}", e))?;

        let frigate_base_handle = self.graphics_engine.upload_texture_from_image_data(
            frigate_base_image,
            TextureType::BaseColor
        )?;
        let frigate_emission_handle = self.graphics_engine.upload_texture_from_image_data(
            frigate_emission_image,
            TextureType::Emission
        )?;

        // Load turret textures
        let turret_base_image = ImageData::from_file("resources/textures/simple_turret_base_texture.png")
            .map_err(|e| format!("Failed to load turret base texture: {}", e))?;
        let turret_normal_image = ImageData::from_file("resources/textures/simple_turret_normal_map.png")
            .map_err(|e| format!("Failed to load turret normal map: {}", e))?;

        let turret_base_handle = self.graphics_engine.upload_texture_from_image_data(
            turret_base_image,
            TextureType::BaseColor
        )?;
        let turret_normal_handle = self.graphics_engine.upload_texture_from_image_data(
            turret_normal_image,
            TextureType::Normal
        )?;
        
        // Frigate mesh loaded above - no need for assignment here
        if let Some(ref frigate_mesh) = self.frigate_mesh {
            log::info!("Using frigate mesh with {} vertices and {} indices", 
                      frigate_mesh.vertices.len(), frigate_mesh.indices.len());
        } else {
            log::error!("Frigate mesh not loaded");
            return Err("Frigate mesh not available".into());
        }

        // Create frigate pool with frigate mesh and textures
        if let Some(ref frigate_mesh) = self.frigate_mesh {
            let frigate_material = MaterialBuilder::new()
                .base_color_rgb(1.0, 1.0, 1.0)  // White to show texture
                .metallic(0.6)
                .roughness(0.3)
                .emission_rgb(1.0, 1.0, 1.0)
                .emission_strength(2.0)
                .name("Frigate Material")
                .build()
                .with_base_color_texture(frigate_base_handle)
                .with_emission_texture(frigate_emission_handle);
            
            self.graphics_engine.create_mesh_pool(
                MeshType::Frigate,
                frigate_mesh,
                &[frigate_material.clone()],
                5
            )?;
            
            // Store the material for entity creation
            self.frigate_material = Some(frigate_material);
            
            log::info!("Created frigate pool with textures");
        }

        // Create turret base pool with turret-specific textures
        if let Some(ref base_mesh) = self.turret_base_mesh {
            let turret_base_material = MaterialBuilder::new()
                .base_color_rgb(1.0, 1.0, 1.0)  // Let texture define colors
                .metallic(0.8)
                .roughness(0.3)
                .name("Turret Base")
                .build()
                .with_base_color_texture(turret_base_handle)
                .with_normal_texture(turret_normal_handle);
            
            // Store material for instance creation
            self.turret_base_material = Some(turret_base_material.clone());
            
            // Debug: Check material texture attachments
            log::info!("DEBUG: Turret base material textures - base_color: {:?}, normal: {:?}, emission: {:?}", 
                       turret_base_material.textures.base_color.is_some(),
                       turret_base_material.textures.normal.is_some(),
                       turret_base_material.textures.emission.is_some());
                       
            // Debug: Check material params texture flags
            if let rust_engine::render::resources::materials::MaterialType::StandardPBR(params) = &turret_base_material.material_type {
                log::info!("DEBUG: Turret base material params texture flags - base_color_enabled: {}, normal_enabled: {}", 
                           params.base_color_texture_enabled,
                           params.normal_texture_enabled);
            }
            
            self.graphics_engine.create_mesh_pool(
                MeshType::TurretBase,
                base_mesh,
                &[turret_base_material],
                5
            )?;
            log::info!("Created turret base mesh pool with normal maps");
        }

        if let Some(ref barrel_mesh) = self.turret_barrel_mesh {
            let turret_barrel_material = MaterialBuilder::new()
                .base_color_rgb(0.3, 0.3, 0.35)
                .metallic(0.9)
                .roughness(0.2)
                .name("Turret Barrel")
                .build();

            self.graphics_engine.create_mesh_pool(
                MeshType::TurretBarrel,
                barrel_mesh,
                &[turret_barrel_material],
                5
            )?;
            log::info!("Created turret barrel mesh pool");
        }

        if let Some(ref sphere_mesh) = self.sphere_mesh {
            let target_material = Material::transparent_unlit(UnlitMaterialParams {
                color: Vec3::new(1.0, 0.5, 0.2),
                alpha: 0.9,
            })
            .with_name("Target");
            
            self.graphics_engine.create_mesh_pool(
                MeshType::Sphere,
                sphere_mesh,
                &[target_material],
                10
            )?;
            log::info!("Created target mesh pool");
        }

        // Create monkey mesh pool
        if let Some(ref monkey_mesh) = self.monkey_mesh {
            let monkey_material = MaterialBuilder::new()
                .base_color_rgb(0.8, 0.6, 0.4)  // Brown monkey
                .metallic(0.1)
                .roughness(0.7)
                .name("Monkey")
                .build();
            
            self.graphics_engine.create_mesh_pool(
                MeshType::Monkey,
                monkey_mesh,
                &[monkey_material],
                5
            )?;
            log::info!("Created monkey mesh pool for lighting test");
        }
        
        /*
        // Create skybox
        let skybox_mesh = Mesh::skybox();
        
        let skybox_image = ImageData::from_file("resources/textures/skybox.png")
            .map_err(|e| format!("Failed to load skybox texture: {}", e))?;
        
        let skybox_texture_handle = self.graphics_engine.upload_texture_from_image_data(
            skybox_image,
            TextureType::BaseColor
        )?;
        
        let skybox_material = Material::skybox(UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        })
        .with_name("Skybox")
        .with_base_color_texture(skybox_texture_handle);
        
        self.graphics_engine.create_mesh_pool(
            MeshType::Cube,
            &skybox_mesh,
            &[skybox_material.clone()],
            1
        )?;
        
        // Spawn skybox entity
        use rust_engine::foundation::math::Transform;
        let skybox_transform = Transform {
            position: self.camera.position,
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        let skybox_entity = self.scene_manager.create_renderable_entity_with_layer(
            &mut self.world,
            skybox_mesh,
            MeshType::Cube,
            skybox_material,
            skybox_transform,
            255,
        );
        self.skybox_entity = Some(skybox_entity);
        */

        // Create skybox
        self.spawn_skybox()?;
        
        // Spawn planet
        self.spawn_planet()?;
        
        // Spawn frigate orbiting planet
        self.spawn_frigate()?;
        
        // Restore working turret spawning
        self.spawn_turret()?;
        
        // Spawn some targets
        self.spawn_targets()?;
        
        Ok(())
    }

    fn spawn_skybox(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        use rust_engine::assets::ImageData;
        use rust_engine::render::resources::materials::TextureType;
        
        // Create skybox
        let skybox_mesh = Mesh::skybox();
        
        // Load skybox texture
        let skybox_image = ImageData::from_file("resources/textures/skybox.png")
            .map_err(|e| format!("Failed to load skybox texture: {}", e))?;
        
        let skybox_texture_handle = self.graphics_engine.upload_texture_from_image_data(
            skybox_image,
            TextureType::BaseColor
        )?;
        
        // Create skybox material
        let skybox_material = Material::skybox(UnlitMaterialParams {
            color: Vec3::new(1.0, 1.0, 1.0),
            alpha: 1.0,
        })
        .with_name("Skybox")
        .with_base_color_texture(skybox_texture_handle);
        
        // Create skybox mesh pool
        if let Err(e) = self.graphics_engine.create_mesh_pool(
            MeshType::Cube,
            &skybox_mesh,
            &[skybox_material.clone()],
            1
        ) {
            log::error!("Failed to create skybox mesh pool: {}", e);
            return Err(e);
        }
        
        // Spawn skybox entity (follows camera)
        let skybox_transform = Transform {
            position: self.camera.position,
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        let skybox_entity = self.scene_manager.create_renderable_entity_with_layer(
            &mut self.world,
            skybox_mesh,
            MeshType::Cube,
            skybox_material,
            skybox_transform,
            255,  // Skybox layer (renders after opaque, before transparent)
        );
        self.skybox_entity = Some(skybox_entity);
        
        Ok(())
    }
    
    fn spawn_planet(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        
        let planet_position = Vec3::new(0.0, 0.0, 0.0);  // Center of scene
        
        let planet_material = MaterialBuilder::new()
            .base_color_rgb(0.3, 0.5, 0.8)
            .metallic(0.1)
            .roughness(0.8)
            .name("Planet Instance")
            .build();
        
        let planet_transform = Transform {
            position: planet_position,
            rotation: Quat::identity(),
            scale: Vec3::new(50.0, 50.0, 50.0),  // Bigger planet
        };
        
        let planet_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.sphere_mesh.as_ref().unwrap().clone(),
            MeshType::Sphere,
            planet_material,
            planet_transform
        );
        
        self.planet_entity = Some(planet_entity);
        log::info!("Spawned planet at origin");
        Ok(())
    }
    
    fn spawn_frigate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        
        // Start frigate at orbit position
        let orbit_x = self.orbit_radius * self.orbit_angle.cos();
        let orbit_z = self.orbit_radius * self.orbit_angle.sin();
        let frigate_position = Vec3::new(orbit_x, 0.0, orbit_z);
        
        // Load frigate material
        // Use the textured material from pool creation (like turret base does)
        let frigate_material = self.frigate_material.as_ref()
            .expect("Frigate material not initialized")
            .clone();
        
        let frigate_transform = Transform {
            position: frigate_position,
            rotation: Quat::from_euler_angles(0.0, -self.orbit_angle, 0.0),
            scale: Vec3::new(3.0, 3.0, 3.0),
        };
        
        let frigate_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.frigate_mesh.as_ref().unwrap().clone(),
            MeshType::Frigate,
            frigate_material,
            frigate_transform
        );
        
        self.frigate_entity = Some(frigate_entity);
        log::info!("Spawned frigate in orbit");
        Ok(())
    }
    
    fn spawn_turret(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        
        // Turret base position is relative to frigate
        // Place it center-top of frigate (raised higher)
        let turret_offset_from_frigate = Vec3::new(0.0, 3.5, 0.0);  // Moved down 0.5 units
        let (frigate_position, frigate_rotation) = if let Some(entity) = self.frigate_entity {
            let position = self.world.get_component::<TransformComponent>(entity)
                .map(|t| t.position)
                .unwrap_or(Vec3::new(0.0, 0.0, 0.0));
            let rotation = self.world.get_component::<TransformComponent>(entity)
                .map(|t| t.rotation)
                .unwrap_or(Quat::identity());
            (position, rotation)
        } else {
            (Vec3::new(0.0, 0.0, 0.0), Quat::identity())
        };
        let base_position = frigate_position + frigate_rotation * turret_offset_from_frigate;
        
        // Create turret base entity with frigate's rotation
        let base_transform = Transform {
            position: base_position,
            rotation: frigate_rotation,  // Use frigate's rotation instead of identity
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        // Use the textured material from pool creation
        let turret_base_material = self.turret_base_material.as_ref()
            .expect("Turret base material not initialized")
            .clone();
        
        let base_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.turret_base_mesh.as_ref().unwrap().clone(),
            MeshType::TurretBase,
            turret_base_material,
            base_transform
        );
        
        self.turret_base = Some(TurretBase {
            entity: base_entity,
            position: base_position,
            rotation_y: 0.0,
            target_rotation_y: 0.0,
        });
        
        // Create turret barrel entity as child
        // Barrel is positioned relative to base (slightly up and forward)
        let barrel_local_position = Vec3::new(0.0, 0.5, -1.2);  // Keep in sync with update_scene!
        let barrel_world_position = base_position + frigate_rotation * barrel_local_position;
        
        let barrel_transform = Transform {
            position: barrel_world_position,
            rotation: frigate_rotation,  // Use frigate's rotation instead of identity
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        let turret_barrel_material = MaterialBuilder::new()
            .base_color_rgb(0.3, 0.3, 0.35)
            .metallic(0.9)
            .roughness(0.2)
            .name("Turret Barrel Instance")
            .build();
        
        let barrel_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.turret_barrel_mesh.as_ref().unwrap().clone(),
            MeshType::TurretBarrel,
            turret_barrel_material,
            barrel_transform
        );
        
        self.turret_barrel = Some(TurretBarrel {
            entity: barrel_entity,
            parent_base: base_entity,
            recoil_offset: 0.0,
            is_recoiling: false,
            pitch: 0.0,
            target_pitch: 0.3,  // Aim slightly upward (about 17 degrees)
        });
        
        log::info!("Spawned turret with base and barrel");
        Ok(())
    }
    
    fn spawn_targets(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        
        let target_material = Material::transparent_unlit(UnlitMaterialParams {
            color: Vec3::new(1.0, 0.5, 0.2),
            alpha: 0.9,
        })
        .with_name("Target Instance");
        
        // Spawn 3 targets with random velocities
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Get current frigate position for spawning targets nearby
        let frigate_pos = Vec3::new(
            self.orbit_radius * self.orbit_angle.cos(),
            0.0,
            self.orbit_radius * self.orbit_angle.sin()
        );
        
        for _i in 0..3 {
            // Spawn at random position near frigate
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(20.0..40.0);  // Closer to frigate
            let height = rng.gen_range(5.0..15.0);     // Higher up
            
            let position = frigate_pos + Vec3::new(
                angle.cos() * distance,
                height,
                angle.sin() * distance,
            );
            
            // Random velocity - either flying past turret or perpendicular
            let velocity_type = rng.gen_range(0..2);
            let speed = rng.gen_range(3.0..7.0);
            
            let velocity = if velocity_type == 0 {
                // Flying toward/past turret center
                let to_center = Vec3::new(0.0, 0.0, 0.0) - position;
                let perpendicular = Vec3::new(-to_center.z, 0.0, to_center.x).normalize();
                perpendicular * speed
            } else {
                // Flying perpendicular to turret
                let direction = Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-0.3..0.3),
                    rng.gen_range(-1.0..1.0),
                ).normalize();
                direction * speed
            };
            
            let transform = Transform {
                position,
                rotation: Quat::identity(),
                scale: Vec3::new(0.5, 0.5, 0.5),
            };
            
            let entity = self.scene_manager.create_renderable_entity(
                &mut self.world,
                self.sphere_mesh.as_ref().unwrap().clone(),
                MeshType::Sphere,
                target_material.clone(),
                transform
            );
            
            self.targets.push(Target {
                entity,
                position,
                velocity,
            });
        }
        
        log::info!("Spawned {} targets with random velocities", self.targets.len());
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting turret demo...");
        self.initialize()?;
        
        let mut framebuffer_resized = false;
        
        log::info!("Entering main loop");
        while !self.window.should_close() {
            self.window.poll_events();
            
            // Handle events
            let events: Vec<_> = self.window.event_iter().collect();
            
            for (_, event) in events {
                match event {
                    WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
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
                    WindowEvent::Key(Key::LeftShift, _, action, _) | WindowEvent::Key(Key::RightShift, _, action, _) => {
                        self.camera_boost = action == Action::Press || action == Action::Repeat;
                    }
                    // Manual controls removed - using automatic behavior
                    WindowEvent::FramebufferSize(width, height) => {
                        framebuffer_resized = true;
                        if width > 0 && height > 0 {
                            self.camera.set_aspect_ratio(width as f32 / height as f32);
                            self.ui_manager.set_screen_size(width as f32, height as f32);
                        }
                    }
                    WindowEvent::CursorPos(x, y) => {
                        self.ui_manager.update_mouse_position(x as f32, y as f32);
                    }
                    WindowEvent::MouseButton(button, action, _) => {
                        use rust_engine::ui::MouseButton as UIMouseButton;
                        let ui_button = match button {
                            glfw::MouseButton::Button1 => UIMouseButton::Left,
                            glfw::MouseButton::Button2 => UIMouseButton::Right,
                            glfw::MouseButton::Button3 => UIMouseButton::Middle,
                            _ => continue,
                        };
                        let pressed = action == Action::Press;
                        self.ui_manager.update_mouse_button(ui_button, pressed);
                    }
                    _ => {}
                }
            }
            
            if framebuffer_resized {
                self.graphics_engine.recreate_swapchain(&mut self.window);
                let (width, height) = self.graphics_engine.get_swapchain_extent();
                if width > 0 && height > 0 {
                    self.camera.set_aspect_ratio(width as f32 / height as f32);
                }
                framebuffer_resized = false;
            }
            
            // Update scene
            self.update_scene();
            
            // Render frame
            if let Err(e) = self.render_frame() {
                log::error!("Failed to render frame: {:?}", e);
                break;
            }
        }
        
        log::info!("Turret demo completed");
        Ok(())
    }
    
    fn update_scene(&mut self) {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Apply camera movement (always active, even when paused)
        let base_camera_speed = 5.0;
        let camera_speed = if self.camera_boost { base_camera_speed * 3.0 } else { base_camera_speed };
        let camera_rotation_speed = 1.5;
        
        let forward = Vec3::new(
            self.camera_yaw.cos() * self.camera_pitch.cos(),
            self.camera_pitch.sin(),
            self.camera_yaw.sin() * self.camera_pitch.cos(),
        ).normalize();
        let right = Vec3::new(
            (self.camera_yaw + std::f32::consts::FRAC_PI_2).cos(),
            0.0,
            (self.camera_yaw + std::f32::consts::FRAC_PI_2).sin(),
        ).normalize();
        
        let mut camera_pos = self.camera.position;
        if self.camera_move_forward { camera_pos = camera_pos + forward * camera_speed * delta_time; }
        if self.camera_move_backward { camera_pos = camera_pos - forward * camera_speed * delta_time; }
        if self.camera_move_left { camera_pos = camera_pos - right * camera_speed * delta_time; }
        if self.camera_move_right { camera_pos = camera_pos + right * camera_speed * delta_time; }
        
        if self.camera_yaw_left { self.camera_yaw -= camera_rotation_speed * delta_time; }
        if self.camera_yaw_right { self.camera_yaw += camera_rotation_speed * delta_time; }
        if self.camera_pitch_up { self.camera_pitch += camera_rotation_speed * delta_time; }
        if self.camera_pitch_down { self.camera_pitch -= camera_rotation_speed * delta_time; }
        
        self.camera_pitch = self.camera_pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.1, std::f32::consts::FRAC_PI_2 - 0.1);
        
        let look_at_point = camera_pos + forward;
        self.camera.set_position(camera_pos);
        self.camera.look_at(look_at_point, Vec3::new(0.0, 1.0, 0.0));
        
        // Update skybox to follow camera
        if let Some(skybox_entity) = self.skybox_entity {
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(skybox_entity) {
                transform.position = camera_pos;
            }
        }
        
        // Pause check - only affects simulation, not camera
        if self.paused {
            // Still sync entities even when paused for camera-dependent rendering
            self.scene_manager.sync_from_world(&mut self.world);
            return;
        }
        
        // Restore automatic orbit angle update
        self.orbit_angle += self.orbit_speed * delta_time;
        
        // Restore automatic frigate orbital movement
        if let Some(frigate_entity) = self.frigate_entity {
            // Calculate orbital position
            let frigate_position = Vec3::new(
                self.orbit_radius * self.orbit_angle.cos(),
                0.0,
                self.orbit_radius * self.orbit_angle.sin()
            );
            
            // Point frigate forward in its orbital direction  
            let frigate_rotation = Quat::from_euler_angles(0.0, -self.orbit_angle, 0.0);
            
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(frigate_entity) {
                transform.position = frigate_position;
                transform.rotation = frigate_rotation;
            }
            
            // Update turret base and barrel to follow frigate
            let turret_offset_from_frigate = Vec3::new(0.0, 4.5, 0.0);  // Moved down 0.5 units
            let turret_base_world_pos = frigate_position + (frigate_rotation * turret_offset_from_frigate);
            
            if let Some(ref mut turret_base) = self.turret_base {
                turret_base.position = turret_base_world_pos;
                
                if let Some(transform) = self.world.get_component_mut::<TransformComponent>(turret_base.entity) {
                    // Combine frigate rotation with turret targeting rotation
                    let turret_targeting_rotation = Quat::from_euler_angles(0.0, turret_base.rotation_y, 0.0);
                    transform.rotation = frigate_rotation * turret_targeting_rotation;  // Combine rotations
                    transform.position = turret_base_world_pos;
                }
            }
        }
        
        // Update targets (linear motion)
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        for (i, target) in self.targets.iter_mut().enumerate() {
            target.position = target.position + target.velocity * delta_time;
            
            // Update target color based on whether it's currently targeted
            let is_targeted = i == self.current_target_index;
            let target_color = if is_targeted {
                Vec3::new(1.0, 0.2, 0.2)  // Red for current target
            } else {
                Vec3::new(0.2, 1.0, 0.2)  // Green for other targets
            };
            
            // Update renderable material color
            if let Some(renderable) = self.world.get_component_mut::<rust_engine::ecs::components::RenderableComponent>(target.entity) {
                renderable.material = Material::transparent_unlit(UnlitMaterialParams {
                    color: target_color,
                    alpha: 0.9,
                })
                .with_name("Target Instance");
            }
            
            // Respawn target if it gets too far from frigate
            let current_frigate_pos = Vec3::new(
                self.orbit_radius * self.orbit_angle.cos(),
                0.0,
                self.orbit_radius * self.orbit_angle.sin()
            );
            let distance_from_frigate = (target.position - current_frigate_pos).magnitude();
            if distance_from_frigate > 60.0 {  // Increased threshold since frigate is further out
                // Respawn at new random position near frigate
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let distance = rng.gen_range(20.0..40.0);  // Match initial spawn distance
                let height = rng.gen_range(5.0..15.0);     // Match initial spawn height
                
                target.position = current_frigate_pos + Vec3::new(
                    angle.cos() * distance,
                    height,
                    angle.sin() * distance,
                );
                
                let velocity_type = rng.gen_range(0..2);
                let speed = rng.gen_range(3.0..7.0);
                
                target.velocity = if velocity_type == 0 {
                    // Flying toward/past frigate
                    let to_frigate = current_frigate_pos - target.position;
                    let perpendicular = Vec3::new(-to_frigate.z, 0.0, to_frigate.x).normalize();
                    perpendicular * speed
                } else {
                    let direction = Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(-0.3..0.3),
                        rng.gen_range(-1.0..1.0),
                    ).normalize();
                    direction * speed
                };
            }
            
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(target.entity) {
                transform.position = target.position;
            }
        }
        
        // Restored automatic turret targeting system
        if !self.targets.is_empty() && self.turret_base.is_some() && self.turret_barrel.is_some() {
            let turret_base = self.turret_base.as_mut().unwrap();
            let turret_barrel = self.turret_barrel.as_mut().unwrap();
            let current_target = &self.targets[self.current_target_index];
            
            // Get frigate rotation first for turret targeting calculations
            let frigate_rotation = if let Some(frigate_entity) = self.frigate_entity {
                self.world.get_component::<TransformComponent>(frigate_entity)
                    .map(|t| t.rotation)
                    .unwrap_or(Quat::identity())
            } else {
                Quat::identity()
            };
            
            // Extract frigate's yaw angle from its rotation
            let frigate_yaw = frigate_rotation.euler_angles().1;  // Y rotation (yaw)
            
            // Calculate barrel mount position (where barrel pivots from)
            let barrel_pivot_local = Vec3::new(0.0, 0.5, -0.5);  // Pivot point inside base
            let base_rotation = frigate_rotation * Quat::from_euler_angles(0.0, turret_base.rotation_y, 0.0);
            let barrel_pivot_world = turret_base.position + (base_rotation * barrel_pivot_local);
            
            // Calculate direction to target from barrel pivot
            let to_target = current_target.position - barrel_pivot_world;
            let horizontal_dist = (to_target.x * to_target.x + to_target.z * to_target.z).sqrt();
            
            // Calculate yaw (base rotation) and pitch (barrel elevation)
            // Convert world yaw to turret-local yaw by subtracting frigate's rotation
            let world_target_yaw = (-to_target.z).atan2(to_target.x) - std::f32::consts::FRAC_PI_2;
            let target_yaw = world_target_yaw - frigate_yaw;  // Convert to frigate-relative angle
            let target_pitch = to_target.y.atan2(horizontal_dist);
            
            turret_base.target_rotation_y = target_yaw;
            turret_barrel.target_pitch = target_pitch.clamp(-0.2, 0.8);
            
            // Smoothly rotate base towards target
            let yaw_diff = turret_base.target_rotation_y - turret_base.rotation_y;
            let yaw_diff_normalized = ((yaw_diff + std::f32::consts::PI) % (2.0 * std::f32::consts::PI)) - std::f32::consts::PI;
            turret_base.rotation_y += yaw_diff_normalized.signum() * (TURRET_ROTATION_SPEED * delta_time).min(yaw_diff_normalized.abs());
            
            // Smoothly pitch barrel towards target
            let pitch_diff = turret_barrel.target_pitch - turret_barrel.pitch;
            turret_barrel.pitch += pitch_diff.signum() * (TURRET_ROTATION_SPEED * delta_time).min(pitch_diff.abs());
            
            // Update base transform (frigate_rotation already calculated above)
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(turret_base.entity) {
                // Combine frigate rotation with turret targeting rotation
                let turret_targeting_rotation = Quat::from_euler_angles(0.0, turret_base.rotation_y, 0.0);
                transform.rotation = frigate_rotation * turret_targeting_rotation;
            }
        }
        
        // Update barrel recoil and position
        if let Some(turret_barrel) = self.turret_barrel.as_mut() {
            if let Some(turret_base) = self.turret_base.as_ref() {
                // Return barrel to forward position
                if turret_barrel.recoil_offset > 0.0 {
                    turret_barrel.recoil_offset -= TURRET_RETURN_SPEED * delta_time;
                    turret_barrel.recoil_offset = turret_barrel.recoil_offset.max(0.0);
                }
                
                // Get frigate rotation first to avoid borrowing conflicts
                let frigate_rotation = if let Some(frigate_entity) = self.frigate_entity {
                    self.world.get_component::<TransformComponent>(frigate_entity)
                        .map(|t| t.rotation)
                        .unwrap_or(Quat::identity())
                } else {
                    Quat::identity()
                };
                
                // Calculate barrel world position with pitch rotation around pivot point
                // Combine frigate rotation with turret targeting rotation
                let turret_targeting_rotation = Quat::from_euler_angles(0.0, turret_base.rotation_y, 0.0);
                let base_rotation = frigate_rotation * turret_targeting_rotation;
                
                // Pivot point is inside the turret base where barrel mounts
                let barrel_pivot_local = Vec3::new(0.0, 0.5, -0.5);
                
                // Barrel offset from its pivot point (in barrel's local space, pre-pitch)
                // Extended further forward (-Z direction)
                let barrel_from_pivot = Vec3::new(0.0, 0.0, -1.5);  // Barrel extends forward from pivot
                
                // Apply pitch rotation to barrel offset
                let barrel_pitch_rotation = Quat::from_euler_angles(turret_barrel.pitch, 0.0, 0.0);
                let barrel_offset_pitched = barrel_pitch_rotation * barrel_from_pivot;
                
                // Apply recoil along the pitched barrel's local Z axis
                let recoil_vector = barrel_pitch_rotation * Vec3::new(0.0, 0.0, turret_barrel.recoil_offset);
                let barrel_offset_with_recoil = barrel_offset_pitched + recoil_vector;
                
                // Combine barrel position: pivot + pitched barrel offset
                let barrel_local_total = barrel_pivot_local + barrel_offset_with_recoil;
                
                // Apply base rotation and add to base world position
                let rotated_barrel_offset = base_rotation * barrel_local_total;
                let barrel_world_position = turret_base.position + rotated_barrel_offset;
                
                // Barrel rotation is base yaw + barrel pitch
                let barrel_rotation = base_rotation * barrel_pitch_rotation;
                
                if let Some(transform) = self.world.get_component_mut::<TransformComponent>(turret_barrel.entity) {
                    transform.position = barrel_world_position;
                    transform.rotation = barrel_rotation;
                }
            }
        }
        
        // Fire timer
        self.fire_timer += delta_time;
        if self.fire_timer >= FIRE_INTERVAL {
            self.fire_timer = 0.0;
            self.fire_turret();
        }
        
        // Update FPS counter
        self.frame_count += 1;
        let fps_elapsed = now.duration_since(self.fps_update_timer).as_secs_f32();
        if fps_elapsed >= 1.0 {
            self.current_fps = self.frame_count as f32 / fps_elapsed;
            self.frame_count = 0;
            self.fps_update_timer = now;
            
            // Update UI
            let fps_text = format!("FPS: {:.1}", self.current_fps);
            self.ui_manager.update_text(self.fps_label_id, fps_text);
        }
        
        // CRITICAL: Sync entities to scene manager for rendering
        // This updates the render cache from ECS components
        self.scene_manager.sync_from_world(&mut self.world);
    }
    
    fn fire_turret(&mut self) {
        if let Some(turret_barrel) = self.turret_barrel.as_mut() {
            turret_barrel.recoil_offset = TURRET_RECOIL_DISTANCE;
            turret_barrel.is_recoiling = true;
            log::info!("Turret fired! Recoil: {}", TURRET_RECOIL_DISTANCE);
        }
    }
    
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Sync entities to render queue
        let render_queue = self.scene_manager.build_render_queue();
        self.graphics_engine.render_entities_from_queue(&render_queue)?;
        
        // Build lighting environment
        let multi_light_env = self.lighting_system.build_multi_light_environment(
            &self.world,
            Vec3::new(0.0, 0.0, 0.0),  // No ambient light in space
            0.0
        ).clone();
        

        
        // Export UI data
        let ui_data = self.ui_manager.get_render_data();
        
        // Render frame
        self.graphics_engine.render_frame(
            &rust_engine::render::RenderFrameData {
                camera: &self.camera,
                lights: &multi_light_env,
                ui: &ui_data,
                billboards: &[],
            },
            &mut self.window,
        )?;
        
        // Dispatch UI events
        self.ui_manager.dispatch_events();
        
        Ok(())
    }
    
    fn spawn_standalone_turret(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        
        // Place standalone turret at a fixed position away from frigate
        let standalone_position = Vec3::new(-50.0, 0.0, -50.0);
        
        // Create standalone turret base
        let base_transform = Transform {
            position: standalone_position,
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        let turret_base_material = self.turret_base_material.as_ref()
            .expect("Turret base material not initialized")
            .clone();
        
        let base_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.turret_base_mesh.as_ref().unwrap().clone(),
            MeshType::TurretBase,
            turret_base_material,
            base_transform
        );
        
        // Create standalone turret barrel
        let barrel_position = standalone_position + Vec3::new(0.0, 0.5, -1.2);
        let barrel_transform = Transform {
            position: barrel_position,
            rotation: Quat::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        
        let turret_barrel_material = MaterialBuilder::new()
            .base_color_rgb(0.3, 0.3, 0.35)
            .metallic(0.9)
            .roughness(0.2)
            .name("Standalone Turret Barrel")
            .build();
        
        let barrel_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.turret_barrel_mesh.as_ref().unwrap().clone(),
            MeshType::TurretBarrel,
            turret_barrel_material,
            barrel_transform
        );
        
        self.standalone_turret_base = Some(base_entity);
        self.standalone_turret_barrel = Some(barrel_entity);
        
        log::info!("Spawned standalone turret at position {:?}", standalone_position);
        Ok(())
    }
    
    fn spawn_monkey(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        use rust_engine::foundation::math::Transform;
        
        // Place monkey between the two turrets
        let monkey_position = Vec3::new(-25.0, 5.0, -25.0);
        
        let monkey_transform = Transform {
            position: monkey_position,
            rotation: Quat::identity(),
            scale: Vec3::new(3.0, 3.0, 3.0),  // Make it bigger so it's visible
        };
        
        let monkey_material = MaterialBuilder::new()
            .base_color_rgb(0.8, 0.6, 0.4)  // Brown monkey
            .metallic(0.1)
            .roughness(0.7)
            .name("Monkey Instance")
            .build();
        
        let monkey_entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.monkey_mesh.as_ref().unwrap().clone(),
            MeshType::Monkey,
            monkey_material,
            monkey_transform
        );
        
        self.monkey_entity = Some(monkey_entity);
        
        log::info!("Spawned monkey at position {:?}", monkey_position);
        Ok(())
    }
}

impl Drop for TurretDemoApp {
    fn drop(&mut self) {
        // Cleanup handled by renderer's drop implementation
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    
    log::info!("Starting Rusteroids Turret Demo");
    
    let mut app = TurretDemoApp::new();
    let result = app.run();
    
    match result {
        Ok(()) => {
            log::info!("Turret demo completed successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Turret demo failed: {:?}", e);
            Err(e)
        }
    }
}
