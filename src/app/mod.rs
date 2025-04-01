use library::{
    Library, LibraryItem, LibraryItemContainer, LibraryPath, LibraryPathId, LibraryPathStatus,
    LibraryView, Picture, ViewType,
};
use player::Player;
use playlist::Playlist;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use id3::{Tag, TagLike};
use rayon::prelude::*;

use rand::Rng;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

// Window size constants
pub const DEFAULT_WINDOW_WIDTH: f32 = 750.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 468.0;

mod app_impl;
mod components;
pub mod i18n;
mod library;
pub mod player;
mod playlist;
mod style;

// Re-export the i18n functions for convenience
pub use i18n::{get_language, set_language, t, tf, Language};

pub enum AudioCommand {
    Stop,
    Play,
    Pause,
    Seek(u64),
    LoadFile(std::path::PathBuf),
    Select(usize),
    SetVolume(f32),
}

pub enum UiCommand {
    AudioFinished,
    TotalTrackDuration(u64),
    CurrentTimestamp(u64),
    PlaybackStateChanged(bool), // true = playing, false = paused
}

pub enum LibraryCommand {
    AddView(LibraryView),
    AddItem(LibraryItem),
    AddPathId(LibraryPathId),
}

// Struct for storing basic settings in confy
#[derive(Serialize, Deserialize)]
pub struct AppSettings {
    // Language setting
    pub current_language: i18n::Language,

    // Player state persistence
    pub last_track_path: Option<PathBuf>,
    pub last_position: Option<u64>,
    pub last_playback_mode: Option<player::PlaybackMode>,
    pub last_volume: Option<f32>,
    pub was_playing: Option<bool>,

    // UI state
    pub library_folders_expanded: bool,
    pub default_window_height: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            current_language: i18n::Language::English,
            last_track_path: None,
            last_position: None,
            last_playback_mode: None,
            last_volume: None,
            was_playing: None,
            library_folders_expanded: false,
            default_window_height: DEFAULT_WINDOW_HEIGHT as f64,
        }
    }
}

#[derive(Debug, Clone)]
pub enum TempError {
    MissingAppState,
}

impl std::fmt::Display for TempError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Couldn't load app state")
    }
}

#[derive(Serialize, Deserialize)]
pub struct App {
    pub library: Library,

    pub playlists: Vec<Playlist>,

    pub current_playlist_idx: Option<usize>,

    // New field to track which playlist is currently playing
    pub playing_playlist_idx: Option<usize>,

    // Language setting
    pub current_language: i18n::Language,

    // New fields for player state persistence
    pub last_track_path: Option<PathBuf>,
    pub last_position: Option<u64>,
    pub last_playback_mode: Option<player::PlaybackMode>,
    pub last_volume: Option<f32>,
    pub was_playing: Option<bool>,

    #[serde(skip_serializing, skip_deserializing)]
    pub player: Option<Player>,

    #[serde(skip_serializing, skip_deserializing)]
    pub playlist_idx_to_remove: Option<usize>,

    #[serde(skip_serializing, skip_deserializing)]
    pub playlist_being_renamed: Option<usize>,

    #[serde(skip_serializing, skip_deserializing)]
    pub library_cmd_tx: Option<Sender<LibraryCommand>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub library_cmd_rx: Option<Receiver<LibraryCommand>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub database: Option<Arc<crate::db::Database>>,

    pub quit: bool,

    pub is_maximized: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub lib_config_selections: std::collections::HashSet<LibraryPathId>,

    #[serde(skip_serializing, skip_deserializing)]
    pub is_library_cfg_open: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub is_processing_ui_change: Option<Arc<AtomicBool>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub show_library_and_playlist: bool,

    pub library_folders_expanded: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub show_about_dialog: bool,

    pub default_window_height: f64,
}

impl Default for App {
    fn default() -> Self {
        Self {
            library: Library::new(),
            playlists: vec![],          // Start with empty playlists
            current_playlist_idx: None, // No playlist selected initially
            playing_playlist_idx: None,
            current_language: i18n::Language::English, // Default language
            // Initialize the new fields
            last_track_path: None,
            last_position: None,
            last_playback_mode: None,
            last_volume: None,
            was_playing: None,
            player: None,
            playlist_idx_to_remove: None,
            playlist_being_renamed: None,
            library_cmd_tx: None,
            library_cmd_rx: None,
            database: None,
            quit: false,
            is_maximized: false,
            lib_config_selections: Default::default(),
            is_library_cfg_open: false,
            is_processing_ui_change: None,
            show_library_and_playlist: true,
            library_folders_expanded: false,
            show_about_dialog: false,
            default_window_height: DEFAULT_WINDOW_HEIGHT as f64,
        }
    }
}

impl App {
    pub fn load() -> Result<Self, TempError> {
        // Still use confy for app settings
        let config_result = confy::load::<AppSettings>("bird-player", None);

        // Create a new default app - this doesn't have a database set yet
        let mut app = App::default();

        // Initialize i18n
        i18n::init();

        if let Ok(settings) = config_result {
            // Apply settings from confy
            app.current_language = settings.current_language;
            app.last_track_path = settings.last_track_path;
            app.last_position = settings.last_position;
            app.last_playback_mode = settings.last_playback_mode;
            app.last_volume = settings.last_volume;
            app.was_playing = settings.was_playing;
            app.library_folders_expanded = settings.library_folders_expanded;
            app.default_window_height = settings.default_window_height;
        }

        // Set the language from the loaded config
        i18n::set_language(app.current_language);

        // Initialize database if it's not already set
        if app.database.is_none() {
            match crate::db::Database::new() {
                Ok(db) => {
                    app.database = Some(Arc::new(db));
                    tracing::info!("Database created during App::load()");
                }
                Err(e) => {
                    tracing::error!("Failed to create database during App::load(): {}", e);
                }
            }
        }

        // Try to load library and playlists if we have a database
        if let Some(ref db) = app.database {
            // Try to load library from database
            match Library::load_from_db(&db.connection()) {
                Ok(library) => {
                    app.library = library;
                    tracing::info!("Successfully loaded library from database");
                }
                Err(e) => {
                    tracing::error!("Failed to load library from database: {}", e);
                    // Keep the default empty library
                }
            }

            // Try to load playlists from database
            match playlist::Playlist::load_all_from_db(&db.connection()) {
                Ok(playlists) => {
                    if !playlists.is_empty() {
                        app.playlists = playlists;

                        // If there was a last played track, try to find its playlist
                        if let Some(last_track_path) = &app.last_track_path {
                            for (idx, playlist) in app.playlists.iter().enumerate() {
                                if playlist
                                    .tracks
                                    .iter()
                                    .any(|track| track.path() == *last_track_path)
                                {
                                    app.current_playlist_idx = Some(idx);
                                    app.playing_playlist_idx = Some(idx);
                                    tracing::info!(
                                        "Found last played track in playlist '{}', selecting it",
                                        playlist.get_name().unwrap_or_default()
                                    );
                                    break;
                                }
                            }
                        }

                        // If no playlist was selected (no last track or track not found), select first playlist
                        if app.current_playlist_idx.is_none() {
                            app.current_playlist_idx = Some(0);
                            tracing::info!("No last played track found, selecting first playlist");
                        }
                    } else {
                        // Only create a default playlist if no playlists exist in the database
                        let mut default_playlist = playlist::Playlist::new();
                        default_playlist.set_name("Default Playlist".to_string());
                        app.playlists = vec![default_playlist];
                        app.current_playlist_idx = Some(0);
                        tracing::info!("No playlists found in database, created default playlist");
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to load playlists from database: {}", e);
                    // Keep the default playlist
                }
            }
        } else {
            tracing::warn!("No database connection available when loading app state");
        }

        app.is_maximized = false;
        app.is_library_cfg_open = false;
        app.show_about_dialog = false;
        app.is_processing_ui_change = None;
        app.show_library_and_playlist = true;

        Ok(app)
    }

    pub fn get_album_art_dir() -> PathBuf {
        confy::get_configuration_file_path("bird-player", None)
            .map(|p| {
                p.parent()
                    .map_or_else(|| PathBuf::from("album_art"), |path| path.join("album_art"))
            })
            .unwrap_or_else(|_| PathBuf::from("album_art"))
    }

    pub fn save_state(&self) {
        // Split app state - settings go to confy, library and playlists go to SQLite
        let settings = AppSettings {
            current_language: self.current_language,
            last_track_path: self.last_track_path.clone(),
            last_position: self.last_position,
            last_playback_mode: self.last_playback_mode,
            last_volume: self.last_volume,
            was_playing: self.was_playing,
            library_folders_expanded: self.library_folders_expanded,
            default_window_height: self.default_window_height,
        };

        // Save app settings to confy
        let store_result = confy::store("bird-player", None, &settings);
        match store_result {
            Ok(_) => tracing::info!("Settings stored successfully"),
            Err(err) => tracing::error!("Failed to store app settings: {}", err),
        }

        // Only save to database if we're quitting or if there are pending changes
        if self.quit {
            // Save library and playlists to SQLite if database is available
            if let Some(ref db) = &self.database {
                // Save library
                if let Err(e) = self.library.save_to_db(&db.connection()) {
                    tracing::error!("Failed to save library to database: {}", e);
                }

                // Save playlists
                for playlist in &self.playlists {
                    if let Err(e) = playlist.save_to_db(&db.connection()) {
                        tracing::error!("Failed to save playlist to database: {}", e);
                    }
                }
            }
        }
    }

    /// Capture the current player state for persistence
    pub fn update_player_persistence(&mut self) {
        if let Some(player) = &self.player {
            // Save the current track path if there's a selected track
            self.last_track_path = player.selected_track.as_ref().map(|track| track.path());

            // Save the current playing position
            self.last_position = Some(player.seek_to_timestamp);

            // Save the current playback mode
            self.last_playback_mode = Some(player.playback_mode);

            // Save the current volume
            self.last_volume = Some(player.volume);

            // Save whether the player was playing or paused
            self.was_playing = Some(matches!(player.track_state, player::TrackState::Playing));
        }
    }

    pub fn quit(&mut self) {
        self.quit = true;
    }

    // Spawns a background thread and imports files
    // from each unimported library path
    fn import_library_paths(&self, lib_path: &LibraryPath) {
        if lib_path.status() == LibraryPathStatus::Imported {
            tracing::info!("already imported library path...");
            return;
        }

        tracing::info!("adding library path...");

        let lib_cmd_tx = self.library_cmd_tx.as_ref().unwrap().clone();
        let path = lib_path.path().clone();
        let path_id = lib_path.id();
        // Store path display string for later use
        let path_display = path.display().to_string();

        // Get the album art directory path
        let album_art_dir = App::get_album_art_dir();
        // Ensure the album art directory exists
        if let Err(err) = fs::create_dir_all(&album_art_dir) {
            tracing::error!("Failed to create album art directory: {}", err);
            return;
        }

        std::thread::spawn(move || {
            let files = walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .skip(1)
                .filter(|entry| {
                    entry.file_type().is_file()
                        && entry.path().extension().unwrap_or(std::ffi::OsStr::new("")) == "mp3"
                })
                .collect::<Vec<_>>();

            let items = files
                .par_iter()
                .map(|entry| {
                    let tag = Tag::read_from_path(entry.path());

                    let library_item = match tag {
                        Ok(tag) => {
                            let mut item = LibraryItem::new(entry.path().to_path_buf(), path_id);

                            // Get filename without extension as fallback title
                            let filename_title = entry
                                .path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown Title")
                                .to_string();

                            // Use filename as title if ID3 tag is missing or contains invalid UTF-8
                            let title = tag
                                .title()
                                .and_then(|t| {
                                    if t.chars().any(|c| !c.is_ascii() && !c.is_alphabetic()) {
                                        None
                                    } else {
                                        Some(t)
                                    }
                                })
                                .unwrap_or(&filename_title);

                            item = item
                                .set_title(Some(title))
                                .set_artist(tag.artist())
                                .set_album(tag.album())
                                .set_year(tag.year())
                                .set_genre(tag.genre())
                                .set_track_number(tag.get("TRCK").and_then(|frame| {
                                    frame.content().text().map(|t| {
                                        t.split('/')
                                            .next()
                                            .unwrap_or("0")
                                            .parse::<u32>()
                                            .unwrap_or(0)
                                    })
                                }))
                                .set_lyrics(tag.lyrics().next().map(|l| l.text.as_str()));

                            // Extract pictures from ID3 tag
                            for pic in tag.pictures() {
                                // Create a unique filename for the picture
                                let file_name = album_art_dir.join(format!(
                                    "{}_{}_{}.{}",
                                    entry
                                        .path()
                                        .file_stem()
                                        .unwrap_or_default()
                                        .to_string_lossy(),
                                    u8::from(pic.picture_type),
                                    rand::thread_rng().gen::<u64>(), // Add random number to ensure uniqueness
                                    match pic.mime_type.as_str() {
                                        "image/jpeg" => "jpg",
                                        "image/png" => "png",
                                        _ => "jpg", // Default to jpg for unknown types
                                    }
                                ));

                                // Save the picture data to a file
                                if let Ok(mut file) = fs::File::create(&file_name) {
                                    if file.write_all(&pic.data).is_ok() {
                                        item.add_picture(Picture::new(
                                            pic.mime_type.to_string(),
                                            u8::from(pic.picture_type),
                                            pic.description.to_string(),
                                            file_name,
                                        ));
                                    }
                                }
                            }

                            item
                        }
                        Err(_err) => {
                            tracing::warn!("Couldn't parse to id3: {:?}", &entry.path());
                            // Get filename without extension as title for failed ID3 reads
                            let filename_title = entry
                                .path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Unknown Title")
                                .to_string();

                            LibraryItem::new(entry.path().to_path_buf(), path_id)
                                .set_title(Some(&filename_title))
                        }
                    };

                    library_item
                })
                .collect::<Vec<LibraryItem>>();

            tracing::info!("Done parsing library items");

            // Populate the library with parsed items
            for item in &items {
                lib_cmd_tx
                    .send(LibraryCommand::AddItem((*item).clone()))
                    .expect("failed to send library item")
            }

            // The new implementation doesn't need album grouping anymore as we're organizing by folders
            // We'll still create a view for backward compatibility, but it won't be used
            // in our updated library_component
            let mut library_view = LibraryView {
                view_type: ViewType::Album,
                containers: Vec::new(),
            };

            // Create a single container for all items of this path
            // This maintains compatibility with the existing code
            let lib_item_container = LibraryItemContainer {
                name: format!("Folder: {}", path_display),
                items: items.clone(),
            };

            library_view.containers.push(lib_item_container);

            lib_cmd_tx
                .send(LibraryCommand::AddView(library_view))
                .expect("Failed to send library view");

            lib_cmd_tx
                .send(LibraryCommand::AddPathId(path_id))
                .expect("Failed to send library view");
        });
    }

    pub fn update_track_metadata(
        &mut self,
        track: &mut LibraryItem,
        field: &str,
        value: &str,
    ) -> bool {
        // Get the file path from the LibraryItem
        let path = track.path();

        // Try to read the existing tag
        let mut tag = match id3::Tag::read_from_path(&path) {
            Ok(tag) => tag,
            Err(err) => {
                // If there's no tag, create a new one
                if let id3::ErrorKind::NoTag = err.kind {
                    tracing::info!("Creating new ID3 tag for file: {:?}", path);
                    id3::Tag::new()
                } else {
                    tracing::error!("Failed to read ID3 tag for file {:?}: {}", path, err);
                    return false;
                }
            }
        };

        // Update the corresponding field in the tag and track
        match field {
            "title" => {
                tag.set_title(value);
                track.set_title(Some(value));
            }
            "artist" => {
                tag.set_artist(value);
                track.set_artist(Some(value));
            }
            "album" => {
                tag.set_album(value);
                track.set_album(Some(value));
            }
            "genre" => {
                tag.set_genre(value);
                track.set_genre(Some(value));
            }
            _ => return false, // Unsupported field
        }

        // Write the updated tag back to the file
        let file_update_success = match tag.write_to_path(&path, id3::Version::Id3v24) {
            Ok(_) => {
                tracing::info!(
                    "Successfully updated {} to '{}' for file: {:?}",
                    field,
                    value,
                    path
                );
                true
            }
            Err(e) => {
                tracing::error!("Failed to write {} tag for file {:?}: {}", field, path, e);
                false
            }
        };

        // Update the database if file update was successful
        if file_update_success {
            if let Some(ref db) = self.database {
                let conn = db.connection();
                let result = {
                    let mut conn_guard = conn.lock().unwrap();
                    let tx = conn_guard.transaction().ok();

                    if let Some(tx) = tx {
                        let update_result = tx.execute(
                            &format!("UPDATE library_items SET {} = ?1 WHERE key = ?2", field),
                            rusqlite::params![value, track.key().to_string()],
                        );

                        match update_result.and_then(|_| tx.commit()) {
                            Ok(_) => {
                                tracing::info!(
                                    "Successfully updated {} in database for track {}",
                                    field,
                                    track.key()
                                );
                                true
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to update {} in database for track {}: {}",
                                    field,
                                    track.key(),
                                    e
                                );
                                false
                            }
                        }
                    } else {
                        tracing::error!("Failed to start database transaction for metadata update");
                        false
                    }
                };

                // If database update was successful, update all instances of this track
                if result {
                    // Update all instances of this track in all playlists
                    for playlist in &mut self.playlists {
                        for playlist_track in playlist.tracks.iter_mut() {
                            if playlist_track.key() == track.key() {
                                let updated_track = match field {
                                    "title" => playlist_track.set_title(Some(value)),
                                    "artist" => playlist_track.set_artist(Some(value)),
                                    "album" => playlist_track.set_album(Some(value)),
                                    "genre" => playlist_track.set_genre(Some(value)),
                                    _ => playlist_track.clone(),
                                };
                                *playlist_track = updated_track;
                            }
                        }
                    }

                    // Reload the library from the database to get updated metadata
                    if let Ok(updated_library) = library::Library::load_from_db(&db.connection()) {
                        self.library = updated_library;
                    }

                    // Save the updated state to ensure persistence
                    self.save_state();
                }

                result
            } else {
                tracing::warn!("No database connection available for metadata update");
                file_update_success
            }
        } else {
            false
        }
    }

    // Add these new methods for language handling
    pub fn set_language(&mut self, lang: i18n::Language) {
        self.current_language = lang;
        i18n::set_language(lang);
        // Save state to persist language preference
        self.save_state();
    }

    pub fn get_language(&self) -> i18n::Language {
        self.current_language
    }
}

// Include the version info module generated at build time
pub mod version_info {
    include!(concat!(env!("OUT_DIR"), "/version_info.rs"));

    // Return formatted version string with commit hash
    pub fn formatted_version() -> String {
        format!("Version {} ({})", VERSION, GIT_HASH)
    }
}
