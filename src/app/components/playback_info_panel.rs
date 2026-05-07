use super::AppComponent;
use crate::app::style::tokens;
use crate::app::App;
use eframe::egui::{self, Align, Layout, RichText};

pub struct PlaybackInfoPanel;

impl AppComponent for PlaybackInfoPanel {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Check if we have a playing playlist
        let playing_playlist_idx = ctx.app_settings.playing_playlist_idx;

        if let Some(playlist_idx) = playing_playlist_idx {
            // Case 1: Playlist is playing
            Self::render_playlist_info(ctx, ui, playlist_idx);
        } else {
            // Case 2: Single track (no playlist)
            Self::render_track_info(ctx, ui);
        }
    }
}

impl PlaybackInfoPanel {
    fn render_playlist_info(ctx: &App, ui: &mut egui::Ui, playlist_idx: usize) {
        if let Some(playlist) = ctx.playlists.get(playlist_idx) {
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                let weak_color = ui.visuals().weak_text_color();

                // Playlist name
                if let Some(name) = playlist.get_name() {
                    ui.label(
                        RichText::new(&name)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }

                // Track position
                if let Some(selected_track) = &ctx.player_ref().selected_track {
                    if let Some(pos) = playlist.get_pos(selected_track) {
                        let total = playlist.tracks.len();
                        ui.label(
                            RichText::new(format!("Track {:02}/{:02}", pos + 1, total))
                                .size(tokens::text::SM)
                                .color(weak_color),
                        );
                    }
                }

                // Description preview (first 30 chars)
                if let Some(desc) = playlist.description() {
                    let preview = if desc.len() > 30 {
                        format!("{}...", &desc[..30])
                    } else {
                        desc.to_string()
                    };
                    ui.label(
                        RichText::new(preview)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
            });
        }
    }

    fn render_track_info(ctx: &App, ui: &mut egui::Ui) {
        let player = ctx.player_ref();

        if let Some(track) = &player.selected_track {
            ui.with_layout(Layout::top_down(Align::RIGHT), |ui| {
                let weak_color = ui.visuals().weak_text_color();

                // Album · Year
                let mut album_line = String::new();
                if let Some(album) = track.album() {
                    album_line.push_str(&album);
                }
                if let Some(year) = track.year() {
                    if !album_line.is_empty() {
                        album_line.push_str(" · ");
                    }
                    album_line.push_str(&year.to_string());
                }
                if !album_line.is_empty() {
                    ui.label(
                        RichText::new(album_line)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }

                // Genre
                if let Some(genre) = track.genre() {
                    ui.label(
                        RichText::new(genre)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }

                // Format · Sample Rate · Channels
                let mut tech_line = String::new();
                if let Some(codec) = &player.codec {
                    tech_line.push_str(codec);
                }
                if let Some(sample_rate) = player.sample_rate {
                    if !tech_line.is_empty() {
                        tech_line.push_str(" · ");
                    }
                    tech_line.push_str(&format!("{:.1}kHz", sample_rate as f32 / 1000.0));
                }
                if let Some(channels) = player.channels {
                    if !tech_line.is_empty() {
                        tech_line.push_str(" · ");
                    }
                    let channel_str = match channels {
                        1 => "Mono",
                        2 => "Stereo",
                        _ => {
                            tech_line.push_str(&format!("{}ch", channels));
                            ""
                        }
                    };
                    if !channel_str.is_empty() {
                        tech_line.push_str(channel_str);
                    }
                }
                if !tech_line.is_empty() {
                    ui.label(
                        RichText::new(tech_line)
                            .size(tokens::text::SM)
                            .color(weak_color),
                    );
                }
            });
        }
    }
}
