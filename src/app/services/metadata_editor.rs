use id3::{Tag, TagLike, Version};
use std::path::PathBuf;

use crate::app::library::LibraryItem;

/// Service for editing audio file metadata
pub struct MetadataEditor;

impl MetadataEditor {
    /// Update a metadata field for a track
    ///
    /// Returns true if successful, false otherwise.
    /// Updates both the file's ID3 tag and the database.
    pub fn update_track_metadata(
        track: &mut LibraryItem,
        field: &str,
        value: &str,
        db_conn: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> bool {
        let path = track.path();

        // Read or create ID3 tag
        let mut tag = match Self::read_or_create_tag(&path) {
            Ok(tag) => tag,
            Err(e) => {
                tracing::error!("Failed to read/create ID3 tag for {:?}: {}", path, e);
                return false;
            }
        };

        // Update the tag and track
        if !Self::update_tag_field(&mut tag, track, field, value) {
            return false;
        }

        // Write tag to file
        if !Self::write_tag_to_file(&tag, &path) {
            return false;
        }

        // Update database
        Self::update_database(track, field, value, db_conn)
    }

    /// Read existing tag or create a new one
    fn read_or_create_tag(path: &PathBuf) -> Result<Tag, id3::Error> {
        match Tag::read_from_path(path) {
            Ok(tag) => Ok(tag),
            Err(err) => {
                if let id3::ErrorKind::NoTag = err.kind {
                    tracing::info!("Creating new ID3 tag for file: {:?}", path);
                    Ok(Tag::new())
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Update the specified field in both tag and track
    fn update_tag_field(tag: &mut Tag, track: &mut LibraryItem, field: &str, value: &str) -> bool {
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
            _ => {
                tracing::warn!("Unsupported metadata field: {}", field);
                return false;
            }
        }
        true
    }

    /// Write tag to file
    fn write_tag_to_file(tag: &Tag, path: &PathBuf) -> bool {
        match tag.write_to_path(path, Version::Id3v24) {
            Ok(_) => {
                tracing::info!("Successfully updated metadata in file: {:?}", path);
                true
            }
            Err(e) => {
                tracing::error!("Failed to write tag to file {:?}: {}", path, e);
                false
            }
        }
    }

    /// Update the database with new metadata
    fn update_database(
        track: &LibraryItem,
        field: &str,
        value: &str,
        db_conn: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> bool {
        let mut conn_guard = match db_conn.lock() {
            Ok(guard) => guard,
            Err(e) => {
                tracing::error!("Failed to acquire database lock: {}", e);
                return false;
            }
        };

        let tx = match conn_guard.transaction() {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Failed to start database transaction: {}", e);
                return false;
            }
        };

        let query = format!("UPDATE library_items SET {} = ?1 WHERE key = ?2", field);
        let result = tx.execute(&query, rusqlite::params![value, track.key().to_string()]);

        match result.and_then(|_| tx.commit()) {
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
    }
}
