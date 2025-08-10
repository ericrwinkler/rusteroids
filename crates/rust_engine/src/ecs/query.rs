//! Query system for component access

/// Query for accessing components
pub struct Query<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Query<T> {
    /// Create a new query
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}
