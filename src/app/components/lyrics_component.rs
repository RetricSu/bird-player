use eframe::egui;

use super::AppComponent;
use crate::app::lyrics::{Lyrics, LyricsService};
use crate::app::{App, LyricsFetchState};

enum ManualUploadResult {
    Cancelled,
    Updated,
    Failed(String),
}

pub struct LyricsComponent;

impl AppComponent for LyricsComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Upload Lyrics…").clicked() {
                match Self::handle_manual_upload(ctx) {
                    ManualUploadResult::Updated => {
                        ctx.lyrics_fetch_state = LyricsFetchState::Loaded;
                    }
                    ManualUploadResult::Failed(message) => {
                        ctx.lyrics_fetch_state = LyricsFetchState::Failed(message);
                    }
                    ManualUploadResult::Cancelled => {}
                }
            }

            ui.add_space(12.0);
            Self::show_status(ui, ctx);
        });

        ui.add_space(6.0);
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            Self::show_body(ctx, ui);
        });
    }
}

impl LyricsComponent {
    fn show_status(ui: &mut egui::Ui, ctx: &App) {
        match &ctx.lyrics_fetch_state {
            LyricsFetchState::Loading => {
                ui.add(egui::Spinner::new().size(14.0));
                ui.label("Fetching lyrics…");
            }
            LyricsFetchState::Failed(message) => {
                ui.colored_label(
                    egui::Color32::from_rgb(230, 80, 80),
                    format!("Lyrics unavailable: {}", message),
                );
            }
            _ => {}
        }
    }

    fn show_body(ctx: &App, ui: &mut egui::Ui) {
        if let Some(lyrics) = &ctx.current_lyrics {
            // Show track info
            ui.label(format!("Lyrics：{}", &lyrics.track_name));
            ui.label(format!("Artist：{}", &lyrics.artist_name));
            if let Some(album) = &lyrics.album_name {
                ui.label(format!("From：{}", album));
            }

            // Show lyrics type indicator
            let lyrics_type = if !lyrics.lines.is_empty() {
                "Synced Lyrics"
            } else if lyrics.plain_lyrics.is_some() {
                "Plain Text Lyrics"
            } else if lyrics.instrumental {
                "Instrumental"
            } else {
                "No Lyrics"
            };
            ui.label(
                egui::RichText::new(lyrics_type)
                    .color(egui::Color32::from_rgb(100, 150, 255))
                    .italics(),
            );

            ui.separator();

            // Show lyrics
            if lyrics.instrumental {
                ui.add(egui::Label::new(
                    egui::RichText::new("♪ Instrumental ♪").italics(),
                ));
            } else if !lyrics.lines.is_empty() {
                // Show synced lyrics with current line highlighting
                Self::show_synced_lyrics(ui, lyrics, ctx);
            } else if let Some(plain_lyrics) = &lyrics.plain_lyrics {
                Self::show_plain_lyrics(ui, plain_lyrics);
            } else {
                ui.add(egui::Label::new(
                    egui::RichText::new("No lyrics available").italics(),
                ));
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new("No lyrics loaded").italics(),
                ));
                ui.add(egui::Label::new(
                    egui::RichText::new("Select a track to view lyrics").italics(),
                ));
            });
        }
    }

    fn show_synced_lyrics(ui: &mut eframe::egui::Ui, lyrics: &Lyrics, ctx: &App) {
        let current_time_ms = if let Some(player) = &ctx.player {
            player.seek_to_timestamp
        } else {
            0
        };

        for line in &lyrics.lines {
            let is_current_line =
                if let (Some(start), Some(end)) = (line.start_time_ms, line.end_time_ms) {
                    current_time_ms >= start && current_time_ms < end
                } else if let Some(start) = line.start_time_ms {
                    current_time_ms >= start
                } else {
                    false
                };

            let label = if line.text.trim().is_empty() {
                "♪".to_string()
            } else {
                line.text.clone()
            };

            let response = if is_current_line {
                // Highlight current line and scroll to it
                ui.add(egui::Label::new(
                    egui::RichText::new(label)
                        .color(egui::Color32::BLUE)
                        .size(14.0)
                        .strong(),
                ))
            } else {
                ui.add(egui::Label::new(egui::RichText::new(label)))
            };

            // Scroll to current line to keep it visible
            if is_current_line {
                response.scroll_to_me(Some(egui::Align::Center));
            }

            // Add some spacing between lines
            ui.add_space(4.0);
        }
    }

    fn show_plain_lyrics(ui: &mut eframe::egui::Ui, plain_lyrics: &str) {
        for line in plain_lyrics.lines() {
            if line.trim().is_empty() {
                ui.add_space(8.0);
            } else {
                ui.add(egui::Label::new(egui::RichText::new(line)));
            }
        }
    }

    fn handle_manual_upload(ctx: &mut App) -> ManualUploadResult {
        let Some(player) = ctx.player.as_ref() else {
            return ManualUploadResult::Failed("Player not available".to_string());
        };

        let Some(track) = &player.selected_track else {
            return ManualUploadResult::Failed(
                "Select a track before uploading lyrics".to_string(),
            );
        };

        let Some(path) = rfd::FileDialog::new()
            .add_filter("Lyrics", &["lrc", "txt", "lyric", "lyrics"])
            .pick_file()
        else {
            return ManualUploadResult::Cancelled;
        };

        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                tracing::error!("Failed to read lyrics file '{}': {}", path.display(), err);
                return ManualUploadResult::Failed(format!("Failed to read lyrics file: {}", err));
            }
        };

        let artist = track
            .artist()
            .unwrap_or_else(|| "Unknown Artist".to_string());
        let title = track.title().unwrap_or_else(|| "Unknown Title".to_string());
        let album = track.album();

        let mut manual_lyrics = Lyrics {
            id: 0,
            name: "Manual Upload".to_string(),
            track_name: title,
            artist_name: artist,
            album_name: album,
            duration: None,
            instrumental: false,
            plain_lyrics: None,
            synced_lyrics: None,
            lines: Vec::new(),
        };

        let parsed_lines = Lyrics::parse_synced_lyrics(&content);
        if !parsed_lines.is_empty() {
            manual_lyrics.synced_lyrics = Some(content);
            manual_lyrics.lines = parsed_lines;
        } else {
            manual_lyrics.plain_lyrics = Some(content);
        }

        ctx.current_lyrics = Some(manual_lyrics.clone());
        ctx.show_lyrics_panel = true;
        ctx.pending_lyrics_rx = None;

        match LyricsService::write_lyrics_to_file(track.path(), &manual_lyrics) {
            Ok(_) => ManualUploadResult::Updated,
            Err(err) => {
                tracing::error!(
                    "Failed to write manual lyrics to ID3 tag for '{}': {}",
                    track.path().display(),
                    err
                );
                ManualUploadResult::Failed(format!(
                    "Lyrics loaded, but failed to write to audio file: {}",
                    err
                ))
            }
        }
    }
}
