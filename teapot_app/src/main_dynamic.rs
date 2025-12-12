//! Dynamic Teapot demo application with multiple random teapots and lights
//! 
//! This demonstrates the engine's rendering capabilities by displaying
//! multiple teapots with random properties and dynamic lighting effects.

#![allow(dead_code)] // Allow unused fields in structs for demo purposes

use rust_engine::assets::{ObjLoader, MaterialBuilder};
use rust_engine::audio::{AudioSystem, SoundHandle};
use rust_engine::ecs::{
    World, Entity, LightFactory, LightingSystem as EcsLightingSystem, 
    TransformComponent, LightComponent, LifecycleComponent,
    components::RenderableComponent,
};
use rust_engine::scene::SceneManager;
use rust_engine::render::{
    Camera,
    Mesh,
    Material,
    systems::lighting::MultiLightEnvironment,
    GraphicsEngine,
    VulkanRendererConfig,
    WindowHandle,
    resources::materials::{MaterialId, MaterialType},
    systems::dynamic::{DynamicObjectHandle, MeshType},
    FontAtlas,
    TextLayout,
    systems::text::{create_lit_text_material, create_unlit_text_material, TextRenderer},
};
use rust_engine::foundation::math::{Vec3, Quat};
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;
use rand::prelude::*;

// Configuration constants
const MAX_TEAPOTS: usize = 5;   // Maximum number of monkeys before oldest are despawned
const MAX_SPACESHIPS: usize = 10; // Maximum number of spaceships
const MAX_LIGHTS: usize = 12;    // Maximum number of lights before oldest are despawned

#[derive(Clone)]
struct TeapotInstance {
    entity: Option<Entity>,
    material: Material,  // Store the actual material with all properties
    spawn_time: Instant, // When this instance was created
    lifetime: f32,       // How long it should live (in seconds)
}

#[derive(Clone)]
struct SpaceshipInstance {
    entity: Option<Entity>,
    material: Material,  // Store the actual material with all properties
    spawn_time: Instant,
    lifetime: f32,
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
    light_instances: Vec<LightInstance>,
    teapot_mesh: Option<Mesh>,
    sphere_mesh: Option<Mesh>,
    spaceship_mesh: Option<Mesh>,
    spaceship_base_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    spaceship_emission_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    frigate_mesh: Option<Mesh>,
    frigate_base_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    frigate_emission_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    monkey_base_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    monkey_normal_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    monkey_emission_texture: Option<rust_engine::render::resources::materials::TextureHandle>,
    quad_mesh: Option<Mesh>, // Simple quad for testing
    material_ids: Vec<MaterialId>,
    
    // Animation state
    start_time: Instant,
    last_material_switch: Instant,
    camera_orbit_speed: f32,
    camera_orbit_radius: f32,
    camera_height_oscillation: f32,
    last_shown_teapot_index: usize, // Track which teapot was shown last frame
    
    // Dynamic spawning state
    last_teapot_spawn: Instant,
    last_spaceship_spawn: Instant,
    last_light_spawn: Instant,
    max_teapots: usize,
    max_spaceships: usize,
    max_lights: usize,
    
    // Sunlight entity
    sunlight_entity: Option<Entity>,
    
    // Text rendering test
    font_atlas: Option<FontAtlas>,
    text_test_handle: Option<DynamicObjectHandle>,
    unlit_text_handle: Option<DynamicObjectHandle>,
    text_renderer: Option<TextRenderer>,
    text_material: Option<Material>,
    
    // FPS tracking for Step 3
    fps_update_timer: Instant,
    frame_count: u32,
    current_fps: f32,
    
    // UI System (clean architecture)
    ui_manager: rust_engine::ui::UIManager,
    fps_label_id: rust_engine::ui::UINodeId,
    test_button_id: rust_engine::ui::UINodeId,
    high_beep_button_id: rust_engine::ui::UINodeId,
    low_beep_button_id: rust_engine::ui::UINodeId,
    spam_test_button_id: rust_engine::ui::UINodeId,
    
    // Volume control UI
    master_vol_label_id: rust_engine::ui::UINodeId,
    sfx_vol_label_id: rust_engine::ui::UINodeId,
    music_vol_label_id: rust_engine::ui::UINodeId,
    master_vol_up_id: rust_engine::ui::UINodeId,
    master_vol_down_id: rust_engine::ui::UINodeId,
    sfx_vol_up_id: rust_engine::ui::UINodeId,
    sfx_vol_down_id: rust_engine::ui::UINodeId,
    music_vol_up_id: rust_engine::ui::UINodeId,
    music_vol_down_id: rust_engine::ui::UINodeId,
    
    // Music control UI
    music_status_label_id: rust_engine::ui::UINodeId,
    play_music_button_id: rust_engine::ui::UINodeId,
    pause_music_button_id: rust_engine::ui::UINodeId,
    stop_music_button_id: rust_engine::ui::UINodeId,
    
    // Music state
    music_is_playing: bool,
    music_loop_enabled: bool,
    
    // Debug tracking for UI events
    last_debug_log: Instant,
    last_mouse_pos: (f64, f64),
    last_frame_time: Instant,
    
    // Audio system
    audio: Option<AudioSystem>,
    last_beep_sound: Option<SoundHandle>,
    button_clicked_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    high_beep_clicked_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    low_beep_clicked_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    spam_test_clicked_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    
    // Volume control flags
    master_vol_up_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    master_vol_down_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    sfx_vol_up_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    sfx_vol_down_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    music_vol_up_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    music_vol_down_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    
    // Music control flags
    play_music_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    pause_music_flag: std::rc::Rc<std::cell::RefCell<bool>>,
    stop_music_flag: std::rc::Rc<std::cell::RefCell<bool>>,
}

impl DynamicTeapotApp {
    pub fn new() -> Self {
        log::info!("Creating dynamic teapot demo application...");
        
        // Create window
        log::info!("Creating window...");
        let mut window = WindowHandle::new(1200, 800, "Rusteroids - Dynamic Monkey Demo");
        log::info!("Window created successfully");
        
        // Create renderer
        log::info!("Creating Vulkan renderer...");
        let renderer_config = VulkanRendererConfig::new("Rusteroids - Dynamic Monkey Demo")
            .with_version(1, 0, 0)
            .with_shader_paths(
                "target/shaders/standard_pbr_vert.spv".to_string(),
                "target/shaders/standard_pbr_frag.spv".to_string()
            );
        let mut graphics_engine = GraphicsEngine::new_from_window(&mut window, &renderer_config)
            .expect("Failed to create graphics engine");
        log::info!("Graphics engine created successfully");
        
        // Initialize UI text rendering system
        graphics_engine.initialize_ui_text_system()
            .expect("Failed to initialize UI text system");
        log::info!("UI text rendering system initialized");
        
        // Event handlers will be registered after UIManager is created
        log::info!("UI text rendering system initialized");
        
        // Initialize UI screen size for input collision detection
        graphics_engine.update_ui_screen_size(1200.0, 800.0);
        
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
        
        // Create sunlight - one gentle directional light at an angle
        let sunlight_entity = Self::create_sunlight(&mut world);
        
        // Generate random light instances (excluding sunlight)
        let _rng = thread_rng(); // Reserved for future dynamic light generation
        let light_instances = Vec::new(); // Start with no dynamic lights
        
        log::info!("Created sunlight and prepared for dynamic spawning");
        
        // Initialize UI Manager (clean architecture)
        log::info!("Initializing UI Manager...");
        let mut ui_manager = rust_engine::ui::UIManager::new();
        ui_manager.set_screen_size(1200.0, 800.0);
        
        // Create font atlas for UI (font is in workspace root)
        let font_path = "resources/fonts/default.ttf";
        let font_data = std::fs::read(font_path)
            .expect(&format!("Failed to read font file: {}", font_path));
        let mut font_atlas = FontAtlas::new(&font_data, 24.0)
            .expect("Failed to create font atlas");
        font_atlas.rasterize_glyphs()
            .expect("Failed to rasterize glyphs");
        ui_manager.initialize_font_atlas(font_atlas);
        
        // Create UI elements
        use rust_engine::ui::{UIPanel, UIText, UIButton, Anchor};
        use rust_engine::foundation::math::Vec4;
        
        // Blue panel at bottom-left
        let panel = UIPanel {
            element: rust_engine::ui::UIElement {
                position: (10.0, -60.0),
                size: (50.0, 50.0),
                anchor: Anchor::BottomLeft,
                visible: true,
                z_order: 0,
            },
            color: Vec4::new(0.0, 0.0, 1.0, 0.7),
            ..Default::default()
        };
        ui_manager.add_panel(panel);
        
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
        
        // "Click Me!" button (440Hz beep)
        let button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (100.0, 100.0),
                size: (200.0, 40.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "Normal Beep (440Hz)".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.3, 0.3, 0.3, 0.9),
            hover_color: Vec4::new(0.4, 0.5, 0.7, 0.9),
            pressed_color: Vec4::new(0.2, 0.3, 0.5, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(1),
        };
        let test_button_id = ui_manager.add_button(button);
        
        // "High Beep" button (880Hz)
        let high_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (100.0, 150.0),
                size: (200.0, 40.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "High Beep (880Hz)".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.3, 0.5, 0.3, 0.9),
            hover_color: Vec4::new(0.4, 0.7, 0.4, 0.9),
            pressed_color: Vec4::new(0.2, 0.4, 0.2, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(2),
        };
        let high_beep_button_id = ui_manager.add_button(high_button);
        
        // "Low Beep" button (220Hz)
        let low_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (100.0, 200.0),
                size: (200.0, 40.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "Low Beep (220Hz)".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.5, 0.3, 0.3, 0.9),
            hover_color: Vec4::new(0.7, 0.4, 0.4, 0.9),
            pressed_color: Vec4::new(0.4, 0.2, 0.2, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(3),
        };
        let low_beep_button_id = ui_manager.add_button(low_button);
        
        // "Spam Test" button (tests voice limiting)
        let spam_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (100.0, 250.0),
                size: (200.0, 40.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "Spam Test (5x)".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.5, 0.3, 0.5, 0.9),
            hover_color: Vec4::new(0.7, 0.4, 0.7, 0.9),
            pressed_color: Vec4::new(0.4, 0.2, 0.4, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(4),
        };
        let spam_test_button_id = ui_manager.add_button(spam_button);
        
        // === VOLUME CONTROLS ===
        
        // Master Volume label and controls
        let master_vol_text = UIText {
            element: rust_engine::ui::UIElement {
                position: (350.0, 100.0),
                size: (0.0, 0.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "Master: 100%".to_string(),
            font_size: 18.0,
            color: Vec4::new(1.0, 1.0, 0.5, 1.0),
            h_align: rust_engine::ui::HorizontalAlign::Left,
            v_align: rust_engine::ui::VerticalAlign::Top,
        };
        let master_vol_label_id = ui_manager.add_text(master_vol_text);
        
        let master_vol_up_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (500.0, 95.0),
                size: (40.0, 30.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "+".to_string(),
            font_size: 20.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.2, 0.6, 0.2, 0.9),
            hover_color: Vec4::new(0.3, 0.8, 0.3, 0.9),
            pressed_color: Vec4::new(0.1, 0.4, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(5),
        };
        let master_vol_up_id = ui_manager.add_button(master_vol_up_button);
        
        let master_vol_down_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (550.0, 95.0),
                size: (40.0, 30.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "-".to_string(),
            font_size: 20.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.6, 0.2, 0.2, 0.9),
            hover_color: Vec4::new(0.8, 0.3, 0.3, 0.9),
            pressed_color: Vec4::new(0.4, 0.1, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(6),
        };
        let master_vol_down_id = ui_manager.add_button(master_vol_down_button);
        
        // SFX Volume label and controls
        let sfx_vol_text = UIText {
            element: rust_engine::ui::UIElement {
                position: (350.0, 140.0),
                size: (0.0, 0.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "SFX: 100%".to_string(),
            font_size: 18.0,
            color: Vec4::new(0.5, 1.0, 0.5, 1.0),
            h_align: rust_engine::ui::HorizontalAlign::Left,
            v_align: rust_engine::ui::VerticalAlign::Top,
        };
        let sfx_vol_label_id = ui_manager.add_text(sfx_vol_text);
        
        let sfx_vol_up_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (500.0, 135.0),
                size: (40.0, 30.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "+".to_string(),
            font_size: 20.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.2, 0.6, 0.2, 0.9),
            hover_color: Vec4::new(0.3, 0.8, 0.3, 0.9),
            pressed_color: Vec4::new(0.1, 0.4, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(7),
        };
        let sfx_vol_up_id = ui_manager.add_button(sfx_vol_up_button);
        
        let sfx_vol_down_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (550.0, 135.0),
                size: (40.0, 30.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "-".to_string(),
            font_size: 20.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.6, 0.2, 0.2, 0.9),
            hover_color: Vec4::new(0.8, 0.3, 0.3, 0.9),
            pressed_color: Vec4::new(0.4, 0.1, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(8),
        };
        let sfx_vol_down_id = ui_manager.add_button(sfx_vol_down_button);
        
        // Music Volume label and controls
        let music_vol_text = UIText {
            element: rust_engine::ui::UIElement {
                position: (350.0, 180.0),
                size: (0.0, 0.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "Music: 100%".to_string(),
            font_size: 18.0,
            color: Vec4::new(0.5, 0.5, 1.0, 1.0),
            h_align: rust_engine::ui::HorizontalAlign::Left,
            v_align: rust_engine::ui::VerticalAlign::Top,
        };
        let music_vol_label_id = ui_manager.add_text(music_vol_text);
        
        let music_vol_up_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (500.0, 175.0),
                size: (40.0, 30.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "+".to_string(),
            font_size: 20.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.2, 0.6, 0.2, 0.9),
            hover_color: Vec4::new(0.3, 0.8, 0.3, 0.9),
            pressed_color: Vec4::new(0.1, 0.4, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(9),
        };
        let music_vol_up_id = ui_manager.add_button(music_vol_up_button);
        
        let music_vol_down_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (550.0, 175.0),
                size: (40.0, 30.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "-".to_string(),
            font_size: 20.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.6, 0.2, 0.2, 0.9),
            hover_color: Vec4::new(0.8, 0.3, 0.3, 0.9),
            pressed_color: Vec4::new(0.4, 0.1, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(10),
        };
        let music_vol_down_id = ui_manager.add_button(music_vol_down_button);
        
        // === MUSIC CONTROLS ===
        
        // Music status label
        let music_status_text = UIText {
            element: rust_engine::ui::UIElement {
                position: (350.0, 230.0),
                size: (0.0, 0.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "Music: Stopped".to_string(),
            font_size: 18.0,
            color: Vec4::new(1.0, 0.8, 0.3, 1.0),
            h_align: rust_engine::ui::HorizontalAlign::Left,
            v_align: rust_engine::ui::VerticalAlign::Top,
        };
        let music_status_label_id = ui_manager.add_text(music_status_text);
        
        // Play/Stop music button (toggles)
        let play_music_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (350.0, 260.0),
                size: (90.0, 35.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "▶ Start".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.2, 0.6, 0.2, 0.9),
            hover_color: Vec4::new(0.3, 0.8, 0.3, 0.9),
            pressed_color: Vec4::new(0.1, 0.4, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(11),
        };
        let play_music_button_id = ui_manager.add_button(play_music_button);
        
        // Pause music button
        let pause_music_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (450.0, 260.0),
                size: (80.0, 35.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "⏸ Pause".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.5, 0.5, 0.2, 0.9),
            hover_color: Vec4::new(0.7, 0.7, 0.3, 0.9),
            pressed_color: Vec4::new(0.3, 0.3, 0.1, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(12),
        };
        let pause_music_button_id = ui_manager.add_button(pause_music_button);
        
        // Loop toggle button
        let stop_music_button = UIButton {
            element: rust_engine::ui::UIElement {
                position: (540.0, 260.0),
                size: (90.0, 35.0),
                anchor: Anchor::TopLeft,
                visible: true,
                z_order: 1,
            },
            text: "🔁 Loop: Off".to_string(),
            font_size: 16.0,
            state: rust_engine::ui::ButtonState::Normal,
            normal_color: Vec4::new(0.4, 0.4, 0.5, 0.9),
            hover_color: Vec4::new(0.5, 0.5, 0.6, 0.9),
            pressed_color: Vec4::new(0.3, 0.3, 0.4, 0.9),
            disabled_color: Vec4::new(0.2, 0.2, 0.2, 0.5),
            text_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            border_color: Vec4::new(0.8, 0.8, 0.8, 1.0),
            border_width: 2.0,
            enabled: true,
            on_click_id: Some(13),
        };
        let stop_music_button_id = ui_manager.add_button(stop_music_button);
        
        // Register button click event handler WITH UIManager's EventSystem
        // (NOT GraphicsEngine - UIManager has its own EventSystem!)
        log::info!("Registering UI event handlers...");
        use rust_engine::events::{Event, EventHandler, EventType};
        
        struct ButtonClickHandler {
            button_id: u32,
            clicked_flag: std::rc::Rc<std::cell::RefCell<bool>>,
        }
        impl EventHandler for ButtonClickHandler {
            fn on_event(&mut self, event: &Event) -> bool {
                if let Some(event_button_id) = event.get_button_id() {
                    if event_button_id == self.button_id {
                        log::info!("🎉 Button {} was clicked!", event_button_id);
                        *self.clicked_flag.borrow_mut() = true;
                        return true; // Consume the event
                    }
                }
                false
            }
        }
        
        // Create shared flags for button clicks
        let button_clicked_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let high_beep_clicked_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let low_beep_clicked_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let spam_test_clicked_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        
        // Volume control flags
        let master_vol_up_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let master_vol_down_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let sfx_vol_up_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let sfx_vol_down_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let music_vol_up_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let music_vol_down_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        
        // Music control flags
        let play_music_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let pause_music_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        let stop_music_flag = std::rc::Rc::new(std::cell::RefCell::new(false));
        
        // Register handlers for each button
        ui_manager.event_system_mut().register_handler(
            EventType::ButtonClicked,
            Box::new(ButtonClickHandler {
                button_id: 1,
                clicked_flag: button_clicked_flag.clone(),
            })
        );
        
        ui_manager.event_system_mut().register_handler(
            EventType::ButtonClicked,
            Box::new(ButtonClickHandler {
                button_id: 2,
                clicked_flag: high_beep_clicked_flag.clone(),
            })
        );
        
        ui_manager.event_system_mut().register_handler(
            EventType::ButtonClicked,
            Box::new(ButtonClickHandler {
                button_id: 3,
                clicked_flag: low_beep_clicked_flag.clone(),
            })
        );
        
        ui_manager.event_system_mut().register_handler(
            EventType::ButtonClicked,
            Box::new(ButtonClickHandler {
                button_id: 4,
                clicked_flag: spam_test_clicked_flag.clone(),
            })
        );
        
        // Register volume control button handlers
        for (button_id, flag) in [
            (5, master_vol_up_flag.clone()),
            (6, master_vol_down_flag.clone()),
            (7, sfx_vol_up_flag.clone()),
            (8, sfx_vol_down_flag.clone()),
            (9, music_vol_up_flag.clone()),
            (10, music_vol_down_flag.clone()),
        ] {
            ui_manager.event_system_mut().register_handler(
                EventType::ButtonClicked,
                Box::new(ButtonClickHandler {
                    button_id,
                    clicked_flag: flag,
                })
            );
        }
        
        // Register music control button handlers
        for (button_id, flag) in [
            (11, play_music_flag.clone()),
            (12, pause_music_flag.clone()),
            (13, stop_music_flag.clone()),
        ] {
            ui_manager.event_system_mut().register_handler(
                EventType::ButtonClicked,
                Box::new(ButtonClickHandler {
                    button_id,
                    clicked_flag: flag,
                })
            );
        }
        
        // Register hover event handler for debugging
        struct HoverHandler;
        impl EventHandler for HoverHandler {
            fn on_event(&mut self, _event: &Event) -> bool {
                // Silently handle hover events without logging
                false // Don't consume hover events
            }
        }
        ui_manager.event_system_mut().register_handler(
            EventType::ButtonHoverChanged,
            Box::new(HoverHandler)
        );
        log::info!("UI event handlers registered with UIManager's EventSystem");
        
        log::info!("UI Manager initialized with panel, FPS label, and button");
        
        let now = Instant::now();
        
        Self {
            window,
            graphics_engine,
            camera,
            scene_manager,
            world,
            lighting_system,
            light_instances,
            teapot_mesh: None,
            sphere_mesh: None,
            spaceship_mesh: None,
            spaceship_base_texture: None,
            spaceship_emission_texture: None,
            frigate_mesh: None,
            frigate_base_texture: None,
            frigate_emission_texture: None,
            monkey_base_texture: None,
            monkey_normal_texture: None,
            monkey_emission_texture: None,
            quad_mesh: None,
            material_ids,

            last_material_switch: now,
            start_time: now,
            camera_orbit_speed: 0.15, // Slow camera orbit
            camera_orbit_radius: 20.0,
            camera_height_oscillation: 3.0,
            last_shown_teapot_index: usize::MAX, // Initialize to invalid index
            last_teapot_spawn: now,
            last_spaceship_spawn: now,
            last_light_spawn: now,
            max_teapots: 5,
            max_spaceships: 10,
            max_lights: 10,
            sunlight_entity: Some(sunlight_entity),
            font_atlas: None,
            text_test_handle: None,
            unlit_text_handle: None,
            text_renderer: None,
            text_material: None,
            
            // FPS tracking
            fps_update_timer: Instant::now(),
            frame_count: 0,
            current_fps: 60.0,
            
            // UI Manager (clean architecture)
            ui_manager,
            fps_label_id,
            test_button_id,
            high_beep_button_id,
            low_beep_button_id,
            spam_test_button_id,
            
            // Volume control UI
            master_vol_label_id,
            sfx_vol_label_id,
            music_vol_label_id,
            master_vol_up_id,
            master_vol_down_id,
            sfx_vol_up_id,
            sfx_vol_down_id,
            music_vol_up_id,
            music_vol_down_id,
            
            // Music control UI
            music_status_label_id,
            play_music_button_id,
            pause_music_button_id,
            stop_music_button_id,
            
            // Music state
            music_is_playing: false,
            music_loop_enabled: false,
            
            // Debug tracking
            last_debug_log: Instant::now(),
            last_mouse_pos: (0.0, 0.0),
            last_frame_time: Instant::now(),
            
            // Audio system
            audio: AudioSystem::new().ok(),
            last_beep_sound: None,
            button_clicked_flag: button_clicked_flag.clone(),
            high_beep_clicked_flag: high_beep_clicked_flag.clone(),
            low_beep_clicked_flag: low_beep_clicked_flag.clone(),
            spam_test_clicked_flag: spam_test_clicked_flag.clone(),
            
            // Volume control flags
            master_vol_up_flag,
            master_vol_down_flag,
            sfx_vol_up_flag,
            sfx_vol_down_flag,
            music_vol_up_flag,
            music_vol_down_flag,
            
            // Music control flags
            play_music_flag,
            pause_music_flag,
            stop_music_flag,
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
        
        // Weighted material selection:
        // 60% Standard PBR (no emission)
        // 25% Standard PBR with emission  
        // 10% Transparent variants
        // 5% Unlit variants
        let roll = rng.gen_range(0.0..1.0);
        
        let selected_material = if roll < 0.60 {
            // Standard PBR without emission (60%)
            let base_colors = [
                Vec3::new(0.8, 0.2, 0.2),  // Red
                Vec3::new(0.2, 0.8, 0.2),  // Green
                Vec3::new(0.2, 0.2, 0.8),  // Blue
                Vec3::new(0.8, 0.8, 0.2),  // Yellow
                Vec3::new(0.8, 0.2, 0.8),  // Magenta
                Vec3::new(0.2, 0.8, 0.8),  // Cyan
                Vec3::new(0.8, 0.5, 0.3),  // Orange-brown
            ];
            let base_color = base_colors[rng.gen_range(0..base_colors.len())];
            
            let mut material = MaterialBuilder::new()
                .base_color(base_color)
                .metallic(rng.gen_range(0.0..0.3))
                .roughness(rng.gen_range(0.6..0.9))
                .build();
            
            // 50% chance to add base texture and normal map
            if rng.gen_bool(0.5) {
                if let Some(base_tex) = self.monkey_base_texture {
                    material = material.with_base_color_texture(base_tex);
                }
                if let Some(normal_tex) = self.monkey_normal_texture {
                    material = material.with_normal_texture(normal_tex);
                }
            }
            
            material
        } else if roll < 0.85 {
            // Standard PBR with emission (25%)
            let emission_colors = [
                Vec3::new(1.0, 0.5, 0.0),  // Orange
                Vec3::new(1.0, 0.0, 0.5),  // Pink
                Vec3::new(0.0, 1.0, 0.5),  // Cyan-green
                Vec3::new(0.5, 0.0, 1.0),  // Purple
                Vec3::new(1.0, 1.0, 0.0),  // Yellow
                Vec3::new(0.0, 0.5, 1.0),  // Blue
            ];
            let emission = emission_colors[rng.gen_range(0..emission_colors.len())];
            let emission_strength = rng.gen_range(0.8..2.5);
            
            let mut material = MaterialBuilder::new()
                .base_color_rgb(0.8, 0.8, 0.8)
                .metallic(0.1)
                .roughness(0.8)
                .emission(emission)
                .emission_strength(emission_strength)
                .build();
            
            // 70% chance to add emission texture on emissive materials
            if rng.gen_bool(0.7) {
                if let Some(emission_tex) = self.monkey_emission_texture {
                    material = material.with_emission_texture(emission_tex);
                }
            }
            
            // 30% chance to also add base texture and normal map
            if rng.gen_bool(0.3) {
                if let Some(base_tex) = self.monkey_base_texture {
                    material = material.with_base_color_texture(base_tex);
                }
                if let Some(normal_tex) = self.monkey_normal_texture {
                    material = material.with_normal_texture(normal_tex);
                }
            }
            
            material
        } else if roll < 0.95 {
            // Transparent variants (10%)
            if rng.gen_bool(0.5) {
                // Transparent PBR (glass-like)
                let glass_colors = [
                    Vec3::new(0.2, 0.4, 1.0),  // Blue glass
                    Vec3::new(0.4, 1.0, 0.8),  // Cyan glass
                    Vec3::new(1.0, 0.2, 0.8),  // Purple glass
                ];
                let color = glass_colors[rng.gen_range(0..glass_colors.len())];
                
                MaterialBuilder::new()
                    .base_color(color)
                    .metallic(0.9)
                    .roughness(0.1)
                    .alpha(0.3)
                    .build_transparent()
            } else {
                // Transparent Unlit (ghost-like)
                let ghost_colors = [
                    Vec3::new(1.0, 1.0, 0.5),  // Yellow ghost
                    Vec3::new(0.5, 1.0, 0.5),  // Green ghost
                    Vec3::new(1.0, 0.5, 0.5),  // Orange ghost
                ];
                let color = ghost_colors[rng.gen_range(0..ghost_colors.len())];
                
                MaterialBuilder::new()
                    .base_color(color)
                    .metallic(0.0)
                    .roughness(1.0)
                    .alpha(0.4)
                    .build_transparent()
            }
        } else {
            // Unlit variants (5%)
            let bright_colors = [
                Vec3::new(1.0, 0.0, 0.0),  // Bright red
                Vec3::new(0.0, 1.0, 0.0),  // Bright green
                Vec3::new(0.0, 0.0, 1.0),  // Bright blue
                Vec3::new(1.0, 1.0, 0.0),  // Bright yellow
            ];
            let color = bright_colors[rng.gen_range(0..bright_colors.len())];
            
            MaterialBuilder::new()
                .base_color(color)
                .metallic(0.0)
                .roughness(1.0)
                .build()
        };
        
        // Get material name for logging
        let material_name = match &selected_material.material_type {
            MaterialType::StandardPBR(params) => {
                if params.emission.x > 0.0 || params.emission.y > 0.0 || params.emission.z > 0.0 {
                    "Emissive PBR Monkey"
                } else {
                    "Standard PBR Monkey"
                }
            }
            MaterialType::Unlit(_) => "Unlit Monkey",
            MaterialType::Transparent { base_material, .. } => {
                match base_material.as_ref() {
                    MaterialType::StandardPBR(_) => "Transparent PBR Monkey",
                    MaterialType::Unlit(_) => "Transparent Unlit Monkey",
                    _ => "Transparent Monkey"
                }
            }
        };
        
        // ✅ Use Scene Manager API to create renderable entity (Proposal #1 Phase 2)
        use rust_engine::foundation::math::Transform;
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.teapot_mesh.as_ref().unwrap().clone(),
            rust_engine::render::systems::dynamic::MeshType::Teapot,
            selected_material.clone(), // Pass the actual material (not MaterialId)
            Transform::from_position(position),
        );
        
        // ✅ Add LifecycleComponent for automatic lifetime management
        let current_time = self.start_time.elapsed().as_secs_f32();
        let lifecycle = LifecycleComponent::new_with_lifetime(current_time, lifetime);
        self.world.add_component(entity, lifecycle);
        
        log::info!("Spawned monkey (Entity {:?}) at {:?} with {} material (lifetime: {:.1}s)", 
                  entity, position, material_name, lifetime);
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
        
        // Effect pattern - 70% steady, 30% pulse
        let effect_pattern = match rng.gen_range(0..10) {
            0..=6 => EffectPattern::Steady,
            _ => EffectPattern::Pulse {
                frequency: rng.gen_range(0.5..2.0),
                amplitude: rng.gen_range(0.3..0.6),
            },
        };
        
        // Spawn sphere marker for light visualization
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
        // Create unlit emissive material with the light's color (no lighting interaction)
        let sphere_material = MaterialBuilder::new()
            .base_color(color)
            .metallic(0.0)
            .roughness(0.1)
            .emission(color * 2.0) // Bright emission
            .emission_strength(1.0)
            .alpha(0.3) // Moderately transparent
            .build_transparent();
        
        // ✅ Use TransformComponent builder pattern (Proposal #3)
        use rust_engine::ecs::components::TransformComponent;
        let transform_component = TransformComponent::identity()
            .with_position(position)
            .with_uniform_scale(0.8);
        
        // Spawn sphere using multi-mesh system with appropriate scale
        match self.graphics_engine.allocate_from_pool(
            MeshType::Sphere,
            transform_component.to_matrix(),
            sphere_material,
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
    
    fn spawn_random_spaceship(&mut self) {
        let mut rng = thread_rng();
        
        // Random position
        let position = Vec3::new(
            rng.gen_range(-15.0..15.0),
            rng.gen_range(-2.0..6.0),
            rng.gen_range(-15.0..15.0),
        );
        
        // Random rotation
        let rotation = Vec3::new(
            rng.gen_range(0.0..std::f32::consts::TAU),
            rng.gen_range(0.0..std::f32::consts::TAU),
            rng.gen_range(0.0..std::f32::consts::TAU),
        );
        
        // Random lifetime between 10-20 seconds
        let lifetime = rng.gen_range(10.0..20.0);
        
        let scale = rng.gen_range(1.5..3.0); // Varying sizes
        
        // Load spaceship material from MTL file with automatic texture loading
        // MTL file defines: base color, metallic (from Ks), roughness (from Ns), 
        // emission (from Ke), alpha/transparency (from d), and textures
        let spaceship_material = rust_engine::assets::MaterialLoader::load_mtl_with_textures(
            "resources/models/spaceship_simple.mtl",
            "Material.001",
            &mut self.graphics_engine
        ).unwrap_or_else(|e| {
            log::warn!("Failed to load spaceship material from MTL: {}. Using fallback.", e);
            // Fallback material if MTL loading fails
            MaterialBuilder::new()
                .base_color_rgb(0.7, 0.75, 0.8)
                .metallic(0.8)
                .roughness(0.2)
                .emission_rgb(0.2, 0.6, 1.0)
                .emission_strength(3.0)
                .name("Spaceship Fallback")
                .build_transparent()
        });
        
        // ✅ Use Scene Manager API to create renderable entity (Proposal #1 Phase 2)
        use rust_engine::foundation::math::Transform;
        let rotation_quat = Quat::from_euler_angles(rotation.x, rotation.y, rotation.z);
        let entity = self.scene_manager.create_renderable_entity(
            &mut self.world,
            self.spaceship_mesh.as_ref().unwrap().clone(),
            rust_engine::render::systems::dynamic::MeshType::Spaceship,
            spaceship_material.clone(), // Pass the actual material (not MaterialId)
            Transform {
                position,
                rotation: rotation_quat,
                scale: Vec3::new(scale, scale, scale),
            },
        );
        
        // ✅ Add LifecycleComponent for automatic lifetime management
        let current_time = self.start_time.elapsed().as_secs_f32();
        let lifecycle = LifecycleComponent::new_with_lifetime(current_time, lifetime);
        self.world.add_component(entity, lifecycle);
        
        log::info!("Spawned spaceship (Entity {:?}) at {:?} (lifetime: {:.1}s)", 
                  entity, position, lifetime);
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
    
    fn despawn_excess_objects(&mut self) {
        use rust_engine::ecs::RenderableComponent;
        
        // ✅ Use ECS queries to count entities by mesh type
        let current_time = self.start_time.elapsed().as_secs_f32();
        
        // Query all entities with RenderableComponent and LifecycleComponent
        let entities: Vec<_> = self.world.entities().cloned().collect();
        
        // Separate by mesh type (Teapot vs Spaceship)
        let mut teapots: Vec<(Entity, f32)> = Vec::new();
        let mut spaceships: Vec<(Entity, f32)> = Vec::new();
        
        for entity in entities {
            if let Some(renderable) = self.world.get_component::<RenderableComponent>(entity) {
                if let Some(lifecycle) = self.world.get_component::<LifecycleComponent>(entity) {
                    let spawn_time = lifecycle.spawn_time;
                    match renderable.mesh_type {
                        rust_engine::render::systems::dynamic::MeshType::Teapot => {
                            teapots.push((entity, spawn_time));
                        }
                        rust_engine::render::systems::dynamic::MeshType::Spaceship => {
                            spaceships.push((entity, spawn_time));
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Despawn oldest teapots if we exceed the maximum
        if teapots.len() > MAX_TEAPOTS {
            let excess_count = teapots.len() - MAX_TEAPOTS;
            // Sort by spawn time to get oldest first
            teapots.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            
            for i in 0..excess_count {
                let entity = teapots[i].0;
                let age = current_time - teapots[i].1;
                self.scene_manager.destroy_entity(&mut self.world, entity);
                log::info!("Despawned old teapot entity (lived {:.1}s, over limit)", age);
            }
        }
        
        // Despawn oldest spaceships if we exceed the maximum
        if spaceships.len() > MAX_SPACESHIPS {
            let excess_count = spaceships.len() - MAX_SPACESHIPS;
            // Sort by spawn time to get oldest first
            spaceships.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
            
            for i in 0..excess_count {
                let entity = spaceships[i].0;
                let age = current_time - spaceships[i].1;
                self.scene_manager.destroy_entity(&mut self.world, entity);
                log::info!("Despawned old spaceship entity (lived {:.1}s, over limit)", age);
            }
        }
        
        // Despawn oldest lights if we exceed the maximum
        if self.light_instances.len() > MAX_LIGHTS {
            let excess_count = self.light_instances.len() - MAX_LIGHTS;
            
            for _ in 0..excess_count {
                let instance = self.light_instances.remove(0);
                // Remove light component from ECS world
                self.world.remove_component::<LightComponent>(instance.entity);
                log::info!("Despawned old light (over limit)");
            }
        }
    }
    
    fn create_teapot_materials() -> Vec<Material> {
        log::info!("Creating test materials using MaterialBuilder...");
        vec![
            // 1. Red ceramic with emission glow
            MaterialBuilder::new()
                .base_color_rgb(0.8, 0.2, 0.2)
                .metallic(0.1)
                .roughness(0.3)
                .emission_rgb(1.0, 0.5, 0.0)
                .emission_strength(2.0)
                .name("Red Ceramic with Emission")
                .build(),
            
            // 2. Flat green unlit
            MaterialBuilder::new()
                .base_color_rgb(0.2, 0.8, 0.2)
                .metallic(0.0)
                .roughness(1.0)
                .name("Flat Green Unlit")
                .build(),
            
            // 3. Metallic gold
            MaterialBuilder::new()
                .base_color_rgb(1.0, 0.8, 0.2)
                .metallic(0.9)
                .roughness(0.2)
                .emission_rgb(1.0, 0.8, 0.0)
                .emission_strength(1.5)
                .name("Gold Metal")
                .build(),
            
            // 4. Flat magenta unlit
            MaterialBuilder::new()
                .base_color_rgb(0.8, 0.2, 0.8)
                .metallic(0.0)
                .roughness(1.0)
                .name("Flat Magenta Unlit")
                .build(),
            
            // 5. Blue glass (transparent)
            MaterialBuilder::new()
                .base_color_rgb(0.2, 0.2, 0.8)
                .metallic(0.1)
                .roughness(0.1)
                .alpha(0.5)
                .name("Blue Glass")
                .build_transparent(),
            
            // 6. Yellow ghost (transparent unlit)
            MaterialBuilder::new()
                .base_color_rgb(0.8, 0.8, 0.2)
                .metallic(0.0)
                .roughness(1.0)
                .alpha(0.35)
                .name("Yellow Ghost")
                .build_transparent(),
            
            // 7. Cyan glass (highly transparent)
            MaterialBuilder::new()
                .base_color_rgb(0.2, 0.8, 0.8)
                .metallic(0.2)
                .roughness(0.4)
                .alpha(0.3)
                .name("Cyan Glass")
                .build_transparent(),
            
            // 8. Orange ghost (transparent)
            MaterialBuilder::new()
                .base_color_rgb(0.9, 0.5, 0.1)
                .metallic(0.0)
                .roughness(1.0)
                .alpha(0.4)
                .name("Orange Ghost")
                .build_transparent(),
            
            // 9. Silver metal with emission
            MaterialBuilder::new()
                .base_color_rgb(0.95, 0.93, 0.88)
                .metallic(0.9)
                .roughness(0.1)
                .emission_rgb(0.5, 0.7, 1.0)
                .emission_strength(2.5)
                .name("Silver Metal")
                .build(),
            
            // 10. Purple gem (glowing crystal)
            MaterialBuilder::glowing_crystal(Vec3::new(0.6, 0.2, 0.8), 1.5),
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
        
        // Load monkey mesh (has proper UVs and normals for texture testing)
        match ObjLoader::load_obj("resources/models/monkey.obj") {
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
                
                log::info!("Loaded monkey mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.teapot_mesh = Some(mesh);
            }
            Err(e) => {
                log::warn!("Failed to load monkey.obj: {:?}, using fallback cube", e);
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
        
        // Load spaceship mesh
        match ObjLoader::load_obj("resources/models/spaceship_simple.obj") {
            Ok(mesh) => {
                log::info!("Loaded spaceship mesh successfully with {} vertices and {} indices", 
                          mesh.vertices.len(), mesh.indices.len());
                self.spaceship_mesh = Some(mesh);
            }
            Err(e) => {
                log::warn!("Failed to load spaceship_simple.obj: {:?}", e);
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
                log::warn!("Failed to load frigate.obj: {:?}", e);
            }
        }
        
        // Create simple quad mesh for testing
        use rust_engine::render::Vertex;
        let quad_vertices = vec![
            // Bottom-left
            Vertex {
                position: [-1.0, -1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tex_coord: [0.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                _padding: 0.0,
            },
            // Top-left
            Vertex {
                position: [-1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tex_coord: [0.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                _padding: 0.0,
            },
            // Top-right
            Vertex {
                position: [1.0, 1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tex_coord: [1.0, 1.0],
                tangent: [1.0, 0.0, 0.0],
                _padding: 0.0,
            },
            // Bottom-right
            Vertex {
                position: [1.0, -1.0, 0.0],
                normal: [0.0, 0.0, 1.0],
                tex_coord: [1.0, 0.0],
                tangent: [1.0, 0.0, 0.0],
                _padding: 0.0,
            },
        ];
        
        let quad_indices = vec![
            0, 2, 1,  // First triangle (CCW from front)
            0, 3, 2,  // Second triangle (CCW from front)
        ];
        
        let quad_mesh = Mesh {
            vertices: quad_vertices,
            indices: quad_indices,
        };
        self.quad_mesh = Some(quad_mesh.clone());
        log::info!("Created quad mesh for testing");
        
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
            let sphere_materials = vec![
                MaterialBuilder::new()
                    .base_color_rgb(1.0, 1.0, 0.0)
                    .metallic(0.0)
                    .roughness(0.8)
                    .emission_rgb(0.2, 0.2, 0.0)
                    .name("Light Marker")
                    .build()
            ];
            
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
        
        // Create spaceship mesh pool
        if let Some(ref spaceship_mesh) = self.spaceship_mesh {
            use rust_engine::assets::ImageData;
            use rust_engine::render::resources::materials::TextureType;
            
            // Load custom spaceship textures
            log::info!("Loading spaceship textures...");
            let base_texture_path = "resources/textures/spaceship_simple_texture.png";
            let emission_texture_path = "resources/textures/spaceship_simple_emission_map.png";
            
            let base_image = ImageData::from_file(base_texture_path)
                .map_err(|e| format!("Failed to load spaceship base texture: {}", e))?;
            let emission_image = ImageData::from_file(emission_texture_path)
                .map_err(|e| format!("Failed to load spaceship emission texture: {}", e))?;
            
            // Upload textures to GPU
            let base_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                base_image,
                TextureType::BaseColor
            )?;
            let emission_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                emission_image,
                TextureType::Emission
            )?;
            log::info!("Uploaded spaceship textures to GPU");
            
            // Store texture handles for later use
            self.spaceship_base_texture = Some(base_texture_handle);
            self.spaceship_emission_texture = Some(emission_texture_handle);
            
            // Create spaceship material with custom textures
            let spaceship_material = MaterialBuilder::new()
                .base_color_rgb(1.0, 1.0, 1.0) // White to show texture colors
                .metallic(0.8)
                .roughness(0.2)
                .emission_rgb(1.0, 1.0, 1.0) // White to show emission texture colors
                .emission_strength(5.0) // Strong emission for engine glow
                .name("Spaceship")
                .build()
                .with_base_color_texture(base_texture_handle)
                .with_emission_texture(emission_texture_handle);
            
            if let Err(e) = self.graphics_engine.create_mesh_pool(
                MeshType::Spaceship,
                spaceship_mesh,
                &[spaceship_material.clone()],
                20 // Pool for up to 20 spaceships
            ) {
                log::error!("Failed to create spaceship mesh pool: {}", e);
                return Err(e);
            }
            log::info!("Created spaceship mesh pool successfully");
        }
        
        // Create frigate mesh pool
        if let Some(ref frigate_mesh) = self.frigate_mesh {
            use rust_engine::assets::ImageData;
            use rust_engine::render::resources::materials::TextureType;
            
            // Load custom frigate textures
            log::info!("Loading frigate textures...");
            let base_texture_path = "resources/textures/Frigate_texture.png";
            let emission_texture_path = "resources/textures/Frigate_emission.png";
            
            let base_image = ImageData::from_file(base_texture_path)
                .map_err(|e| format!("Failed to load frigate base texture: {}", e))?;
            let emission_image = ImageData::from_file(emission_texture_path)
                .map_err(|e| format!("Failed to load frigate emission texture: {}", e))?;
            
            // Upload textures to GPU
            let base_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                base_image,
                TextureType::BaseColor
            )?;
            let emission_texture_handle = self.graphics_engine.upload_texture_from_image_data(
                emission_image,
                TextureType::Emission
            )?;
            log::info!("Uploaded frigate textures to GPU");
            
            // Store texture handles for later use
            self.frigate_base_texture = Some(base_texture_handle);
            self.frigate_emission_texture = Some(emission_texture_handle);
            
            // Create frigate material with custom textures
            let frigate_material = MaterialBuilder::new()
                .base_color_rgb(1.0, 1.0, 1.0) // White to show texture colors
                .metallic(0.6)
                .roughness(0.3)
                .emission_rgb(1.0, 1.0, 1.0) // White to show emission texture colors
                .emission_strength(8.0) // Strong emission for engines
                .name("Frigate")
                .build()
                .with_base_color_texture(base_texture_handle)
                .with_emission_texture(emission_texture_handle);
            
            if let Err(e) = self.graphics_engine.create_mesh_pool(
                MeshType::Frigate,
                frigate_mesh,
                &[frigate_material.clone()],
                5 // Pool for up to 5 frigates
            ) {
                log::error!("Failed to create frigate mesh pool: {}", e);
                return Err(e);
            }
            log::info!("Created frigate mesh pool successfully");
            
            // Spawn a large background frigate
            self.spawn_background_frigate();
        }
        
        // Load monkey textures for material variations
        {
            use rust_engine::assets::ImageData;
            use rust_engine::render::resources::materials::TextureType;
            
            log::info!("Loading monkey textures...");
            
            // Load base color texture
            if let Ok(base_image) = ImageData::from_file("resources/textures/texture.png") {
                if let Ok(handle) = self.graphics_engine.upload_texture_from_image_data(
                    base_image,
                    TextureType::BaseColor
                ) {
                    self.monkey_base_texture = Some(handle);
                    log::info!("Uploaded monkey base texture to GPU");
                } else {
                    log::warn!("Failed to upload monkey base texture to GPU");
                }
            } else {
                log::warn!("Failed to load monkey base texture from file");
            }
            
            // Load normal map texture
            if let Ok(normal_image) = ImageData::from_file("resources/textures/normal_map.png") {
                if let Ok(handle) = self.graphics_engine.upload_texture_from_image_data(
                    normal_image,
                    TextureType::Normal
                ) {
                    self.monkey_normal_texture = Some(handle);
                    log::info!("Uploaded monkey normal map to GPU");
                } else {
                    log::warn!("Failed to upload monkey normal map to GPU");
                }
            } else {
                log::warn!("Failed to load monkey normal map from file");
            }
            
            // Load emission map texture
            if let Ok(emission_image) = ImageData::from_file("resources/textures/emission_map.png") {
                if let Ok(handle) = self.graphics_engine.upload_texture_from_image_data(
                    emission_image,
                    TextureType::Emission
                ) {
                    self.monkey_emission_texture = Some(handle);
                    log::info!("Uploaded monkey emission map to GPU");
                } else {
                    log::warn!("Failed to upload monkey emission map to GPU");
                }
            } else {
                log::warn!("Failed to load monkey emission map from file");
            }
        }
        
        // Initialize text rendering test
        log::info!("Initializing text rendering test...");
        self.initialize_text_test()?;

        Ok(())
    }
    
    fn initialize_text_test(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load font
        log::info!("Loading font from resources/fonts/default.ttf...");
        let font_data = std::fs::read("resources/fonts/default.ttf")?;
        
        // Create font atlas
        log::info!("Creating font atlas at 48px...");
        let mut font_atlas = FontAtlas::new(&font_data, 48.0)?;
        
        // Upload font atlas to GPU
        log::info!("Uploading font atlas to GPU...");
        let texture_handle = font_atlas.upload_to_gpu(&mut self.graphics_engine)?;
        log::info!("Font atlas uploaded successfully: {:?}", texture_handle);
        
        // Create text material - using lit text so it participates in scene lighting
        // This makes text behave like other world objects (monkeys, etc.)
        let bright_cyan = Vec3::new(0.0, 1.0, 1.0);
        let text_material = create_lit_text_material(texture_handle, bright_cyan);
        log::info!("Text material created");
        
        // Create TextRenderer (simple utility wrapper)
        let text_renderer = TextRenderer::new(font_atlas.clone());
        
        // Store state
        self.font_atlas = Some(font_atlas);
        self.text_renderer = Some(text_renderer);
        self.text_material = Some(text_material.clone());
        
        log::info!("✓ Text rendering system initialized");
        
        // Spawn static 3D LIT text at origin using pool system
        log::info!("Creating static 3D text 'LIT TEXT'...");
        
        use rust_engine::render::systems::text::text_vertices_to_mesh;
        let text_layout = TextLayout::new(self.font_atlas.as_ref().unwrap());
        let (test_vertices, test_indices) = text_layout.layout_text("HI SCOTT LIT TEXT");
        let test_mesh = text_vertices_to_mesh(test_vertices, test_indices);
        
        let test_text_material = self.text_material.as_ref().unwrap().clone();
        
        // Create pool for this text string
        self.graphics_engine.create_mesh_pool(
            MeshType::TextQuad,
            &test_mesh,
            &vec![test_text_material.clone()],
            10
        )?;
        
        // ✅ Spawn at origin using TransformComponent (Proposal #3)
        use rust_engine::ecs::components::TransformComponent;
        let transform_component = TransformComponent::identity()
            .with_uniform_scale(0.05);
        
        let test_text_handle = self.graphics_engine.allocate_from_pool(
            MeshType::TextQuad,
            transform_component.to_matrix(),
            test_text_material,
        )?;
        
        self.text_test_handle = Some(test_text_handle);
        log::info!("✓ Static lit text spawned at origin");
        
        // Create UNLIT text at a different location
        log::info!("Creating unlit text 'UNLIT TEXT'...");
        
        let text_layout = TextLayout::new(self.font_atlas.as_ref().unwrap());
        let (unlit_vertices, unlit_indices) = text_layout.layout_text("UNLIT TEXT");
        let unlit_mesh = text_vertices_to_mesh(unlit_vertices, unlit_indices);
        
        // Create UNLIT material - bright yellow color, always full brightness
        let bright_yellow = Vec3::new(1.0, 1.0, 0.0);
        let unlit_text_material = create_unlit_text_material(texture_handle, bright_yellow);
        log::info!("Unlit text material created");
        
        // Create a separate pool for the unlit text (using Cube mesh type)
        self.graphics_engine.create_mesh_pool(
            MeshType::Cube,
            &unlit_mesh,
            &vec![unlit_text_material.clone()],
            10
        )?;
        
        // ✅ Position unlit text at (5, 2, 0) using TransformComponent (Proposal #3)
        let transform_component = TransformComponent::identity()
            .with_position(Vec3::new(5.0, 2.0, 0.0))
            .with_uniform_scale(0.05);
        
        let unlit_text_handle = self.graphics_engine.allocate_from_pool(
            MeshType::Cube,
            transform_component.to_matrix(),
            unlit_text_material,
        )?;
        
        self.unlit_text_handle = Some(unlit_text_handle);
        log::info!("✓ Unlit text spawned at (5, 2, 0)");
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting dynamic teapot demo...");
        self.initialize()?;
        
        let mut framebuffer_resized = false;
        
        println!("🚀 App starting main loop - move mouse and click to test button at (100, 100)");
        
        while !self.window.should_close() {
            self.window.poll_events();
            
            // Debug: Log every 2 seconds that we're alive
            if self.last_debug_log.elapsed().as_secs() >= 2 {
                println!("💓 App alive, FPS: {:.1}", self.current_fps);
                self.last_debug_log = Instant::now();
            }
            
            // Handle events FIRST
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
                            // Update UI screen size
                            self.ui_manager.set_screen_size(width as f32, height as f32);
                        }
                    }
                    WindowEvent::CursorPos(x, y) => {
                        // Update mouse position for UI
                        self.ui_manager.update_mouse_position(x as f32, y as f32);
                        self.last_mouse_pos = (x, y);
                    }
                    WindowEvent::MouseButton(button, action, _) => {
                        // Update mouse button state for UI
                        use rust_engine::ui::MouseButton as UIMouseButton;
                        let ui_button = match button {
                            glfw::MouseButton::Button1 => UIMouseButton::Left,
                            glfw::MouseButton::Button2 => UIMouseButton::Right,
                            glfw::MouseButton::Button3 => UIMouseButton::Middle,
                            _ => continue, // Ignore other buttons
                        };
                        let pressed = action == Action::Press;
                        self.ui_manager.update_mouse_button(ui_button, pressed);
                    }
                    WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                        // Play audio beep with voice management
                        if let Some(audio) = &mut self.audio {
                            let (active, max) = audio.get_voice_stats();
                            log::info!("🔊 Voice stats before play: {}/{} voices active", active, max);
                            
                            match audio.play_audio_file_with_priority(
                                "resources/audio/test_beep.wav",
                                rust_engine::audio::VoicePriority::Normal,
                                Some("space_beep".to_string()),
                                rust_engine::audio::mixer::VolumeGroup::SFX,
                            ) {
                                Ok(handle) => {
                                    log::info!("🔊 Playing beep sound!");
                                    self.last_beep_sound = Some(handle);
                                    
                                    let (active, max) = audio.get_voice_stats();
                                    log::info!("🔊 Voice stats after play: {}/{} voices active", active, max);
                                }
                                Err(e) => log::warn!("⚠️ Failed to play beep: {} (Instance limit or pool full)", e),
                            }
                        } else {
                            log::info!("Space key pressed - audio system not available");
                        }
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
            
            // Render frame (sync and rendering now happens inside render_frame)
            if let Err(e) = self.render_frame() {
                log::error!("Failed to render frame: {:?}", e);
                break;
            }
        }
        
        log::info!("Dynamic teapot demo completed");
        Ok(())
    }
    
    fn update_scene(&mut self) {
        let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Update audio system (cleans up finished sounds)
        if let Some(audio) = &mut self.audio {
            audio.update(delta_time);
        }
        
        // Dynamic spawning with random intervals
        let mut rng = thread_rng();
        let now = Instant::now();
        
        // Spawn teapots randomly (every 2-5 seconds, less frequently)
        if now.duration_since(self.last_teapot_spawn).as_secs_f32() > rng.gen_range(2.0..5.0) {
            self.spawn_random_teapot();
            self.last_teapot_spawn = now;
        }
        
        // Spawn spaceships randomly (every 1-2 seconds, more frequently)
        if now.duration_since(self.last_spaceship_spawn).as_secs_f32() > rng.gen_range(1.0..2.0) {
            self.spawn_random_spaceship();
            self.last_spaceship_spawn = now;
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
                    // True flickering: combine multiple sine waves at different frequencies for erratic behavior
                    let t = elapsed_seconds + light_instance.phase_offset;
                    let flicker1 = (t * base_rate).sin();
                    let flicker2 = (t * base_rate * 2.3).sin(); // Prime-ish multiplier for decoherence
                    let flicker3 = (t * base_rate * 5.7).sin(); // Higher frequency jitter
                    let flicker4 = (t * base_rate * 11.3).sin(); // Even higher frequency
                    // Combine with different weights for chaotic, flickering effect
                    let combined_flicker = (flicker1 * 0.5 + flicker2 * 0.3 + flicker3 * 0.15 + flicker4 * 0.05) * chaos;
                    // Clamp to prevent negative intensity or extreme values
                    (light_instance.base_intensity * (1.0 + combined_flicker)).max(0.1).min(light_instance.base_intensity * 2.0)
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

                    // Build transform matrix from position and scale
                    use rust_engine::foundation::math::{Mat4, Vector3};
                    let pos = &light_instance.current_position;
                    let translation = Mat4::new_translation(&Vector3::new(pos.x, pos.y, pos.z));
                    let scale_matrix = Mat4::new_nonuniform_scaling(&Vector3::new(
                        scale_vec.x, scale_vec.y, scale_vec.z
                    ));
                    let transform = translation * scale_matrix;

                    // Update both position and scale
                    if let Err(e) = self.graphics_engine.update_pool_instance(
                        MeshType::Sphere,
                        sphere_handle,
                        transform,
                    ) {
                        log::warn!("Failed to update sphere marker transform: {}", e);
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
        
        // Update FPS counter
        self.frame_count += 1;
        let fps_elapsed = self.fps_update_timer.elapsed();
        if fps_elapsed.as_secs_f32() >= 0.5 {  // Update FPS every 0.5 seconds
            self.current_fps = self.frame_count as f32 / fps_elapsed.as_secs_f32();
            self.frame_count = 0;
            self.fps_update_timer = Instant::now();
        }
        
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        
        // Update FPS text
        let fps_text = format!("FPS: {:.1}", self.current_fps);
        self.ui_manager.update_text(self.fps_label_id, fps_text);
        
        // Update UI state
        self.ui_manager.update(delta_time);
        
        // ✅ PHASE 4 COMPLETE: Sync ECS → Scene Manager
        self.scene_manager.sync_from_world(&mut self.world);
        let render_queue = self.scene_manager.build_render_queue();
        
        // ✅ PROPOSAL #1 COMPLETE: Engine automatically manages entity→handle lifecycle
        // No more manual tracking! No more hacky mesh classification! Engine knows everything.
        self.graphics_engine.render_entities_from_queue(&render_queue)?;
        
        // Build multi-light environment from entity system
        let multi_light_env = self.build_multi_light_environment_from_entities().clone();
        
        // Export UI data for rendering
        let ui_data = self.ui_manager.get_render_data();
        
        // ✅ PROPOSAL #6: Unified frame rendering (replaces ~100 lines of ceremony)
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
        
        // Check for button click and play audio
        if *self.button_clicked_flag.borrow() {
            if let Some(audio) = &mut self.audio {
                let (active, max) = audio.get_voice_stats();
                log::info!("🎵 Normal beep button - voice stats: {}/{} voices active", active, max);
                
                match audio.play_audio_file_with_priority(
                    "resources/audio/test_beep.wav",
                    rust_engine::audio::VoicePriority::Normal,
                    Some("normal_beep".to_string()),
                    rust_engine::audio::mixer::VolumeGroup::SFX,
                ) {
                    Ok(handle) => {
                        log::info!("🔊 Playing normal beep (440Hz, 0.5s)");
                        self.last_beep_sound = Some(handle);
                    }
                    Err(e) => log::warn!("⚠️ Failed to play normal beep: {} (Hit instance limit - max 4 per sound)", e),
                }
            }
            *self.button_clicked_flag.borrow_mut() = false; // Reset flag
        }
        
        // Check for high beep button
        if *self.high_beep_clicked_flag.borrow() {
            if let Some(audio) = &mut self.audio {
                let (active, max) = audio.get_voice_stats();
                log::info!("🎵 High beep button - voice stats: {}/{} voices active", active, max);
                
                match audio.play_audio_file_with_priority(
                    "resources/audio/beep_high.wav",
                    rust_engine::audio::VoicePriority::Normal,
                    Some("high_beep".to_string()),
                    rust_engine::audio::mixer::VolumeGroup::SFX,
                ) {
                    Ok(handle) => {
                        log::info!("🔊 Playing high beep (880Hz, 1.0s)");
                        self.last_beep_sound = Some(handle);
                    }
                    Err(e) => log::warn!("⚠️ Failed to play high beep: {} (Hit instance limit - max 4 per sound)", e),
                }
            }
            *self.high_beep_clicked_flag.borrow_mut() = false;
        }
        
        // Check for low beep button
        if *self.low_beep_clicked_flag.borrow() {
            if let Some(audio) = &mut self.audio {
                let (active, max) = audio.get_voice_stats();
                log::info!("🎵 Low beep button - voice stats: {}/{} voices active", active, max);
                
                match audio.play_audio_file_with_priority(
                    "resources/audio/beep_low.wav",
                    rust_engine::audio::VoicePriority::Normal,
                    Some("low_beep".to_string()),
                    rust_engine::audio::mixer::VolumeGroup::SFX,
                ) {
                    Ok(handle) => {
                        log::info!("🔊 Playing low beep (220Hz, 1.5s)");
                        self.last_beep_sound = Some(handle);
                    }
                    Err(e) => log::warn!("⚠️ Failed to play low beep: {} (Hit instance limit - max 4 per sound)", e),
                }
            }
            *self.low_beep_clicked_flag.borrow_mut() = false;
        }
        
        // Check for spam test button (plays 5 sounds rapidly to test voice limiting)
        if *self.spam_test_clicked_flag.borrow() {
            if let Some(audio) = &mut self.audio {
                let (active, max) = audio.get_voice_stats();
                log::info!("🎵🎵🎵 SPAM TEST - Playing 5 beeps rapidly! Voice stats: {}/{}", active, max);
                
                // Play 5 sounds rapidly - should hit instance limit
                for i in 0..5 {
                    match audio.play_audio_file_with_priority(
                        "resources/audio/test_beep.wav",
                        rust_engine::audio::VoicePriority::Normal,
                        Some("spam_beep".to_string()),
                        rust_engine::audio::mixer::VolumeGroup::SFX,
                    ) {
                        Ok(_) => log::info!("  Spam beep #{} started", i + 1),
                        Err(e) => log::warn!("  Spam beep #{} failed: {}", i + 1, e),
                    }
                }
                
                let (active_after, max) = audio.get_voice_stats();
                log::info!("🎵 After spam test - voice stats: {}/{} voices active", active_after, max);
            }
            *self.spam_test_clicked_flag.borrow_mut() = false;
        }
        
        // Handle volume control buttons
        if let Some(audio) = &mut self.audio {
            const VOLUME_STEP: f32 = 0.1;
            
            // Master volume controls
            if *self.master_vol_up_flag.borrow() {
                let current = audio.get_group_volume(rust_engine::audio::mixer::VolumeGroup::Master);
                let new_volume = (current + VOLUME_STEP).min(1.0);
                audio.set_group_volume(rust_engine::audio::mixer::VolumeGroup::Master, new_volume);
                self.ui_manager.update_text(self.master_vol_label_id, format!("Master: {}%", (new_volume * 100.0) as u32));
                log::info!("🔊 Master volume: {}%", (new_volume * 100.0) as u32);
                *self.master_vol_up_flag.borrow_mut() = false;
            }
            if *self.master_vol_down_flag.borrow() {
                let current = audio.get_group_volume(rust_engine::audio::mixer::VolumeGroup::Master);
                let new_volume = (current - VOLUME_STEP).max(0.0);
                audio.set_group_volume(rust_engine::audio::mixer::VolumeGroup::Master, new_volume);
                self.ui_manager.update_text(self.master_vol_label_id, format!("Master: {}%", (new_volume * 100.0) as u32));
                log::info!("🔊 Master volume: {}%", (new_volume * 100.0) as u32);
                *self.master_vol_down_flag.borrow_mut() = false;
            }
            
            // SFX volume controls
            if *self.sfx_vol_up_flag.borrow() {
                let current = audio.get_group_volume(rust_engine::audio::mixer::VolumeGroup::SFX);
                let new_volume = (current + VOLUME_STEP).min(1.0);
                audio.set_group_volume(rust_engine::audio::mixer::VolumeGroup::SFX, new_volume);
                self.ui_manager.update_text(self.sfx_vol_label_id, format!("SFX: {}%", (new_volume * 100.0) as u32));
                log::info!("🔊 SFX volume: {}%", (new_volume * 100.0) as u32);
                *self.sfx_vol_up_flag.borrow_mut() = false;
            }
            if *self.sfx_vol_down_flag.borrow() {
                let current = audio.get_group_volume(rust_engine::audio::mixer::VolumeGroup::SFX);
                let new_volume = (current - VOLUME_STEP).max(0.0);
                audio.set_group_volume(rust_engine::audio::mixer::VolumeGroup::SFX, new_volume);
                self.ui_manager.update_text(self.sfx_vol_label_id, format!("SFX: {}%", (new_volume * 100.0) as u32));
                log::info!("🔊 SFX volume: {}%", (new_volume * 100.0) as u32);
                *self.sfx_vol_down_flag.borrow_mut() = false;
            }
            
            // Music volume controls
            if *self.music_vol_up_flag.borrow() {
                let current = audio.get_group_volume(rust_engine::audio::mixer::VolumeGroup::Music);
                let new_volume = (current + VOLUME_STEP).min(1.0);
                audio.set_group_volume(rust_engine::audio::mixer::VolumeGroup::Music, new_volume);
                self.ui_manager.update_text(self.music_vol_label_id, format!("Music: {}%", (new_volume * 100.0) as u32));
                log::info!("🔊 Music volume: {}%", (new_volume * 100.0) as u32);
                *self.music_vol_up_flag.borrow_mut() = false;
            }
            if *self.music_vol_down_flag.borrow() {
                let current = audio.get_group_volume(rust_engine::audio::mixer::VolumeGroup::Music);
                let new_volume = (current - VOLUME_STEP).max(0.0);
                audio.set_group_volume(rust_engine::audio::mixer::VolumeGroup::Music, new_volume);
                self.ui_manager.update_text(self.music_vol_label_id, format!("Music: {}%", (new_volume * 100.0) as u32));
                log::info!("🔊 Music volume: {}%", (new_volume * 100.0) as u32);
                *self.music_vol_down_flag.borrow_mut() = false;
            }
        }
        
        // Handle music control buttons
        if let Some(audio) = &mut self.audio {
            // Start/Stop toggle button
            if *self.play_music_flag.borrow() {
                if self.music_is_playing {
                    // Stop music
                    match audio.stop_music(Some(0.5)) {
                        Ok(_) => {
                            self.music_is_playing = false;
                            self.ui_manager.update_text(self.music_status_label_id, "Music: Stopped".to_string());
                            log::info!("⏹ Stopped music");
                        }
                        Err(e) => log::warn!("⚠️ Failed to stop music: {}", e),
                    }
                } else {
                    // Start music
                    use rust_engine::audio::MusicTrack;
                    
                    let track = MusicTrack::new("background_music", "resources/audio/music.mp3")
                        .with_fade_duration(1.0)
                        .with_volume(0.7);
                    
                    match audio.play_music_track(track, Some(0.5)) {
                        Ok(_) => {
                            self.music_is_playing = true;
                            self.ui_manager.update_text(self.music_status_label_id, "Music: Playing".to_string());
                            log::info!("🎵 Started playing music.mp3");
                        }
                        Err(e) => {
                            log::warn!("⚠️ Failed to play music: {}", e);
                        }
                    }
                }
                *self.play_music_flag.borrow_mut() = false;
            }
            
            // Pause/Resume music
            if *self.pause_music_flag.borrow() {
                if audio.is_music_playing() {
                    match audio.pause_music() {
                        Ok(_) => {
                            self.ui_manager.update_text(self.music_status_label_id, "Music: Paused".to_string());
                            log::info!("⏸ Paused music");
                        }
                        Err(e) => log::warn!("⚠️ Failed to pause music: {}", e),
                    }
                } else {
                    // Resume if paused
                    match audio.resume_music() {
                        Ok(_) => {
                            self.ui_manager.update_text(self.music_status_label_id, "Music: Playing".to_string());
                            log::info!("▶ Resumed music");
                        }
                        Err(e) => log::warn!("⚠️ Failed to resume music: {}", e),
                    }
                }
                *self.pause_music_flag.borrow_mut() = false;
            }
            
            // Loop toggle button
            if *self.stop_music_flag.borrow() {
                self.music_loop_enabled = !self.music_loop_enabled;
                
                // Update button text to reflect new state
                let loop_text = if self.music_loop_enabled {
                    "🔁 Loop: On"
                } else {
                    "🔁 Loop: Off"
                };
                
                if let Some(button) = self.ui_manager.get_button_mut(self.stop_music_button_id) {
                    button.text = loop_text.to_string();
                }
                
                log::info!("🔁 Music loop: {}", if self.music_loop_enabled { "ON" } else { "OFF" });
                *self.stop_music_flag.borrow_mut() = false;
            }
            
            // Update music status label based on current state
            use rust_engine::audio::MusicState;
            match audio.music_state() {
                MusicState::Playing => {
                    let time = audio.music_playback_time();
                    let loop_indicator = if self.music_loop_enabled { " 🔁" } else { "" };
                    self.ui_manager.update_text(self.music_status_label_id, format!("Music: Playing ({:.1}s){}", time, loop_indicator));
                }
                MusicState::Crossfading => {
                    self.ui_manager.update_text(self.music_status_label_id, "Music: Crossfading...".to_string());
                }
                MusicState::Paused => {
                    self.ui_manager.update_text(self.music_status_label_id, "Music: Paused".to_string());
                }
                MusicState::Stopped => {
                    // Music stopped, update state
                    if self.music_is_playing {
                        self.music_is_playing = false;
                        
                        // If looping is enabled, restart the music
                        if self.music_loop_enabled {
                            use rust_engine::audio::MusicTrack;
                            let track = MusicTrack::new("background_music", "resources/audio/music.mp3")
                                .with_fade_duration(1.0)
                                .with_volume(0.7);
                            
                            if let Ok(_) = audio.play_music_track(track, Some(0.5)) {
                                self.music_is_playing = true;
                                log::info!("🔁 Looping music...");
                            }
                        }
                    }
                }
            }
        }
        
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
                
                // Count teapot entities using ECS query
                use rust_engine::render::systems::dynamic::MeshType;
                let teapot_count = self.world.query::<RenderableComponent>()
                    .iter()
                    .filter(|(_, renderable)| matches!(renderable.mesh_type, MeshType::Teapot))
                    .count();
                
                log::info!("Total teapot instances: {}, {} active lights (sunlight + {} dynamic)", 
                          teapot_count,
                          self.light_instances.len() + 1, // +1 for sunlight
                          self.light_instances.len());
                LAST_LOG_TIME = elapsed_seconds;
            }
        }
        
        Ok(())
    }

    /// ✅ Update entity lifecycles and automatically destroy expired entities
    fn update_lifecycle_system(&mut self) {
        let current_time = self.start_time.elapsed().as_secs_f32();
        let mut entities_to_destroy = Vec::new();
        
        // Update all lifecycle components
        let entities: Vec<_> = self.world.entities().cloned().collect();
        for entity in entities {
            if let Some(lifecycle) = self.world.get_component_mut::<LifecycleComponent>(entity) {
                lifecycle.update(current_time, 0.016); // Use ~60 FPS delta
                
                // Mark expired entities for removal
                if lifecycle.should_destroy() {
                    entities_to_destroy.push(entity);
                }
            }
        }
        
        // Destroy expired entities
        for entity in entities_to_destroy {
            log::debug!("Destroying expired entity {:?}", entity);
            self.scene_manager.destroy_entity(&mut self.world, entity);
        }
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
