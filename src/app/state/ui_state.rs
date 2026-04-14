use crate::app::libstate::lyrics_state::LyricsFetchState;
use serde::{Deserialize, Serialize};
/// UI-specific state that doesn't need to be persisted
#[derive(Debug, Clone)]
pub struct UiState {
    /// Whether to show the library and playlist panel
    pub show_library_and_playlist: bool,

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

    /// Whether to fetch lyrics on init (after heavy data loaded)
    pub should_fetch_lyrics_on_init: bool,

    /// Current lyrics fetch state
    pub lyrics_fetch_state: LyricsFetchState,

    /// Last window title to avoid redundant updates
    pub last_window_title: Option<String>,

    /// Whether library import is currently running
    pub is_importing: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_library_and_playlist: true,
            library_folders_expanded: false,
            show_about_dialog: false,
            playlist_idx_to_remove: None,
            playlist_being_renamed: None,
            default_window_height: crate::app::constants::DEFAULT_WINDOW_HEIGHT as f64,
            is_maximized: false,
            show_lyrics_panel: false,
            should_fetch_lyrics_on_init: false,
            lyrics_fetch_state: LyricsFetchState::Idle,
            last_window_title: None,
            is_importing: false,
        }
    }
}

/// Persistable UI settings
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UiSettings {
    pub library_folders_expanded: bool,
    pub default_window_height: f64,
    pub show_lyrics_panel: bool,
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            library_folders_expanded: false,
            default_window_height: crate::app::constants::DEFAULT_WINDOW_HEIGHT as f64,
            show_lyrics_panel: false,
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
        }
    }

    /// Apply settings to UI state
    pub fn apply_settings(&mut self, settings: UiSettings) {
        self.library_folders_expanded = settings.library_folders_expanded;
        self.default_window_height = settings.default_window_height;
        self.show_lyrics_panel = settings.show_lyrics_panel;
    }
}
