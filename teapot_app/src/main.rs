//! Teapot demo application
//! 
//! This demonstrates the engine's rendering capabilities by displaying
//! a rotating teapot model.

use rust_engine::assets::obj_loader::ObjLoader;
use rust_engine::render::{
    Camera,
    Mesh,
    Material,
    lighting::{LightingEnvironment, Light},
    Renderer,
    WindowHandle,
};
use rust_engine::foundation::math::{Vec3, Mat4, Mat4Ext};
use glfw::{Action, Key, WindowEvent};
use std::time::Instant;

pub struct IntegratedApp {
    window: WindowHandle,
    renderer: Renderer,
    camera: Camera,
    teapot_mesh: Option<Mesh>,
    teapot_material: Material,
    lighting_env: LightingEnvironment,
    start_time: Instant,
    total_rotation: f32, // Total rotation in radians
}

impl IntegratedApp {
    pub fn new() -> Self {
        log::info!("Creating teapot demo application...");
        // Create window
        log::info!("Creating window...");
        let mut window = WindowHandle::new(800, 600, "Rusteroids - Teapot Demo");
        log::info!("Window created successfully");
        
        // Create renderer
        log::info!("Creating Vulkan renderer...");
        let renderer = Renderer::new_from_window(&mut window);
        log::info!("Vulkan renderer created successfully");
        
        // Create camera using Vulkan coordinate system
        log::info!("Creating camera...");
        let mut camera = Camera::perspective(
            Vec3::new(4.0, -6.0, 8.0), // Vulkan: -Y = above origin, +Z = away from origin
            45.0, // FOV in degrees - will be converted to radians in update_projection_matrix
            800.0 / 600.0,
            0.1,
            100.0
        );
        
        // Position camera to look at teapot using Vulkan coordinate conventions
        camera.set_position(Vec3::new(4.0, -6.0, 8.0)); // Vulkan: -Y = above, +Z = away from origin
        camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0)); // Vulkan Y-down up vector
        
        // Create material for teapot
        let teapot_material = Material::new()
            .with_color(0.8, 0.7, 0.5) // Warm tan color instead of extreme magenta
            .with_metallic(0.1)  // Metallic
            .with_roughness(0.3); // Roughness
        
        // Create custom lighting environment with single strong directional light
        // NOTE: Current renderer only supports ONE light (uses first directional light only)
        // Multiple lights require expanding push constants or using uniform buffers
        let lighting_env = LightingEnvironment::new()
            // Low ambient for dramatic contrast
            .with_ambient(Vec3::new(0.15, 0.12, 0.18), 0.1) // Slightly blue ambient
            // Main directional light from upper-left-front using Vulkan coordinates
            .add_light(Light::directional(
                Vec3::new(-0.5, -1.0, 0.3),  // Vulkan: -Y = from above, +Z = toward viewer
                Vec3::new(1.0, 0.95, 0.9),   // Warm white color
                2.0,                         // Strong intensity for clear lighting
            ));
        
        Self {
            window,
            renderer,
            camera,
            teapot_mesh: None,
            teapot_material,
            lighting_env,
            start_time: Instant::now(),
            total_rotation: 0.0,
        }
    }
    
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Initializing teapot demo...");
        
        // Log current working directory for debugging
        if let Ok(cwd) = std::env::current_dir() {
            log::info!("Current working directory: {:?}", cwd);
        }
        
        // Load teapot OBJ model
        // Load teapot mesh with pre-generated normals
        match ObjLoader::load_obj("resources/models/teapot_with_normals.obj") {
            Ok(mut mesh) => {
                // Check if vertices are reasonable (not too large/small)
                let mut min_pos = [f32::MAX; 3];
                let mut max_pos = [f32::MIN; 3];
                
                for vertex in &mesh.vertices {
                    for i in 0..3 {
                        min_pos[i] = min_pos[i].min(vertex.position[i]);
                        max_pos[i] = max_pos[i].max(vertex.position[i]);
                    }
                }
                
                // If the model is too large, scale it down
                let max_extent = (max_pos[0] - min_pos[0]).max(max_pos[1] - min_pos[1]).max(max_pos[2] - min_pos[2]);
                if max_extent > 10.0 {
                    let scale_factor = 4.0 / max_extent; // Scale to fit in 4 unit cube
                    
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
                // Fallback to test cube if teapot loading fails
                log::warn!("Failed to load teapot.obj: {:?}, using fallback cube", e);
                self.teapot_mesh = Some(Mesh::cube());
            }
        }
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Starting teapot demo...");
        self.initialize()?;
        
        let mut framebuffer_resized = false;
        
        while !self.window.should_close() {
            self.window.poll_events();
            
            // Collect events to avoid borrow checker issues
            let events: Vec<_> = self.window.event_iter().collect();
            for (_, event) in events {
                match event {
                    WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        self.window.set_should_close(true);
                    }
                    WindowEvent::FramebufferSize(width, height) => {
                        framebuffer_resized = true;
                        // Update camera aspect ratio
                        if width > 0 && height > 0 {
                            self.camera.set_aspect_ratio(width as f32 / height as f32);
                        }
                    }
                    WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                        // Reset camera position using Vulkan coordinates
                        self.camera.set_position(Vec3::new(0.0, -2.0, 8.0));  // Vulkan: -Y = above
                        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
                    }
                    _ => {}
                }
            }
            
            if framebuffer_resized {
                // Recreate swapchain for the new window size
                self.renderer.recreate_swapchain(&mut self.window);
                framebuffer_resized = false;
                continue;
            }
            
            // Update time for animations
            let current_time = Instant::now();
            let elapsed_seconds = current_time.duration_since(self.start_time).as_secs_f32();
            
            // Calculate rotation: 1 full rotation every 8 seconds (π/4 radians per second)
            let angular_velocity = std::f32::consts::PI / 4.0;
            self.total_rotation = elapsed_seconds * angular_velocity;
            
            // Update camera and lighting
            self.update_scene();
            
            // Render frame
            if let Err(e) = self.render_frame() {
                eprintln!("Render error: {:?}", e);
                break;
            }
        }
        
        log::info!("Teapot demo completed");
        Ok(())
    }
    
    fn update_scene(&mut self) {
        // Calculate elapsed time for smooth animations
        let elapsed_seconds = self.start_time.elapsed().as_secs_f32();
        
        // Camera oscillation: move up and down slowly (period of 6 seconds)
        let oscillation_period = 6.0; // seconds for full up-down-up cycle
        let oscillation_amplitude = 3.0; // how far up/down to move
        let base_height = -4.0; // Vulkan: negative Y = above teapot
        
        let y_offset = (elapsed_seconds * 2.0 * std::f32::consts::PI / oscillation_period).sin() * oscillation_amplitude;
        let camera_y = base_height + y_offset;
        
        // Keep camera at fixed X and Z distance, but vary height
        let camera_x = 4.0;   // To the right for perspective  
        let camera_z = 8.0;   // Away from object (Vulkan: +Z = away from origin)
        
        // Update camera position to oscillate up and down using Vulkan coordinates
        self.camera.set_position(Vec3::new(camera_x, camera_y, camera_z));
        // Always look at the teapot at origin - use Vulkan Y-down up vector
        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
    }
    
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin frame
        self.renderer.begin_frame();
        
        // Set camera and lighting for the frame
        self.renderer.set_camera(&self.camera);
        self.renderer.set_lighting(&self.lighting_env);
        
        // Render teapot if loaded
        if let Some(ref teapot_mesh) = self.teapot_mesh {
            // Rotate around Y axis for spinning - no additional transforms needed 
            // since assets are now loaded in correct Vulkan coordinate system
            let rotation_y = self.total_rotation;
            let model_matrix = Mat4::rotation_y(rotation_y);
            
            // Submit teapot for rendering
            self.renderer.draw_mesh_3d(
                teapot_mesh,
                &model_matrix,
                &self.teapot_material
            )?;
        } else {
            log::warn!("No teapot mesh to render!");
        }
        
        // End frame and present
        self.renderer.end_frame(&mut self.window)?;
        
        Ok(())
    }
}

impl Drop for IntegratedApp {
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
        .filter_level(log::LevelFilter::Debug) // Changed from Info to Debug
        .init();
    
    log::info!("Starting Rusteroids Teapot Demo");
    
    // Create and run the application
    log::info!("Creating application instance...");
    
    // Wrap in catch_unwind to handle panics gracefully
    let result = std::panic::catch_unwind(|| {
        let mut app = IntegratedApp::new();
        log::info!("Running application...");
        app.run()
    });
    
    match result {
        Ok(Ok(())) => {
            log::info!("Teapot demo finished successfully");
            Ok(())
        }
        Ok(Err(e)) => {
            log::error!("Application error: {:?}", e);
            Err(e)
        }
        Err(panic) => {
            log::error!("Application panicked: {:?}", panic);
            Err("Application panicked during execution".into())
        }
    }
}
