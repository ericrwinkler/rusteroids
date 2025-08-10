// Collision system
use rust_engine::ecs::{System, World};

pub struct CollisionSystem;

impl System for CollisionSystem {
    fn run(&mut self, _world: &mut World) {
        // TODO: Implement collision detection
    }
}
