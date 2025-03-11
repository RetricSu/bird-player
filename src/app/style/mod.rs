use eframe::egui::{style::HandleShape, vec2, Button, Color32, Slider, Stroke};

pub trait ButtonExt {
    fn player_style(self) -> Self;
}

impl ButtonExt for Button<'_> {
    fn player_style(self) -> Self {
        self.min_size(vec2(40.0, 40.0))
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, Color32::BLACK))
            .corner_radius(5.0)
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
