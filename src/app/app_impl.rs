use eframe::egui;

use super::{App, LibraryCommand};
use crate::app::components::{
    footer::Footer, library_component::LibraryComponent, lyrics_component::LyricsComponent,
    player_component::PlayerComponent, playlist_table::PlaylistTable, playlist_tabs::PlaylistTabs,
    window_chrome::WindowChrome, AppComponent,
};

impl eframe::App for App {
    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        tracing::info!("exiting and saving");
        self.update_player_persistence();
        self.save_state();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Load heavy data on first frame if not already loaded
        if !self.heavy_data_loaded {
            // Show a simple loading screen
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bird Player");
                    ui.add_space(20.0);
                    ui.label("Loading...");
                    ui.add_space(10.0);
                    ui.spinner();
                });
            });

            // Load heavy data synchronously since database is now small and fast
            self.load_heavy_data();
            return;
        }

        if self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if let Some(lib_cmd_rx) = &self.library_cmd_rx {
            if let Ok(lib_cmd) = lib_cmd_rx.try_recv() {
                match lib_cmd {
                    LibraryCommand::AddItem(lib_item) => self.library.add_item(lib_item),
                    LibraryCommand::AddView(lib_view) => self.library.add_view(lib_view),
                    LibraryCommand::AddPathId(path_id) => {
                        self.library.set_path_to_imported(path_id)
                    }
                }
            }
        }

        if let Some(selected_track) = &self.player.as_mut().unwrap().selected_track {
            let display = format!(
                "{} - {} [ Music Player ]",
                &selected_track
                    .artist()
                    .unwrap_or("unknown artist".to_string()),
                &selected_track
                    .title()
                    .unwrap_or("unknown title".to_string())
            );

            ctx.send_viewport_cmd(egui::ViewportCommand::Title(display));
        }

        // Add window chrome at the top
        egui::TopBottomPanel::top("Window Chrome")
            .show_separator_line(true)
            .show(ctx, |ui| {
                WindowChrome::add(self, ui);
            });

        egui::TopBottomPanel::top("Player").show(ctx, |ui| {
            PlayerComponent::add(self, ui);
            ui.add_space(5.0); // Add margin at the bottom
        });

        egui::TopBottomPanel::bottom("Footer").show(ctx, |ui| {
            Footer::add(self, ui);
        });

        egui::CentralPanel::default().show(ctx, |_ui| {
            egui::SidePanel::left("Library Window")
                .default_width(200.0)
                .show(ctx, |ui| {
                    LibraryComponent::add(self, ui);
                });
        });

        // Lyrics panel (right side, can be toggled)
        if self.show_lyrics_panel {
            egui::SidePanel::right("Lyrics Panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Lyrics");
                    ui.separator();
                    LyricsComponent::add(self, ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |_ui| {
            egui::TopBottomPanel::top("Playlist Tabs").show(ctx, |ui| {
                PlaylistTabs::add(self, ui);
            });

            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(_current_playlist_idx) = &mut self.current_playlist_idx {
                    // Create a scroll area with a unique ID for tracking scroll position
                    let playlist_id =
                        format!("playlist_{}", self.current_playlist_idx.unwrap_or(0));
                    let scroll_area_id = ui.id().with(playlist_id).with("scroll_area");

                    egui::ScrollArea::both().show(ui, |ui| {
                        ui.push_id(scroll_area_id, |ui| {
                            PlaylistTable::add(self, ui);
                        });
                    });
                }
            });
        });
    }
}
