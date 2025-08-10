//! High-level window handle for applications

use crate::render::vulkan::Window as VulkanWindow;
use glfw::WindowEvent;

/// High-level window handle that wraps the Vulkan window
pub struct WindowHandle {
    vulkan_window: VulkanWindow,
}

impl WindowHandle {
    /// Create a new window handle
    pub fn new(width: u32, height: u32, title: &str) -> Self {
        let vulkan_window = VulkanWindow::new(title, width, height)
            .expect("Failed to create window");
        
        Self {
            vulkan_window,
        }
    }
    
    /// Check if the window should close
    pub fn should_close(&self) -> bool {
        self.vulkan_window.should_close()
    }
    
    /// Set whether the window should close
    pub fn set_should_close(&mut self, should_close: bool) {
        self.vulkan_window.set_should_close(should_close);
    }
    
    /// Poll for events
    pub fn poll_events(&mut self) {
        self.vulkan_window.poll_events();
    }
    
    /// Get event iterator
    pub fn event_iter(&self) -> impl Iterator<Item = (f64, WindowEvent)> + use<'_> {
        self.vulkan_window.flush_events()
    }
    
    /// Get the underlying Vulkan window (for renderer use)
    pub fn vulkan_window(&self) -> &VulkanWindow {
        &self.vulkan_window
    }
    
    /// Get mutable reference to the underlying Vulkan window (for renderer use)
    pub fn vulkan_window_mut(&mut self) -> &mut VulkanWindow {
        &mut self.vulkan_window
    }
}
