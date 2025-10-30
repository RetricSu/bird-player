use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::player::PlaybackMode;

/// Player state manager - handles player state persistence and restoration
#[derive(Default)]
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

/// Persistable player settings
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct PlayerSettings {
    pub last_track_path: Option<PathBuf>,
    pub last_position: Option<u64>,
    pub last_playback_mode: Option<PlaybackMode>,
    pub last_volume: Option<f32>,
    pub was_playing: Option<bool>,
}

impl PlayerStateManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Extract persistable settings from player state
    pub fn to_settings(&self) -> PlayerSettings {
        PlayerSettings {
            last_track_path: self.last_track_path.clone(),
            last_position: self.last_position,
            last_playback_mode: self.last_playback_mode,
            last_volume: self.last_volume,
            was_playing: self.was_playing,
        }
    }

    /// Apply settings to player state
    pub fn apply_settings(&mut self, settings: PlayerSettings) {
        self.last_track_path = settings.last_track_path;
        self.last_position = settings.last_position;
        self.last_playback_mode = settings.last_playback_mode;
        self.last_volume = settings.last_volume;
        self.was_playing = settings.was_playing;
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
