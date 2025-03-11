use eframe::egui::{vec2, Button, Color32, Stroke};

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
