use crate::app::{App, Playlist};

/// Encapsulates non-UI operations triggered from the playlist table UI.
pub(crate) struct PlaylistTableService<'a> {
    ctx: &'a mut App,
    playlist_idx: usize,
}

impl<'a> PlaylistTableService<'a> {
    pub(crate) fn new(ctx: &'a mut App, playlist_idx: usize) -> Option<Self> {
        if playlist_idx < ctx.playlists.len() {
            Some(Self { ctx, playlist_idx })
        } else {
            None
        }
    }

    pub(crate) fn clear_lyrics(&mut self, track_key: String) {
        self.ctx.update_track_lyrics(track_key, None);
    }

    pub(crate) fn toggle_selection(&mut self, idx: usize) {
        if let Some(playlist) = self.playlist_mut() {
            if idx < playlist.tracks.len() {
                playlist.toggle_selection(idx);
            }
        }
    }

    pub(crate) fn update_metadata(&mut self, idx: usize, field: &str, value: String) {
        let Some(mut track) = self
            .playlist()
            .and_then(|playlist| playlist.tracks.get(idx).cloned())
        else {
            return;
        };

        if self.ctx.update_track_metadata(&mut track, field, &value) {
            if let Some(slot) = self
                .playlist_mut()
                .and_then(|playlist| playlist.tracks.get_mut(idx))
            {
                *slot = track;
            }
        }
    }

    pub(crate) fn play_track(&mut self, idx: usize) {
        let Some(track) = self
            .playlist()
            .and_then(|playlist| playlist.tracks.get(idx).cloned())
        else {
            return;
        };

        {
            let player = self.ctx.player_mut_ref();
            player.selected_track = Some(track.clone());
            player.select_track(Some(track));
            player.play();
        }

        self.ctx.app_settings.playing_playlist_idx = Some(self.playlist_idx);
        self.ctx.fetch_lyrics_for_current_track();
    }

    pub(crate) fn remove_track(&mut self, idx: usize) {
        if let Some(playlist) = self.playlist_mut() {
            if idx < playlist.tracks.len() {
                playlist.tracks.remove(idx);
            }
        }
    }

    fn playlist(&self) -> Option<&Playlist> {
        self.ctx.playlists.get(self.playlist_idx)
    }

    fn playlist_mut(&mut self) -> Option<&mut Playlist> {
        self.ctx.playlists.get_mut(self.playlist_idx)
    }
}
