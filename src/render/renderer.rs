// src/render/renderer.rs
use crate::render::{
    camera::Camera,
    draw_queue::{DrawQueue, DrawCommand},
    lighting::LightingEnvironment,
    material::Material,
    mesh::Mesh,
    texture::Texture,
    vulkan::VulkanContext,
    window::WindowHandle,
};
use crate::util::math::{Vec3, Mat4};
use ash::vk;

// Uniform buffers for shaders
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CameraUniforms {
    view_matrix: [f32; 16],
    projection_matrix: [f32; 16],
    view_projection_matrix: [f32; 16],
    camera_position: [f32; 3],
    _padding: f32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ObjectUniforms {
    model_matrix: [f32; 16],
    normal_matrix: [f32; 16], // For transforming normals
}

pub struct Renderer {
    vulkan_context: VulkanContext,
    
    // Resource storage
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    textures: Vec<Texture>,
    
    // Rendering
    draw_queue: DrawQueue,
    camera: Camera,
    
    current_frame: usize,
    
    // Track current mesh state
    current_mesh_loaded: bool,
}

impl Renderer {
    pub fn new(window: &mut WindowHandle) -> Self {
        let vulkan_context = VulkanContext::new(window);
        
        // Create default camera
        let camera = Camera::perspective(
            Vec3::new(0.0, 0.0, 3.0),
            45.0,
            1.0, // Will be updated with actual aspect ratio
            0.1,
            100.0,
        );

        Self {
            vulkan_context,
            meshes: Vec::new(),
            materials: Vec::new(),
            textures: Vec::new(),
            draw_queue: DrawQueue::new(),
            camera,
            current_frame: 0,
            current_mesh_loaded: false,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load default resources
        self.load_default_resources()?;
        Ok(())
    }

    pub fn add_mesh(&mut self, mesh: Mesh) -> usize {
        let id = self.meshes.len();
        self.meshes.push(mesh);
        id
    }

    pub fn add_material(&mut self, material: Material) -> usize {
        let id = self.materials.len();
        self.materials.push(material);
        id
    }

    pub fn add_texture(&mut self, texture: Texture) -> usize {
        let id = self.textures.len();
        self.textures.push(texture);
        id
    }

    pub fn get_camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn submit_draw_command(&mut self, command: DrawCommand) {
        if let Some(material_id) = command.material_id {
            if let Some(material) = self.materials.get(material_id) {
                if material.is_transparent() {
                    self.draw_queue.submit_transparent(command);
                } else {
                    self.draw_queue.submit_opaque(command);
                }
            } else {
                self.draw_queue.submit_opaque(command);
            }
        } else {
            self.draw_queue.submit_opaque(command);
        }
    }

    pub fn draw_frame(&mut self, window: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        // Sort draw queue
        self.draw_queue.sort();
        
        // Execute Vulkan rendering
        match self.vulkan_context.draw_frame(window) {
            Ok(()) => {},
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) | Err(ash::vk::Result::SUBOPTIMAL_KHR) => {
                // Recreate swapchain and try again
                self.vulkan_context.recreate_swapchain(window);
                return Ok(()); // Skip this frame
            },
            Err(e) => {
                return Err(format!("Vulkan draw_frame error: {:?}", e).into());
            }
        }
        
        // Clear draw queue for next frame
        self.draw_queue.clear();
        
        self.current_frame = (self.current_frame + 1) % 2; // Assuming double buffering
        
        Ok(())
    }

    pub fn recreate_swapchain(&mut self, window: &mut WindowHandle) {
        self.vulkan_context.recreate_swapchain(window);
    }

    fn load_default_resources(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Add default mesh (cube)
        self.add_mesh(Mesh::cube());
        
        // Add default material
        self.add_material(Material::new());
        
        Ok(())
    }

    pub fn cleanup(&mut self) {
        // Cleanup Vulkan context
        self.vulkan_context.cleanup();
    }
    
    // New methods for integrated app
    pub fn begin_frame(&mut self) {
        // Clear draw queue for new frame
        self.draw_queue.clear();
    }
    
    pub fn end_frame(&mut self, window: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        // Sort and execute draw queue, then present frame
        self.draw_frame(window)
    }
    
    pub fn set_camera(&mut self, camera: &Camera) {
        self.camera = camera.clone();
    }
    
    pub fn set_lighting(&mut self, _lighting_env: &LightingEnvironment) {
        // Store lighting environment for shader uniforms
        // Implementation would upload lighting data to GPU
    }
    
    pub fn draw_mesh_3d(
        &mut self, 
        mesh: &Mesh, 
        model_matrix: &Mat4, 
        _material: &Material
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Only update mesh data if we haven't loaded one yet
        if !self.current_mesh_loaded {
            self.vulkan_context.set_mesh_data(&mesh.vertices, &mesh.indices)?;
            self.current_mesh_loaded = true;
        }
        
        // Compute MVP matrix: Projection * View * Model
        let view_matrix = self.camera.get_view_matrix();
        let projection_matrix = self.camera.get_projection_matrix();
        let mvp_matrix = projection_matrix * view_matrix * *model_matrix;
        
        // Set MVP matrix in Vulkan context
        self.vulkan_context.set_mvp_matrix(&mvp_matrix);
        
        // Create draw command
        let command = DrawCommand {
            mesh_id: 0, // Would need proper mesh management
            material_id: Some(0), // Would need proper material management
            transform_matrix: model_matrix.clone(),
            depth: 0.0, // Calculate proper depth for sorting
        };
        
        // Submit to draw queue
        self.draw_queue.submit_opaque(command);
        
        Ok(())
    }
}
