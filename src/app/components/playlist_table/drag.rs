use crate::app::App;
use eframe::egui;

use super::row_texts::LocalizedFallbacks;
use super::state::PlaylistTableState;

pub(crate) fn render_drag_placeholder_row(
    ui: &mut egui::Ui,
    available_width: f32,
    column_proportions: &[f32],
    num_columns: usize,
) {
    for proportion in column_proportions.iter().take(num_columns) {
        ui.scope(|ui| {
            let col_width = available_width * proportion;
            ui.set_min_width(col_width);
            ui.label("");
        });
    }
    ui.end_row();
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_drag_feedback(
    ctx: &App,
    ui: &mut egui::Ui,
    state: &mut PlaylistTableState,
    current_playlist_idx: usize,
    dragged_item: Option<usize>,
    pointer_pos: Option<egui::Pos2>,
    row_rects: &[(usize, egui::Rect)],
    fallbacks: &LocalizedFallbacks,
) {
    let Some(drag_idx) = dragged_item else {
        state.set_drop_target(ui, None);
        return;
    };

    let Some(pointer_pos) = pointer_pos else {
        state.set_drop_target(ui, None);
        return;
    };

    let mut nearest_row: Option<(usize, egui::Rect)> = None;
    let mut best_distance = f32::MAX;

    for (idx, rect) in row_rects.iter() {
        if Some(*idx) == dragged_item {
            continue;
        }

        let distance = (rect.center().y - pointer_pos.y).abs();
        if distance < best_distance {
            best_distance = distance;
            nearest_row = Some((*idx, *rect));
        }
    }

    if let Some((target_idx, target_rect)) = nearest_row {
        state.set_drop_target(ui, Some(target_idx));

        let insert_above = pointer_pos.y < target_rect.center().y;
        let line_y = if insert_above {
            target_rect.min.y
        } else {
            target_rect.max.y
        };

        let line_rect = egui::Rect::from_min_max(
            egui::pos2(target_rect.min.x, line_y - 1.0),
            egui::pos2(target_rect.max.x, line_y + 1.0),
        );
        ui.painter()
            .rect_filled(line_rect, 0.0, egui::Color32::from_rgb(50, 150, 250));
    } else {
        state.set_drop_target(ui, None);
    }

    if let Some(track) = ctx.playlists[current_playlist_idx].tracks.get(drag_idx) {
        let rect_height = 24.0;
        let rect_width = 400.0;
        let drag_rect = egui::Rect::from_min_max(
            egui::pos2(pointer_pos.x - 10.0, pointer_pos.y - rect_height / 2.0),
            egui::pos2(
                pointer_pos.x + rect_width,
                pointer_pos.y + rect_height / 2.0,
            ),
        );

        ui.painter().rect_filled(
            drag_rect,
            4.0,
            egui::Color32::from_rgba_premultiplied(100, 100, 180, 200),
        );

        let drag_text = track.title_ref().unwrap_or(fallbacks.title());
        ui.painter().text(
            drag_rect.center(),
            egui::Align2::CENTER_CENTER,
            drag_text,
            egui::FontId::default(),
            egui::Color32::WHITE,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_drag_end(
    ctx: &mut App,
    ui: &mut egui::Ui,
    state: &mut PlaylistTableState,
    current_playlist_idx: usize,
    dragged_item: Option<usize>,
    pointer_pos: Option<egui::Pos2>,
    row_rects: &[(usize, egui::Rect)],
    playlist_len: usize,
) {
    if playlist_len == 0 {
        state.clear_drag(ui);
        return;
    }

    if let (Some(drag_idx), Some(drop_idx)) = (dragged_item, state.drop_target()) {
        if drag_idx != drop_idx && drag_idx < playlist_len && drop_idx < playlist_len {
            let insert_before = pointer_pos
                .and_then(|pos| {
                    row_rects
                        .iter()
                        .find(|(i, _)| *i == drop_idx)
                        .map(|(_, rect)| pos.y < rect.center().y)
                })
                .unwrap_or(false);

            let target_pos = if insert_before {
                if drop_idx < drag_idx {
                    drop_idx
                } else {
                    drop_idx.saturating_sub(1)
                }
            } else if drop_idx > drag_idx {
                drop_idx
            } else {
                drop_idx
                    .saturating_add(1)
                    .min(playlist_len.saturating_sub(1))
            };

            ctx.playlists[current_playlist_idx].reorder(drag_idx, target_pos);
        }
    }

    state.clear_drag(ui);
}
