// ECS system trait and system stubs
pub trait System {
    fn run(&mut self /*, world: &mut World, dt: f32 */);
}

pub struct MovementSystem;
impl System for MovementSystem {
    fn run(&mut self) {
        // TODO: Move entities
    }
}
