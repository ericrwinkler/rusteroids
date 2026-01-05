//! Debug module for visualization and debugging tools
//! 
//! Based on Game Engine Architecture 3rd Edition, Chapter 10.2:
//! "Debug Drawing Facilities"

pub mod draw;
pub mod collision_debug;

pub use draw::{DebugShape, DebugDrawSystem, DebugShapeId};
pub use collision_debug::CollisionDebugVisualizer;
