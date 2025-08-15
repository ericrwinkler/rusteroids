//! GLFW-based window management for Vulkan rendering
//! 
//! This module provides cross-platform window creation and event handling specifically
//! optimized for Vulkan rendering. Handles window lifecycle, event processing, and
//! Vulkan surface creation with proper integration for the rendering system.
//! 
//! # Architecture Note: BACKEND-SPECIFIC IMPLEMENTATION
//! 
//! This is appropriately placed in the `vulkan/` module as it contains Vulkan-specific
//! window and surface management code. Unlike the high-level `window.rs` which has
//! abstraction violations, this module correctly isolates backend-specific functionality.
//! 
//! ## Design Strengths:
//! - ✅ **Platform Abstraction**: GLFW provides cross-platform windowing
//! - ✅ **Vulkan Integration**: Proper surface creation for Vulkan rendering
//! - ✅ **Event Handling**: Complete window event processing
//! - ✅ **Backend Isolation**: Correctly placed in backend-specific module
//! 
//! ## Integration Quality:
//! This module represents proper architecture by keeping platform and API-specific
//! code isolated in the backend layer, unlike the abstraction violations in the
//! high-level window module.

use thiserror::Error;
use crate::render::window::backend::{WindowBackend, RenderSurface};
use std::any::Any;

/// Window management errors
#[derive(Error, Debug)]
pub enum WindowError {
    #[error("GLFW initialization failed")]
    InitializationFailed,
    
    #[error("Window creation failed")]
    CreationFailed,
    
    #[error("GLFW error: {0}")]
    GlfwError(String),
}

pub type WindowResult<T> = Result<T, WindowError>;

/// GLFW window wrapper with proper resource management
pub struct Window {
    glfw: glfw::Glfw,
    window: glfw::PWindow,
    events: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
}

impl Window {
    pub fn new(title: &str, width: u32, height: u32) -> WindowResult<Self> {
        let mut glfw = glfw::init(glfw::fail_on_errors)
            .map_err(|_| WindowError::InitializationFailed)?;

        // Configure for Vulkan (no OpenGL context)
        glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
        glfw.window_hint(glfw::WindowHint::Resizable(true));

        // Create window
        let (mut window, events) = glfw
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .ok_or(WindowError::CreationFailed)?;

        // Set up event polling
        window.set_key_polling(true);
        window.set_close_polling(true);
        window.set_size_polling(true);
        window.set_framebuffer_size_polling(true);

        Ok(Self {
            glfw,
            window,
            events,
        })
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn poll_events(&mut self) {
        self.glfw.poll_events();
    }

    pub fn flush_events(&self) -> glfw::FlushedMessages<(f64, glfw::WindowEvent)> {
        glfw::flush_messages(&self.events)
    }

    pub fn get_size(&self) -> (u32, u32) {
        let (width, height) = self.window.get_size();
        (width as u32, height as u32)
    }

    pub fn get_framebuffer_size(&self) -> (u32, u32) {
        let (width, height) = self.window.get_framebuffer_size();
        (width as u32, height as u32)
    }

    pub fn set_should_close(&mut self, should_close: bool) {
        self.window.set_should_close(should_close);
    }
    
    /// Get the next key event
    pub fn get_key(&mut self) -> Option<(glfw::Key, i32, glfw::Action, glfw::Modifiers)> {
        for (_, event) in glfw::flush_messages(&self.events) {
            if let glfw::WindowEvent::Key(key, scancode, action, mods) = event {
                return Some((key, scancode, action, mods));
            }
        }
        None
    }
    
    /// Get required Vulkan instance extensions from GLFW
    pub fn get_required_instance_extensions(&self) -> WindowResult<Vec<String>> {
        self.glfw
            .get_required_instance_extensions()
            .ok_or(WindowError::GlfwError("Failed to get required extensions".to_string()))
    }

    /// Create Vulkan surface using GLFW's built-in functionality
    pub fn create_vulkan_surface(&mut self, instance: ash::vk::Instance) -> WindowResult<ash::vk::SurfaceKHR> {
        let mut surface = ash::vk::SurfaceKHR::null();
        let result = self.window.create_window_surface(instance, std::ptr::null(), &mut surface);
        
        if result == ash::vk::Result::SUCCESS {
            Ok(surface)
        } else {
            Err(WindowError::GlfwError(format!("Failed to create Vulkan surface: {:?}", result)))
        }
    }
}

// Implementation of WindowBackend trait for vulkan::Window
impl WindowBackend for Window {
    fn should_close(&self) -> bool {
        self.should_close()
    }
    
    fn set_should_close(&mut self, should_close: bool) {
        self.set_should_close(should_close);
    }
    
    fn poll_events(&mut self) {
        self.poll_events();
    }
    
    fn event_iter(&self) -> Box<dyn Iterator<Item = (f64, glfw::WindowEvent)> + '_> {
        Box::new(self.flush_events())
    }
    
    fn get_size(&self) -> (u32, u32) {
        self.get_size()
    }
    
    fn set_size(&mut self, width: u32, height: u32) {
        self.window.set_size(width as i32, height as i32);
    }
    
    fn get_title(&self) -> String {
        // GLFW doesn't provide a way to get the current title, so we'll return a placeholder
        // In a more complete implementation, we'd store the title in the Window struct
        "Rusteroids".to_string()
    }
    
    fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }
    
    fn get_position(&self) -> (i32, i32) {
        self.window.get_pos()
    }
    
    fn set_position(&mut self, x: i32, y: i32) {
        self.window.set_pos(x, y);
    }
    
    fn is_fullscreen(&self) -> bool {
        // GLFW-specific: Check if window has a monitor set (fullscreen mode)
        // This is a simplified implementation - in practice we'd track this state
        false // Placeholder - would need to track fullscreen state in Window struct
    }
    
    fn set_fullscreen(&mut self, _fullscreen: bool) {
        // GLFW fullscreen implementation would go here
        // For now, this is a placeholder that does nothing
        // In a complete implementation, we'd use GLFW's monitor and video mode APIs
        log::warn!("Fullscreen mode not implemented for GLFW backend");
    }
    
    fn is_minimized(&self) -> bool {
        self.window.is_iconified()
    }
    
    fn has_focus(&self) -> bool {
        self.window.is_focused()
    }
    
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// Implementation of RenderSurface trait for vulkan::Window
impl RenderSurface for Window {
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
