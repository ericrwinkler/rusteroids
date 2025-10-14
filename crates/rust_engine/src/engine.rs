//! Core engine implementation

use crate::{
    application::{Application, AppError, AppEvent},
    foundation::time::Timer,
    ecs::World,
    assets::AssetManager,
    render::GraphicsEngine,
    input::InputManager,
};
use thiserror::Error;

/// Main engine struct
/// 
/// The engine coordinates all subsystems and manages the main loop.
pub struct Engine {
    /// ECS world containing all entities, components, and systems
    pub world: World,
    
    /// Asset management system
    pub assets: AssetManager,
    
    /// Graphics engine
    pub graphics_engine: GraphicsEngine,
    
    /// Input handling system
    pub input: InputManager,
    
    /// Frame timing
    timer: Timer,
    
    /// Engine configuration
    #[allow(dead_code)] // Will be used for runtime engine configuration
    config: EngineConfig,
    
    /// Whether the engine should continue running
    running: bool,
}

impl Engine {
    /// Create a new engine instance
    pub fn new(config: EngineConfig) -> Result<Self, EngineError> {
        log::info!("Initializing engine...");
        
        // Initialize subsystems
        let world = World::new();
        let assets = AssetManager::new(&config.assets)
            .map_err(|e| EngineError::InitializationFailed(format!("Asset manager: {}", e)))?;
        let graphics_engine = GraphicsEngine::new(&config.renderer, &config.window)
            .map_err(|e| EngineError::InitializationFailed(format!("Graphics engine: {}", e)))?;
        let input = InputManager::new();
        let timer = Timer::new();
        
        Ok(Self {
            world,
            assets,
            graphics_engine,
            input,
            timer,
            config,
            running: true,
        })
    }
    
    /// Run the engine main loop with the given application
    pub fn run<T: Application>(config: EngineConfig, app: &mut T) -> Result<(), EngineError> {
        let mut engine = Self::new(config)?;
        
        // Initialize application
        app.initialize(&mut engine)
            .map_err(|e| EngineError::ApplicationError(format!("App initialization: {}", e)))?;
        
        log::info!("Starting main loop...");
        
        while engine.running {
            let delta_time = engine.timer.delta_time();
            
            // Update application
            app.update(&mut engine, delta_time)
                .map_err(|e| EngineError::ApplicationError(format!("App update: {}", e)))?;
            
            // Update engine systems
            engine.update(delta_time)?;
            
            // Render
            app.render(&mut engine)
                .map_err(|e| EngineError::ApplicationError(format!("App render: {}", e)))?;
        }
        
        // Cleanup
        app.cleanup(&mut engine);
        
        log::info!("Engine shutdown complete");
        Ok(())
    }
    
    /// Update engine systems
    fn update(&mut self, delta_time: f32) -> Result<(), EngineError> {
        // Update subsystems
        self.input.update();
        self.assets.update().map_err(|e| EngineError::AssetError(e.to_string()))?;
        self.world.update(delta_time);
        
        Ok(())
    }
    
    /// Render the current frame
    pub fn render(&mut self) -> Result<(), AppError> {
        self.graphics_engine.render()
            .map_err(|e| AppError::Custom(format!("Render error: {}", e)))
    }
    
    /// Handle an application event
    pub fn handle_event(&mut self, event: AppEvent) -> Result<(), AppError> {
        match event {
            AppEvent::WindowCloseRequested => {
                self.running = false;
            }
            AppEvent::KeyPressed(key) => {
                self.input.handle_key_input(key, true);
            }
            AppEvent::KeyReleased(key) => {
                self.input.handle_key_input(key, false);
            }
            AppEvent::KeyInput { key, pressed } => {
                self.input.handle_key_input(key, pressed);
            }
            AppEvent::MouseButton { button, pressed } => {
                self.input.handle_mouse_button(button, pressed);
            }
            AppEvent::MouseMoved { x, y } => {
                self.input.handle_mouse_move(x, y);
            }
            _ => {
                // Handle other events as needed
            }
        }
        
        Ok(())
    }
    
    /// Request engine shutdown
    pub fn quit(&mut self) {
        log::info!("Engine shutdown requested");
        self.running = false;
    }
    
    /// Get the ECS world
    pub fn world(&self) -> &World {
        &self.world
    }
    
    /// Get mutable access to the ECS world
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
    
    /// Get the asset manager
    pub fn assets(&self) -> &AssetManager {
        &self.assets
    }
    
    /// Get mutable access to the asset manager
    pub fn assets_mut(&mut self) -> &mut AssetManager {
        &mut self.assets
    }
    
    /// Get the graphics engine
    pub fn graphics_engine(&self) -> &GraphicsEngine {
        &self.graphics_engine
    }
    
    /// Get mutable access to the graphics engine
    pub fn graphics_engine_mut(&mut self) -> &mut GraphicsEngine {
        &mut self.graphics_engine
    }
    
    /// Get the input manager
    pub fn input(&self) -> &InputManager {
        &self.input
    }
    
    /// Get the current frame delta time
    pub fn delta_time(&self) -> f32 {
        self.timer.delta_time()
    }
    
    /// Request engine shutdown
    pub fn shutdown(&mut self) {
        self.running = false;
    }
}

/// Engine configuration
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// Window configuration
    pub window: WindowConfig,
    
    /// Renderer configuration
    pub renderer: RendererConfig,
    
    /// Asset system configuration
    pub assets: AssetConfig,
    
    /// Engine features to enable
    pub features: EngineFeatures,
}

/// Window configuration
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title
    pub title: String,
    
    /// Window width
    pub width: u32,
    
    /// Window height
    pub height: u32,
    
    /// Whether window is resizable
    pub resizable: bool,
    
    /// Whether to start in fullscreen
    pub fullscreen: bool,
    
    /// VSync setting
    pub vsync: bool,
}

/// Renderer configuration
#[derive(Debug, Clone)]
pub struct RendererConfig {
    /// Enable Vulkan validation layers (debug builds only)
    pub validation: bool,
    
    /// Maximum number of frames in flight
    pub max_frames_in_flight: u32,
    
    /// Enable HDR rendering
    pub hdr: bool,
    
    /// MSAA sample count
    pub msaa_samples: u32,
}

/// Asset system configuration
#[derive(Debug, Clone)]
pub struct AssetConfig {
    /// Asset search paths
    pub search_paths: Vec<String>,
    
    /// Enable hot reloading in debug builds
    pub hot_reload: bool,
    
    /// Asset cache size in MB
    pub cache_size_mb: u32,
}

/// Engine features
#[derive(Debug, Clone)]
pub struct EngineFeatures {
    /// Enable audio system
    pub audio: bool,
    
    /// Enable physics system
    pub physics: bool,
    
    /// Enable profiling
    pub profiling: bool,
    
    /// Enable debug rendering
    pub debug_rendering: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig {
                title: "Rust Engine Application".to_string(),
                width: 1280,
                height: 720,
                resizable: true,
                fullscreen: false,
                vsync: true,
            },
            renderer: RendererConfig {
                validation: cfg!(debug_assertions),
                max_frames_in_flight: 2,
                hdr: false,
                msaa_samples: 1,
            },
            assets: AssetConfig {
                search_paths: vec!["resources".to_string()],
                hot_reload: cfg!(debug_assertions),
                cache_size_mb: 512,
            },
            features: EngineFeatures {
                audio: true,
                physics: false,
                profiling: cfg!(debug_assertions),
                debug_rendering: cfg!(debug_assertions),
            },
        }
    }
}

/// Engine-level errors
#[derive(Error, Debug)]
pub enum EngineError {
    /// Initialization error
    #[error("Engine initialization failed: {0}")]
    InitializationFailed(String),
    
    /// Rendering error
    #[error("Rendering error: {0}")]
    RenderError(String),
    
    /// Asset system error
    #[error("Asset system error: {0}")]
    AssetError(String),
    
    /// Input handling error
    #[error("Input error: {0}")]
    InputError(String),
    
    /// Application error
    #[error("Application error: {0}")]
    ApplicationError(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}
