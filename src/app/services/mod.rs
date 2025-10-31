/// Application-level services that coordinate business logic
///
/// These services act as coordinators between low-level business services
/// and application state, handling UI updates and data persistence.
pub mod lyrics_service;

// Re-export commonly used types
pub use lyrics_service::LyricsService;
