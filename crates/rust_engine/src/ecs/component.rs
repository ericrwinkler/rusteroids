//! Component trait and implementations

/// Marker trait for components
pub trait Component: 'static + Send + Sync {}

// Implement Component for common types
impl Component for crate::foundation::math::Transform {}
impl Component for crate::render::Camera {}
impl Component for crate::render::Material {}
impl Component for crate::foundation::collections::Handle {}
impl<T: 'static + Send + Sync> Component for crate::foundation::collections::TypedHandle<T> {}

// Implement Component for collision components
impl Component for crate::ecs::components::ColliderComponent {}
impl Component for crate::ecs::components::CollisionStateComponent {}
