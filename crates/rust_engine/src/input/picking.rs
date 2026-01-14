//! Mouse state for picking operations
//!
//! Provides utilities for converting screen-space coordinates to
//! Normalized Device Coordinates (NDC) for ray casting.

/// Mouse state for picking operations
#[derive(Debug, Clone)]
pub struct MouseState {
    /// Current screen-space X position (pixels)
    pub screen_x: f64,
    /// Current screen-space Y position (pixels)
    pub screen_y: f64,
    /// Window width in pixels
    pub window_width: u32,
    /// Window height in pixels
    pub window_height: u32,
    /// Left mouse button pressed this frame
    pub left_click: bool,
    /// Right mouse button pressed this frame
    pub right_click: bool,
    /// Middle mouse button pressed this frame
    pub middle_click: bool,
    /// Drag start position (None if not dragging)
    pub drag_start: Option<(f64, f64)>,
    /// Whether mouse button is currently held down
    pub button_down: bool,
}

impl MouseState {
    /// Create a new mouse state with default values
    pub fn new(window_width: u32, window_height: u32) -> Self {
        Self {
            screen_x: 0.0,
            screen_y: 0.0,
            window_width,
            window_height,
            left_click: false,
            right_click: false,
            middle_click: false,
            drag_start: None,
            button_down: false,
        }
    }

    /// Convert screen coordinates to Normalized Device Coordinates (NDC)
    ///
    /// NDC range: [-1, 1] where:
    /// - X: -1 = left, +1 = right
    /// - Y: -1 = top, +1 = bottom (following ARCHITECTURE.md viewport convention)
    ///
    /// # Returns
    /// Tuple of (ndc_x, ndc_y) in the range [-1, 1]
    ///
    /// # Examples
    /// ```
    /// # use rust_engine::input::picking::MouseState;
    /// let mouse = MouseState::new(1920, 1080);
    /// // Center of screen should be (0, 0) in NDC
    /// // Top-left should be (-1, -1) in NDC
    /// // Bottom-right should be (1, 1) in NDC
    /// ```
    pub fn screen_to_ndc(&self) -> (f32, f32) {
        let ndc_x = (self.screen_x / self.window_width as f64) as f32 * 2.0 - 1.0;
        let ndc_y = (self.screen_y / self.window_height as f64) as f32 * 2.0 - 1.0;
        (ndc_x, ndc_y)
    }

    /// Update mouse position from window events
    ///
    /// # Arguments
    /// * `x` - Mouse X position in screen space (pixels from left)
    /// * `y` - Mouse Y position in screen space (pixels from top)
    pub fn update_position(&mut self, x: f64, y: f64) {
        self.screen_x = x;
        self.screen_y = y;
    }

    /// Update window size (for NDC conversion)
    ///
    /// This should be called whenever the window is resized to ensure
    /// accurate screen-to-NDC conversion.
    ///
    /// # Arguments
    /// * `width` - New window width in pixels
    /// * `height` - New window height in pixels
    pub fn update_window_size(&mut self, width: u32, height: u32) {
        self.window_width = width;
        self.window_height = height;
    }

    /// Set left mouse button state
    pub fn set_left_click(&mut self, clicked: bool) {
        self.left_click = clicked;
    }

    /// Set right mouse button state
    pub fn set_right_click(&mut self, clicked: bool) {
        self.right_click = clicked;
    }

    /// Set middle mouse button state
    pub fn set_middle_click(&mut self, clicked: bool) {
        self.middle_click = clicked;
    }

    /// Clear all click states (call at end of frame)
    pub fn clear_clicks(&mut self) {
        self.left_click = false;
        self.right_click = false;
        self.middle_click = false;
    }

    /// Start a drag operation at current mouse position
    pub fn start_drag(&mut self) {
        self.drag_start = Some((self.screen_x, self.screen_y));
        self.button_down = true;
    }

    /// End drag operation
    pub fn end_drag(&mut self) {
        self.drag_start = None;
        self.button_down = false;
    }

    /// Check if currently dragging (threshold: 5 pixels to distinguish from click)
    pub fn is_dragging(&self) -> bool {
        const DRAG_THRESHOLD: f64 = 5.0;
        if let Some((start_x, start_y)) = self.drag_start {
            if self.button_down {
                let dx = self.screen_x - start_x;
                let dy = self.screen_y - start_y;
                let distance = (dx * dx + dy * dy).sqrt();
                return distance >= DRAG_THRESHOLD;
            }
        }
        false
    }

    /// Get drag box in screen space (min, max corners)
    pub fn get_drag_box(&self) -> Option<((f64, f64), (f64, f64))> {
        if let Some((start_x, start_y)) = self.drag_start {
            if self.button_down {
                let min_x = start_x.min(self.screen_x);
                let max_x = start_x.max(self.screen_x);
                let min_y = start_y.min(self.screen_y);
                let max_y = start_y.max(self.screen_y);
                return Some(((min_x, min_y), (max_x, max_y)));
            }
        }
        None
    }
}

impl Default for MouseState {
    fn default() -> Self {
        Self::new(1920, 1080)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_to_ndc_center() {
        let mouse = MouseState {
            screen_x: 960.0,
            screen_y: 540.0,
            window_width: 1920,
            window_height: 1080,
            left_click: false,
            right_click: false,
            middle_click: false,
        };

        let (ndc_x, ndc_y) = mouse.screen_to_ndc();
        assert!((ndc_x - 0.0).abs() < 0.001);
        assert!((ndc_y - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_screen_to_ndc_corners() {
        let mouse = MouseState {
            screen_x: 0.0,
            screen_y: 0.0,
            window_width: 1920,
            window_height: 1080,
            left_click: false,
            right_click: false,
            middle_click: false,
        };

        let (ndc_x, ndc_y) = mouse.screen_to_ndc();
        assert!((ndc_x - (-1.0)).abs() < 0.001); // Left edge
        assert!((ndc_y - (-1.0)).abs() < 0.001); // Top edge
    }
}
