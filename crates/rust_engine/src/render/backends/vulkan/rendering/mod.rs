//! Vulkan rendering components

pub mod commands;
pub mod render_pass;
pub mod shader;
pub mod vertex_layout;
pub mod command_recorder;
pub mod dynamic_coordinator;

// Re-export specific types from commands to avoid ambiguity
pub use commands::{CommandPool, ActiveRenderPass};
// Note: CommandRecorder from commands is aliased as VulkanCommandRecorder in parent mod

pub use render_pass::*;
pub use shader::*;
pub use vertex_layout::*;

// Re-export CommandRecorder from command_recorder (the new one)
pub use command_recorder::CommandRecorder;

pub use dynamic_coordinator::*;
