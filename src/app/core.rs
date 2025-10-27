use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::error::AppLoadError;
use super::i18n;
use super::library::{Library, LibraryCommand, LibraryPath};
use super::player::Player;
use super::services::{LibraryImportService, LyricsManager, MetadataEditor, PlayerRestoreService};
use super::state::{persistence::AppSettings, ui_state::LyricsFetchState, ui_state::UiState};
use super::state::{PlayerStateManager, StatePersistence};

pub use super::library::{LibraryItem, LibraryPathId};
pub use super::playlist::Playlist;

/// Main application struct
///
/// Refactored to separate concerns into dedicated state managers
#[derive(Serialize, Deserialize)]
pub struct App {
    // Core data
    pub library: Library,
    pub playlists: Vec<Playlist>,
    pub current_playlist_idx: Option<usize>,
    pub playing_playlist_idx: Option<usize>,
    pub current_language: i18n::Language,

    // Persisted UI settings
    pub library_folders_expanded: bool,
    pub default_window_height: f64,

    // Runtime state (not serialized)
    #[serde(skip_serializing, skip_deserializing)]
    pub boot_cfg: Option<crate::BirdBootCfg>,

    #[serde(skip_serializing, skip_deserializing)]
    pub runtime: Option<crate::BirdRuntime>,

    #[serde(skip_serializing, skip_deserializing)]
    pub player_state: PlayerStateManager,

    #[serde(skip_serializing, skip_deserializing)]
    pub ui_state: UiState,

    #[serde(skip_serializing, skip_deserializing)]
    pub lyrics_manager: LyricsManager,

    #[serde(skip_serializing, skip_deserializing)]
    pub last_window_title: Option<String>,

    #[serde(skip_serializing, skip_deserializing)]
    pub heavy_data_loaded: bool,

    pub quit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            library: Library::new(),
            playlists: vec![],
            current_playlist_idx: None,
            playing_playlist_idx: None,
            current_language: i18n::Language::English,
            library_folders_expanded: false,
            default_window_height: super::constants::DEFAULT_WINDOW_HEIGHT as f64,
            boot_cfg: None,
            runtime: None,
            player_state: PlayerStateManager::default(),
            ui_state: UiState::default(),
            lyrics_manager: LyricsManager::new(),
            last_window_title: None,
            heavy_data_loaded: false,
            quit: false,
        }
    }
}

impl App {
    // 便捷访问器 - boot_cfg
    pub fn db(&self) -> &Arc<crate::db::Database> {
        &self.boot_cfg.as_ref().expect("boot_cfg not initialized").db
    }

    pub fn lib_cmd_tx(&self) -> &Sender<LibraryCommand> {
        &self
            .boot_cfg
            .as_ref()
            .expect("boot_cfg not initialized")
            .lib_cmd_tx
    }

    pub fn lib_cmd_rx(&self) -> &Receiver<LibraryCommand> {
        &self
            .boot_cfg
            .as_ref()
            .expect("boot_cfg not initialized")
            .lib_cmd_rx
    }

    pub fn is_processing_ui_change(&self) -> Arc<AtomicBool> {
        Arc::clone(
            &self
                .boot_cfg
                .as_ref()
                .expect("boot_cfg not initialized")
                .is_processing_ui_change,
        )
    }

    // 便捷访问器 - runtime
    pub fn player_ref(&self) -> &Player {
        &self
            .runtime
            .as_ref()
            .expect("runtime not initialized")
            .player
    }

    pub fn player_mut_ref(&mut self) -> &mut Player {
        &mut self
            .runtime
            .as_mut()
            .expect("runtime not initialized")
            .player
    }

    pub fn load_basic() -> Result<Self, AppLoadError> {
        let mut app = App::default();

        // Initialize i18n
        i18n::init();

        // Load settings from confy
        if let Ok(settings) = StatePersistence::load_settings() {
            // Apply settings
            app.current_language = settings.current_language;
            app.player_state.apply_settings(settings.player);
            app.ui_state.apply_settings(settings.ui);
        }

        // Set the language from the loaded config
        i18n::set_language(app.current_language);

        Ok(app)
    }

    pub fn load() -> Result<Self, AppLoadError> {
        // Load basic app state first
        let mut app = Self::load_basic()?;

        // Now load the heavy data (library and playlists) if we have boot_cfg
        let db_connection = app.db().connection();
        // Try to load library from database
        match Library::load_from_db(&db_connection) {
            Ok(library) => {
                app.library = library;
                tracing::info!("Successfully loaded library from database");
            }
            Err(e) => {
                tracing::error!("Failed to load library from database: {}", e);
                // Keep the default empty library
            }
        }

        // Try to load playlists from database
        match Playlist::load_all_from_db(&db_connection) {
            Ok(playlists) => {
                if !playlists.is_empty() {
                    app.playlists = playlists;

                    // If there was a last played track, try to find its playlist
                    if let Some(last_track_path) = &app.player_state.last_track_path {
                        for (idx, playlist) in app.playlists.iter().enumerate() {
                            if playlist
                                .tracks
                                .iter()
                                .any(|track| track.path() == *last_track_path)
                            {
                                app.current_playlist_idx = Some(idx);
                                app.playing_playlist_idx = Some(idx);
                                tracing::info!(
                                    "Found last played track in playlist '{}', selecting it",
                                    playlist.get_name().unwrap_or_default()
                                );
                                break;
                            }
                        }
                    }

                    // If no playlist was selected (no last track or track not found), select first playlist
                    if app.current_playlist_idx.is_none() {
                        app.current_playlist_idx = Some(0);
                        tracing::info!("No last played track found, selecting first playlist");
                    }
                } else {
                    // Only create a default playlist if no playlists exist in the database
                    let mut default_playlist = Playlist::new();
                    default_playlist.set_name("Default Playlist".to_string());
                    app.playlists = vec![default_playlist];
                    app.current_playlist_idx = Some(0);
                    tracing::info!("No playlists found in database, created default playlist");
                }
            }
            Err(e) => {
                tracing::error!("Failed to load playlists from database: {}", e);
                // Keep the default playlist
            }
        }

        Ok(app)
    }

    pub fn start_async_loading(&mut self) {
        if self.heavy_data_loaded {
            return;
        }

        tracing::info!("Starting async heavy data loading...");

        // Clone the database connection for the background thread
        let db_connection = self.db().clone();

        // Start loading in a background thread
        std::thread::spawn(move || {
            // Load library
            let _library_result = Library::load_from_db(&db_connection.connection());

            // Load playlists
            let _playlists_result = Playlist::load_all_from_db(&db_connection.connection());

            tracing::info!("Async loading completed");
        });
    }

    pub fn load_heavy_data(&mut self) {
        if self.heavy_data_loaded {
            return;
        }

        tracing::info!("Loading heavy data (library and playlists)...");

        let db_connection = self.db().connection();

        // Load library
        match StatePersistence::load_library(&db_connection) {
            Ok(library) => {
                self.library = library;
                tracing::info!("Successfully loaded library from database");
            }
            Err(e) => {
                tracing::error!("Failed to load library from database: {}", e);
            }
        }

        // Load playlists
        match StatePersistence::load_playlists(&db_connection) {
            Ok(playlists) => {
                if !playlists.is_empty() {
                    self.playlists = playlists;

                    // Find playlist containing the last played track
                    if let Some(last_track_path) = &self.player_state.last_track_path {
                        for (idx, playlist) in self.playlists.iter().enumerate() {
                            if playlist
                                .tracks
                                .iter()
                                .any(|track| track.path() == *last_track_path)
                            {
                                self.current_playlist_idx = Some(idx);
                                self.playing_playlist_idx = Some(idx);
                                tracing::info!(
                                    "Found last played track in playlist '{}', selecting it",
                                    playlist.get_name().unwrap_or_default()
                                );
                                break;
                            }
                        }
                    }

                    // Select first playlist if none selected
                    if self.current_playlist_idx.is_none() {
                        self.current_playlist_idx = Some(0);
                        tracing::info!("No last played track found, selecting first playlist");
                    }
                } else {
                    // Create default playlist
                    let mut default_playlist = Playlist::new();
                    default_playlist.set_name("Default Playlist".to_string());
                    self.playlists = vec![default_playlist];
                    self.current_playlist_idx = Some(0);
                    tracing::info!("No playlists found in database, created default playlist");
                }
            }
            Err(e) => {
                tracing::error!("Failed to load playlists from database: {}", e);
            }
        }

        // Restore player state after heavy data is loaded
        self.restore_player_state();

        // Fetch lyrics for restored track if needed
        if self.ui_state.should_fetch_lyrics_on_init {
            self.fetch_lyrics_for_current_track();
            self.ui_state.should_fetch_lyrics_on_init = false;
        }

        self.heavy_data_loaded = true;
        tracing::info!("Heavy data loading completed");
    }

    pub fn get_album_art_dir() -> PathBuf {
        confy::get_configuration_file_path("bird-player", None)
            .map(|p| {
                p.parent()
                    .map_or_else(|| PathBuf::from("album_art"), |path| path.join("album_art"))
            })
            .unwrap_or_else(|_| PathBuf::from("album_art"))
    }

    pub fn save_state(&mut self) {
        // Build settings from current state
        let settings = AppSettings {
            current_language: self.current_language,
            player: self.player_state.to_settings(),
            ui: self.ui_state.to_settings(),
        };

        // Save settings to confy
        match StatePersistence::save_settings(&settings) {
            Ok(_) => tracing::info!("Settings stored successfully"),
            Err(err) => tracing::error!("Failed to store app settings: {}", err),
        }

        // Save library and playlists to SQLite
        let db_conn = self.db().connection();

        // Save library
        if let Err(e) = StatePersistence::save_library(&self.library, &db_conn) {
            tracing::error!("Failed to save library to database: {}", e);
        }

        // Save playlists
        if let Err(e) = StatePersistence::save_playlists(&mut self.playlists, &db_conn) {
            tracing::error!("Failed to save playlists to database: {}", e);
        }
    }

    /// Capture the current player state for persistence
    pub fn update_player_persistence(&mut self) {
        if let Some(runtime) = &self.runtime {
            self.player_state.update_from_player(&runtime.player);
        }
    }

    /// Restore player state from saved settings
    ///
    /// This function is called during application startup to restore the player's
    /// last state including volume, playback mode, track position, and playback status.
    pub fn restore_player_state(&mut self) {
        // Get is_processing first before borrowing anything else
        let is_processing = self.is_processing_ui_change();

        // Split borrows: we need mutable access to player (via runtime)
        // and mutable access to player_state, and immutable access to playlists.
        // Since all are different fields, we can do this via pointer manipulation.

        let runtime_ptr =
            self.runtime.as_mut().expect("runtime not initialized") as *mut crate::BirdRuntime;
        let player_state_ptr = &mut self.player_state as *mut PlayerStateManager;
        let playlists_ref = &self.playlists;

        // SAFETY: We're accessing different fields of self, so there's no aliasing.
        // runtime.player is separate from player_state and playlists.
        let (playing_playlist_idx, should_fetch_lyrics) = unsafe {
            PlayerRestoreService::restore_player_state(
                &mut (*runtime_ptr).player,
                &mut *player_state_ptr,
                playlists_ref,
                is_processing,
            )
        };

        if let Some(idx) = playing_playlist_idx {
            self.playing_playlist_idx = Some(idx);
        }

        if should_fetch_lyrics {
            self.ui_state.should_fetch_lyrics_on_init = true;
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    // Spawns a background thread and imports files from a library path
    pub fn import_library_paths(&self, lib_path: &LibraryPath) {
        let lib_cmd_tx = self.lib_cmd_tx().clone();
        let album_art_dir = App::get_album_art_dir();

        LibraryImportService::import_library_path(lib_path, lib_cmd_tx, album_art_dir);
    }

    pub fn update_track_lyrics(&mut self, track_key: usize, lyrics: Option<&str>) {
        let lyrics_owned = self.library.update_item_lyrics(track_key, lyrics);

        for playlist in &mut self.playlists {
            for item in playlist.tracks.iter_mut() {
                if item.key() == track_key {
                    item.replace_lyrics(lyrics_owned.clone());
                }
            }

            if let Some(selected) = playlist.selected.as_mut() {
                if selected.key() == track_key {
                    selected.replace_lyrics(lyrics_owned.clone());
                }
            }
        }

        if self.runtime.is_some() {
            let player = self.player_mut_ref();
            if let Some(selected_track) = player.selected_track.as_mut() {
                if selected_track.key() == track_key {
                    selected_track.replace_lyrics(lyrics_owned.clone());

                    if lyrics_owned.is_none() {
                        self.lyrics_manager.set_current_lyrics(None);
                    }
                }
            }
        }

        let db = self.db();
        let conn_arc = db.connection();
        let lyrics_param = lyrics_owned.as_deref();

        let update_result = {
            match conn_arc.lock() {
                Ok(conn_guard) => conn_guard.execute(
                    "UPDATE library_items SET lyrics = ?1 WHERE key = ?2",
                    rusqlite::params![lyrics_param, track_key.to_string()],
                ),
                Err(e) => {
                    tracing::error!(
                        "Failed to acquire database lock for lyrics update on track {}: {}",
                        track_key,
                        e
                    );
                    return;
                }
            }
        };

        match update_result {
            Ok(0) => {
                if let Err(e) = self.library.save_to_db(&conn_arc) {
                    tracing::error!(
                        "Failed to persist lyrics update for track {}: {}",
                        track_key,
                        e
                    );
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(
                    "Failed to update lyrics in database for track {}: {}",
                    track_key,
                    e
                );
            }
        }
    }

    pub fn update_track_metadata(
        &mut self,
        track: &mut LibraryItem,
        field: &str,
        value: &str,
    ) -> bool {
        let db_conn = self.db().connection();

        // Use the MetadataEditor service
        let success = MetadataEditor::update_track_metadata(track, field, value, &db_conn);

        if success {
            // Update all instances of this track in playlists
            for playlist in &mut self.playlists {
                for playlist_track in playlist.tracks.iter_mut() {
                    if playlist_track.key() == track.key() {
                        let updated_track = match field {
                            "title" => playlist_track.set_title(Some(value)),
                            "artist" => playlist_track.set_artist(Some(value)),
                            "album" => playlist_track.set_album(Some(value)),
                            "genre" => playlist_track.set_genre(Some(value)),
                            _ => playlist_track.clone(),
                        };
                        *playlist_track = updated_track;
                    }
                }
            }

            // Reload the library from the database
            if let Ok(updated_library) = Library::load_from_db(&db_conn) {
                self.library = updated_library;
            }

            // Save the updated state
            self.save_state();
        }

        success
    }

    // Add these new methods for language handling
    pub fn set_language(&mut self, lang: i18n::Language) {
        self.current_language = lang;
        i18n::set_language(lang);
        // Save state to persist language preference
        self.save_state();
    }

    pub fn get_language(&self) -> i18n::Language {
        self.current_language
    }

    pub fn cleanup_duplicate_pictures(&self) {
        let db = self.db();
        let conn = db.connection();
        let conn_guard = conn.lock().unwrap();

        tracing::info!("Cleaning up duplicate pictures in database...");

        // Delete all duplicate pictures, keeping only the first one for each library_item_id
        let result = conn_guard.execute(
            "DELETE FROM pictures WHERE id NOT IN (
                SELECT MIN(id) FROM pictures GROUP BY library_item_id, mime_type, picture_type, description, file_path
            )",
            [],
        );

        match result {
            Ok(rows_deleted) => {
                tracing::info!("Cleaned up {} duplicate picture records", rows_deleted);
            }
            Err(e) => {
                tracing::error!("Failed to clean up duplicate pictures: {}", e);
            }
        }
    }

    /// Fetch lyrics for the currently selected track
    pub fn fetch_lyrics_for_current_track(&mut self) {
        if self.runtime.is_none() {
            tracing::warn!("⚠️  Player not available for lyrics fetch");
            self.ui_state.lyrics_fetch_state =
                LyricsFetchState::Failed("Player not available".to_string());
            return;
        }

        // Extract the data we need from player first
        let (selected_track, selected_track_key) = {
            let player = self.player_ref();
            let selected = player.selected_track.clone();
            let selected_key = player.selected_track.as_ref().map(|t| t.key());
            (selected, selected_key)
        };

        // Now we can mutate self without holding the player reference
        let (lyrics_found, should_show_panel) = self.lyrics_manager.fetch_lyrics_for_track_data(
            selected_track.as_ref(),
            &mut self.ui_state.lyrics_fetch_state,
        );

        if lyrics_found && should_show_panel {
            self.ui_state.show_lyrics_panel = true;

            // Update track lyrics if we got them from cache
            // Extract lyrics text and convert to owned String to avoid borrow issues
            let lyrics_text_owned: Option<String> = {
                self.lyrics_manager.current_lyrics().and_then(|lyrics| {
                    lyrics
                        .synced_lyrics
                        .as_ref()
                        .or(lyrics.plain_lyrics.as_ref())
                        .map(|s| s.to_owned())
                })
            };

            if let (Some(track_key), Some(lyrics_text)) = (selected_track_key, lyrics_text_owned) {
                self.update_track_lyrics(track_key, Some(&lyrics_text));
            }
        }
    }
}
