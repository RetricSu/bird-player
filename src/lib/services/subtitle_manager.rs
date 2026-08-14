use crate::library::LibraryItem;
use crate::subtitles::{SubtitleService, SubtitleTrack};
use std::path::PathBuf;

#[derive(Default)]
pub struct SubtitleManager {
    current_track: Option<SubtitleTrack>,
    current_path: Option<PathBuf>,
    last_error: Option<String>,
}

impl SubtitleManager {
    pub fn current_track(&self) -> Option<&SubtitleTrack> {
        self.current_track.as_ref()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn load_for_track(&mut self, track: Option<&LibraryItem>) {
        let Some(track) = track else {
            self.current_track = None;
            self.current_path = None;
            self.last_error = None;
            return;
        };
        let path = track.path();
        if self.current_path.as_ref() == Some(&path) {
            return;
        }

        self.current_path = Some(path.clone());
        match SubtitleService::read_from_audio(&path) {
            Ok(track) => {
                self.current_track = track;
                self.last_error = None;
            }
            Err(err) => {
                tracing::warn!(
                    "Failed to load embedded subtitles from '{}': {}",
                    path.display(),
                    err
                );
                self.current_track = None;
                self.last_error = Some(err);
            }
        }
    }
}
