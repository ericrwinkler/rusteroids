//! Game-specific components

use rust_engine::prelude::*;
use rust_engine::foundation::math::Vec2;

/// Player ship component
#[derive(Debug, Clone)]
pub struct Player {
    /// Current thrust power (0.0 to 1.0)
    pub thrust: f32,
    
    /// Maximum speed
    pub max_speed: f32,
    
    /// Turn speed in radians per second
    pub turn_speed: f32,
    
    /// Whether the player is invulnerable (after respawn)
    pub invulnerable: bool,
    
    /// Invulnerability timer
    pub invulnerability_time: f32,
}

impl Component for Player {}

impl Default for Player {
    fn default() -> Self {
        Self {
            thrust: 0.0,
            max_speed: 300.0,
            turn_speed: 3.0,
            invulnerable: false,
            invulnerability_time: 0.0,
        }
    }
}

/// Asteroid component
#[derive(Debug, Clone)]
pub struct Asteroid {
    /// Asteroid size category
    pub size: AsteroidSize,
    
    /// Rotation speed in radians per second
    pub rotation_speed: f32,
    
    /// Health/hits required to destroy
    pub health: u32,
    
    /// Points awarded when destroyed
    pub points: u32,
}

impl Component for Asteroid {}

/// Asteroid size categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsteroidSize {
    /// Large asteroid (splits into medium)
    Large,
    
    /// Medium asteroid (splits into small)
    Medium,
    
    /// Small asteroid (destroyed completely)
    Small,
}

impl AsteroidSize {
    /// Get the scale factor for this size
    pub fn scale_factor(self) -> f32 {
        match self {
            AsteroidSize::Large => 2.0,
            AsteroidSize::Medium => 1.5,
            AsteroidSize::Small => 1.0,
        }
    }
    
    /// Get the points awarded for destroying this size
    pub fn points(self) -> u32 {
        match self {
            AsteroidSize::Large => 20,
            AsteroidSize::Medium => 50,
            AsteroidSize::Small => 100,
        }
    }
    
    /// Get the next smaller size when split
    pub fn split_into(self) -> Option<AsteroidSize> {
        match self {
            AsteroidSize::Large => Some(AsteroidSize::Medium),
            AsteroidSize::Medium => Some(AsteroidSize::Small),
            AsteroidSize::Small => None,
        }
    }
}

impl Default for Asteroid {
    fn default() -> Self {
        Self {
            size: AsteroidSize::Large,
            rotation_speed: 1.0,
            health: 1,
            points: 20,
        }
    }
}

/// Bullet projectile component
#[derive(Debug, Clone)]
pub struct Bullet {
    /// Remaining lifetime in seconds
    pub lifetime: f32,
    
    /// Damage dealt on impact
    pub damage: u32,
    
    /// Who fired this bullet
    pub owner: BulletOwner,
}

impl Component for Bullet {}

/// Who fired a bullet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BulletOwner {
    /// Player bullet
    Player,
    
    /// Enemy bullet (future use)
    Enemy,
}

impl Default for Bullet {
    fn default() -> Self {
        Self {
            lifetime: 2.0,
            damage: 1,
            owner: BulletOwner::Player,
        }
    }
}

/// Velocity component for physics
#[derive(Debug, Clone)]
pub struct Velocity {
    /// Linear velocity
    pub linear: Vec3,
    
    /// Angular velocity in radians per second
    pub angular: f32,
}

impl Component for Velocity {}

impl Default for Velocity {
    fn default() -> Self {
        Self {
            linear: Vec3::zeros(),
            angular: 0.0,
        }
    }
}

/// Health component
#[derive(Debug, Clone)]
pub struct Health {
    /// Current health
    pub current: u32,
    
    /// Maximum health
    pub max: u32,
}

impl Component for Health {}

impl Health {
    /// Create a new health component
    pub fn new(max_health: u32) -> Self {
        Self {
            current: max_health,
            max: max_health,
        }
    }
    
    /// Take damage
    pub fn take_damage(&mut self, damage: u32) {
        self.current = self.current.saturating_sub(damage);
    }
    
    /// Heal
    pub fn heal(&mut self, amount: u32) {
        self.current = (self.current + amount).min(self.max);
    }
    
    /// Check if dead
    pub fn is_dead(&self) -> bool {
        self.current == 0
    }
    
    /// Check if at full health
    pub fn is_full(&self) -> bool {
        self.current == self.max
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new(1)
    }
}

/// Score component for tracking points
#[derive(Debug, Clone)]
pub struct Score {
    /// Current score
    pub value: u32,
    
    /// Multiplier for bonus points
    pub multiplier: f32,
}

impl Component for Score {}

impl Default for Score {
    fn default() -> Self {
        Self {
            value: 0,
            multiplier: 1.0,
        }
    }
}

impl Score {
    /// Add points to the score
    pub fn add_points(&mut self, points: u32) {
        self.value += (points as f32 * self.multiplier) as u32;
    }
}

/// Collision circle component for simple collision detection
#[derive(Debug, Clone)]
pub struct CollisionCircle {
    /// Collision radius
    pub radius: f32,
    
    /// Collision layers this entity belongs to
    pub layers: CollisionLayers,
    
    /// Collision layers this entity can collide with
    pub mask: CollisionLayers,
}

impl Component for CollisionCircle {}

/// Collision layers (simplified version without bitflags for now)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionLayers(u32);

impl CollisionLayers {
    pub const PLAYER: Self = Self(1 << 0);
    pub const ASTEROIDS: Self = Self(1 << 1);
    pub const BULLETS: Self = Self(1 << 2);
    pub const POWER_UPS: Self = Self(1 << 3);
    pub const ENEMIES: Self = Self(1 << 4);
    
    pub fn empty() -> Self {
        Self(0)
    }
    
    pub fn all() -> Self {
        Self(0xFFFFFFFF)
    }
}

impl Default for CollisionCircle {
    fn default() -> Self {
        Self {
            radius: 1.0,
            layers: CollisionLayers::empty(),
            mask: CollisionLayers::all(),
        }
    }
}

/// Wrap-around component for screen wrapping
#[derive(Debug, Clone)]
pub struct WrapAround {
    /// Screen bounds for wrapping
    pub bounds: ScreenBounds,
}

impl Component for WrapAround {}

/// Screen bounds for wrapping
#[derive(Debug, Clone)]
pub struct ScreenBounds {
    /// Minimum X coordinate
    pub min_x: f32,
    
    /// Maximum X coordinate
    pub max_x: f32,
    
    /// Minimum Y coordinate
    pub min_y: f32,
    
    /// Maximum Y coordinate
    pub max_y: f32,
}

impl Default for ScreenBounds {
    fn default() -> Self {
        Self {
            min_x: -640.0,
            max_x: 640.0,
            min_y: -360.0,
            max_y: 360.0,
        }
    }
}

impl Default for WrapAround {
    fn default() -> Self {
        Self {
            bounds: ScreenBounds::default(),
        }
    }
}
