//! # Core Engine Module
//!
//! This module contains the core engine functionality and shared abstractions
//! that are used throughout the engine. It provides the foundational components
//! that other subsystems depend on.
//!
//! ## Organization
//!
//! - **Config**: Unified configuration system for all engine subsystems
//! - **Foundation**: Low-level utilities (math, collections, memory, etc.)
//! - **Assets**: Asset loading and management system
//! - **ECS**: Entity-Component-System implementation

pub mod config;

// Re-export foundation modules for convenience
pub use crate::foundation;
pub use crate::assets;
pub use crate::ecs;

// Re-export commonly used config types
pub use config::{
    ApplicationConfig,
    EngineConfig, 
    VulkanRendererConfig,
    ShaderConfig,
    AssetConfig,
    Config,
    ConfigError,
};
