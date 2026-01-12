//! Trail emitter component for creating light trails behind moving objects

use crate::foundation::math::{Vec3, Vec4};
use crate::ecs::component::Component;
use std::collections::VecDeque;

/// Trail segment representing a point in the trail's history
#[derive(Debug, Clone)]
pub struct TrailSegment {
    /// World position of this segment
    pub position: Vec3,
    
    /// Velocity direction at time of creation (for velocity-aligned billboards)
    pub velocity: Vec3,
    
    /// Age of this segment in seconds
    pub age: f32,
    
    /// Width of trail at this segment (can vary along length)
    pub width: f32,
}

impl TrailSegment {
    /// Create a new trail segment
    pub fn new(position: Vec3, velocity: Vec3, width: f32) -> Self {
        Self {
            position,
            velocity,
            age: 0.0,
            width,
        }
    }
    
    /// Update segment age
    pub fn update(&mut self, delta_time: f32) {
        self.age += delta_time;
    }
    
    /// Check if segment has expired
    pub fn is_expired(&self, lifetime: f32) -> bool {
        self.age >= lifetime
    }
    
    /// Calculate fade alpha based on age and lifetime (exponential fade)
    pub fn calculate_alpha(&self, lifetime: f32) -> f32 {
        let t = (self.age / lifetime).clamp(0.0, 1.0);
        (1.0 - t).powf(2.0)
    }
    
    /// Check if segment has faded out enough to remove
    /// Remove segments that are nearly invisible (below 1% alpha) to avoid hard cutoff
    pub fn should_remove(&self, lifetime: f32) -> bool {
        self.calculate_alpha(lifetime) < 0.01
    }
}

/// Component for entities that emit light trails
#[derive(Debug, Clone)]
pub struct TrailEmitterComponent {
    /// Trail segments (position history)
    segments: VecDeque<TrailSegment>,
    
    /// Maximum number of segments to keep
    pub max_segments: usize,
    
    /// Lifetime of each segment in seconds
    pub segment_lifetime: f32,
    
    /// Spawn new segment when moved this distance
    pub spawn_distance_threshold: f32,
    
    /// Trail width in world units
    pub width: f32,
    
    /// Base color of trail (RGBA)
    pub color: Vec4,
    
    /// Position where last segment was spawned
    last_spawn_position: Vec3,
    
    /// Whether trail emission is enabled
    pub enabled: bool,
}

impl TrailEmitterComponent {
    /// Create a new trail emitter with default settings
    pub fn new() -> Self {
        Self {
            segments: VecDeque::with_capacity(32),
            max_segments: 30,
            segment_lifetime: 1.5,
            spawn_distance_threshold: 0.5,
            width: 0.8,
            color: Vec4::new(0.0, 0.8, 1.0, 1.0), // Cyan
            last_spawn_position: Vec3::zeros(),
            enabled: true,
        }
    }
    
    /// Create a trail emitter with custom color
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }
    
    /// Set trail width
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }
    
    /// Configure trail from intuitive parameters: number of segments, distance between segments, and fade lifetime
    /// - num_segments: How many trail segments to maintain
    /// - segment_length: Distance between segments (world units)
    /// - lifetime: How long segments live before fading out (seconds)
    pub fn with_trail_config(mut self, num_segments: usize, segment_length: f32, lifetime: f32) -> Self {
        self.max_segments = num_segments;
        self.spawn_distance_threshold = segment_length;
        self.segment_lifetime = lifetime;
        self
    }
    
    /// Initialize last spawn position (call when component is added)
    pub fn initialize(&mut self, position: Vec3) {
        self.last_spawn_position = position;
        self.segments.clear();
    }
    
    /// Check if new segment should be spawned
    pub fn should_spawn_segment(&self, current_position: Vec3) -> bool {
        if !self.enabled {
            return false;
        }
        
        let distance = (current_position - self.last_spawn_position).norm();
        distance >= self.spawn_distance_threshold
    }
    
    /// Spawn a new trail segment
    pub fn spawn_segment(&mut self, position: Vec3, velocity: Vec3) {
        let segment = TrailSegment::new(position, velocity, self.width);
        self.segments.push_back(segment);
        self.last_spawn_position = position;
    }
    
    /// Update all segments (age them, remove expired)
    pub fn update_segments(&mut self, delta_time: f32) {
        // Age all segments
        for segment in self.segments.iter_mut() {
            segment.update(delta_time);
        }
        
        // Remove segments that have faded to nearly invisible (< 1% alpha)
        // This prevents hard cutoff at the end of trails
        while let Some(segment) = self.segments.front() {
            if segment.should_remove(self.segment_lifetime) {
                self.segments.pop_front();
            } else {
                break; // Segments are in age order
            }
        }
    }
    
    /// Get all segments
    pub fn segments(&self) -> &VecDeque<TrailSegment> {
        &self.segments
    }
    
    /// Clear all segments
    pub fn clear(&mut self) {
        self.segments.clear();
    }
    
    /// Get number of active segments
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

impl Default for TrailEmitterComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for TrailEmitterComponent {}

/// Factory for creating trail emitter components
pub struct TrailEmitterFactory;

impl TrailEmitterFactory {
    /// Create a cyan trail emitter (Tron style)
    pub fn cyan_trail() -> TrailEmitterComponent {
        TrailEmitterComponent::new()
            .with_color(Vec4::new(0.0, 0.8, 1.0, 1.0))
            .with_trail_config(20, 0.5, 1.5)  // 20 segments, 0.5 units apart, 1.5s lifetime
            .with_width(0.8)
    }
    
    /// Create an orange trail emitter (Tron style)
    pub fn orange_trail() -> TrailEmitterComponent {
        TrailEmitterComponent::new()
            .with_color(Vec4::new(1.0, 0.6, 0.0, 1.0))
            .with_trail_config(20, 0.5, 1.5)  // 20 segments, 0.5 units apart, 1.5s lifetime
            .with_width(0.8)
    }
    
    /// Create a white engine exhaust trail
    pub fn engine_exhaust() -> TrailEmitterComponent {
        TrailEmitterComponent::new()
            .with_color(Vec4::new(1.0, 1.0, 1.0, 1.0))
            .with_trail_config(15, 0.3, 1.0)  // 15 segments, 0.3 units apart, 1.0s lifetime
            .with_width(0.5)
    }
    
    /// Create a short, fast-fading trail
    pub fn short_trail() -> TrailEmitterComponent {
        TrailEmitterComponent::new()
            .with_trail_config(15, 0.3, 0.5)  // 15 segments, 0.3 units apart, 0.5s lifetime
    }
}
