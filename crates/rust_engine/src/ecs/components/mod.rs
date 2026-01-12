//! ECS Components module
//! 
//! Contains all engine components following Game Engine Architecture principles

pub mod lighting;
pub mod transform;
pub mod renderable;
pub mod movement;
pub mod lifecycle;
pub mod pickable;
pub mod selection;
pub mod collision;
pub mod trail_emitter;

pub use lighting::{LightComponent, LightType, LightFactory};
pub use transform::{TransformComponent, TransformFactory};
pub use renderable::{RenderableComponent, RenderableFactory};
pub use movement::{MovementComponent, MovementFactory};
pub use lifecycle::{LifecycleComponent, LifecycleFactory, EntityState};
pub use pickable::PickableComponent;
pub use selection::SelectionComponent;
pub use collision::{ColliderComponent, CollisionStateComponent};
pub use trail_emitter::{TrailEmitterComponent, TrailEmitterFactory, TrailSegment};
