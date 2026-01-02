//! Fleet demo with frigates, escort formations, and particle effects

#![allow(dead_code)]

use rust_engine::assets::{ObjLoader, MaterialBuilder};
use rust_engine::render::resources::materials::{Material, MaterialType, UnlitMaterialParams};
use rust_engine::ecs::{
    World, Entity, LightFactory, LightingSystem as EcsLightingSystem,
};
use rust_engine::ecs::components::{TransformComponent, LightComponent, RenderableComponent};
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

// Fleet configuration
const NUM_FRIGATES: usize = 3;  // Three frigates with slight drift
const SHIPS_PER_V_FORMATION: usize = 5;  // V formation (1 leader + 2 wings of 2)
const FORMATIONS_PER_FRIGATE: usize = 4;  // Multiple escort groups per frigate
const FRIGATE_SPEED: f32 = 5.0;  // Increased for more visible movement
const FRIGATE_SPACING: f32 = 50.0;
const FORMATION_ORBIT_RADIUS: f32 = 15.0;
const FORMATION_ORBIT_SPEED: f32 = 0.5;
const PARTICLE_SPAWN_DISTANCE: f32 = 50.0;  // Much closer to see movement
const PARTICLE_DESPAWN_DISTANCE: f32 = -50.0;
const PARTICLES_PER_SPAWN: usize = 8;  // Spawn more particles per batch
const MAX_PARTICLES: usize = 64;  // Maximum particles in scene (matches shader limit)
const PARTICLE_SPREAD: f32 = 80.0;
const CAMERA_FOLLOW_SMOOTHNESS: f32 = 0.05;

struct FormationShip {
    entity: Entity,
    offset: Vec3,
    phase: f32,
    local_offset: Vec3,
    orbit_phase: f32,
}

struct SpaceshipFormation {
    parent_frigate: usize,  // Index into frigates vec
    ships: Vec<FormationShip>,
    orbit_radius: f32,
    orbit_speed: f32,
    angle_offset: f32,
    pattern_type: u8,  // 0=circular, 1=figure-8, 2=spiral, 3=wave
    variance_x: f32,   // Random variance for x movement
    variance_y: f32,   // Random variance for y movement
    variance_z: f32,   // Random variance for z movement
}

struct FleetFrigate {
    entity: Entity,
    position: Vec3,
}

struct ParticleLight {
    entity: Entity,
    position: Vec3,
    base_material: Material,  // Store base material for cloning
    fade_timer: Option<f32>,  // Some(time) when fading out, None when active
}

pub struct DynamicTeapotApp {
    window: WindowHandle,
    graphics_engine: GraphicsEngine,
    camera: Camera,
    scene_manager: SceneManager,
    world: World,
    lighting_system: EcsLightingSystem,
    
    frigate_mesh: Option<Mesh>,
    spaceship_mesh: Option<Mesh>,
    spaceship_material: Option<Material>,
    sphere_mesh: Option<Mesh>,
    sunlight_entity: Option<Entity>,
    skybox_entity: Option<Entity>,  // Track skybox for camera-centering
    
    // Fleet entities
    frigates: Vec<FleetFrigate>,
    formations: Vec<SpaceshipFormation>,
    particles: Vec<ParticleLight>,
    flyby_group: Option<SpaceshipFormation>,  // Special flyby formation
    flyby_spawn_timer: f32,  // Time until next flyby spawn
    flyby_group_2: Option<SpaceshipFormation>,  // Second flyby formation
    flyby_spawn_timer_2: f32,  // Time until second flyby spawn
    
    // Camera tracking
    camera_target_position: Vec3,
    camera_target_lookat: Vec3,
    
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
    
    // Audio
    audio: Option<rust_engine::audio::AudioSystem>,
    
    // Timing
    start_time: Instant,
    particle_spawn_timer: Instant,
    
    // FPS tracking
    fps_update_timer: Instant,
    frame_count: u32,
    current_fps: f32,
    last_frame_time: Instant,
    
    // UI System
    ui_manager: rust_engine::ui::UIManager,
    fps_label_id: rust_engine::ui::UINodeId,
}

impl DynamicTeapotApp {
    pub fn new() -> Self {
        log::info!("Creating fleet demo application...");
        
        // Create window
        let mut window = WindowHandle::new(1200, 800, "Rusteroids - Fleet Demo");
        
        // Create renderer
        let renderer_config = VulkanRendererConfig::new("Rusteroids - Fleet Demo")
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
        
        // Initialize dynamic object system with more capacity for fleet
        graphics_engine.initialize_dynamic_system(500)
            .expect("Failed to initialize dynamic object system");
        
        // Create camera - positioned front-right-above the fleet
        let camera_position = Vec3::new(80.0, 30.0, 120.0);  // Front, right, and above
        let fleet_center = Vec3::new(0.0, 0.0, 0.0);  // Looking at origin where frigates are
        let mut camera = Camera::perspective(
            camera_position,
            60.0,
            1200.0 / 800.0,
            0.1,
            1000.0  // Far plane for deep space
        );
        camera.set_position(camera_position);
        camera.look_at(fleet_center, Vec3::new(0.0, 1.0, 0.0));
        
        // Initialize SceneManager
        let scene_manager = SceneManager::new();
        
        // Initialize ECS for lighting
        let mut world = World::new();
        let lighting_system = EcsLightingSystem::new();
        
        // Create dim yellow sunlight in front of frigates, pointing down and back
        let _sunlight_entity = Some(Self::create_sunlight(&mut world));
        
        // Initialize audio system and start music loop
        let mut audio = rust_engine::audio::AudioSystem::new().ok();
        if let Some(ref mut audio_system) = audio {
            let music_track = rust_engine::audio::MusicTrack::new(
                "background", 
                "resources/audio/music.mp3"
            );
            // Set looping via the play_music_track method - parameter is Option<f32> for volume
            let _ = audio_system.play_music_track(music_track, None);
        }
        
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
        
        Self {
            window,
            graphics_engine,
            camera,
            scene_manager,
            world,
            lighting_system,
            frigate_mesh: None,
            spaceship_mesh: None,
            spaceship_material: None,
            sphere_mesh: None,
            sunlight_entity: None,
            skybox_entity: None,
            fps_update_timer: Instant::now(),
            frame_count: 0,
            current_fps: 60.0,
            last_frame_time: Instant::now(),
            ui_manager,
            fps_label_id,
            audio,
            frigates: Vec::new(),
            formations: Vec::new(),
            particles: Vec::new(),
            flyby_group: None,
            flyby_spawn_timer: 5.0,  // First flyby after 5 seconds
            flyby_group_2: None,
            flyby_spawn_timer_2: 8.0,  // Second group starts 8 seconds in
            camera_target_position: Vec3::new(80.0, 30.0, 120.0),
            camera_target_lookat: Vec3::new(0.0, 0.0, 0.0),
            start_time,
            particle_spawn_timer: Instant::now(),
            camera_move_forward: false,
            camera_move_backward: false,
            camera_move_left: false,
            camera_move_right: false,
            camera_yaw_left: false,
            camera_yaw_right: false,
            camera_pitch_up: false,
            camera_pitch_down: false,
            camera_yaw: 0.325,  // Positive yaw for left rotation (~19 degrees)
            camera_pitch: -0.07,  // Pitch down ~4 degrees (in radians)
        }
    }
    
    fn create_sunlight(world: &mut World) -> Entity {
        // Create a dim yellow sunlight more level with frigates, pointing slightly down and back
        let sunlight_entity = world.create_entity();
        
        // Direction: more level with frigates, slight downward angle and backward
        // Y component reduced from -0.6 to -0.2 for less steep angle
        // Increased Z component for more backward direction
        let direction = Vec3::new(-1.0, -0.9, -1.6).normalize(); // Points slightly down and mostly backward
        
        let sunlight = LightFactory::directional(
            direction,
            Vec3::new(0.9, 0.7, 0.9), // Dim yellow color (reduced blue component)
            0.5 // Dim intensity to not overpower particle lights
        );
        
        world.add_component(sunlight_entity, sunlight);
        log::info!("Created dim yellow sunlight more level with frigates, pointing slightly down and back");
        sunlight_entity
    }
    
    fn spawn_fleet_frigates(&mut self) {
        // Spawn frigates in a staggered formation
        for i in 0..NUM_FRIGATES {
            // All frigates in staggered formation
            let x_offset = ((i as f32) - (NUM_FRIGATES as f32 - 1.0) / 2.0) * FRIGATE_SPACING;
            let z_offset = ((i % 2) as f32) * -30.0;
            let position = Vec3::new(x_offset, 0.0, z_offset);
            
            // Load frigate material from MTL
            let frigate_material = rust_engine::assets::MaterialLoader::load_mtl_with_textures(
                "resources/models/frigate.mtl",
                "Material.001",
                &mut self.graphics_engine
            ).unwrap_or_else(|e| {
                log::warn!("Failed to load frigate material: {}. Using fallback.", e);
                MaterialBuilder::new()
                    .base_color_rgb(1.0, 1.0, 1.0)
                    .metallic(0.6)
                    .roughness(0.3)
                    .emission_rgb(1.0, 1.0, 1.0)
                    .emission_strength(12.0)  // Fallback needs high emission
                    .name("Frigate Fallback")
                    .build()
            });
            
            // Create frigate entity
            use rust_engine::foundation::math::Transform;
            let rotation_quat = Quat::from_euler_angles(0.0, 0.0, 0.0);
            let transform = Transform::from_position_rotation(position, rotation_quat);
            let transform_scaled = Transform {
                position: transform.position,
                rotation: transform.rotation,
                scale: Vec3::new(3.0, 3.0, 3.0),
            };
            let entity = self.scene_manager.create_renderable_entity(
                &mut self.world,
                self.frigate_mesh.as_ref().unwrap().clone(),
                rust_engine::render::systems::dynamic::MeshType::Frigate,
                frigate_material,
                transform_scaled
            );
            
            // Ensure transform component exists (redundant but safe)
            use rust_engine::ecs::components::TransformComponent;
            if self.world.get_component::<TransformComponent>(entity).is_none() {
                let transform_comp = TransformComponent {
                    position,
                    rotation: Quat::from_euler_angles(0.0, 0.0, 0.0),
                    scale: Vec3::new(3.0, 3.0, 3.0),
                };
                self.world.add_component(entity, transform_comp);
                log::info!("Manually added Transform component to frigate {}", entity.id());
            }
            
            self.frigates.push(FleetFrigate { entity, position });
            
            // Spawn formations for this frigate
            for j in 0..FORMATIONS_PER_FRIGATE {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                
                // Position formation groups around the frigate (front, sides, above)
                let angle_offset = (j as f32) * (2.0 * std::f32::consts::PI / FORMATIONS_PER_FRIGATE as f32);
                let formation_distance = 25.0;  // Distance from frigate
                
                let formation = SpaceshipFormation {
                    parent_frigate: i,
                    ships: Vec::new(),
                    orbit_radius: formation_distance,
                    orbit_speed: 0.1,  // Very slow group drift
                    angle_offset,
                    pattern_type: 0,  // Not used for V formations
                    variance_x: rng.gen_range(-0.5..0.5),  // Small group drift variance
                    variance_y: rng.gen_range(-0.3..0.3),
                    variance_z: rng.gen_range(-0.5..0.5),
                };
                
                self.formations.push(formation);
            }
        }
        
        log::info!("Spawned {} frigates with {} formations", NUM_FRIGATES, self.formations.len());
    }
    
    fn spawn_formation_ships(&mut self) {
        let spaceship_mesh = self.spaceship_mesh.clone().expect("Spaceship mesh not loaded");
        let spaceship_material = self.spaceship_material.clone().expect("Spaceship material not loaded");
        
        for formation_idx in 0..self.formations.len() {
            let parent_frigate_idx = self.formations[formation_idx].parent_frigate;
            let frigate_position = self.frigates[parent_frigate_idx].position;
            let angle_offset = self.formations[formation_idx].angle_offset;
            let formation_distance = self.formations[formation_idx].orbit_radius;
            
            // Calculate formation group center position (around frigate)
            let group_center_offset = Vec3::new(
                angle_offset.cos() * formation_distance,
                (angle_offset * 2.0).sin() * 5.0 + 15.0,  // Higher above frigates (+15)
                angle_offset.sin() * formation_distance + (formation_idx as f32) * 8.0  // Forward stagger
            );
            
            // Spawn V formation
            for ship_idx in 0..SHIPS_PER_V_FORMATION {
                // V formation offsets: 1 leader at tip, 2 on each wing
                let v_offset = match ship_idx {
                    0 => Vec3::new(0.0, 0.0, 5.0),      // Leader at front
                    1 => Vec3::new(-3.0, -0.5, 0.0),    // Left wing, position 1
                    2 => Vec3::new(-6.0, -1.0, -5.0),   // Left wing, position 2
                    3 => Vec3::new(3.0, -0.5, 0.0),     // Right wing, position 1
                    4 => Vec3::new(6.0, -1.0, -5.0),    // Right wing, position 2
                    _ => Vec3::new(0.0, 0.0, 0.0),
                };
                
                let local_offset = group_center_offset + v_offset;
                let position = frigate_position + local_offset;
                
                // Point ships forward (same direction as frigate)
                let rotation_quat = Quat::from_euler_angles(0.0, 0.0, 0.0);
                
                use rust_engine::foundation::math::Transform;
                let transform = Transform::from_position_rotation(position, rotation_quat);
                let transform_scaled = Transform {
                    position: transform.position,
                    rotation: transform.rotation,
                    scale: Vec3::new(0.5, 0.5, 0.5),
                };
                let entity = self.scene_manager.create_renderable_entity(
                    &mut self.world,
                    spaceship_mesh.clone(),
                    rust_engine::render::systems::dynamic::MeshType::Spaceship,
                    spaceship_material.clone(),
                    transform_scaled
                );
                
                self.formations[formation_idx].ships.push(FormationShip {
                    entity,
                    offset: local_offset,
                    phase: (ship_idx as f32) * 0.7,  // Stagger drift phase
                    local_offset: v_offset,  // Store V formation offset
                    orbit_phase: 0.0,
                });
            }
        }
        
        log::info!("Spawned {} escort ships in V formations", self.formations.iter().map(|f| f.ships.len()).sum::<usize>());
    }

    fn spawn_flyby_formation(&mut self, frigate_index: usize, spawn_z: f32) {
        if frigate_index >= self.frigates.len() {
            return;
        }
        
        let spaceship_mesh = self.spaceship_mesh.clone().expect("Spaceship mesh not loaded");
        let spaceship_material = self.spaceship_material.clone().expect("Spaceship material not loaded");
        
        // Pick specified frigate for flyby spawn
        let frigate_position = self.frigates[frigate_index].position;
        
        // Spawn below frigate, well behind camera
        let spawn_position = Vec3::new(
            frigate_position.x,
            frigate_position.y - 20.0,  // 20 units below frigate
            spawn_z  // Passed in parameter
        );
        
        log::info!("Spawning flyby V formation {} at {:?}", frigate_index, spawn_position);
        
        let mut ships = Vec::new();
        
        // Create V formation
        for ship_idx in 0..SHIPS_PER_V_FORMATION {
            let v_offset = match ship_idx {
                0 => Vec3::new(0.0, 0.0, 5.0),      // Leader at front
                1 => Vec3::new(-3.0, -0.5, 0.0),    // Left wing
                2 => Vec3::new(-6.0, -1.0, -5.0),   // Left wing
                3 => Vec3::new(3.0, -0.5, 0.0),     // Right wing
                4 => Vec3::new(6.0, -1.0, -5.0),    // Right wing
                _ => Vec3::new(0.0, 0.0, 0.0),
            };
            
            let position = spawn_position + v_offset;
            let rotation_quat = Quat::from_euler_angles(0.0, 0.0, 0.0);
            
            use rust_engine::foundation::math::Transform;
            let transform = Transform::from_position_rotation(position, rotation_quat);
            let transform_scaled = Transform {
                position: transform.position,
                rotation: transform.rotation,
                scale: Vec3::new(0.5, 0.5, 0.5),
            };
            let entity = self.scene_manager.create_renderable_entity(
                &mut self.world,
                spaceship_mesh.clone(),
                rust_engine::render::systems::dynamic::MeshType::Spaceship,
                spaceship_material.clone(),
                transform_scaled
            );
            
            ships.push(FormationShip {
                entity,
                offset: v_offset,
                phase: 0.0,
                local_offset: v_offset,
                orbit_phase: 0.0,
            });
        }
        
        let formation = SpaceshipFormation {
            parent_frigate: frigate_index,
            ships,
            orbit_radius: 0.0,
            orbit_speed: 0.0,
            angle_offset: 0.0,
            pattern_type: 0,
            variance_x: 0.0,
            variance_y: 0.0,
            variance_z: 0.0,
        };
        
        // Store in appropriate flyby group slot
        if frigate_index == 0 {
            self.flyby_group = Some(formation);
        } else {
            self.flyby_group_2 = Some(formation);
        }
    }
    
    fn spawn_particle_lights_count(&mut self, z_position: f32, count: usize) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // Color palette for varied particles - MATCH MONKEY DEMO APPROACH
        let colors = [
            Vec3::new(1.0, 0.4, 0.2),  // Orange-red
            Vec3::new(1.0, 0.8, 0.2),  // Yellow
            Vec3::new(0.2, 0.6, 1.0),  // Blue
            Vec3::new(0.3, 1.0, 0.8),  // Cyan
            Vec3::new(0.8, 0.3, 1.0),  // Purple
            Vec3::new(0.4, 1.0, 0.3),  // Green
            Vec3::new(1.0, 0.5, 0.8),  // Pink
            Vec3::new(1.0, 0.3, 0.3),  // Red
        ];
        
        // Spawn a batch of particle lights
        for _ in 0..count {
            let x = rng.gen_range(-PARTICLE_SPREAD..PARTICLE_SPREAD);
            let y = rng.gen_range(-PARTICLE_SPREAD..PARTICLE_SPREAD);
            let z = z_position + rng.gen_range(-20.0..20.0);  // Add Z variance
            let position = Vec3::new(x, y, z);
            
            // Pick a random color from the palette
            let color = colors[rng.gen_range(0..colors.len())];
            
            // Create small emissive sphere - UNLIT so not affected by other lights!
            // Unlit materials don't have emission, so we just use the color directly
            let material = Material::transparent_unlit(UnlitMaterialParams {
                color: color * 2.0,  // Brighten the color for visibility
                alpha: 0.8,
            })
            .with_name("Particle");
            
            // Use sphere mesh for particles
            let mesh = self.sphere_mesh.clone().expect("Sphere mesh not loaded");
            
            use rust_engine::foundation::math::Transform;
            let transform = Transform::from_position(position);
            let transform_scaled = Transform {
                position: transform.position,
                rotation: transform.rotation,
                scale: Vec3::new(0.3, 0.3, 0.3),
            };
            let entity = self.scene_manager.create_renderable_entity(
                &mut self.world,
                mesh,
                rust_engine::render::systems::dynamic::MeshType::Sphere,
                material.clone(),  // Clone so we can store in ParticleLight
                transform_scaled
            );
            
            // Add a point light at the particle position matching the sphere's color
            let light = LightFactory::point(
                position,
                color,                       // Use the Vec3 color directly
                30.0,  // High intensity to reach frigates
                60.0   // Large range to cover frigate spacing
            );
            self.world.add_component(entity, light);
            
            self.particles.push(ParticleLight { 
                entity, 
                position,
                base_material: material,  // Store for fade cloning
                fade_timer: None 
            });
        }
    }

    fn spawn_particle_lights(&mut self, z_position: f32) {
        self.spawn_particle_lights_count(z_position, PARTICLES_PER_SPAWN);
    }

    fn spawn_background_frigate(&mut self) {
        // Spawn frigate in the background
        let position = Vec3::new(0.0, 3.0, -80.0); // Higher and back
        let scale = 3.0; // Larger since it's further away
        
        // Slight angle for visual interest
        let rotation = Vec3::new(
            -0.1, // Slight pitch down
            0.7,  // Slight yaw
            0.0,  // No roll
        );
        
        // Load frigate material from MTL file with automatic texture loading
        let frigate_material = rust_engine::assets::MaterialLoader::load_mtl_with_textures(
            "resources/models/frigate.mtl",
            "Material.001",
            &mut self.graphics_engine
        ).unwrap_or_else(|e| {
            log::warn!("Failed to load frigate material from MTL: {}. Using fallback.", e);
            // Fallback material if MTL loading fails
            MaterialBuilder::new()
                .base_color_rgb(1.0, 1.0, 1.0)
                .metallic(0.6)
                .roughness(0.3)
                .emission_rgb(1.0, 1.0, 1.0)
                .emission_strength(8.0)
                .name("Frigate Fallback")
                .build()
        });
        
        // Create the frigate entity
        use rust_engine::foundation::math::Transform;
        let rotation_quat = Quat::from_euler_angles(rotation.x, rotation.y, rotation.z);
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.frigate_mesh.as_ref().unwrap().clone(),
            rust_engine::render::systems::dynamic::MeshType::Frigate,
            frigate_material,
            Transform {
                position,
                rotation: rotation_quat,
                scale: Vec3::new(scale, scale, scale),
            },
        );
        
        // Make it persistent (no LifecycleComponent, so it won't despawn)
        log::info!("Spawned background frigate (Entity {:?}) at {:?} with scale {:.2}", 
                  entity, position, scale);
    }
    

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing fleet demo...");
        
        // Load frigate mesh
        match ObjLoader::load_obj("resources/models/frigate.obj") {
            Ok(mesh) => {
                log::info!("Loaded frigate mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.frigate_mesh = Some(mesh);
            }
            Err(e) => {
                log::warn!("Failed to load frigate.obj: {:?}", e);
                return Err(e.into());
            }
        }
        
        // Create frigate mesh pool
        if let Some(ref frigate_mesh) = self.frigate_mesh {
            use rust_engine::assets::ImageData;
            use rust_engine::render::resources::materials::TextureType;
            
            let base_texture_path = "resources/textures/Frigate_texture.png";
            let emission_texture_path = "resources/textures/Frigate_emission.png";
            
            let base_image = ImageData::from_file(base_texture_path)
                .map_err(|e| format!("Failed to load frigate base texture: {}", e))?;
            let emission_image = ImageData::from_file(emission_texture_path)
                .map_err(|e| format!("Failed to load frigate emission texture: {}", e))?;
            
            let base_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                base_image,
                TextureType::BaseColor
            )?;
            let emission_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                emission_image,
                TextureType::Emission
            )?;
            
            let frigate_material = MaterialBuilder::new()
                .base_color_rgb(1.0, 1.0, 1.0)
                .metallic(0.6)
                .roughness(0.3)
                .emission_rgb(1.0, 1.0, 1.0)
                .emission_strength(12.0)  // Higher value needed because emission texture multiplies this
                .name("Frigate")
                .build()
                .with_base_color_texture(base_texture_handle)
                .with_emission_texture(emission_texture_handle);
            
            if let Err(e) = self.graphics_engine.create_mesh_pool(
                MeshType::Frigate,
                frigate_mesh,
                &[frigate_material.clone()],
                20
            ) {
                log::error!("Failed to create frigate mesh pool: {}", e);
                return Err(e);
            }
            log::info!("Created frigate mesh pool successfully");
        }
        
        // Load sphere mesh for particles
        match ObjLoader::load_obj("resources/models/sphere.obj") {
            Ok(mesh) => {
                log::info!("Loaded sphere mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.sphere_mesh = Some(mesh.clone());
                
                // Create sphere mesh pool with UNLIT TRANSPARENT base (particles not affected by lights)
                // Per-instance materials will completely override this
                let sphere_material = Material::transparent_unlit(UnlitMaterialParams {
                    color: Vec3::new(1.0, 1.0, 1.0),  // Pure white neutral
                    alpha: 0.8,                       // Match particle transparency
                })
                .with_name("Particle Sphere Base");
                
                if let Err(e) = self.graphics_engine.create_mesh_pool(
                    MeshType::Sphere,
                    &mesh,
                    &[sphere_material],
                    100  // Enough for many particles
                ) {
                    log::error!("Failed to create sphere mesh pool: {}", e);
                    return Err(e);
                }
                log::info!("Created sphere mesh pool successfully (transparent, neutral base)");
            }
            Err(e) => {
                log::warn!("Failed to load sphere.obj: {:?}", e);
                return Err(e.into());
            }
        }
        
        // Create skybox
        let skybox_mesh = Mesh::skybox();
        
        // Load skybox texture
        use rust_engine::assets::ImageData;
        use rust_engine::render::resources::materials::TextureType;
        
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
            255,  // Skybox layer (renders after opaque, before transparent)
        );
        self.skybox_entity = Some(skybox_entity);
        
        // Load spaceship mesh
        match ObjLoader::load_obj("resources/models/spaceship_simple.obj") {
            Ok(mesh) => {
                log::info!("Loaded spaceship mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.spaceship_mesh = Some(mesh.clone());
                
                // Load spaceship textures
                use rust_engine::assets::ImageData;
                use rust_engine::render::resources::materials::TextureType;
                
                let base_texture_path = "resources/textures/spaceship_simple_texture.png";
                let emission_texture_path = "resources/textures/spaceship_simple_emission_map.png";
                
                let base_image = ImageData::from_file(base_texture_path)
                    .map_err(|e| format!("Failed to load spaceship base texture: {}", e))?;
                let emission_image = ImageData::from_file(emission_texture_path)
                    .map_err(|e| format!("Failed to load spaceship emission texture: {}", e))?;
                
                let base_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                    base_image,
                    TextureType::BaseColor
                )?;
                let emission_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                    emission_image,
                    TextureType::Emission
                )?;
                
                // Create spaceship mesh pool with simple material
                let spaceship_material = MaterialBuilder::new()
                    .base_color_rgb(1.0, 1.0, 1.0)
                    .metallic(0.8)
                    .roughness(0.2)
                    .emission_rgb(1.0, 1.0, 1.0)
                    .emission_strength(12.0)  // Increased from 8.0
                    .name("Spaceship")
                    .build()
                    .with_base_color_texture(base_texture_handle)
                    .with_emission_texture(emission_texture_handle);
                
                // Store material for reuse
                self.spaceship_material = Some(spaceship_material.clone());
                
                if let Err(e) = self.graphics_engine.create_mesh_pool(
                    MeshType::Spaceship,
                    &mesh,
                    &[spaceship_material],
                    200  // Many escort ships
                ) {
                    log::error!("Failed to create spaceship mesh pool: {}", e);
                    return Err(e);
                }
                log::info!("Created spaceship mesh pool successfully");
            }
            Err(e) => {
                log::warn!("Failed to load spaceship_simple.obj: {:?}", e);
                return Err(e.into());
            }
        }
        
        // Spawn the fleet
        self.spawn_fleet_frigates();
        self.spawn_formation_ships();
        
        // Spawn initial batch of particle lights in front of ships
        log::info!("Spawning initial particles in front of fleet...");
        for i in 0..8 {
            let z = 200.0 - (i as f32) * 25.0;  // Spawn ahead: 200, 175, 150, ...
            self.spawn_particle_lights(z);
        }
        log::info!("Initial spawn complete. Total particles: {}", self.particles.len());

        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting fleet demo...");
        self.initialize()?;
        
        let mut framebuffer_resized = false;
        
        while !self.window.should_close() {
            self.window.poll_events();
            
            // Handle events
            let events: Vec<_> = self.window.event_iter().collect();
            
            for (_, event) in events {
                match event {
                    WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
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
            
            // Update FPS
            self.update_scene();
            
            // Render frame
            if let Err(e) = self.render_frame() {
                log::error!("Failed to render frame: {:?}", e);
                break;
            }
        }
        
        log::info!("Fleet demo completed");
        Ok(())
    }
    
    fn update_scene(&mut self) {
        let now = Instant::now();
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Apply camera movement
        let camera_speed = 10.0; // Units per second
        let camera_rotation_speed = 1.5; // Radians per second
        
        // Get camera forward and right vectors
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
        self.camera_pitch = self.camera_pitch.clamp(-std::f32::consts::FRAC_PI_2 + 0.1, std::f32::consts::FRAC_PI_2 - 0.1);
        
        // Update camera position and orientation
        self.camera.set_position(camera_pos);
        let look_target = camera_pos + forward;
        self.camera.look_at(look_target, Vec3::new(0.0, 1.0, 0.0));
        
        // Update skybox to follow camera position (camera-centered skybox)
        if let Some(skybox_entity) = self.skybox_entity {
            use rust_engine::ecs::components::TransformComponent;
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(skybox_entity) {
                transform.position = camera_pos;  // Skybox always centered on camera
                self.scene_manager.mark_entity_dirty(skybox_entity);
            }
        }
        
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
        
        // Apply slight cyclical drift to frigates to simulate movement
        for (idx, frigate) in self.frigates.iter_mut().enumerate() {
            // Slight cyclical drift motion (not moving forward, just drifting)
            // Stagger each frigate in the cycle with phase offset
            let drift_phase = elapsed * 0.3 + (idx as f32) * 2.1;  // 2.1 rad offset staggers them nicely
            frigate.position.x += (drift_phase.sin() * 0.5) * delta_time;
            frigate.position.y += (drift_phase.cos() * 0.3) * delta_time;
            frigate.position.z += ((drift_phase * 1.5).sin() * 0.4) * delta_time;
            
            // Update frigate entity transform component
            use rust_engine::ecs::components::TransformComponent;
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(frigate.entity) {
                transform.position = frigate.position;
                // Mark entity as dirty so scene manager syncs it
                self.scene_manager.mark_entity_dirty(frigate.entity);
            }
        }
        
        // Update formation ship positions with minimal cyclical drift
        for (formation_idx, formation) in self.formations.iter_mut().enumerate() {
            if formation.parent_frigate >= self.frigates.len() {
                continue;
            }
            
            let frigate = &self.frigates[formation.parent_frigate];
            
            // Group-level drift (whole formation drifts together slightly)
            let group_drift_phase = elapsed * formation.orbit_speed;
            let group_drift = Vec3::new(
                formation.variance_x * group_drift_phase.sin(),
                formation.variance_y * (group_drift_phase * 0.8).cos(),
                formation.variance_z * (group_drift_phase * 0.6).sin()
            );
            
            // Calculate formation group center with drift
            let group_center_offset = Vec3::new(
                formation.angle_offset.cos() * formation.orbit_radius,
                (formation.angle_offset * 2.0).sin() * 5.0 + 15.0,  // Higher above frigates
                formation.angle_offset.sin() * formation.orbit_radius + (formation_idx as f32) * 8.0  // Forward stagger
            ) + group_drift;
            
            for ship in &mut formation.ships {
                // Individual ship cyclical drift (very subtle)
                let ship_drift_phase = elapsed * 0.2 + ship.phase;
                let ship_drift = Vec3::new(
                    0.3 * ship_drift_phase.sin(),
                    0.2 * (ship_drift_phase * 1.3).cos(),
                    0.3 * (ship_drift_phase * 0.7).sin()
                );
                
                // Final position: frigate + group offset + V formation offset + drifts
                let new_position = frigate.position + group_center_offset + ship.local_offset + ship_drift;
                
                // Update ship transform
                use rust_engine::ecs::components::TransformComponent;
                if let Some(transform) = self.world.get_component_mut::<TransformComponent>(ship.entity) {
                    transform.position = new_position;
                    // Keep ships pointing forward (same orientation as frigate)
                    transform.rotation = Quat::from_euler_angles(0.0, 0.0, 0.0);
                    self.scene_manager.mark_entity_dirty(ship.entity);
                }
            }
        }
        
        // Sync all entity transforms to scene manager for rendering
        self.scene_manager.sync_from_world(&mut self.world);
        
        // Note: GPU handle allocation and transform updates are handled automatically
        // by render_entities_from_queue() in render_frame()
        
        
        // Update flyby formation timer and spawn
        self.flyby_spawn_timer -= delta_time;
        if self.flyby_spawn_timer <= 0.0 && self.flyby_group.is_none() {
            self.spawn_flyby_formation(0, -50.0);
            self.flyby_spawn_timer = 15.0;  // Next flyby in 15 seconds
        }
        
        // Update second flyby formation timer and spawn
        self.flyby_spawn_timer_2 -= delta_time;
        if self.flyby_spawn_timer_2 <= 0.0 && self.flyby_group_2.is_none() {
            self.spawn_flyby_formation(1, -70.0);  // Different frigate, further back
            self.flyby_spawn_timer_2 = 18.0;  // Next flyby in 18 seconds
        }
        
        // Update flyby formation movement
        if let Some(ref mut flyby) = self.flyby_group {
            const FLYBY_SPEED: f32 = 25.0;
            let mut all_past_camera = true;
            
            for ship in &mut flyby.ships {
                // Move forward
                let mut current_pos = Vec3::zeros();
                if let Some(transform) = self.world.get_component::<TransformComponent>(ship.entity) {
                    current_pos = transform.position;
                }
                
                current_pos.z += FLYBY_SPEED * delta_time;
                
                // Update position
                if let Some(transform) = self.world.get_component_mut::<TransformComponent>(ship.entity) {
                    transform.position = current_pos;
                    self.scene_manager.mark_entity_dirty(ship.entity);
                }
                
                // Check if past camera (camera is at z=120)
                if current_pos.z < 150.0 {
                    all_past_camera = false;
                }
            }
            
            // Despawn if all ships passed camera
            if all_past_camera {
                log::info!("Flyby formation passed camera, despawning");
                for ship in &flyby.ships {
                    self.scene_manager.destroy_entity(&mut self.world, &mut self.graphics_engine, ship.entity);
                }
                self.flyby_group = None;
            }
        }
        
        // Update second flyby formation movement
        if let Some(ref mut flyby) = self.flyby_group_2 {
            const FLYBY_SPEED: f32 = 25.0;
            let mut all_past_camera = true;
            
            for ship in &mut flyby.ships {
                // Move forward
                let mut current_pos = Vec3::zeros();
                if let Some(transform) = self.world.get_component::<TransformComponent>(ship.entity) {
                    current_pos = transform.position;
                }
                
                current_pos.z += FLYBY_SPEED * delta_time;
                
                // Update position
                if let Some(transform) = self.world.get_component_mut::<TransformComponent>(ship.entity) {
                    transform.position = current_pos;
                    self.scene_manager.mark_entity_dirty(ship.entity);
                }
                
                // Check if past camera (camera is at z=120)
                if current_pos.z < 150.0 {
                    all_past_camera = false;
                }
            }
            
            // Despawn if all ships passed camera
            if all_past_camera {
                log::info!("Flyby formation 2 passed camera, despawning");
                for ship in &flyby.ships {
                    self.scene_manager.destroy_entity(&mut self.world, &mut self.graphics_engine, ship.entity);
                }
                self.flyby_group_2 = None;
            }
        }
        
        // Manual camera controls (replace automatic following)
        let camera_speed = 20.0 * delta_time;
        let rotation_speed = 2.0 * delta_time;
        
        // Update yaw and pitch
        if self.camera_yaw_left {
            self.camera_yaw += rotation_speed;
        }
        if self.camera_yaw_right {
            self.camera_yaw -= rotation_speed;
        }
        if self.camera_pitch_up {
            self.camera_pitch += rotation_speed;
            self.camera_pitch = self.camera_pitch.min(std::f32::consts::PI / 2.0 - 0.1);
        }
        if self.camera_pitch_down {
            self.camera_pitch -= rotation_speed;
            self.camera_pitch = self.camera_pitch.max(-std::f32::consts::PI / 2.0 + 0.1);
        }
        
        // Calculate forward and right vectors based on yaw
        let forward = Vec3::new(
            -self.camera_yaw.sin(),
            0.0,
            -self.camera_yaw.cos()
        );
        let right = Vec3::new(
            self.camera_yaw.cos(),
            0.0,
            -self.camera_yaw.sin()
        );
        
        // Update camera position based on WASD input
        let mut current_pos = self.camera.position;
        if self.camera_move_forward {
            current_pos = current_pos + forward * camera_speed;
        }
        if self.camera_move_backward {
            current_pos = current_pos - forward * camera_speed;
        }
        if self.camera_move_left {
            current_pos = current_pos - right * camera_speed;
        }
        if self.camera_move_right {
            current_pos = current_pos + right * camera_speed;
        }
        
        // Calculate look-at point based on yaw and pitch
        let look_direction = Vec3::new(
            -self.camera_yaw.sin() * self.camera_pitch.cos(),
            self.camera_pitch.sin(),
            -self.camera_yaw.cos() * self.camera_pitch.cos()
        );
        let lookat_point = current_pos + look_direction * 10.0;
        
        self.camera.set_position(current_pos);
        self.camera.look_at(lookat_point, Vec3::new(0.0, 1.0, 0.0));
        
        // Move particles backward (simulate ships moving forward through particle field)
        const PARTICLE_SPEED: f32 = 15.0;  // Speed particles move backward
        for particle in &mut self.particles {
            particle.position.z -= PARTICLE_SPEED * delta_time;
            
            // Update particle entity transform
            if let Some(transform) = self.world.get_component_mut::<TransformComponent>(particle.entity) {
                transform.position = particle.position;
                self.scene_manager.mark_entity_dirty(particle.entity);
            }
            
            // CRITICAL: Also update the LightComponent position!
            if let Some(light) = self.world.get_component_mut::<LightComponent>(particle.entity) {
                light.position = particle.position;
            }
        }
        
        // Mark particles for fade-out when they go behind threshold
        let despawn_threshold_z = -150.0;  // Behind camera and ships
        for particle in &mut self.particles {
            if particle.position.z < despawn_threshold_z && particle.fade_timer.is_none() {
                log::info!("Particle entity {:?} at z={:.1} starting fade-out", particle.entity, particle.position.z);
                particle.fade_timer = Some(1.0);  // Start 1-second fade
            }
        }
        
        // Update fading particles and despawn when fade completes
        let mut destroyed_entities = Vec::new();
        self.particles.retain_mut(|particle| {
            if let Some(fade_time) = particle.fade_timer {
                let new_fade_time = fade_time - delta_time;
                
                if new_fade_time <= 0.0 {
                    // Fade complete, despawn
                    log::info!("Particle entity {:?} fade complete, despawning", particle.entity);
                    destroyed_entities.push(particle.entity);
                    false
                } else {
                    // Update fade - use ALPHA transparency instead of scale
                    particle.fade_timer = Some(new_fade_time);
                    let alpha = new_fade_time / 1.0;  // 1.0 to 0.0 over 1 second
                    
                    // Clone material and update alpha (Option B approach)
                    let mut faded_material = particle.base_material.clone();
                    
                    // Update alpha field based on material type
                    match &mut faded_material.material_type {
                        MaterialType::Transparent { base_material, alpha_mode: _ } => {
                            // Update alpha in the base material params
                            match base_material.as_mut() {
                                MaterialType::StandardPBR(ref mut params) => {
                                    params.alpha = alpha;
                                }
                                MaterialType::Unlit(ref mut params) => {
                                    params.alpha = alpha;
                                }
                                _ => {
                                    log::warn!("Unexpected nested transparent material!");
                                }
                            }
                        }
                        _ => {
                            // Shouldn't happen since we create with build_transparent()
                            log::warn!("Particle material is not transparent!");
                        }
                    }
                    
                    // Update material through ECS (proper architecture)
                    if let Some(renderable) = self.world.get_component_mut::<RenderableComponent>(particle.entity) {
                        renderable.set_material(faded_material);
                        self.scene_manager.mark_entity_dirty(particle.entity);
                    }
                    
                    // Fade light intensity too
                    if let Some(light) = self.world.get_component_mut::<LightComponent>(particle.entity) {
                        light.intensity = 30.0 * alpha;
                    }
                    
                    true
                }
            } else {
                true
            }
        });
        
        for entity in destroyed_entities {
            self.world.remove_component::<LightComponent>(entity);
            self.scene_manager.destroy_entity(&mut self.world, &mut self.graphics_engine, entity);
        }
        
        // Spawn new particles in front of ships (off-camera)
        let spawn_z_position = 200.0;  // Far in front of camera
        let spawn_threshold = 180.0;  // Spawn when furthest particle is closer than this
        
        if self.particles.len() < MAX_PARTICLES {
            let furthest_particle_z = self.particles.iter()
                .map(|p| p.position.z)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap_or(f32::MIN);
            
            if furthest_particle_z < spawn_threshold {
                let spawn_count = PARTICLES_PER_SPAWN.min(MAX_PARTICLES - self.particles.len());
                if spawn_count > 0 {
                    self.spawn_particle_lights_count(spawn_z_position, spawn_count);
                    log::info!("Spawned {} particles at z={:.1}, total: {}", spawn_count, spawn_z_position, self.particles.len());
                }
            }
        }
    }
    
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update FPS text
        let fps_text = format!("FPS: {:.1}", self.current_fps);
        self.ui_manager.update_text(self.fps_label_id, fps_text);
        
        // Update UI state
        self.ui_manager.update(0.016);
        
        // DEBUG: Check RANDOM particle materials before rendering to see variety!
        if self.frame_count % 60 == 0 && self.particles.len() >= 5 {
            use rand::Rng;
            let mut rng = rand::thread_rng();
            log::info!("DEBUG: Checking 5 random particles out of {} total", self.particles.len());
            for _ in 0..5 {
                let i = rng.gen_range(0..self.particles.len());
                let particle = &self.particles[i];
                if let Some(renderable) = self.world.get_component::<RenderableComponent>(particle.entity) {
                    log::info!("  Particle idx={} (Entity {:?}): base_color={:?}, emission={:?}", 
                              i, particle.entity,
                              match &renderable.material.material_type {
                                  MaterialType::Transparent { base_material, .. } => {
                                      match base_material.as_ref() {
                                          MaterialType::StandardPBR(params) => params.base_color,
                                          _ => [0.0, 0.0, 0.0].into()
                                      }
                                  }
                                  _ => [0.0, 0.0, 0.0].into()
                              },
                              match &renderable.material.material_type {
                                  MaterialType::Transparent { base_material, .. } => {
                                      match base_material.as_ref() {
                                          MaterialType::StandardPBR(params) => params.emission,
                                          _ => [0.0, 0.0, 0.0].into()
                                      }
                                  }
                                  _ => [0.0, 0.0, 0.0].into()
                              });
                }
            }
        }
        
        // CRITICAL: Sync entities to render queue and allocate/update GPU handles
        // This must be called before render_frame() to ensure entities have GPU resources
        let render_queue = self.scene_manager.build_render_queue();
        self.graphics_engine.render_entities_from_queue(&render_queue)?;
        
        // Build multi-light environment from entity system
        let multi_light_env = self.lighting_system.build_multi_light_environment(
            &self.world,
            Vec3::new(0.15, 0.12, 0.18),
            0.1
        ).clone();
        
        // Export UI data for rendering
        let ui_data = self.ui_manager.get_render_data();
        
        // Render frame
        self.graphics_engine.render_frame(
            &rust_engine::render::RenderFrameData {
                camera: &self.camera,
                lights: &multi_light_env,
                ui: &ui_data,
            },
            &mut self.window,
        )?;
        
        // Dispatch UI events
        self.ui_manager.dispatch_events();
        
        Ok(())
    }
}

impl Drop for DynamicTeapotApp {
    fn drop(&mut self) {
        // Cleanup handled by renderer's drop implementation
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic hook for better error reporting
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("PANIC occurred: {:?}", panic_info);
        
        if let Some(location) = panic_info.location() {
            eprintln!("Panic location: {}:{}:{}", location.file(), location.line(), location.column());
        }
        
        if let Some(payload) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("Panic message: {}", payload);
        } else if let Some(payload) = panic_info.payload().downcast_ref::<String>() {
            eprintln!("Panic message: {}", payload);
        }
    }));
    
    // Initialize logging
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info) // Reduced verbosity for dynamic demo
        .init();
    
    log::info!("Starting Rusteroids Dynamic Teapot Demo");
    
    // Create and run the application
    log::info!("Creating dynamic application instance...");
    let mut app = DynamicTeapotApp::new();
    
    log::info!("Running dynamic application...");
    let result = app.run();
    
    match result {
        Ok(()) => {
            log::info!("Dynamic teapot demo completed successfully");
            Ok(())
        }
        Err(e) => {
            log::error!("Dynamic teapot demo failed: {:?}", e);
            Err(e)
        }
    }
}
