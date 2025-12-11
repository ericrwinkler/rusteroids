//! # Rendering System
//!
//! This module provides the core rendering abstraction layer for the Rusteroids engine.
//! It acts as a high-level, application-agnostic interface over the Vulkan rendering backend.
//!
//! ## Architecture
//!
//! The rendering system is designed with clear separation of concerns:
//! - **Renderer**: High-level rendering coordinator and API facade
//! - **Camera**: 3D perspective camera with mathematically correct projection matrices
//! - **Vulkan Backend**: Low-level graphics API implementation
//! - **Resource Types**: Meshes, materials, and lighting abstractions
//!
//! ## Design Goals
//!
//! - **API Agnostic**: High-level interface that could theoretically support other backends
//! - **Library-First**: Usable as a standalone rendering library, not tied to specific applications
//! - **Performance**: Efficient resource management with minimal overhead
//! - **Standards Compliance**: Follows academic Vulkan projection matrix guidelines
//!
//! ## Current Limitations (FIXME)
//!
//! - **Backend Coupling**: Some application-specific logic leaks through the abstraction
//! - **Push Constant Limitations**: Complex rendering features constrained by 160-byte limit

// Public modules for application use
pub mod window;
pub mod api;

// Core primitives
pub mod primitives;

// Resources
pub mod resources;

// Systems
pub mod systems;

/// Graphics backend implementations
/// 
/// Contains platform-specific rendering backend implementations.
/// Currently supports Vulkan with potential for additional backends in the future.
pub mod backends;

// High-level APIs that applications should use
pub use api::{VulkanRendererConfig, RenderBackend, WindowBackendAccess, BackendResult, MeshHandle, MaterialHandle, ObjectResourceHandle, RenderFrameData};
pub use resources::materials::ShaderConfig;
pub use window::WindowHandle;
pub use primitives::Camera;
pub use systems::lighting::MultiLightEnvironment;

// Core rendering types that applications need
pub use primitives::{Mesh, Vertex, CoordinateSystem, CoordinateConverter};
pub use resources::materials::{Material, MaterialType, MaterialId, PipelineType, AlphaMode};
pub use systems::lighting::{Light, LightType, LightingEnvironment};
pub use systems::dynamic::{MeshType, DynamicObjectHandle};
pub use systems::text::{
    FontAtlas, GlyphInfo, TextLayout, TextVertex, TextBounds, text_vertices_to_mesh,
    create_lit_text_material, create_unlit_text_material,
    WorldTextManager, WorldTextHandle, WorldTextConfig,
};

// DEPRECATED: These should be removed in favor of cleaner APIs
// TODO: Remove these exports once applications migrate to new APIs
//
// Current overlapping patterns that should be consolidated:
// - MaterialManager + TextureManager: Should be unified into ResourceManager
// - SharedRenderingResources: Should be internal to backend, not exposed
//   * Currently used internally by create_mesh_pool() for dynamic object system
//   * Applications don't use this directly (asteroids codebase has zero usages)
//   * Can be made private once create_mesh_pool() is refactored to use MeshHandle API
// - BatchRenderer: Should be integrated into core renderer, not separate
//
// Migration Strategy:
// 1. Refactor create_mesh_pool() to use backend.create_mesh_resource() → MeshHandle
//    instead of create_instanced_rendering_resources() → SharedRenderingResources
// 2. Make SharedRenderingResources pub(crate) only (internal implementation detail)
// 3. Consider whether MaterialManager/TextureManager need public exposure or should
//    be accessed only through GraphicsEngine methods (upload_texture_from_image_data, etc.)
// 4. Evaluate if BatchRenderer is still needed or if dynamic pool system supersedes it
//
// Status: Not yet safe to remove - actively used by internal dynamic rendering system
pub use resources::materials::{
    StandardMaterialParams, UnlitMaterialParams,
    MaterialManager, TextureManager, MaterialTextures, TextureHandle, TextureType
};
pub use resources::pipelines::{
    PipelineManager, PipelineConfig, CullMode,
    // Future rendering attributes (currently stubbed with FIXME)
    BlendMode, RenderPriority, PolygonMode, DepthBiasConfig,
};
pub use resources::shared::{RenderQueue, RenderCommand, CommandType, RenderCommandId};
pub use resources::shared::ObjectUBO;  // Legitimate rendering data structure
// SharedRenderingResources is internal only (used by dynamic rendering system)
pub(crate) use resources::shared::SharedRenderingResources;
pub use systems::batching::{BatchRenderer, RenderBatch, BatchError, BatchStats, BatchResult};

use thiserror::Error;
// REMOVED: Engine coupling for cleaner library abstraction
// use crate::engine::{RendererConfig, WindowConfig}; 
use crate::foundation::math::{Vec3, Mat4, Mat4Ext};
use crate::render::systems::dynamic::MeshPoolManager;
use crate::ecs::Entity;
use std::collections::HashMap;

/// # Graphics Engine
///
/// High-level graphics coordinator that provides an application-friendly interface
/// over the Vulkan rendering backend. Manages the rendering state, camera, lighting,
/// and coordinates frame rendering operations.
///
/// ## Responsibilities
///
/// - **Graphics Coordination**: Orchestrates frame rendering with proper sequencing
/// - **State Management**: Tracks camera, lighting, and per-frame state
/// - **Resource Binding**: Connects high-level resources (meshes, materials) to GPU
/// - **Backend Abstraction**: Hides Vulkan complexity behind clean API
///
/// ## Design Notes
///
/// The GraphicsEngine acts as a facade pattern implementation, providing a simplified
/// interface over the complex Vulkan rendering pipeline. It maintains minimal
/// state and delegates actual rendering work to the Vulkan backend.
pub struct GraphicsEngine {
    /// Backend abstraction for graphics rendering
    /// FIXME: Should support dynamic backend selection at runtime
    backend: Box<dyn RenderBackend>,
    
    /// Currently active camera for 3D rendering
    /// FIXME: Should support multiple cameras for multi-viewport rendering
    current_camera: Option<Camera>,
    
    /// Current lighting environment
    /// FIXME: Should be integrated with scene management system
    current_lighting: Option<LightingEnvironment>,
    
    /// Dynamic object pool management system for temporary objects
    /// Manages multiple single-mesh pools for optimal rendering performance
    pool_manager: Option<MeshPoolManager>,
    
    /// Entity to pool handle tracking (automatic resource management)
    /// Maps ECS entities to their (MeshType, DynamicObjectHandle) for automatic lifecycle
    entity_handles: HashMap<Entity, (systems::dynamic::MeshType, systems::dynamic::DynamicObjectHandle)>,
    
}

impl GraphicsEngine {
    /// Create a new renderer from a window handle with configuration
    ///
    /// This is the preferred construction method as it maintains clear ownership
    /// of the window resource and allows applications to configure the renderer.
    ///
    /// # Arguments
    /// * `window_handle` - Mutable reference to window handle for Vulkan surface creation
    /// * `config` - Renderer configuration with application-specific settings
    ///
    /// # Design Notes
    /// This method represents good separation of concerns - it only requires
    /// what's necessary (a window and config) and doesn't depend on engine-specific configs.
    pub fn new_from_window(window_handle: &mut WindowHandle, config: &VulkanRendererConfig) -> RenderResult<Self> {
        log::info!("Initializing Vulkan renderer for '{}'", config.application_name);
        
        // Create Vulkan renderer using backend access
        // IMPLEMENTATION NOTE: This uses internal backend access with type-safe downcasting
        // rather than exposing Vulkan types through the public WindowHandle API
        
        // Safe downcast to Vulkan window using the RenderSurface trait
        let render_surface = window_handle.render_surface();
        let vulkan_window = render_surface.as_any_mut()
            .downcast_mut::<crate::render::backends::vulkan::Window>()
            .expect("Expected Vulkan window backend");
        
        let vulkan_renderer = crate::render::backends::vulkan::VulkanRenderer::new(vulkan_window, config)
            .map_err(|e| RenderError::InitializationFailed(format!("Failed to create Vulkan renderer: {:?}", e)))?;
        
        Ok(Self {
            backend: Box::new(vulkan_renderer),
            current_camera: None,
            current_lighting: None,
            pool_manager: None, // Will be initialized when first needed
            entity_handles: HashMap::new(),
        })
    }
    
    /// Initialize UI text rendering system
    ///
    /// Loads fonts, creates font atlas, uploads to GPU, and initializes text rendering pipeline.
    /// Should be called once during application initialization if text rendering is needed.
    pub fn initialize_ui_text_system(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Downcast to VulkanRenderer to call the initialization method
        if let Some(vulkan_backend) = self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
            vulkan_backend.initialize_ui_text_system_internal()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            Ok(())
        } else {
            Err("Backend doesn't support UI text rendering".into())
        }
    }
    
    // ========================================================================
    // Proposal #6: Simplified Frame Rendering API
    // ========================================================================
    
    /// Render a complete frame (Proposal #6 - main rendering API)
    ///
    /// This is the primary rendering entry point. Application provides all frame data,
    /// and the renderer handles the complete frame lifecycle internally:
    /// 1. Begin frame (acquire swapchain, start render pass)
    /// 2. Bind per-frame resources (camera, lights)
    /// 3. Draw 3D scene
    /// 4. Draw UI overlay
    /// 5. End frame (submit commands, present)
    ///
    /// # Arguments
    /// * `data` - Complete frame rendering data (camera, lights, UI)
    /// * `window` - Window handle for presentation
    ///
    /// # Example
    /// ```no_run
    /// # use rust_engine::render::{GraphicsEngine, RenderFrameData, Camera, MultiLightEnvironment, UIRenderData};
    /// # let mut graphics_engine: GraphicsEngine = unimplemented!();
    /// # let mut window = unimplemented!();
    /// # let camera: Camera = unimplemented!();
    /// # let lights: MultiLightEnvironment = unimplemented!();
    /// # let ui_data = UIRenderData::empty();
    /// graphics_engine.render_frame(
    ///     &RenderFrameData {
    ///         camera: &camera,
    ///         lights: &lights,
    ///         ui: &ui_data,
    ///     },
    ///     &mut window,
    /// )?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Book Reference
    /// Game Engine Architecture Chapter 8.2 - The Game Loop
    /// > "The game loop services each subsystem in turn. The rendering engine should 
    /// > have its own internal lifecycle—game code calls render_frame(scene) once per 
    /// > iteration, and the renderer handles setup, draw calls, and presentation internally."
    pub fn render_frame(
        &mut self,
        data: &api::RenderFrameData,
        window: &mut WindowHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // 1. Begin frame (internal)
        self.begin_dynamic_frame()?;
        
        // 2. Bind per-frame resources (camera + lights)
        self.set_camera(data.camera);
        self.set_multi_light_environment(data.lights);
        
        // 3. Draw 3D scene (ECS entities rendered via Scene Manager)
        // Note: Currently handled by record_dynamic_draws() which processes
        // entities that were allocated via allocate_from_pool()
        self.record_dynamic_draws()?;
        
        // 4. Draw UI overlay
        self.draw_ui_internal(data.ui)?;
        
        // 5. End frame and present
        self.end_dynamic_frame(window)?;
        
        Ok(())
    }
    
    /// Internal: Draw UI overlay from backend-agnostic render data
    /// 
    /// Converts UIRenderData (quads, text) into vertex buffers and
    /// submits to the Vulkan backend. Called internally by render_frame().
    fn draw_ui_internal(&mut self, ui_data: &systems::ui::UIRenderData) -> Result<(), Box<dyn std::error::Error>> {
        // Get Vulkan backend (for now, we need direct access)
        // TODO: Abstract this into RenderBackend trait when adding other backends
        let vulkan_renderer = match self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
            Some(renderer) => renderer,
            None => return Err("UI rendering requires Vulkan backend".into()),
        };
        
        // Delegate to UI rendering bridge
        systems::ui::renderer::render_ui_data(ui_data, vulkan_renderer)
    }
    
    /// Update mouse position for UI input processingsingon
    pub fn update_ui_screen_size(&mut self, width: f32, height: f32) {
        if let Some(vulkan_backend) = self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
            vulkan_backend.update_ui_screen_size(width, height);
        }
    }
    
    /// Dispatch all pending UI eventsulkan renderer (for UI rendering)
    /// 
    /// Returns None if the backend is not Vulkan
    pub fn get_vulkan_renderer_mut(&mut self) -> Option<&mut crate::render::backends::vulkan::VulkanRenderer> {
        self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>()
    }
    
    /// Begin a new frame
    ///
    /// Prepares the renderer for a new frame of rendering. Currently this is
    /// a minimal operation that primarily handles frame counting for debugging.
    ///
    /// FIXME: This method doesn't actually do meaningful setup work. The real
    /// frame setup happens inside draw_mesh_3d(). This should either be removed
    /// or expanded to handle proper per-frame setup like updating uniform buffers.
    pub fn begin_frame(&mut self) {
        // FIXME: Frame setup should be more comprehensive
        // Should handle: UBO updates, command buffer reset, sync object management
        log::trace!("Begin frame");
    }
    
    /// Set the active camera for 3D rendering
    ///
    /// Configures the camera that will be used for subsequent 3D rendering operations.
    /// The camera defines the view and projection transformations for the scene.
    ///
    /// # Arguments
    /// * `camera` - Reference to camera configuration
    ///
    /// # Design Notes
    /// Currently stores a full clone of the camera data. This is inefficient for
    /// frequently changing cameras, but acceptable for current usage patterns.
    ///
    /// FIXME: Consider using camera handles/IDs for better performance and to
    /// support multiple active cameras for multi-viewport rendering.
    pub fn set_camera(&mut self, camera: &Camera) {
        self.current_camera = Some(camera.clone()); // FIXME: Cloning entire camera - inefficient
        log::trace!("Camera set: pos={:?}, target={:?}", camera.position, camera.target);
    }
    
    /// Set the lighting environment for the scene
    ///
    /// Configures the lighting that will be applied to rendered objects.
    /// Currently limited to a single directional light due to push constant constraints.
    ///
    /// # Arguments
    /// * `lighting` - The lighting environment containing lights and ambient settings
    ///
    /// # Current Limitations
    /// - Only supports one directional light (first in the array)
    /// - Other light types are ignored with warnings
    /// - Limited by 160-byte push constant size constraint
    ///
    /// # Future Architecture
    /// This method demonstrates a clear architectural limitation where the high-level
    /// renderer is constrained by low-level implementation details (push constants).
    /// The proper solution is uniform buffer objects (UBOs) for lighting data.
    pub fn set_lighting(&mut self, lighting: &LightingEnvironment) {
        self.current_lighting = Some(lighting.clone()); // FIXME: Another full clone - inefficient
        
        // FIXME: This logic couples high-level renderer to low-level push constant limitations
        // The renderer should abstract this complexity, not expose it to the application
        
        // Update the Vulkan renderer with lighting data
        // NOTE: Current architecture uses UBOs for lighting (proper approach)
        // Push constants are reserved for frequently changing per-draw data (transforms)
        if let Some(first_light) = lighting.lights.first() {
            match first_light.light_type {
                LightType::Directional => {
                    // Use helper methods for array conversion
                    self.backend.set_directional_light(
                        first_light.direction_array(),
                        first_light.intensity,
                        first_light.color_array(),
                        lighting.ambient_intensity,
                    );
                },
                _ => {
                    // FIXME: Should support all light types, not just directional
                    log::warn!("Only directional lights supported in current UBO implementation. Found: {:?}", first_light.light_type);
                    log::info!("To use multiple lights, need to implement uniform buffer lighting system");
                }
            }
        } else {
            // FIXME: Hardcoded default lighting - should be configurable
            self.backend.set_directional_light(
                [0.0, 1.0, 0.0],  // Default light coming from below (positive Y direction in Y-up world space)
                0.3,              // Mild default intensity
                [1.0, 1.0, 1.0],  // White light
                lighting.ambient_intensity,
            );
            log::trace!("No directional lights found, using default downward light");
        }
        
        // FIXME: These warnings show that the API contract is broken - the renderer
        // accepts lighting environments but can't actually use them fully
        if lighting.lights.len() > 1 {
            log::warn!("LightingEnvironment contains {} lights, but only the first directional light is used", lighting.lights.len());
            log::info!("Multiple light support requires expanding UBO system and shader updates");
        }
        
        log::trace!("Lighting set with {} lights", lighting.lights.len());
    }
    
    /// Set multi-light environment using UBO-based lighting
    /// 
    /// This method uses uniform buffer objects for lighting data (industry best practice),
    /// while push constants handle per-draw transform data for optimal performance.
    /// enabling support for multiple lights while maintaining identical visual output.
    pub fn set_multi_light_environment(&mut self, multi_light_env: &systems::lighting::MultiLightEnvironment) {
        self.backend.set_multi_light_environment(multi_light_env);
    }
    
    /// Render a 3D mesh with transformation and material
    ///
    /// Primary rendering method for 3D objects. Draws a mesh using the currently
    /// configured camera and lighting with the specified transformation and material.
    ///
    /// # Arguments
    /// * `mesh` - The mesh data (vertices and indices)
    /// * `model_matrix` - Transformation matrix from object space to world space
    /// * `material` - Material properties (color, textures, shading parameters)
    ///
    /// # Matrix Mathematics
    /// Implements Johannes Unterguggenberger's mathematically correct approach:
    /// Clip = P × X × V × M × Vertex where:
    /// - P = Projection matrix (perspective/orthographic transformation)
    /// - X = Vulkan coordinate transform (Y-up view space → Vulkan conventions)
    /// - V = View matrix (world space → camera/view space)
    /// - M = Model matrix (object space → world space)
    ///
    /// # Architecture Violations
    /// This method violates several library-agnostic design principles:
    /// 1. **Vulkan Coupling**: Direct calls to vulkan_renderer methods
    /// 2. **Matrix Format Coupling**: Manual nalgebra → array conversion
    /// 3. **Error Type Leakage**: Exposes Vulkan-specific error types
    /// 4. **Immediate Mode**: Updates GPU state on every call (inefficient)
    ///
    /// # Performance Issues
    /// - Mesh data updated on every draw call (should be cached)
    /// - Matrix calculations repeated for identical transformations
    /// - Lighting set per mesh instead of per frame
    /// - No batching for objects with similar materials
    ///
    /// FIXME: This method does too much and should be decomposed into:
    /// - update_mesh_data() - GPU resource management
    /// - set_transform_matrices() - Matrix calculations

    // ================================
    // NEW COMMAND RECORDING API (PROTOTYPE)
    // ================================
    
    /// Begin command recording for multiple objects (NEW API)
    /// This replaces the immediate-mode pattern with proper command recording
    pub fn begin_command_recording(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Begin proper render pass for multiple objects
        self.backend.begin_render_pass()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Set up per-frame uniforms (camera, lighting) once
        self.setup_frame_uniforms()?;
        
        Ok(())
    }
    
    /// Set up per-frame uniforms (camera, lighting) - called once per frame
    fn setup_frame_uniforms(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Set per-frame uniforms first
        if let Some(ref camera) = self.current_camera {
            let view_matrix = camera.get_view_matrix();
            let coord_transform = Mat4::vulkan_coordinate_transform();
            let projection_matrix = camera.get_projection_matrix();

            // Update camera UBO using existing backend method
            let view_projection = projection_matrix * coord_transform * view_matrix;
            self.backend.update_camera_ubo(view_matrix, projection_matrix, view_projection, camera.position);
        }

        // Set lighting environment using existing method
        if let Some(ref lighting) = self.current_lighting {
            let multi_light_env = self.convert_lighting_to_multi_light_environment(lighting);
            self.backend.set_multi_light_environment(&multi_light_env);
        }
        
        Ok(())
    }
    
    // === NEW CLEAN API - Use these methods instead ===
    
    /// Load a mesh and return an opaque handle for rendering
    /// This replaces create_shared_rendering_resources with a cleaner abstraction
    pub fn load_mesh(&mut self, mesh: &Mesh, materials: &[Material]) -> Result<api::MeshHandle, Box<dyn std::error::Error>> {
        let mesh_handle = self.backend.create_mesh_resource(mesh, materials)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        log::info!("Loaded mesh with handle {:?}", mesh_handle);
        Ok(mesh_handle)
    }
    
    /// Render multiple objects with the same mesh using transforms
    /// This replaces the complex bind_shared_geometry + record_object_draw pattern
    pub fn render_objects(&mut self, mesh_handle: api::MeshHandle, transforms: &[Mat4]) -> Result<(), Box<dyn std::error::Error>> {
        self.backend.render_objects_with_mesh(mesh_handle, transforms)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        log::trace!("Rendered {} objects with mesh handle {:?}", transforms.len(), mesh_handle);
        Ok(())
    }
    
    /// Submit all recorded commands and present (NEW API)
    /// This replaces individual draw_frame() calls with batched submission
    pub fn submit_commands_and_present(&mut self, _window: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        // End render pass and submit all recorded draw commands
        self.backend.end_render_pass_and_submit()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        Ok(())
    }
    
    /// Create instanced pipeline for dynamic object rendering
    fn create_instanced_pipeline(&self, context: &crate::render::backends::vulkan::VulkanContext, frame_layout: ash::vk::DescriptorSetLayout, material_layout: ash::vk::DescriptorSetLayout) -> Result<(crate::render::backends::vulkan::GraphicsPipeline, ash::vk::PipelineLayout), Box<dyn std::error::Error>> {
        // Get render pass from VulkanRenderer
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::backends::vulkan::VulkanRenderer>() {
            let render_pass = vulkan_backend.get_render_pass();
            
            // Delegate to pipeline builder module
            systems::dynamic::instancing::create_instanced_pipeline(
                context,
                frame_layout,
                material_layout,
                render_pass,
            )
        } else {
            Err("Backend is not Vulkan".into())
        }
    }
    
    /// Create SharedRenderingResources for instanced dynamic object rendering (INTERNAL)
    /// This creates resources with an instanced pipeline for efficient batch rendering
    /// Used internally by create_mesh_pool()
    fn create_instanced_rendering_resources(&self, mesh: &Mesh, materials: &[Material]) -> Result<resources::shared::SharedRenderingResources, Box<dyn std::error::Error>> {
        // Access VulkanContext through backend downcasting
        if let Some(vulkan_backend) = self.backend.as_any().downcast_ref::<crate::render::backends::vulkan::VulkanRenderer>() {
            #[allow(deprecated)]
            resources::shared::create_instanced_rendering_resources(
                vulkan_backend,
                mesh,
                materials,
                |context, frame_layout, material_layout| {
                    self.create_instanced_pipeline(context, frame_layout, material_layout)
                },
            )
        } else {
            Err("Renderer is not using Vulkan backend".into())
        }
    }
    
    /// Helper method to convert lighting environment (temporary until lighting refactor)
    fn convert_lighting_to_multi_light_environment(&self, _lighting: &LightingEnvironment) -> systems::lighting::MultiLightEnvironment {
        // Convert from the old LightingEnvironment to the new MultiLightEnvironment
        // For now, create a simple environment with ambient lighting
        // TODO: Implement proper conversion when lighting system is refactored
        systems::lighting::MultiLightEnvironment::new()
    }

    // ================================
    // DYNAMIC OBJECT RENDERING API
    // ================================

    /// Initialize the dynamic object pool system
    ///
    /// Creates the MeshPoolManager and sets up initial pools for common mesh types.
    /// This must be called before spawning any dynamic objects.
    ///
    /// # Arguments
    /// * `max_objects_per_pool` - Maximum objects per individual mesh pool
    ///
    /// # Returns
    /// Result indicating successful initialization
    ///
    /// # Examples
    /// ```rust
    /// // Initialize with 100 objects per mesh type
    /// graphics_engine.initialize_dynamic_system(100)?;
    ///
    /// // Now you can spawn objects
    /// let handle = graphics_engine.spawn_dynamic_object(
    ///     MeshType::Teapot,           // specify mesh type
    ///     Vec3::new(0.0, 0.0, 0.0),   // position
    ///     Vec3::zeros(),              // rotation
    ///     Vec3::new(1.0, 1.0, 1.0),   // scale
    ///     MaterialProperties::default(), // material
    ///     5.0                         // lifetime in seconds
    /// )?;
    /// ```
    pub fn initialize_dynamic_system(&mut self, max_objects_per_pool: usize) -> Result<(), Box<dyn std::error::Error>> {
        systems::dynamic::graphics_api::initialize_dynamic_system(&mut self.pool_manager, &*self.backend, max_objects_per_pool)?;
        
        // Initialize backend's dynamic rendering system
        self.backend.initialize_dynamic_rendering(max_objects_per_pool)?;
        
        Ok(())
    }

    /// Create a mesh pool for a specific mesh type (MeshPoolManager system)
    ///
    /// This method creates a pool for a specific mesh type using the MeshPoolManager.
    /// Must be called after initialize_dynamic_system and before spawning objects of this type.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh to create a pool for (Teapot, Sphere, Cube)
    /// * `mesh` - The mesh data to create SharedRenderingResources from
    /// * `materials` - Array of materials for this mesh type
    /// * `max_objects` - Maximum number of objects this pool can hold
    ///
    /// # Returns
    /// Result indicating successful pool creation or error
    pub fn create_mesh_pool(
        &mut self,
        mesh_type: crate::render::systems::dynamic::MeshType,
        mesh: &Mesh,
        materials: &[Material],
        max_objects: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create SharedRenderingResources first (before borrowing pool_manager mutably)
        #[allow(deprecated)]
        let shared_resources = self.create_instanced_rendering_resources(mesh, materials)?;
        
        // Extract textures from first material
        let base_color_texture = materials.get(0)
            .and_then(|mat| mat.textures.base_color);
        let emission_texture = materials.get(0)
            .and_then(|mat| mat.textures.emission);
        
        log::info!("Creating mesh pool for {:?} with capacity {}, base_color: {:?}, emission: {:?}", 
                   mesh_type, max_objects, base_color_texture, emission_texture);
        
        // Now delegate to graphics_api with pre-created resources
        if let Some(ref mut mgr) = self.pool_manager {
            if let Some(vulkan_backend) = self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
                // Create descriptor set based on available textures
                let material_descriptor_set = match (base_color_texture, emission_texture) {
                    (Some(base_tex), Some(emission_tex)) => {
                        log::info!("Creating descriptor set with base_color {:?} and emission {:?}", base_tex, emission_tex);
                        vulkan_backend.create_material_descriptor_set_with_textures(base_tex, emission_tex)?
                    }
                    (Some(base_tex), None) => {
                        log::info!("Creating descriptor set with base_color {:?}", base_tex);
                        vulkan_backend.create_material_descriptor_set_with_texture(base_tex)?
                    }
                    _ => {
                        log::info!("No textures, using default descriptor set");
                        vulkan_backend.get_default_material_descriptor_set()
                    }
                };
                
                let context = vulkan_backend.get_context();
                
                mgr.create_pool(
                    mesh_type,
                    shared_resources,
                    max_objects,
                    context,
                    material_descriptor_set,
                    base_color_texture,
                )?;
                log::info!("Successfully created pool for {:?}", mesh_type);
                Ok(())
            } else {
                Err("Backend is not a VulkanRenderer".into())
            }
        } else {
            Err("Dynamic system not initialized".into())
        }
    }
    
    /// Register a mesh type for multi-mesh rendering (Phase 0)
    ///
    /// This method registers a mesh type with its associated SharedRenderingResources
    /// in the MeshRegistry. This must be called for each mesh type that will be used
    /// with dynamic objects before spawning objects of that type.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh to register (Teapot, Sphere, Cube)
    /// * `mesh` - The mesh data to create SharedRenderingResources from
    /// * `materials` - Array of materials for this mesh type
    ///
    /// # Returns
    /// Result indicating successful registration or error
    ///
    /// # Example
    /// ```rust
    /// Spawn a dynamic object with specified parameters
    ///
    /// Creates a new dynamic object using the pooled resource system. Objects are
    /// automatically managed by the pool and will be despawned when their lifetime expires.
    /// This is the high-level API for creating temporary objects.
    ///
    /// # Arguments
    /// * `position` - World space position of the object
    /// * `rotation` - Rotation angles in radians (Euler angles: X, Y, Z)
    /// * `scale` - Scale factors for each axis
    /// * `material` - Material properties for rendering
    /// * `lifetime` - Object lifetime in seconds (0.0 for infinite)
    ///
    /// # Returns
    /// Handle to the spawned object for safe access, or SpawnerError on failure
    ///
    /// # Error Conditions
    /// - Pool exhaustion: No more objects available in the pool
    /// - Invalid parameters: Position/scale contains NaN or infinite values
    /// - System not initialized: initialize_dynamic_system() not called
    ///
    /// Begin dynamic frame rendering setup
    ///
    /// Prepares the dynamic rendering system for a new frame. This should be called
    /// once per frame before any dynamic object operations.
    ///
    /// # Returns
    /// Result indicating successful frame setup
    ///  
    /// # Frame Workflow
    /// The proper dynamic rendering workflow is:
    /// 1. `begin_dynamic_frame()` - Setup command recording
    /// 2. Update entity transforms in ECS
    /// 3. `record_dynamic_draws()` - Record drawing commands
    /// 4. `end_dynamic_frame()` - Submit and present
    ///
    /// # Example
    /// ```rust
    /// // At start of render loop
    /// graphics_engine.begin_dynamic_frame()?;
    /// 
    /// // ECS systems update entity transforms
    /// 
    /// // Record and submit rendering
    /// graphics_engine.record_dynamic_draws()?;
    /// graphics_engine.end_dynamic_frame(window)?;
    /// ```
    pub fn begin_dynamic_frame(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        
        // Process cleanup of manually despawned objects
        systems::dynamic::graphics_api::begin_dynamic_frame(&mut self.pool_manager);
        
        // Begin command recording for the frame (same as static objects)
        self.begin_command_recording()?;
        
        Ok(())
    }

    /// Allocate a dynamic instance from a mesh pool
    ///
    /// Allocates GPU resources for a dynamic object. The application must compute
    /// the transform matrix from position/rotation/scale before calling this method.
    /// This enforces separation of concerns: rendering manages GPU resources,
    /// while application/ECS manages transform logic.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh to spawn (Teapot, Sphere, Cube, etc.)
    /// * `transform` - Pre-computed model-to-world transformation matrix
    /// * `material` - Material properties for rendering
    ///
    /// # Returns
    /// Handle to the allocated pool instance, or error
    ///
    /// # Example
    /// ```no_run
    /// use rust_engine::foundation::math::{Vec3, Mat4};
    /// 
    /// // Application computes transform
    /// let position = Vec3::new(1.0, 2.0, 3.0);
    /// let rotation = Vec3::new(0.0, 0.5, 0.0);
    /// let scale = Vec3::new(1.0, 1.0, 1.0);
    /// let transform = Mat4::new_translation(&position)
    ///     * Mat4::from_euler_angles(rotation.x, rotation.y, rotation.z)
    ///     * Mat4::new_nonuniform_scaling(&scale);
    /// 
    /// // Renderer just allocates GPU resources
    /// let handle = graphics_engine.allocate_from_pool(mesh_type, transform, material)?;
    /// ```
    pub fn allocate_from_pool(
        &mut self,
        mesh_type: systems::dynamic::MeshType,
        transform: crate::foundation::math::Mat4,
        material: resources::materials::Material,
    ) -> Result<systems::dynamic::DynamicObjectHandle, Box<dyn std::error::Error>> {
        systems::dynamic::graphics_api::allocate_from_pool(
            &mut self.pool_manager,
            mesh_type,
            transform,
            material,
        ).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Free a pool instance
    ///
    /// Returns the allocated GPU resources back to the pool for reuse.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh (determines which pool to free from)
    /// * `handle` - Handle to the pool instance to free
    pub fn free_pool_instance(
        &mut self,
        mesh_type: systems::dynamic::MeshType,
        handle: systems::dynamic::DynamicObjectHandle,
    ) -> Result<(), Box<dyn std::error::Error>> {
        systems::dynamic::graphics_api::free_pool_instance(
            &mut self.pool_manager,
            mesh_type,
            handle,
        ).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Update the transform of a pool instance
    ///
    /// Updates the transformation matrix of an existing pool instance.
    /// Application must compute the new transform matrix.
    ///
    /// # Arguments
    /// * `mesh_type` - The type of mesh (determines which pool to search)
    /// * `handle` - Handle to the pool instance to update
    /// * `transform` - New pre-computed model-to-world transformation matrix
    pub fn update_pool_instance(
        &mut self,
        mesh_type: systems::dynamic::MeshType,
        handle: systems::dynamic::DynamicObjectHandle,
        transform: crate::foundation::math::Mat4,
    ) -> Result<(), Box<dyn std::error::Error>> {
        systems::dynamic::graphics_api::update_pool_instance(
            &mut self.pool_manager,
            mesh_type,
            handle,
            transform,
        ).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    /// Render entities from a Scene Manager's RenderQueue (Proposal #1 - Complete API)
    ///
    /// This method automatically manages entity lifecycle in the rendering system:
    /// - New entities: Allocates pool handles based on mesh_type stored in RenderableObject
    /// - Existing entities: Updates transforms only (efficient)
    /// - Destroyed entities: Automatically freed when no longer in queue
    ///
    /// Applications no longer need to manually track entity→handle mappings OR determine mesh types.
    ///
    /// # Arguments
    /// * `render_queue` - Queue of renderable objects from SceneManager
    ///
    /// # Example
    /// ```ignore
    /// scene_manager.sync_from_world(&mut world);
    /// let render_queue = scene_manager.build_render_queue();
    /// 
    /// // That's it! Engine handles everything automatically.
    /// graphics_engine.render_entities_from_queue(&render_queue)?;
    /// ```
    pub fn render_entities_from_queue(
        &mut self,
        render_queue: &crate::scene::RenderQueue,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::collections::HashSet;
        
        // Track which entities are still active this frame
        let mut active_entities = HashSet::new();
        
        // Process all renderable objects in the queue
        for batch in render_queue.opaque_batches() {
            for obj in &batch.objects {
                active_entities.insert(obj.entity);
                
                // Check if we already have a handle for this entity
                if let Some(&(mesh_type, handle)) = self.entity_handles.get(&obj.entity) {
                    // Existing entity - just update its transform
                    if let Err(e) = self.update_pool_instance(mesh_type, handle, obj.transform) {
                        log::warn!("Failed to update entity {:?} transform: {}", obj.entity, e);
                    }
                } else {
                    // New entity - use the mesh_type stored in RenderableObject
                    let mesh_type = obj.mesh_type;
                    
                    // Allocate a handle for this entity
                    match self.allocate_from_pool(
                        mesh_type,
                        obj.transform,
                        obj.material.clone(),
                    ) {
                        Ok(handle) => {
                            self.entity_handles.insert(obj.entity, (mesh_type, handle));
                            log::debug!("Allocated {:?} handle for entity {:?}", mesh_type, obj.entity);
                        }
                        Err(e) => {
                            log::warn!("Failed to allocate {:?} for entity {:?}: {}", mesh_type, obj.entity, e);
                        }
                    }
                }
            }
        }
        
        // Free handles for entities that no longer exist
        let entities_to_remove: Vec<_> = self.entity_handles.keys()
            .filter(|entity| !active_entities.contains(entity))
            .copied()
            .collect();
        
        for entity in entities_to_remove {
            if let Some((mesh_type, handle)) = self.entity_handles.remove(&entity) {
                if let Err(e) = self.free_pool_instance(mesh_type, handle) {
                    log::warn!("Failed to free handle for destroyed entity {:?}: {}", entity, e);
                }
                log::debug!("Freed {:?} handle for destroyed entity {:?}", mesh_type, entity);
            }
        }
        
        Ok(())
    }

    /// Record dynamic object draw commands
    ///
    /// Records all active dynamic objects into the command buffer using instanced
    /// rendering for optimal performance. This batches all dynamic objects by
    /// mesh type into minimal draw calls.
    ///
    /// # Returns
    /// Result indicating successful command recording
    ///
    /// # Performance Notes
    /// This method uses instanced rendering to draw all active dynamic objects
    /// in as few draw calls as possible (one per mesh type). It's much more efficient 
    /// than individual draw calls.
    ///
    /// # Prerequisites
    /// - Pool system must be initialized
    /// - begin_dynamic_frame() must be called first
    /// - Camera and lighting should be set up
    pub fn record_dynamic_draws(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Get camera position and target for depth sorting
        let (camera_position, camera_target) = self.current_camera.as_ref()
            .map(|cam| (cam.position, cam.target))
            .unwrap_or((Vec3::zeros(), Vec3::new(0.0, 0.0, -1.0)));
        
        systems::dynamic::graphics_api::record_dynamic_draws(
            &mut self.pool_manager,
            &mut *self.backend,
            camera_position,
            camera_target,
        )
    }

    /// End dynamic frame and submit commands
    ///
    /// Finalizes the dynamic rendering frame by submitting all recorded commands
    /// and presenting the frame. This completes the dynamic rendering workflow.
    ///
    /// # Arguments
    /// * `window` - Window handle for presentation
    ///
    /// # Returns
    /// Result indicating successful frame completion
    ///
    /// # Frame Completion
    /// This method handles:
    /// - Command buffer submission
    /// - Frame presentation  
    /// - Frame counter increment
    /// - Resource state updates
    pub fn end_dynamic_frame(&mut self, window: &mut WindowHandle) -> Result<(), Box<dyn std::error::Error>> {
        // Submit commands and present (same as static objects)
        self.submit_commands_and_present(window)?;
        
        log::trace!("Dynamic frame completed");
        Ok(())
    }

    /// Record UI overlay rendering commands (Chapter 11.4.4)
    ///
    /// **CRITICAL ARCHITECTURAL IMPLEMENTATION:**
    /// This method implements the UI overlay rendering strategy described in Chapter 11.4.4:
    /// "Overlays are generally rendered after the primary scene, with z-testing disabled to
    /// ensure that they appear on top of the three-dimensional scene."
    ///
    /// This method MUST be called AFTER `record_dynamic_draws()` and BEFORE `end_dynamic_frame()`.
    /// The render pass remains OPEN during this call - we switch pipeline state but stay within
    /// the same render pass (single render pass approach per the book's guidance).
    ///
    /// Get dynamic system statistics
    ///
    /// Returns statistics about the dynamic object system including pool utilization,
    /// active object count, and performance metrics. Useful for debugging and
    /// performance monitoring.
    ///
    /// # Returns
    /// Option containing spawner statistics, or None if system not initialized
    ///
    /// # Statistics Include
    /// - Active object count
    /// - Pool utilization percentage
    /// - Peak usage statistics
    /// - Automatic vs manual despawn counts
    ///
    /// # Example
    /// ```rust
    /// if let Some(stats) = graphics_engine.get_dynamic_stats() {
    ///     println!("Dynamic objects: {}/{} ({}% utilization)", 
    ///              stats.active_objects, 
    ///              stats.pool_capacity,
    ///              (stats.utilization_percentage * 100.0) as u32);
    /// }
    /// ```
    pub fn get_dynamic_stats(&self) -> Option<crate::render::systems::dynamic::PoolManagerStats> {
        systems::dynamic::graphics_api::get_dynamic_stats(&self.pool_manager)
    }

    /// Initialize instanced rendering system for efficient batch rendering
    ///
    /// Sets up the instance renderer for efficient batch rendering of multiple objects
    /// with the same mesh using Vulkan instanced rendering (vkCmdDrawInstanced).
    /// This provides significant performance improvements over individual draw calls.
    ///
    /// # Arguments
    /// * `max_instances` - Maximum number of instances that can be rendered in a single batch
    ///
    /// # Returns
    /// Result indicating successful initialization or error
    ///
    /// # Example
    /// ```rust
    /// // Initialize for up to 100 instances per batch
    /// graphics_engine.initialize_instance_renderer(100)?;
    /// ```
    pub fn initialize_instance_renderer(&mut self, max_instances: usize) -> Result<(), Box<dyn std::error::Error>> {
        // Use the backend trait method for initialization
        self.backend.initialize_instance_renderer(max_instances)?;
        log::info!("Initialized instance renderer with max {} instances", max_instances);
        Ok(())
    }

    // ================================
    // TEXTURE MANAGEMENT API
    // ================================

    /// Upload texture from image data to GPU
    ///
    /// Uploads a texture from raw image data (useful for procedurally generated textures
    /// like font atlases). The texture is uploaded to GPU memory and a handle is returned
    /// for use with materials.
    ///
    /// # Arguments
    /// * `image_data` - Image data (RGBA format)
    /// * `texture_type` - Type of texture (BaseColor, Normal, etc.)
    ///
    /// # Returns
    /// TextureHandle for use with materials, or error on failure
    ///
    /// # Example
    /// ```rust
    /// // Upload font atlas texture
    /// let image_data = ImageData {
    ///     data: atlas_pixels,
    ///     width: 1024,
    ///     height: 1024,
    ///     channels: 4,
    /// };
    /// let texture_handle = graphics_engine.upload_texture_from_image_data(
    ///     image_data,
    ///     TextureType::BaseColor,
    /// )?;
    /// ```
    pub fn upload_texture_from_image_data(
        &mut self,
        image_data: crate::assets::ImageData,
        texture_type: resources::materials::TextureType,
    ) -> Result<resources::materials::TextureHandle, Box<dyn std::error::Error>> {
        // Delegate to backend's texture manager
        if let Some(vulkan_backend) = self.backend.as_any_mut().downcast_mut::<crate::render::backends::vulkan::VulkanRenderer>() {
            let texture_handle = vulkan_backend.upload_texture_from_image_data(image_data, texture_type)?;
            log::debug!("Uploaded texture to GPU: {:?} (type: {:?})", texture_handle, texture_type);
            Ok(texture_handle)
        } else {
            Err("Backend doesn't support texture upload".into())
        }
    }
    
    /// Recreate the Vulkan swapchain for window resizing
    pub fn recreate_swapchain(&mut self, window_handle: &mut WindowHandle) {
        log::info!("Recreating swapchain for window resize");
        
        if let Err(e) = self.backend.recreate_swapchain(window_handle) {
            log::error!("Failed to recreate swapchain: {:?}", e);
        } else {
            log::info!("Successfully recreated swapchain");
        }
    }
    
    /// Get the current swapchain dimensions
    pub fn get_swapchain_extent(&self) -> (u32, u32) {
        self.backend.get_swapchain_extent()
    }
}

/// High-level rendering error types
///
/// Represents errors that can occur during rendering operations, abstracted
/// from specific graphics API error types to maintain library-agnostic design.
///
/// # Design Philosophy
/// These error types provide meaningful information for application developers
/// without exposing Vulkan-specific details. The actual backend errors are
/// logged for debugging purposes but abstracted from the public API.
#[derive(Error, Debug)]
pub enum RenderError {
    /// Renderer initialization failed during setup
    ///
    /// Occurs when the rendering system cannot be properly initialized, typically
    /// due to missing graphics drivers, incompatible hardware, or configuration issues.
    #[error("Renderer initialization failed: {0}")]
    InitializationFailed(String),
    
    /// A rendering operation failed during execution
    ///
    /// Indicates failure during active rendering operations such as drawing meshes,
    /// updating textures, or presenting frames. Often recoverable with retry logic.
    #[error("Rendering failed: {0}")]
    RenderingFailed(String),
    
    /// Resource creation or management failed
    ///
    /// Occurs when GPU resources (buffers, textures, shaders) cannot be created
    /// or managed properly, typically due to memory constraints or invalid data.
    #[error("Resource creation failed: {0}")]
    ResourceCreationFailed(String),
    
    /// Backend-specific error occurred
    ///
    /// Wraps backend-specific errors (Vulkan, OpenGL, etc.) in a generic form
    /// for consistent error handling across different graphics backends.
    #[error("Backend error: {0}")]
    BackendError(String),
}

/// Result type for rendering operations
pub type RenderResult<T> = Result<T, RenderError>;
