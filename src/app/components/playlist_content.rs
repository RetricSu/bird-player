use super::playlist_table::PlaylistTable;
use super::playlist_tabs::PlaylistTabs;
use super::AppComponent;
use crate::app::App;
use eframe::egui;

pub struct PlaylistContent;

impl AppComponent for PlaylistContent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        egui::ScrollArea::horizontal()
            .auto_shrink([false, true])
            .show(ui, |ui| {
                PlaylistTabs::add(ctx, ui);
            });

        ui.add_space(8.0);

        if let Some(current_playlist_idx) = ctx.current_playlist_idx {
            let playlist_id = format!("playlist_{}", current_playlist_idx);
            let scroll_area_id = ui.id().with(playlist_id).with("scroll_area");

            egui::ScrollArea::both().show(ui, |ui| {
                ui.push_id(scroll_area_id, |ui| {
                    PlaylistTable::add(ctx, ui);
                });
            });
        }
    }
}
