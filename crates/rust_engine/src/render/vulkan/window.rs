//! Window management using GLFW
//! 
//! Provides cross-platform window creation and event handling for Vulkan

use thiserror::Error;

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
