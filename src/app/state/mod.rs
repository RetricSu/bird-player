/// State management modules
///
/// This module contains all state-related structures and logic,
/// separated by concern for better maintainability.
pub mod persistence;
pub mod ui_state;

// Re-export commonly used types
pub use persistence::StatePersistence;
