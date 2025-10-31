use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::error::AppLoadError;
use super::i18n;
use super::lib_services::{
    LibraryImportService, LyricsManager, MetadataEditor, PlayerRestoreService,
};
use super::library::{Library, LibraryCommand, LibraryPath};
pub use super::library::{LibraryItem, LibraryPathId};
use super::libstate::lyrics_state::LyricsFetchState;
use super::libstate::player_state::PlayerStateManager;
use super::player::{Player, TrackState};
pub use super::playlist::Playlist;
use super::state::StatePersistence;
use super::state::{persistence::AppConfig, ui_state::UiState};
use crate::app::bootstrap;
use crate::app::db;
use crate::app::runtime;
use crate::app::services::LyricsService;

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

    // Persisted settings
    pub config: AppConfig,

    // Runtime state (not serialized)
    #[serde(skip_serializing, skip_deserializing)]
    pub boot_cfg: Option<bootstrap::BirdBootCfg>,

    #[serde(skip_serializing, skip_deserializing)]
    pub runtime: Option<runtime::BirdRuntime>,

    #[serde(skip_serializing, skip_deserializing)]
    pub ui_state: UiState,

    #[serde(skip_serializing, skip_deserializing)]
    pub lyrics_service: LyricsService,

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
            config: AppConfig::default(),
            boot_cfg: None,
            runtime: None,
            ui_state: UiState::default(),
            lyrics_service: LyricsService::new(),
            heavy_data_loaded: false,
            quit: false,
        }
    }
}

impl App {
    // 便捷访问器 - boot_cfg
    pub fn db(&self) -> &Arc<db::Database> {
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

    // 便捷访问器 - player state
    pub fn player_state(&self) -> &PlayerStateManager {
        &self.config.player
    }

    pub fn player_state_mut(&mut self) -> &mut PlayerStateManager {
        &mut self.config.player
    }

    /// Access to lyrics manager for UI components
    pub fn lyrics_manager(&self) -> &LyricsManager {
        self.lyrics_service.manager()
    }

    pub fn lyrics_manager_mut(&mut self) -> &mut LyricsManager {
        self.lyrics_service.manager_mut()
    }

    pub fn load_basic() -> Result<Self, AppLoadError> {
        // Initialize i18n
        i18n::init();

        // Load settings from confy
        let config = StatePersistence::load_config().unwrap_or_else(|err| {
            tracing::warn!(
                error = %AppLoadError::MissingAppState,
                "Falling back to default settings: {err}"
            );
            AppConfig::default()
        });

        let mut app = App {
            config: config.clone(),
            ..Default::default()
        };

        // Apply settings to state managers
        app.ui_state.apply_settings(config.ui);

        // Set the language
        i18n::set_language(config.current_language);

        Ok(app)
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
                    if let Some(last_track_path) = &self.player_state().last_track_path {
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
        // Update config from current state
        self.config.ui = self.ui_state.to_settings();

        // Save config to confy
        match StatePersistence::save_config(&self.config) {
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
            let last_track_path = runtime
                .player
                .selected_track
                .as_ref()
                .map(|track| track.path());
            let last_position = Some(runtime.player.seek_to_timestamp);
            let last_playback_mode = Some(runtime.player.playback_mode);
            let last_volume = Some(runtime.player.volume);
            let was_playing = Some(matches!(runtime.player.track_state, TrackState::Playing));

            let player_state = self.player_state_mut();
            player_state.last_track_path = last_track_path;
            player_state.last_position = last_position;
            player_state.last_playback_mode = last_playback_mode;
            player_state.last_volume = last_volume;
            player_state.was_playing = was_playing;
        }
    }

    /// Restore player state from saved settings
    ///
    /// This function is called during application startup to restore the player's
    /// last state including volume, playback mode, track position, and playback status.
    pub fn restore_player_state(&mut self) {
        // Get is_processing first before borrowing anything else
        let is_processing = self.is_processing_ui_change();

        let (playing_playlist_idx, should_fetch_lyrics) = {
            let runtime: &mut runtime::BirdRuntime =
                self.runtime.as_mut().expect("runtime not initialized");

            let playlists = &self.playlists;

            PlayerRestoreService::restore_player_state(
                &mut runtime.player,
                &mut self.config.player,
                playlists,
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

    // Spawns a background thread and imports files from a library path
    pub fn import_library_paths(&self, lib_path: &LibraryPath) {
        let lib_cmd_tx = self.lib_cmd_tx().clone();
        let album_art_dir = App::get_album_art_dir();

        LibraryImportService::import_library_path(lib_path, lib_cmd_tx, album_art_dir);
    }

    pub fn update_track_lyrics(&mut self, track_key: usize, lyrics: Option<&str>) {
        let db_conn = self.db().connection();
        let player = self.runtime.as_mut().map(|rt| &mut rt.player);

        self.lyrics_service.update_track_lyrics(
            track_key,
            lyrics,
            &mut self.library,
            &mut self.playlists,
            player,
            &db_conn,
        );
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
        self.config.current_language = lang;
        i18n::set_language(lang);
        // Save state to persist language preference
        self.save_state();
    }

    pub fn get_language(&self) -> i18n::Language {
        self.config.current_language
    }

    /// Process a freshly received lyrics response.
    pub fn handle_lyrics_response(&mut self, should_show_panel: bool) {
        let track_key = self
            .runtime
            .as_ref()
            .and_then(|rt| rt.player.selected_track.as_ref().map(|track| track.key()));

        let lyrics_text = self
            .lyrics_service
            .handle_lyrics_response(should_show_panel, &mut self.ui_state);

        if let Some(track_key) = track_key {
            if let Some(lyrics) = &lyrics_text {
                self.update_track_lyrics(track_key, Some(lyrics.as_str()));
            }
        }

        // Handle lyrics caching to ID3 tag
        if let Some(lyrics_data) = self.lyrics_service.manager().current_lyrics() {
            if self.runtime.is_some() {
                let player = self.player_ref();
                if let Some(track) = &player.selected_track {
                    if let Err(e) = crate::app::lyrics::LyricsService::write_lyrics_to_file(
                        track.path(),
                        lyrics_data,
                    ) {
                        tracing::warn!("⚠️  Failed to cache lyrics in ID3 tag: {}", e);
                    } else {
                        tracing::info!("📝 Successfully cached lyrics in ID3 tag");
                    }
                }
            }
        }
    }

    pub fn process_library_command(&mut self, lib_cmd: LibraryCommand) {
        match lib_cmd {
            LibraryCommand::AddItem(lib_item) => self.library.add_item(lib_item),
            LibraryCommand::AddView(lib_view) => self.library.add_view(lib_view),
            LibraryCommand::AddPathId(path_id) => self.library.set_path_to_imported(path_id),
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

        // Get data we need before any mutable borrows
        let selected_track = self.runtime.as_ref().unwrap().player.selected_track.clone();
        let selected_track_key = selected_track.as_ref().map(|t| t.key());

        // Now we can mutate
        if let Some(track) = &selected_track {
            let (lyrics_found, should_show_panel) = self
                .lyrics_service
                .manager_mut()
                .fetch_lyrics_for_track_data(Some(track), &mut self.ui_state.lyrics_fetch_state);

            if lyrics_found && should_show_panel {
                self.ui_state.show_lyrics_panel = true;
            }
        } else {
            tracing::warn!("⚠️  No track selected for lyrics fetch");
            self.ui_state.lyrics_fetch_state =
                LyricsFetchState::Failed("No track selected".to_string());
        }

        // Handle cached lyrics update
        let lyrics_text_owned: Option<String> = self
            .lyrics_service
            .manager()
            .current_lyrics()
            .and_then(|lyrics| {
                lyrics
                    .synced_lyrics
                    .as_ref()
                    .or(lyrics.plain_lyrics.as_ref())
                    .map(|s| s.to_owned())
            });

        if let (Some(track_key), Some(lyrics_text)) = (selected_track_key, lyrics_text_owned) {
            self.update_track_lyrics(track_key, Some(lyrics_text.as_str()));
        }
    }
}
