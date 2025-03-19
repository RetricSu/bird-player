use crate::app::LibraryItem;
use crate::AudioCommand;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Playlist {
    name: Option<String>,
    pub tracks: Vec<LibraryItem>,
    pub selected: Option<LibraryItem>,
    #[serde(skip_serializing, skip_deserializing)]
    pub selected_indices: HashSet<usize>,
}

impl Default for Playlist {
    fn default() -> Self {
        Self::new()
    }
}

impl Playlist {
    pub fn new() -> Self {
        Self {
            name: None,
            tracks: vec![],
            selected: None,
            selected_indices: HashSet::new(),
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    pub fn add(&mut self, track: LibraryItem) {
        self.tracks.push(track);
    }

    // TODO - should probably return a Result
    pub fn remove(&mut self, idx: usize) {
        self.tracks.remove(idx);
        self.selected_indices.remove(&idx);

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
        self.tracks.iter().position(|t| t == track)
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
}

#[cfg(test)]
mod tests {
    use crate::app::library::LibraryPathId;

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

        let mut playlist = Playlist {
            name: Some("test".to_string()),
            tracks: vec![
                LibraryItem::new(path1.clone(), LibraryPathId::new(0)),
                LibraryItem::new(path2.clone(), LibraryPathId::new(1)),
                LibraryItem::new(path3.clone(), LibraryPathId::new(2)),
            ],
            selected: None,
            selected_indices: HashSet::new(),
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

        let mut playlist = Playlist {
            name: Some("test".to_string()),
            tracks: vec![
                LibraryItem::new(path1.clone(), LibraryPathId::new(0)),
                LibraryItem::new(path2.clone(), LibraryPathId::new(1)),
                LibraryItem::new(path3.clone(), LibraryPathId::new(2)),
            ],
            selected: None,
            selected_indices: HashSet::new(),
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
    //         name: Some("test".to_string()),
    //         tracks: vec![track1, track2, track3.clone()],
    //         selected: None,
    //     };

    //     assert_eq!(playlist.tracks.len(), 3);

    //     playlist.select(2);

    //     assert_eq!(playlist.selected, Some(track3));
    // }
}
