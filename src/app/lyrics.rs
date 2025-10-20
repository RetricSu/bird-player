use serde::{Deserialize, Serialize};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LyricsLine {
    pub text: String,
    pub start_time_ms: Option<u64>,
    pub end_time_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lyrics {
    pub id: i64,
    pub name: String,
    #[serde(rename = "trackName")]
    pub track_name: String,
    #[serde(rename = "artistName")]
    pub artist_name: String,
    #[serde(rename = "albumName")]
    pub album_name: Option<String>,
    pub duration: Option<f64>,
    pub instrumental: bool,
    #[serde(rename = "plainLyrics")]
    pub plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    pub synced_lyrics: Option<String>,
    #[serde(skip)]
    pub lines: Vec<LyricsLine>,
}

impl Lyrics {
    pub fn parse_synced_lyrics(synced_lyrics: &str) -> Vec<LyricsLine> {
        let mut lines = Vec::new();

        for line in synced_lyrics.lines() {
            if let Some((timestamp_str, text)) = line.split_once(']') {
                if let Some(timestamp_str) = timestamp_str.strip_prefix('[') {
                    if let Ok(timestamp_ms) = Self::parse_timestamp(timestamp_str) {
                        lines.push(LyricsLine {
                            text: text.trim().to_string(),
                            start_time_ms: Some(timestamp_ms),
                            end_time_ms: None,
                        });
                    }
                }
            }
        }

        // Set end times based on next line's start time
        for i in 0..lines.len().saturating_sub(1) {
            if let Some(next_start) = lines[i + 1].start_time_ms {
                lines[i].end_time_ms = Some(next_start);
            }
        }

        lines
    }

    fn parse_timestamp(timestamp: &str) -> Result<u64, ()> {
        let parts: Vec<&str> = timestamp.split(':').collect();
        if parts.len() != 2 {
            return Err(());
        }

        let minutes: u64 = parts[0].parse().map_err(|_| ())?;
        let seconds: f64 = parts[1].parse().map_err(|_| ())?;

        Ok((minutes * 60 * 1000) as u64 + (seconds * 1000.0) as u64)
    }
}

pub struct LyricsService {
    tx: Sender<LyricsRequest>,
    _handle: thread::JoinHandle<()>,
}

impl LyricsService {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let handle = thread::spawn(move || {
            Self::run_service(rx);
        });

        Self {
            tx,
            _handle: handle,
        }
    }

    pub fn fetch_lyrics(
        &self,
        artist: String,
        title: String,
        album: Option<String>,
        duration: Option<u64>,
    ) -> Receiver<Option<Lyrics>> {
        let (response_tx, response_rx) = mpsc::channel();

        let request = LyricsRequest {
            artist,
            title,
            album,
            duration,
            response_tx,
        };

        if self.tx.send(request).is_err() {
            tracing::error!("Failed to send lyrics request");
        }

        response_rx
    }

    fn run_service(rx: Receiver<LyricsRequest>) {
        while let Ok(request) = rx.recv() {
            let lyrics = Self::fetch_from_api(
                &request.artist,
                &request.title,
                &request.album,
                request.duration,
            );
            let _ = request.response_tx.send(lyrics);
        }
    }

    fn fetch_from_api(
        artist: &str,
        title: &str,
        _album: &Option<String>,
        _duration: Option<u64>,
    ) -> Option<Lyrics> {
        // Only use search API with artist and track name parameters
        let search_url = format!(
            "https://lrclib.net/api/search?artist_name={}&track_name={}",
            urlencoding::encode(artist),
            urlencoding::encode(title)
        );

        tracing::debug!("📡 Trying search API: {}", search_url);
        if let Some(lyrics) = Self::try_search_api(&search_url, artist, title) {
            return Some(lyrics);
        }

        None
    }

    fn try_search_api(url: &str, artist: &str, title: &str) -> Option<Lyrics> {
        match ureq::get(url).call() {
            Ok(response) => {
                let status = response.status();
                tracing::debug!("📡 Search API Response: Status {}", status);

                match response.into_string() {
                    Ok(body) => {
                        tracing::debug!(
                            "📡 Search response body length: {} characters",
                            body.len()
                        );

                        match serde_json::from_str::<Vec<Lyrics>>(&body) {
                            Ok(lyrics_array) => {
                                // Find the best match by comparing artist and title (case-insensitive)
                                let artist_lower = artist.to_lowercase();
                                let title_lower = title.to_lowercase();

                                for lyrics in lyrics_array {
                                    let api_artist_lower = lyrics.artist_name.to_lowercase();
                                    let api_title_lower = lyrics.track_name.to_lowercase();

                                    tracing::debug!("🔍 Checking match: API '{}' vs search '{}' | API '{}' vs search '{}'", 
                                        api_artist_lower, artist_lower, api_title_lower, title_lower);

                                    if api_artist_lower == artist_lower
                                        && api_title_lower == title_lower
                                    {
                                        tracing::info!("✅ Found exact match in search results for '{}' by '{}'", title, artist);
                                        tracing::debug!("📊 Match details - ID: {}, Album: {:?}, Duration: {:?}, Instrumental: {}", 
                                            lyrics.id, lyrics.album_name, lyrics.duration, lyrics.instrumental);

                                        let mut lyrics = lyrics;
                                        // Parse synced lyrics if available
                                        if let Some(synced) = &lyrics.synced_lyrics {
                                            lyrics.lines = Lyrics::parse_synced_lyrics(synced);
                                            tracing::debug!(
                                                "🎵 Parsed {} synced lyric lines",
                                                lyrics.lines.len()
                                            );
                                        } else if let Some(plain) = &lyrics.plain_lyrics {
                                            tracing::debug!(
                                                "📝 Using plain lyrics ({} characters)",
                                                plain.len()
                                            );
                                            if plain.trim().is_empty() {
                                                tracing::warn!("⚠️  Plain lyrics are empty");
                                            } else {
                                                tracing::debug!(
                                                    "📄 Plain lyrics preview: {}",
                                                    &plain[..plain.len().min(100)]
                                                );
                                            }
                                        } else {
                                            tracing::warn!("⚠️  Lyrics record found but no content available (instrumental: {})", lyrics.instrumental);
                                            // Don't return lyrics if there's no actual content
                                            continue;
                                        }

                                        return Some(lyrics);
                                    }
                                }

                                tracing::warn!(
                                    "❌ No exact matches found in search results for '{}' by '{}'",
                                    title,
                                    artist
                                );
                                None
                            }
                            Err(e) => {
                                tracing::error!(
                                    "❌ Failed to parse search results JSON for '{}' by '{}': {}",
                                    title,
                                    artist,
                                    e
                                );
                                tracing::debug!(
                                    "📄 Raw search response body (first 500 chars): {}",
                                    &body[..body.len().min(500)]
                                );
                                None
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ Failed to read search response body for '{}' by '{}': {}",
                            title,
                            artist,
                            e
                        );
                        None
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "❌ Search API request failed for '{}' by '{}': {}",
                    title,
                    artist,
                    e
                );
                None
            }
        }
    }
}

struct LyricsRequest {
    artist: String,
    title: String,
    album: Option<String>,
    duration: Option<u64>,
    response_tx: Sender<Option<Lyrics>>,
}
