use super::AppComponent;
use crate::app::App;

pub struct LibraryComponent;

impl AppComponent for LibraryComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        eframe::egui::ScrollArea::both().show(ui, |ui| {
            eframe::egui::CollapsingHeader::new(eframe::egui::RichText::new("All Music"))
                .default_open(true)
                .show(ui, |ui| {
                    for container in &ctx.library.view().containers {
                        for item in &container.items {
                            let item_label = ui.add(
                                eframe::egui::Label::new(eframe::egui::RichText::new(
                                    item.title().unwrap_or("unknown title".to_string()),
                                ))
                                .sense(eframe::egui::Sense::click()),
                            );

                            if item_label.double_clicked() {
                                if let Some(current_playlist_idx) = &ctx.current_playlist_idx {
                                    let current_playlist =
                                        &mut ctx.playlists[*current_playlist_idx];

                                    if !current_playlist.tracks.contains(item) {
                                        current_playlist.add(item.clone());
                                    }
                                }
                            }
                        }
                    }
                });
        });
    }
}
