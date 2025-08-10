//! System trait and implementations

/// System trait for processing entities and components
pub trait System {
    /// Run the system
    fn run(&mut self, world: &mut crate::ecs::World);
}
