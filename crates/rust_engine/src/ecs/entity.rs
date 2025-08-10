//! Entity implementation

/// Entity identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    id: u32,
}

impl Entity {
    /// Create a new entity with the given ID
    pub(super) fn new(id: u32) -> Self {
        Self { id }
    }
    
    /// Get the entity ID
    pub fn id(&self) -> u32 {
        self.id
    }
}
