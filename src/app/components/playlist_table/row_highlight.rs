use crate::app::style::tokens;
use eframe::egui;

pub(crate) fn paint_selection_background(ui: &egui::Ui, row_rect: egui::Rect, is_selected: bool) {
    if !is_selected {
        return;
    }

    ui.painter()
        .rect_filled(row_rect, 0.0, tokens::color::PLAYLIST_ROW_HIGHLIGHT);
}
