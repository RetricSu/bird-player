/// Errors that can occur while bootstrapping the application state.
#[derive(Debug, Clone)]
pub enum AppLoadError {
    MissingAppState,
}

impl std::fmt::Display for AppLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingAppState => write!(f, "Couldn't load app state"),
        }
    }
}

impl std::error::Error for AppLoadError {}
