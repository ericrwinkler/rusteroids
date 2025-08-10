//! Application trait and lifecycle management

use crate::engine::{Engine, EngineError};
use thiserror::Error;

/// Application lifecycle trait
/// 
/// Implement this trait to create your game or application using the engine.
pub trait Application {
    /// Initialize the application
    /// 
    /// Called once after the engine is initialized. Use this to set up your
    /// initial game state, load assets, and configure systems.
    fn initialize(&mut self, engine: &mut Engine) -> Result<(), AppError>;
    
    /// Update the application
    /// 
    /// Called every frame. Implement your game logic here.
    /// 
    /// # Arguments
    /// * `engine` - Mutable reference to the engine
    /// * `delta_time` - Time since last frame in seconds
    fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), AppError>;
    
    /// Render the application
    /// 
    /// Called after update. Use this for any custom rendering that isn't
    /// handled by the ECS rendering systems.
    fn render(&mut self, engine: &mut Engine) -> Result<(), AppError> {
        // Default implementation uses the engine's renderer
        engine.render()
    }
    
    /// Handle application events
    /// 
    /// Called when the application receives events (window events, input, etc.)
    fn handle_event(&mut self, engine: &mut Engine, event: AppEvent) -> Result<(), AppError> {
        // Default implementation forwards to engine
        engine.handle_event(event)
    }
    
    /// Cleanup the application
    /// 
    /// Called when the application is shutting down. Use this to save state,
    /// clean up resources, etc.
    fn cleanup(&mut self, engine: &mut Engine);
}

/// Application-level errors
#[derive(Error, Debug)]
pub enum AppError {
    /// Engine error propagated to application level
    #[error("Engine error: {0}")]
    Engine(#[from] EngineError),
    
    /// Custom application error
    #[error("Application error: {0}")]
    Custom(String),
    
    /// Asset loading error
    #[error("Asset error: {0}")]
    Asset(String),
    
    /// Configuration error
    #[error("Config error: {0}")]
    Config(String),
    
    /// Game logic error
    #[error("Game logic error: {0}")]
    GameLogic(String),
}

/// Application events
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Window was resized
    WindowResized { 
        /// New window width
        width: u32, 
        /// New window height
        height: u32 
    },
    
    /// Window close requested
    WindowCloseRequested,
    
    /// Window gained focus
    WindowFocused,
    
    /// Window lost focus
    WindowUnfocused,
    
    /// Key was pressed
    KeyPressed(crate::input::KeyCode),
    
    /// Key was released
    KeyReleased(crate::input::KeyCode),
    
    /// Key input event
    KeyInput { 
        /// The key that was pressed/released
        key: crate::input::KeyCode, 
        /// Whether the key was pressed (true) or released (false)
        pressed: bool 
    },
    
    /// Mouse button event
    MouseButton { 
        /// The mouse button that was pressed/released
        button: crate::input::MouseButton, 
        /// Whether the button was pressed (true) or released (false)
        pressed: bool 
    },
    
    /// Mouse movement
    MouseMoved { 
        /// New X coordinate
        x: f64, 
        /// New Y coordinate
        y: f64 
    },
    
    /// Mouse wheel
    MouseWheel { 
        /// Horizontal scroll delta
        delta_x: f64, 
        /// Vertical scroll delta
        delta_y: f64 
    },
}
