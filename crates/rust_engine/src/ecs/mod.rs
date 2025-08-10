//! Entity-Component-System implementation
//!
//! Provides a high-performance ECS architecture for game logic.

pub mod world;
pub mod entity;
pub mod component;
pub mod system;
pub mod query;

pub use world::World;
pub use entity::Entity;
pub use component::Component;
pub use system::System;
pub use query::Query;
