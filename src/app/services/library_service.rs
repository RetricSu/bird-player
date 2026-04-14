use std::sync::Arc;
use std::sync::Mutex;

use crate::app::lib_services::MetadataEditor;
use crate::app::library::{Library, LibraryCommand, LibraryItem};
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
        library: &mut Library,
        _playlists: &mut [Playlist],
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> bool {
        let success = MetadataEditor::update_track_cover(track, image_path);

        // When the cover changes, we simply reload the library from the database
        // and playlists just like `update_track_metadata` does because the cover
        // path needs to be refreshed from the newly extracted data
        if success {
            if let Ok(updated_library) = Library::load_from_db(db_conn) {
                *library = updated_library;
            }
        }
        success
    }
}
