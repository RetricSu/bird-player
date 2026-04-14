use std::time::Duration;

use eframe::egui;

use super::App;
use crate::app::components::main_shell::MainShell;
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
}

impl eframe::App for App {
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

        if self.ui_state.is_importing {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.refresh_lyrics_display();
        self.refresh_window_title(ctx);

        MainShell::show(self, ctx);

        // Request repaint during playback for smooth synced lyrics updates
        if self.runtime.is_some() {
            let player = self.player_ref();
            if matches!(player.track_state, crate::app::player::TrackState::Playing) {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }

        eprintln!("fps = {:.0}", 1.0 / ctx.input(|i| i.unstable_dt));
    }
}
