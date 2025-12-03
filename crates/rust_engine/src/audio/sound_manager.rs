//! Sound asset management
//!
//! Handles loading, caching, and management of audio assets.
//! Supports sound banks for grouped loading/unloading.

use std::collections::HashMap;

/// Unique identifier for a sound
pub type SoundId = String;

/// Sound bank containing multiple related sounds
pub struct SoundBank {
    name: String,
    sounds: HashMap<SoundId, SoundAsset>,
}

/// Individual sound asset
struct SoundAsset {
    _id: SoundId,
    // TODO: Add actual audio data (buffer, format info, etc.)
}

/// Sound asset manager
pub struct SoundManager {
    banks: HashMap<String, SoundBank>,
    loaded_sounds: HashMap<SoundId, SoundAsset>,
}

impl SoundManager {
    /// Create a new sound manager
    pub fn new() -> Self {
        Self {
            banks: HashMap::new(),
            loaded_sounds: HashMap::new(),
        }
    }
    
    /// Check if a sound is loaded
    pub fn is_loaded(&self, sound_id: &str) -> bool {
        self.loaded_sounds.contains_key(sound_id)
    }
}

impl Default for SoundManager {
    fn default() -> Self {
        Self::new()
    }
}
