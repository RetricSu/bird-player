use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Library {
    paths: HashSet<PathBuf>,
    items: Vec<LibraryItem>,
    library_view: LibraryView,
}

impl Library {
    pub fn new() -> Self {
        Self {
            paths: HashSet::new(),
            items: Vec::new(),
            library_view: LibraryView {
                view_type: ViewType::Album,
                containers: Vec::new(),
            },
        }
    }

    pub fn paths(&self) -> &HashSet<PathBuf> {
        &self.paths
    }

    // TODO - This should potentially be a result and checks if the path really exists on the system?
    // A library path could also have state which holds the PathBuf and validity, so that can be reported in the UI
    // if a user ever removes it from the drive after it has been added to the library.
    pub fn add_path(&mut self, path: PathBuf) -> bool {
        self.paths.insert(path)
    }

    pub fn items(&self) -> &Vec<LibraryItem> {
        self.items.as_ref()
    }

    pub fn view(&self) -> &LibraryView {
        &self.library_view
    }

    pub fn add_item(&mut self, library_item: LibraryItem) {
        self.items.push(library_item);
    }

    pub fn add_view(&mut self, library_view: LibraryView) {
        self.library_view = library_view;
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct LibraryItem {
    path: PathBuf,
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<i32>,
    genre: Option<String>,
    track_number: Option<u32>,
    key: usize,
}

impl LibraryItem {
    pub fn new(path: PathBuf) -> Self {
        use rand::Rng; // TODO - use ULID?
        Self {
            path,
            title: None,
            artist: None,
            album: None,
            year: None,
            genre: None,
            track_number: None,
            key: rand::thread_rng().gen(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn key(&self) -> usize {
        self.key
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
        self.year.clone()
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
        self.track_number.clone()
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
