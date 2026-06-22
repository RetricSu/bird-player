use crate::app::lib_services::YoutubeDownloadService;
use crate::app::libstate::lyrics_state::LyricsFetchState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// UI-specific state that doesn't need to be persisted
#[derive(Debug, Clone)]
pub struct UiState {
    /// Whether the library folders section is expanded
    pub library_folders_expanded: bool,

    /// Whether the about dialog is shown
    pub show_about_dialog: bool,

    /// Index of playlist to remove (if any)
    pub playlist_idx_to_remove: Option<usize>,

    /// Index of playlist being renamed (if any)
    pub playlist_being_renamed: Option<usize>,

    /// Default window height
    pub default_window_height: f64,

    /// Whether the window is maximized
    pub is_maximized: bool,

    /// Whether the lyrics panel is shown
    pub show_lyrics_panel: bool,

    /// Whether the desktop lyrics mode is active
    pub desktop_lyrics_enabled: bool,

    /// Whether to fetch lyrics on init (after heavy data loaded)
    pub should_fetch_lyrics_on_init: bool,

    /// Current lyrics fetch state
    pub lyrics_fetch_state: LyricsFetchState,

    /// Last window title to avoid redundant updates
    pub last_window_title: Option<String>,

    /// Whether library import is currently running
    pub is_importing: bool,

    /// Whether the authorized audio download window is open.
    pub show_youtube_download_dialog: bool,

    /// URL entered in the authorized audio download form.
    pub youtube_download_url: String,

    /// Destination folder for authorized audio downloads.
    pub youtube_download_dir: PathBuf,

    /// Whether a yt-dlp download task is currently running.
    pub youtube_download_in_progress: bool,

    /// Last user-facing download status or error message.
    pub youtube_download_status: Option<String>,

    /// Volume value to restore when the user un-mutes via the speaker icon.
    /// `None` while not muted. Not serialized: a fresh launch always starts
    /// with whatever volume the player itself remembers.
    pub volume_before_mute: Option<f32>,

    /// Last time player persistence was flushed (used to throttle disk writes
    /// while a track is playing). Not serialized.
    pub last_persistence_save: Instant,

    /// Font size for the desktop lyrics overlay.
    pub desktop_lyrics_font_size: f32,

    /// Foreground color for the desktop lyrics overlay (sRGBA).
    pub desktop_lyrics_color: [u8; 4],

    /// When true, the desktop lyrics overlay ignores drag input.
    pub desktop_lyrics_locked: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            library_folders_expanded: false,
            show_about_dialog: false,
            playlist_idx_to_remove: None,
            playlist_being_renamed: None,
            default_window_height: crate::app::constants::DEFAULT_WINDOW_HEIGHT as f64,
            is_maximized: false,
            show_lyrics_panel: false,
            desktop_lyrics_enabled: false,
            should_fetch_lyrics_on_init: false,
            lyrics_fetch_state: LyricsFetchState::Idle,
            last_window_title: None,
            is_importing: false,
            show_youtube_download_dialog: false,
            youtube_download_url: String::new(),
            youtube_download_dir: YoutubeDownloadService::default_download_dir(),
            youtube_download_in_progress: false,
            youtube_download_status: None,
            volume_before_mute: None,
            last_persistence_save: Instant::now(),
            desktop_lyrics_font_size: 48.0,
            desktop_lyrics_color: [0, 255, 255, 255],
            desktop_lyrics_locked: false,
        }
    }
}

/// Persistable UI settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiSettings {
    pub library_folders_expanded: bool,
    pub default_window_height: f64,
    pub show_lyrics_panel: bool,
    pub desktop_lyrics_enabled: bool,
    #[serde(default = "default_desktop_lyrics_font_size")]
    pub desktop_lyrics_font_size: f32,
    #[serde(default = "default_desktop_lyrics_color")]
    pub desktop_lyrics_color: [u8; 4],
    #[serde(default)]
    pub desktop_lyrics_locked: bool,
}

fn default_desktop_lyrics_font_size() -> f32 {
    48.0
}

fn default_desktop_lyrics_color() -> [u8; 4] {
    [0, 255, 255, 255]
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            library_folders_expanded: false,
            default_window_height: crate::app::constants::DEFAULT_WINDOW_HEIGHT as f64,
            show_lyrics_panel: false,
            desktop_lyrics_enabled: false,
            desktop_lyrics_font_size: default_desktop_lyrics_font_size(),
            desktop_lyrics_color: default_desktop_lyrics_color(),
            desktop_lyrics_locked: false,
        }
    }
}

impl UiState {
    /// Extract persistable settings from UI state
    pub fn to_settings(&self) -> UiSettings {
        UiSettings {
            library_folders_expanded: self.library_folders_expanded,
            default_window_height: self.default_window_height,
            show_lyrics_panel: self.show_lyrics_panel,
            desktop_lyrics_enabled: self.desktop_lyrics_enabled,
            desktop_lyrics_font_size: self.desktop_lyrics_font_size,
            desktop_lyrics_color: self.desktop_lyrics_color,
            desktop_lyrics_locked: self.desktop_lyrics_locked,
        }
    }

    /// Apply settings to UI state
    pub fn apply_settings(&mut self, settings: UiSettings) {
        self.library_folders_expanded = settings.library_folders_expanded;
        self.default_window_height = settings.default_window_height;
        self.show_lyrics_panel = settings.show_lyrics_panel;
        self.desktop_lyrics_enabled = settings.desktop_lyrics_enabled;
        self.desktop_lyrics_font_size = settings.desktop_lyrics_font_size;
        self.desktop_lyrics_color = settings.desktop_lyrics_color;
        self.desktop_lyrics_locked = settings.desktop_lyrics_locked;
    }
}
