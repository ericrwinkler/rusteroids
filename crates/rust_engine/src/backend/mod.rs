//! # Backend Module
//!
//! This module contains all backend implementations for different subsystems.
//! It provides the concrete implementations of traits defined in the core modules.
//!
//! ## Organization
//!
//! - **Render**: Graphics backend abstraction and implementations
//! - **Vulkan**: Vulkan graphics backend implementation
//! - **Window**: Window management backend implementations
//!
//! ## Design Principles
//!
//! - **Abstraction**: Backends implement traits defined in core modules
//! - **Isolation**: Backend-specific code is isolated from high-level APIs
//! - **Modularity**: Easy to add new backends without changing core code
//! - **Performance**: Direct access to platform-specific optimizations

pub mod render;
pub mod vulkan;

// Re-export commonly used backend traits
pub use render::{RenderBackend, WindowBackendAccess, BackendResult};
