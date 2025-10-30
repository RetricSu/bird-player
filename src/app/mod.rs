pub mod bootstrap;
mod components;
pub mod constants;
mod core;
pub mod db;
mod error;
pub mod font;
pub mod i18n;
pub mod icon;
pub mod runtime;
pub mod state;
mod style;
mod ui;
mod version;
pub mod viewport;

pub mod library {
    pub use bird_player::library::*;
}
pub mod messaging {
    pub use bird_player::messaging::*;
}
pub mod playlist {
    pub use bird_player::playlist::*;
}
pub mod lyrics {
    pub use bird_player::lyrics::*;
}
pub mod player {
    pub use bird_player::player::*;
}
pub mod services {
    pub use bird_player::services::*;
}
pub mod libstate {
    pub use bird_player::state::*;
}

pub use core::{App, LibraryItem, LibraryPathId, Playlist};
pub use library::LibraryCommand;
pub use messaging::AudioEvent;

// Re-export the i18n functions for convenience
pub use i18n::{t, tf, Language};
