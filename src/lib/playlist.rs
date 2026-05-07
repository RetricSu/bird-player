use crate::AudioCommand;
use crate::LibraryItem;
use rusqlite::{Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Option<i64>,
    name: Option<String>,
    description: Option<String>,
    created_at: i64,
    updated_at: i64,
    pub tracks: Vec<LibraryItem>,
    pub selected: Option<LibraryItem>,
    #[serde(skip_serializing, skip_deserializing)]
    pub selected_indices: HashSet<usize>,
    #[serde(skip)]
    pub is_dirty: bool,
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

impl Playlist {
    fn current_timestamp_ms() -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    fn touch(&mut self) {
        self.updated_at = Self::current_timestamp_ms();
        self.is_dirty = true;
    }

    pub fn new() -> Self {
        let now = Self::current_timestamp_ms();

        Self {
            id: None,
            name: None,
            description: None,
            created_at: now,
            updated_at: now,
            tracks: vec![],
            selected: None,
            selected_indices: HashSet::new(),
            is_dirty: true,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
        self.touch();
    }

    pub fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn description(&self) -> Option<String> {
        self.description.clone()
    }

    pub fn set_description(&mut self, description: Option<String>) {
        self.description = description;
        self.touch();
    }

    pub fn created_at(&self) -> i64 {
        self.created_at
    }

    pub fn updated_at(&self) -> i64 {
        self.updated_at
    }

    pub fn add(&mut self, track: LibraryItem) {
        self.tracks.push(track);
        self.touch();
    }

    // TODO - should probably return a Result
    pub fn remove(&mut self, idx: usize) {
        self.tracks.remove(idx);
        self.selected_indices.remove(&idx);
        self.touch();

        // Update indices greater than the removed index
        let mut to_remove = Vec::new();
        let mut to_add = Vec::new();

        for &i in &self.selected_indices {
            if i > idx {
                to_remove.push(i);
                to_add.push(i - 1);
            }
        }

        for i in to_remove {
            self.selected_indices.remove(&i);
        }

        for i in to_add {
            self.selected_indices.insert(i);
        }
    }

    // TODO - should probably return a Result
    pub fn reorder(&mut self, current_pos: usize, destination_pos: usize) {
        let track = self.tracks.remove(current_pos);
        self.tracks.insert(destination_pos, track);
        self.touch();

        // Update selected indices after reordering
        let mut new_selected = HashSet::new();

        for &idx in &self.selected_indices {
            if idx == current_pos {
                new_selected.insert(destination_pos);
            } else if (idx < current_pos && idx < destination_pos)
                || (idx > current_pos && idx > destination_pos)
            {
                new_selected.insert(idx);
            } else if idx < current_pos && idx >= destination_pos {
                new_selected.insert(idx + 1);
            } else if idx > current_pos && idx <= destination_pos {
                new_selected.insert(idx - 1);
            }
        }

        self.selected_indices = new_selected;
    }

    // TODO - should probably return a Result
    pub fn select(&mut self, idx: usize, audio_cmd_tx: &Sender<AudioCommand>) {
        tracing::info!("SELECTED");
        let track = self.tracks[idx].clone();
        let path = &track.path();
        audio_cmd_tx
            .send(AudioCommand::LoadFile((*path).clone()))
            .expect("Failed to send to audio thread");

        self.selected = Some(track);
    }

    pub fn get_pos(&self, track: &LibraryItem) -> Option<usize> {
        self.get_pos_by_key(track.key_str())
    }

    pub fn get_pos_by_key(&self, key: &str) -> Option<usize> {
        self.tracks.iter().position(|t| t.key_str() == key)
    }

    pub fn select_all(&mut self) {
        self.selected_indices.clear();
        for i in 0..self.tracks.len() {
            self.selected_indices.insert(i);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_indices.clear();
    }

    pub fn toggle_selection(&mut self, idx: usize) {
        if self.selected_indices.contains(&idx) {
            self.selected_indices.remove(&idx);
        } else {
            self.selected_indices.insert(idx);
        }
    }

    pub fn is_selected(&self, idx: usize) -> bool {
        self.selected_indices.contains(&idx)
    }

    // Database methods

    pub fn save_to_db(&mut self, conn: &Arc<Mutex<Connection>>) -> SqlResult<()> {
        if !self.is_dirty {
            return Ok(());
        }

        let mut conn = conn.lock().unwrap();

        // Start a transaction
        let tx = conn.transaction()?;

        // Insert or update the playlist record
        let playlist_id = match self.id {
            Some(id) => {
                // Update existing playlist
                tx.execute(
                    "UPDATE playlists SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
                    rusqlite::params![self.name, self.description, self.updated_at, id],
                )?;
                id
            }
            None => {
                // Insert new playlist
                tx.execute(
                    "INSERT INTO playlists (name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![self.name, self.description, self.created_at, self.updated_at],
                )?;
                tx.last_insert_rowid()
            }
        };

        // Clear existing playlist items
        tx.execute(
            "DELETE FROM playlist_items WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
        )?;

        // Insert the tracks with their positions
        for (position, track) in self.tracks.iter().enumerate() {
            tx.execute(
                "INSERT INTO playlist_items (playlist_id, library_item_id, position) 
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![playlist_id, track.key().to_string(), position as i32],
            )?;
        }

        // Commit the transaction
        tx.commit()?;
        self.is_dirty = false;

        Ok(())
    }

    pub fn save_to_db_and_update_id(&mut self, conn: &Arc<Mutex<Connection>>) -> SqlResult<()> {
        if !self.is_dirty {
            return Ok(());
        }

        let mut conn = conn.lock().unwrap();

        // Start a transaction
        let tx = conn.transaction()?;

        // Insert or update the playlist record
        let playlist_id = match self.id {
            Some(id) => {
                // Update existing playlist
                tx.execute(
                    "UPDATE playlists SET name = ?1, description = ?2, updated_at = ?3 WHERE id = ?4",
                    rusqlite::params![self.name, self.description, self.updated_at, id],
                )?;
                id
            }
            None => {
                // Insert new playlist
                tx.execute(
                    "INSERT INTO playlists (name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![self.name, self.description, self.created_at, self.updated_at],
                )?;
                tx.last_insert_rowid()
            }
        };

        // Update the playlist ID if it was None
        if self.id.is_none() {
            self.id = Some(playlist_id);
        }

        // Clear existing playlist items
        tx.execute(
            "DELETE FROM playlist_items WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
        )?;

        // Insert the tracks with their positions
        for (position, track) in self.tracks.iter().enumerate() {
            tx.execute(
                "INSERT INTO playlist_items (playlist_id, library_item_id, position) 
                 VALUES (?1, ?2, ?3)",
                rusqlite::params![playlist_id, track.key().to_string(), position as i32],
            )?;
        }

        // Commit the transaction
        tx.commit()?;
        self.is_dirty = false;

        Ok(())
    }

    pub fn load_from_db(conn: &Arc<Mutex<Connection>>, playlist_id: i64) -> SqlResult<Self> {
        let conn_guard = conn.lock().unwrap();

        // Get the playlist info
        let mut stmt = conn_guard.prepare(
            "SELECT id, name, description, created_at, updated_at FROM playlists WHERE id = ?1"
        )?;

        let mut playlist_rows = stmt.query(rusqlite::params![playlist_id])?;

        if let Some(row) = playlist_rows.next()? {
            let id: i64 = row.get(0)?;
            let name: Option<String> = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let created_at: i64 = row.get(3)?;
            let updated_at: i64 = row.get(4)?;

            let mut playlist = Playlist {
                id: Some(id),
                name,
                description,
                created_at,
                updated_at,
                tracks: vec![],
                selected: None,
                selected_indices: HashSet::new(),
                is_dirty: false,
            };

            // Get the tracks
            let mut items_stmt = conn_guard.prepare(
                "SELECT li.key, li.library_path_id, li.path, li.title, li.artist, li.album, li.year, li.genre, li.track_number, li.lyrics 
                 FROM library_items li
                 JOIN playlist_items pi ON li.key = pi.library_item_id
                 WHERE pi.playlist_id = ?1
                 ORDER BY pi.position",
            )?;

            let mut library_item_rows = items_stmt.query(rusqlite::params![playlist_id])?;

            while let Some(row) = library_item_rows.next()? {
                // Build a LibraryItem from the row
                let key_str: String = row.get(0)?;
                let library_id_raw: i64 = row.get(1)?;
                let path: String = row.get(2)?;

                let library_id = crate::library::LibraryPathId::new(library_id_raw as usize);
                let mut item = LibraryItem::new(std::path::PathBuf::from(path), library_id);

                // Set metadata fields
                item.set_title(row.get::<_, Option<String>>(3)?.as_deref());
                item.set_artist(row.get::<_, Option<String>>(4)?.as_deref());
                item.set_album(row.get::<_, Option<String>>(5)?.as_deref());
                item.set_year(row.get::<_, Option<i32>>(6)?);
                item.set_genre(row.get::<_, Option<String>>(7)?.as_deref());
                item.set_track_number(row.get::<_, Option<u32>>(8)?);
                item.set_lyrics(row.get::<_, Option<String>>(9)?.as_deref());

                // Set the key from the database
                item.set_key(key_str.clone());

                // Load album art (pictures) from the database
                let mut pic_stmt = conn_guard.prepare(
                    "SELECT mime_type, picture_type, description, file_path 
                     FROM pictures WHERE library_item_id = ?",
                )?;

                let picture_rows = pic_stmt.query_map(rusqlite::params![key_str], |row| {
                    let mime_type: String = row.get(0)?;
                    let picture_type: u8 = row.get(1)?;
                    let description: String = row.get(2)?;
                    let file_path: String = row.get(3)?;

                    Ok(crate::library::Picture::new(
                        mime_type,
                        picture_type,
                        description,
                        std::path::PathBuf::from(file_path),
                    ))
                })?;

                for picture in picture_rows {
                    item.add_picture(picture?);
                }

                playlist.tracks.push(item);
            }

            Ok(playlist)
        } else {
            Err(rusqlite::Error::QueryReturnedNoRows)
        }
    }

    pub fn load_all_from_db(conn: &Arc<Mutex<Connection>>) -> SqlResult<Vec<Self>> {
        let mut playlists = Vec::new();

        // First, get all playlist IDs
        let playlist_ids = {
            let conn_guard = conn.lock().unwrap();
            let mut stmt = conn_guard.prepare("SELECT id FROM playlists")?;
            let id_iter = stmt.query_map([], |row| row.get::<_, i64>(0))?;

            // Collect IDs into a Vec to release the connection lock
            let mut ids = Vec::new();
            for id_result in id_iter {
                ids.push(id_result?);
            }
            ids
        };

        // Now load each playlist by ID
        for id in playlist_ids {
            match Self::load_from_db(conn, id) {
                Ok(playlist) => playlists.push(playlist),
                Err(e) => tracing::error!("Failed to load playlist {}: {}", id, e),
            }
        }

        Ok(playlists)
    }

    pub fn delete_from_db(conn: &Arc<Mutex<Connection>>, playlist_id: i64) -> SqlResult<()> {
        let mut conn_guard = conn.lock().unwrap();

        // Start a transaction
        let tx = conn_guard.transaction()?;

        // First delete playlist items
        tx.execute(
            "DELETE FROM playlist_items WHERE playlist_id = ?1",
            rusqlite::params![playlist_id],
        )?;

        // Then delete the playlist
        tx.execute(
            "DELETE FROM playlists WHERE id = ?1",
            rusqlite::params![playlist_id],
        )?;

        // Commit the transaction
        tx.commit()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::library::LibraryPathId;

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn create_playlist() {
        let playlist = Playlist::new();

        assert_eq!(playlist.name, None);
        assert_eq!(playlist.tracks.len(), 0);
        assert_eq!(playlist.selected, None);
    }

    #[test]
    fn set_name() {
        let mut playlist = Playlist::new();
        playlist.set_name("Test".to_string());

        assert_eq!(playlist.name, playlist.get_name());
        assert_eq!(playlist.tracks.len(), 0);
        assert_eq!(playlist.selected, None);
    }

    #[test]
    fn add_track_to_playlist() {
        let track = LibraryItem::new(PathBuf::from(r"C:\music\song.mp3"), LibraryPathId::new(0));

        let mut playlist = Playlist::new();
        playlist.add(track);

        assert_eq!(playlist.tracks.len(), 1);
    }

    #[test]
    fn remove_track_from_playlist() {
        let path1 = PathBuf::from(r"C:\music\song1.mp3");
        let path2 = PathBuf::from(r"C:\music\song2.mp3");
        let path3 = PathBuf::from(r"C:\music\song3.mp3");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let mut playlist = Playlist {
            id: None,
            name: Some("test".to_string()),
            description: None,
            created_at: now,
            updated_at: now,
            tracks: vec![
                LibraryItem::new(path1.clone(), LibraryPathId::new(0)),
                LibraryItem::new(path2.clone(), LibraryPathId::new(1)),
                LibraryItem::new(path3.clone(), LibraryPathId::new(2)),
            ],
            selected: None,
            selected_indices: HashSet::new(),
            is_dirty: false,
        };

        assert_eq!(playlist.tracks.len(), 3);

        playlist.remove(1);

        assert_eq!(playlist.tracks.len(), 2);
        assert_eq!(playlist.tracks.first().unwrap().path(), path1);
        assert_eq!(playlist.tracks.last().unwrap().path(), path3);
    }

    #[test]
    fn reorder_track_in_playlist() {
        let path1 = PathBuf::from(r"C:\music\song1.mp3");
        let path2 = PathBuf::from(r"C:\music\song2.mp3");
        let path3 = PathBuf::from(r"C:\music\song3.mp3");

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let mut playlist = Playlist {
            id: None,
            name: Some("test".to_string()),
            description: None,
            created_at: now,
            updated_at: now,
            tracks: vec![
                LibraryItem::new(path1.clone(), LibraryPathId::new(0)),
                LibraryItem::new(path2.clone(), LibraryPathId::new(1)),
                LibraryItem::new(path3.clone(), LibraryPathId::new(2)),
            ],
            selected: None,
            selected_indices: HashSet::new(),
            is_dirty: false,
        };

        assert_eq!(playlist.tracks.len(), 3);

        playlist.reorder(0, 2);

        assert_eq!(playlist.tracks.len(), 3);
        assert_eq!(playlist.tracks[0].path(), path2);
        assert_eq!(playlist.tracks[1].path(), path3);
        assert_eq!(playlist.tracks[2].path(), path1);
    }

    // #[test]
    // fn select_track() {
    //     let track1 = LibraryItem::new(PathBuf::from(r"C:\music\song1.mp3"));
    //     let track2 = LibraryItem::new(PathBuf::from(r"C:\music\song2.mp3"));
    //     let track3 = LibraryItem::new(PathBuf::from(r"C:\music\song3.mp3"));

    //     let mut playlist = Playlist {
    //         id: None,
    //         name: Some("test".to_string()),
    //         tracks: vec![track1, track2, track3.clone()],
    //         selected: None,
    //     };

    //     assert_eq!(playlist.tracks.len(), 3);

    //     playlist.select(2);

    //     assert_eq!(playlist.selected, Some(track3));
    // }

    #[test]
    fn test_playlist_metadata_fields() {
        let playlist = Playlist::new();

        // Check that timestamps are initialized
        assert!(playlist.created_at() > 0);
        assert!(playlist.updated_at() > 0);
        assert_eq!(playlist.created_at(), playlist.updated_at());

        // Check that description is None by default
        assert_eq!(playlist.description(), None);
    }

    #[test]
    fn test_set_description() {
        let mut playlist = Playlist::new();
        let initial_updated_at = playlist.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        playlist.set_description(Some("My favorite songs".to_string()));

        assert_eq!(playlist.description(), Some("My favorite songs".to_string()));
        assert!(playlist.updated_at() > initial_updated_at);
    }

    #[test]
    fn test_set_name_updates_timestamp() {
        let mut playlist = Playlist::new();
        let initial_updated_at = playlist.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        playlist.set_name("Test Playlist".to_string());

        assert!(playlist.updated_at() > initial_updated_at);
    }

    #[test]
    fn test_add_updates_timestamp() {
        let mut playlist = Playlist::new();
        let initial_updated_at = playlist.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let track = LibraryItem::new(PathBuf::from(r"C:\music\song.mp3"), LibraryPathId::new(0));
        playlist.add(track);

        assert!(playlist.updated_at() > initial_updated_at);
    }

    #[test]
    fn test_remove_updates_timestamp() {
        let mut playlist = Playlist::new();
        let track = LibraryItem::new(PathBuf::from(r"C:\music\song.mp3"), LibraryPathId::new(0));
        playlist.add(track);

        let initial_updated_at = playlist.updated_at();
        std::thread::sleep(std::time::Duration::from_millis(10));

        playlist.remove(0);

        assert!(playlist.updated_at() > initial_updated_at);
    }

    #[test]
    fn test_reorder_updates_timestamp() {
        let mut playlist = Playlist::new();
        playlist.add(LibraryItem::new(PathBuf::from(r"C:\music\song1.mp3"), LibraryPathId::new(0)));
        playlist.add(LibraryItem::new(PathBuf::from(r"C:\music\song2.mp3"), LibraryPathId::new(1)));

        let initial_updated_at = playlist.updated_at();
        std::thread::sleep(std::time::Duration::from_millis(10));

        playlist.reorder(0, 1);

        assert!(playlist.updated_at() > initial_updated_at);
    }
}
