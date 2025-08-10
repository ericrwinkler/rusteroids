// Cleanup system
use rust_engine::ecs::{System, World};

pub struct CleanupSystem;

impl System for CleanupSystem {
    fn run(&mut self, _world: &mut World) {
        // TODO: Implement cleanup logic for expired entities
    }
}
