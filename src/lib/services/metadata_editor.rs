use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::probe::Probe;
use lofty::tag::{Accessor, TagExt};
use lofty::tag::Tag;
use std::fs;
use std::path::PathBuf;

use crate::library::LibraryItem;

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

        let mut tagged_file = match Probe::open(&path).and_then(|p| p.read()) {
            Ok(file) => file,
            Err(e) => {
                tracing::error!("Failed to open file {:?} for metadata editing: {}", path, e);
                return false;
            }
        };

        let mut tag = match tagged_file.primary_tag_mut() {
            Some(t) => t.clone(),
            None => {
                if let Some(t) = tagged_file.first_tag_mut() {
                    t.clone()
                } else {
                    tracing::info!("Creating new tag for file: {:?}", path);
                    Tag::new(tagged_file.primary_tag_type())
                }
            }
        };

        // Update the tag and track
        if !Self::update_tag_field(&mut tag, track, field, value) {
            return false;
        }

        // Write tag to file
        if let Err(e) = tag.save_to_path(&path, lofty::config::WriteOptions::new()) {
            tracing::error!("Failed to write tag to file {:?}: {}", path, e);
            return false;
        }

        tracing::info!("Successfully updated metadata in file: {:?}", path);

        // Update database
        Self::update_database(track, field, value, db_conn)
    }

    /// Update the specified field in both tag and track
    fn update_tag_field(tag: &mut Tag, track: &mut LibraryItem, field: &str, value: &str) -> bool {
        match field {
            "title" => {
                tag.set_title(value.to_string());
                track.set_title(Some(value));
            }
            "artist" => {
                tag.set_artist(value.to_string());
                track.set_artist(Some(value));
            }
            "album" => {
                tag.set_album(value.to_string());
                track.set_album(Some(value));
            }
            "genre" => {
                tag.set_genre(value.to_string());
                track.set_genre(Some(value));
            }
            _ => {
                tracing::warn!("Unsupported metadata field: {}", field);
                return false;
            }
        }
        true
    }

    /// Update the track's album cover
    pub fn update_track_cover(
        track: &mut LibraryItem,
        image_path: &PathBuf,
    ) -> bool {
        let path = track.path();

        let image_data = match fs::read(image_path) {
            Ok(data) => data,
            Err(e) => {
                tracing::error!("Failed to read new cover image: {}", e);
                return false;
            }
        };

        let mut tagged_file = match Probe::open(&path).and_then(|p| p.read()) {
            Ok(file) => file,
            Err(e) => {
                tracing::error!("Failed to open file {:?} for cover editing: {}", path, e);
                return false;
            }
        };

        let mut tag = match tagged_file.primary_tag_mut() {
            Some(t) => t.clone(),
            None => {
                if let Some(t) = tagged_file.first_tag_mut() {
                    t.clone()
                } else {
                    Tag::new(tagged_file.primary_tag_type())
                }
            }
        };

        // Determine MimeType based on extension
        let ext = image_path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
        let mime_type = match ext.as_str() {
            "png" => MimeType::Png,
            "jpeg" | "jpg" => MimeType::Jpeg,
            "gif" => MimeType::Gif,
            "bmp" => MimeType::Bmp,
            "tiff" => MimeType::Tiff,
            _ => MimeType::Jpeg,
        };

        let picture = Picture::new_unchecked(
            PictureType::CoverFront,
            Some(mime_type),
            None,
            image_data,
        );

        // Remove old front covers
        tag.remove_picture_type(PictureType::CoverFront);
        tag.push_picture(picture);

        if let Err(e) = tag.save_to_path(&path, lofty::config::WriteOptions::new()) {
            tracing::error!("Failed to save new cover to file {:?}: {}", path, e);
            return false;
        }

        tracing::info!("Successfully embedded new album cover for {:?}", path);
        true
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
