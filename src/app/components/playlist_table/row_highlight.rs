use crate::app::App;
use eframe::egui;

pub(crate) struct RowHighlight {
    pub drag_handle: egui::RichText,
    pub title: egui::RichText,
    pub artist: egui::RichText,
    pub album: egui::RichText,
    pub genre: egui::RichText,
    pub is_current_track: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_row_decorations(
    ctx: &App,
    ui: &egui::Ui,
    current_playlist_idx: usize,
    idx: usize,
    mut drag_handle: egui::RichText,
    mut title: egui::RichText,
    mut artist: egui::RichText,
    mut album: egui::RichText,
    mut genre: egui::RichText,
) -> RowHighlight {
    if let Some(selected_track) = &ctx.player_ref().selected_track {
        if let Some(track) = ctx.playlists[current_playlist_idx].tracks.get(idx) {
            if selected_track == track {
                let highlight_color = ui.style().visuals.selection.bg_fill;
                drag_handle = drag_handle.color(highlight_color);
                title = title.color(highlight_color);
                artist = artist.color(highlight_color);
                album = album.color(highlight_color);
                genre = genre.color(highlight_color);

                return RowHighlight {
                    drag_handle,
                    title,
                    artist,
                    album,
                    genre,
                    is_current_track: true,
                };
            }
        }
    }

    RowHighlight {
        drag_handle,
        title,
        artist,
        album,
        genre,
        is_current_track: false,
    }
}

pub(crate) fn paint_selection_background(ui: &egui::Ui, row_rect: egui::Rect, is_selected: bool) {
    if !is_selected {
        return;
    }

    let highlight_color = egui::Color32::from_rgba_premultiplied(100, 150, 255, 200);
    ui.painter().rect_filled(row_rect, 0.0, highlight_color);
}
