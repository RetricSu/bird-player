use super::actions::PendingActions;
use super::columns::{
    render_album_column, render_artist_column, render_genre_column, render_lyrics_column,
    render_number_column, render_title_column,
};
use super::controller;
use super::drag::{handle_drag_end, render_drag_feedback, render_drag_placeholder_row};
use super::post_render::{handle_auto_scroll, handle_scroll_request};
use super::row_highlight::{apply_row_decorations, paint_selection_background, RowHighlight};
use super::row_texts::extract_row_texts;
use super::state::PlaylistTableState;
use crate::app::t;
use crate::app::App;
use eframe::egui;
use std::sync::atomic::{AtomicUsize, Ordering};

static LAST_PLAYED_TRACK: AtomicUsize = AtomicUsize::new(0);

pub(super) fn render(ctx: &mut App, ui: &mut egui::Ui) {
    let Some(current_playlist_idx) = ctx.current_playlist_idx else {
        return;
    };

    let base_id = ui.id().with(format!("playlist_{}", current_playlist_idx));
    let mut state = PlaylistTableState::load(ui, base_id);
    let editing_field = state.editing_field.clone();
    let editing_track_idx = state.editing_track_idx;

    if state.is_dragging() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
        ui.style_mut().interaction.selectable_labels = false;
    }

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    let mouse_released = ui.input(|i| i.pointer.primary_released());

    if !state.is_dragging() || mouse_released {
        state.clear_drop_target(ui);
    }

    let playlist_len = ctx.playlists[current_playlist_idx].tracks.len();
    let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
    let available_width = ui.available_width();
    let column_proportions = [0.05, 0.30, 0.18, 0.20, 0.12, 0.15];
    let num_columns = column_proportions.len();

    let current_track_idx = ctx
        .player_ref()
        .selected_track
        .as_ref()
        .and_then(|track| ctx.playlists[current_playlist_idx].get_pos(track));

    let last_played_track = LAST_PLAYED_TRACK.load(Ordering::Relaxed);

    let mut row_rects = Vec::with_capacity(playlist_len);
    let mut actions = PendingActions::default();
    let mut dragged_item = state.dragged_item();
    let mut is_dragging = state.is_dragging();

    egui::containers::Frame::new()
        .fill(ui.style().visuals.widgets.noninteractive.bg_fill)
        .show(ui, |ui| {
            ui.set_min_width(available_width);

            egui::Grid::new("playlist_full")
                .striped(true)
                .spacing([5.0, 5.0])
                .num_columns(num_columns)
                .show(ui, |ui| {
                    ui.scope(|ui| {
                        ui.set_min_width(available_width * column_proportions[0]);
                        ui.strong(t("column_number"));
                    });

                    ui.scope(|ui| {
                        ui.set_min_width(available_width * column_proportions[1]);
                        ui.strong(t("column_title"));
                    });

                    ui.scope(|ui| {
                        ui.set_min_width(available_width * column_proportions[2]);
                        ui.strong(t("column_artist"));
                    });

                    ui.scope(|ui| {
                        ui.set_min_width(available_width * column_proportions[3]);
                        ui.strong(t("column_album"));
                    });

                    ui.scope(|ui| {
                        ui.set_min_width(available_width * column_proportions[4]);
                        ui.strong(t("column_lyrics"));
                    });

                    ui.scope(|ui| {
                        ui.set_min_width(available_width * column_proportions[5]);
                        ui.strong(t("column_genre"));
                    });

                    ui.end_row();

                    for idx in 0..playlist_len {
                        let row_id = base_id.with(format!("row_{idx}"));
                        let is_being_dragged = dragged_item == Some(idx);

                        if is_being_dragged && is_dragging {
                            render_drag_placeholder_row(
                                ui,
                                available_width,
                                &column_proportions,
                                num_columns,
                            );
                            continue;
                        }

                        let row_rect = ui.available_rect_before_wrap();
                        row_rects.push((idx, row_rect));

                        let is_selected = ctx.playlists[current_playlist_idx].is_selected(idx);
                        paint_selection_background(ui, row_rect, is_selected);

                        let track = &ctx.playlists[current_playlist_idx].tracks[idx];
                        let row_texts = extract_row_texts(track);

                        let drag_handle_text = egui::RichText::new((idx + 1).to_string()).strong();
                        let title_text = egui::RichText::new(row_texts.title.clone());
                        let artist_text = egui::RichText::new(row_texts.artist.clone());
                        let album_text = egui::RichText::new(row_texts.album.clone());
                        let genre_text = egui::RichText::new(row_texts.genre.clone());

                        let RowHighlight {
                            drag_handle,
                            title,
                            artist,
                            album,
                            genre,
                            is_current_track,
                        } = apply_row_decorations(
                            ctx,
                            ui,
                            current_playlist_idx,
                            idx,
                            drag_handle_text,
                            title_text,
                            artist_text,
                            album_text,
                            genre_text,
                        );

                        let mut drag_handle = drag_handle;
                        if is_dragging {
                            drag_handle = drag_handle.color(egui::Color32::from_rgb(120, 120, 180));
                        }

                        render_number_column(
                            ui,
                            row_id,
                            available_width * column_proportions[0],
                            idx,
                            drag_handle,
                            &mut state,
                            &mut dragged_item,
                            &mut is_dragging,
                            ctrl_pressed,
                            &mut actions,
                        );

                        render_title_column(
                            ui,
                            row_id,
                            available_width * column_proportions[1],
                            idx,
                            is_dragging,
                            ctrl_pressed,
                            is_current_track,
                            &row_texts.title,
                            title,
                            &mut state,
                            &mut actions,
                            editing_field.as_deref(),
                            editing_track_idx,
                        );

                        render_artist_column(
                            ui,
                            row_id,
                            available_width * column_proportions[2],
                            idx,
                            is_dragging,
                            ctrl_pressed,
                            &row_texts.artist,
                            artist,
                            &mut state,
                            &mut actions,
                            editing_field.as_deref(),
                            editing_track_idx,
                        );

                        render_album_column(
                            ui,
                            row_id,
                            available_width * column_proportions[3],
                            idx,
                            is_dragging,
                            ctrl_pressed,
                            &row_texts.album,
                            album,
                            &mut state,
                            &mut actions,
                            editing_field.as_deref(),
                            editing_track_idx,
                        );

                        render_lyrics_column(
                            ui,
                            row_id,
                            available_width * column_proportions[4],
                            track.has_lyrics(),
                            track.key(),
                            track.path(),
                            &mut actions,
                        );

                        render_genre_column(
                            ui,
                            row_id,
                            available_width * column_proportions[5],
                            idx,
                            is_dragging,
                            ctrl_pressed,
                            &row_texts.genre,
                            genre,
                            &mut state,
                            &mut actions,
                            editing_field.as_deref(),
                            editing_track_idx,
                        );

                        ui.end_row();
                    }
                });
        });

    controller::apply(ctx, current_playlist_idx, actions);

    if let Some(new_last_played) =
        handle_auto_scroll(ui, current_track_idx, last_played_track, &row_rects)
    {
        LAST_PLAYED_TRACK.store(new_last_played, Ordering::Relaxed);
    }

    handle_scroll_request(ui, &mut state, playlist_len, &row_rects);

    if is_dragging {
        render_drag_feedback(
            ctx,
            ui,
            &mut state,
            current_playlist_idx,
            dragged_item,
            pointer_pos,
            &row_rects,
        );
    }

    if mouse_released && is_dragging {
        handle_drag_end(
            ctx,
            ui,
            &mut state,
            current_playlist_idx,
            dragged_item,
            pointer_pos,
            &row_rects,
            playlist_len,
        );
    }
}
