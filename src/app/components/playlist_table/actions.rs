#[derive(Default)]
pub(crate) struct PendingActions {
    pub clear_lyrics: Vec<String>,
    pub toggle_selection: Option<usize>,
    pub metadata_updates: Vec<(usize, String, String)>,
    pub play_track: Option<usize>,
    pub remove_track: Option<usize>,
}

impl PendingActions {
    pub(crate) fn clear_lyrics(&mut self, key: String) {
        self.clear_lyrics.push(key);
    }

    pub(crate) fn toggle_selection(&mut self, idx: usize) {
        self.toggle_selection = Some(idx);
    }

    pub(crate) fn update_metadata(&mut self, idx: usize, field: &str, value: String) {
        self.metadata_updates.push((idx, field.to_string(), value));
    }

    pub(crate) fn play_track(&mut self, idx: usize) {
        self.play_track = Some(idx);
    }

    pub(crate) fn remove_track(&mut self, idx: usize) {
        self.remove_track = Some(idx);
    }
}
