use std::sync::mpsc::Receiver;

use crate::{
    library::LibraryItem,
    lyrics::{Lyrics, LyricsService},
    player::Player,
    state::lyrics_state::LyricsFetchState,
};

/// Service for managing lyrics fetching and display
pub struct LyricsManager {
    /// The lyrics service for fetching from API
    lyrics_service: Option<LyricsService>,

    /// Current lyrics for the playing track
    current_lyrics: Option<Lyrics>,

    /// Pending lyrics fetch receiver
    pending_lyrics_rx: Option<Receiver<Option<Lyrics>>>,
}

impl Default for LyricsManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LyricsManager {
    pub fn new() -> Self {
        Self {
            lyrics_service: Some(LyricsService::new()),
            current_lyrics: None,
            pending_lyrics_rx: None,
        }
    }

    pub fn current_lyrics(&self) -> Option<&Lyrics> {
        self.current_lyrics.as_ref()
    }

    pub fn current_lyrics_mut(&mut self) -> Option<&mut Lyrics> {
        self.current_lyrics.as_mut()
    }

    pub fn set_current_lyrics(&mut self, lyrics: Option<Lyrics>) {
        self.current_lyrics = lyrics;
    }

    pub fn pending_lyrics_rx(&self) -> Option<&Receiver<Option<Lyrics>>> {
        self.pending_lyrics_rx.as_ref()
    }

    pub fn take_pending_lyrics_rx(&mut self) -> Option<Receiver<Option<Lyrics>>> {
        self.pending_lyrics_rx.take()
    }

    /// Fetch lyrics for a given track (using track data directly)
    ///
    /// First tries to read from ID3 tag cache, then fetches from API if needed.
    /// Returns (lyrics_found, should_show_panel)
    pub fn fetch_lyrics_for_track_data(
        &mut self,
        track: Option<&LibraryItem>,
        fetch_state: &mut LyricsFetchState,
    ) -> (bool, bool) {
        // If there's already a pending request, don't start a new one
        if self.pending_lyrics_rx.is_some() {
            tracing::debug!("📡 Lyrics fetch already in progress, skipping");
            return (false, false);
        }

        // Clear current lyrics when starting a new fetch
        self.current_lyrics = None;
        *fetch_state = LyricsFetchState::Idle;

        let track = match track {
            Some(track) => track,
            None => {
                tracing::debug!("No track currently selected for lyrics fetch");
                return (false, false);
            }
        };

        let artist = track
            .artist()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let title = track.title().unwrap_or_else(|| "Unknown Title".to_string());

        // Try to read cached lyrics from ID3 tag
        if let Some(cached_lyrics) =
            LyricsService::read_lyrics_from_file(track.path(), &artist, &title)
        {
            tracing::info!(
                "✅ Found cached lyrics in ID3 tag for '{}'",
                track.path().display()
            );
            self.current_lyrics = Some(cached_lyrics);
            *fetch_state = LyricsFetchState::Loaded;

            let should_show = self
                .current_lyrics
                .as_ref()
                .is_some_and(|lyrics| !lyrics.lines.is_empty() || lyrics.plain_lyrics.is_some());

            return (true, should_show);
        }

        // If no cached lyrics, fetch from API (using 0 duration since we don't have it)
        self.fetch_from_api(track, 0, fetch_state);

        (false, false)
    }

    /// Fetch lyrics for a given track
    ///
    /// First tries to read from ID3 tag cache, then fetches from API if needed.
    /// Returns (lyrics_found, should_show_panel)
    pub fn fetch_lyrics_for_track(
        &mut self,
        player: &Player,
        fetch_state: &mut LyricsFetchState,
    ) -> (bool, bool) {
        // If there's already a pending request, don't start a new one
        if self.pending_lyrics_rx.is_some() {
            tracing::debug!("📡 Lyrics fetch already in progress, skipping");
            return (false, false);
        }

        // Clear current lyrics when starting a new fetch
        self.current_lyrics = None;
        *fetch_state = LyricsFetchState::Idle;

        let track = match player.selected_track.as_ref() {
            Some(track) => track,
            None => {
                tracing::debug!("No track currently selected for lyrics fetch");
                return (false, false);
            }
        };

        let artist = track
            .artist()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let title = track.title().unwrap_or_else(|| "Unknown Title".to_string());

        // Try to read cached lyrics from ID3 tag
        if let Some(cached_lyrics) =
            LyricsService::read_lyrics_from_file(track.path(), &artist, &title)
        {
            tracing::info!(
                "✅ Found cached lyrics in ID3 tag for '{}'",
                track.path().display()
            );
            self.current_lyrics = Some(cached_lyrics);
            *fetch_state = LyricsFetchState::Loaded;

            let should_show = self
                .current_lyrics
                .as_ref()
                .is_some_and(|lyrics| !lyrics.lines.is_empty() || lyrics.plain_lyrics.is_some());

            return (true, should_show);
        }

        // If no cached lyrics, fetch from API
        self.fetch_from_api(track, player.duration, fetch_state);

        (false, false)
    }

    /// Fetch lyrics from API
    fn fetch_from_api(
        &mut self,
        track: &LibraryItem,
        duration: u64,
        fetch_state: &mut LyricsFetchState,
    ) {
        let Some(lyrics_service) = &self.lyrics_service else {
            tracing::warn!("⚠️  Lyrics service not available");
            *fetch_state = LyricsFetchState::Failed("Lyrics service not available".to_string());
            return;
        };

        let artist = track
            .artist()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let title = track.title().unwrap_or_else(|| "Unknown Title".to_string());
        let album = track.album();
        let duration_opt = if duration > 0 { Some(duration) } else { None };

        tracing::info!(
            "🎵 No cached lyrics found, fetching from API for track: '{}' by '{}'",
            title,
            artist
        );
        tracing::debug!("📁 Track file: '{}'", track.path().display());

        let response_rx = lyrics_service.fetch_lyrics(artist, title, album, duration_opt);
        self.pending_lyrics_rx = Some(response_rx);
        *fetch_state = LyricsFetchState::Loading;

        tracing::debug!("📡 Lyrics fetch initiated, waiting for response...");
    }

    /// Check for pending lyrics response
    ///
    /// Returns (lyrics_received, should_show_panel)
    pub fn check_pending_lyrics(&mut self, fetch_state: &mut LyricsFetchState) -> (bool, bool) {
        let Some(lyrics_rx) = &self.pending_lyrics_rx else {
            return (false, false);
        };

        match lyrics_rx.try_recv() {
            Ok(Some(lyrics)) => {
                tracing::info!("✅ Successfully received lyrics from API");
                self.current_lyrics = Some(lyrics);
                self.pending_lyrics_rx = None;
                *fetch_state = LyricsFetchState::Loaded;

                let should_show = self.current_lyrics.as_ref().is_some_and(|lyrics| {
                    !lyrics.lines.is_empty() || lyrics.plain_lyrics.is_some()
                });

                (true, should_show)
            }
            Ok(None) => {
                tracing::warn!("⚠️  No lyrics found from API");
                self.pending_lyrics_rx = None;
                *fetch_state = LyricsFetchState::Failed("No lyrics found".to_string());
                (false, false)
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                // Still waiting
                (false, false)
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                tracing::error!("❌ Lyrics fetch channel disconnected");
                self.pending_lyrics_rx = None;
                *fetch_state = LyricsFetchState::Failed("Fetch failed".to_string());
                (false, false)
            }
        }
    }
}
