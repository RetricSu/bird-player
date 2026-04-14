use lofty::file::TaggedFileExt;
use lofty::picture::Picture as LoftyPicture;
use lofty::probe::Probe;
use lofty::tag::{Accessor};
use lofty::tag::Tag;
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
                if !entry.file_type().is_file() {
                    return false;
                }
                
                let ext = entry.path().extension().unwrap_or(std::ffi::OsStr::new("")).to_string_lossy().to_lowercase();
                matches!(ext.as_str(), "mp3" | "flac" | "wav" | "ogg" | "m4a")
            })
            .collect::<Vec<_>>();

        tracing::info!("Found {} audio files to import", files.len());

        // Parse files in parallel
        let items: Vec<LibraryItem> = files
            .par_iter()
            .map(|entry| Self::parse_audio_file(entry.path(), path_id, album_art_dir))
            .collect();

        // Send items as they're processed
        for item in &items {
            lib_cmd_tx
                .send(LibraryCommand::AddItem(Box::new(item.clone())))
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
        let probe_result = Probe::open(file_path).and_then(|p| p.read());

        match probe_result {
            Ok(tagged_file) => {
                tracing::debug!("Successfully read metadata from: {}", file_path.display());
                if let Some(tag) = tagged_file.primary_tag() {
                    Self::create_item_from_tag(file_path, path_id, tag, album_art_dir)
                } else if let Some(tag) = tagged_file.first_tag() {
                    Self::create_item_from_tag(file_path, path_id, tag, album_art_dir)
                } else {
                    Self::create_fallback_item(file_path, path_id)
                }
            }
            Err(err) => {
                tracing::warn!("Failed to read metadata from {:?}: {}", file_path, err);
                Self::create_fallback_item(file_path, path_id)
            }
        }
    }

    /// Create LibraryItem from Tag
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

        let title = tag.title().unwrap_or(std::borrow::Cow::Borrowed(&filename_title));

        let mut item = LibraryItem::new(file_path.to_path_buf(), path_id)
            .set_title(Some(&title))
            .set_artist(tag.artist().as_deref())
            .set_album(tag.album().as_deref())
            .set_year(tag.year().map(|y| y as i32))
            .set_genre(tag.genre().as_deref())
            .set_track_number(tag.track())
            .set_lyrics(None); // Lyrics will be handled via Lofty's ItemKey::Lyrics in lyrics.rs

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


    /// Extract and save album art from tag
    fn extract_album_art(
        item: &mut LibraryItem,
        tag: &Tag,
        file_path: &std::path::Path,
        album_art_dir: &Path,
    ) {
        for pic in tag.pictures() {
            let file_name = Self::generate_picture_filename(file_path, pic, album_art_dir);

            if let Ok(mut file) = fs::File::create(&file_name) {
                if file.write_all(pic.data()).is_ok() {
                    item.add_picture(Picture::new(
                        pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg").to_string(),
                        pic.pic_type().as_u8(),
                        pic.description().unwrap_or("").to_string(),
                        file_name,
                    ));
                }
            }
        }
    }

    /// Generate unique filename for album art
    fn generate_picture_filename(
        file_path: &std::path::Path,
        pic: &LoftyPicture,
        album_art_dir: &Path,
    ) -> PathBuf {
        let mime_str = pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
        let extension = if mime_str.contains("png") {
            "png"
        } else {
            "jpg"
        };

        album_art_dir.join(format!(
            "{}_{}_{}.{}",
            file_path.file_stem().unwrap_or_default().to_string_lossy(),
            pic.pic_type().as_u8(),
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
