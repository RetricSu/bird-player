use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::player::PlaybackMode;

/// Player state manager - handles player state persistence and restoration
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
pub struct PlayerStateManager {
    /// Last played track path
    pub last_track_path: Option<PathBuf>,

    /// Last playback position in milliseconds
    pub last_position: Option<u64>,

    /// Last playback mode
    pub last_playback_mode: Option<PlaybackMode>,

    /// Last volume level
    pub last_volume: Option<f32>,

    /// Whether the player was playing when app closed
    pub was_playing: Option<bool>,
}

impl PlayerStateManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear the player state after restoration
    pub fn clear_restore_state(&mut self) {
        self.last_track_path = None;
        self.last_position = None;
        // Keep playback mode and volume
    }

    /// Update state from current player
    pub fn update_from_player(&mut self, player: &crate::player::Player) {
        self.last_track_path = player.selected_track.as_ref().map(|track| track.path());
        self.last_position = Some(player.seek_to_timestamp);
        self.last_playback_mode = Some(player.playback_mode);
        self.last_volume = Some(player.volume);
        self.was_playing = Some(matches!(
            player.track_state,
            crate::player::TrackState::Playing
        ));
    }
}
