//! Window management subsystem
//! 
//! This module provides a complete window management abstraction layer that supports
//! multiple rendering backends while maintaining clean separation of concerns.
//! 
//! # Architecture Overview
//! 
//! The window subsystem follows a layered architecture:
//! 
//! ```
//! ┌─────────────────────────────────┐
//! │     Application Code            │
//! └─────────────┬───────────────────┘
//!               │ Uses
//!         ┌─────▼─────┐
//!         │ WindowHandle │ ← Public API (handle.rs)
//!         └─────┬─────┘
//!               │ Uses
//!      ┌────────▼────────┐
//!      │ WindowBackend   │ ← Internal trait (backend.rs)  
//!      │ trait           │
//!      └────────┬────────┘
//!               │ Implemented by
//!   ┌───────────▼───────────┐
//!   │ vulkan::Window        │ ← Concrete backend (vulkan/window.rs)
//!   │ opengl::Window        │ ← Future backends
//!   │ directx::Window       │
//!   └───────────────────────┘
//! ```
//! 
//! # Module Organization
//! 
//! - **`handle`**: High-level application-facing window interface
//! - **`backend`**: Internal trait defining the backend contract
//! - Backend implementations are in their respective folders (e.g., `../vulkan/window.rs`)
//! 
//! # Design Goals
//! 
//! - **Backend Agnostic**: Applications don't depend on specific rendering APIs
//! - **Type Safe**: Compile-time prevention of backend coupling
//! - **Extensible**: Easy to add new rendering backends
//! - **Testable**: Support for mock backends in tests

pub mod handle;
pub mod backend;

// Re-export the main public type for convenience
pub use handle::WindowHandle;
