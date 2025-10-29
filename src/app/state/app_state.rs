use std::sync::{Arc, Mutex};

use super::{PlayerStateManager, StatePersistence, UiState};
use crate::app::{i18n::Language, library::Library, playlist::Playlist};

/// Core application state
/// This contains the main data structures
pub struct AppState {
    /// Music library
    pub library: Library,

    /// List of playlists
    pub playlists: Vec<Playlist>,

    /// Currently selected playlist index
    pub current_playlist_idx: Option<usize>,

    /// Index of the playlist that is currently playing
    pub playing_playlist_idx: Option<usize>,

    /// Current language setting
    pub current_language: Language,

    /// Whether heavy data (library, playlists) has been loaded
    pub heavy_data_loaded: bool,

    /// Application quit flag
    pub quit: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            library: Library::new(),
            playlists: vec![],
            current_playlist_idx: None,
            playing_playlist_idx: None,
            current_language: Language::English,
            heavy_data_loaded: false,
            quit: false,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load basic app state (settings only)
    pub fn load_basic() -> Result<(Self, PlayerStateManager, UiState), Box<dyn std::error::Error>> {
        let mut app_state = Self::default();
        let mut player_state = PlayerStateManager::default();
        let mut ui_state = UiState::default();

        // Load settings from confy
        if let Ok(settings) = StatePersistence::load_settings() {
            app_state.current_language = settings.current_language;
            player_state.apply_settings(settings.player);
            ui_state.apply_settings(settings.ui);
        }

        // Initialize i18n
        crate::app::i18n::init();
        crate::app::i18n::set_language(app_state.current_language);

        Ok((app_state, player_state, ui_state))
    }

    /// Load library and playlists from database
    pub fn load_heavy_data(
        &mut self,
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
        player_state: &PlayerStateManager,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.heavy_data_loaded {
            return Ok(());
        }

        tracing::info!("Loading heavy data (library and playlists)...");

        // Load library
        match StatePersistence::load_library(db_conn) {
            Ok(library) => {
                self.library = library;
                tracing::info!("Successfully loaded library from database");
            }
            Err(e) => {
                tracing::error!("Failed to load library from database: {}", e);
            }
        }

        // Load playlists
        match StatePersistence::load_playlists(db_conn) {
            Ok(playlists) => {
                if !playlists.is_empty() {
                    self.playlists = playlists;
                    self.select_playlist_for_track(&player_state.last_track_path);
                } else {
                    self.create_default_playlist();
                }
            }
            Err(e) => {
                tracing::error!("Failed to load playlists from database: {}", e);
                self.create_default_playlist();
            }
        }

        self.heavy_data_loaded = true;
        tracing::info!("Heavy data loading completed");

        Ok(())
    }

    /// Find and select the playlist containing the given track
    fn select_playlist_for_track(&mut self, track_path: &Option<std::path::PathBuf>) {
        if let Some(path) = track_path {
            for (idx, playlist) in self.playlists.iter().enumerate() {
                if playlist.tracks.iter().any(|track| track.path() == *path) {
                    self.current_playlist_idx = Some(idx);
                    self.playing_playlist_idx = Some(idx);
                    tracing::info!(
                        "Found last played track in playlist '{}', selecting it",
                        playlist.get_name().unwrap_or_default()
                    );
                    return;
                }
            }
        }

        // If no playlist was selected, select first one
        if self.current_playlist_idx.is_none() && !self.playlists.is_empty() {
            self.current_playlist_idx = Some(0);
            tracing::info!("No last played track found, selecting first playlist");
        }
    }

    /// Create a default playlist
    fn create_default_playlist(&mut self) {
        let mut default_playlist = Playlist::new();
        default_playlist.set_name("Default Playlist".to_string());
        self.playlists = vec![default_playlist];
        self.current_playlist_idx = Some(0);
        tracing::info!("Created default playlist");
    }

    /// Set language and update i18n
    pub fn set_language(&mut self, lang: Language) {
        self.current_language = lang;
        crate::app::i18n::set_language(lang);
    }

    /// Mark app for quit
    pub fn quit(&mut self) {
        self.quit = true;
    }
}
