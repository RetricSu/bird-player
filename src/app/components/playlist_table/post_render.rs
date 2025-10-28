use eframe::egui;

use super::state::PlaylistTableState;

pub(crate) fn handle_auto_scroll(
    ui: &mut egui::Ui,
    current_track_idx: Option<usize>,
    last_played_track: usize,
    row_rects: &[(usize, egui::Rect)],
) -> Option<usize> {
    let Some(current_idx) = current_track_idx else {
        return Some(0);
    };

    if last_played_track == current_idx {
        return None;
    }

    if let Some((_, row_rect)) = row_rects.iter().find(|(idx, _)| *idx == current_idx) {
        ui.scroll_to_rect(*row_rect, Some(egui::Align::Center));
    }

    Some(current_idx)
}

pub(crate) fn handle_scroll_request(
    ui: &mut egui::Ui,
    state: &mut PlaylistTableState,
    playlist_len: usize,
    row_rects: &[(usize, egui::Rect)],
) {
    let Some(idx) = state.take_scroll_request(ui) else {
        return;
    };

    if idx >= playlist_len {
        return;
    }

    if let Some((_, row_rect)) = row_rects.iter().find(|(i, _)| *i == idx) {
        ui.scroll_to_rect(*row_rect, Some(egui::Align::Center));
    }
}
