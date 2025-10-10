//! Thread-Safe Component Storage
//! 
//! This module provides the foundation for thread-safe ECS operations,
//! addressing race conditions and enabling parallel system execution.

use std::collections::HashMap;
use std::sync::{Arc, RwLock, atomic::{AtomicU64, Ordering}};
use std::marker::PhantomData;

/// Generation-based handle for safe component access across threads
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentHandle {
    pub id: u64,
    pub generation: u64,
}

/// Thread-safe component storage using generation-based handles
pub struct ComponentStorage<T: Component + Send + Sync> {
    components: RwLock<slotmap::SlotMap<slotmap::DefaultKey, T>>,
    handle_map: RwLock<HashMap<ComponentHandle, slotmap::DefaultKey>>,
    generation: AtomicU64,
    _phantom: PhantomData<T>,
}

impl<T: Component + Send + Sync> ComponentStorage<T> {
    pub fn new() -> Self {
        Self {
            components: RwLock::new(slotmap::SlotMap::new()),
            handle_map: RwLock::new(HashMap::new()),
            generation: AtomicU64::new(0),
            _phantom: PhantomData,
        }
    }

    /// Insert a component and return a thread-safe handle
    pub fn insert(&self, component: T) -> ComponentHandle {
        let mut components = self.components.write().unwrap();
        let mut handle_map = self.handle_map.write().unwrap();
        
        let key = components.insert(component);
        let handle = ComponentHandle {
            id: key.data().as_ffi(),
            generation: self.generation.fetch_add(1, Ordering::SeqCst),
        };
        
        handle_map.insert(handle, key);
        handle
    }

    /// Get a component by handle (read-only access)
    pub fn get(&self, handle: ComponentHandle) -> Option<std::sync::RwLockReadGuard<T>> {
        let handle_map = self.handle_map.read().unwrap();
        let components = self.components.read().unwrap();
        
        if let Some(&key) = handle_map.get(&handle) {
            if components.contains_key(key) {
                // This is a simplified version - real implementation would need
                // to return a guard that maintains the lock
                // For now, we'll use a different approach
                todo!("Implement safe component access pattern")
            }
        }
        None
    }

    /// Remove a component by handle
    pub fn remove(&self, handle: ComponentHandle) -> Option<T> {
        let mut components = self.components.write().unwrap();
        let mut handle_map = self.handle_map.write().unwrap();
        
        if let Some(key) = handle_map.remove(&handle) {
            components.remove(key)
        } else {
            None
        }
    }

    /// Get all components for read-only iteration
    pub fn iter_all(&self) -> Vec<T> 
    where 
        T: Clone 
    {
        let components = self.components.read().unwrap();
        components.values().cloned().collect()
    }
}

/// Safe component access wrapper
pub struct ComponentRef<T> {
    _guard: std::sync::RwLockReadGuard<'static, slotmap::SlotMap<slotmap::DefaultKey, T>>,
    _component: &'static T,
}

/// Mutable component access wrapper  
pub struct ComponentMut<T> {
    _guard: std::sync::RwLockWriteGuard<'static, slotmap::SlotMap<slotmap::DefaultKey, T>>,
    _component: &'static mut T,
}

// Re-export the Component trait
use crate::ecs::Component;
