//! Time management utilities

use std::time::{Duration, Instant};

/// High-precision timer for frame timing
pub struct Timer {
    last_frame: Instant,
    delta_time: f32,
    total_time: f32,
    frame_count: u64,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    /// Create a new timer
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            last_frame: now,
            delta_time: 0.0,
            total_time: 0.0,
            frame_count: 0,
        }
    }
    
    /// Update the timer (should be called once per frame)
    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.delta_time = elapsed.as_secs_f32();
        self.total_time += self.delta_time;
        self.last_frame = now;
        self.frame_count += 1;
    }
    
    /// Get the time since the last frame in seconds
    pub fn delta_time(&self) -> f32 {
        self.delta_time
    }
    
    /// Get the total elapsed time since timer creation
    pub fn total_time(&self) -> f32 {
        self.total_time
    }
    
    /// Get the current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
    
    /// Get the average FPS since timer creation
    pub fn average_fps(&self) -> f32 {
        if self.total_time > 0.0 {
            self.frame_count as f32 / self.total_time
        } else {
            0.0
        }
    }
    
    /// Get the current FPS (based on last frame time)
    pub fn current_fps(&self) -> f32 {
        if self.delta_time > 0.0 {
            1.0 / self.delta_time
        } else {
            0.0
        }
    }
}

/// Simple stopwatch for measuring elapsed time
pub struct Stopwatch {
    start_time: Option<Instant>,
    elapsed: Duration,
}

impl Default for Stopwatch {
    fn default() -> Self {
        Self::new()
    }
}

impl Stopwatch {
    /// Create a new stopped stopwatch
    pub fn new() -> Self {
        Self {
            start_time: None,
            elapsed: Duration::ZERO,
        }
    }
    
    /// Create a new stopwatch and start it immediately
    pub fn start_new() -> Self {
        let mut stopwatch = Self::new();
        stopwatch.start();
        stopwatch
    }
    
    /// Start the stopwatch
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }
    
    /// Stop the stopwatch and accumulate elapsed time
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time {
            self.elapsed += start.elapsed();
            self.start_time = None;
        }
    }
    
    /// Reset the stopwatch to zero
    pub fn reset(&mut self) {
        self.start_time = None;
        self.elapsed = Duration::ZERO;
    }
    
    /// Restart the stopwatch (reset and start)
    pub fn restart(&mut self) {
        self.reset();
        self.start();
    }
    
    /// Get the elapsed time
    pub fn elapsed(&self) -> Duration {
        let current_elapsed = if let Some(start) = self.start_time {
            start.elapsed()
        } else {
            Duration::ZERO
        };
        self.elapsed + current_elapsed
    }
    
    /// Get the elapsed time in seconds
    pub fn elapsed_secs(&self) -> f32 {
        self.elapsed().as_secs_f32()
    }
    
    /// Get the elapsed time in milliseconds
    pub fn elapsed_millis(&self) -> f32 {
        self.elapsed().as_secs_f32() * 1000.0
    }
    
    /// Check if the stopwatch is currently running
    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    }
}
