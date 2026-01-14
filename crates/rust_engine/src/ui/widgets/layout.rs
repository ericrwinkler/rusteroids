//! UI layout calculations
//!
//! Converts UI component data to screen-space coordinates for rendering.

use crate::foundation::math::Vec2;
#[allow(unused_imports)] // Used in tests
use crate::ui::widgets::{UIElement, Anchor};

/// Layout calculator for UI elements
pub struct UILayout;

impl UILayout {
    /// Calculate screen position from element properties
    ///
    /// # Arguments
    /// * `element` - UI element with position and anchor
    /// * `screen_width` - Window width in pixels
    /// * `screen_height` - Window height in pixels
    ///
    /// # Returns
    /// Screen position in pixels (top-left origin)
    pub fn calculate_position(
        element: &UIElement,
        screen_width: f32,
        screen_height: f32,
    ) -> Vec2 {
        let (anchor_x, anchor_y) = element.anchor.to_normalized();
        
        // Start from anchor point
        let anchor_screen_x = anchor_x * screen_width;
        let anchor_screen_y = anchor_y * screen_height;
        
        // Add element offset
        Vec2::new(
            anchor_screen_x + element.position.0,
            anchor_screen_y + element.position.1,
        )
    }
    
    /// Calculate bounding box for an element
    ///
    /// # Returns
    /// (min_x, min_y, max_x, max_y) in screen pixels
    pub fn calculate_bounds(
        element: &UIElement,
        screen_width: f32,
        screen_height: f32,
    ) -> (f32, f32, f32, f32) {
        let pos = Self::calculate_position(element, screen_width, screen_height);
        
        let min_x = pos.x;
        let min_y = pos.y;
        let max_x = pos.x + element.size.0;
        let max_y = pos.y + element.size.1;
        
        (min_x, min_y, max_x, max_y)
    }
    
    /// Check if a point is inside an element's bounds
    ///
    /// # Deprecated
    /// This method has been moved to `crate::input::collision::point_in_ui_element`.
    /// Hit testing is an input system concern, not a rendering concern.
    #[deprecated(
        since = "0.2.0",
        note = "Use `crate::input::collision::point_in_ui_element` instead. Hit testing belongs in the input system."
    )]
    pub fn contains_point(
        element: &UIElement,
        screen_width: f32,
        screen_height: f32,
        point_x: f32,
        point_y: f32,
    ) -> bool {
        // Delegate to the input system's collision module
        crate::input::collision::point_in_ui_element(
            element,
            screen_width,
            screen_height,
            point_x,
            point_y,
        )
    }
    
    /// Convert screen coordinates to normalized device coordinates (NDC)
    /// NDC range: [-1, 1] where (0, 0) is center
    pub fn screen_to_ndc(
        screen_x: f32,
        screen_y: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (f32, f32) {
        let ndc_x = (screen_x / screen_width) * 2.0 - 1.0;
        let ndc_y = (screen_y / screen_height) * 2.0 - 1.0;
        (ndc_x, ndc_y)
    }
    
    /// Convert pixel size to NDC size
    pub fn size_to_ndc(
        width_pixels: f32,
        height_pixels: f32,
        screen_width: f32,
        screen_height: f32,
    ) -> (f32, f32) {
        let ndc_width = (width_pixels / screen_width) * 2.0;
        let ndc_height = (height_pixels / screen_height) * 2.0;
        (ndc_width, ndc_height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_top_left_anchor() {
        let element = UIElement {
            position: (10.0, 20.0),
            size: (100.0, 50.0),
            anchor: Anchor::TopLeft,
            visible: true,
            z_order: 0,
        };
        
        let pos = UILayout::calculate_position(&element, 800.0, 600.0);
        assert_eq!(pos.x, 10.0);
        assert_eq!(pos.y, 20.0);
    }
    
    #[test]
    fn test_center_anchor() {
        let element = UIElement {
            position: (0.0, 0.0),
            size: (100.0, 50.0),
            anchor: Anchor::Center,
            visible: true,
            z_order: 0,
        };
        
        let pos = UILayout::calculate_position(&element, 800.0, 600.0);
        assert_eq!(pos.x, 400.0);
        assert_eq!(pos.y, 300.0);
    }
    
    #[test]
    fn test_contains_point() {
        use crate::input::collision;
        
        let element = UIElement {
            position: (100.0, 100.0),
            size: (200.0, 100.0),
            anchor: Anchor::TopLeft,
            visible: true,
            z_order: 0,
        };
        
        assert!(collision::point_in_ui_element(&element, 800.0, 600.0, 150.0, 150.0));
        assert!(collision::point_in_ui_element(&element, 800.0, 600.0, 100.0, 100.0));
        assert!(collision::point_in_ui_element(&element, 800.0, 600.0, 300.0, 200.0));
        assert!(!collision::point_in_ui_element(&element, 800.0, 600.0, 50.0, 50.0));
        assert!(!collision::point_in_ui_element(&element, 800.0, 600.0, 350.0, 250.0));
    }
}
