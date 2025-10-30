use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use super::ui_state::UiSettings;
use crate::app::i18n::Language;
use crate::app::libstate::player_state::PlayerSettings;

/// Persistable application settings
/// These are saved to confy and restored on app startup
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    /// Language setting
    pub current_language: Language,

    /// Player state persistence
    #[serde(flatten)]
    pub player: PlayerSettings,

    /// UI state persistence
    #[serde(flatten)]
    pub ui: UiSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            current_language: Language::English,
            player: PlayerSettings::default(),
            ui: UiSettings::default(),
        }
    }
}

/// State persistence manager
pub struct StatePersistence;

impl StatePersistence {
    /// Load application settings from confy
    pub fn load_settings() -> Result<AppSettings, confy::ConfyError> {
        confy::load::<AppSettings>("bird-player", None)
    }

    /// Save application settings to confy
    pub fn save_settings(settings: &AppSettings) -> Result<(), confy::ConfyError> {
        confy::store("bird-player", None, settings)
    }

    /// Save library to database
    pub fn save_library(
        library: &crate::app::library::Library,
        db_conn: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        library.save_to_db(db_conn)?;
        Ok(())
    }

    /// Save playlists to database
    pub fn save_playlists(
        playlists: &mut [crate::app::playlist::Playlist],
        db_conn: &std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for playlist in playlists.iter_mut() {
            playlist.save_to_db_and_update_id(db_conn)?;
        }
        Ok(())
    }

    /// Load library from database
    pub fn load_library(
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<crate::app::library::Library, Box<dyn std::error::Error>> {
        Ok(crate::app::library::Library::load_from_db(db_conn)?)
    }

    /// Load playlists from database
    pub fn load_playlists(
        db_conn: &Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Vec<crate::app::playlist::Playlist>, Box<dyn std::error::Error>> {
        Ok(crate::app::playlist::Playlist::load_all_from_db(db_conn)?)
    }
}
