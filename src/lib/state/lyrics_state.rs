#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LyricsFetchState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Failed(String),
}
