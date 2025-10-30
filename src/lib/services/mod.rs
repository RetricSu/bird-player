/// Service layer modules
///
/// This module contains business logic services that handle
/// complex operations separated from the main App struct.
pub mod library_import;
pub mod lyrics_manager;
pub mod metadata_editor;
pub mod player_restore;

// Re-export commonly used types
pub use library_import::LibraryImportService;
pub use lyrics_manager::LyricsManager;
pub use metadata_editor::MetadataEditor;
pub use player_restore::PlayerRestoreService;
