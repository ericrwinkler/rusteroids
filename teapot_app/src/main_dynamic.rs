//! Dynamic Teapot demo application with multiple random teapots and lights
//! 
//! This demonstrates the engine's rendering capabilities by displaying
//! multiple teapots with random properties and dynamic lighting effects.

#![allow(dead_code)] // Allow unused fields in structs for demo purposes

use rust_engine::assets::obj_loader::ObjLoader;
use rust_engine::ecs::{
    World, Entity, LightFactory, LightingSystem as EcsLightingSystem, 
    TransformComponent, LightComponent,
    SceneManager, SceneConfig, RenderableComponent, MovementComponent
};
use rust_engine::render::{
    Camera,
    Mesh,
    Vertex,
    Material,
    StandardMaterialParams,
    lighting::MultiLightEnvironment,
    Renderer,
    VulkanRendererConfig,
    WindowHandle,
    material::MaterialId,
    GameObject,
    shared_resources::SharedRenderingResources,
};
use rust_engine::foundation::math::{Vec3, Vec4, Mat4, Mat4Ext};
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;
use rand::prelude::*;

#[derive(Clone)]
struct TeapotInstance {
    entity: Option<Entity>,
    position: Vec3,
    rotation: Vec3,      // Rotation angles in radians
    scale: f32,
    spin_speed: Vec3,    // Angular velocity for each axis
    orbit_radius: f32,   // Radius for orbital motion
    orbit_speed: f32,    // Speed of orbital motion
    orbit_center: Vec3,  // Center point for orbit
    material_index: usize,
}

#[derive(Clone)]
struct LightInstance {
    entity: Entity,
    light_type: LightType,
    base_position: Vec3,
    current_position: Vec3,
    color: Vec3,
    base_intensity: f32,
    movement_pattern: MovementPattern,
    effect_pattern: EffectPattern,
    phase_offset: f32,   // Phase offset for animation variety
}

#[derive(Clone)]
enum LightType {
    Directional,
    Point,
}

#[derive(Clone)]
enum MovementPattern {
    Static,
    Circular { radius: f32, speed: f32, axis: Vec3 },
    Figure8 { radius: f32, speed: f32 },
    Bounce { amplitude: f32, speed: f32, direction: Vec3 },
}

#[derive(Clone)]
enum EffectPattern {
    Steady,
    Pulse { frequency: f32, amplitude: f32 },
    Flicker { base_rate: f32, chaos: f32 },
    Breathe { frequency: f32 },
}

pub struct DynamicTeapotApp {
    window: WindowHandle,
    renderer: Renderer,
    camera: Camera,
    
    // New SceneManager for batch rendering
    scene_manager: SceneManager,
    
    // Traditional lighting system (keep for compatibility)
    world: World,
    lighting_system: EcsLightingSystem,
    
    // Dynamic content (entities managed by SceneManager now)
    teapot_entities: Vec<Entity>,
    light_instances: Vec<LightInstance>,
    teapot_mesh: Option<Mesh>,
    material_ids: Vec<MaterialId>,
    
    // GameObject rendering system
    teapot_game_objects: Vec<GameObject>,
    shared_resources: Option<SharedRenderingResources>,
    
    // Animation state
    start_time: Instant,
    last_material_switch: Instant,
    camera_orbit_speed: f32,
    camera_orbit_radius: f32,
    camera_height_oscillation: f32,
    last_shown_teapot_index: usize, // Track which teapot was shown last frame
}

impl DynamicTeapotApp {
    pub fn new() -> Self {
        log::info!("Creating dynamic teapot demo application...");
        
        // Create window
        log::info!("Creating window...");
        let mut window = WindowHandle::new(1200, 800, "Rusteroids - Dynamic Teapot Demo");
        log::info!("Window created successfully");
        
        // Create renderer
        log::info!("Creating Vulkan renderer...");
        let renderer_config = VulkanRendererConfig::new("Rusteroids - Dynamic Teapot Demo")
            .with_version(1, 0, 0)
            .with_shader_paths(
                "target/shaders/standard_pbr_vert.spv".to_string(),
                "target/shaders/standard_pbr_frag.spv".to_string()
            );
        let renderer = Renderer::new_from_window(&mut window, &renderer_config)
            .expect("Failed to create renderer");
        log::info!("Vulkan renderer created successfully");
        
        // Create camera with wider view for multiple teapots
        log::info!("Creating camera...");
        let mut camera = Camera::perspective(
            Vec3::new(0.0, 8.0, 15.0), // Higher and further back for better overview
            60.0, // Wider FOV for better scene coverage
            1200.0 / 800.0,
            0.1,
            100.0
        );
        
        // Position camera to overlook the scene
        camera.set_position(Vec3::new(0.0, 8.0, 15.0));
        camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        
        // Create diverse materials for teapots
        let teapot_materials = Self::create_teapot_materials();
        
        // Initialize SceneManager for batch rendering
        let mut scene_manager = SceneManager::new();
        
        // Initialize traditional ECS for lighting
        let mut world = World::new();
        let lighting_system = EcsLightingSystem::new();
        
        // Store material IDs for teapot creation
        let material_ids: Vec<MaterialId> = (0..teapot_materials.len())
            .map(|i| MaterialId(i as u32))
            .collect();
        
        // Generate random scene content using SceneManager
        let mut rng = thread_rng();
        let mut teapot_entities = Vec::new();
        
        // Create 10 random teapots with movement
        for i in 0..10 {
            let position = Vec3::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-2.0..4.0),
                rng.gen_range(-10.0..10.0),
            );
            let velocity = Vec3::new(
                rng.gen_range(-2.0..2.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-2.0..2.0),
            );
            let material_id = material_ids[i % material_ids.len()];
            
            let entity = scene_manager.create_moving_teapot(material_id, position, velocity);
            teapot_entities.push(entity);
        }
        
        let light_instances = Self::generate_random_lights(&mut world, &mut rng);
        
        log::info!("Generated {} teapots and {} lights", teapot_entities.len(), light_instances.len());
        
        Self {
            window,
            renderer,
            camera,
            scene_manager,
            world,
            lighting_system,
            teapot_entities,
            light_instances,
            teapot_mesh: None,
            material_ids,
            teapot_game_objects: Vec::new(),
            shared_resources: None,
            last_material_switch: Instant::now(),
            start_time: Instant::now(),
            camera_orbit_speed: 0.15, // Slow camera orbit
            camera_orbit_radius: 20.0,
            camera_height_oscillation: 3.0,
            last_shown_teapot_index: usize::MAX, // Initialize to invalid index
        }
    }
    
    fn create_teapot_materials() -> Vec<Material> {
        vec![
            // Ceramic variations
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.9, 0.85, 0.7), // Cream ceramic
                alpha: 1.0,
                metallic: 0.05,
                roughness: 0.4,
                ..Default::default()
            }).with_name("Cream Ceramic"),
            
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.7, 0.4, 0.3), // Terracotta
                alpha: 1.0,
                metallic: 0.1,
                roughness: 0.6,
                ..Default::default()
            }).with_name("Terracotta"),
            
            // Metallic variations
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.95, 0.93, 0.88), // Silver
                alpha: 1.0,
                metallic: 0.9,
                roughness: 0.1,
                ..Default::default()
            }).with_name("Polished Silver"),
            
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(1.0, 0.8, 0.2), // Gold
                alpha: 1.0,
                metallic: 0.9,
                roughness: 0.15,
                ..Default::default()
            }).with_name("Gold"),
            
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.7, 0.3, 0.2), // Copper
                alpha: 1.0,
                metallic: 0.8,
                roughness: 0.2,
                ..Default::default()
            }).with_name("Copper"),
            
            // Colored plastics
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.2, 0.6, 0.9), // Blue plastic
                alpha: 1.0,
                metallic: 0.0,
                roughness: 0.7,
                ..Default::default()
            }).with_name("Blue Plastic"),
            
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.8, 0.2, 0.3), // Red plastic
                alpha: 1.0,
                metallic: 0.0,
                roughness: 0.8,
                ..Default::default()
            }).with_name("Red Plastic"),
            
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.3, 0.7, 0.2), // Green plastic
                alpha: 1.0,
                metallic: 0.0,
                roughness: 0.75,
                ..Default::default()
            }).with_name("Green Plastic"),
            
            // Special materials
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.1, 0.1, 0.1), // Matte black
                alpha: 1.0,
                metallic: 0.0,
                roughness: 0.9,
                ..Default::default()
            }).with_name("Matte Black"),
            
            Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(0.6, 0.2, 0.8), // Purple gem-like
                alpha: 1.0,
                metallic: 0.3,
                roughness: 0.1,
                ..Default::default()
            }).with_name("Purple Gem"),
        ]
    }
    
    fn generate_random_teapots(rng: &mut ThreadRng, materials: &[Material]) -> Vec<TeapotInstance> {
        let num_teapots = rng.gen_range(3..=10); // 3-10 teapots
        let mut teapots = Vec::with_capacity(num_teapots);
        
        for i in 0..num_teapots {
            // Position teapots in a rough grid with some randomness
            let grid_size = (num_teapots as f32).sqrt().ceil() as i32;
            let x_index = i as i32 % grid_size;
            let z_index = i as i32 / grid_size;
            
            let base_x = (x_index as f32 - grid_size as f32 * 0.5) * 6.0;
            let base_z = (z_index as f32 - grid_size as f32 * 0.5) * 6.0;
            
            // Add randomness to position
            let position = Vec3::new(
                base_x + rng.gen_range(-2.0..2.0),
                rng.gen_range(-1.0..1.0), // Slight height variation
                base_z + rng.gen_range(-2.0..2.0),
            );
            
            // Random orbital motion parameters
            let orbit_center = Vec3::new(
                rng.gen_range(-5.0..5.0),
                0.0,
                rng.gen_range(-5.0..5.0),
            );
            
            teapots.push(TeapotInstance {
                entity: None,
                position,
                rotation: Vec3::new(
                    rng.gen_range(0.0..std::f32::consts::TAU),
                    rng.gen_range(0.0..std::f32::consts::TAU),
                    rng.gen_range(0.0..std::f32::consts::TAU),
                ),
                scale: rng.gen_range(0.5..2.0), // Random size
                spin_speed: Vec3::new(
                    rng.gen_range(-1.0..1.0),
                    rng.gen_range(-1.5..1.5), // Slightly faster Y rotation
                    rng.gen_range(-0.8..0.8),
                ),
                orbit_radius: rng.gen_range(1.0..4.0),
                orbit_speed: rng.gen_range(0.2..0.8),
                orbit_center,
                material_index: rng.gen_range(0..materials.len()),
            });
        }
        
        teapots
    }
    
    fn generate_random_lights(world: &mut World, rng: &mut ThreadRng) -> Vec<LightInstance> {
        let num_lights = rng.gen_range(4..=8); // 4-8 lights
        let mut lights = Vec::with_capacity(num_lights);
        
        // Always include one main directional light
        let main_light_entity = world.create_entity();
        world.add_component(main_light_entity, TransformComponent::identity());
        world.add_component(main_light_entity, LightFactory::directional(
            Vec3::new(-0.5, -1.0, 0.3), // Main directional light
            Vec3::new(1.0, 0.95, 0.9),  // Warm white
            0.6
        ));
        
        lights.push(LightInstance {
            entity: main_light_entity,
            light_type: LightType::Directional,
            base_position: Vec3::zeros(), // Directional lights don't use position
            current_position: Vec3::zeros(),
            color: Vec3::new(1.0, 0.95, 0.9),
            base_intensity: 0.6,
            movement_pattern: MovementPattern::Static,
            effect_pattern: EffectPattern::Steady,
            phase_offset: 0.0,
        });
        
        // Generate random point lights
        for _i in 1..num_lights {
            let position = Vec3::new(
                rng.gen_range(-15.0..15.0),
                rng.gen_range(2.0..12.0), // Keep lights above teapots
                rng.gen_range(-15.0..15.0),
            );
            
            // Random colors with good saturation
            let hue = rng.gen_range(0.0..360.0);
            let color = Self::hue_to_rgb(hue);
            
            let entity = world.create_entity();
            world.add_component(entity, TransformComponent::from_position(position));
            world.add_component(entity, LightFactory::point(
                position,
                color,
                rng.gen_range(0.8..2.0), // Random intensity
                rng.gen_range(8.0..20.0) // Random range
            ));
            
            // Random movement pattern
            let movement = match rng.gen_range(0..4) {
                0 => MovementPattern::Static,
                1 => MovementPattern::Circular {
                    radius: rng.gen_range(3.0..8.0),
                    speed: rng.gen_range(0.3..1.2),
                    axis: Vec3::new(0.0, 1.0, 0.0), // Horizontal circles
                },
                2 => MovementPattern::Figure8 {
                    radius: rng.gen_range(2.0..6.0),
                    speed: rng.gen_range(0.4..1.0),
                },
                _ => MovementPattern::Bounce {
                    amplitude: rng.gen_range(2.0..5.0),
                    speed: rng.gen_range(0.5..1.5),
                    direction: Vec3::new(
                        rng.gen_range(-1.0..1.0),
                        rng.gen_range(0.2..1.0), // Prefer vertical bouncing
                        rng.gen_range(-1.0..1.0),
                    ).normalize(),
                },
            };
            
            // Random effect pattern
            let effect = match rng.gen_range(0..4) {
                0 => EffectPattern::Steady,
                1 => EffectPattern::Pulse {
                    frequency: rng.gen_range(0.5..3.0),
                    amplitude: rng.gen_range(0.3..0.8),
                },
                2 => EffectPattern::Flicker {
                    base_rate: rng.gen_range(5.0..15.0),
                    chaos: rng.gen_range(0.2..0.6),
                },
                _ => EffectPattern::Breathe {
                    frequency: rng.gen_range(0.3..1.2),
                },
            };
            
            lights.push(LightInstance {
                entity,
                light_type: LightType::Point,
                base_position: position,
                current_position: position,
                color,
                base_intensity: rng.gen_range(0.8..2.0),
                movement_pattern: movement,
                effect_pattern: effect,
                phase_offset: rng.gen_range(0.0..std::f32::consts::TAU),
            });
        }
        
        lights
    }
    
    fn hue_to_rgb(hue: f32) -> Vec3 {
        let h = hue / 60.0;
        let c = 1.0; // Full saturation
        let x = 1.0 - (h % 2.0 - 1.0).abs();
        
        let (r, g, b) = match h as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        
        Vec3::new(r, g, b)
    }
    
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing dynamic teapot demo...");
        
        // Load teapot mesh
        match ObjLoader::load_obj("resources/models/teapot_with_normals.obj") {
            Ok(mut mesh) => {
                // Scale mesh appropriately
                let max_extent = mesh.vertices.iter()
                    .map(|v| v.position[0].abs().max(v.position[1].abs()).max(v.position[2].abs()))
                    .fold(0.0f32, f32::max);
                
                if max_extent > 3.0 {
                    let scale_factor = 3.0 / max_extent;
                    for vertex in &mut mesh.vertices {
                        vertex.position[0] *= scale_factor;
                        vertex.position[1] *= scale_factor;
                        vertex.position[2] *= scale_factor;
                    }
                }
                
                log::info!("Loaded teapot mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.teapot_mesh = Some(mesh);
            }
            Err(e) => {
                log::warn!("Failed to load teapot.obj: {:?}, using fallback cube", e);
                self.teapot_mesh = Some(Mesh::cube());
            }
        }
        
        // Initialize GameObject system
        if let Some(ref teapot_mesh) = self.teapot_mesh {
            // Create GameObjects for each teapot
            let materials = Self::create_teapot_materials();
            let num_teapots = self.teapot_entities.len();
            
            // Create SharedRenderingResources for efficient multiple object rendering
            match self.renderer.create_shared_rendering_resources(teapot_mesh, &materials) {
                Ok(shared_resources) => {
                    log::info!("Created SharedRenderingResources successfully");
                    self.shared_resources = Some(shared_resources);
                }
                Err(e) => {
                    log::error!("Failed to create SharedRenderingResources: {}", e);
                    return Err(e);
                }
            }
            
            for i in 0..num_teapots {
                let material_index = i % materials.len();
                let material = materials[material_index].clone();
                
                // Create GameObject with simple factory method
                match self.renderer.create_game_object(
                    rust_engine::foundation::math::Vec3::new(0.0, 0.0, 0.0), // Will be updated in render
                    rust_engine::foundation::math::Vec3::new(0.0, 0.0, 0.0), // No rotation initially
                    rust_engine::foundation::math::Vec3::new(0.8, 0.8, 0.8), // Scale
                    material.clone()
                ) {
                    Ok(game_object) => {
                        self.teapot_game_objects.push(game_object);
                    }
                    Err(e) => {
                        log::warn!("Failed to create GameObject {}: {}", i, e);
                        // Create a basic GameObject without GPU resources for now
                        let game_object = GameObject::new(
                            rust_engine::foundation::math::Vec3::new(0.0, 0.0, 0.0),
                            rust_engine::foundation::math::Vec3::new(0.0, 0.0, 0.0),
                            rust_engine::foundation::math::Vec3::new(0.8, 0.8, 0.8),
                            material
                        );
                        self.teapot_game_objects.push(game_object);
                    }
                }
            }
            
            log::info!("Created {} GameObjects for teapot rendering", self.teapot_game_objects.len());
        }
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting dynamic teapot demo...");
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
                    WindowEvent::FramebufferSize(width, height) => {
                        framebuffer_resized = true;
                        if width > 0 && height > 0 {
                            self.camera.set_aspect_ratio(width as f32 / height as f32);
                        }
                    }
                    WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                        // Regenerate scene with new random content
                        self.regenerate_scene();
                    }
                    WindowEvent::Key(Key::R, _, Action::Press, _) => {
                        // Reset camera to default position
                        self.camera.set_position(Vec3::new(0.0, 8.0, 15.0));
                        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
                    }
                    _ => {}
                }
            }
            
            if framebuffer_resized {
                log::info!("Framebuffer resized detected, recreating swapchain...");
                self.renderer.recreate_swapchain(&mut self.window);
                let (width, height) = self.renderer.get_swapchain_extent();
                if width > 0 && height > 0 {
                    self.camera.set_aspect_ratio(width as f32 / height as f32);
                }
                framebuffer_resized = false;
            }
            
            // Update animations
            self.update_scene();
            
            // Render frame
            if let Err(e) = self.render_frame() {
                log::error!("Failed to render frame: {:?}", e);
                break;
            }
        }
        
        log::info!("Dynamic teapot demo completed");
        Ok(())
    }
    
    fn regenerate_scene(&mut self) {
        log::info!("Regenerating scene with new random content...");
        let mut rng = thread_rng();
        
        // Clear existing teapots from scene manager
        for entity in &self.teapot_entities {
            self.scene_manager.remove_entity(*entity);
        }
        self.teapot_entities.clear();
        
        // Clear existing lights (except keep world structure)
        self.world = World::new();
        
        // Create new teapots with random properties
        for i in 0..10 {
            let position = Vec3::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-2.0..4.0),
                rng.gen_range(-10.0..10.0),
            );
            let velocity = Vec3::new(
                rng.gen_range(-2.0..2.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-2.0..2.0),
            );
            let material_id = self.material_ids[i % self.material_ids.len()];
            
            let entity = self.scene_manager.create_moving_teapot(material_id, position, velocity);
            self.teapot_entities.push(entity);
        }
        
        // Generate new lights
        self.light_instances = Self::generate_random_lights(&mut self.world, &mut rng);
        
        log::info!("Regenerated {} teapots and {} lights", 
                  self.teapot_entities.len(), self.light_instances.len());
    }
    
    fn update_scene(&mut self) {
        let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
        let delta_time = 0.016; // Assume ~60 FPS for now
        
        // Update camera with orbital motion
        let camera_angle = elapsed_seconds * self.camera_orbit_speed;
        let camera_height = 8.0 + (elapsed_seconds * 0.3).sin() * self.camera_height_oscillation;
        let camera_x = camera_angle.cos() * self.camera_orbit_radius;
        let camera_z = camera_angle.sin() * self.camera_orbit_radius;
        
        self.camera.set_position(Vec3::new(camera_x, camera_height, camera_z));
        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        
        // Update SceneManager (handles all teapot movement automatically)
        self.scene_manager.update(delta_time, &mut self.renderer);
        
        // Simplified light updates - keep existing light system working
        for light_instance in &mut self.light_instances {
            // Simple light position updates
            match &light_instance.movement_pattern {
                MovementPattern::Static => {
                    light_instance.current_position = light_instance.base_position;
                }
                MovementPattern::Circular { radius, speed, axis: _ } => {
                    let angle = elapsed_seconds * speed + light_instance.phase_offset;
                    let offset_x = angle.cos() * radius;
                    let offset_z = angle.sin() * radius;
                    light_instance.current_position = light_instance.base_position + Vec3::new(offset_x, 0.0, offset_z);
                }
                MovementPattern::Figure8 { radius, speed } => {
                    let t = elapsed_seconds * speed + light_instance.phase_offset;
                    let x = t.sin() * radius;
                    let z = (t * 2.0).sin() * radius * 0.5;
                    light_instance.current_position = light_instance.base_position + Vec3::new(x, 0.0, z);
                }
                MovementPattern::Bounce { amplitude, speed, direction } => {
                    let bounce = (elapsed_seconds * speed + light_instance.phase_offset).sin() * amplitude;
                    light_instance.current_position = light_instance.base_position + direction * bounce;
                }
            }
            
            // Update the light component in ECS
            if let Some(light_comp) = self.world.get_component_mut::<LightComponent>(light_instance.entity) {
                light_comp.position = light_instance.current_position;
            }
        }
    }
    
    fn build_multi_light_environment_from_entities(&mut self) -> &MultiLightEnvironment {
        let multi_light_env = self.lighting_system.build_multi_light_environment(
            &self.world,
            Vec3::new(0.15, 0.12, 0.18), // Ambient color
            0.1                           // Ambient intensity
        );
        
        log::trace!("MultiLight Environment - Directional: {}, Point: {}, Spot: {}", 
                   multi_light_env.header.directional_light_count,
                   multi_light_env.header.point_light_count,
                   multi_light_env.header.spot_light_count);
        
        multi_light_env
    }
    
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin the frame for batch rendering
        self.renderer.begin_frame();
        
        // Set camera for the frame (this needs to be done before any rendering)
        self.renderer.set_camera(&self.camera);
        
        // Build multi-light environment from entity system
        let multi_light_env = self.build_multi_light_environment_from_entities().clone();
        self.renderer.set_multi_light_environment(&multi_light_env);
        
        // PROPER VULKAN MULTIPLE OBJECTS: Command recording with per-object uniform buffers
        if let Some(ref teapot_mesh) = self.teapot_mesh {
            if !self.teapot_game_objects.is_empty() {
                let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
                
                // Begin command recording for multiple objects
                self.renderer.begin_command_recording()?;
                
                // Bind shared geometry once for all objects
                if let Some(ref shared_resources) = self.shared_resources {
                    self.renderer.bind_shared_geometry(shared_resources)?;
                }
                
                // Arrange teapots in a grid pattern
                let grid_size = (self.teapot_game_objects.len() as f32).sqrt().ceil() as usize;
                let spacing = 4.0; // Distance between teapots
                
                // Record draw commands for all teapots
                for (i, game_object) in self.teapot_game_objects.iter_mut().enumerate() {
                    let row = i / grid_size;
                    let col = i % grid_size;
                    
                    // Calculate grid position centered around origin
                    let offset_x = (col as f32 - grid_size as f32 * 0.5) * spacing;
                    let offset_z = (row as f32 - grid_size as f32 * 0.5) * spacing;
                    
                    // Individual animation for each teapot
                    let phase_offset = i as f32 * 0.5; // Different phase for each teapot
                    let y_offset = (elapsed_seconds * 1.2 + phase_offset).sin() * 1.5;
                    let rotation_y = elapsed_seconds * 0.4 + phase_offset;
                    
                    // Update GameObject transforms
                    game_object.position = rust_engine::foundation::math::Vec3::new(offset_x, y_offset, offset_z);
                    game_object.rotation = rust_engine::foundation::math::Vec3::new(0.0, rotation_y, 0.0);
                    game_object.scale = rust_engine::foundation::math::Vec3::new(1.0, 1.0, 1.0); // Normal size
                    
                    // Record draw command for this object (sets per-object uniforms)
                    if let Some(ref shared_resources) = self.shared_resources {
                        self.renderer.record_object_draw(game_object, shared_resources)?;
                    }
                }
                
                // Submit all recorded commands and present
                self.renderer.submit_commands_and_present(&mut self.window)?;
                
                // Log performance info occasionally
                static mut LAST_LOG_TIME: f32 = 0.0;
                unsafe {
                    if elapsed_seconds - LAST_LOG_TIME >= 2.0 {
                        log::info!("Rendered {} teapots using proper Vulkan multiple objects pattern ({} vertices shared, {} triangles per teapot)", 
                                  self.teapot_game_objects.len(),
                                  teapot_mesh.vertices.len(),
                                  teapot_mesh.indices.len() / 3);
                        LAST_LOG_TIME = elapsed_seconds;
                    }
                }
            }
        } else {
            log::warn!("No teapot mesh loaded");
            // Still need to end the frame even if no rendering
            self.renderer.end_frame(&mut self.window)?;
        }
        
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
    
    let result = std::panic::catch_unwind(|| {
        let mut app = DynamicTeapotApp::new();
        log::info!("Running dynamic application...");
        app.run()
    });
    
    match result {
        Ok(Ok(())) => {
            log::info!("Dynamic teapot demo completed successfully");
            Ok(())
        }
        Ok(Err(e)) => {
            log::error!("Dynamic teapot demo failed: {:?}", e);
            Err(e)
        }
        Err(panic) => {
            log::error!("Dynamic teapot demo panicked");
            if let Some(s) = panic.downcast_ref::<String>() {
                log::error!("Panic message: {}", s);
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                log::error!("Panic message: {}", s);
            }
            Err("Application panicked".into())
        }
    }
}
