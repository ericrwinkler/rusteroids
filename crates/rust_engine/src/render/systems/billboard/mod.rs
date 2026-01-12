//! Billboard rendering system
//! 
//! Provides camera-facing quad rendering for particles, trails, and effects.
//! Supports multiple orientation modes and efficient batched rendering.

pub mod types;
pub mod orientation;
pub mod renderer;

pub use types::{BillboardQuad, BillboardOrientation, BillboardInstance, BlendMode};
pub use orientation::{
    calculate_screen_aligned_matrix,
    calculate_velocity_aligned_matrix,
    calculate_world_axis_aligned_matrix,
};
pub use renderer::{BillboardRenderer, render_billboards_from_pool};
