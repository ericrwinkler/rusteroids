//! # Rust Engine
//!
//! A modular game engine written in Rust with Vulkan rendering support.
//!
//! ## Features
//!
//! - **Vulkan Rendering**: High-performance 3D and 2D rendering
//! - **ECS Architecture**: Flexible Entity-Component-System
//! - **Asset Management**: Efficient asset loading and caching
//! - **Cross-Platform**: Windows, Linux, and macOS support
//! - **Hot Reloading**: Development-time asset reloading
//! - **Plugin System**: Extensible architecture
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rust_engine::prelude::*;
//!
//! struct MyApp;
//!
//! impl Application for MyApp {
//!     fn initialize(&mut self, engine: &mut Engine) -> Result<(), AppError> {
//!         // Initialize your game
//!         Ok(())
//!     }
//!     
//!     fn update(&mut self, engine: &mut Engine, delta_time: f32) -> Result<(), AppError> {
//!         // Update game logic
//!         Ok(())
//!     }
//!     
//!     fn cleanup(&mut self, engine: &mut Engine) {
//!         // Cleanup resources
//!     }
//! }
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = EngineConfig::default();
//!     let mut app = MyApp;
//!     Engine::run(config, &mut app)?;
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions, clippy::similar_names, clippy::too_many_arguments)]

// Core engine modules
pub mod core;

// Legacy module organization (maintained for compatibility)
pub mod foundation;
pub mod ecs;
pub mod assets;
pub mod render;
pub mod input;
pub mod audio;
pub mod physics;
pub mod settings;

mod application;
mod engine;

pub use application::{Application, AppError};
pub use engine::{Engine, EngineConfig, EngineError};

/// Common imports for engine users
pub mod prelude {
    pub use crate::{
        Application, AppError,
        application::AppEvent,
        Engine, EngineConfig, EngineError,
        foundation::{
            math::{Vec3, Mat4, Transform},
            time::{Timer, Stopwatch},
        },
        ecs::{World, Entity, Component, System, Query},
        assets::{Asset, AssetHandle, AssetManager},
        render::{Renderer, Camera, Mesh, Material},
        input::{InputManager, KeyCode, MouseButton},
        // New unified config system
        core::config::{ApplicationConfig, VulkanRendererConfig, ShaderConfig},
    };
}
