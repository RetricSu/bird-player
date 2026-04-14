use std::path::Path;

use eframe::egui;

use super::{
    actions::PendingActions, localization::PlaylistLocalization, state::PlaylistTableState,
};

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_number_column(
    ui: &mut egui::Ui,
    row_id: egui::Id,
    column_width: f32,
    idx: usize,
    highlight_color: Option<egui::Color32>,
    state: &mut PlaylistTableState,
    dragged_item: &mut Option<usize>,
    is_dragging: &mut bool,
    ctrl_pressed: bool,
    actions: &mut PendingActions,
) {
    ui.push_id(row_id.with("number_col"), |ui| {
        ui.set_min_width(column_width);

        let number_str = format!("{}", idx + 1);
        let mut text = egui::RichText::new(number_str).strong();
        if let Some(color) = highlight_color {
            text = text.color(color);
        }

        let drag_handle_response =
            ui.add(egui::Label::new(text).sense(egui::Sense::click_and_drag()));

        if drag_handle_response.hovered() && !*is_dragging {
            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
        }

        if drag_handle_response.dragged() && dragged_item.is_none() {
            state.begin_drag(ui, idx);
            *dragged_item = state.dragged_item();
            *is_dragging = state.is_dragging();
        }

        if drag_handle_response.clicked() && ctrl_pressed {
            actions.toggle_selection(idx);
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_title_column(
    ui: &mut egui::Ui,
    row_id: egui::Id,
    column_width: f32,
    idx: usize,
    is_dragging: bool,
    ctrl_pressed: bool,
    is_current_track: bool,
    track_title: &str,
    highlight_color: Option<egui::Color32>,
    localization: &PlaylistLocalization,
    state: &mut PlaylistTableState,
    actions: &mut PendingActions,
    editing_field: Option<&str>,
    editing_track_idx: Option<usize>,
) {
    ui.push_id(row_id.with("title_col"), |ui| {
        ui.set_min_width(column_width);

        if editing_field == Some("title") && editing_track_idx == Some(idx) {
            let mut current_value = state.edit_buffer_or(track_title);
            let response = ui.text_edit_singleline(&mut current_value);

            state.set_edit_value(ui, current_value.clone());
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            if enter_pressed || response.lost_focus() {
                if current_value != track_title {
                    actions.update_metadata(idx, "title", current_value);
                }
                state.clear_edit(ui);
            }
        } else {
            let mut text = egui::RichText::new(track_title);
            if let Some(color) = highlight_color {
                text = text.color(color);
            }

            let title_response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));

            if title_response.hovered() && !is_dragging {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
            }

            title_response.context_menu(|ui| {
                if ui.button(localization.menu_edit_title()).clicked() {
                    state.begin_edit(ui, "title", idx, track_title.to_string());
                    ui.close_menu();
                }

                if ui
                    .button(localization.menu_remove_from_playlist())
                    .clicked()
                {
                    actions.remove_track(idx);
                    ui.close_menu();
                }
            });

            if title_response.double_clicked() && !is_dragging {
                state.begin_edit(ui, "title", idx, track_title.to_string());
            }

            if title_response.clicked() && !title_response.double_clicked() && !is_dragging {
                if ctrl_pressed {
                    actions.toggle_selection(idx);
                } else if !is_current_track {
                    actions.play_track(idx);
                }
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_artist_column(
    ui: &mut egui::Ui,
    row_id: egui::Id,
    column_width: f32,
    idx: usize,
    is_dragging: bool,
    ctrl_pressed: bool,
    track_artist: &str,
    highlight_color: Option<egui::Color32>,
    localization: &PlaylistLocalization,
    state: &mut PlaylistTableState,
    actions: &mut PendingActions,
    editing_field: Option<&str>,
    editing_track_idx: Option<usize>,
) {
    ui.push_id(row_id.with("artist_col"), |ui| {
        ui.set_min_width(column_width);

        if editing_field == Some("artist") && editing_track_idx == Some(idx) {
            let mut current_value = state.edit_buffer_or(track_artist);
            let response = ui.text_edit_singleline(&mut current_value);

            state.set_edit_value(ui, current_value.clone());
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            if response.lost_focus() || enter_pressed {
                if current_value != track_artist {
                    actions.update_metadata(idx, "artist", current_value);
                }
                state.clear_edit(ui);
            }
        } else {
            let mut text = egui::RichText::new(track_artist);
            if let Some(color) = highlight_color {
                text = text.color(color);
            }

            let artist_response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));

            artist_response.context_menu(|ui| {
                if ui.button(localization.menu_edit_artist()).clicked() {
                    state.begin_edit(ui, "artist", idx, track_artist.to_string());
                    ui.close_menu();
                }

                if ui
                    .button(localization.menu_remove_from_playlist())
                    .clicked()
                {
                    actions.remove_track(idx);
                    ui.close_menu();
                }
            });

            if artist_response.double_clicked() && !is_dragging {
                state.begin_edit(ui, "artist", idx, track_artist.to_string());
            }

            if artist_response.clicked()
                && !artist_response.double_clicked()
                && !is_dragging
                && ctrl_pressed
            {
                actions.toggle_selection(idx);
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_album_column(
    ui: &mut egui::Ui,
    row_id: egui::Id,
    column_width: f32,
    idx: usize,
    is_dragging: bool,
    ctrl_pressed: bool,
    track_album: &str,
    highlight_color: Option<egui::Color32>,
    localization: &PlaylistLocalization,
    state: &mut PlaylistTableState,
    actions: &mut PendingActions,
    editing_field: Option<&str>,
    editing_track_idx: Option<usize>,
) {
    ui.push_id(row_id.with("album_col"), |ui| {
        ui.set_min_width(column_width);

        if editing_field == Some("album") && editing_track_idx == Some(idx) {
            let mut current_value = state.edit_buffer_or(track_album);
            let response = ui.text_edit_singleline(&mut current_value);

            state.set_edit_value(ui, current_value.clone());
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            if response.lost_focus() || enter_pressed {
                if current_value != track_album {
                    actions.update_metadata(idx, "album", current_value);
                }
                state.clear_edit(ui);
            }
        } else {
            let mut text = egui::RichText::new(track_album);
            if let Some(color) = highlight_color {
                text = text.color(color);
            }

            let album_response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));

            album_response.context_menu(|ui| {
                if ui.button(localization.menu_edit_album()).clicked() {
                    state.begin_edit(ui, "album", idx, track_album.to_string());
                    ui.close_menu();
                }

                if ui
                    .button(localization.menu_remove_from_playlist())
                    .clicked()
                {
                    actions.remove_track(idx);
                    ui.close_menu();
                }
            });

            if album_response.double_clicked() && !is_dragging {
                state.begin_edit(ui, "album", idx, track_album.to_string());
            }

            if album_response.clicked()
                && !album_response.double_clicked()
                && !is_dragging
                && ctrl_pressed
            {
                actions.toggle_selection(idx);
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_lyrics_column(
    ui: &mut egui::Ui,
    row_id: egui::Id,
    column_width: f32,
    has_lyrics: bool,
    track_key: String,
    track_path: &Path,
    localization: &PlaylistLocalization,
    actions: &mut PendingActions,
) {
    ui.push_id(row_id.with("lyrics_col"), |ui| {
        ui.set_min_width(column_width);

        if has_lyrics {
            ui.horizontal(|ui| {
                ui.label("🎵");
                if ui
                    .small_button("❌")
                    .on_hover_text(localization.remove_lyrics())
                    .clicked()
                {
                    if let Err(e) =
                        crate::app::lyrics::LyricsService::remove_lyrics_from_file(track_path)
                    {
                        tracing::error!("Failed to remove lyrics from file: {}", e);
                    } else {
                        tracing::info!("Successfully removed lyrics from file: {:?}", track_path);
                        actions.clear_lyrics(track_key);
                    }
                }
            });
        } else {
            ui.label("―");
        }
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_genre_column(
    ui: &mut egui::Ui,
    row_id: egui::Id,
    column_width: f32,
    idx: usize,
    is_dragging: bool,
    ctrl_pressed: bool,
    track_genre: &str,
    highlight_color: Option<egui::Color32>,
    localization: &PlaylistLocalization,
    state: &mut PlaylistTableState,
    actions: &mut PendingActions,
    editing_field: Option<&str>,
    editing_track_idx: Option<usize>,
) {
    ui.push_id(row_id.with("genre_col"), |ui| {
        ui.set_min_width(column_width);

        if editing_field == Some("genre") && editing_track_idx == Some(idx) {
            let mut current_value = state.edit_buffer_or(track_genre);
            let response = ui.text_edit_singleline(&mut current_value);

            state.set_edit_value(ui, current_value.clone());
            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));

            if response.lost_focus() || enter_pressed {
                if current_value != track_genre {
                    actions.update_metadata(idx, "genre", current_value);
                }
                state.clear_edit(ui);
            }
        } else {
            let mut text = egui::RichText::new(track_genre);
            if let Some(color) = highlight_color {
                text = text.color(color);
            }

            let genre_response = ui.add(egui::Label::new(text).sense(egui::Sense::click()));

            genre_response.context_menu(|ui| {
                if ui.button(localization.menu_edit_genre()).clicked() {
                    state.begin_edit(ui, "genre", idx, track_genre.to_string());
                    ui.close_menu();
                }

                if ui
                    .button(localization.menu_remove_from_playlist())
                    .clicked()
                {
                    actions.remove_track(idx);
                    ui.close_menu();
                }
            });

            if genre_response.double_clicked() && !is_dragging {
                state.begin_edit(ui, "genre", idx, track_genre.to_string());
            }

            if genre_response.clicked()
                && !genre_response.double_clicked()
                && !is_dragging
                && ctrl_pressed
            {
                actions.toggle_selection(idx);
            }
        }
    });
}
