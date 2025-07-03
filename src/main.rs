extern crate ash;
extern crate glfw;

mod window;
use window::Window;

mod vulkan;
use vulkan::Vulkan;

mod lve_pipeline;
use lve_pipeline::Lve_pipeline;

use glfw::{Action, Context, Key};

fn main() {
    let mut window: Window = Window::try_new(800, 600, "Hello this is window");

    main_loop(&mut window);
}

fn main_loop(window: &mut Window) {
    // This function is not in the tutorial yet, but it will be used to run the main loop
    // It will handle events and rendering

    println!("Main loop running...");

    // Loop until the user closes the window
    while !window.glfw_window.should_close() {
        // Poll for and process events
        window.glfw_context.poll_events();
        for (_, event) in glfw::flush_messages(&window.glfw_receiver) {
            println!("{:?}", event);
            match event {
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.glfw_window.set_should_close(true)
                }
                _ => {}
            }
        }
    }
}

