use std::sync::{Arc, Mutex};

/// Database persistence manager
/// Low-level persistence operations for database backend
pub struct DBPersistence;

impl DBPersistence {
    /// Save library to database
    pub fn save_library(
        library: &mut crate::app::library::Library,
        db_conn: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        library.save_to_db(db_conn)?;
        Ok(())
    }

    /// Save playlists to database
    pub fn save_playlists(
        playlists: &mut [crate::app::playlist::Playlist],
        db_conn: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for playlist in playlists.iter_mut() {
            playlist.save_to_db_and_update_id(db_conn)?;
        }
        Ok(())
    }

    /// Load library from database
    pub fn load_library(
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<crate::app::library::Library, Box<dyn std::error::Error>> {
        Ok(crate::app::library::Library::load_from_db(db_conn)?)
    }

    /// Load playlists from database
    pub fn load_playlists(
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Vec<crate::app::playlist::Playlist>, Box<dyn std::error::Error>> {
        Ok(crate::app::playlist::Playlist::load_all_from_db(db_conn)?)
    }
}
