use id3::{Tag, TagLike};
use rand::Rng;
use rayon::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use walkdir::WalkDir;

use crate::{
    library::{
        LibraryItem, LibraryItemContainer, LibraryPath, LibraryPathId, LibraryPathStatus,
        LibraryView, Picture, ViewType,
    },
    LibraryCommand,
};

/// Service for importing library paths in the background
pub struct LibraryImportService;

impl LibraryImportService {
    /// Import files from a library path
    ///
    /// This spawns a background thread that:
    /// 1. Walks the directory tree
    /// 2. Finds all MP3 files
    /// 3. Reads ID3 tags
    /// 4. Extracts album art
    /// 5. Sends items back via the command channel
    pub fn import_library_path(
        lib_path: &LibraryPath,
        lib_cmd_tx: Sender<LibraryCommand>,
        album_art_dir: PathBuf,
    ) {
        if lib_path.status() == LibraryPathStatus::Imported {
            tracing::info!("Library path already imported, skipping");
            return;
        }

        tracing::info!("Starting library path import: {:?}", lib_path.path());

        let path = lib_path.path().clone();
        let path_id = lib_path.id();
        let path_display = path.display().to_string();

        // Ensure album art directory exists
        if let Err(err) = fs::create_dir_all(&album_art_dir) {
            tracing::error!("Failed to create album art directory: {}", err);
            return;
        }

        std::thread::spawn(move || {
            let result = Self::import_files(&path, path_id, &album_art_dir, &lib_cmd_tx);

            match result {
                Ok(items) => {
                    Self::send_library_view(&items, path_id, &path_display, lib_cmd_tx);
                }
                Err(e) => {
                    tracing::error!("Failed to import library path: {}", e);
                }
            }
        });
    }

    /// Walk directory and import all MP3 files
    fn import_files(
        path: &Path,
        path_id: LibraryPathId,
        album_art_dir: &Path,
        lib_cmd_tx: &Sender<LibraryCommand>,
    ) -> Result<Vec<LibraryItem>, Box<dyn std::error::Error>> {
        // Find all MP3 files
        let files = WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .skip(1)
            .filter(|entry| {
                entry.file_type().is_file()
                    && entry.path().extension().unwrap_or(std::ffi::OsStr::new("")) == "mp3"
            })
            .collect::<Vec<_>>();

        tracing::info!("Found {} MP3 files to import", files.len());

        // Parse files in parallel
        let items: Vec<LibraryItem> = files
            .par_iter()
            .map(|entry| Self::parse_audio_file(entry.path(), path_id, album_art_dir))
            .collect();

        // Send items as they're processed
        for item in &items {
            lib_cmd_tx
                .send(LibraryCommand::AddItem(item.clone()))
                .map_err(|e| format!("Failed to send library item: {}", e))?;
        }

        tracing::info!("Finished parsing {} library items", items.len());

        Ok(items)
    }

    /// Parse a single audio file and extract metadata
    fn parse_audio_file(
        file_path: &std::path::Path,
        path_id: LibraryPathId,
        album_art_dir: &Path,
    ) -> LibraryItem {
        let tag_result = Tag::read_from_path(file_path);

        match tag_result {
            Ok(tag) => {
                tracing::debug!("Successfully read ID3 tag from: {}", file_path.display());
                Self::create_item_from_tag(file_path, path_id, &tag, album_art_dir)
            }
            Err(err) => {
                tracing::warn!("Failed to read ID3 tag from {:?}: {}", file_path, err);
                Self::create_fallback_item(file_path, path_id)
            }
        }
    }

    /// Create LibraryItem from ID3 tag
    fn create_item_from_tag(
        file_path: &std::path::Path,
        path_id: LibraryPathId,
        tag: &Tag,
        album_art_dir: &Path,
    ) -> LibraryItem {
        // Get filename as fallback title
        let filename_title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Title")
            .to_string();

        let title = tag.title().unwrap_or(&filename_title);

        let mut item = LibraryItem::new(file_path.to_path_buf(), path_id)
            .set_title(Some(title))
            .set_artist(tag.artist())
            .set_album(tag.album())
            .set_year(tag.year())
            .set_genre(tag.genre())
            .set_track_number(Self::extract_track_number(tag))
            .set_lyrics(tag.lyrics().next().map(|l| l.text.as_str()));

        // Extract and save album art
        Self::extract_album_art(&mut item, tag, file_path, album_art_dir);

        item
    }

    /// Create fallback LibraryItem when ID3 tag is missing
    fn create_fallback_item(file_path: &std::path::Path, path_id: LibraryPathId) -> LibraryItem {
        let filename_title = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown Title")
            .to_string();

        LibraryItem::new(file_path.to_path_buf(), path_id).set_title(Some(&filename_title))
    }

    /// Extract track number from ID3 tag
    fn extract_track_number(tag: &Tag) -> Option<u32> {
        tag.get("TRCK").and_then(|frame| {
            frame.content().text().map(|t| {
                t.split('/')
                    .next()
                    .unwrap_or("0")
                    .parse::<u32>()
                    .unwrap_or(0)
            })
        })
    }

    /// Extract and save album art from ID3 tag
    fn extract_album_art(
        item: &mut LibraryItem,
        tag: &Tag,
        file_path: &std::path::Path,
        album_art_dir: &Path,
    ) {
        for pic in tag.pictures() {
            let file_name = Self::generate_picture_filename(file_path, pic, album_art_dir);

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
    }

    /// Generate unique filename for album art
    fn generate_picture_filename(
        file_path: &std::path::Path,
        pic: &id3::frame::Picture,
        album_art_dir: &Path,
    ) -> PathBuf {
        let extension = match pic.mime_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            _ => "jpg",
        };

        album_art_dir.join(format!(
            "{}_{}_{}.{}",
            file_path.file_stem().unwrap_or_default().to_string_lossy(),
            u8::from(pic.picture_type),
            rand::thread_rng().gen::<u64>(),
            extension
        ))
    }

    /// Send library view back via command channel
    fn send_library_view(
        items: &[LibraryItem],
        path_id: LibraryPathId,
        path_display: &str,
        lib_cmd_tx: Sender<LibraryCommand>,
    ) {
        let mut library_view = LibraryView {
            view_type: ViewType::Album,
            containers: Vec::new(),
        };

        let container = LibraryItemContainer {
            name: format!("Folder: {}", path_display),
            items: items.to_vec(),
        };

        library_view.containers.push(container);

        let _ = lib_cmd_tx.send(LibraryCommand::AddView(library_view));
        let _ = lib_cmd_tx.send(LibraryCommand::AddPathId(path_id));
    }
}
