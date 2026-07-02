use super::AppComponent;
use crate::app::style::{icons, player_button, tokens, ButtonExt};
use crate::app::t;
use crate::app::App;
use eframe::egui::{self, Align, Layout, RichText, TextEdit};

const DESCRIPTION_PREVIEW_LENGTH: usize = 30;

pub struct PlaybackInfoPanel;

impl AppComponent for PlaybackInfoPanel {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        Self::render_download_entry(ctx, ui);

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
    fn render_download_entry(ctx: &mut App, ui: &mut egui::Ui) {
        let active =
            ctx.ui_state.show_youtube_download_dialog || ctx.ui_state.youtube_download_in_progress;
        let button = ui
            .add(player_button(icons::DOWNLOAD, active))
            .on_hover_text(t("download_authorized_audio"));

        if button.clicked() {
            ctx.ui_state.show_youtube_download_dialog = true;
        }

        Self::render_download_dialog(ctx, ui.ctx().clone());
    }

    fn render_download_dialog(ctx: &mut App, egui_ctx: egui::Context) {
        if !ctx.ui_state.show_youtube_download_dialog {
            return;
        }

        let mut open = ctx.ui_state.show_youtube_download_dialog;
        egui::Window::new(t("download_authorized_audio"))
            .id(egui::Id::new("youtube_download_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(&egui_ctx, |ui| {
                ui.set_min_width(420.0);
                ui.label(RichText::new(t("authorized_audio_notice")).weak());
                ui.add_space(tokens::spacing::XS);

                ui.label(t("youtube_url"));
                ui.add_enabled(
                    !ctx.ui_state.youtube_download_in_progress,
                    TextEdit::singleline(&mut ctx.ui_state.youtube_download_url)
                        .hint_text("https://www.youtube.com/watch?v=..."),
                );

                ui.add_space(tokens::spacing::XS);
                ui.add_enabled_ui(!ctx.ui_state.youtube_download_in_progress, |ui| {
                    ui.checkbox(
                        &mut ctx.ui_state.youtube_download_include_playlist,
                        t("download_entire_playlist"),
                    )
                    .on_hover_text(t("download_entire_playlist_hint"));
                });

                ui.add_space(tokens::spacing::XS);
                ui.label(t("save_to"));
                ui.horizontal(|ui| {
                    let mut path_text = ctx.ui_state.youtube_download_dir.display().to_string();
                    ui.add_enabled(
                        false,
                        TextEdit::singleline(&mut path_text).desired_width(340.0),
                    );

                    let choose = ui
                        .add_enabled(
                            !ctx.ui_state.youtube_download_in_progress,
                            egui::Button::new(icons::FOLDER).player_style(),
                        )
                        .on_hover_text(t("choose_download_folder"));
                    if choose.clicked() {
                        if let Some(folder) = rfd::FileDialog::new()
                            .set_directory(&ctx.ui_state.youtube_download_dir)
                            .pick_folder()
                        {
                            ctx.ui_state.youtube_download_dir = folder;
                        }
                    }
                });

                if let Some(status) = &ctx.ui_state.youtube_download_status {
                    ui.add_space(tokens::spacing::XS);
                    ui.label(RichText::new(status).weak());
                }

                if let Some(progress) = ctx.ui_state.youtube_download_progress {
                    ui.add_space(tokens::spacing::XS);
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .show_percentage()
                            .desired_width(ui.available_width()),
                    );
                } else if ctx.ui_state.youtube_download_in_progress {
                    ui.add_space(tokens::spacing::XS);
                    ui.add(
                        egui::ProgressBar::new(0.0)
                            .animate(true)
                            .desired_width(ui.available_width()),
                    );
                }

                ui.add_space(tokens::spacing::SM);
                ui.horizontal(|ui| {
                    let can_download = !ctx.ui_state.youtube_download_in_progress
                        && !ctx.ui_state.youtube_download_url.trim().is_empty();
                    if ui
                        .add_enabled(can_download, egui::Button::new(t("download")))
                        .clicked()
                    {
                        ctx.start_youtube_download();
                    }

                    if ui
                        .add_enabled(
                            !ctx.ui_state.youtube_download_in_progress,
                            egui::Button::new(t("clear")),
                        )
                        .clicked()
                    {
                        ctx.ui_state.youtube_download_url.clear();
                        ctx.ui_state.youtube_download_progress = None;
                        ctx.ui_state.youtube_download_last_file_count = None;
                        ctx.ui_state.youtube_download_resync_in_progress = false;
                        ctx.ui_state.youtube_download_status = None;
                    }
                });
            });

        ctx.ui_state.show_youtube_download_dialog = open;
    }

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
                    let preview = if desc.chars().count() > DESCRIPTION_PREVIEW_LENGTH {
                        format!(
                            "{}...",
                            desc.chars()
                                .take(DESCRIPTION_PREVIEW_LENGTH)
                                .collect::<String>()
                        )
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
