//! Dynamic Teapot demo application with multiple random teapots and lights
//! 
//! This demonstrates the engine's rendering capabilities by displaying
//! multiple teapots with random properties and dynamic lighting effects.

#![allow(dead_code)] // Allow unused fields in structs for demo purposes

use rust_engine::assets::obj_loader::ObjLoader;
use rust_engine::ecs::{
    World, Entity, LightFactory, LightingSystem as EcsLightingSystem, 
    TransformComponent, LightComponent,
    SceneManager,
};
use rust_engine::render::{
    Camera,
    Mesh,
    Material,
    StandardMaterialParams,
    lighting::MultiLightEnvironment,
    GraphicsEngine,
    VulkanRendererConfig,
    WindowHandle,
    material::{MaterialId, MaterialType},
    GameObject,
    shared_resources::SharedRenderingResources,
    dynamic::{DynamicObjectHandle, MaterialProperties, MeshType},
};
use rust_engine::foundation::math::Vec3;
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;
use rand::prelude::*;

// Configuration constants
const MAX_TEAPOTS: usize = 10;  // Maximum number of teapots before oldest are despawned
const MAX_LIGHTS: usize = 8;    // Maximum number of lights before oldest are despawned

#[derive(Clone)]
struct TeapotInstance {
    entity: Option<Entity>,
    dynamic_handle: Option<DynamicObjectHandle>, // Dynamic object handle for pooled rendering
    position: Vec3,
    rotation: Vec3,      // Rotation angles in radians
    scale: f32,
    spin_speed: Vec3,    // Angular velocity for each axis
    orbit_radius: f32,   // Radius for orbital motion
    orbit_speed: f32,    // Speed of orbital motion
    orbit_center: Vec3,  // Center point for orbit
    material_index: usize,
    spawn_time: Instant, // When this instance was created
    lifetime: f32,       // How long it should live (in seconds)
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
    spawn_time: Instant, // When this instance was created
    lifetime: f32,       // How long it should live (in seconds)
    sphere_handle: Option<DynamicObjectHandle>, // Visual sphere marker for the light
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
    graphics_engine: GraphicsEngine,
    camera: Camera,
    
    // New SceneManager for batch rendering
    scene_manager: SceneManager,
    
    // Traditional lighting system (keep for compatibility)
    world: World,
    lighting_system: EcsLightingSystem,
    
    // Dynamic content tracking
    teapot_instances: Vec<TeapotInstance>,
    light_instances: Vec<LightInstance>,
    teapot_mesh: Option<Mesh>,
    sphere_mesh: Option<Mesh>,
    material_ids: Vec<MaterialId>,
    
    // GameObject rendering system - unified multi-mesh architecture
    teapot_game_objects: Vec<GameObject>,
    shared_resources: Option<SharedRenderingResources>, // Teapot mesh resources
    sphere_shared_resources: Option<SharedRenderingResources>, // Sphere mesh resources
    
    // Animation state
    start_time: Instant,
    last_material_switch: Instant,
    camera_orbit_speed: f32,
    camera_orbit_radius: f32,
    camera_height_oscillation: f32,
    last_shown_teapot_index: usize, // Track which teapot was shown last frame
    
    // Dynamic spawning state
    last_teapot_spawn: Instant,
    last_light_spawn: Instant,
    max_teapots: usize,
    max_lights: usize,
    
    // Sunlight entity
    sunlight_entity: Option<Entity>,
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
        let mut graphics_engine = GraphicsEngine::new_from_window(&mut window, &renderer_config)
            .expect("Failed to create graphics engine");
        log::info!("Graphics engine created successfully");
        
        // Initialize dynamic object system with pool for 50 objects
        log::info!("Initializing dynamic object system...");
        graphics_engine.initialize_dynamic_system(50)
            .expect("Failed to initialize dynamic object system");
        log::info!("Dynamic object system initialized successfully");
        
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
        let scene_manager = SceneManager::new();
        
        // Initialize traditional ECS for lighting
        let mut world = World::new();
        let lighting_system = EcsLightingSystem::new();
        
        // Store material IDs for teapot creation
        let material_ids: Vec<MaterialId> = (0..teapot_materials.len())
            .map(|i| MaterialId(i as u32))
            .collect();
        
        // Initialize empty dynamic content
        let teapot_instances = Vec::new();
        
        // Create sunlight - one gentle directional light at an angle
        let sunlight_entity = Self::create_sunlight(&mut world);
        
        // Generate random light instances (excluding sunlight)
        let _rng = thread_rng(); // Reserved for future dynamic light generation
        let light_instances = Vec::new(); // Start with no dynamic lights
        
        log::info!("Created sunlight and prepared for dynamic spawning");
        
        let now = Instant::now();
        
        Self {
            window,
            graphics_engine,
            camera,
            scene_manager,
            world,
            lighting_system,
            teapot_instances,
            light_instances,
            teapot_mesh: None,
            sphere_mesh: None,
            material_ids,
            teapot_game_objects: Vec::new(),
            shared_resources: None,
            sphere_shared_resources: None,

            last_material_switch: now,
            start_time: now,
            camera_orbit_speed: 0.15, // Slow camera orbit
            camera_orbit_radius: 20.0,
            camera_height_oscillation: 3.0,
            last_shown_teapot_index: usize::MAX, // Initialize to invalid index
            last_teapot_spawn: now,
            last_light_spawn: now,
            max_teapots: 10,
            max_lights: 10,
            sunlight_entity: Some(sunlight_entity),
        }
    }
    
    fn create_sunlight(world: &mut World) -> Entity {
        // Create a gentle directional sunlight at an angle
        let sunlight_entity = world.create_entity();
        
        let sunlight = LightFactory::directional(
            Vec3::new(-1.0, -1.5, -1.0).normalize(), // Direction (angled from top-left-front)
            Vec3::new(1.0, 0.95, 0.8), // Warm sunlight color (slightly yellowish)
            0.05 // Very gentle intensity to let dynamic lights be more prominent
        );
        
        world.add_component(sunlight_entity, sunlight);
        log::info!("Created gentle sunlight at angle");
        sunlight_entity
    }
    
    fn spawn_random_teapot(&mut self) {
        let mut rng = thread_rng();
        
        // Random position
        let position = Vec3::new(
            rng.gen_range(-12.0..12.0),
            rng.gen_range(-1.0..5.0),
            rng.gen_range(-12.0..12.0),
        );
        
        // Random lifetime between 5-15 seconds
        let lifetime = rng.gen_range(5.0..15.0);
        
        // Select random material and convert to MaterialProperties
        let materials = Self::create_teapot_materials();
        let material_index = rng.gen_range(0..materials.len());
        let selected_material = &materials[material_index];
        
        // Convert Material to MaterialProperties for dynamic system
        let material_props = match &selected_material.material_type {
            MaterialType::StandardPBR(params) => {
                MaterialProperties {
                    base_color: [params.base_color.x, params.base_color.y, params.base_color.z, params.alpha],
                    metallic: params.metallic,
                    roughness: params.roughness,
                    emission: [0.0, 0.0, 0.0], // No emission for now
                    alpha: params.alpha,
                }
            }
            MaterialType::Unlit(params) => {
                MaterialProperties {
                    base_color: [params.color.x, params.color.y, params.color.z, params.alpha],
                    metallic: 0.0,
                    roughness: 1.0,
                    emission: [0.0, 0.0, 0.0],
                    alpha: params.alpha,
                }
            }
            MaterialType::Transparent { base_material, .. } => {
                // Handle transparent materials by using their base material
                match base_material.as_ref() {
                    MaterialType::StandardPBR(params) => {
                        MaterialProperties {
                            base_color: [params.base_color.x, params.base_color.y, params.base_color.z, params.alpha],
                            metallic: params.metallic,
                            roughness: params.roughness,
                            emission: [0.0, 0.0, 0.0],
                            alpha: params.alpha,
                        }
                    }
                    MaterialType::Unlit(params) => {
                        MaterialProperties {
                            base_color: [params.color.x, params.color.y, params.color.z, params.alpha],
                            metallic: 0.0,
                            roughness: 1.0,
                            emission: [0.0, 0.0, 0.0],
                            alpha: params.alpha,
                        }
                    }
                    MaterialType::Transparent { .. } => {
                        // Nested transparent materials, use default
                        MaterialProperties::default()
                    }
                }
            }
        };
        
        // Spawn dynamic object with random material properties
        match self.graphics_engine.spawn_dynamic_object(
            MeshType::Teapot,
            position,
            Vec3::new(0.0, 0.0, 0.0), // No initial rotation
            Vec3::new(1.0, 1.0, 1.0), // Unit scale
            material_props,
        ) {
            Ok(dynamic_handle) => {
                // Create instance tracker
                let instance = TeapotInstance {
                    entity: None, // No longer using SceneManager
                    dynamic_handle: Some(dynamic_handle),
                    position,
                    rotation: Vec3::new(0.0, 0.0, 0.0),
                    scale: 1.0,
                    spin_speed: Vec3::new(
                        rng.gen_range(-2.0..2.0),
                        rng.gen_range(-2.0..2.0),
                        rng.gen_range(-2.0..2.0),
                    ),
                    orbit_radius: rng.gen_range(2.0..8.0),
                    orbit_speed: rng.gen_range(0.5..2.0),
                    orbit_center: position,
                    material_index,
                    spawn_time: Instant::now(),
                    lifetime,
                };
                
                self.teapot_instances.push(instance);
                
                let material_name = selected_material.name.as_deref().unwrap_or("Unknown");
                
                log::info!("Spawned teapot #{} at {:?} with {} material (lifetime: {:.1}s)", 
                          self.teapot_instances.len(), position, material_name, lifetime);
            }
            Err(e) => {
                log::error!("Failed to spawn dynamic teapot: {}", e);
            }
        }
    }
    
    fn spawn_random_light(&mut self) {
        if self.light_instances.len() >= self.max_lights {
            return; // Max limit reached
        }
        
        let mut rng = thread_rng();
        
        // Random position
        let position = Vec3::new(
            rng.gen_range(-15.0..15.0),
            rng.gen_range(2.0..8.0),
            rng.gen_range(-15.0..15.0),
        );
        
        // Random color (hue-based for vibrant colors)
        let hue = rng.gen_range(0.0..360.0);
        let color = Self::hue_to_rgb(hue);
        
        // Random lifetime between 5-15 seconds
        let lifetime = rng.gen_range(5.0..15.0);
        
        // Create entity and light component
        let entity = self.world.create_entity();
        let intensity = rng.gen_range(1.2..2.5); // More vibrant intensity
        let light = LightFactory::point(
            position,
            color,
            intensity,
            rng.gen_range(12.0..26.0) // Longer range for broader coverage
        );
        self.world.add_component(entity, light);
        
        // Random movement pattern
        let movement_pattern = match rng.gen_range(0..4) {
            0 => MovementPattern::Static,
            1 => MovementPattern::Circular {
                radius: rng.gen_range(3.0..8.0),
                speed: rng.gen_range(0.5..1.5),
                axis: Vec3::new(0.0, 1.0, 0.0),
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
                    rng.gen_range(0.2..1.0),
                    rng.gen_range(-1.0..1.0),
                ).normalize(),
            },
        };
        
        // Random effect pattern
        let effect_pattern = match rng.gen_range(0..4) {
            0 => EffectPattern::Steady,
            1 => EffectPattern::Pulse {
                frequency: rng.gen_range(0.5..2.0),
                amplitude: rng.gen_range(0.2..0.4), // Much more subtle pulsing
            },
            2 => EffectPattern::Flicker {
                base_rate: rng.gen_range(8.0..12.0),
                chaos: rng.gen_range(0.1..0.3), // Much more subtle flickering
            },
            _ => EffectPattern::Breathe {
                frequency: rng.gen_range(0.3..1.2),
            },
        };
        
        // Spawn a colored sphere marker at the light position using sphere mesh
        let sphere_handle = self.spawn_sphere_marker(position, color);
        
        let instance = LightInstance {
            entity,
            light_type: LightType::Point,
            base_position: position,
            current_position: position,
            color,
            base_intensity: intensity,
            movement_pattern,
            effect_pattern,
            phase_offset: rng.gen_range(0.0..std::f32::consts::TAU),
            spawn_time: Instant::now(),
            lifetime,
            sphere_handle,
        };
        
        self.light_instances.push(instance);
        log::info!("Spawned light #{} at {:?} (lifetime: {:.1}s, color: {:?})", 
                  self.light_instances.len(), position, lifetime, color);
    }
    
    /// Spawn a sphere marker for light visualization
    /// 
    /// This demonstrates the proper architectural approach for multi-mesh rendering:
    /// - Separate mesh-specific spawning methods
    /// - Clear separation of concerns
    /// - Reusable systems with different resource sets
    fn spawn_sphere_marker(&mut self, position: Vec3, color: Vec3) -> Option<DynamicObjectHandle> {
        // Create MaterialProperties with the light's color
        let sphere_material_props = MaterialProperties {
            base_color: [color.x, color.y, color.z, 1.0],
            metallic: 0.0,
            roughness: 0.1,
            emission: [color.x * 2.0, color.y * 2.0, color.z * 2.0], // Bright emission
            alpha: 1.0,
        };
        
        // Spawn sphere using multi-mesh system with appropriate scale
        match self.graphics_engine.spawn_dynamic_object(
            MeshType::Sphere,
            position,
            Vec3::new(0.0, 0.0, 0.0), // No rotation
            Vec3::new(0.5, 0.5, 0.5), // Medium sphere scale for visible light markers
            sphere_material_props,
        ) {
            Ok(handle) => {
                Some(handle)
            }
            Err(e) => {
                log::warn!("Failed to spawn sphere light marker: {}", e);
                None
            }
        }
    }
    
    fn despawn_excess_objects(&mut self) {
        let initial_teapot_count = self.teapot_instances.len();
        let initial_light_count = self.light_instances.len();
        
        // Despawn oldest teapots if we exceed the maximum
        if self.teapot_instances.len() > MAX_TEAPOTS {
            let excess_count = self.teapot_instances.len() - MAX_TEAPOTS;
            
            // Sort by spawn time to get oldest first
            self.teapot_instances.sort_by_key(|instance| instance.spawn_time);
            
            // Remove the oldest excess teapots
            for _ in 0..excess_count {
                let instance = self.teapot_instances.remove(0);  // Remove first (oldest)
                
                let elapsed = instance.spawn_time.elapsed().as_secs_f32();
                
                // Handle both old entity system and new dynamic handle system
                if let Some(entity) = instance.entity {
                    self.scene_manager.remove_entity(entity);
                    log::info!("Despawned old teapot entity (lived {:.1}s)", elapsed);
                }
                if let Some(handle) = instance.dynamic_handle {
                    if let Err(e) = self.graphics_engine.despawn_dynamic_object(MeshType::Teapot, handle) {
                        log::warn!("Failed to despawn old teapot: {}", e);
                    } else {
                        log::info!("Despawned old teapot (lived {:.1}s, over limit)", elapsed);
                    }
                }
            }
        }
        
        // Despawn oldest lights if we exceed the maximum  
        if self.light_instances.len() > MAX_LIGHTS {
            let excess_count = self.light_instances.len() - MAX_LIGHTS;
            
            // Sort by spawn time to get oldest first
            self.light_instances.sort_by_key(|instance| instance.spawn_time);
            
            // Remove the oldest excess lights
            for _ in 0..excess_count {
                let instance = self.light_instances.remove(0);  // Remove first (oldest)
                
                let elapsed = instance.spawn_time.elapsed().as_secs_f32();
                
                // Remove light component from ECS world
                self.world.remove_component::<LightComponent>(instance.entity);
                
                // Also despawn the sphere marker if it exists
                if let Some(sphere_handle) = instance.sphere_handle {
                    if let Err(e) = self.graphics_engine.despawn_dynamic_object(MeshType::Sphere, sphere_handle) {
                        log::warn!("Failed to despawn light sphere marker: {}", e);
                    } else {
                        log::trace!("Despawned sphere marker for old light");
                    }
                }
                
                log::info!("Despawned old light (lived {:.1}s, over limit)", elapsed);
            }
        }
        
        let teapots_despawned = initial_teapot_count - self.teapot_instances.len();
        let lights_despawned = initial_light_count - self.light_instances.len();
        
        if teapots_despawned > 0 || lights_despawned > 0 {
            log::info!("Despawned {} teapots and {} lights. Active: {} teapots, {} lights",
                      teapots_despawned, lights_despawned,
                      self.teapot_instances.len(), self.light_instances.len());
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
    
    fn generate_random_lights(world: &mut World, rng: &mut ThreadRng) -> Vec<LightInstance> {
        let num_lights = rng.gen_range(4..=8); // 4-8 lights
        let lights = Vec::with_capacity(num_lights);
        
        // Always include one main directional light
        let main_light_entity = world.create_entity();
        world.add_component(main_light_entity, TransformComponent::identity());
        world.add_component(main_light_entity, LightFactory::directional(
            Vec3::new(-0.5, -1.0, 0.3), // Main directional light
            Vec3::new(1.0, 0.95, 0.9),  // Warm white
            0.6
        ));
        
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
                rng.gen_range(0.2..0.6), // Much gentler intensity (was 0.8..2.0)
                rng.gen_range(8.0..12.0) // More reasonable range (was 8.0..20.0)
            ));
            
            // Random movement pattern (for future use when re-enabling creation)
            let _movement = match rng.gen_range(0..4) {
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
            
            // Random effect pattern (for future use when re-enabling creation)
            let _effect = match rng.gen_range(0..4) {
                0 => EffectPattern::Steady,
                1 => EffectPattern::Pulse {
                    frequency: rng.gen_range(0.5..3.0),
                    amplitude: rng.gen_range(0.2..0.4), // Much more subtle pulsing (was 0.3..0.8)
                },
                2 => EffectPattern::Flicker {
                    base_rate: rng.gen_range(5.0..15.0),
                    chaos: rng.gen_range(0.2..0.6),
                },
                _ => EffectPattern::Breathe {
                    frequency: rng.gen_range(0.3..1.2),
                },
            };
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
        
        // Load sphere mesh for light markers
        match ObjLoader::load_obj("resources/models/sphere.obj") {
            Ok(mesh) => {
                // Use original sphere mesh size - scaling will be done through transform scale
                log::info!("Loaded sphere mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.sphere_mesh = Some(mesh);
            }
            Err(e) => {
                log::warn!("Failed to load sphere.obj: {:?}, using fallback small cube", e);
                // Create a small cube as fallback for light markers
                let mut cube_mesh = Mesh::cube();
                for vertex in &mut cube_mesh.vertices {
                    vertex.position[0] *= 0.15; // Make cube even smaller than sphere
                    vertex.position[1] *= 0.15;
                    vertex.position[2] *= 0.15;
                }
                self.sphere_mesh = Some(cube_mesh);
            }
        }
        
        // Initialize GameObject system for multiple individual draw calls
        if let Some(ref teapot_mesh) = self.teapot_mesh {
            // Create materials for teapots
            let teapot_materials = Self::create_teapot_materials();
            
            // Create mesh pool for teapots
            if let Err(e) = self.graphics_engine.create_mesh_pool(
                MeshType::Teapot,
                teapot_mesh,
                &teapot_materials,
                100 // Max 100 teapots
            ) {
                log::error!("Failed to create teapot mesh pool: {}", e);
                return Err(e);
            }
            log::info!("Created teapot mesh pool successfully");
            
            // Initialize the instance renderer system (now does multiple individual draw calls)
            match self.graphics_engine.initialize_instance_renderer(100) {
                Ok(()) => {
                    log::info!("Initialized renderer with capacity for 100 individual draw calls");
                }
                Err(e) => {
                    log::error!("Failed to initialize renderer: {}", e);
                    return Err(e);
                }
            }
        }
        
        // Create sphere mesh pool for light markers
        if let Some(ref sphere_mesh) = self.sphere_mesh {
            // Create simple materials for spheres (light markers)
            let sphere_materials = vec![Material::standard_pbr(StandardMaterialParams {
                base_color: Vec3::new(1.0, 1.0, 0.0), // Yellow for light markers
                alpha: 1.0,
                metallic: 0.0,
                roughness: 0.8,
                emission: Vec3::new(0.2, 0.2, 0.0), // Slight emission for visibility
                ..Default::default()
            }).with_name("Light Marker")];
            
            if let Err(e) = self.graphics_engine.create_mesh_pool(
                MeshType::Sphere,
                sphere_mesh,
                &sphere_materials,
                50 // Max 50 light markers
            ) {
                log::error!("Failed to create sphere mesh pool: {}", e);
                return Err(e);
            }
            log::info!("Created sphere mesh pool successfully");
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
                        // Dynamic spawning is automatic now - space key disabled
                        log::info!("Dynamic spawning is automatic - space key no longer regenerates scene");
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
                self.graphics_engine.recreate_swapchain(&mut self.window);
                let (width, height) = self.graphics_engine.get_swapchain_extent();
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
    // TODO: Remove this old regenerate_scene method - using dynamic spawning now
    /*
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
    */
    
    fn update_scene(&mut self) {
        let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
        let delta_time = 0.016; // Assume ~60 FPS for now
        
        // Dynamic spawning with random intervals
        let mut rng = thread_rng();
        let now = Instant::now();
        
        // Spawn teapots randomly (every 1-3 seconds)
        if now.duration_since(self.last_teapot_spawn).as_secs_f32() > rng.gen_range(1.0..3.0) {
            self.spawn_random_teapot();
            self.last_teapot_spawn = now;
        }
        
        // Spawn lights randomly (every 2-4 seconds)
        if now.duration_since(self.last_light_spawn).as_secs_f32() > rng.gen_range(2.0..4.0) {
            self.spawn_random_light();
            self.last_light_spawn = now;
        }
        
        // Remove excess objects AFTER spawning (keep only newest within limits)
        self.despawn_excess_objects();
        
        // Update camera with orbital motion
        let camera_angle = elapsed_seconds * self.camera_orbit_speed;
        let camera_height = 8.0 + (elapsed_seconds * 0.3).sin() * self.camera_height_oscillation;
        let camera_x = camera_angle.cos() * self.camera_orbit_radius;
        let camera_z = camera_angle.sin() * self.camera_orbit_radius;
        
        self.camera.set_position(Vec3::new(camera_x, camera_height, camera_z));
        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
        
        // Update SceneManager (handles all teapot movement automatically)
        self.scene_manager.update(delta_time, &mut self.graphics_engine);
        
        // Update light movements and effects
        for light_instance in &mut self.light_instances {
            // Update light position based on movement pattern
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
            
            // Update light intensity based on effect pattern
            let current_intensity = match &light_instance.effect_pattern {
                EffectPattern::Steady => light_instance.base_intensity,
                EffectPattern::Pulse { frequency, amplitude } => {
                    let pulse = (elapsed_seconds * frequency + light_instance.phase_offset).sin();
                    light_instance.base_intensity * (1.0 + pulse * amplitude)
                }
                EffectPattern::Flicker { base_rate, chaos } => {
                    let flicker = (elapsed_seconds * base_rate + light_instance.phase_offset).sin() * chaos;
                    light_instance.base_intensity * (1.0 + flicker)
                }
                EffectPattern::Breathe { frequency } => {
                    let breathe = (elapsed_seconds * frequency + light_instance.phase_offset).sin();
                    light_instance.base_intensity * (0.5 + 0.5 * breathe.abs())
                }
            };
            
            // Update the light component in ECS
            if let Some(light_comp) = self.world.get_component_mut::<LightComponent>(light_instance.entity) {
                light_comp.position = light_instance.current_position;
                light_comp.intensity = current_intensity.max(0.0); // Ensure non-negative
            }
            
            // Update sphere marker position and scale to follow the light
            if let Some(sphere_handle) = light_instance.sphere_handle {
                // Get the current light component to read intensity and range
                if let Some(light_comp) = self.world.get_component::<LightComponent>(light_instance.entity) {
                    // Calculate scale based on light intensity and range
                    // Much smaller base scale, then multiply by intensity and a fraction of range
                    let base_scale = 0.5;
                    let intensity_factor = light_comp.intensity.max(0.1); // Minimum visible scale
                    let range_factor = (light_comp.range * 0.01).max(0.1); // 1% of range, minimum 0.1
                    let combined_scale = base_scale * intensity_factor * range_factor;
                    let scale_vec = Vec3::new(combined_scale, combined_scale, combined_scale);
                    
                    // Update both position and scale
                    if let Err(e) = self.graphics_engine.update_dynamic_object_transform(
                        MeshType::Sphere,
                        sphere_handle,
                        light_instance.current_position,
                        Vec3::new(0.0, 0.0, 0.0), // No rotation
                        scale_vec,
                    ) {
                        log::warn!("Failed to update sphere marker transform: {}", e);
                    }
                } else {
                    // Fallback to position-only update if light component not found
                    if let Err(e) = self.graphics_engine.update_dynamic_object_position(
                        MeshType::Sphere,
                        sphere_handle,
                        light_instance.current_position,
                    ) {
                        log::warn!("Failed to update sphere marker position: {}", e);
                    }
                }
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
        let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
        let delta_time = 0.016; // Approximate 60 FPS delta time
        
        // BEGIN DYNAMIC FRAME: Setup and lifecycle updates
        self.graphics_engine.begin_dynamic_frame(delta_time)?;
        
        // Set camera for the frame (this needs to be done before any rendering)
        self.graphics_engine.set_camera(&self.camera);
        
        // Build multi-light environment from entity system
        let multi_light_env = self.build_multi_light_environment_from_entities().clone();
        self.graphics_engine.set_multi_light_environment(&multi_light_env);
        
        // UPDATE DYNAMIC OBJECTS: All objects are spawned directly in spawn methods
        // No additional spawning logic needed here
        
        // RECORD DYNAMIC DRAWS: Record all active dynamic objects for rendering
        log::debug!("Recording dynamic object draw commands...");
        self.graphics_engine.record_dynamic_draws()?;
        log::debug!("Dynamic draw commands recorded successfully");
        
        // END DYNAMIC FRAME: Submit and present
        log::debug!("Ending dynamic frame...");
        self.graphics_engine.end_dynamic_frame(&mut self.window)?;
        log::debug!("Dynamic frame ended successfully");
        
        // Log performance info occasionally
        static mut LAST_LOG_TIME: f32 = 0.0;
        unsafe {
            if elapsed_seconds - LAST_LOG_TIME >= 5.0 {
                if let Some(stats) = self.graphics_engine.get_dynamic_stats() {
                    // Calculate utilization based on render batches (fewer batches = better performance)
                    let utilization_estimate = ((stats.render_batches_per_frame as f32 / stats.active_pools as f32) * 100.0).min(100.0);
                    log::info!("Dynamic rendering: {} active objects, {} total spawned, {} render batches/frame", 
                              stats.total_active_objects,
                              stats.total_spawned,
                              stats.render_batches_per_frame);
                    log::info!("Pool efficiency: {} active pools, estimated {}% efficiency",
                              stats.active_pools,
                              utilization_estimate as u32);
                }
                log::info!("Total teapot instances: {}, {} active lights (sunlight + {} dynamic)", 
                          self.teapot_instances.len(),
                          self.light_instances.len() + 1, // +1 for sunlight
                          self.light_instances.len());
                LAST_LOG_TIME = elapsed_seconds;
            }
        }
        
        Ok(())
    }

    /// Clean up expired teapot instances that were automatically despawned by the dynamic system
    fn cleanup_expired_instances(&mut self) {
        let now = Instant::now();
        
        // Remove teapot instances that have exceeded their lifetime
        self.teapot_instances.retain(|instance| {
            let elapsed = now.duration_since(instance.spawn_time).as_secs_f32();
            let should_keep = elapsed < instance.lifetime;
            
            if !should_keep {
                log::debug!("Removing expired teapot instance (lived {:.1}s of {:.1}s)", 
                           elapsed, instance.lifetime);
            }
            
            should_keep
        });
        
        // Also clean up light instances
        self.light_instances.retain(|instance| {
            let elapsed = now.duration_since(instance.spawn_time).as_secs_f32();
            let should_keep = elapsed < instance.lifetime;
            
            if !should_keep {
                log::debug!("Removing expired light instance (lived {:.1}s of {:.1}s)", 
                           elapsed, instance.lifetime);
            }
            
            should_keep
        });
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
