use std::time::Duration;

use eframe::egui;

use super::App;
use crate::app::components::main_shell::MainShell;
use crate::app::components::playlist_booklet_component::PlaylistBookletComponent;
use crate::app::constants::DEFAULT_WINDOW_TITLE;

impl App {
    fn refresh_window_title(&mut self, ctx: &egui::Context) {
        if let Some(track) = self
            .runtime
            .as_ref()
            .and_then(|rt| rt.player.selected_track.as_ref())
        {
            let artist = track.artist_ref().unwrap_or("unknown artist");
            let title = track.title_ref().unwrap_or("unknown title");

            let new_title = format!("{} - {} [ Music Player ]", artist, title);

            if self.ui_state.last_window_title.as_deref() != Some(&new_title) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(new_title.clone()));
                self.ui_state.last_window_title = Some(new_title);
            }
        } else if self.ui_state.last_window_title.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                DEFAULT_WINDOW_TITLE.to_string(),
            ));
            self.ui_state.last_window_title = None;
        }
    }

    fn show_desktop_lyrics(&mut self, ctx: &egui::Context) {
        if !self.ui_state.desktop_lyrics_enabled {
            return;
        }

        let mut current_lyric_text = "BIRD PLAYER".to_string();

        let current_time_ms = if self.runtime.is_some() {
            self.player_ref().seek_to_timestamp
        } else {
            0
        };

        if let Some(lyrics) = self.lyrics_service.manager().current_lyrics() {
            for line in &lyrics.lines {
                let is_current =
                    if let (Some(start), Some(end)) = (line.start_time_ms, line.end_time_ms) {
                        current_time_ms >= start && current_time_ms < end
                    } else if let Some(start) = line.start_time_ms {
                        current_time_ms >= start
                    } else {
                        false
                    };

                if is_current {
                    if !line.text.trim().is_empty() {
                        current_lyric_text = line.text.clone();
                    }
                    break;
                }
            }
        }

        let viewport_builder = egui::ViewportBuilder::default()
            .with_title("Desktop Lyrics")
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_inner_size(egui::vec2(800.0, 100.0));

        let font_size = self.ui_state.desktop_lyrics_font_size;
        let [r, g, b, a] = self.ui_state.desktop_lyrics_color;
        let fg = egui::Color32::from_rgba_unmultiplied(r, g, b, a);
        let locked = self.ui_state.desktop_lyrics_locked;

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("desktop_lyrics_window"),
            viewport_builder,
            move |ctx, _class| {
                let frame = egui::Frame::NONE.fill(egui::Color32::TRANSPARENT);

                egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
                    let full_rect = ui.max_rect();

                    // A 16×16 drag handle in the top-left corner. Only this
                    // region starts a window drag, so the rest of the surface
                    // stays free for future interactions (e.g. text select).
                    // Drag is suppressed entirely when the overlay is locked.
                    if !locked {
                        let handle_rect =
                            egui::Rect::from_min_size(full_rect.min, egui::vec2(16.0, 16.0));
                        let handle_resp = ui.interact(
                            handle_rect,
                            egui::Id::new("desktop_lyrics_drag_handle"),
                            egui::Sense::drag(),
                        );
                        if handle_resp.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        // Visual hint — small dim square so the user can find it.
                        ui.painter().rect_filled(
                            handle_rect.shrink(4.0),
                            2.0,
                            egui::Color32::from_white_alpha(if handle_resp.hovered() {
                                160
                            } else {
                                80
                            }),
                        );
                    }

                    // Render text twice for a cheap stroke/outline effect:
                    // a black drop-shadow underneath, and the coloured glyphs
                    // on top. Greatly improves readability over busy desktops.
                    let painter = ui.painter();
                    let center = full_rect.center();
                    let font_id = egui::FontId::proportional(font_size);
                    let shadow = egui::Color32::from_black_alpha(180);
                    for offset in [
                        egui::vec2(-1.5, 0.0),
                        egui::vec2(1.5, 0.0),
                        egui::vec2(0.0, -1.5),
                        egui::vec2(0.0, 1.5),
                    ] {
                        painter.text(
                            center + offset,
                            egui::Align2::CENTER_CENTER,
                            &current_lyric_text,
                            font_id.clone(),
                            shadow,
                        );
                    }
                    painter.text(
                        center,
                        egui::Align2::CENTER_CENTER,
                        &current_lyric_text,
                        font_id,
                        fg,
                    );
                });
            },
        );
    }

    fn refresh_lyrics_display(&mut self) {
        let (lyrics_received, should_show_panel) =
            self.lyrics_service.check_pending_lyrics(&mut self.ui_state);

        if lyrics_received {
            self.handle_lyrics_response(should_show_panel);
        }
    }

    fn refresh_library_command_processor(&mut self) {
        while let Ok(lib_cmd) = self.lib_cmd_rx().try_recv() {
            self.process_library_command(lib_cmd);
        }
    }

    fn refresh_youtube_download_processor(&mut self) {
        let mut events = Vec::new();
        while let Ok(event) = self.youtube_download_rx().try_recv() {
            events.push(event);
        }

        for event in events {
            self.handle_youtube_download_event(event);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0] // Fix macOS ghosting inside transparent windows
    }

    fn on_exit(&mut self, _ctx: Option<&eframe::glow::Context>) {
        tracing::info!("exiting and saving");
        self.update_player_persistence();
        self.save_state();
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        self.refresh_library_command_processor();
        self.refresh_youtube_download_processor();
        self.pump_audio_events();

        if self.ui_state.is_importing
            || self.ui_state.youtube_download_in_progress
            || self.ui_state.youtube_discover_in_progress
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.refresh_lyrics_display();
        self.refresh_window_title(ctx);
        self.show_desktop_lyrics(ctx);

        MainShell::show(self, ctx);
        PlaylistBookletComponent::show_window(self, ctx);

        // Request repaint during playback for smooth synced lyrics updates
        if self.runtime.is_some() {
            let player = self.player_ref();
            if matches!(player.track_state, crate::app::player::TrackState::Playing) {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
    }
}
