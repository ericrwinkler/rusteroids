// ECS component definitions
pub struct Position { pub x: f32, pub y: f32, pub z: f32 }
pub struct Velocity { pub dx: f32, pub dy: f32, pub dz: f32 }
pub struct Renderable { /* mesh/sprite handle, color, etc. */ }
pub struct Collider { pub radius: f32 }
pub struct Player;
pub struct Asteroid;
pub struct Bullet;
