use crate::app::constants::CONFIG_APP_NAME;
use crate::app::state::app_state::AppSettings;

/// Configuration persistence manager
/// Low-level persistence operations for configuration files
pub struct ConfigPersistence;

impl ConfigPersistence {
    /// Load application settings from confy
    pub fn load_config() -> Result<AppSettings, confy::ConfyError> {
        confy::load::<AppSettings>(CONFIG_APP_NAME, None)
    }

    /// Save application settings to confy
    pub fn save_config(settings: &AppSettings) -> Result<(), confy::ConfyError> {
        confy::store(CONFIG_APP_NAME, None, settings)
    }
}
