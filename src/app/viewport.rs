use super::constants::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};
use eframe::egui;
use eframe::{egui::IconData, NativeOptions};

pub fn build_viewport_with_icon(icon: IconData) -> NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT])
            .with_min_inner_size([300.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_icon(icon)
            .with_resizable(true),
        ..Default::default()
    }
}
