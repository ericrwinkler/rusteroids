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

pub struct IntegratedApp {
    window: WindowHandle,
    renderer: Renderer,
    camera: Camera,
    teapot_mesh: Option<Mesh>,
    teapot_material: Material,
    lighting_env: LightingEnvironment,
    time: f32,
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
            45.0f32.to_radians(),
            800.0 / 600.0,
            0.1,
            100.0
        );
        
        // Position camera to look at teapot
        camera.position = Vec3::new(0.0, 2.0, 8.0);
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
            time: 0.0,
        }
    }
    
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load teapot mesh using OBJ loader
        match ObjLoader::load_obj("resources/models/teapot.obj") {
            Ok(mesh) => {
                println!("Successfully loaded teapot.obj with {} vertices", mesh.vertices.len());
                self.teapot_mesh = Some(mesh);
            }
            Err(e) => {
                eprintln!("Failed to load teapot.obj: {}", e);
                return Err(format!("Failed to load teapot model: {}", e).into());
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
                        self.camera.position = Vec3::new(0.0, 2.0, 8.0);
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
            self.time += 0.016; // Assume ~60 FPS
            
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
        // Rotate camera around teapot
        let radius = 8.0;
        let camera_x = (self.time * 0.5).cos() * radius;
        let camera_z = (self.time * 0.5).sin() * radius;
        let camera_y = 2.0 + (self.time * 0.3).sin() * 1.0;
        
        self.camera.position = Vec3::new(camera_x, camera_y, camera_z);
        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0));
        
        // Add some dynamic lighting
        if let Some(light) = self.lighting_env.lights.get_mut(0) {
            light.position = Vec3::new(
                (self.time * 0.7).cos() * 5.0,
                3.0,
                (self.time * 0.7).sin() * 5.0
            );
        }
    }
    
    fn render_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin frame
        self.renderer.begin_frame();
        
        // Set camera and lighting for the frame
        self.renderer.set_camera(&self.camera);
        self.renderer.set_lighting(&self.lighting_env);
        
        // Render teapot if loaded
        if let Some(ref teapot_mesh) = self.teapot_mesh {
            // Create model matrix (identity for now, teapot at origin)
            let model_matrix = Mat4::identity();
            
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
