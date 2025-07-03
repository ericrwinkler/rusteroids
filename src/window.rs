extern crate ash;
extern crate glfw;

use ash::Entry;
use glfw::{Action, Context, Key};

pub struct Window {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub glfw_context: glfw::Glfw,
    pub glfw_receiver: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
    pub glfw_window: glfw::PWindow,
}

impl Window {
    pub fn try_new(width: u32, height: u32, title: &str) -> Self {
        // Initialize GLFW
        let mut context = glfw::init(glfw::log_errors).expect("Failed to initialize GLFW");

        // Disable OpenGL context creation
        context.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
        // Disable GLFW resizable window
        context.window_hint(glfw::WindowHint::Resizable(false));

        let (mut window, receiver) = context
            .create_window(width, height, title, glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");

        // Set the window's context to the current thread
        window.make_current();

        // Set the key polling to true to receive key events
        window.set_key_polling(true);

        Window {
            width,
            height,
            title: title.to_string(),
            glfw_context: context,
            glfw_window: window,
            glfw_receiver: receiver,
        }
    }
}
