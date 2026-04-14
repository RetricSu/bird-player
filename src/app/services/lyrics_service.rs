use std::sync::Arc;
use std::sync::Mutex;

use crate::app::lib_services::LyricsManager;
use crate::app::library::Library;
use crate::app::player::Player;
use crate::app::state::ui_state::UiState;

/// Application-level lyrics service that coordinates lyrics operations
///
/// This service acts as a coordinator between the low-level LyricsManager
/// and the application state, handling UI updates and data persistence.
pub struct LyricsService {
    lyrics_manager: LyricsManager,
}

impl Default for LyricsService {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsService {
    pub fn new() -> Self {
        Self {
            lyrics_manager: LyricsManager::new(),
        }
    }

    /// Get reference to the underlying lyrics manager
    pub fn manager(&self) -> &LyricsManager {
        &self.lyrics_manager
    }

    /// Get mutable reference to the underlying lyrics manager
    pub fn manager_mut(&mut self) -> &mut LyricsManager {
        &mut self.lyrics_manager
    }

    /// Update lyrics for a track and persist to database
    pub fn update_track_lyrics(
        &mut self,
        track_key: String,
        lyrics: Option<&str>,
        library: &mut Library,
        playlists: &mut [crate::app::Playlist],
        player: Option<&mut Player>,
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) {
        let lyrics_owned = library.update_item_lyrics(track_key.clone(), lyrics);

        // Update lyrics in all playlists
        for playlist in playlists.iter_mut() {
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

        // Update lyrics in player if track is currently playing
        if let Some(player) = player {
            if let Some(selected_track) = player.selected_track.as_mut() {
                if selected_track.key() == track_key {
                    selected_track.replace_lyrics(lyrics_owned.clone());

                    if lyrics_owned.is_none() {
                        self.lyrics_manager.set_current_lyrics(None);
                    }
                }
            }
        }

        // Persist to database
        let lyrics_param = lyrics_owned.as_deref();
        let update_result = {
            match db_conn.lock() {
                Ok(conn_guard) => conn_guard.execute(
                    "UPDATE library_items SET lyrics = ?1 WHERE key = ?2",
                    rusqlite::params![lyrics_param, track_key],
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
                if let Err(e) = library.save_to_db(db_conn) {
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

    /// Handle lyrics response from async fetch
    pub fn handle_lyrics_response(
        &mut self,
        should_show_panel: bool,
        ui_state: &mut UiState,
    ) -> Option<String> {
        tracing::debug!("📡 Lyrics response received");

        if should_show_panel {
            ui_state.show_lyrics_panel = true;
        }

        let lyrics_text_owned = self
            .lyrics_manager
            .current_lyrics()
            .and_then(|lyrics_data| {
                lyrics_data
                    .synced_lyrics
                    .as_deref()
                    .or(lyrics_data.plain_lyrics.as_deref())
                    .map(|text| text.to_string())
            });

        if let Some(lyrics_data) = self.lyrics_manager.current_lyrics() {
            if lyrics_data.instrumental {
                tracing::info!("✅ Found instrumental track");
            } else if !lyrics_data.lines.is_empty() || lyrics_data.plain_lyrics.is_some() {
                tracing::info!("✅ Lyrics loaded successfully");
            } else {
                tracing::warn!("⚠️  Lyrics record found but no content available");
            }
        } else {
            tracing::warn!("❌ No lyrics found");
        }

        lyrics_text_owned
    }

    /// Check for pending lyrics and update UI state
    pub fn check_pending_lyrics(&mut self, ui_state: &mut UiState) -> (bool, bool) {
        self.lyrics_manager
            .check_pending_lyrics(&mut ui_state.lyrics_fetch_state)
    }
}
