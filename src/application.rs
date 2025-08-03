// application.rs
// All logic from the original main.rs is moved here for stepwise modularization.

use crate::render::vulkan::VulkanContext;
use crate::render::window::WindowHandle;
use glfw::{Action, Key, WindowEvent};

pub fn run() {
    // Create window
    let mut window = WindowHandle::new(800, 600, "Vulkan Triangle");
    // Create Vulkan context
    let mut vk_ctx = VulkanContext::new(&mut window);

    let mut framebuffer_resized = false;
    while !window.should_close() {
        window.poll_events();
        // Collect events to avoid borrow checker issues
        let events: Vec<_> = window.event_iter().collect();
        for (_, event) in events {
            match event {
                WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
                WindowEvent::FramebufferSize(_, _) => framebuffer_resized = true,
                _ => {}
            }
        }

        if framebuffer_resized {
            vk_ctx.recreate_swapchain(&mut window);
            framebuffer_resized = false;
            continue;
        }

        // Draw frame
        if let Err(e) = vk_ctx.draw_frame(&mut window) {
            if e == ash::vk::Result::ERROR_OUT_OF_DATE_KHR || e == ash::vk::Result::SUBOPTIMAL_KHR {
                vk_ctx.recreate_swapchain(&mut window);
            } else {
                panic!("Vulkan draw_frame error: {:?}", e);
            }
        }
    }

    vk_ctx.cleanup();
}
