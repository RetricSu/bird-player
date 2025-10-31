use serde::{Deserialize, Serialize};

use super::ui_state::UiSettings;
use crate::app::i18n::Language;
use crate::app::libstate::player_state::PlayerStateManager;

/// Persistable application settings
/// These are saved to confy and restored on app startup
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Language setting
    pub current_language: Language,

    /// Player state persistence
    #[serde(flatten)]
    pub player: PlayerStateManager,

    /// UI state persistence
    #[serde(flatten)]
    pub ui: UiSettings,

    /// Current selected playlist index
    pub current_playlist_idx: Option<usize>,

    /// Currently playing playlist index
    pub playing_playlist_idx: Option<usize>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            current_language: Language::English,
            player: PlayerStateManager::default(),
            ui: UiSettings::default(),
            current_playlist_idx: None,
            playing_playlist_idx: None,
        }
    }
}
