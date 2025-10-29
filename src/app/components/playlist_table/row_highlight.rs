use eframe::egui;

pub(crate) fn paint_selection_background(ui: &egui::Ui, row_rect: egui::Rect, is_selected: bool) {
    if !is_selected {
        return;
    }

    let highlight_color = egui::Color32::from_rgba_premultiplied(100, 150, 255, 200);
    ui.painter().rect_filled(row_rect, 0.0, highlight_color);
}
