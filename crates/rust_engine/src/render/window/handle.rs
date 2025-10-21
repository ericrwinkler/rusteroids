//! High-level window handle for applications
//! 
//! This module provides a simplified window management interface that abstracts
//! away the underlying windowing system and graphics backend details.
//! 
//! # Architecture Issues and Design Analysis
//! 
//! ## Current State: INCOMPLETE ABSTRACTION
//! While this module attempts to provide a high-level window interface, it suffers
//! from significant abstraction violations that expose Vulkan-specific implementation
//! details to application code.
//! 
//! ## Critical Design Violations:
//! 
//! ### ABSTRACTION LEAKAGE: Direct Vulkan Access Methods
//! The `WindowHandle` struct exposes `vulkan_window()` and `vulkan_window_mut()` methods
//! that completely bypass the abstraction layer. This creates several problems:
//! 
//! 1. **Backend Coupling**: Applications can directly access Vulkan-specific functionality
//! 2. **Type Pollution**: Vulkan types leak into application code that should be backend-agnostic
//! 3. **Future Incompatibility**: Adding other backends (OpenGL, DirectX, WebGPU) becomes impossible
//! 4. **Testing Difficulties**: Applications using these methods cannot be tested without Vulkan
//! 
//! **FIXME: Remove vulkan_window() accessor methods entirely**
//! These methods should not exist in a properly abstracted window interface. The renderer
//! should access backend-specific window data through internal interfaces, not through
//! the public application API.
//! 
//! ### INCOMPLETE WINDOW MANAGEMENT
//! The current interface only exposes basic window lifecycle operations (close, events)
//! but lacks many common window management features that applications typically need:
//! - Window resizing and position management
//! - Fullscreen/windowed mode switching  
//! - Window title changes
//! - Icon/cursor management
//! - Multi-monitor support
//! 
//! ## Recommended Architecture:
//! 
//! ### Backend-Agnostic Window Interface
//! The window handle should provide a complete interface for window management without
//! exposing any backend-specific details:
//! 
//! ```rust
//! pub struct WindowHandle {
//!     // Private backend-specific data - NO public accessors
//!     backend: Box<dyn WindowBackend>,
//! }
//! 
//! impl WindowHandle {
//!     // Complete window management API
//!     pub fn set_size(&mut self, width: u32, height: u32);
//!     pub fn get_size(&self) -> (u32, u32);  
//!     pub fn set_title(&mut self, title: &str);
//!     pub fn set_fullscreen(&mut self, fullscreen: bool);
//!     // ... other high-level operations
//! }
//! ```
//! 
//! ### Internal Backend Access Pattern
//! The renderer should access backend-specific data through internal interfaces:
//! 
//! ```rust
//! // In renderer module, not exposed to applications
//! impl WindowHandle {
//!     pub(crate) fn backend_data<T: WindowBackend>(&mut self) -> &mut T {
//!         // Safe downcast to specific backend type
//!     }
//! }
//! ```
//! 
//! ## Performance and Resource Management:
//! 
//! ### Event Handling Efficiency
//! Current event polling approach may not be optimal for all applications:
//! - Consider callback-based event handling for responsive applications
//! - Support both polling and callback patterns based on application needs
//! - Efficient event filtering to avoid processing unnecessary events
//! 
//! ### Window Creation Optimization
//! - Lazy window creation for applications that need custom setup
//! - Window creation parameter validation
//! - Support for different window types (main, dialog, utility)
//! 
//! ## Design Goals for Refactored Version:
//! 
//! 1. **Complete Backend Abstraction**: Zero exposure of Vulkan or other backend types
//! 2. **Rich Window Management**: Full set of window operations for modern applications  
//! 3. **Multiple Backend Support**: Easy integration of OpenGL, DirectX, WebGPU backends
//! 4. **Performance**: Efficient event handling and resource management
//! 5. **Platform Integration**: Native platform features (decorations, DPI, etc.)

use crate::render::vulkan::Window as VulkanWindow;
use crate::render::window::backend::{WindowBackend, RenderSurface};
use glfw::WindowEvent;

/// High-level window handle that abstracts windowing system details
/// 
/// **ARCHITECTURE VIOLATION**: This struct currently exposes Vulkan-specific implementation
/// details through its public API, breaking the abstraction that applications should rely on.
/// 
/// The window handle should provide a complete, backend-agnostic interface for window
/// management without requiring applications to know about Vulkan, GLFW, or other
/// implementation details.
/// 
/// **FIXME**: Remove all backend-specific accessor methods and implement proper abstraction
pub struct WindowHandle {
    backend: Box<dyn WindowBackend>,
}

impl WindowHandle {
    /// Create a new window with specified dimensions and title
    /// 
    /// # Arguments
    /// * `width` - Window width in pixels
    /// * `height` - Window height in pixels  
    /// * `title` - Window title string
    /// 
    /// # Backend Coupling Issue
    /// This method directly creates a Vulkan window, making it impossible to use
    /// different rendering backends. The window creation should be backend-agnostic.
    /// 
    /// **FIXME**: Use backend selection pattern:
    /// ```rust
    /// pub fn new(width: u32, height: u32, title: &str, backend: RenderBackend) -> Self
    /// ```
    /// 
    /// # Error Handling
    /// Currently panics if window creation fails. Should return Result for proper error handling.
    /// Applications should be able to handle window creation failures gracefully.
    /// 
    /// **FIXME**: Return `Result<Self, WindowError>` instead of panicking
    pub fn new(width: u32, height: u32, title: &str) -> Self {
        let vulkan_window = VulkanWindow::new(title, width, height)
            .expect("Failed to create window");
        
        Self {
            backend: Box::new(vulkan_window),
        }
    }
    
    /// Check if the window should close
    pub fn should_close(&self) -> bool {
        self.backend.should_close()
    }
    
    /// Set whether the window should close
    pub fn set_should_close(&mut self, should_close: bool) {
        self.backend.set_should_close(should_close);
    }
    
    /// Poll for events
    pub fn poll_events(&mut self) {
        self.backend.poll_events();
    }
    
    /// Get event iterator
    pub fn event_iter(&self) -> impl Iterator<Item = (f64, WindowEvent)> + use<'_> {
        self.backend.event_iter()
    }
    
    /// Get the current window size in pixels
    ///
    /// Returns the window's client area size (drawable area, excluding decorations).
    pub fn get_size(&self) -> (u32, u32) {
        self.backend.get_size()
    }
    
    /// Set the window size in pixels
    ///
    /// Resizes the window's client area to the specified dimensions.
    pub fn set_size(&mut self, width: u32, height: u32) {
        self.backend.set_size(width, height);
    }
    
    /// Get the current window title
    pub fn get_title(&self) -> String {
        self.backend.get_title()
    }
    
    /// Set the window title
    pub fn set_title(&mut self, title: &str) {
        self.backend.set_title(title);
    }
    
    /// Get the window position on screen
    pub fn get_position(&self) -> (i32, i32) {
        self.backend.get_position()
    }
    
    /// Set the window position on screen
    pub fn set_position(&mut self, x: i32, y: i32) {
        self.backend.set_position(x, y);
    }
    
    /// Check if the window is in fullscreen mode
    pub fn is_fullscreen(&self) -> bool {
        self.backend.is_fullscreen()
    }
    
    /// Set fullscreen mode
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.backend.set_fullscreen(fullscreen);
    }
    
    /// Check if the window is minimized
    pub fn is_minimized(&self) -> bool {
        self.backend.is_minimized()
    }
    
    /// Check if the window has input focus
    pub fn has_focus(&self) -> bool {
        self.backend.has_focus()
    }
    
    /// Internal method for renderer backend access
    ///
    /// **INTERNAL USE ONLY**: This method is used by the renderer to access
    /// backend-specific rendering data. Applications should never call this method.
    /// 
    /// This replaces the old vulkan_window() accessor methods with a safer,
    /// type-erased approach that doesn't expose backend implementation details
    /// to the public API.
    #[allow(dead_code)] // Part of window backend API design
    pub(crate) fn backend_for_renderer(&mut self) -> &mut dyn WindowBackend {
        self.backend.as_mut()
    }
    
    /// Internal method to get render surface access for the renderer
    /// 
    /// This method provides access to backend-specific rendering data through
    /// the RenderSurface trait, allowing safe downcasting for renderer initialization.
    pub(crate) fn render_surface(&mut self) -> &mut dyn RenderSurface {
        // Use the WindowBackend's as_any_mut method for safe downcasting
        let any_backend = self.backend.as_any_mut();
        let vulkan_window = any_backend.downcast_mut::<VulkanWindow>()
            .expect("Expected Vulkan window backend");
        vulkan_window as &mut dyn RenderSurface
    }
}

/// Implementation of WindowBackendAccess for WindowHandle
///
/// Provides access to backend-specific window operations needed by the renderer.
impl crate::render::WindowBackendAccess for WindowHandle {
    fn get_backend_window(&mut self) -> &mut dyn std::any::Any {
        // Return the backend window as a trait object for downcasting
        self.backend.as_any_mut()
    }
}
