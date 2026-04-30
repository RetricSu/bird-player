//! Design tokens — single source of truth for spacing, sizing, radii, and colours.
//!
//! Phase 1 of the UI redesign extracts every magic number / literal colour
//! into this module. Visual output is unchanged: every constant carries the
//! exact value previously hard-coded at its call site. Later phases will:
//!   - swap the brand palette into theme-aware places (current line, buttons)
//!   - replace emoji icon literals (see `icons.rs`) with a real icon font
//!   - tighten the `spacing::*` scale once usage settles

use eframe::egui::Color32;

/// Whitespace scale (in egui logical pixels).
#[allow(dead_code)] // LG/XL reserved for upcoming layout work
pub mod spacing {
    pub const XS: f32 = 2.0;
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const XL: f32 = 20.0;
}

/// Corner radii.
pub mod radius {
    /// Used by the player control buttons today.
    pub const SM: f32 = 5.0;
    /// Used by the album-art placeholder card.
    pub const LG: f32 = 8.0;
}

/// Component sizes (logical pixels).
pub mod size {
    /// Square icon-button side length used in the player control row.
    pub const ICON_BTN: f32 = 40.0;
    /// Album-art / cassette square side length.
    pub const ALBUM: f32 = 180.0;
    /// Volume slider width.
    pub const SLIDER_VOLUME: f32 = 160.0;
    /// Stroke width used by the player buttons and album-art card.
    pub const STROKE_WIDTH: f32 = 1.0;
}

/// Typography sizes (in egui logical pixels).
#[allow(dead_code)] // LG reserved for phase-3 title hierarchy
pub mod text {
    pub const SM: f32 = 12.0;
    pub const MD: f32 = 14.0;
    pub const LG: f32 = 16.0;
}

/// Named colours. Phase 2 will start consuming `BRAND_*` in place of the
/// per-component blues currently in use; for now they are defined but not
/// yet wired in so that the visual output matches main exactly.
pub mod color {
    use super::Color32;

    // ─── Brand palette ────────────────────────────────────────────────────
    /// Primary brand blue. Used for selection backgrounds and active toggle
    /// buttons (desktop-lyrics on, non-default playback modes, etc.).
    pub const BRAND: Color32 = Color32::from_rgb(0x2D, 0x7D, 0xEC);
    /// Hover variant of the brand colour. Reserved for hover-state styling
    /// once we push per-button visuals (see phase 3).
    #[allow(dead_code)]
    pub const BRAND_HOVER: Color32 = Color32::from_rgb(0x4A, 0x93, 0xF0);
    /// Pressed/active variant of the brand colour.
    pub const BRAND_ACTIVE: Color32 = Color32::from_rgb(0x1E, 0x68, 0xCF);

    // ─── Lyrics ───────────────────────────────────────────────────────────
    /// Soft blue used for the lyrics-type indicator (🎤/📝/🎸).
    pub const LYRICS_TYPE_ICON: Color32 = Color32::from_rgb(100, 150, 255);
    /// Red used for the "lyrics unavailable: …" status line.
    pub const LYRICS_FAILED: Color32 = Color32::from_rgb(230, 80, 80);
    /// Highlight colour for the synced-lyrics current line.
    pub const LYRICS_CURRENT_LINE: Color32 = Color32::BLUE;

    // ─── Playlist (drag/highlight) ────────────────────────────────────────
    /// Border colour of the row-being-dragged ghost.
    pub const PLAYLIST_DRAG_BORDER: Color32 = Color32::from_rgb(120, 120, 180);
    /// Insertion-point line drawn between rows during drag.
    pub const PLAYLIST_DRAG_INSERT_LINE: Color32 = Color32::from_rgb(50, 150, 250);
    /// Translucent fill for the drag ghost row.
    pub const PLAYLIST_DRAG_GHOST_FILL: Color32 =
        Color32::from_rgba_premultiplied(100, 100, 180, 200);
    /// Highlight overlay for selected/playing row.
    pub const PLAYLIST_ROW_HIGHLIGHT: Color32 =
        Color32::from_rgba_premultiplied(100, 150, 255, 200);

    // ─── Album-art placeholder ────────────────────────────────────────────
    pub const ALBUM_BG_DARK: Color32 = Color32::from_rgb(30, 30, 35);
    pub const ALBUM_BG_LIGHT: Color32 = Color32::from_rgb(220, 220, 225);
    pub const ALBUM_STROKE_DARK: Color32 = Color32::from_rgb(60, 60, 65);
    pub const ALBUM_STROKE_LIGHT: Color32 = Color32::from_rgb(160, 160, 165);
}
