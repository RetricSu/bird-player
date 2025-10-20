use eframe::egui;

use super::AppComponent;
use crate::app::lyrics::Lyrics;
use crate::app::App;

pub struct LyricsComponent;

impl AppComponent for LyricsComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            if let Some(lyrics) = &ctx.current_lyrics {
                // Show track info
                ui.label(format!("Lyrics：{}", &lyrics.track_name));
                ui.label(format!("Artist：{}", &lyrics.artist_name));
                if let Some(album) = &lyrics.album_name {
                    ui.label(format!("From：{}", album));
                }
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
        });
    }
}

impl LyricsComponent {
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

            if is_current_line {
                // Highlight current line
                ui.add(egui::Label::new(
                    egui::RichText::new(label)
                        .color(egui::Color32::BLUE)
                        .size(16.0)
                        .strong(),
                ));
            } else {
                ui.add(egui::Label::new(egui::RichText::new(label)));
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
}
