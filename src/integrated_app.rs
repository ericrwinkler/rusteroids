// src/integrated_app.rs
// Integrated application using the new modular renderer

use crate::assets::obj_loader::ObjLoader;
use crate::render::{
    camera::Camera,
    mesh::Mesh,
    material::Material,
    lighting::{Light, LightType, LightingEnvironment},
    renderer::Renderer,
    window::WindowHandle,
};
use crate::util::math::{Vec3, Mat4};
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
        // Create window
        let mut window = WindowHandle::new(800, 600, "Rusteroids - Teapot Demo");
        
        // Create renderer
        let renderer = Renderer::new(&mut window);
        
        // Create camera
        let mut camera = Camera::perspective(
            Vec3::new(0.0, 2.0, 8.0), // Position
            45.0, // FOV in degrees - will be converted to radians in update_projection_matrix
            800.0 / 600.0,
            0.1,
            100.0
        );
        
        // Position camera to look at teapot using proper setter methods
        camera.set_position(Vec3::new(0.0, 2.0, 8.0));
        camera.look_at(Vec3::new(0.0, 0.0, 0.0));
        
        // Create material for teapot
        let teapot_material = Material::new()
            .with_color(0.7, 0.5, 0.3) // Base color - bronze-ish
            .with_metallic(0.1)  // Metallic
            .with_roughness(0.3); // Roughness
        
        // Create lighting environment with warm indoor lighting
        let lighting_env = LightingEnvironment::indoor_warm();
        
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
        // Load teapot OBJ model
        match ObjLoader::load_obj("resources/models/teapot.obj") {
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
                
                self.teapot_mesh = Some(mesh);
            }
            Err(_) => {
                // Fallback to test cube if teapot loading fails
                self.teapot_mesh = Some(Mesh::cube());
            }
        }
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
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
                        // Reset camera position
                        self.camera.set_position(Vec3::new(0.0, 2.0, 8.0));
                        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0));
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
        
        Ok(())
    }
    
    fn update_scene(&mut self) {
        // Position camera for good viewing angle
        let camera_x = 3.0;
        let camera_z = 8.0;
        let camera_y = 3.0;
        
        // Use proper setter methods to ensure matrices are updated
        self.camera.set_position(Vec3::new(camera_x, camera_y, camera_z));
        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0));
    }
    
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin frame
        self.renderer.begin_frame();
        
        // Set camera and lighting for the frame
        self.renderer.set_camera(&self.camera);
        self.renderer.set_lighting(&self.lighting_env);
        
        // Render teapot if loaded
        if let Some(ref teapot_mesh) = self.teapot_mesh {
            // Apply rotation and flip to correct orientation
            let rotation_y = self.total_rotation;
            let flip_matrix = Mat4::rotation_x(std::f32::consts::PI);
            let rotation_matrix = Mat4::rotation_y(rotation_y);
            let model_matrix = rotation_matrix * flip_matrix;
            
            // Submit teapot for rendering
            self.renderer.draw_mesh_3d(
                teapot_mesh,
                &model_matrix,
                &self.teapot_material
            )?;
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
