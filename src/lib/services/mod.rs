/// Service layer modules
///
/// This module contains business logic services that handle
/// complex operations separated from the main App struct.
pub mod library_import;
pub mod lyrics_manager;
pub mod metadata_editor;
pub mod player_restore;
pub mod playlist_export;
pub mod playlist_import;
pub mod subtitle_manager;
pub mod youtube_download;

// Re-export commonly used types
pub use library_import::LibraryImportService;
pub use lyrics_manager::LyricsManager;
pub use metadata_editor::MetadataEditor;
pub use player_restore::PlayerRestoreService;
pub use playlist_export::{PlaylistExportResult, PlaylistExportService};
pub use playlist_import::{PlaylistImportResult, PlaylistImportService};
pub use subtitle_manager::SubtitleManager;
pub use youtube_download::{
    YoutubeDownloadEvent, YoutubeDownloadOptions, YoutubeDownloadResult, YoutubeDownloadService,
    YoutubeSearchResult,
};
