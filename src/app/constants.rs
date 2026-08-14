// Window size constants
pub const DEFAULT_WINDOW_WIDTH: f32 = 750.0;
pub const DEFAULT_WINDOW_HEIGHT: f32 = 468.0;
pub const DEFAULT_WINDOW_TITLE: &str = "Bird Player";

/// Development builds must never open the user's production database.
///
/// Release installers opt into `production-data` explicitly. Plain
/// `cargo run`, including `cargo run --release`, uses an isolated profile.
#[cfg(feature = "production-data")]
pub const CONFIG_APP_NAME: &str = "bird-player";

#[cfg(not(feature = "production-data"))]
pub const CONFIG_APP_NAME: &str = "bird-player-dev";
