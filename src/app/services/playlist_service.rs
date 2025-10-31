use crate::app::playlist::Playlist;

/// Application-level playlist service that coordinates all playlist management operations
///
/// This service acts as a coordinator between the application state and playlist
/// operations (creation, deletion, selection, renaming, etc.).
#[derive(Default)]
pub struct PlaylistService;

impl PlaylistService {
    /// Create a new playlist and return its index
    pub fn create_playlist(
        playlists: &mut Vec<Playlist>,
        current_playlist_idx: &mut Option<usize>,
        playlist_being_renamed: &mut Option<usize>,
        default_name: String,
    ) -> usize {
        let mut new_playlist = Playlist::new();
        new_playlist.set_name(default_name);
        playlists.push(new_playlist);
        let new_idx = playlists.len() - 1;
        *current_playlist_idx = Some(new_idx);
        *playlist_being_renamed = Some(new_idx); // Start renaming the new playlist immediately
        new_idx
    }

    /// Delete a playlist at the specified index and update indices accordingly
    pub fn delete_playlist(
        playlists: &mut Vec<Playlist>,
        current_playlist_idx: &mut Option<usize>,
        playlist_idx_to_remove: usize,
    ) {
        if playlist_idx_to_remove >= playlists.len() {
            return;
        }

        // Update current playlist index if necessary
        if let Some(mut current_idx) = *current_playlist_idx {
            if current_idx == 0 && playlist_idx_to_remove == 0 {
                *current_playlist_idx = None;
            } else if current_idx >= playlist_idx_to_remove {
                current_idx = current_idx.saturating_sub(1);
                *current_playlist_idx = Some(current_idx);
            }
        }

        playlists.remove(playlist_idx_to_remove);
    }

    /// Select a playlist as the current playlist
    pub fn select_playlist(current_playlist_idx: &mut Option<usize>, playlist_idx: usize) {
        *current_playlist_idx = Some(playlist_idx);
    }

    /// Start renaming a playlist
    pub fn start_renaming_playlist(
        playlist_being_renamed: &mut Option<usize>,
        playlist_idx: usize,
    ) {
        *playlist_being_renamed = Some(playlist_idx);
    }

    /// Finish renaming a playlist (called from within a playlist iteration)
    pub fn finish_renaming_playlist_ui(playlist_being_renamed: &mut Option<usize>) {
        *playlist_being_renamed = None;
    }

    /// Cancel renaming a playlist
    pub fn cancel_renaming_playlist(playlist_being_renamed: &mut Option<usize>) {
        *playlist_being_renamed = None;
    }

    /// Get the current playlist (mutable reference)
    pub fn get_current_playlist_mut<'a>(
        playlists: &'a mut Vec<Playlist>,
        current_playlist_idx: Option<usize>,
    ) -> Option<&'a mut Playlist> {
        current_playlist_idx.and_then(|idx| playlists.get_mut(idx))
    }

    /// Get the current playlist (immutable reference)
    pub fn get_current_playlist<'a>(
        playlists: &'a Vec<Playlist>,
        current_playlist_idx: Option<usize>,
    ) -> Option<&'a Playlist> {
        current_playlist_idx.and_then(|idx| playlists.get(idx))
    }
}
