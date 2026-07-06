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

/// Keep scrollbars in the quiet overlay style macOS users expect. egui's
/// default floating bar expands to 10 px on hover, which reads as a heavy
/// border in compact headers and tables.
pub fn apply_light_scrollbars(spacing: &mut eframe::egui::style::Spacing) {
    spacing.scroll = eframe::egui::style::ScrollStyle::floating();
    spacing.scroll.bar_width = 5.0;
    spacing.scroll.floating_width = 2.0;
    spacing.scroll.floating_allocated_width = 0.0;
    spacing.scroll.bar_inner_margin = 0.0;
    spacing.scroll.bar_outer_margin = 0.0;
    spacing.scroll.active_background_opacity = 0.0;
    spacing.scroll.interact_background_opacity = 0.0;
    spacing.scroll.active_handle_opacity = 0.45;
    spacing.scroll.interact_handle_opacity = 0.75;
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

/// Mutate the current `Visuals` so any button rendered inside the calling
/// scope reads as borderless: transparent inactive, soft hover fill, no
/// outline. Used for chrome menus and accent buttons (e.g. lyrics upload)
/// so they match the borderless treatment we already apply to the window
/// chrome buttons.
pub fn borderless_button_visuals(visuals: &mut eframe::egui::Visuals) {
    let widgets = &mut visuals.widgets;
    let hover_fill = widgets.hovered.weak_bg_fill;
    widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    widgets.inactive.bg_fill = Color32::TRANSPARENT;
    widgets.inactive.bg_stroke = Stroke::NONE;
    widgets.hovered.weak_bg_fill = hover_fill;
    widgets.hovered.bg_fill = hover_fill;
    widgets.hovered.bg_stroke = Stroke::NONE;
    widgets.active.bg_stroke = Stroke::NONE;
}

/// Render a compact panel header row whose bottom rule extends edge-to-edge
/// across its parent panel. egui's `Frame::side_top_panel` (used by all
/// three columns since the central-panel fix) has an 8 px horizontal
/// `inner_margin`; a vanilla `ui.separator()` therefore stops 8 px short of
/// each panel border. `Separator::grow(8.0)` cancels exactly that inset so
/// the rule “seals” the header to the panel edges and the three column
/// header lines read as one continuous horizontal seam.
pub fn panel_header<R>(
    ui: &mut eframe::egui::Ui,
    add_contents: impl FnOnce(&mut eframe::egui::Ui) -> R,
) -> R {
    let inner = ui
        .horizontal(|ui| {
            ui.set_min_height(tokens::size::HEADER_HEIGHT);
            add_contents(ui)
        })
        .inner;
    ui.add(eframe::egui::Separator::default().grow(8.0).spacing(0.0));
    inner
}
