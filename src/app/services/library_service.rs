use std::sync::Arc;
use std::sync::Mutex;

use crate::app::lib_services::MetadataEditor;
use crate::app::library::{Library, LibraryCommand, LibraryItem, Picture};
use crate::app::playlist::Playlist;

/// Application-level library service that coordinates all library management operations
///
/// This service acts as a coordinator between the application state and various
/// library-related operations (metadata editing, command processing, etc.).
#[derive(Default)]
pub struct LibraryService;

impl LibraryService {
    /// Update track metadata and synchronize changes across library and playlists
    pub fn update_track_metadata(
        track: &mut LibraryItem,
        field: &str,
        value: &str,
        library: &mut Library,
        playlists: &mut [Playlist],
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> bool {
        // Use the MetadataEditor service
        let success = MetadataEditor::update_track_metadata(track, field, value, db_conn);

        if success {
            // Update all instances of this track in playlists
            for playlist in playlists.iter_mut() {
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

            // Reload the library from the database
            if let Ok(updated_library) = Library::load_from_db(db_conn) {
                *library = updated_library;
            }
        }

        success
    }

    /// Process a library command and update the library accordingly
    pub fn process_library_command(library: &mut Library, lib_cmd: LibraryCommand) {
        match lib_cmd {
            LibraryCommand::AddItem(lib_item) => library.add_item(*lib_item),
            LibraryCommand::AddView(lib_view) => library.add_view(lib_view),
            LibraryCommand::AddPathId(path_id) => library.set_path_to_imported(path_id),
        }
    }

    /// Update track cover image
    #[allow(dead_code)]
    pub fn update_track_cover(
        track: &mut LibraryItem,
        image_path: &std::path::PathBuf,
        album_art_dir: &std::path::Path,
        library: &mut Library,
        playlists: &mut [Playlist],
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> bool {
        let success = MetadataEditor::update_track_cover(track, image_path);

        if success {
            let picture = Self::picture_from_selected_cover(image_path, album_art_dir);

            track.clear_pictures();
            if let Some(picture) = picture {
                track.add_picture(picture.clone());
            }

            library.update_item_pictures(track.key_str(), track.pictures().clone());

            for playlist in playlists.iter_mut() {
                for playlist_track in playlist.tracks.iter_mut() {
                    if playlist_track.key() == track.key() {
                        playlist_track.clear_pictures();
                        for picture in track.pictures() {
                            playlist_track.add_picture(picture.clone());
                        }
                    }
                }
            }

            if let Err(err) = library.save_to_db(db_conn) {
                tracing::error!("Failed to save updated cover metadata: {}", err);
            }
        }
        success
    }

    fn picture_from_selected_cover(
        image_path: &std::path::Path,
        album_art_dir: &std::path::Path,
    ) -> Option<Picture> {
        if let Err(err) = std::fs::create_dir_all(album_art_dir) {
            tracing::error!("Failed to create album art directory: {}", err);
            return None;
        }

        let extension = image_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg")
            .to_ascii_lowercase();
        let mime_type = match extension.as_str() {
            "png" => "image/png",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "tiff" | "tif" => "image/tiff",
            _ => "image/jpeg",
        };
        let file_name = format!(
            "cover_{}_{}.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or_default(),
            rand::random::<u64>(),
            extension
        );
        let destination = album_art_dir.join(file_name);

        if let Err(err) = std::fs::copy(image_path, &destination) {
            tracing::error!("Failed to copy selected cover image: {}", err);
            return None;
        }

        Some(Picture::new(
            mime_type.to_string(),
            lofty::picture::PictureType::CoverFront.as_u8(),
            String::new(),
            destination,
        ))
    }
}
