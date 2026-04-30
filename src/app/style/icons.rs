//! Icon glyph constants.
//!
//! Phase 1 keeps the existing emoji / ASCII icons but routes every call site
//! through a single module so phase 2 can swap to an icon font (e.g.
//! `egui_phosphor`) by editing one file.

// ─── Playback transport ──────────────────────────────────────────────────
pub const PREV: &str = "|◀";
pub const NEXT: &str = "▶|";
pub const PLAY: &str = "▶";
pub const PAUSE: &str = "⏸";

// ─── Playback modes ──────────────────────────────────────────────────────
pub const MODE_NORMAL: &str = "➡";
pub const MODE_REPEAT: &str = "🔁";
pub const MODE_REPEAT_ONE: &str = "🔂";
pub const MODE_SHUFFLE: &str = "🔀";

// ─── Misc controls ───────────────────────────────────────────────────────
pub const VOLUME: &str = "📢";
/// Toggles the desktop-lyrics floating viewport.
pub const LYRICS_TOGGLE: &str = "词";
pub const SEARCH: &str = "🔍";
pub const CLOSE: &str = "x";

// ─── Lyrics type indicators ──────────────────────────────────────────────
pub const LYRICS_SYNCED: &str = "🎤";
pub const LYRICS_PLAIN: &str = "📝";
pub const LYRICS_INSTRUMENTAL: &str = "🎸";
pub const LYRICS_NONE: &str = "-";
