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

use itertools::Itertools;

use id3::{Tag, TagLike};
use rayon::prelude::*;

use rand::Rng;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

mod app_impl;
mod components;
mod library;
pub mod player;
mod playlist;
pub mod scope;

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

    #[serde(skip_serializing, skip_deserializing)]
    pub player: Option<Player>,

    #[serde(skip_serializing, skip_deserializing)]
    pub playlist_idx_to_remove: Option<usize>,

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

    #[serde(skip_serializing, skip_deserializing)]
    pub quit: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub lib_config_selections: std::collections::HashSet<LibraryPathId>,

    #[serde(skip_serializing, skip_deserializing)]
    pub is_library_cfg_open: bool,

    #[serde(skip_serializing, skip_deserializing)]
    pub is_processing_ui_change: Option<Arc<AtomicBool>>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            library: Library::new(),
            playlists: vec![],
            current_playlist_idx: None,
            player: None,
            playlist_idx_to_remove: None,
            library_cmd_tx: None,
            library_cmd_rx: None,
            played_audio_buffer: None,
            scope: Some(Scope::new()),
            temp_buf: Some(vec![0.0f32; 4096]),
            quit: false,
            lib_config_selections: Default::default(),
            is_library_cfg_open: false,
            is_processing_ui_change: None,
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
        let config_dir = confy::get_configuration_file_path("music_player", None)
            .map_err(|_| TempError::MissingAppState)?
            .parent()
            .ok_or(TempError::MissingAppState)?
            .to_path_buf();

        // Create album_art directory in the config directory
        let album_art_dir = config_dir.join("album_art");
        fs::create_dir_all(&album_art_dir).map_err(|_| TempError::MissingAppState)?;

        println!(
            "Load configuration file {:#?}",
            config_dir.join("music_player.yml")
        );
        confy::load("music_player", None).map_err(|_| TempError::MissingAppState)
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
        let store_result = confy::store("music_player", None, self);
        match store_result {
            Ok(_) => tracing::info!("Store was successful"),
            Err(err) => tracing::error!("Failed to store the app state: {}", err),
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
                                .set_track_number(tag.track());

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

            // Populate the library
            for item in &items {
                lib_cmd_tx
                    .send(LibraryCommand::AddItem((*item).clone()))
                    .expect("failed to send library item")
            }

            // Build the views
            let mut library_view = LibraryView {
                view_type: ViewType::Album,
                containers: Vec::new(),
            };

            // In order for group by to work from itertools, items must be consecutive, so sort them first.
            let mut library_items_clone = items.clone();
            library_items_clone.sort_by_key(|item| item.album());

            let grouped_library_by_album = &library_items_clone.into_iter().group_by(|item| {
                item.album()
                    .unwrap_or("unknown album".to_string())
                    .to_string()
            });

            for (album_name, album_library_items) in grouped_library_by_album {
                let lib_item_container = LibraryItemContainer {
                    name: album_name.clone(),
                    items: album_library_items.collect::<Vec<LibraryItem>>(),
                };

                library_view.containers.push(lib_item_container.clone());
            }

            lib_cmd_tx
                .send(LibraryCommand::AddView(library_view))
                .expect("Failed to send library view");

            lib_cmd_tx
                .send(LibraryCommand::AddPathId(path_id))
                .expect("Failed to send library view");
            //lib_path.set_imported();
        });
    }
}
