//! Backend-agnostic window management trait
//! 
//! This module defines the internal trait that all window backends must implement.
//! This trait is not exposed to applications - it's purely for internal abstraction
//! between the high-level WindowHandle and backend-specific implementations.

use glfw::WindowEvent;

/// Internal trait for window backend implementations
/// 
/// This trait defines the interface that all window backends must implement.
/// It provides complete window management functionality while remaining
/// backend-agnostic. Applications never interact with this trait directly.
/// 
/// # Design Philosophy
/// - **Internal Use Only**: This trait is `pub(crate)` and not part of the public API
/// - **Complete Interface**: Covers all window operations needed by applications
/// - **Backend Agnostic**: No backend-specific types in the interface
/// - **Performance**: Designed for efficiency while maintaining abstraction
/// 
/// # Thread Safety
/// Note: Send requirement removed temporarily due to GLFW limitations.
/// Window operations typically need to happen on the main thread anyway.
pub(crate) trait WindowBackend {
    /// Check if the window should close
    /// 
    /// Returns true if the user has requested the window to close (clicked X button,
    /// pressed Alt+F4, etc.) or if the application has explicitly requested closure.
    fn should_close(&self) -> bool;
    
    /// Set whether the window should close
    /// 
    /// Allows the application to programmatically request window closure.
    /// This is typically used when the application determines it should exit
    /// based on game logic or user interface interactions.
    fn set_should_close(&mut self, should_close: bool);
    
    /// Poll for window events
    /// 
    /// Processes pending window system events like input, resize, focus changes, etc.
    /// This should be called regularly (typically once per frame) to ensure
    /// responsive window behavior and proper event processing.
    fn poll_events(&mut self);
    
    /// Get an iterator over recent window events
    /// 
    /// Returns an iterator that yields window events that have occurred since
    /// the last call to poll_events(). Events include input, resize, focus
    /// changes, and other window system notifications.
    fn event_iter(&self) -> Box<dyn Iterator<Item = (f64, WindowEvent)> + '_>;
    
    /// Get the current window size in pixels
    /// 
    /// Returns the current client area size (excluding window decorations
    /// like title bar, borders, etc.). This is the actual drawable area size.
    fn get_size(&self) -> (u32, u32);
    
    /// Set the window size in pixels
    /// 
    /// Resizes the window's client area to the specified dimensions.
    /// The actual window size including decorations will be larger.
    fn set_size(&mut self, width: u32, height: u32);
    
    /// Get the current window title
    fn get_title(&self) -> String;
    
    /// Set the window title text
    /// 
    /// Updates the text displayed in the window's title bar.
    /// The title is typically visible in the title bar and taskbar.
    fn set_title(&mut self, title: &str);
    
    /// Get the current window position on screen
    /// 
    /// Returns the position of the window's top-left corner relative to
    /// the screen's coordinate system (typically top-left origin).
    fn get_position(&self) -> (i32, i32);
    
    /// Set the window position on screen
    /// 
    /// Moves the window so its top-left corner is at the specified screen coordinates.
    fn set_position(&mut self, x: i32, y: i32);
    
    /// Check if the window is currently in fullscreen mode
    fn is_fullscreen(&self) -> bool;
    
    /// Set fullscreen mode
    /// 
    /// Switches between windowed and fullscreen mode. In fullscreen mode,
    /// the window typically covers the entire screen and window decorations
    /// are hidden for maximum immersion.
    fn set_fullscreen(&mut self, fullscreen: bool);
    
    /// Check if the window is currently minimized (iconified)
    fn is_minimized(&self) -> bool;
    
    /// Check if the window currently has input focus
    fn has_focus(&self) -> bool;
    
    /// Get access to the concrete type for downcasting
    /// 
    /// This method enables safe downcasting from trait objects to concrete types.
    /// It's used internally by the rendering system to access backend-specific functionality
    /// while maintaining type safety.
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get mutable access to the concrete type for downcasting
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Internal trait for render backend access
/// 
/// This trait provides type-erased access to backend-specific rendering data
/// without exposing the specific backend type to the high-level API.
/// 
/// # Purpose
/// The renderer needs access to backend-specific window data (like Vulkan surfaces)
/// but this should not be exposed through the public WindowHandle API.
/// This trait provides a safe, internal mechanism for that access.
pub(crate) trait RenderSurface {
    /// Get a type-erased reference to the backend-specific render surface data
    /// 
    /// This method allows the renderer to access backend-specific data through
    /// a safe downcasting mechanism. The specific backend type is determined
    /// at runtime based on which backend created the window.
    fn as_any(&self) -> &dyn std::any::Any;
    
    /// Get a type-erased mutable reference to the backend-specific render surface data
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Extension trait to provide render surface access for window backends
/// 
/// This trait is automatically implemented for any type that implements both
/// WindowBackend and can be converted to a RenderSurface. It provides the
/// bridge between window management and rendering without exposing backend
/// details to the public API.
pub(crate) trait WindowBackendExt: WindowBackend {
    /// Get render surface access for the renderer
    /// 
    /// This method is used internally by the renderer to access backend-specific
    /// rendering data (like Vulkan surfaces). Applications never call this method.
    fn as_render_surface(&mut self) -> &mut dyn RenderSurface;
}

// Automatic implementation for any WindowBackend that also implements RenderSurface
impl<T> WindowBackendExt for T 
where 
    T: WindowBackend + RenderSurface 
{
    fn as_render_surface(&mut self) -> &mut dyn RenderSurface {
        self
    }
}
