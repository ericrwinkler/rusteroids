//! Specialized collection types

pub use slotmap::{SlotMap, DefaultKey};

/// Handle-based map using slot map for stable references
pub type HandleMap<T> = SlotMap<DefaultKey, T>;

/// Handle type for stable references
pub type Handle = DefaultKey;

/// Typed handle for type-safe asset references
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypedHandle<T> {
    key: DefaultKey,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TypedHandle<T> {
    /// Create a new typed handle from a key
    pub fn new(key: DefaultKey) -> Self {
        Self {
            key,
            _phantom: std::marker::PhantomData,
        }
    }
    
    /// Get the underlying key
    pub fn key(&self) -> DefaultKey {
        self.key
    }
}

/// Free list for object pooling
pub struct FreeList<T> {
    items: Vec<Option<T>>,
    free_indices: Vec<usize>,
}

impl<T> FreeList<T> {
    /// Create a new free list
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            free_indices: Vec::new(),
        }
    }
    
    /// Insert an item and return its index
    pub fn insert(&mut self, item: T) -> usize {
        if let Some(index) = self.free_indices.pop() {
            self.items[index] = Some(item);
            index
        } else {
            let index = self.items.len();
            self.items.push(Some(item));
            index
        }
    }
    
    /// Remove an item by index
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < self.items.len() {
            if let Some(item) = self.items[index].take() {
                self.free_indices.push(index);
                Some(item)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Get an item by index
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)?.as_ref()
    }
    
    /// Get a mutable reference to an item by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.items.get_mut(index)?.as_mut()
    }
}

impl<T> Default for FreeList<T> {
    fn default() -> Self {
        Self::new()
    }
}
