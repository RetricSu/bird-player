mod app;
mod app_impl;
mod components;
pub mod constants;
pub mod font;
pub mod i18n;
pub mod icon;
pub mod library;
pub mod lyrics;
pub mod player;
mod playlist;
pub mod services;
pub mod state;
mod style;
mod version;
pub mod viewport;

pub use app::{
    App, AudioCommand, LibraryCommand, LibraryItem, LibraryPathId, Playlist, TempError, UiCommand,
};

// Re-export the i18n functions for convenience
pub use i18n::{get_language, set_language, t, tf, Language};

// Re-export commonly used state types
pub use state::{ui_state::LyricsFetchState, AppState, PlayerStateManager, StatePersistence};
