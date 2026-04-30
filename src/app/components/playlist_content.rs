use super::playlist_table::PlaylistTable;
use super::playlist_tabs::PlaylistTabs;
use super::AppComponent;
use crate::app::style::tokens;
use crate::app::App;

pub struct PlaylistContent;

impl AppComponent for PlaylistContent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Header (tab strip) is rendered OUTSIDE any ScrollArea so its
        // bottom separator stays on the same horizontal seam as the
        // library / lyrics panel separators. Wrapping it in
        // ScrollArea::horizontal previously reserved a few pixels for the
        // scrollbar gutter and pushed this separator down.
        PlaylistTabs::add(ctx, ui);

        ui.add_space(tokens::spacing::XS);
        ui.separator();

        if let Some(current_playlist_idx) = ctx.app_settings.current_playlist_idx {
            ui.push_id(("playlist", current_playlist_idx), |ui| {
                PlaylistTable::add(ctx, ui);
            });
        }
    }
}
