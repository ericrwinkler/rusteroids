//! Audio mixer system
//!
//! Manages volume groups and mixing of multiple audio channels.
//! Provides independent volume control for different categories of sounds.

use std::collections::HashMap;

/// Volume group categories for independent volume control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VolumeGroup {
    /// Master volume (affects all sounds)
    Master,
    /// Sound effects
    SFX,
    /// Background music
    Music,
    /// User interface sounds
    UI,
    /// Ambient environmental sounds
    Ambient,
    /// Event-triggered sounds (alerts, notifications)
    Events,
}

/// Audio mixer managing volume groups and mixing
pub struct MixerSystem {
    /// Volume levels for each group (0.0 to 1.0)
    group_volumes: HashMap<VolumeGroup, f32>,
    /// Mute state for each group
    group_muted: HashMap<VolumeGroup, bool>,
}

impl MixerSystem {
    /// Create a new mixer system with default volumes
    pub fn new() -> Self {
        let mut group_volumes = HashMap::new();
        group_volumes.insert(VolumeGroup::Master, 1.0);
        group_volumes.insert(VolumeGroup::SFX, 1.0);
        group_volumes.insert(VolumeGroup::Music, 1.0);
        group_volumes.insert(VolumeGroup::UI, 1.0);
        group_volumes.insert(VolumeGroup::Ambient, 0.7);
        group_volumes.insert(VolumeGroup::Events, 1.0);
        
        Self {
            group_volumes,
            group_muted: HashMap::new(),
        }
    }
    
    /// Set volume for a specific group (0.0 to 1.0)
    pub fn set_group_volume(&mut self, group: VolumeGroup, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        self.group_volumes.insert(group, clamped);
    }
    
    /// Get volume for a specific group
    pub fn get_group_volume(&self, group: VolumeGroup) -> f32 {
        *self.group_volumes.get(&group).unwrap_or(&1.0)
    }
    
    /// Get effective volume for a group (considering master volume and mute)
    pub fn get_effective_volume(&self, group: VolumeGroup) -> f32 {
        if self.is_muted(group) {
            return 0.0;
        }
        
        let group_vol = self.get_group_volume(group);
        let master_vol = self.get_group_volume(VolumeGroup::Master);
        
        group_vol * master_vol
    }
    
    /// Mute a volume group
    pub fn mute_group(&mut self, group: VolumeGroup) {
        self.group_muted.insert(group, true);
    }
    
    /// Unmute a volume group
    pub fn unmute_group(&mut self, group: VolumeGroup) {
        self.group_muted.insert(group, false);
    }
    
    /// Check if a group is muted
    pub fn is_muted(&self, group: VolumeGroup) -> bool {
        *self.group_muted.get(&group).unwrap_or(&false)
    }
    
    /// Toggle mute state for a group
    pub fn toggle_mute(&mut self, group: VolumeGroup) {
        let is_muted = self.is_muted(group);
        self.group_muted.insert(group, !is_muted);
    }
}

impl Default for MixerSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_volumes() {
        let mixer = MixerSystem::new();
        assert_eq!(mixer.get_group_volume(VolumeGroup::Master), 1.0);
        assert_eq!(mixer.get_group_volume(VolumeGroup::SFX), 1.0);
    }
    
    #[test]
    fn test_set_volume() {
        let mut mixer = MixerSystem::new();
        mixer.set_group_volume(VolumeGroup::SFX, 0.5);
        assert_eq!(mixer.get_group_volume(VolumeGroup::SFX), 0.5);
    }
    
    #[test]
    fn test_volume_clamping() {
        let mut mixer = MixerSystem::new();
        mixer.set_group_volume(VolumeGroup::SFX, 2.0);
        assert_eq!(mixer.get_group_volume(VolumeGroup::SFX), 1.0);
        
        mixer.set_group_volume(VolumeGroup::SFX, -0.5);
        assert_eq!(mixer.get_group_volume(VolumeGroup::SFX), 0.0);
    }
    
    #[test]
    fn test_effective_volume_with_master() {
        let mut mixer = MixerSystem::new();
        mixer.set_group_volume(VolumeGroup::Master, 0.5);
        mixer.set_group_volume(VolumeGroup::SFX, 0.8);
        
        assert_eq!(mixer.get_effective_volume(VolumeGroup::SFX), 0.4);
    }
    
    #[test]
    fn test_mute() {
        let mut mixer = MixerSystem::new();
        mixer.set_group_volume(VolumeGroup::SFX, 1.0);
        mixer.mute_group(VolumeGroup::SFX);
        
        assert_eq!(mixer.get_effective_volume(VolumeGroup::SFX), 0.0);
    }
}
