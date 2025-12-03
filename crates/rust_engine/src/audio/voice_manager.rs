//! Voice management system
//!
//! Manages allocation and pooling of audio voices (channels).
//! Implements priority-based voice stealing when all voices are in use.

use crate::audio::backend::rodio_backend::SoundHandle;
use crate::audio::mixer::VolumeGroup;
use std::collections::HashMap;

/// Handle to an active voice
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceHandle(u32);

impl VoiceHandle {
    /// Create an invalid voice handle
    pub fn invalid() -> Self {
        Self(u32::MAX)
    }
    
    /// Check if this handle is valid
    pub fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

/// Priority for voice allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VoicePriority {
    /// Lowest priority (ambient sounds)
    Low = 0,
    /// Normal priority (most sound effects)
    Normal = 1,
    /// High priority (important gameplay sounds)
    High = 2,
    /// Critical priority (never steal - UI feedback, player actions)
    Critical = 3,
}

/// Information about an active voice slot
#[derive(Debug, Clone)]
struct VoiceSlot {
    /// Backend sound handle
    sound_handle: SoundHandle,
    /// Priority of this voice
    priority: VoicePriority,
    /// Optional sound identifier for instance limiting
    sound_id: Option<String>,
    /// Volume group this voice belongs to
    volume_group: VolumeGroup,
    /// Is this voice currently playing
    is_playing: bool,
}

/// Voice manager for allocation and pooling
pub struct VoiceManager {
    /// Maximum number of simultaneous voices
    max_voices: usize,
    /// Active voice slots
    voices: HashMap<VoiceHandle, VoiceSlot>,
    /// Next voice handle ID
    next_handle_id: u32,
    /// Per-sound instance counts for limiting
    instance_counts: HashMap<String, usize>,
    /// Maximum instances per sound (0 = unlimited)
    max_instances_per_sound: usize,
}

impl VoiceManager {
    /// Create a new voice manager with specified capacity
    pub fn with_capacity(max_voices: usize) -> Self {
        Self {
            max_voices,
            voices: HashMap::new(),
            next_handle_id: 0,
            instance_counts: HashMap::new(),
            max_instances_per_sound: 4, // Default: max 4 of same sound
        }
    }
    
    /// Generate a new unique voice handle
    fn next_handle(&mut self) -> VoiceHandle {
        let id = self.next_handle_id;
        self.next_handle_id = self.next_handle_id.wrapping_add(1);
        VoiceHandle(id)
    }
    
    /// Allocate a voice for a sound
    ///
    /// # Arguments
    /// * `sound_handle` - Backend sound handle
    /// * `priority` - Priority for this voice
    /// * `sound_id` - Optional identifier for instance limiting
    /// * `volume_group` - Volume group this sound belongs to
    ///
    /// # Returns
    /// A `VoiceHandle` if allocation succeeds, or `None` if:
    /// - Voice pool is full and no voice can be stolen
    /// - Instance limit would be exceeded
    pub fn allocate_voice(
        &mut self,
        sound_handle: SoundHandle,
        priority: VoicePriority,
        sound_id: Option<String>,
        volume_group: VolumeGroup,
    ) -> Option<VoiceHandle> {
        // Check instance limit
        if let Some(ref id) = sound_id {
            let count = self.instance_counts.get(id).copied().unwrap_or(0);
            if self.max_instances_per_sound > 0 && count >= self.max_instances_per_sound {
                return None; // Instance limit exceeded
            }
        }
        
        // Check if we have available slots
        if self.voices.len() < self.max_voices {
            // Allocate new voice
            let handle = self.next_handle();
            let slot = VoiceSlot {
                sound_handle,
                priority,
                sound_id: sound_id.clone(),
                volume_group,
                is_playing: true,
            };
            
            self.voices.insert(handle, slot);
            
            // Increment instance count
            if let Some(id) = sound_id {
                *self.instance_counts.entry(id).or_insert(0) += 1;
            }
            
            return Some(handle);
        }
        
        // Pool is full - attempt voice stealing
        self.steal_voice(priority, sound_handle, sound_id, volume_group)
    }
    
    /// Steal a voice from a lower priority sound
    ///
    /// Finds the lowest priority voice and replaces it if the new sound has higher priority.
    fn steal_voice(
        &mut self,
        new_priority: VoicePriority,
        sound_handle: SoundHandle,
        sound_id: Option<String>,
        volume_group: VolumeGroup,
    ) -> Option<VoiceHandle> {
        // Find lowest priority voice
        let mut lowest_priority = VoicePriority::Critical;
        let mut victim_handle = None;
        
        for (handle, slot) in &self.voices {
            if slot.priority < lowest_priority {
                lowest_priority = slot.priority;
                victim_handle = Some(*handle);
            }
        }
        
        // Only steal if new sound has higher priority
        if let Some(victim) = victim_handle {
            if new_priority > lowest_priority {
                // Remove victim voice
                if let Some(victim_slot) = self.voices.remove(&victim) {
                    // Decrement instance count for victim
                    if let Some(ref id) = victim_slot.sound_id {
                        if let Some(count) = self.instance_counts.get_mut(id) {
                            *count = count.saturating_sub(1);
                        }
                    }
                    
                    // Note: Backend sound should be stopped by caller
                    
                    // Allocate new voice in victim's slot
                    let slot = VoiceSlot {
                        sound_handle,
                        priority: new_priority,
                        sound_id: sound_id.clone(),
                        volume_group,
                        is_playing: true,
                    };
                    
                    self.voices.insert(victim, slot);
                    
                    // Increment instance count for new sound
                    if let Some(id) = sound_id {
                        *self.instance_counts.entry(id).or_insert(0) += 1;
                    }
                    
                    return Some(victim);
                }
            }
        }
        
        None // Cannot steal
    }
    
    /// Update the voice manager (call once per frame)
    ///
    /// Cleans up finished sounds and updates internal state.
    pub fn update(&mut self) {
        // Collect finished voices
        let finished: Vec<VoiceHandle> = self.voices
            .iter()
            .filter(|(_, slot)| !slot.is_playing)
            .map(|(handle, _)| *handle)
            .collect();
        
        // Remove finished voices
        for handle in finished {
            self.remove_voice(handle);
        }
    }
    
    /// Mark a voice as finished (will be cleaned up on next update)
    pub fn mark_finished(&mut self, handle: VoiceHandle) {
        if let Some(slot) = self.voices.get_mut(&handle) {
            slot.is_playing = false;
        }
    }
    
    /// Remove a voice from the pool
    fn remove_voice(&mut self, handle: VoiceHandle) {
        if let Some(slot) = self.voices.remove(&handle) {
            // Decrement instance count
            if let Some(ref id) = slot.sound_id {
                if let Some(count) = self.instance_counts.get_mut(id) {
                    *count = count.saturating_sub(1);
                    // Remove entry if count is 0
                    if *count == 0 {
                        self.instance_counts.remove(id);
                    }
                }
            }
        }
    }
    
    /// Stop all active voices
    pub fn stop_all(&mut self) {
        self.voices.clear();
        self.instance_counts.clear();
    }
    
    /// Get the number of currently active voices
    pub fn active_count(&self) -> usize {
        self.voices.len()
    }
    
    /// Get the maximum number of voices
    pub fn max_voices(&self) -> usize {
        self.max_voices
    }
    
    /// Check if voices are available
    pub fn has_available_voices(&self) -> bool {
        self.voices.len() < self.max_voices
    }
    
    /// Get sound handle for a voice
    pub fn get_sound_handle(&self, handle: VoiceHandle) -> Option<SoundHandle> {
        self.voices.get(&handle).map(|slot| slot.sound_handle)
    }
    
    /// Get volume group for a voice
    pub fn get_volume_group(&self, handle: VoiceHandle) -> Option<VolumeGroup> {
        self.voices.get(&handle).map(|slot| slot.volume_group)
    }
    
    /// Get all active voice handles
    pub fn get_all_voices(&self) -> Vec<VoiceHandle> {
        self.voices.keys().copied().collect()
    }
    
    /// Set maximum instances per sound (0 = unlimited)
    pub fn set_max_instances_per_sound(&mut self, max: usize) {
        self.max_instances_per_sound = max;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sound_handle(id: u32) -> SoundHandle {
        SoundHandle::new(id, 0)
    }
    
    const TEST_VOLUME_GROUP: VolumeGroup = VolumeGroup::SFX;

    #[test]
    fn test_voice_manager_creation() {
        let manager = VoiceManager::with_capacity(64);
        assert_eq!(manager.max_voices(), 64);
        assert_eq!(manager.active_count(), 0);
        assert!(manager.has_available_voices());
    }

    #[test]
    fn test_voice_allocation() {
        let mut manager = VoiceManager::with_capacity(4);
        
        // Allocate voices
        let handle1 = manager.allocate_voice(
            create_test_sound_handle(1),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        );
        assert!(handle1.is_some());
        assert_eq!(manager.active_count(), 1);
        
        let handle2 = manager.allocate_voice(
            create_test_sound_handle(2),
            VoicePriority::High,
            None,
            TEST_VOLUME_GROUP,
        );
        assert!(handle2.is_some());
        assert_eq!(manager.active_count(), 2);
    }

    #[test]
    fn test_voice_pool_full() {
        let mut manager = VoiceManager::with_capacity(2);
        
        // Fill pool
        let _h1 = manager.allocate_voice(
            create_test_sound_handle(1),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        );
        let _h2 = manager.allocate_voice(
            create_test_sound_handle(2),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        );
        
        assert_eq!(manager.active_count(), 2);
        assert!(!manager.has_available_voices());
        
        // Try to allocate with same priority - should fail
        let h3 = manager.allocate_voice(
            create_test_sound_handle(3),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        );
        assert!(h3.is_none());
    }

    #[test]
    fn test_voice_stealing() {
        let mut manager = VoiceManager::with_capacity(2);
        
        // Fill pool with low priority sounds
        let _h1 = manager.allocate_voice(
            create_test_sound_handle(1),
            VoicePriority::Low,
            None,
            TEST_VOLUME_GROUP,
        );
        let _h2 = manager.allocate_voice(
            create_test_sound_handle(2),
            VoicePriority::Low,
            None,
            TEST_VOLUME_GROUP,
        );
        
        assert_eq!(manager.active_count(), 2);
        
        // Allocate high priority - should steal a low priority voice
        let h3 = manager.allocate_voice(
            create_test_sound_handle(3),
            VoicePriority::High,
            None,
            TEST_VOLUME_GROUP,
        );
        assert!(h3.is_some());
        assert_eq!(manager.active_count(), 2); // Still 2, but one was stolen
    }

    #[test]
    fn test_priority_ordering() {
        let mut manager = VoiceManager::with_capacity(3);
        
        // Add different priorities
        manager.allocate_voice(
            create_test_sound_handle(1),
            VoicePriority::Critical,
            None,
            TEST_VOLUME_GROUP,
        );
        manager.allocate_voice(
            create_test_sound_handle(2),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        );
        manager.allocate_voice(
            create_test_sound_handle(3),
            VoicePriority::Low,
            None,
            TEST_VOLUME_GROUP,
        );
        
        // Pool full - try to add High priority
        let h4 = manager.allocate_voice(
            create_test_sound_handle(4),
            VoicePriority::High,
            None,
            TEST_VOLUME_GROUP,
        );
        
        // Should steal Low priority, not Critical or Normal
        assert!(h4.is_some());
        assert_eq!(manager.active_count(), 3);
    }

    #[test]
    fn test_instance_limiting() {
        let mut manager = VoiceManager::with_capacity(10);
        manager.set_max_instances_per_sound(2);
        
        // Allocate 2 instances of "laser"
        let h1 = manager.allocate_voice(create_test_sound_handle(1), VoicePriority::Normal, Some("laser".to_string()), TEST_VOLUME_GROUP,
        );
        assert!(h1.is_some());
        
        let h2 = manager.allocate_voice(create_test_sound_handle(2), VoicePriority::Normal, Some("laser".to_string()), TEST_VOLUME_GROUP,
        );
        assert!(h2.is_some());
        
        // Third instance should fail
        let h3 = manager.allocate_voice(create_test_sound_handle(3), VoicePriority::Normal, Some("laser".to_string()), TEST_VOLUME_GROUP,
        );
        assert!(h3.is_none());
        
        // Different sound should work
        let h4 = manager.allocate_voice(create_test_sound_handle(4), VoicePriority::Normal, Some("explosion".to_string()), TEST_VOLUME_GROUP,
        );
        assert!(h4.is_some());
    }

    #[test]
    fn test_cleanup_finished() {
        let mut manager = VoiceManager::with_capacity(4);
        
        let h1 = manager.allocate_voice(
            create_test_sound_handle(1),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        ).unwrap();
        
        let h2 = manager.allocate_voice(
            create_test_sound_handle(2),
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        ).unwrap();
        
        assert_eq!(manager.active_count(), 2);
        
        // Mark one as finished
        manager.mark_finished(h1);
        assert_eq!(manager.active_count(), 2); // Still there until update
        
        // Update should remove finished
        manager.update();
        assert_eq!(manager.active_count(), 1);
        
        // h2 should still be there
        assert!(manager.get_sound_handle(h2).is_some());
        assert!(manager.get_sound_handle(h1).is_none());
    }

    #[test]
    fn test_stop_all() {
        let mut manager = VoiceManager::with_capacity(4);
        
        manager.allocate_voice(create_test_sound_handle(1), VoicePriority::Normal, None, TEST_VOLUME_GROUP);
        manager.allocate_voice(create_test_sound_handle(2), VoicePriority::High, None, TEST_VOLUME_GROUP);
        manager.allocate_voice(create_test_sound_handle(3), VoicePriority::Low, None, TEST_VOLUME_GROUP);
        
        assert_eq!(manager.active_count(), 3);
        
        manager.stop_all();
        assert_eq!(manager.active_count(), 0);
        assert!(manager.has_available_voices());
    }

    #[test]
    fn test_get_sound_handle() {
        let mut manager = VoiceManager::with_capacity(4);
        
        let sound_handle = create_test_sound_handle(42);
        let voice_handle = manager.allocate_voice(
            sound_handle,
            VoicePriority::Normal,
            None,
            TEST_VOLUME_GROUP,
        ).unwrap();
        
        let retrieved = manager.get_sound_handle(voice_handle);
        assert_eq!(retrieved, Some(sound_handle));
    }
}


