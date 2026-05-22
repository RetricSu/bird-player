use super::playlist_table::PlaylistTable;
use super::playlist_tabs::PlaylistTabs;
use super::AppComponent;
use crate::app::App;

pub struct PlaylistContent;

impl AppComponent for PlaylistContent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Header (tab strip) is rendered OUTSIDE any ScrollArea so its
        // bottom rule stays on the same horizontal seam as the library /
        // lyrics panel rules.
        crate::app::style::panel_header(ui, |ui| {
            PlaylistTabs::add(ctx, ui);
        });

        if let Some(current_playlist_idx) = ctx.app_settings.current_playlist_idx {
            ui.push_id(("playlist", current_playlist_idx), |ui| {
                PlaylistTable::add(ctx, ui);
            });
        }
    }
}
