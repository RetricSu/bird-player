/// State management modules
///
/// This module contains all state-related structures and logic,
/// separated by concern for better maintainability.
pub mod app_state;
pub mod persistence;
pub mod player_state;
pub mod ui_state;

// Re-export commonly used types
pub use app_state::AppState;
pub use persistence::StatePersistence;
pub use player_state::PlayerStateManager;
pub use ui_state::UiState;
