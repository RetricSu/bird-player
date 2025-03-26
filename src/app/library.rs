use rusqlite::{Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    paths: Vec<LibraryPath>,
    items: Vec<LibraryItem>,
    library_view: LibraryView,
}

impl Default for Library {
    fn default() -> Self {
        Self::new()
    }
}

impl Library {
    pub fn new() -> Self {
        Self {
            paths: Vec::new(),
            items: Vec::new(),
            library_view: LibraryView {
                view_type: ViewType::Album,
                containers: Vec::new(),
            },
        }
    }

    pub fn paths(&self) -> &Vec<LibraryPath> {
        &self.paths
    }

    pub fn add_path(&mut self, path: PathBuf) -> bool {
        if self.paths.iter().any(|p| *p.path() == path) {
            false
        } else {
            let new_path = LibraryPath::new(path);
            self.paths.push(new_path);
            true
        }
    }

    pub fn remove_path(&mut self, path_id: LibraryPathId) {
        // Remove the path from the library path list
        if let Some(idx) = self.paths.iter().position(|l| l.id() == path_id) {
            self.paths.remove(idx);
        }

        // Remove the actual items.
        while let Some(idx) = self
            .items
            .iter()
            .position(|item| item.library_id() == path_id)
        {
            self.items.swap_remove(idx);
        }

        // Remove the view container items
        for container in &mut self.library_view.containers {
            while let Some(ct_idx) = container
                .items
                .iter()
                .position(|ci| ci.library_id() == path_id)
            {
                container.items.swap_remove(ct_idx);
            }
        }

        // Remove the empty containers
        while let Some(idx) = self
            .library_view
            .containers
            .iter()
            .position(|ct| ct.items.is_empty())
        {
            self.library_view.containers.swap_remove(idx);
        }
    }

    pub fn set_path_to_imported(&mut self, id: LibraryPathId) {
        for path in self.paths.iter_mut() {
            if path.id() == id {
                path.set_status(LibraryPathStatus::Imported);
            }
        }
    }

    pub fn set_path_to_not_imported(&mut self, id: LibraryPathId) {
        for path in self.paths.iter_mut() {
            if path.id() == id {
                path.set_status(LibraryPathStatus::NotImported);
            }
        }
    }

    pub fn items(&self) -> &Vec<LibraryItem> {
        self.items.as_ref()
    }

    pub fn view(&self) -> &LibraryView {
        &self.library_view
    }

    pub fn add_item(&mut self, library_item: LibraryItem) {
        // Check if an item with this path already exists
        if let Some(idx) = self
            .items
            .iter()
            .position(|item| item.path() == library_item.path())
        {
            // Update the existing item but preserve its key
            let existing_key = self.items[idx].key();
            let mut updated_item = library_item;
            updated_item.set_key(existing_key);
            self.items[idx] = updated_item;
        } else {
            // Add as a new item
            self.items.push(library_item);
        }
    }

    pub fn add_view(&mut self, library_view: LibraryView) {
        let mut new = library_view.containers.clone();

        self.library_view.containers.append(&mut new);
    }

    // Database methods

    pub fn save_to_db(&self, conn: &Arc<Mutex<Connection>>) -> SqlResult<()> {
        let mut conn_guard = conn.lock().unwrap();

        // Start a transaction
        let tx = conn_guard.transaction()?;

        // Save all library paths
        for path in &self.paths {
            let status_value = match path.status() {
                LibraryPathStatus::NotImported => 0,
                LibraryPathStatus::Imported => 1,
            };

            tx.execute(
                "INSERT OR REPLACE INTO library_paths (id, path, status, display_name) 
                 VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![
                    path.id().0 as i64,
                    path.path().to_string_lossy().to_string(),
                    status_value,
                    path.display_name()
                ],
            )?;
        }

        // Save all library items
        for item in &self.items {
            tx.execute(
                "INSERT OR REPLACE INTO library_items 
                 (key, library_path_id, path, title, artist, album, year, genre, track_number, lyrics) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    item.key().to_string(),
                    item.library_id().0 as i64,
                    item.path().to_string_lossy().to_string(),
                    item.title(),
                    item.artist(),
                    item.album(),
                    item.year(),
                    item.genre(),
                    item.track_number(),
                    item.lyrics(),
                ],
            )?;

            // Save pictures for this item
            for picture in item.pictures() {
                tx.execute(
                    "INSERT OR REPLACE INTO pictures 
                     (library_item_id, mime_type, picture_type, description, file_path) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        item.key().to_string(),
                        picture.mime_type,
                        picture.picture_type,
                        picture.description,
                        picture.file_path.to_string_lossy().to_string(),
                    ],
                )?;
            }
        }

        // Commit the transaction
        tx.commit()?;

        Ok(())
    }

    pub fn load_from_db(conn: &Arc<Mutex<Connection>>) -> SqlResult<Self> {
        let conn_guard = conn.lock().unwrap();

        let mut library = Library::new();

        // Load library paths
        let mut path_stmt =
            conn_guard.prepare("SELECT id, path, status, display_name FROM library_paths")?;

        let path_rows = path_stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let path_str: String = row.get(1)?;
            let status_raw: u8 = row.get(2)?;
            let display_name: String = row.get(3)?;

            let status = match status_raw {
                0 => LibraryPathStatus::NotImported,
                _ => LibraryPathStatus::Imported,
            };

            let path = PathBuf::from(path_str);
            let id = LibraryPathId::new(id as usize);

            // Create a new library path but with the database values
            let mut lib_path = LibraryPath::new(path);
            lib_path.id = id;
            lib_path.status = status;
            lib_path.display_name = display_name;

            Ok(lib_path)
        })?;

        for path_result in path_rows {
            library.paths.push(path_result?);
        }

        // Load library items
        let mut item_stmt = conn_guard.prepare(
            "SELECT key, library_path_id, path, title, artist, album, year, genre, track_number, lyrics 
             FROM library_items"
        )?;

        let item_rows = item_stmt.query_map([], |row| {
            let key_str: String = row.get(0)?;
            let library_id_raw: i64 = row.get(1)?;
            let path_str: String = row.get(2)?;

            let library_id = LibraryPathId::new(library_id_raw as usize);
            let path = PathBuf::from(path_str);

            // Create a new library item
            let mut item = LibraryItem::new(path, library_id);

            // Set all metadata
            item.set_title(row.get::<_, Option<String>>(3)?.as_deref());
            item.set_artist(row.get::<_, Option<String>>(4)?.as_deref());
            item.set_album(row.get::<_, Option<String>>(5)?.as_deref());
            item.set_year(row.get::<_, Option<i32>>(6)?);
            item.set_genre(row.get::<_, Option<String>>(7)?.as_deref());
            item.set_track_number(row.get::<_, Option<u32>>(8)?);
            item.set_lyrics(row.get::<_, Option<String>>(9)?.as_deref());

            // Force the key to match the database
            if let Ok(key_val) = key_str.parse::<usize>() {
                item.set_key(key_val);
            }

            Ok(item)
        })?;

        let mut items = Vec::new();
        for item_result in item_rows {
            items.push(item_result?);
        }

        // Load pictures for each item
        for item in &mut items {
            let item_key = item.key() as i64;

            let mut pic_stmt = conn_guard.prepare(
                "SELECT mime_type, picture_type, description, file_path 
                 FROM pictures WHERE library_item_id = ?",
            )?;

            let picture_rows = pic_stmt.query_map(rusqlite::params![item_key], |row| {
                let mime_type: String = row.get(0)?;
                let picture_type: u8 = row.get(1)?;
                let description: String = row.get(2)?;
                let file_path: String = row.get(3)?;

                Ok(Picture::new(
                    mime_type,
                    picture_type,
                    description,
                    PathBuf::from(file_path),
                ))
            })?;

            for picture_result in picture_rows {
                item.add_picture(picture_result?);
            }
        }

        // Add items to the library
        library.items = items;

        // Build view containers from the items
        // This logic depends on how you want to organize your views
        let mut album_containers: std::collections::HashMap<String, LibraryItemContainer> =
            std::collections::HashMap::new();

        for item in &library.items {
            if let Some(album) = item.album() {
                if !album_containers.contains_key(&album) {
                    album_containers.insert(
                        album.clone(),
                        LibraryItemContainer {
                            name: album.clone(),
                            items: Vec::new(),
                        },
                    );
                }

                if let Some(container) = album_containers.get_mut(&album) {
                    container.items.push(item.clone());
                }
            }
        }

        // Convert the HashMap to a Vec
        let containers = album_containers.into_values().collect();

        // Set the library view
        library.library_view = LibraryView {
            view_type: ViewType::Album,
            containers,
        };

        Ok(library)
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct LibraryPath {
    id: LibraryPathId,
    path: PathBuf,
    status: LibraryPathStatus,
    display_name: String,
}

impl LibraryPath {
    pub fn new(path: PathBuf) -> Self {
        use rand::Rng; // TODO - use ULID?
                       // Extract the folder name from the path for display
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Unknown Folder")
            .to_string();

        Self {
            path,
            status: LibraryPathStatus::NotImported,
            id: LibraryPathId::new(rand::thread_rng().gen()),
            display_name,
        }
    }

    pub fn id(&self) -> LibraryPathId {
        self.id
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn status(&self) -> LibraryPathStatus {
        self.status
    }

    pub fn set_status(&mut self, status: LibraryPathStatus) {
        self.status = status;
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct LibraryPathId(pub usize);

impl LibraryPathId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum LibraryPathStatus {
    NotImported,
    Imported,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryItem {
    library_id: LibraryPathId,
    path: PathBuf,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    track_number: Option<u32>,
    key: usize,
    pictures: Vec<Picture>,
    lyrics: Option<String>,
}

impl LibraryItem {
    pub fn new(path: PathBuf, library_id: LibraryPathId) -> Self {
        use rand::Rng; // TODO - use ULID?
        Self {
            library_id,
            path,
            title: None,
            artist: None,
            album: None,
            year: None,
            genre: None,
            track_number: None,
            key: rand::thread_rng().gen(),
            pictures: Vec::new(),
            lyrics: None,
        }
    }

    pub fn library_id(&self) -> LibraryPathId {
        self.library_id
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn key(&self) -> usize {
        self.key
    }

    pub fn set_key(&mut self, key: usize) {
        self.key = key;
    }

    pub fn set_title(&mut self, title: Option<&str>) -> Self {
        if let Some(title) = title {
            self.title = Some(title.to_string());
        }

        self.to_owned()
    }

    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }

    pub fn set_artist(&mut self, artist: Option<&str>) -> Self {
        if let Some(artist) = artist {
            self.artist = Some(artist.to_string());
        }
        self.to_owned()
    }

    pub fn artist(&self) -> Option<String> {
        self.artist.clone()
    }

    pub fn set_album(&mut self, album: Option<&str>) -> Self {
        if let Some(album) = album {
            self.album = Some(album.to_string());
        }
        self.to_owned()
    }

    pub fn album(&self) -> Option<String> {
        self.album.clone()
    }

    pub fn set_year(&mut self, year: Option<i32>) -> Self {
        self.year = year;
        self.to_owned()
    }

    pub fn year(&self) -> Option<i32> {
        self.year
    }

    pub fn set_genre(&mut self, genre: Option<&str>) -> Self {
        if let Some(genre) = genre {
            self.genre = Some(genre.to_string());
        }
        self.to_owned()
    }

    pub fn genre(&self) -> Option<String> {
        self.genre.clone()
    }

    pub fn set_track_number(&mut self, track_number: Option<u32>) -> Self {
        self.track_number = track_number;
        self.to_owned()
    }

    pub fn track_number(&self) -> Option<u32> {
        self.track_number
    }

    pub fn pictures(&self) -> &Vec<Picture> {
        &self.pictures
    }

    pub fn add_picture(&mut self, picture: Picture) {
        self.pictures.push(picture);
    }

    pub fn clear_pictures(&mut self) {
        self.pictures.clear();
    }

    pub fn set_lyrics(&mut self, lyrics: Option<&str>) -> Self {
        if let Some(lyrics) = lyrics {
            self.lyrics = Some(lyrics.to_string());
        }
        self.to_owned()
    }

    pub fn lyrics(&self) -> Option<String> {
        self.lyrics.clone()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryView {
    pub view_type: ViewType,
    pub containers: Vec<LibraryItemContainer>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryItemContainer {
    pub name: String,
    pub items: Vec<LibraryItem>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum ViewType {
    Album,
    Artist,
    Genre,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Picture {
    pub mime_type: String,
    pub picture_type: u8,
    pub description: String,
    pub file_path: PathBuf,
}

impl Picture {
    pub fn new(
        mime_type: String,
        picture_type: u8,
        description: String,
        file_path: PathBuf,
    ) -> Self {
        Self {
            mime_type,
            picture_type,
            description,
            file_path,
        }
    }
}
