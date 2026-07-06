//! Icon glyph constants.
//!
//! All icons resolve to glyphs from the bundled Phosphor regular font, which
//! is registered in `font::setup_fonts`. Keeping every icon string in one
//! place lets us swap variants (regular / fill / bold) globally by editing
//! this file alone.

use egui_phosphor::regular as p;

// ─── Playback transport ──────────────────────────────────────────────────
pub const PREV: &str = p::SKIP_BACK;
pub const NEXT: &str = p::SKIP_FORWARD;
pub const PLAY: &str = p::PLAY;
pub const PAUSE: &str = p::PAUSE;

// ─── Playback modes ──────────────────────────────────────────────────────
pub const MODE_NORMAL: &str = p::ARROW_RIGHT;
pub const MODE_REPEAT: &str = p::REPEAT;
pub const MODE_REPEAT_ONE: &str = p::REPEAT_ONCE;
pub const MODE_SHUFFLE: &str = p::SHUFFLE;

// ─── Misc controls ───────────────────────────────────────────────────────
pub const VOLUME: &str = p::SPEAKER_HIGH;
pub const VOLUME_MUTE: &str = p::SPEAKER_X;
/// Toggles the desktop-lyrics floating viewport.
pub const LYRICS_TOGGLE: &str = p::MICROPHONE_STAGE;
/// Toggles the in-app lyrics side panel.
pub const LYRICS_PANEL: &str = p::SUBTITLES;
pub const SEARCH: &str = p::MAGNIFYING_GLASS;
pub const CLOSE: &str = p::X;
pub const DOWNLOAD: &str = p::DOWNLOAD_SIMPLE;
pub const DISCOVER: &str = p::COMPASS;
pub const EXPORT: &str = p::EXPORT;
pub const YOUTUBE: &str = p::YOUTUBE_LOGO;
pub const FOLDER: &str = p::FOLDER_OPEN;
/// Generic "add / new" affordance — used by library/playlist "+" buttons.
pub const PLUS: &str = p::PLUS;
/// Indicator shown in the playlist's lyrics column when a track has lyrics.
pub const LYRICS_PRESENT: &str = p::MUSIC_NOTES;

// ─── Lyrics type indicators ──────────────────────────────────────────────
pub const LYRICS_SYNCED: &str = p::MICROPHONE;
pub const LYRICS_PLAIN: &str = p::NOTE_PENCIL;
pub const LYRICS_INSTRUMENTAL: &str = p::GUITAR;
pub const LYRICS_NONE: &str = p::MINUS;

// ─── Window chrome ───────────────────────────────────────────────────────
pub const WINDOW_MINIMIZE: &str = p::MINUS;
pub const WINDOW_MAXIMIZE: &str = p::CORNERS_OUT;
pub const WINDOW_RESTORE: &str = p::CORNERS_IN;
pub const WINDOW_CLOSE: &str = p::X;
