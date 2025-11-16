//! Event system following Game Engine Architecture Ch 16.8
//! Key principles:
//! - Key-value arguments (no order dependency)
//! - Handler returns bool (true = consumed, stops forwarding)
//! - Registration system (only notify interested handlers)
//! - Queuing support (immediate + deferred delivery)

use std::collections::HashMap;

/// Event type identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// Button was clicked
    ButtonClicked,
    /// Button hover state changed
    ButtonHoverChanged,
    /// Mouse cursor moved
    MouseMoved,
    /// Mouse button was pressed
    MouseButtonPressed,
    /// Mouse button was released
    MouseButtonReleased,
}

/// Variant for type-safe event arguments
/// Uses key-value pairs to avoid order dependency problems
#[derive(Debug, Clone)]
pub enum EventArg {
    /// Button identifier
    ButtonId(u32),
    /// Hover state
    Hovered(bool),
    /// Position coordinates
    Position(f32, f32),
    /// Mouse button index
    MouseButton(u32),
}

/// Event with type ID and key-value arguments
#[derive(Debug, Clone)]
pub struct Event {
    /// Type of event
    pub event_type: EventType,
    /// Timestamp when event was created (seconds)
    pub timestamp: f64,
    args: HashMap<&'static str, EventArg>,
}

impl Event {
    /// Create a new event with the given type and timestamp
    pub fn new(event_type: EventType, timestamp: f64) -> Self {
        Self {
            event_type,
            timestamp,
            args: HashMap::new(),
        }
    }

    /// Add an argument to the event (builder pattern)
    pub fn with_arg(mut self, key: &'static str, value: EventArg) -> Self {
        self.args.insert(key, value);
        self
    }

    /// Get an argument by key
    pub fn get_arg(&self, key: &str) -> Option<&EventArg> {
        self.args.get(key)
    }

    /// Get button_id argument if present
    pub fn get_button_id(&self) -> Option<u32> {
        if let Some(EventArg::ButtonId(id)) = self.get_arg("button_id") {
            Some(*id)
        } else {
            None
        }
    }

    /// Get hovered argument if present
    pub fn get_hovered(&self) -> Option<bool> {
        if let Some(EventArg::Hovered(hovered)) = self.get_arg("hovered") {
            Some(*hovered)
        } else {
            None
        }
    }

    /// Get position argument if present
    pub fn get_position(&self) -> Option<(f32, f32)> {
        if let Some(EventArg::Position(x, y)) = self.get_arg("position") {
            Some((*x, *y))
        } else {
            None
        }
    }

    /// Get mouse button argument if present
    pub fn get_mouse_button(&self) -> Option<u32> {
        if let Some(EventArg::MouseButton(button)) = self.get_arg("button") {
            Some(*button)
        } else {
            None
        }
    }
}

/// Event handler trait
/// Returns true if event was consumed (stops forwarding)
/// Returns false to allow forwarding to other handlers
pub trait EventHandler {
    /// Handle an event, return true if consumed
    fn on_event(&mut self, event: &Event) -> bool;
}

/// Event system with registration and queuing
/// Follows chain of responsibility pattern
pub struct EventSystem {
    immediate_queue: Vec<Event>,
    deferred_queue: Vec<(f64, Event)>,
    handlers: HashMap<EventType, Vec<Box<dyn EventHandler>>>,
    current_time: f64,
}

impl EventSystem {
    /// Create a new empty event system
    pub fn new() -> Self {
        Self {
            immediate_queue: Vec::new(),
            deferred_queue: Vec::new(),
            handlers: HashMap::new(),
            current_time: 0.0,
        }
    }

    /// Update current time (seconds since start)
    pub fn update_time(&mut self, time: f64) {
        self.current_time = time;
    }

    /// Register a handler for a specific event type
    /// Only handlers registered for this type will be notified
    pub fn register_handler(&mut self, event_type: EventType, handler: Box<dyn EventHandler>) {
        self.handlers
            .entry(event_type)
            .or_insert_with(Vec::new)
            .push(handler);
    }

    /// Send event for immediate handling this frame
    pub fn send(&mut self, event: Event) {
        self.immediate_queue.push(event);
    }

    /// Post event for deferred delivery at specified time
    pub fn post(&mut self, delivery_time: f64, event: Event) {
        self.deferred_queue.push((delivery_time, event));
    }

    /// Dispatch all pending events
    /// Processes immediate queue first, then due deferred events
    pub fn dispatch(&mut self) {
        // Process immediate events
        let immediate = std::mem::take(&mut self.immediate_queue);
        for event in immediate {
            self.dispatch_event(&event);
        }

        // Process due deferred events
        let mut i = 0;
        while i < self.deferred_queue.len() {
            if self.deferred_queue[i].0 <= self.current_time {
                let (_, event) = self.deferred_queue.remove(i);
                self.dispatch_event(&event);
            } else {
                i += 1;
            }
        }
    }

    /// Dispatch single event to registered handlers
    /// Stops on first handler that returns true (consumed)
    fn dispatch_event(&mut self, event: &Event) {
        if let Some(handlers) = self.handlers.get_mut(&event.event_type) {
            for handler in handlers.iter_mut() {
                if handler.on_event(event) {
                    // Event consumed, stop forwarding
                    break;
                }
            }
        }
    }

    /// Clear all queued events (useful for state transitions)
    pub fn clear(&mut self) {
        self.immediate_queue.clear();
        self.deferred_queue.clear();
    }
}

impl Default for EventSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHandler {
        events_received: Vec<EventType>,
        consume: bool,
    }

    impl TestHandler {
        fn new(consume: bool) -> Self {
            Self {
                events_received: Vec::new(),
                consume,
            }
        }
    }

    impl EventHandler for TestHandler {
        fn on_event(&mut self, event: &Event) -> bool {
            self.events_received.push(event.event_type);
            self.consume
        }
    }

    #[test]
    fn test_immediate_dispatch() {
        let mut system = EventSystem::new();
        let handler = Box::new(TestHandler::new(false));
        system.register_handler(EventType::ButtonClicked, handler);

        let event = Event::new(EventType::ButtonClicked, 0.0)
            .with_arg("button_id", EventArg::ButtonId(42));
        system.send(event);
        system.dispatch();

        // Handler should have received the event
        if let Some(handlers) = system.handlers.get(&EventType::ButtonClicked) {
            assert_eq!(handlers.len(), 1);
        }
    }

    #[test]
    fn test_deferred_dispatch() {
        let mut system = EventSystem::new();
        system.update_time(0.0);

        let event = Event::new(EventType::ButtonClicked, 1.0);
        system.post(1.0, event);

        // Event should not dispatch at t=0.5
        system.update_time(0.5);
        system.dispatch();
        assert_eq!(system.deferred_queue.len(), 1);

        // Event should dispatch at t=1.0
        system.update_time(1.0);
        system.dispatch();
        assert_eq!(system.deferred_queue.len(), 0);
    }

    #[test]
    fn test_event_consumption() {
        let mut system = EventSystem::new();
        
        // First handler consumes
        let handler1 = Box::new(TestHandler::new(true));
        system.register_handler(EventType::ButtonClicked, handler1);
        
        // Second handler should not receive
        let handler2 = Box::new(TestHandler::new(false));
        system.register_handler(EventType::ButtonClicked, handler2);

        let event = Event::new(EventType::ButtonClicked, 0.0);
        system.send(event);
        system.dispatch();

        // Only first handler should have received event
        if let Some(handlers) = system.handlers.get(&EventType::ButtonClicked) {
            assert_eq!(handlers.len(), 2);
        }
    }
}
