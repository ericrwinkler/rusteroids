//! Lifecycle component for managing entity lifespan and state
//!
//! This component handles entity creation, destruction, and state management
//! for dynamic scene objects.

use crate::ecs::Component;

/// Current state of an entity's lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityState {
    /// Entity is being created/initialized
    Spawning,
    /// Entity is active and updating normally
    Active,
    /// Entity is being destroyed but still exists
    Destroying,
    /// Entity is paused (no updates)
    Paused,
    /// Entity is disabled (invisible and no updates)
    Disabled,
}

/// Component for managing entity lifecycle and automatic cleanup
#[derive(Debug, Clone)]
pub struct LifecycleComponent {
    /// Time when entity was created (in seconds)
    pub spawn_time: f32,
    
    /// How long entity should live (None = permanent)
    pub lifetime: Option<f32>,
    
    /// Current state of the entity
    pub state: EntityState,
    
    /// Time spent in current state
    pub state_time: f32,
    
    /// Whether to auto-destroy when lifetime expires
    pub auto_destroy: bool,
    
    /// User-defined tags for this entity
    pub tags: Vec<String>,
    
    /// Priority for destruction order (higher = destroyed first)
    pub destruction_priority: u8,
}

impl LifecycleComponent {
    /// Create a new lifecycle component
    pub fn new(spawn_time: f32) -> Self {
        Self {
            spawn_time,
            lifetime: None,
            state: EntityState::Spawning,
            state_time: 0.0,
            auto_destroy: true,
            tags: Vec::new(),
            destruction_priority: 0,
        }
    }
    
    /// Create a permanent entity (no automatic destruction)
    pub fn new_permanent(spawn_time: f32) -> Self {
        Self {
            spawn_time,
            lifetime: None,
            state: EntityState::Active,
            state_time: 0.0,
            auto_destroy: false,
            tags: Vec::new(),
            destruction_priority: 0,
        }
    }
    
    /// Create an entity with a specific lifetime
    pub fn new_with_lifetime(spawn_time: f32, lifetime: f32) -> Self {
        Self {
            spawn_time,
            lifetime: Some(lifetime),
            state: EntityState::Spawning,
            state_time: 0.0,
            auto_destroy: true,
            tags: Vec::new(),
            destruction_priority: 0,
        }
    }
    
    /// Update the lifecycle (call every frame)
    pub fn update(&mut self, current_time: f32, delta_time: f32) {
        self.state_time += delta_time;
        
        // Check for automatic state transitions
        match self.state {
            EntityState::Spawning => {
                // Automatically transition to active after one frame
                if self.state_time > 0.0 {
                    self.set_state(EntityState::Active);
                }
            }
            EntityState::Active => {
                // Check if lifetime has expired
                if let Some(lifetime) = self.lifetime {
                    let age = current_time - self.spawn_time;
                    if age >= lifetime && self.auto_destroy {
                        self.set_state(EntityState::Destroying);
                    }
                }
            }
            EntityState::Destroying => {
                // Entity should be removed by the system
            }
            EntityState::Paused | EntityState::Disabled => {
                // No automatic transitions
            }
        }
    }
    
    /// Set the entity state
    pub fn set_state(&mut self, new_state: EntityState) {
        if self.state != new_state {
            self.state = new_state;
            self.state_time = 0.0;
        }
    }
    
    /// Get current age of the entity
    pub fn get_age(&self, current_time: f32) -> f32 {
        current_time - self.spawn_time
    }
    
    /// Get remaining lifetime (None if permanent)
    pub fn get_remaining_lifetime(&self, current_time: f32) -> Option<f32> {
        self.lifetime.map(|lifetime| {
            let age = self.get_age(current_time);
            (lifetime - age).max(0.0)
        })
    }
    
    /// Check if entity should be destroyed
    pub fn should_destroy(&self) -> bool {
        self.state == EntityState::Destroying
    }
    
    /// Check if entity is active (should update)
    pub fn is_active(&self) -> bool {
        matches!(self.state, EntityState::Active | EntityState::Spawning)
    }
    
    /// Check if entity is visible (should render)
    pub fn is_visible(&self) -> bool {
        matches!(self.state, EntityState::Active | EntityState::Spawning | EntityState::Paused)
    }
    
    /// Add a tag to this entity
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
    
    /// Remove a tag from this entity
    pub fn remove_tag(&mut self, tag: &str) {
        self.tags.retain(|t| t != tag);
    }
    
    /// Check if entity has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
    
    /// Set lifetime
    pub fn set_lifetime(&mut self, lifetime: Option<f32>) {
        self.lifetime = lifetime;
    }
    
    /// Set destruction priority
    pub fn set_destruction_priority(&mut self, priority: u8) {
        self.destruction_priority = priority;
    }
    
    /// Mark for destruction
    pub fn destroy(&mut self) {
        self.set_state(EntityState::Destroying);
    }
    
    /// Pause the entity
    pub fn pause(&mut self) {
        if self.is_active() {
            self.set_state(EntityState::Paused);
        }
    }
    
    /// Resume the entity
    pub fn resume(&mut self) {
        if self.state == EntityState::Paused {
            self.set_state(EntityState::Active);
        }
    }
    
    /// Disable the entity
    pub fn disable(&mut self) {
        self.set_state(EntityState::Disabled);
    }
    
    /// Enable the entity
    pub fn enable(&mut self) {
        if self.state == EntityState::Disabled {
            self.set_state(EntityState::Active);
        }
    }
}

impl Component for LifecycleComponent {}

/// Factory for creating lifecycle components with common patterns
pub struct LifecycleFactory;

impl LifecycleFactory {
    /// Create a permanent entity
    pub fn create_permanent(spawn_time: f32) -> LifecycleComponent {
        LifecycleComponent::new_permanent(spawn_time)
    }
    
    /// Create a temporary entity that auto-destructs
    pub fn create_temporary(spawn_time: f32, lifetime: f32) -> LifecycleComponent {
        LifecycleComponent::new_with_lifetime(spawn_time, lifetime)
    }
    
    /// Create an entity for effects (short lifetime, high destruction priority)
    pub fn create_effect(spawn_time: f32, lifetime: f32) -> LifecycleComponent {
        let mut lifecycle = LifecycleComponent::new_with_lifetime(spawn_time, lifetime);
        lifecycle.set_destruction_priority(255); // Highest priority
        lifecycle.add_tag("effect".to_string());
        lifecycle
    }
    
    /// Create an entity for projectiles
    pub fn create_projectile(spawn_time: f32, lifetime: f32) -> LifecycleComponent {
        let mut lifecycle = LifecycleComponent::new_with_lifetime(spawn_time, lifetime);
        lifecycle.set_destruction_priority(128);
        lifecycle.add_tag("projectile".to_string());
        lifecycle
    }
    
    /// Create an entity for game objects (medium priority)
    pub fn create_game_object(spawn_time: f32, lifetime: Option<f32>) -> LifecycleComponent {
        let mut lifecycle = if let Some(lt) = lifetime {
            LifecycleComponent::new_with_lifetime(spawn_time, lt)
        } else {
            LifecycleComponent::new_permanent(spawn_time)
        };
        lifecycle.set_destruction_priority(64);
        lifecycle.add_tag("game_object".to_string());
        lifecycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_component_creation() {
        let spawn_time = 10.0;
        let lifecycle = LifecycleComponent::new(spawn_time);
        
        assert_eq!(lifecycle.spawn_time, spawn_time);
        assert_eq!(lifecycle.lifetime, None);
        assert_eq!(lifecycle.state, EntityState::Spawning);
        assert!(lifecycle.auto_destroy);
    }

    #[test]
    fn test_lifetime_management() {
        let spawn_time = 5.0;
        let lifetime = 3.0;
        let mut lifecycle = LifecycleComponent::new_with_lifetime(spawn_time, lifetime);
        
        assert_eq!(lifecycle.lifetime, Some(lifetime));
        
        // Should be active after first update
        lifecycle.update(5.1, 0.1);
        assert_eq!(lifecycle.state, EntityState::Active);
        
        // Should start destroying after lifetime expires
        lifecycle.update(8.5, 0.1); // Age = 3.5, lifetime = 3.0
        assert_eq!(lifecycle.state, EntityState::Destroying);
    }

    #[test]
    fn test_age_calculation() {
        let spawn_time = 2.0;
        let lifecycle = LifecycleComponent::new(spawn_time);
        
        let current_time = 7.0;
        assert_eq!(lifecycle.get_age(current_time), 5.0);
    }

    #[test]
    fn test_remaining_lifetime() {
        let spawn_time = 1.0;
        let lifetime = 5.0;
        let lifecycle = LifecycleComponent::new_with_lifetime(spawn_time, lifetime);
        
        let current_time = 4.0; // Age = 3.0
        let remaining = lifecycle.get_remaining_lifetime(current_time);
        assert_eq!(remaining, Some(2.0));
        
        // Test expired lifetime
        let current_time = 7.0; // Age = 6.0
        let remaining = lifecycle.get_remaining_lifetime(current_time);
        assert_eq!(remaining, Some(0.0));
    }

    #[test]
    fn test_state_checks() {
        let mut lifecycle = LifecycleComponent::new_permanent(0.0);
        lifecycle.set_state(EntityState::Active);
        
        assert!(lifecycle.is_active());
        assert!(lifecycle.is_visible());
        assert!(!lifecycle.should_destroy());
        
        lifecycle.pause();
        assert!(!lifecycle.is_active());
        assert!(lifecycle.is_visible());
        
        lifecycle.disable();
        assert!(!lifecycle.is_active());
        assert!(!lifecycle.is_visible());
        
        lifecycle.destroy();
        assert!(lifecycle.should_destroy());
    }

    #[test]
    fn test_tags() {
        let mut lifecycle = LifecycleComponent::new(0.0);
        
        assert!(!lifecycle.has_tag("test"));
        
        lifecycle.add_tag("test".to_string());
        assert!(lifecycle.has_tag("test"));
        
        lifecycle.add_tag("another".to_string());
        assert!(lifecycle.has_tag("test"));
        assert!(lifecycle.has_tag("another"));
        
        lifecycle.remove_tag("test");
        assert!(!lifecycle.has_tag("test"));
        assert!(lifecycle.has_tag("another"));
    }

    #[test]
    fn test_factory_methods() {
        let spawn_time = 1.0;
        
        // Test permanent creation
        let permanent = LifecycleFactory::create_permanent(spawn_time);
        assert_eq!(permanent.lifetime, None);
        assert!(!permanent.auto_destroy);
        
        // Test temporary creation
        let temporary = LifecycleFactory::create_temporary(spawn_time, 5.0);
        assert_eq!(temporary.lifetime, Some(5.0));
        assert!(temporary.auto_destroy);
        
        // Test effect creation
        let effect = LifecycleFactory::create_effect(spawn_time, 1.0);
        assert_eq!(effect.destruction_priority, 255);
        assert!(effect.has_tag("effect"));
        
        // Test projectile creation
        let projectile = LifecycleFactory::create_projectile(spawn_time, 3.0);
        assert_eq!(projectile.destruction_priority, 128);
        assert!(projectile.has_tag("projectile"));
    }

    #[test]
    fn test_state_transitions() {
        let mut lifecycle = LifecycleComponent::new(0.0);
        
        assert_eq!(lifecycle.state, EntityState::Spawning);
        assert_eq!(lifecycle.state_time, 0.0);
        
        lifecycle.update(0.1, 0.1);
        assert_eq!(lifecycle.state, EntityState::Active);
        assert_eq!(lifecycle.state_time, 0.0); // Reset on state change
    }
}
