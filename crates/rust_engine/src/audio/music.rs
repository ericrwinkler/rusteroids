//! Music system
//!
//! Manages background music playback with crossfading and playlist support.

use crate::audio::backend::AudioBackend;
use crate::audio::SoundHandle;
use std::path::PathBuf;

/// Music track metadata and playback configuration
#[derive(Debug, Clone)]
pub struct MusicTrack {
    /// Unique identifier for this track
    pub id: String,
    
    /// Path to the music file
    pub path: PathBuf,
    
    /// Intro section duration in seconds (plays once, then loops main section)
    /// If None, the entire track loops
    pub intro_duration: Option<f32>,
    
    /// Default crossfade duration when transitioning to/from this track
    pub default_fade_duration: f32,
    
    /// Volume multiplier for this specific track (0.0 to 1.0)
    pub volume: f32,
}

impl MusicTrack {
    /// Create a new music track
    pub fn new<S: Into<String>, P: Into<PathBuf>>(id: S, path: P) -> Self {
        Self {
            id: id.into(),
            path: path.into(),
            intro_duration: None,
            default_fade_duration: 2.0, // 2 second default fade
            volume: 1.0,
        }
    }
    
    /// Set intro duration for intro/loop structure
    pub fn with_intro(mut self, intro_duration: f32) -> Self {
        self.intro_duration = Some(intro_duration);
        self
    }
    
    /// Set default fade duration
    pub fn with_fade_duration(mut self, duration: f32) -> Self {
        self.default_fade_duration = duration;
        self
    }
    
    /// Set track volume multiplier
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }
}

/// Music playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicState {
    /// No music is playing
    Stopped,
    
    /// Music is playing normally
    Playing,
    
    /// Music is paused
    Paused,
    
    /// Crossfading between two tracks
    Crossfading,
}

/// Internal track playback info
#[derive(Debug, Clone)]
struct PlayingTrack {
    track: MusicTrack,
    handle: SoundHandle,
    volume: f32, // Current volume (for fading)
    playback_time: f32, // Time since track started
}

/// Music system managing background music playback
pub struct MusicSystem {
    /// Current playing track
    current: Option<PlayingTrack>,
    
    /// Track being faded in during crossfade
    fading_in: Option<PlayingTrack>,
    
    /// Current playback state
    state: MusicState,
    
    /// Crossfade progress (0.0 to 1.0)
    fade_progress: f32,
    
    /// Current fade duration
    fade_duration: f32,
}

impl MusicSystem {
    /// Create a new music system
    pub fn new() -> Self {
        Self {
            current: None,
            fading_in: None,
            state: MusicState::Stopped,
            fade_progress: 0.0,
            fade_duration: 2.0,
        }
    }
    
    /// Play a music track
    /// 
    /// # Arguments
    /// * `track` - The track to play
    /// * `backend` - Audio backend for playback
    /// * `crossfade_duration` - Optional crossfade duration (uses track default if None)
    pub fn play(&mut self, track: MusicTrack, backend: &mut dyn AudioBackend, crossfade_duration: Option<f32>) -> Result<(), String> {
        let fade_duration = crossfade_duration.unwrap_or(track.default_fade_duration);
        
        // Start playing the new track
        let handle = backend.play_sound_from_path(&track.path)
            .map_err(|e| format!("Failed to play music: {}", e))?;
        
        let new_track = PlayingTrack {
            track: track.clone(),
            handle,
            volume: if self.current.is_some() { 0.0 } else { track.volume },
            playback_time: 0.0,
        };
        
        // If something is already playing, start crossfade
        if let Some(current) = self.current.take() {
            self.current = Some(current);
            self.fading_in = Some(new_track);
            self.state = MusicState::Crossfading;
            self.fade_progress = 0.0;
            self.fade_duration = fade_duration;
        } else {
            // No crossfade needed, just start playing
            backend.set_volume(handle, track.volume)
                .map_err(|e| format!("Failed to set volume: {}", e))?;
            self.current = Some(new_track);
            self.state = MusicState::Playing;
        }
        
        Ok(())
    }
    
    /// Pause the current music
    pub fn pause(&mut self, backend: &mut dyn AudioBackend) -> Result<(), String> {
        if self.state == MusicState::Playing {
            if let Some(ref current) = self.current {
                backend.pause(current.handle)
                    .map_err(|e| format!("Failed to pause: {}", e))?;
                self.state = MusicState::Paused;
            }
        }
        Ok(())
    }
    
    /// Resume paused music
    pub fn resume(&mut self, backend: &mut dyn AudioBackend) -> Result<(), String> {
        if self.state == MusicState::Paused {
            if let Some(ref current) = self.current {
                backend.resume(current.handle)
                    .map_err(|e| format!("Failed to resume: {}", e))?;
                self.state = MusicState::Playing;
            }
        }
        Ok(())
    }
    
    /// Stop all music with optional fade out
    pub fn stop(&mut self, backend: &mut dyn AudioBackend, fade_out_duration: Option<f32>) -> Result<(), String> {
        if let Some(_fade_duration) = fade_out_duration {
            // TODO: Implement fade out
            // For now, just stop immediately
            self.stop_immediate(backend)?;
        } else {
            self.stop_immediate(backend)?;
        }
        Ok(())
    }
    
    /// Stop music immediately without fade
    fn stop_immediate(&mut self, backend: &mut dyn AudioBackend) -> Result<(), String> {
        if let Some(current) = self.current.take() {
            backend.stop(current.handle)
                .map_err(|e| format!("Failed to stop: {}", e))?;
        }
        if let Some(fading) = self.fading_in.take() {
            backend.stop(fading.handle)
                .map_err(|e| format!("Failed to stop: {}", e))?;
        }
        self.state = MusicState::Stopped;
        self.fade_progress = 0.0;
        Ok(())
    }
    
    /// Update the music system (call once per frame)
    /// 
    /// # Arguments
    /// * `delta_time` - Time since last frame in seconds
    /// * `backend` - Audio backend for volume control
    /// * `mixer_volume` - Volume multiplier from mixer (Music group volume)
    pub fn update(&mut self, delta_time: f32, backend: &mut dyn AudioBackend, mixer_volume: f32) {
        match self.state {
            MusicState::Playing => {
                // Update playback time
                if let Some(ref mut current) = self.current {
                    current.playback_time += delta_time;
                    
                    // Apply mixer volume
                    let final_volume = current.track.volume * mixer_volume;
                    let _ = backend.set_volume(current.handle, final_volume);
                    
                    // Check if track finished (for looping)
                    if !backend.is_playing(current.handle) {
                        // Track ended, could implement looping here
                        self.state = MusicState::Stopped;
                        self.current = None;
                    }
                }
            }
            MusicState::Crossfading => {
                // Update fade progress
                self.fade_progress += delta_time / self.fade_duration;
                
                if self.fade_progress >= 1.0 {
                    // Crossfade complete
                    if let Some(old) = self.current.take() {
                        let _ = backend.stop(old.handle);
                    }
                    self.current = self.fading_in.take();
                    self.state = MusicState::Playing;
                    self.fade_progress = 0.0;
                } else {
                    // Update volumes during crossfade
                    let fade = self.fade_progress.clamp(0.0, 1.0);
                    
                    // Fade out current track
                    if let Some(ref mut current) = self.current {
                        current.volume = (1.0 - fade) * current.track.volume;
                        let final_volume = current.volume * mixer_volume;
                        let _ = backend.set_volume(current.handle, final_volume);
                    }
                    
                    // Fade in new track
                    if let Some(ref mut fading) = self.fading_in {
                        fading.volume = fade * fading.track.volume;
                        let final_volume = fading.volume * mixer_volume;
                        let _ = backend.set_volume(fading.handle, final_volume);
                        fading.playback_time += delta_time;
                    }
                }
            }
            MusicState::Paused | MusicState::Stopped => {
                // Nothing to update
            }
        }
    }
    
    /// Get current playback state
    pub fn state(&self) -> MusicState {
        self.state
    }
    
    /// Check if music is currently playing
    pub fn is_playing(&self) -> bool {
        self.state == MusicState::Playing || self.state == MusicState::Crossfading
    }
    
    /// Get the currently playing track ID
    pub fn current_track(&self) -> Option<&str> {
        self.current.as_ref().map(|t| t.track.id.as_str())
    }
    
    /// Get current playback time in seconds
    pub fn playback_time(&self) -> f32 {
        self.current.as_ref().map(|t| t.playback_time).unwrap_or(0.0)
    }
}

impl Default for MusicSystem {
    fn default() -> Self {
        Self::new()
    }
}

