use super::AppComponent;
use crate::app::App;

pub struct Footer;

impl AppComponent for Footer {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            if let Some(current_playlist_idx) = ctx.app_settings.current_playlist_idx {
                let selection_count = ctx.playlists[current_playlist_idx].selected_indices.len();
                if selection_count > 0 {
                    ui.label(format!("{} selected", selection_count));

                    if ui.button("Clear Selection").clicked() {
                        let playlist = &mut ctx.playlists[current_playlist_idx];
                        playlist.clear_selection();
                    }
                }
            }
        });
    }
}
