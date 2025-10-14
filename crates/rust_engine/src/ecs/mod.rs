//! Entity-Component-System implementation
//!
//! Provides a high-performance ECS architecture for game logic.
//! Based on Game Engine Architecture principles: cache-friendly data layout,
//! component purity, and structured update phases.

pub mod world;
pub mod entity;
pub mod component;
pub mod system;
pub mod query;
pub mod components;
pub mod systems;
pub mod scene_manager;

#[cfg(test)]
pub mod tests;

pub use world::World;
pub use entity::Entity;
pub use component::Component;
pub use system::System;
pub use query::Query;

// Re-export common components and systems
pub use components::{
    LightComponent, LightType, LightFactory, 
    TransformComponent, TransformFactory,
    RenderableComponent, RenderableFactory,
    MovementComponent, MovementFactory,
    LifecycleComponent, LifecycleFactory, EntityState
};
pub use systems::{LightingSystem, CoordinateSystemValidator, RenderableCollector};
pub use scene_manager::{SceneManager, SceneConfig, SceneStats};
