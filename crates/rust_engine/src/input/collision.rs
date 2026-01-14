//! Collision detection for input processing
//!
//! Hit testing and spatial queries for UI elements and game objects.
//! This is input system responsibility - determining what the user clicked on.

use crate::ui::widgets::{UIElement, UILayout};

/// Check if a point is inside a rectangular UI element's bounds
///
/// # Arguments
/// * `element` - UI element with position, size, and anchor
/// * `screen_width` - Window width in pixels
/// * `screen_height` - Window height in pixels
/// * `point_x` - X coordinate of the point in screen pixels
/// * `point_y` - Y coordinate of the point in screen pixels
///
/// # Returns
/// `true` if the point is inside the element's bounds
pub fn point_in_ui_element(
    element: &UIElement,
    screen_width: f32,
    screen_height: f32,
    point_x: f32,
    point_y: f32,
) -> bool {
    // Calculate the element's bounding box using the layout system
    let (min_x, min_y, max_x, max_y) = calculate_ui_element_bounds(
        element,
        screen_width,
        screen_height,
    );
    
    point_x >= min_x && point_x <= max_x && 
    point_y >= min_y && point_y <= max_y
}

/// Check if a point is inside a rectangular region
///
/// # Arguments
/// * `point_x` - X coordinate of the point
/// * `point_y` - Y coordinate of the point
/// * `rect_x` - X coordinate of rectangle's top-left corner
/// * `rect_y` - Y coordinate of rectangle's top-left corner
/// * `rect_width` - Width of the rectangle
/// * `rect_height` - Height of the rectangle
///
/// # Returns
/// `true` if the point is inside the rectangle
pub fn point_in_rect(
    point_x: f32,
    point_y: f32,
    rect_x: f32,
    rect_y: f32,
    rect_width: f32,
    rect_height: f32,
) -> bool {
    point_x >= rect_x && 
    point_x <= rect_x + rect_width && 
    point_y >= rect_y && 
    point_y <= rect_y + rect_height
}

/// Check if a point is inside a circle
///
/// # Arguments
/// * `point_x` - X coordinate of the point
/// * `point_y` - Y coordinate of the point
/// * `circle_x` - X coordinate of circle's center
/// * `circle_y` - Y coordinate of circle's center
/// * `radius` - Radius of the circle
///
/// # Returns
/// `true` if the point is inside the circle
pub fn point_in_circle(
    point_x: f32,
    point_y: f32,
    circle_x: f32,
    circle_y: f32,
    radius: f32,
) -> bool {
    let dx = point_x - circle_x;
    let dy = point_y - circle_y;
    let distance_squared = dx * dx + dy * dy;
    distance_squared <= radius * radius
}

/// Calculate the bounding box of a UI element in screen coordinates
///
/// # Arguments
/// * `element` - UI element with position, size, and anchor
/// * `screen_width` - Window width in pixels
/// * `screen_height` - Window height in pixels
///
/// # Returns
/// Tuple of (min_x, min_y, max_x, max_y) in screen pixels
fn calculate_ui_element_bounds(
    element: &UIElement,
    screen_width: f32,
    screen_height: f32,
) -> (f32, f32, f32, f32) {
    // Use the layout system to calculate position (handles anchors)
    let pos = UILayout::calculate_position(element, screen_width, screen_height);
    
    let min_x = pos.x;
    let min_y = pos.y;
    let max_x = pos.x + element.size.0;
    let max_y = pos.y + element.size.1;
    
    (min_x, min_y, max_x, max_y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::Anchor;
    
    #[test]
    fn test_point_in_rect() {
        // Point inside
        assert!(point_in_rect(150.0, 150.0, 100.0, 100.0, 200.0, 100.0));
        
        // Point on edge
        assert!(point_in_rect(100.0, 100.0, 100.0, 100.0, 200.0, 100.0));
        assert!(point_in_rect(300.0, 200.0, 100.0, 100.0, 200.0, 100.0));
        
        // Point outside
        assert!(!point_in_rect(50.0, 50.0, 100.0, 100.0, 200.0, 100.0));
        assert!(!point_in_rect(350.0, 250.0, 100.0, 100.0, 200.0, 100.0));
    }
    
    #[test]
    fn test_point_in_circle() {
        // Point at center
        assert!(point_in_circle(100.0, 100.0, 100.0, 100.0, 50.0));
        
        // Point inside
        assert!(point_in_circle(110.0, 110.0, 100.0, 100.0, 50.0));
        
        // Point on edge
        assert!(point_in_circle(150.0, 100.0, 100.0, 100.0, 50.0));
        
        // Point outside
        assert!(!point_in_circle(200.0, 200.0, 100.0, 100.0, 50.0));
    }
    
    #[test]
    fn test_point_in_ui_element() {
        let element = UIElement {
            position: (100.0, 100.0),
            size: (200.0, 100.0),
            anchor: Anchor::TopLeft,
            visible: true,
            z_order: 0,
        };
        
        // Point inside
        assert!(point_in_ui_element(&element, 800.0, 600.0, 150.0, 150.0));
        
        // Point on edge
        assert!(point_in_ui_element(&element, 800.0, 600.0, 100.0, 100.0));
        assert!(point_in_ui_element(&element, 800.0, 600.0, 300.0, 200.0));
        
        // Point outside
        assert!(!point_in_ui_element(&element, 800.0, 600.0, 50.0, 50.0));
        assert!(!point_in_ui_element(&element, 800.0, 600.0, 350.0, 250.0));
    }
    
    #[test]
    fn test_point_in_ui_element_with_center_anchor() {
        let element = UIElement {
            position: (0.0, 0.0),
            size: (100.0, 50.0),
            anchor: Anchor::Center,
            visible: true,
            z_order: 0,
        };
        
        // Element should be centered at (400, 300) with size 100x50
        // So bounds are (400, 300) to (500, 350)
        assert!(point_in_ui_element(&element, 800.0, 600.0, 450.0, 325.0));
        assert!(!point_in_ui_element(&element, 800.0, 600.0, 100.0, 100.0));
    }
}
