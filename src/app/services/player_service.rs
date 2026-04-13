use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::app::player::{PlaybackMode, Player};
use crate::app::playlist::Playlist;

/// Application-level player service that coordinates all player control operations
///
/// This service acts as a coordinator between the application state and player
/// operations (playback control, track navigation, volume control, etc.).
#[derive(Default)]
pub struct PlayerService;

impl PlayerService {
    /// Start or resume playback
    pub fn play(player: &mut Player) {
        player.play();
    }

    /// Pause playback
    pub fn pause(player: &mut Player) {
        player.pause();
    }

    /// Play the next track in the playlist
    pub fn next_track(player: &mut Player, playlist: &Playlist) {
        player.next(playlist);
    }

    /// Play the previous track in the playlist
    pub fn previous_track(player: &mut Player, playlist: &Playlist) {
        player.previous(playlist);
    }

    /// Toggle between playback modes (Normal -> Repeat -> RepeatOne -> Shuffle -> Normal)
    pub fn toggle_playback_mode(player: &mut Player) {
        player.toggle_playback_mode();
    }

    /// Set the volume level
    pub fn set_volume(player: &mut Player, volume: f32, is_processing: &Arc<AtomicBool>) {
        player.set_volume(volume, is_processing);
    }

    /// Seek to a specific timestamp in the current track
    pub fn seek_to(player: &mut Player, timestamp: u64) {
        player.seek_to(timestamp);
    }

    /// Set the seek timestamp (for UI updates without immediate seeking)
    pub fn set_seek_to_timestamp(player: &mut Player, timestamp: u64) {
        player.set_seek_to_timestamp(timestamp);
    }

    /// Set the total duration of the current track
    pub fn set_duration(player: &mut Player, duration: u64) {
        player.set_duration(duration);
    }

    /// Remove the currently selected track from the playlist and handle playback continuation
    /// Returns the track key that was removed, or None if no track was removed
    pub fn remove_current_track(player: &mut Player) -> Option<String> {
        if let Some(track) = &player.selected_track {
            let track_key = track.key();
            // Clear the selected track - the playlist removal will be handled by the caller
            player.select_track(None);
            Some(track_key)
        } else {
            None
        }
    }

    /// Get the current playback state
    pub fn is_playing(player: &Player) -> bool {
        matches!(player.track_state, crate::app::player::TrackState::Playing)
    }

    /// Get the current playback mode
    pub fn get_playback_mode(player: &Player) -> PlaybackMode {
        player.playback_mode
    }

    /// Get the current volume
    pub fn get_volume(player: &Player) -> f32 {
        player.volume
    }

    /// Get the current seek timestamp
    pub fn get_seek_timestamp(player: &Player) -> u64 {
        player.seek_to_timestamp
    }

    /// Get the current track duration
    pub fn get_duration(player: &Player) -> u64 {
        player.duration
    }
}
