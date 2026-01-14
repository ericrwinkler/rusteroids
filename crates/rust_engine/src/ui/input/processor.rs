//! UI Input Processing
//!
//! Handles UI-specific input processing separate from rendering.

use crate::ui::widgets::{UIButton, ButtonState};
use crate::events::{Event, EventType, EventArg, EventSystem};
use crate::input::collision;

/// Mouse button identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button
    Left,
    /// Right mouse button
    Right,
    /// Middle mouse button
    Middle,
}

/// UI input event
#[derive(Debug, Clone)]
pub enum UIInputEvent {
    /// Button was clicked
    ButtonClicked {
        /// Button identifier
        button_id: u32
    },
    /// Button hover state changed
    ButtonHoverChanged {
        /// Button identifier
        button_id: u32,
        /// Is button hovered
        hovered: bool
    },
}

/// UI input processor
///
/// Handles mouse input and button state management separate from rendering.
/// The renderer should only draw buttons in their current state, not process input.
pub struct UIInputProcessor {
    /// Current mouse position in screen coordinates
    mouse_x: f32,
    mouse_y: f32,
    
    /// Mouse button states
    left_button_down: bool,
    left_button_pressed_this_frame: bool,
    left_button_released_this_frame: bool,
    
    /// Screen dimensions for coordinate calculations
    screen_width: f32,
    screen_height: f32,
}

impl UIInputProcessor {
    /// Create a new UI input processor
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            mouse_x: 0.0,
            mouse_y: 0.0,
            left_button_down: false,
            left_button_pressed_this_frame: false,
            left_button_released_this_frame: false,
            screen_width,
            screen_height,
        }
    }
    
    /// Update screen dimensions (call when window resizes)
    pub fn set_screen_size(&mut self, width: f32, height: f32) {
        self.screen_width = width;
        self.screen_height = height;
    }
    
    /// Update mouse position
    pub fn update_mouse_position(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
    
    /// Update mouse button state
    pub fn update_mouse_button(&mut self, button: MouseButton, pressed: bool) {
        if button == MouseButton::Left {
            let was_down = self.left_button_down;
            self.left_button_down = pressed;
            
            // Detect press/release this frame
            self.left_button_pressed_this_frame = !was_down && pressed;
            self.left_button_released_this_frame = was_down && !pressed;
        }
    }
    
    /// Begin new frame (resets per-frame state)
    /// 
    /// NOTE: This should be called AFTER processing window events but BEFORE
    /// processing UI button state, so that button release events are captured.
    pub fn begin_frame(&mut self) {
        // Don't reset here - flags will be reset after use in process_button()
    }
    
    /// Reset per-frame input flags (call after processing all buttons)
    pub fn reset_frame_flags(&mut self) {
        self.left_button_pressed_this_frame = false;
        self.left_button_released_this_frame = false;
    }
    
    /// Process input for a button, updating its state
    ///
    /// # Returns
    /// Optional event if button was clicked or hover changed
    pub fn process_button(&self, button: &mut UIButton, event_system: &mut EventSystem, timestamp: f64) -> Option<UIInputEvent> {
        if !button.enabled || !button.element.visible {
            button.state = ButtonState::Disabled;
            return None;
        }
        
        // Check if mouse is over button - use collision detection from input system
        let is_hovered = collision::point_in_ui_element(
            &button.element,
            self.screen_width,
            self.screen_height,
            self.mouse_x,
            self.mouse_y,
        );
        
        let mut ui_event = None;
        
        // Update button state based on hover and click
        let old_state = button.state;
        
        if !is_hovered {
            button.state = ButtonState::Normal;
        } else {
            if self.left_button_down {
                button.state = ButtonState::Pressed;
            } else {
                button.state = ButtonState::Hovered;
                
                // Fire click event if mouse was just released over button
                if self.left_button_released_this_frame && old_state == ButtonState::Pressed {
                    if let Some(id) = button.on_click_id {
                        ui_event = Some(UIInputEvent::ButtonClicked { button_id: id });
                        
                        // Send to event system
                        let event = Event::new(EventType::ButtonClicked, timestamp)
                            .with_arg("button_id", EventArg::ButtonId(id));
                        event_system.send(event);
                    }
                }
            }
        }
        
        // Fire hover change event
        if old_state != button.state {
            match (old_state, button.state) {
                (ButtonState::Normal, ButtonState::Hovered) |
                (ButtonState::Normal, ButtonState::Pressed) => {
                    if let Some(id) = button.on_click_id {
                        if ui_event.is_none() {
                            ui_event = Some(UIInputEvent::ButtonHoverChanged { 
                                button_id: id, 
                                hovered: true 
                            });
                            
                            // Send to event system
                            let event = Event::new(EventType::ButtonHoverChanged, timestamp)
                                .with_arg("button_id", EventArg::ButtonId(id))
                                .with_arg("hovered", EventArg::Hovered(true));
                            event_system.send(event);
                        }
                    }
                }
                (ButtonState::Hovered, ButtonState::Normal) |
                (ButtonState::Pressed, ButtonState::Normal) => {
                    if let Some(id) = button.on_click_id {
                        if ui_event.is_none() {
                            ui_event = Some(UIInputEvent::ButtonHoverChanged { 
                                button_id: id, 
                                hovered: false 
                            });
                            
                            // Send to event system
                            let event = Event::new(EventType::ButtonHoverChanged, timestamp)
                                .with_arg("button_id", EventArg::ButtonId(id))
                                .with_arg("hovered", EventArg::Hovered(false));
                            event_system.send(event);
                        }
                    }
                }
                _ => {}
            }
        }
        
        ui_event
    }
    
    /// Get current mouse position
    pub fn mouse_position(&self) -> (f32, f32) {
        (self.mouse_x, self.mouse_y)
    }
}
