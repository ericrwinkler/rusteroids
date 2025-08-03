// src/render/window.rs
use glfw::{Glfw, PWindow, WindowEvent, WindowMode};

pub struct WindowHandle {
    pub glfw: Glfw,
    pub window: PWindow,
    pub events: glfw::GlfwReceiver<(f64, WindowEvent)>,
}

impl WindowHandle {
    pub fn new(width: u32, height: u32, title: &str) -> Self {
        let mut glfw = glfw::init(glfw::fail_on_errors).unwrap();
        glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
        let (mut window, events) = glfw
            .create_window(width, height, title, WindowMode::Windowed)
            .expect("Failed to create GLFW window.");
        window.set_key_polling(true);
        window.set_close_polling(true);
        window.set_framebuffer_size_polling(true);
        Self {
            glfw,
            window,
            events,
        }
    }

    pub fn poll_events(&mut self) {
        self.glfw.poll_events();
    }

    pub fn event_iter(&self) -> glfw::FlushedMessages<(f64, WindowEvent)> {
        glfw::flush_messages(&self.events)
    }

    pub fn should_close(&self) -> bool {
        self.window.should_close()
    }

    pub fn set_should_close(&mut self, value: bool) {
        self.window.set_should_close(value);
    }

    #[allow(dead_code)]
    pub fn get_framebuffer_size(&self) -> (i32, i32) {
        self.window.get_framebuffer_size()
    }

    pub fn create_window_surface(
        &mut self,
        instance: ash::vk::Instance,
        surface: &mut ash::vk::SurfaceKHR,
    ) -> ash::vk::Result {
        self.window
            .create_window_surface(instance, std::ptr::null(), surface)
    }

    #[allow(dead_code)]
    pub fn window_mut(&mut self) -> &mut PWindow {
        &mut self.window
    }
}
