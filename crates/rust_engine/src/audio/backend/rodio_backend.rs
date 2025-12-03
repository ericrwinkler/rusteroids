//! Rodio audio backend implementation
//!
//! Uses the Rodio library for cross-platform audio playback.
//! Rodio is pure Rust and supports WAV, OGG Vorbis, MP3, and FLAC formats.
//!
//! # Example
//!
//! ```no_run
//! use rust_engine::audio::backend::{AudioBackend, AudioBackendConfig};
//! use rust_engine::audio::backend::rodio_backend::RodioBackend;
//!
//! let mut backend = RodioBackend::new();
//! let config = AudioBackendConfig::default();
//! backend.initialize(&config).unwrap();
//!
//! // Play a sound from file
//! let handle = backend.play_sound_from_file("resources/audio/sound.wav").unwrap();
//!
//! // Control playback
//! backend.set_volume(handle, 0.5).unwrap();
//! backend.pause(handle).unwrap();
//! backend.resume(handle).unwrap();
//!
//! // Check status
//! if backend.is_playing(handle) {
//!     println!("Sound is playing!");
//! }
//!
//! // Cleanup
//! backend.update(); // Removes finished sounds
//! backend.shutdown();
//! ```

use super::{AudioBackend, AudioBackendConfig};
use crate::audio::AudioError;
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

/// Sound handle for tracking active sounds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundHandle {
    /// Unique identifier for the sound
    pub id: u32,
    /// Generation counter for handle validation
    pub generation: u32,
}

impl SoundHandle {
    /// Create a new sound handle
    pub fn new(id: u32, generation: u32) -> Self {
        Self { id, generation }
    }
}

/// Rodio-based audio backend
pub struct RodioBackend {
    /// Audio output stream (must be kept alive)
    _output_stream: Option<OutputStream>,
    /// Output stream handle for creating sinks
    stream_handle: Option<OutputStreamHandle>,
    /// Active sound sinks
    active_sounds: HashMap<SoundHandle, Sink>,
    /// Next sound ID for handle generation
    next_id: u32,
    /// Initialization state
    initialized: bool,
}

impl RodioBackend {
    /// Create a new Rodio backend
    pub fn new() -> Self {
        Self {
            _output_stream: None,
            stream_handle: None,
            active_sounds: HashMap::new(),
            next_id: 0,
            initialized: false,
        }
    }
    
    /// Generate a new sound handle
    fn next_handle(&mut self) -> SoundHandle {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        SoundHandle::new(id, 0)
    }
    
    /// Play a sound from an in-memory buffer
    ///
    /// # Arguments
    /// * `data` - Raw audio file bytes (WAV, OGG, MP3, or FLAC format)
    ///
    /// # Returns
    /// A `SoundHandle` that can be used to control the sound, or an error if playback fails.
    ///
    /// # Errors
    /// - `BackendNotInitialized` if the backend hasn't been initialized
    /// - `PlaybackFailed` if the sink creation or audio decoding fails
    ///
    /// # Example
    /// ```no_run
    /// # use rust_engine::audio::backend::{AudioBackend, AudioBackendConfig};
    /// # use rust_engine::audio::backend::rodio_backend::RodioBackend;
    /// let mut backend = RodioBackend::new();
    /// backend.initialize(&AudioBackendConfig::default()).unwrap();
    ///
    /// let audio_data = std::fs::read("sound.wav").unwrap();
    /// let handle = backend.play_sound(&audio_data).unwrap();
    /// ```
    pub fn play_sound(&mut self, data: &[u8]) -> Result<SoundHandle, AudioError> {
        let stream_handle = self.stream_handle.as_ref()
            .ok_or_else(|| AudioError::BackendNotInitialized)?;
        
        // Create sink for this sound
        let sink = Sink::try_new(stream_handle)
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to create sink: {}", e)))?;
        
        // Decode audio data
        let cursor = std::io::Cursor::new(data.to_vec());
        let source = Decoder::new(cursor)
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to decode audio: {}", e)))?;
        
        // Play the sound
        sink.append(source);
        
        // Generate handle and store sink
        let handle = self.next_handle();
        self.active_sounds.insert(handle, sink);
        
        Ok(handle)
    }
    
    /// Play a sound from a file path
    ///
    /// # Arguments
    /// * `path` - Path to an audio file (WAV, OGG, MP3, or FLAC)
    ///
    /// # Returns
    /// A `SoundHandle` that can be used to control the sound, or an error if playback fails.
    ///
    /// # Errors
    /// - `BackendNotInitialized` if the backend hasn't been initialized
    /// - `PlaybackFailed` if file opening, sink creation, or audio decoding fails
    ///
    /// # Example
    /// ```no_run
    /// # use rust_engine::audio::backend::{AudioBackend, AudioBackendConfig};
    /// # use rust_engine::audio::backend::rodio_backend::RodioBackend;
    /// let mut backend = RodioBackend::new();
    /// backend.initialize(&AudioBackendConfig::default()).unwrap();
    ///
    /// let handle = backend.play_sound_from_file("resources/audio/explosion.wav").unwrap();
    /// ```
    pub fn play_sound_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<SoundHandle, AudioError> {
        let stream_handle = self.stream_handle.as_ref()
            .ok_or_else(|| AudioError::BackendNotInitialized)?;
        
        // Create sink for this sound
        let sink = Sink::try_new(stream_handle)
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to create sink: {}", e)))?;
        
        // Open and decode audio file
        let file = File::open(path.as_ref())
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to open audio file: {}", e)))?;
        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| AudioError::PlaybackFailed(format!("Failed to decode audio: {}", e)))?;
        
        // Play the sound
        sink.append(source);
        
        // Generate handle and store sink
        let handle = self.next_handle();
        self.active_sounds.insert(handle, sink);
        
        Ok(handle)
    }
    
    /// Pause a playing sound
    ///
    /// # Arguments
    /// * `handle` - Handle to the sound to pause
    ///
    /// # Errors
    /// - `InvalidHandle` if the handle doesn't correspond to an active sound
    pub fn pause(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        let sink = self.active_sounds.get(&handle)
            .ok_or_else(|| AudioError::InvalidHandle)?;
        sink.pause();
        Ok(())
    }
    
    /// Resume a paused sound
    ///
    /// # Arguments
    /// * `handle` - Handle to the sound to resume
    ///
    /// # Errors
    /// - `InvalidHandle` if the handle doesn't correspond to an active sound
    pub fn resume(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        let sink = self.active_sounds.get(&handle)
            .ok_or_else(|| AudioError::InvalidHandle)?;
        sink.play();
        Ok(())
    }
    
    /// Stop a sound and remove it from active sounds
    ///
    /// # Arguments
    /// * `handle` - Handle to the sound to stop
    ///
    /// # Note
    /// This method succeeds even if the handle is invalid (idempotent)
    pub fn stop(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        if let Some(sink) = self.active_sounds.remove(&handle) {
            sink.stop();
        }
        Ok(())
    }
    
    /// Set the volume of a sound (0.0 = silent, 1.0 = full volume)
    ///
    /// # Arguments
    /// * `handle` - Handle to the sound
    /// * `volume` - Volume level (0.0 to 1.0, values > 1.0 will amplify)
    ///
    /// # Errors
    /// - `InvalidHandle` if the handle doesn't correspond to an active sound
    pub fn set_volume(&mut self, handle: SoundHandle, volume: f32) -> Result<(), AudioError> {
        let sink = self.active_sounds.get(&handle)
            .ok_or_else(|| AudioError::InvalidHandle)?;
        sink.set_volume(volume);
        Ok(())
    }
    
    /// Get the volume of a sound
    ///
    /// # Arguments
    /// * `handle` - Handle to the sound
    ///
    /// # Returns
    /// The current volume level (0.0 to 1.0)
    ///
    /// # Errors
    /// - `InvalidHandle` if the handle doesn't correspond to an active sound
    pub fn get_volume(&self, handle: SoundHandle) -> Result<f32, AudioError> {
        let sink = self.active_sounds.get(&handle)
            .ok_or_else(|| AudioError::InvalidHandle)?;
        Ok(sink.volume())
    }
    
    /// Check if a sound is currently playing
    ///
    /// # Arguments
    /// * `handle` - Handle to the sound
    ///
    /// # Returns
    /// `true` if the sound is playing (not paused and not finished), `false` otherwise
    pub fn is_playing(&self, handle: SoundHandle) -> bool {
        self.active_sounds.get(&handle)
            .map(|sink| !sink.is_paused() && !sink.empty())
            .unwrap_or(false)
    }
}

impl AudioBackend for RodioBackend {
    fn initialize(&mut self, _config: &AudioBackendConfig) -> Result<(), AudioError> {
        if self.initialized {
            return Ok(());
        }
        
        // Create output stream
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::BackendInitFailed(format!("Failed to create audio output: {}", e)))?;
        
        self._output_stream = Some(stream);
        self.stream_handle = Some(stream_handle);
        self.initialized = true;
        
        log::info!("Rodio audio backend initialized");
        Ok(())
    }
    
    fn shutdown(&mut self) {
        if !self.initialized {
            return;
        }
        
        // Stop all sounds
        self.stop_all();
        
        // Drop stream handle and output
        self.stream_handle = None;
        self._output_stream = None;
        self.initialized = false;
        
        log::info!("Rodio audio backend shutdown");
    }
    
    fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    fn update(&mut self) {
        // Remove finished sounds
        self.active_sounds.retain(|_handle, sink| !sink.empty());
    }
    
    fn stop_all(&mut self) {
        for (_handle, sink) in self.active_sounds.drain() {
            sink.stop();
        }
    }
    
    fn play_sound(&mut self, data: &[u8]) -> Result<SoundHandle, AudioError> {
        self.play_sound(data)
    }
    
    fn play_sound_from_path(&mut self, path: &Path) -> Result<SoundHandle, AudioError> {
        self.play_sound_from_file(path)
    }
    
    fn pause(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        self.pause(handle)
    }
    
    fn resume(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        self.resume(handle)
    }
    
    fn stop(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        self.stop(handle)
    }
    
    fn set_volume(&mut self, handle: SoundHandle, volume: f32) -> Result<(), AudioError> {
        self.set_volume(handle, volume)
    }
    
    fn get_volume(&self, handle: SoundHandle) -> Result<f32, AudioError> {
        self.get_volume(handle)
    }
    
    fn is_playing(&self, handle: SoundHandle) -> bool {
        self.is_playing(handle)
    }
}

impl Default for RodioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RodioBackend {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_initialization() {
        let mut backend = RodioBackend::new();
        assert!(!backend.is_initialized());
        
        let config = AudioBackendConfig::default();
        let result = backend.initialize(&config);
        
        // May fail in CI/test environments without audio device
        if result.is_ok() {
            assert!(backend.is_initialized());
            backend.shutdown();
            assert!(!backend.is_initialized());
        }
    }
    
    #[test]
    fn test_handle_generation() {
        let mut backend = RodioBackend::new();
        let handle1 = backend.next_handle();
        let handle2 = backend.next_handle();
        
        assert_ne!(handle1.id, handle2.id);
    }
    
    #[test]
    fn test_double_initialization() {
        let mut backend = RodioBackend::new();
        let config = AudioBackendConfig::default();
        
        if backend.initialize(&config).is_ok() {
            // Second initialization should succeed
            assert!(backend.initialize(&config).is_ok());
            backend.shutdown();
        }
    }
    
    #[test]
    fn test_playback_without_initialization() {
        let mut backend = RodioBackend::new();
        let dummy_data = vec![0u8; 100];
        
        let result = backend.play_sound(&dummy_data);
        assert!(matches!(result, Err(AudioError::BackendNotInitialized)));
    }
    
    #[test]
    fn test_invalid_handle_operations() {
        let mut backend = RodioBackend::new();
        let config = AudioBackendConfig::default();
        
        if backend.initialize(&config).is_ok() {
            let invalid_handle = SoundHandle::new(999, 0);
            
            // All operations on invalid handle should return error
            assert!(matches!(backend.pause(invalid_handle), Err(AudioError::InvalidHandle)));
            assert!(matches!(backend.resume(invalid_handle), Err(AudioError::InvalidHandle)));
            assert!(matches!(backend.set_volume(invalid_handle, 0.5), Err(AudioError::InvalidHandle)));
            assert!(matches!(backend.get_volume(invalid_handle), Err(AudioError::InvalidHandle)));
            assert!(!backend.is_playing(invalid_handle));
            
            backend.shutdown();
        }
    }
    
    #[test]
    fn test_stop_all_clears_sounds() {
        let mut backend = RodioBackend::new();
        let config = AudioBackendConfig::default();
        
        if backend.initialize(&config).is_ok() {
            backend.stop_all();
            assert_eq!(backend.active_sounds.len(), 0);
            backend.shutdown();
        }
    }
    
    #[test]
    fn test_update_removes_finished_sounds() {
        let mut backend = RodioBackend::new();
        let config = AudioBackendConfig::default();
        
        if backend.initialize(&config).is_ok() {
            // Add a stopped sink manually (simulating finished sound)
            let handle = backend.next_handle();
            if let Some(stream_handle) = &backend.stream_handle {
                if let Ok(sink) = Sink::try_new(stream_handle) {
                    sink.stop(); // Immediately stop it
                    backend.active_sounds.insert(handle, sink);
                }
            }
            
            // Update should remove the finished sound
            backend.update();
            assert_eq!(backend.active_sounds.len(), 0);
            
            backend.shutdown();
        }
    }
}
