use eframe::egui::{style::HandleShape, vec2, Button, Color32, Slider, Stroke};

pub mod icons;
pub mod tokens;

pub trait ButtonExt {
    fn player_style(self) -> Self;
}

impl ButtonExt for Button<'_> {
    fn player_style(self) -> Self {
        // NOTE(phase-2): the black stroke is invisible in dark mode. Phase 2
        // will switch this to a `&Ui`-aware helper that pulls the stroke
        // colour from `ui.visuals().widgets.inactive.bg_stroke`.
        self.min_size(vec2(tokens::size::ICON_BTN, tokens::size::ICON_BTN))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(tokens::size::STROKE_WIDTH, Color32::BLACK))
            .corner_radius(tokens::radius::SM)
    }
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
