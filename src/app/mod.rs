mod components;
pub mod constants;
mod core;
pub mod font;
pub mod i18n;
pub mod icon;
pub mod library;
pub mod lyrics;
mod messaging;
pub mod player;
mod playlist;
pub mod services;
pub mod state;
mod style;
mod ui;
mod version;
pub mod viewport;

pub use core::{App, LibraryCommand, LibraryItem, LibraryPathId, Playlist, TempError};
pub use messaging::{AudioCommand, UiCommand};

// Re-export the i18n functions for convenience
pub use i18n::{get_language, set_language, t, tf, Language};

// Re-export commonly used state types
pub use state::{ui_state::LyricsFetchState, AppState, PlayerStateManager, StatePersistence};
