//! Audio system for game engine
//!
//! Provides 2D and spatial audio playback with voice management, mixing,
//! and music systems suitable for arcade and RTS-style games.
//!
//! # Architecture
//!
//! The audio system is composed of several subsystems:
//! - **Backend**: Platform abstraction for audio playback
//! - **Sound Manager**: Asset loading and caching
//! - **Voice Manager**: Voice allocation and pooling
//! - **Mixer**: Volume groups and mixing control
//! - **Music System**: Background music with crossfading
//! - **Spatial Audio**: Camera-relative positioning for RTS gameplay
//!
//! # Example Usage
//!
//! ```ignore
//! // Initialize audio system
//! let mut audio = AudioSystem::new()?;
//!
//! // Load sound bank
//! audio.load_sound_bank("sounds/sfx_common.bank")?;
//!
//! // Play a sound
//! audio.play_sound("laser_fire");
//!
//! // Play spatial sound
//! audio.play_sound_at("explosion", Vec2::new(100.0, 200.0));
//!
//! // Control volume
//! audio.set_group_volume(VolumeGroup::SFX, 0.7);
//!
//! // Play music with crossfade
//! audio.play_music("battle_theme", CrossfadeTime::seconds(2.0));
//! ```

// Module declarations
pub mod asset;
pub mod backend;
pub mod mixer;
pub mod music;
pub mod sound_manager;
pub mod spatial;
pub mod voice_manager;

// Re-exports
pub use asset::{AudioAsset, AudioFormat};
pub use backend::{AudioBackend, AudioBackendConfig};
pub use mixer::{MixerSystem, VolumeGroup};
pub use music::{MusicState, MusicSystem, MusicTrack};
pub use sound_manager::{SoundBank, SoundId, SoundManager};
pub use spatial::{SpatialAudio, SpatialConfig};
pub use voice_manager::{VoiceHandle, VoiceManager, VoicePriority};

// Re-export sound handle from backend
pub use backend::rodio_backend::SoundHandle;

use crate::foundation::math::Vec2;
use std::path::Path;

/// Main audio system coordinating all audio subsystems
pub struct AudioSystem {
    backend: Box<dyn AudioBackend>,
    sound_manager: SoundManager,
    voice_manager: VoiceManager,
    mixer: MixerSystem,
    music: MusicSystem,
    spatial: Option<SpatialAudio>,
    enabled: bool,
}

impl AudioSystem {
    /// Create a new audio system with default configuration
    pub fn new() -> Result<Self, AudioError> {
        Self::with_config(AudioConfig::default())
    }

    /// Create a new audio system with custom configuration
    pub fn with_config(config: AudioConfig) -> Result<Self, AudioError> {
        let backend = backend::create_backend(config.backend_config)?;
        
        Ok(Self {
            backend,
            sound_manager: SoundManager::new(),
            voice_manager: VoiceManager::with_capacity(config.max_voices),
            mixer: MixerSystem::new(),
            music: MusicSystem::new(),
            spatial: if config.enable_spatial {
                Some(SpatialAudio::new(config.spatial_config))
            } else {
                None
            },
            enabled: true,
        })
    }

    /// Update the audio system (call once per frame)
    pub fn update(&mut self, _delta_time: f32) {
        if !self.enabled {
            return;
        }

        // Check for finished sounds and mark them in voice manager
        let active_voices = self.voice_manager.get_all_voices();
        let mut finished_voices = Vec::new();
        
        for voice_handle in active_voices {
            if let Some(sound_handle) = self.voice_manager.get_sound_handle(voice_handle) {
                if !self.backend.is_playing(sound_handle) {
                    finished_voices.push(voice_handle);
                } else {
                    // Apply mixer volume for this voice's group
                    if let Some(volume_group) = self.voice_manager.get_volume_group(voice_handle) {
                        let effective_volume = self.mixer.get_effective_volume(volume_group);
                        let _ = self.backend.set_volume(sound_handle, effective_volume);
                    }
                }
            }
        }
        
        // Mark finished voices for cleanup
        for voice in finished_voices {
            self.voice_manager.mark_finished(voice);
        }

        // Update subsystems (will clean up marked voices)
        self.voice_manager.update();
        
        // Update music system with mixer volume
        let music_volume = self.mixer.get_effective_volume(VolumeGroup::Music);
        self.music.update(_delta_time, self.backend.as_mut(), music_volume);
        
        if let Some(spatial) = &mut self.spatial {
            spatial.update();
        }
    }

    /// Enable or disable the entire audio system
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.stop_all();
        }
    }

    /// Stop all currently playing sounds
    pub fn stop_all(&mut self) {
        self.voice_manager.stop_all();
        let _ = self.music.stop(self.backend.as_mut(), None);
    }

    /// Play audio from a loaded AudioAsset
    ///
    /// # Arguments
    /// * `audio_data` - Raw audio bytes from an AudioAsset
    ///
    /// # Returns
    /// A handle to control the playing sound
    ///
    /// # Example
    /// ```ignore
    /// let audio_asset = asset_manager.get(audio_handle)?;
    /// let sound_handle = audio_system.play_audio_asset(audio_asset.data())?;
    /// ```
    pub fn play_audio_asset(&mut self, audio_data: &[u8]) -> Result<SoundHandle, AudioError> {
        if !self.enabled {
            return Err(AudioError::Other("Audio system disabled".to_string()));
        }
        
        // Play directly through backend
        self.backend.play_sound(audio_data)
            .map_err(|e| AudioError::Other(format!("Playback failed: {}", e)))
    }

    /// Play audio from a file path
    ///
    /// # Arguments
    /// * `path` - Path to audio file
    ///
    /// # Returns
    /// A handle to control the playing sound
    pub fn play_audio_file<P: AsRef<Path>>(&mut self, path: P) -> Result<SoundHandle, AudioError> {
        self.play_audio_file_with_priority(path, VoicePriority::Normal, None, VolumeGroup::SFX)
    }
    
    /// Play audio from a file path with priority and optional sound ID for instance limiting
    ///
    /// # Arguments
    /// * `path` - Path to audio file
    /// * `priority` - Voice priority for allocation/stealing
    /// * `sound_id` - Optional identifier for instance limiting
    /// * `volume_group` - Volume group this sound belongs to
    ///
    /// # Returns
    /// A handle to control the playing sound
    pub fn play_audio_file_with_priority<P: AsRef<Path>>(
        &mut self,
        path: P,
        priority: VoicePriority,
        sound_id: Option<String>,
        volume_group: VolumeGroup,
    ) -> Result<SoundHandle, AudioError> {
        if !self.enabled {
            return Err(AudioError::Other("Audio system disabled".to_string()));
        }
        
        // Try to play through backend first
        let sound_handle = self.backend.play_sound_from_path(path.as_ref())
            .map_err(|e| AudioError::Other(format!("Playback failed: {}", e)))?;
        
        // Allocate voice with priority
        let voice_handle = self.voice_manager.allocate_voice(
            sound_handle,
            priority,
            sound_id,
            volume_group,
        );
        
        // If allocation failed, stop the sound we just started
        if voice_handle.is_none() {
            let _ = self.backend.stop(sound_handle);
            return Err(AudioError::NoAvailableVoices);
        }
        
        // Apply mixer volume for this group
        let effective_volume = self.mixer.get_effective_volume(volume_group);
        let _ = self.backend.set_volume(sound_handle, effective_volume);
        
        Ok(sound_handle)
    }

    /// Pause a playing sound
    pub fn pause_sound(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        self.backend.pause(handle)
            .map_err(|e| AudioError::Other(format!("Pause failed: {}", e)))
    }

    /// Resume a paused sound
    pub fn resume_sound(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        self.backend.resume(handle)
            .map_err(|e| AudioError::Other(format!("Resume failed: {}", e)))
    }

    /// Stop a playing sound
    pub fn stop_sound(&mut self, handle: SoundHandle) -> Result<(), AudioError> {
        self.backend.stop(handle)
            .map_err(|e| AudioError::Other(format!("Stop failed: {}", e)))
    }

    /// Set volume of a sound (0.0 = silent, 1.0 = full)
    pub fn set_sound_volume(&mut self, handle: SoundHandle, volume: f32) -> Result<(), AudioError> {
        self.backend.set_volume(handle, volume)
            .map_err(|e| AudioError::Other(format!("Set volume failed: {}", e)))
    }

    /// Check if a sound is playing
    pub fn is_sound_playing(&self, handle: SoundHandle) -> bool {
        self.backend.is_playing(handle)
    }

    // Placeholder methods - will be implemented by subsystems
    
    /// Load a sound bank from file
    pub fn load_sound_bank(&mut self, _path: &Path) -> Result<(), AudioError> {
        todo!("Implement in sound_manager module")
    }

    /// Play a 2D sound effect
    pub fn play_sound(&mut self, _sound_id: &str) -> Result<SoundHandle, AudioError> {
        todo!("Implement with sound_manager + voice_manager")
    }

    /// Play a spatial sound at world position
    pub fn play_sound_at(&mut self, _sound_id: &str, _position: Vec2) -> Result<SoundHandle, AudioError> {
        todo!("Implement with spatial audio system")
    }

    /// Set volume for a volume group
    pub fn set_group_volume(&mut self, group: VolumeGroup, volume: f32) {
        self.mixer.set_group_volume(group, volume);
    }
    
    /// Get volume for a volume group
    pub fn get_group_volume(&self, group: VolumeGroup) -> f32 {
        self.mixer.get_group_volume(group)
    }

    // === Music System API ===
    
    /// Play a music track with optional crossfade
    ///
    /// # Arguments
    /// * `track` - The music track to play
    /// * `crossfade_duration` - Optional crossfade duration in seconds (uses track default if None)
    pub fn play_music_track(&mut self, track: MusicTrack, crossfade_duration: Option<f32>) -> Result<(), AudioError> {
        self.music.play(track, self.backend.as_mut(), crossfade_duration)
            .map_err(|e| AudioError::Other(e))
    }
    
    /// Pause the current music
    pub fn pause_music(&mut self) -> Result<(), AudioError> {
        self.music.pause(self.backend.as_mut())
            .map_err(|e| AudioError::Other(e))
    }
    
    /// Resume paused music
    pub fn resume_music(&mut self) -> Result<(), AudioError> {
        self.music.resume(self.backend.as_mut())
            .map_err(|e| AudioError::Other(e))
    }
    
    /// Stop music with optional fade out
    pub fn stop_music(&mut self, fade_out_duration: Option<f32>) -> Result<(), AudioError> {
        self.music.stop(self.backend.as_mut(), fade_out_duration)
            .map_err(|e| AudioError::Other(e))
    }
    
    /// Check if music is currently playing
    pub fn is_music_playing(&self) -> bool {
        self.music.is_playing()
    }
    
    /// Get the currently playing music track ID
    pub fn current_music_track(&self) -> Option<&str> {
        self.music.current_track()
    }
    
    /// Get current music playback time in seconds
    pub fn music_playback_time(&self) -> f32 {
        self.music.playback_time()
    }
    
    /// Get music system state
    pub fn music_state(&self) -> music::MusicState {
        self.music.state()
    }
    
    // === Legacy / Placeholder Methods ===

    /// Play background music (legacy method - use play_music_track instead)
    pub fn play_music(&mut self, _track_id: &str) -> Result<(), AudioError> {
        todo!("Use play_music_track instead")
    }
    
    /// Get voice manager statistics
    pub fn get_voice_stats(&self) -> (usize, usize) {
        (self.voice_manager.active_count(), self.voice_manager.max_voices())
    }
    
    /// Set maximum instances per sound (0 = unlimited)
    pub fn set_max_instances_per_sound(&mut self, max: usize) {
        self.voice_manager.set_max_instances_per_sound(max);
    }
}

impl Default for AudioSystem {
    fn default() -> Self {
        Self::new().expect("Failed to initialize audio system")
    }
}

/// Configuration for the audio system
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Maximum number of simultaneous voices
    pub max_voices: usize,
    /// Backend-specific configuration
    pub backend_config: AudioBackendConfig,
    /// Enable spatial audio features
    pub enable_spatial: bool,
    /// Spatial audio configuration
    pub spatial_config: SpatialConfig,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            max_voices: 64,
            backend_config: AudioBackendConfig::default(),
            enable_spatial: true,
            spatial_config: SpatialConfig::default(),
        }
    }
}

/// Audio system errors
#[derive(Debug, Clone)]
pub enum AudioError {
    /// Backend initialization failed
    BackendInitFailed(String),
    /// Backend not initialized
    BackendNotInitialized,
    /// Playback failed
    PlaybackFailed(String),
    /// Invalid sound handle
    InvalidHandle,
    /// Sound asset not found
    SoundNotFound(String),
    /// No available voices
    NoAvailableVoices,
    /// File I/O error
    IoError(String),
    /// Unsupported audio format
    UnsupportedFormat(String),
    /// Generic error
    Other(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackendInitFailed(msg) => write!(f, "Audio backend initialization failed: {}", msg),
            Self::BackendNotInitialized => write!(f, "Audio backend not initialized"),
            Self::PlaybackFailed(msg) => write!(f, "Audio playback failed: {}", msg),
            Self::InvalidHandle => write!(f, "Invalid sound handle"),
            Self::SoundNotFound(id) => write!(f, "Sound not found: {}", id),
            Self::NoAvailableVoices => write!(f, "No available audio voices"),
            Self::IoError(msg) => write!(f, "Audio I/O error: {}", msg),
            Self::UnsupportedFormat(fmt) => write!(f, "Unsupported audio format: {}", fmt),
            Self::Other(msg) => write!(f, "Audio error: {}", msg),
        }
    }
}

impl std::error::Error for AudioError {}

impl From<String> for AudioError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}
