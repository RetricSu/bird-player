use super::cassette_component::CassetteComponent;
use super::AppComponent;
use crate::app::services::PlayerService;
use crate::app::style::{ButtonExt, SliderExt};
use crate::app::t;
use crate::app::App;
use eframe::egui::{self, vec2};

pub struct PlayerComponent;

struct SelectedTrackSummary {
    title: Option<String>,
    artist: Option<String>,
}

/// User-driven actions that can be triggered from the player control row.
/// Using an enum (rather than string keys) keeps the UI dispatch type-checked
/// and makes adding new controls a compile-time concern.
enum PlayerAction {
    TogglePlayPause,
    Previous,
    Next,
    ToggleMode,
    ToggleDesktopLyrics,
}

impl AppComponent for PlayerComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Check if runtime is initialized
        if ctx.runtime.is_none() {
            ui.centered_and_justified(|ui| {
                ui.heading("Player not initialized");
            });
            return;
        }

        // NOTE: Audio events are pumped centrally in `App::update` via
        // `pump_audio_events`, so this component is now purely a renderer.

        // Then collect all necessary data (不可变借用)
        let (
            selected_track,
            is_playing,
            playback_mode,
            seek_to_timestamp,
            duration,
            volume,
            current_playlist_name,
        ) = {
            let player = ctx.player_ref();
            let selected_track = player
                .selected_track
                .as_ref()
                .map(|track| SelectedTrackSummary {
                    title: track.title(),
                    artist: track.artist(),
                });
            let is_playing = PlayerService::is_playing(player);
            let playback_mode = PlayerService::get_playback_mode(player);
            let seek_to_timestamp = PlayerService::get_seek_timestamp(player);
            let duration = PlayerService::get_duration(player);
            let volume = PlayerService::get_volume(player);

            let current_playlist_name = ctx
                .app_settings
                .playing_playlist_idx
                .and_then(|idx| ctx.playlists.get(idx))
                .and_then(|playlist| playlist.get_name())
                .unwrap_or_default();

            (
                selected_track,
                is_playing,
                playback_mode,
                seek_to_timestamp,
                duration,
                volume,
                current_playlist_name,
            )
        };

        let has_selected_track = selected_track.is_some();

        // Get playlist tracks info for the current playlist
        let current_playlist_idx = ctx.app_settings.current_playlist_idx;
        // Use is_some_and instead of map_or
        let has_tracks_in_playlist =
            current_playlist_idx.is_some_and(|idx| !ctx.playlists[idx].tracks.is_empty());

        // Now render UI without borrowing ctx in closures that also borrow ctx
        ui.horizontal(|ui| {
            // Call cassette component with separate ctx reference
            CassetteComponent::add(ctx, ui);

            // Constrain middle pane width to make it a dense column
            let panel_width = 320.0;

            ui.allocate_ui_with_layout(
                vec2(panel_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    // Tightly pack the track information (2000s Web high-density style)
                    if let Some(track) = &selected_track {
                        let title = track.title.as_deref().unwrap_or("unknown title");
                        let artist = track.artist.as_deref().unwrap_or("unknown artist");

                        let format_time = |timestamp: u64| -> String {
                            let total_seconds = timestamp / 1000;
                            let minutes = total_seconds / 60;
                            let seconds = total_seconds % 60;
                            format!("{:02}:{:02}", minutes, seconds)
                        };

                        ui.label(
                            eframe::egui::RichText::new(format!("{}{}", t("song"), title)).strong(),
                        );
                        ui.label(format!("{}{}", t("artist"), artist));
                        ui.label(format!(
                            "{} / {}",
                            format_time(seek_to_timestamp),
                            format_time(duration)
                        ));
                        ui.label(format!("{}{}", t("playlist"), current_playlist_name));
                    } else {
                        ui.label(eframe::egui::RichText::new(t("no_track")).strong());
                        if has_tracks_in_playlist {
                            ui.label(t("select_track"));
                        } else if current_playlist_idx.is_some() {
                            ui.label(t("add_tracks"));
                        } else {
                            ui.label(t("create_playlist"));
                        }
                    }

                    // Tightly packed controls
                    ui.add_space(2.0);
                    ui.separator();
                    ui.add_space(2.0);

                    // Row 1: Playback Controls & Volume
                    ui.horizontal(|ui| {
                        let prev_btn = ui.add_enabled(
                            has_selected_track,
                            egui::Button::new("|◀").player_style(),
                        );
                        let play_pause_icon = if is_playing { "⏸" } else { "▶" };
                        let play_pause_btn = ui.add_enabled(
                            has_selected_track,
                            egui::Button::new(play_pause_icon).player_style(),
                        );
                        let next_btn = ui.add_enabled(
                            has_selected_track,
                            egui::Button::new("▶|").player_style(),
                        );

                        let mode_icon = match playback_mode {
                            crate::app::player::PlaybackMode::Normal => "➡",
                            crate::app::player::PlaybackMode::Repeat => "🔁",
                            crate::app::player::PlaybackMode::RepeatOne => "🔂",
                            crate::app::player::PlaybackMode::Shuffle => "🔀",
                        };
                        let mode_btn = ui.add_enabled(
                            has_selected_track,
                            egui::Button::new(mode_icon).player_style(),
                        );

                        ui.add_space(8.0);
                        // TODO(ui-phase): differentiate icon / colour by `desktop_lyrics_enabled`
                        let lyrics_btn = ui.add(egui::Button::new("词").player_style());

                        // Translate UI clicks into a single, type-checked action
                        let action: Option<PlayerAction> =
                            if mode_btn.clicked() && has_selected_track {
                                Some(PlayerAction::ToggleMode)
                            } else if play_pause_btn.clicked() && has_selected_track {
                                Some(PlayerAction::TogglePlayPause)
                            } else if prev_btn.clicked()
                                && has_selected_track
                                && ctx.app_settings.playing_playlist_idx.is_some()
                            {
                                Some(PlayerAction::Previous)
                            } else if next_btn.clicked()
                                && has_selected_track
                                && ctx.app_settings.playing_playlist_idx.is_some()
                            {
                                Some(PlayerAction::Next)
                            } else if lyrics_btn.clicked() {
                                Some(PlayerAction::ToggleDesktopLyrics)
                            } else {
                                None
                            };

                        if let Some(action) = action {
                            let mut fetch_lyrics = false;
                            match action {
                                PlayerAction::ToggleMode => {
                                    PlayerService::toggle_playback_mode(ctx.player_mut_ref());
                                }
                                PlayerAction::TogglePlayPause => {
                                    if is_playing {
                                        PlayerService::pause(ctx.player_mut_ref());
                                    } else {
                                        PlayerService::play(ctx.player_mut_ref());
                                    }
                                }
                                PlayerAction::Previous => {
                                    ctx.play_previous_track();
                                    fetch_lyrics = true;
                                }
                                PlayerAction::Next => {
                                    ctx.play_next_track();
                                    fetch_lyrics = true;
                                }
                                PlayerAction::ToggleDesktopLyrics => {
                                    ctx.ui_state.desktop_lyrics_enabled =
                                        !ctx.ui_state.desktop_lyrics_enabled;
                                }
                            }
                            if fetch_lyrics {
                                ctx.fetch_lyrics_for_current_track();
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("📢");
                        let mut current_volume = volume;
                        let previous_vol = current_volume;
                        ui.style_mut().spacing.slider_width = 160.0; // make it longer now that it has its own row
                        let volume_slider = ui.add(
                            eframe::egui::Slider::new(&mut current_volume, 0.0_f32..=1.0_f32)
                                .volume_style(),
                        );

                        if volume_slider.dragged() && current_volume != previous_vol {
                            let is_processing_ui_change = ctx.is_processing_ui_change();
                            PlayerService::set_volume(
                                ctx.player_mut_ref(),
                                current_volume,
                                &is_processing_ui_change,
                            );
                        }
                    });
                },
            );

            ui.separator();

            // Right Column: Lyrics inside the player panel
            ui.allocate_ui_with_layout(
                eframe::egui::vec2(ui.available_width(), ui.available_height()),
                eframe::egui::Layout::top_down(eframe::egui::Align::LEFT),
                |ui| {
                    crate::app::components::lyrics_component::LyricsComponent::add(ctx, ui);
                },
            );
        });
    }
}
