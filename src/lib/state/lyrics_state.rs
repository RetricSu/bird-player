#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LyricsFetchState {
    Idle,
    Loading,
    Loaded,
    Failed(String),
}

impl Default for LyricsFetchState {
    fn default() -> Self {
        Self::Idle
    }
}
