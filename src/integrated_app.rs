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
            time: 0.0,
        }
    }
    
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // DEBUGGING: Create a simple test cube instead of loading OBJ
        use crate::render::mesh::{Vertex, Mesh};
        
        // DEBUG: Let's create an extremely simple cube that should definitely be visible
        let test_vertices = vec![
            // Front face (much larger and clearly positioned)
            Vertex { position: [-2.0, -2.0,  2.0], normal: [0.0, 0.0, 1.0], tex_coord: [0.0, 0.0] },
            Vertex { position: [ 2.0, -2.0,  2.0], normal: [0.0, 0.0, 1.0], tex_coord: [1.0, 0.0] },
            Vertex { position: [ 2.0,  2.0,  2.0], normal: [0.0, 0.0, 1.0], tex_coord: [1.0, 1.0] },
            Vertex { position: [-2.0,  2.0,  2.0], normal: [0.0, 0.0, 1.0], tex_coord: [0.0, 1.0] },
            
            // Back face
            Vertex { position: [-2.0, -2.0, -2.0], normal: [0.0, 0.0, -1.0], tex_coord: [1.0, 0.0] },
            Vertex { position: [-2.0,  2.0, -2.0], normal: [0.0, 0.0, -1.0], tex_coord: [1.0, 1.0] },
            Vertex { position: [ 2.0,  2.0, -2.0], normal: [0.0, 0.0, -1.0], tex_coord: [0.0, 1.0] },
            Vertex { position: [ 2.0, -2.0, -2.0], normal: [0.0, 0.0, -1.0], tex_coord: [0.0, 0.0] },
        ];
        
        // DEBUG: Print vertex positions to verify they're reasonable
        println!("Test cube vertices:");
        for (i, vertex) in test_vertices.iter().enumerate() {
            println!("  Vertex {}: pos=({:.1}, {:.1}, {:.1})", i, vertex.position[0], vertex.position[1], vertex.position[2]);
        }
        
        let test_indices = vec![
            // Front face
            0, 1, 2,  2, 3, 0,
            // Back face  
            4, 5, 6,  6, 7, 4,
            // Left face
            4, 0, 3,  3, 5, 4,
            // Right face
            1, 7, 6,  6, 2, 1,
            // Top face
            3, 2, 6,  6, 5, 3,
            // Bottom face
            4, 7, 1,  1, 0, 4,
        ];
        
        println!("Test cube indices: {:?}", test_indices);
        
        let test_mesh = Mesh {
            vertices: test_vertices,
            indices: test_indices,
        };
        
        println!("Created test cube with {} vertices, {} triangles", 
                 test_mesh.vertices.len(), test_mesh.indices.len() / 3);
        self.teapot_mesh = Some(test_mesh);
        
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize()?;
        
        println!("Starting main loop...");
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
        // REPOSITION: Much closer camera position and off-center angle to see the cube clearly
        let camera_x = 3.0;
        let camera_z = 8.0; // Closer than before
        let camera_y = 3.0; // Angle from above
        
        // Use proper setter methods to ensure matrices are updated
        self.camera.set_position(Vec3::new(camera_x, camera_y, camera_z));
        self.camera.look_at(Vec3::new(0.0, 0.0, 0.0)); // Look at cube at origin
        
        // Print camera info for debugging (less frequently)
        if (self.time * 60.0) as i32 % 60 == 0 {
            println!("Camera at ({:.1}, {:.1}, {:.1}) looking at origin, time: {:.2}", 
                     camera_x, camera_y, camera_z, self.time);
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
            // ADD SLOW ROTATION: Rotate the cube slowly around Y axis to test for artifacts
            let rotation_y = self.time * 0.5; // Slow rotation (0.5 radians per second)
            let model_matrix = Mat4::rotation_y(rotation_y);
            
            // DEBUG: Print matrix values occasionally
            if (self.time * 60.0) as i32 % 120 == 0 {
                println!("Model rotation: {:.2} radians", rotation_y);
                println!("Camera view matrix: {:?}", self.camera.get_view_matrix().m[0]);
                println!("Camera projection matrix: {:?}", self.camera.get_projection_matrix().m[0]);
            }
            
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
