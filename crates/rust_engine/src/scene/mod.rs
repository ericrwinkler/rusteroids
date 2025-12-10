//! Scene management system
//!
//! Provides high-level scene graph functionality that bridges the ECS (gameplay)
//! layer with the Renderer (graphics) layer. Following Game Engine Architecture
//! Chapter 11.2.7 - Scene Graphs.
//!
//! ## Architecture
//!
//! ```text
//! ECS World (Gameplay)
//!      ↓
//! Scene Manager (Bridge)
//!      ↓
//! Renderer (Graphics)
//! ```
//!
//! The Scene Manager:
//! - Syncs ECS components (Transform, Renderable, Light, Camera) to rendering data
//! - Performs spatial queries (frustum culling, visibility)
//! - Generates optimized render queues (batched by material)
//! - Manages entity lifecycle (automatic cleanup)

mod scene_manager;
mod scene_graph;
mod renderable_object;
mod render_queue;
mod scene_renderer;

pub use scene_manager::SceneManager;
pub use scene_graph::{SceneGraph, SimpleListGraph, AABB, Frustum, Plane};
pub use renderable_object::RenderableObject;
pub use render_queue::{RenderQueue, RenderBatch};
pub use scene_renderer::{SceneRenderer, SceneRendererConfig};
