use crate::app::state::config::AppConfig;

/// Configuration persistence manager
/// Low-level persistence operations for configuration files
pub struct ConfigPersistence;

impl ConfigPersistence {
    /// Load application settings from confy
    pub fn load_config() -> Result<AppConfig, confy::ConfyError> {
        confy::load::<AppConfig>("bird-player", None)
    }

    /// Save application settings to confy
    pub fn save_config(settings: &AppConfig) -> Result<(), confy::ConfyError> {
        confy::store("bird-player", None, settings)
    }
}
