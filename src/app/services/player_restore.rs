use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::app::{
    library::LibraryItem, player::Player, playlist::Playlist, state::PlayerStateManager,
};

/// Service for restoring player state on app startup
pub struct PlayerRestoreService;

impl PlayerRestoreService {
    /// Restore player state from saved settings
    ///
    /// This function is called during application startup to restore the player's
    /// last state including volume, playback mode, track position, and playback status.
    pub fn restore_player_state(
        player: &mut Player,
        player_state: &mut PlayerStateManager,
        playlists: &[Playlist],
        is_processing: Arc<AtomicBool>,
    ) -> (Option<usize>, bool) {
        tracing::info!("Restoring player state...");
        tracing::info!("Last track path: {:?}", player_state.last_track_path);
        tracing::info!("Last position: {:?}", player_state.last_position);
        tracing::info!("Was playing: {:?}", player_state.was_playing);
        tracing::info!("Number of playlists: {}", playlists.len());

        // Restore volume
        if let Some(volume) = player_state.last_volume {
            player.set_volume(volume, &is_processing);
            tracing::info!("Restored volume: {}", volume);
        }

        // Restore playback mode
        if let Some(mode) = player_state.last_playback_mode {
            player.playback_mode = mode;
            tracing::info!("Restored playback mode: {:?}", mode);
        }

        let mut playing_playlist_idx = None;
        let mut should_fetch_lyrics = false;

        // Restore track and playback state
        if let Some(track_path) = &player_state.last_track_path {
            if let Some((playlist_idx, track)) =
                Self::find_track_in_playlists(playlists, track_path)
            {
                tracing::info!("Restoring track: {:?}", track_path);

                // Set selected track
                player.select_track(Some(track));

                // Restore seek position
                if let Some(position) = player_state.last_position {
                    tracing::info!("Restoring position: {} ms", position);
                    player.seek_to(position);
                }

                // Resume playback if it was playing
                if let Some(true) = player_state.was_playing {
                    tracing::info!("Resuming playback");
                    player.play();
                    playing_playlist_idx = Some(playlist_idx);
                }

                should_fetch_lyrics = true;
            } else {
                tracing::warn!("Cannot find saved track in any playlist: {:?}", track_path);
            }
        } else {
            tracing::info!("No previous track to restore");
        }

        // Clear the saved state now that we've restored it
        player_state.clear_restore_state();

        (playing_playlist_idx, should_fetch_lyrics)
    }

    /// Find a track in playlists by its path
    fn find_track_in_playlists(
        playlists: &[Playlist],
        track_path: &std::path::PathBuf,
    ) -> Option<(usize, LibraryItem)> {
        for (playlist_idx, playlist) in playlists.iter().enumerate() {
            if let Some(track) = playlist
                .tracks
                .iter()
                .find(|track| track.path() == *track_path)
            {
                return Some((playlist_idx, track.clone()));
            }
        }
        None
    }
}
