//! Audio backend implementations
//!
//! Platform-independent abstraction over audio playback libraries.

pub mod rodio_backend;

use crate::audio::AudioError;

/// Audio backend trait for platform abstraction
/// 
/// # Threading (Phase 1)
/// Currently NOT Send + Sync as we're using single-threaded architecture.
/// Phase 2 will add proper threading support with command queues and audio thread.
/// See AUDIO_COMPLETE_DESIGN.md Decision 1 for threading roadmap.
pub trait AudioBackend {
    /// Initialize the audio backend
    fn initialize(&mut self, config: &AudioBackendConfig) -> Result<(), AudioError>;
    
    /// Shutdown the audio backend
    fn shutdown(&mut self);
    
    /// Check if backend is initialized
    fn is_initialized(&self) -> bool;
    
    /// Update the backend (cleanup finished sounds, etc.)
    fn update(&mut self);
    
    /// Stop all playing sounds
    fn stop_all(&mut self);
    
    /// Play a sound from memory
    fn play_sound(&mut self, data: &[u8]) -> Result<rodio_backend::SoundHandle, AudioError>;
    
    /// Play a sound from a file path
    fn play_sound_from_path(&mut self, path: &std::path::Path) -> Result<rodio_backend::SoundHandle, AudioError>;
    
    /// Pause a playing sound
    fn pause(&mut self, handle: rodio_backend::SoundHandle) -> Result<(), AudioError>;
    
    /// Resume a paused sound
    fn resume(&mut self, handle: rodio_backend::SoundHandle) -> Result<(), AudioError>;
    
    /// Stop a sound
    fn stop(&mut self, handle: rodio_backend::SoundHandle) -> Result<(), AudioError>;
    
    /// Set volume of a sound
    fn set_volume(&mut self, handle: rodio_backend::SoundHandle, volume: f32) -> Result<(), AudioError>;
    
    /// Get volume of a sound
    fn get_volume(&self, handle: rodio_backend::SoundHandle) -> Result<f32, AudioError>;
    
    /// Check if a sound is playing
    fn is_playing(&self, handle: rodio_backend::SoundHandle) -> bool;
}

/// Configuration for audio backend
#[derive(Debug, Clone)]
pub struct AudioBackendConfig {
    /// Sample rate (e.g., 44100, 48000)
    pub sample_rate: u32,
    /// Number of output channels (1=mono, 2=stereo)
    pub channels: u16,
    /// Buffer size for audio processing
    pub buffer_size: usize,
}

impl Default for AudioBackendConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            buffer_size: 4096,
        }
    }
}

/// Create the default audio backend for the platform
pub fn create_backend(config: AudioBackendConfig) -> Result<Box<dyn AudioBackend>, AudioError> {
    let mut backend = Box::new(rodio_backend::RodioBackend::new());
    backend.initialize(&config)?;
    Ok(backend)
}
