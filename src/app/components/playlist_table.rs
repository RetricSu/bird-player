use super::AppComponent;
use crate::app::App;
use eframe::egui;

pub struct PlaylistTable;

impl AppComponent for PlaylistTable {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        if let Some(current_playlist_idx) = &mut ctx.current_playlist_idx {
            egui::Grid::new("playlist")
                .striped(true)
                .min_col_width(25.)
                .show(ui, |ui| {
                    // Header
                    ui.label("Playing");
                    ui.label("#");
                    ui.label("Title");
                    ui.label("Artist");
                    ui.label("Album");
                    ui.label("Genre");
                    ui.end_row();

                    // Rows
                    for (iter_idx, track) in ctx.playlists[*current_playlist_idx]
                        .tracks
                        .iter()
                        .enumerate()
                    {
                        if let Some(selected_track) = &ctx.player.as_ref().unwrap().selected_track {
                            if selected_track == track {
                                ui.label("▶".to_string());
                            } else {
                                ui.label("-".to_string());
                            }
                        } else {
                            ui.label("-".to_string());
                        }

                        if let Some(track_number) = &track.track_number() {
                            ui.label(track_number.to_string());
                        } else {
                            ui.label((iter_idx + 1).to_string());
                        }

                        let title_label = ui.add(
                            egui::Label::new(track.title().unwrap_or("unknown title".to_string()))
                                .sense(egui::Sense::click()),
                        );
                        if title_label.hovered() {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }

                        ui.label(track.artist().unwrap_or("unknown artist".to_string()));
                        ui.label(track.album().unwrap_or("unknown album".to_string()));
                        ui.label(track.genre().unwrap_or("unknown genre".to_string()));

                        // Temporary hack because I don't yet know how to treat an entire Row
                        // as a response
                        if title_label.clicked() {
                            if ctx.player.as_ref().unwrap().selected_track != Some(track.clone()) {
                                ctx.player.as_mut().unwrap().selected_track = Some(track.clone());
                                ctx.player
                                    .as_mut()
                                    .unwrap()
                                    .select_track(Some(track.clone()));
                                ctx.player.as_mut().unwrap().play();
                            } else {
                                ctx.player.as_mut().unwrap().stop();
                                ctx.player.as_mut().unwrap().selected_track = None;
                            }
                        }

                        ui.end_row();
                    }
                });
        }
    }
}
