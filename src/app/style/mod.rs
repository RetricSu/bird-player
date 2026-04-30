use eframe::egui::{style::HandleShape, vec2, Button, Color32, Slider, Stroke};

pub mod icons;
pub mod tokens;

pub trait ButtonExt {
    fn player_style(self) -> Self;
}

impl ButtonExt for Button<'_> {
    /// Default styling for the player's icon buttons. Inherits stroke / fill
    /// from the active egui theme so the button reads in both light and dark
    /// modes. For an *active* (toggled-on) variant, use [`player_button`].
    fn player_style(self) -> Self {
        self.min_size(vec2(tokens::size::ICON_BTN, tokens::size::ICON_BTN))
            .fill(Color32::TRANSPARENT)
            .corner_radius(tokens::radius::SM)
    }
}

/// Convenience constructor for player icon buttons that need an *active*
/// state visual (e.g. desktop-lyrics on, non-default playback mode). When
/// `active` is true the button is filled with the brand colour and gets a
/// matching stroke; when false it falls back to the regular `player_style`.
pub fn player_button(label: &str, active: bool) -> Button<'_> {
    let btn = Button::new(label).player_style();
    if active {
        btn.fill(tokens::color::BRAND).stroke(Stroke::new(
            tokens::size::STROKE_WIDTH,
            tokens::color::BRAND_ACTIVE,
        ))
    } else {
        btn
    }
}

/// Apply brand-aware visual tweaks on top of the default egui visuals.
/// Used at boot via `egui_ctx.style_mut(...)` — see `bootstrap.rs`.
pub fn apply_brand_visuals(visuals: &mut eframe::egui::Visuals) {
    visuals.selection.bg_fill = tokens::color::BRAND;
    visuals.selection.stroke.color = tokens::color::BRAND_ACTIVE;
}

/// Tighten egui's default whitespace so the UI reads as compact rather than
/// roomy. The defaults (item_spacing 8×3, button_padding 4×1, indent 18) are
/// tuned for desktop apps with low information density; for a music player
/// with lists, tabs, and a control band stacked vertically those defaults
/// translate to a lot of empty pixels. We pull them in by ~25 % so rows in
/// the library / playlist table sit closer together and the player band
/// stops dominating the viewport.
pub fn apply_compact_spacing(spacing: &mut eframe::egui::style::Spacing) {
    spacing.item_spacing = eframe::egui::vec2(tokens::spacing::SM + 2.0, tokens::spacing::XS + 1.0);
    spacing.button_padding = eframe::egui::vec2(tokens::spacing::SM + 2.0, tokens::spacing::XS);
    spacing.menu_margin =
        eframe::egui::Margin::symmetric(tokens::spacing::SM as i8, tokens::spacing::XS as i8);
    spacing.indent = 14.0;
    spacing.interact_size.y = 22.0;
}

pub trait SliderExt {
    fn volume_style(self) -> Self;
}

impl SliderExt for Slider<'_> {
    fn volume_style(self) -> Self {
        self.handle_shape(HandleShape::Circle)
            .logarithmic(false)
            .show_value(false)
            .step_by(0.01)
            .handle_shape(HandleShape::Rect { aspect_ratio: 0.3 })
    }
}
