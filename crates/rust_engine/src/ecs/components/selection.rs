//! Selection state component
//!
//! Tracks whether an entity is currently selected or hovered.

use crate::ecs::Component;

/// Component tracking selection state of an entity
///
/// This component is automatically added by the picking system when an
/// entity is selected or hovered.
///
/// # Examples
/// ```
/// # use rust_engine::ecs::components::SelectionComponent;
/// let mut selection = SelectionComponent::default();
/// selection.selected = true;
/// ```
#[derive(Debug, Clone, Copy)]
pub struct SelectionComponent {
    /// Whether this entity is currently selected
    pub selected: bool,
    
    /// Whether this entity is currently hovered (raycast hit but not clicked)
    pub hovered: bool,
    
    /// Frame when selection state last changed (for animations)
    pub selection_frame: u64,
    
    /// Frame when hover state last changed (for animations)
    pub hover_frame: u64,
}

impl SelectionComponent {
    /// Create a new selection component with default values
    pub fn new() -> Self {
        Self {
            selected: false,
            hovered: false,
            selection_frame: 0,
            hover_frame: 0,
        }
    }

    /// Mark entity as selected
    pub fn select(&mut self, frame: u64) {
        if !self.selected {
            self.selected = true;
            self.selection_frame = frame;
        }
    }

    /// Mark entity as deselected
    pub fn deselect(&mut self, frame: u64) {
        if self.selected {
            self.selected = false;
            self.selection_frame = frame;
        }
    }

    /// Mark entity as hovered
    pub fn hover(&mut self, frame: u64) {
        if !self.hovered {
            self.hovered = true;
            self.hover_frame = frame;
        }
    }

    /// Mark entity as not hovered
    pub fn unhover(&mut self, frame: u64) {
        if self.hovered {
            self.hovered = false;
            self.hover_frame = frame;
        }
    }

    /// Get frames since selection changed
    pub fn frames_since_selection_change(&self, current_frame: u64) -> u64 {
        current_frame.saturating_sub(self.selection_frame)
    }

    /// Get frames since hover changed
    pub fn frames_since_hover_change(&self, current_frame: u64) -> u64 {
        current_frame.saturating_sub(self.hover_frame)
    }

    /// Check if entity is either selected or hovered
    pub fn is_highlighted(&self) -> bool {
        self.selected || self.hovered
    }
}

impl Component for SelectionComponent {}

impl Default for SelectionComponent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_deselect() {
        let mut selection = SelectionComponent::new();
        
        assert!(!selection.selected);
        
        selection.select(100);
        assert!(selection.selected);
        assert_eq!(selection.selection_frame, 100);
        
        selection.deselect(150);
        assert!(!selection.selected);
        assert_eq!(selection.selection_frame, 150);
    }

    #[test]
    fn test_hover_unhover() {
        let mut selection = SelectionComponent::new();
        
        assert!(!selection.hovered);
        
        selection.hover(100);
        assert!(selection.hovered);
        assert_eq!(selection.hover_frame, 100);
        
        selection.unhover(150);
        assert!(!selection.hovered);
        assert_eq!(selection.hover_frame, 150);
    }

    #[test]
    fn test_is_highlighted() {
        let mut selection = SelectionComponent::new();
        
        assert!(!selection.is_highlighted());
        
        selection.selected = true;
        assert!(selection.is_highlighted());
        
        selection.selected = false;
        selection.hovered = true;
        assert!(selection.is_highlighted());
    }
}
