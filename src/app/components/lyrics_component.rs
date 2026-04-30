use eframe::egui;

use super::AppComponent;
use crate::app::libstate::lyrics_state::LyricsFetchState;
use crate::app::lyrics::{Lyrics, LyricsService};
use crate::app::style::{icons, tokens};
use crate::app::{t, App};

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
            ui.set_min_height(tokens::size::HEADER_HEIGHT);
            Self::show_lyrics_type(ui, ctx);
            Self::show_lyrics_header(ui, ctx);
            Self::show_status(ui, ctx);
        });

        ui.add_space(tokens::spacing::XS);
        ui.separator();

        // auto_shrink([false, false]) lets the scroll area span the full
        // panel width so the vertical scrollbar sits flush against the
        // panel's right edge instead of floating in the middle.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Self::show_body(ctx, ui);
            });
    }
}

impl LyricsComponent {
    fn show_lyrics_header(ui: &mut egui::Ui, ctx: &mut App) {
        // Show track info
        if let Some(lyrics) = ctx.lyrics_manager().current_lyrics() {
            ui.label(
                egui::RichText::new(format!("{} — {}", &lyrics.track_name, &lyrics.artist_name))
                    .size(tokens::text::MD)
                    .strong(),
            );
        }

        let upload_clicked = ui
            .scope(|ui| {
                crate::app::style::borderless_button_visuals(ui.visuals_mut());
                ui.button(t("upload_lyrics"))
            })
            .inner
            .clicked();
        if upload_clicked {
            match Self::handle_manual_upload(ctx) {
                ManualUploadResult::Updated => {
                    ctx.ui_state.lyrics_fetch_state = LyricsFetchState::Loaded;
                }
                ManualUploadResult::Failed(message) => {
                    ctx.ui_state.lyrics_fetch_state = LyricsFetchState::Failed(message);
                }
                ManualUploadResult::Cancelled => {}
            }
        }
    }

    fn show_lyrics_type(ui: &mut egui::Ui, ctx: &App) {
        if let Some(lyrics) = ctx.lyrics_manager().current_lyrics() {
            // Show lyrics type indicator
            let lyrics_type = if !lyrics.lines.is_empty() {
                icons::LYRICS_SYNCED
            } else if lyrics.plain_lyrics.is_some() {
                icons::LYRICS_PLAIN
            } else if lyrics.instrumental {
                icons::LYRICS_INSTRUMENTAL
            } else {
                icons::LYRICS_NONE
            };
            ui.label(
                egui::RichText::new(lyrics_type)
                    .color(tokens::color::LYRICS_TYPE_ICON)
                    .italics(),
            );
        }
    }

    fn show_status(ui: &mut egui::Ui, ctx: &App) {
        match &ctx.ui_state.lyrics_fetch_state {
            LyricsFetchState::Loading => {
                ui.add(egui::Spinner::new().size(14.0));
                ui.label("Fetching lyrics…");
            }
            LyricsFetchState::Failed(message) => {
                ui.colored_label(
                    tokens::color::LYRICS_FAILED,
                    format!("Lyrics unavailable: {}", message),
                );
            }
            _ => {}
        }
    }

    fn show_body(ctx: &App, ui: &mut egui::Ui) {
        if let Some(lyrics) = ctx.lyrics_manager().current_lyrics() {
            // Show lyrics
            if lyrics.instrumental {
                ui.add(egui::Label::new(
                    egui::RichText::new("♪ Instrumental ♪")
                        .italics()
                        .size(tokens::text::SM),
                ));
            } else if !lyrics.lines.is_empty() {
                // Show synced lyrics with current line highlighting
                Self::show_synced_lyrics(ui, lyrics, ctx);
            } else if let Some(plain_lyrics) = &lyrics.plain_lyrics {
                Self::show_plain_lyrics(ui, plain_lyrics);
            } else {
                ui.add(egui::Label::new(
                    egui::RichText::new("No lyrics available")
                        .italics()
                        .size(tokens::text::SM),
                ));
            }
        } else {
            ui.vertical_centered(|ui| {
                ui.add(egui::Label::new(
                    egui::RichText::new("No lyrics loaded")
                        .italics()
                        .size(tokens::text::SM),
                ));
                ui.add(egui::Label::new(
                    egui::RichText::new("Select a track to view lyrics")
                        .italics()
                        .size(tokens::text::SM),
                ));
            });
        }
    }

    fn show_synced_lyrics(ui: &mut eframe::egui::Ui, lyrics: &Lyrics, ctx: &App) {
        let current_time_ms = if ctx.runtime.is_some() {
            ctx.player_ref().seek_to_timestamp
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
                        .color(tokens::color::LYRICS_CURRENT_LINE)
                        .size(tokens::text::SM)
                        .strong(),
                ))
            } else {
                ui.add(egui::Label::new(
                    egui::RichText::new(label).size(tokens::text::SM),
                ))
            };

            // Scroll to current line to keep it visible
            if is_current_line {
                response.scroll_to_me(Some(egui::Align::Center));
            }

            // Add some spacing between lines
            ui.add_space(tokens::spacing::SM);
        }
    }

    fn show_plain_lyrics(ui: &mut eframe::egui::Ui, plain_lyrics: &str) {
        for line in plain_lyrics.lines() {
            if line.trim().is_empty() {
                ui.add_space(tokens::spacing::MD);
            } else {
                ui.add(egui::Label::new(
                    egui::RichText::new(line).size(tokens::text::SM),
                ));
            }
        }
    }

    fn handle_manual_upload(ctx: &mut App) -> ManualUploadResult {
        if ctx.runtime.is_none() {
            return ManualUploadResult::Failed("Player not available".to_string());
        }

        let (track_key, track_path, artist, title, album) = {
            let player = ctx.player_ref();
            let Some(track) = &player.selected_track else {
                return ManualUploadResult::Failed(
                    "Select a track before uploading lyrics".to_string(),
                );
            };

            (
                track.key(),
                track.path(),
                track
                    .artist()
                    .unwrap_or_else(|| "Unknown Artist".to_string()),
                track.title().unwrap_or_else(|| "Unknown Title".to_string()),
                track.album(),
            )
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

        ctx.lyrics_manager_mut()
            .set_current_lyrics(Some(manual_lyrics.clone()));
        ctx.ui_state.show_lyrics_panel = true;

        match LyricsService::write_lyrics_to_file(&track_path, &manual_lyrics) {
            Ok(_) => {
                let lyrics_text = manual_lyrics
                    .synced_lyrics
                    .as_deref()
                    .or(manual_lyrics.plain_lyrics.as_deref());
                ctx.update_track_lyrics(track_key, lyrics_text);
                ManualUploadResult::Updated
            }
            Err(err) => {
                tracing::error!(
                    "Failed to write manual lyrics to ID3 tag for '{}': {}",
                    track_path.display(),
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
