use library::{
    Library, LibraryItem, LibraryItemContainer, LibraryPath, LibraryPathId, LibraryPathStatus,
    LibraryView, Picture, ViewType,
};
use player::Player;
use playlist::Playlist;
use scope::Scope;
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

mod app_impl;
mod components;
pub mod i18n;
mod library;
pub mod player;
mod playlist;
pub mod scope;
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
}

pub enum LibraryCommand {
    AddView(LibraryView),
    AddItem(LibraryItem),
    AddPathId(LibraryPathId),
}

#[derive(Serialize, Deserialize)]
pub struct App {
    pub library: Library,

    pub playlists: Vec<Playlist>,

    pub current_playlist_idx: Option<usize>,

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
    pub played_audio_buffer: Option<rb::Consumer<f32>>,

    #[serde(skip_serializing, skip_deserializing)]
    pub scope: Option<Scope>,

    #[serde(skip_serializing, skip_deserializing)]
    pub temp_buf: Option<Vec<f32>>,

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
        // Create a default playlist
        let mut default_playlist = playlist::Playlist::new();
        default_playlist.set_name("Default Playlist".to_string());

        Self {
            library: Library::new(),
            playlists: vec![default_playlist],
            current_playlist_idx: Some(0), // Set the first playlist as selected
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
            played_audio_buffer: None,
            scope: Some(Scope::new()),
            temp_buf: Some(vec![0.0f32; 4096]),
            quit: false,
            is_maximized: false,
            lib_config_selections: Default::default(),
            is_library_cfg_open: false,
            is_processing_ui_change: None,
            show_library_and_playlist: true,
            library_folders_expanded: false,
            show_about_dialog: false,
            default_window_height: 468.0,
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

impl App {
    pub fn load() -> Result<Self, TempError> {
        if let Ok(config) = confy::load::<App>("bird-player", None) {
            let mut app = config;

            // Initialize i18n
            i18n::init();

            // Set the language from the loaded config
            i18n::set_language(app.current_language);

            // Set up scope
            app.scope = Some(Scope::new());
            app.temp_buf = Some(vec![0.0f32; 4096]);

            app.is_maximized = false;
            app.is_library_cfg_open = false;
            app.show_about_dialog = false;
            app.is_processing_ui_change = None;
            app.show_library_and_playlist = true;
            Ok(app)
        } else {
            let mut app = App::default();

            // Initialize i18n
            i18n::init();

            // Set the default language
            i18n::set_language(app.current_language);

            // Set properties not covered by default
            app.is_maximized = false;
            app.is_library_cfg_open = false;
            app.show_about_dialog = false;

            // Save the initial state
            app.save_state();

            Ok(app)
        }
    }

    pub fn get_album_art_dir() -> PathBuf {
        confy::get_configuration_file_path("music_player", None)
            .map(|p| {
                p.parent()
                    .map_or_else(|| PathBuf::from("album_art"), |path| path.join("album_art"))
            })
            .unwrap_or_else(|_| PathBuf::from("album_art"))
    }

    pub fn save_state(&self) {
        // Update player state information before saving
        let store_result = confy::store("music_player", None, self);
        match store_result {
            Ok(_) => tracing::info!("Store was successful"),
            Err(err) => tracing::error!("Failed to store the app state: {}", err),
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
                                .set_track_number(tag.track())
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

        // Update the corresponding field in the tag
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
        match tag.write_to_path(&path, id3::Version::Id3v24) {
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
