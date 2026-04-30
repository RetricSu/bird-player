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
pub const SEARCH: &str = p::MAGNIFYING_GLASS;
pub const CLOSE: &str = p::X;

// ─── Lyrics type indicators ──────────────────────────────────────────────
pub const LYRICS_SYNCED: &str = p::MICROPHONE;
pub const LYRICS_PLAIN: &str = p::NOTE_PENCIL;
pub const LYRICS_INSTRUMENTAL: &str = p::GUITAR;
pub const LYRICS_NONE: &str = p::MINUS;
