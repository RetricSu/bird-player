use super::playlist_table::PlaylistTable;
use super::playlist_tabs::PlaylistTabs;
use super::AppComponent;
use crate::app::style::tokens;
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

        // Visually anchor the tab strip to the table beneath it — mirrors
        // the separator the lyrics panel already uses under its header.
        ui.add_space(tokens::spacing::XS);
        ui.separator();

        if let Some(current_playlist_idx) = ctx.app_settings.current_playlist_idx {
            ui.push_id(("playlist", current_playlist_idx), |ui| {
                PlaylistTable::add(ctx, ui);
            });
        }
    }
}
