/// Application-level services that coordinate business logic
///
/// These services act as coordinators between low-level business services
/// and application state, handling UI updates and data persistence.
pub mod config_persistence;
pub mod db_persistence;
pub mod library_service;
pub mod lyrics_service;
pub mod persistence_service;
pub mod player_service;
pub mod playlist_service;

// Re-export commonly used types
pub use library_service::LibraryService;
pub use lyrics_service::LyricsService;
pub use persistence_service::PersistenceService;
pub use player_service::PlayerService;
pub use playlist_service::PlaylistService;
