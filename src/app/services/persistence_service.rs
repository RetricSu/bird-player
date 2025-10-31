use std::sync::Arc;
use std::sync::Mutex;

use crate::app::error::AppLoadError;
use crate::app::i18n;
use crate::app::library::Library;
use crate::app::libstate::player_state::PlayerStateManager;
use crate::app::player::{Player, TrackState};
use crate::app::playlist::Playlist;
use crate::app::state::persistence::AppConfig;
use crate::app::state::StatePersistence;

/// Application-level persistence service that coordinates all data persistence operations
///
/// This service acts as a coordinator between the application state and various
/// persistence backends (confy for config, SQLite for library/playlists).
#[derive(Default)]
pub struct PersistenceService;

impl PersistenceService {
    /// Load basic application configuration
    pub fn load_basic_config() -> Result<AppConfig, AppLoadError> {
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

        Ok(config)
    }

    /// Load heavy data (library and playlists) from database
    pub fn load_heavy_data(
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
        playlists: &mut Vec<Playlist>,
        library: &mut Library,
        player_state: &PlayerStateManager,
    ) -> (Option<usize>, Option<usize>, bool) {
        tracing::info!("Loading heavy data (library and playlists)...");

        // Load library
        match StatePersistence::load_library(db_conn) {
            Ok(loaded_library) => {
                *library = loaded_library;
                tracing::info!("Successfully loaded library from database");
            }
            Err(e) => {
                tracing::error!("Failed to load library from database: {}", e);
            }
        }

        // Load playlists
        let mut current_playlist_idx = None;
        let mut playing_playlist_idx = None;

        match StatePersistence::load_playlists(db_conn) {
            Ok(loaded_playlists) => {
                if !loaded_playlists.is_empty() {
                    *playlists = loaded_playlists;

                    // Find playlist containing the last played track
                    if let Some(last_track_path) = &player_state.last_track_path {
                        for (idx, playlist) in playlists.iter().enumerate() {
                            if playlist
                                .tracks
                                .iter()
                                .any(|track| track.path() == *last_track_path)
                            {
                                current_playlist_idx = Some(idx);
                                playing_playlist_idx = Some(idx);
                                tracing::info!(
                                    "Found last played track in playlist '{}', selecting it",
                                    playlist.get_name().unwrap_or_default()
                                );
                                break;
                            }
                        }
                    }

                    // Select first playlist if none selected
                    if current_playlist_idx.is_none() {
                        current_playlist_idx = Some(0);
                        tracing::info!("No last played track found, selecting first playlist");
                    }
                } else {
                    // Create default playlist
                    let mut default_playlist = Playlist::new();
                    default_playlist.set_name("Default Playlist".to_string());
                    playlists.push(default_playlist);
                    current_playlist_idx = Some(0);
                    playing_playlist_idx = Some(0);
                    tracing::info!("No playlists found in database, created default playlist");
                }
            }
            Err(e) => {
                tracing::error!("Failed to load playlists from database: {}", e);
            }
        }

        // Check if lyrics should be fetched on init
        let should_fetch_lyrics = player_state.was_playing.unwrap_or(false);

        (
            current_playlist_idx,
            playing_playlist_idx,
            should_fetch_lyrics,
        )
    }

    /// Save all application state
    pub fn save_state(
        config: &AppConfig,
        library: &Library,
        playlists: &mut [Playlist],
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) {
        // Update config from current state - this will be handled by caller

        // Save config to confy
        match StatePersistence::save_config(config) {
            Ok(_) => tracing::info!("Settings stored successfully"),
            Err(err) => tracing::error!("Failed to store app settings: {}", err),
        }

        // Save library and playlists to SQLite
        // Save library
        if let Err(e) = StatePersistence::save_library(library, db_conn) {
            tracing::error!("Failed to save library to database: {}", e);
        }

        // Save playlists
        if let Err(e) = StatePersistence::save_playlists(playlists, db_conn) {
            tracing::error!("Failed to save playlists to database: {}", e);
        }
    }

    /// Update player state for persistence
    pub fn update_player_persistence(player: &Player, player_state: &mut PlayerStateManager) {
        let last_track_path = player.selected_track.as_ref().map(|track| track.path());
        let last_position = Some(player.seek_to_timestamp);
        let last_playback_mode = Some(player.playback_mode);
        let last_volume = Some(player.volume);
        let was_playing = Some(matches!(player.track_state, TrackState::Playing));

        player_state.last_track_path = last_track_path;
        player_state.last_position = last_position;
        player_state.last_playback_mode = last_playback_mode;
        player_state.last_volume = last_volume;
        player_state.was_playing = was_playing;
    }
}
