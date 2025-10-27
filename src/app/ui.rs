use std::time::Duration;

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

        if let Ok(lib_cmd) = self.lib_cmd_rx().try_recv() {
            match lib_cmd {
                LibraryCommand::AddItem(lib_item) => self.library.add_item(lib_item),
                LibraryCommand::AddView(lib_view) => self.library.add_view(lib_view),
                LibraryCommand::AddPathId(path_id) => self.library.set_path_to_imported(path_id),
            }
        }

        // Check for pending lyrics response
        let (lyrics_received, should_show_panel) = self
            .lyrics_manager
            .check_pending_lyrics(&mut self.ui_state.lyrics_fetch_state);

        if lyrics_received {
            tracing::debug!("📡 Lyrics response received");

            let track_key = self
                .runtime
                .as_ref()
                .and_then(|rt| rt.player.selected_track.as_ref().map(|track| track.key()));

            let lyrics_text_owned = self
                .lyrics_manager
                .current_lyrics()
                .and_then(|lyrics_data| {
                    lyrics_data
                        .synced_lyrics
                        .as_deref()
                        .or(lyrics_data.plain_lyrics.as_deref())
                        .map(|text| text.to_string())
                });

            if should_show_panel {
                self.ui_state.show_lyrics_panel = true;
            }

            // Store the fetched lyrics in the ID3 tag for future use
            if let Some(lyrics_data) = self.lyrics_manager.current_lyrics() {
                if self.runtime.is_some() {
                    let player = self.player_ref();
                    if let Some(track) = &player.selected_track {
                        if let Err(e) = crate::app::lyrics::LyricsService::write_lyrics_to_file(
                            track.path(),
                            lyrics_data,
                        ) {
                            tracing::warn!("⚠️  Failed to cache lyrics in ID3 tag: {}", e);
                        } else {
                            tracing::info!("📝 Successfully cached lyrics in ID3 tag");
                        }
                    }
                }
            }

            // Show the lyrics panel when lyrics are loaded
            if let Some(lyrics_data) = self.lyrics_manager.current_lyrics() {
                if lyrics_data.instrumental {
                    tracing::info!("✅ Found instrumental track");
                } else if !lyrics_data.lines.is_empty() || lyrics_data.plain_lyrics.is_some() {
                    tracing::info!("✅ Lyrics loaded successfully");
                } else {
                    tracing::warn!("⚠️  Lyrics record found but no content available");
                }
            } else {
                tracing::warn!("❌ No lyrics found");
            }

            if let Some(track_key) = track_key {
                self.update_track_lyrics(track_key, lyrics_text_owned.as_deref());
            }
        }

        if let Some(selected_track) = &self.player_ref().selected_track {
            let display = format!(
                "{} - {} [ Music Player ]",
                &selected_track
                    .artist()
                    .unwrap_or("unknown artist".to_string()),
                &selected_track
                    .title()
                    .unwrap_or("unknown title".to_string())
            );

            if self.last_window_title.as_deref() != Some(&display) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(display.clone()));
                self.last_window_title = Some(display);
            }
        } else if self.last_window_title.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title("Bird Player".to_string()));
            self.last_window_title = None;
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

        if self.ui_state.show_lyrics_panel {
            egui::SidePanel::right("Lyrics Panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    LyricsComponent::add(self, ui);
                });
        }

        egui::SidePanel::left("Library Window")
            .default_width(200.0)
            .show(ctx, |ui| {
                LibraryComponent::add(self, ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    PlaylistTabs::add(self, ui);
                });

            ui.add_space(8.0);

            if let Some(current_playlist_idx) = self.current_playlist_idx {
                // Create a scroll area with a unique ID for tracking scroll position
                let playlist_id = format!("playlist_{}", current_playlist_idx);
                let scroll_area_id = ui.id().with(playlist_id).with("scroll_area");

                egui::ScrollArea::both().show(ui, |ui| {
                    ui.push_id(scroll_area_id, |ui| {
                        PlaylistTable::add(self, ui);
                    });
                });
            }
        });

        // Request repaint during playback for smooth synced lyrics updates
        if self.runtime.is_some() {
            let player = self.player_ref();
            if matches!(player.track_state, crate::app::player::TrackState::Playing) {
                ctx.request_repaint_after(Duration::from_millis(33));
            }
        }
    }
}
