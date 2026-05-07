use super::cassette_component::CassetteComponent;
use super::playback_info_panel::PlaybackInfoPanel;
use super::AppComponent;
use crate::app::services::PlayerService;
use crate::app::style::{icons, player_button, tokens, ButtonExt, SliderExt};
use crate::app::t;
use crate::app::App;
use eframe::egui::{self, vec2, RichText};

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
        let desktop_lyrics_enabled = ctx.ui_state.desktop_lyrics_enabled;
        let is_muted = ctx.ui_state.volume_before_mute.is_some();

        // Get playlist tracks info for the current playlist
        let current_playlist_idx = ctx.app_settings.current_playlist_idx;
        // Use is_some_and instead of map_or
        let has_tracks_in_playlist =
            current_playlist_idx.is_some_and(|idx| !ctx.playlists[idx].tracks.is_empty());

        // Now render UI without borrowing ctx in closures that also borrow ctx
        ui.vertical(|ui| {
            // ── Top row: cover + track info + playback info ────────────────
            ui.horizontal(|ui| {
                CassetteComponent::add(ctx, ui);

                // Calculate available width for middle section (60% of remaining space)
                let remaining_width = ui.available_width();
                let middle_width = remaining_width * 0.6;

                ui.allocate_ui_with_layout(
                    vec2(middle_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        if let Some(track) = &selected_track {
                            let title = track.title.as_deref().unwrap_or("unknown title");
                            let artist = track.artist.as_deref().unwrap_or("unknown artist");

                            let format_time = |timestamp: u64| -> String {
                                let total_seconds = timestamp / 1000;
                                let minutes = total_seconds / 60;
                                let seconds = total_seconds % 60;
                                format!("{:02}:{:02}", minutes, seconds)
                            };

                            // Title — large + bold, truncated so a long song name
                            // can never push the rest of the layout down.
                            ui.add(
                                egui::Label::new(
                                    RichText::new(title).size(tokens::text::LG).strong(),
                                )
                                .truncate(),
                            );
                            ui.add(egui::Label::new(RichText::new(artist).weak()).truncate());
                            // Time on its own line so a long playlist name
                            // never collides with the elapsed/duration counter.
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!(
                                        "{} / {}",
                                        format_time(seek_to_timestamp),
                                        format_time(duration),
                                    ))
                                    .size(tokens::text::SM)
                                    .weak(),
                                )
                                .truncate(),
                            );
                            if !current_playlist_name.is_empty() {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "{}: {}",
                                            t("playlist_label"),
                                            current_playlist_name,
                                        ))
                                        .size(tokens::text::SM)
                                        .weak(),
                                    )
                                    .truncate(),
                                );
                            }
                        } else {
                            ui.add(egui::Label::new(
                                RichText::new(t("no_track")).size(tokens::text::LG).strong(),
                            ));
                            let hint = if has_tracks_in_playlist {
                                t("select_track")
                            } else if current_playlist_idx.is_some() {
                                t("add_tracks")
                            } else {
                                t("create_playlist")
                            };
                            ui.add(egui::Label::new(RichText::new(hint).weak()));
                        }
                    },
                );

                // Add playback info panel on the right
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width(), ui.available_height()),
                    egui::Layout::top_down(egui::Align::RIGHT),
                    |ui| {
                        PlaybackInfoPanel::add(ctx, ui);
                    },
                );
            });

            // ── Middle row: full-width timeline scrubber ────────────────────
            ui.scope(|ui| {
                ui.style_mut().spacing.slider_rail_height = 2.0;
                // Stretch the slider across the entire player panel.
                ui.style_mut().spacing.slider_width = ui.available_width() - tokens::spacing::MD;
                let mut current_ms = seek_to_timestamp as f64;
                let total_ms = (duration.max(1)) as f64;
                let resp = ui.add_enabled(
                    has_selected_track && duration > 0,
                    egui::Slider::new(&mut current_ms, 0.0..=total_ms)
                        .show_value(false)
                        .handle_shape(egui::style::HandleShape::Circle),
                );
                // Use `.changed()` so click-on-rail (not just drag) seeks too.
                if resp.changed() && (current_ms as u64) != seek_to_timestamp {
                    PlayerService::seek_to(ctx.player_mut_ref(), current_ms as u64);
                }
            });

            // ── Bottom row: transport (left) + volume (right) ───────────────
            ui.horizontal(|ui| {
                let prev_btn = ui.add_enabled(
                    has_selected_track,
                    egui::Button::new(icons::PREV).player_style(),
                );
                let play_pause_icon = if is_playing {
                    icons::PAUSE
                } else {
                    icons::PLAY
                };
                let play_pause_btn = ui.add_enabled(
                    has_selected_track,
                    player_button(play_pause_icon, is_playing && has_selected_track),
                );
                let next_btn = ui.add_enabled(
                    has_selected_track,
                    egui::Button::new(icons::NEXT).player_style(),
                );

                let mode_icon = match playback_mode {
                    crate::app::player::PlaybackMode::Normal => icons::MODE_NORMAL,
                    crate::app::player::PlaybackMode::Repeat => icons::MODE_REPEAT,
                    crate::app::player::PlaybackMode::RepeatOne => icons::MODE_REPEAT_ONE,
                    crate::app::player::PlaybackMode::Shuffle => icons::MODE_SHUFFLE,
                };
                let mode_active =
                    !matches!(playback_mode, crate::app::player::PlaybackMode::Normal);
                let mode_btn =
                    ui.add_enabled(has_selected_track, player_button(mode_icon, mode_active));

                ui.add_space(tokens::spacing::SM);
                let lyrics_btn =
                    ui.add(player_button(icons::LYRICS_TOGGLE, desktop_lyrics_enabled));

                // Volume hugs the right edge.
                let mute_clicked;
                let volume_changed;
                let mut current_volume = volume;
                let previous_vol = current_volume;
                {
                    let resp =
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.style_mut().spacing.slider_width = tokens::size::SLIDER_VOLUME;
                            let slider = ui.add(
                                eframe::egui::Slider::new(&mut current_volume, 0.0_f32..=1.0_f32)
                                    .volume_style(),
                            );
                            let mute_icon = if is_muted {
                                icons::VOLUME_MUTE
                            } else {
                                icons::VOLUME
                            };
                            let mute = ui.add(player_button(mute_icon, is_muted));
                            (mute.clicked(), slider.dragged() || slider.changed())
                        });
                    (mute_clicked, volume_changed) = resp.inner;
                }

                // Translate transport clicks into a single action.
                let action: Option<PlayerAction> = if mode_btn.clicked() && has_selected_track {
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

                if volume_changed && current_volume != previous_vol {
                    ctx.ui_state.volume_before_mute = None;
                    let is_processing_ui_change = ctx.is_processing_ui_change();
                    PlayerService::set_volume(
                        ctx.player_mut_ref(),
                        current_volume,
                        &is_processing_ui_change,
                    );
                }
                if mute_clicked {
                    let is_processing_ui_change = ctx.is_processing_ui_change();
                    if let Some(prev) = ctx.ui_state.volume_before_mute.take() {
                        PlayerService::set_volume(
                            ctx.player_mut_ref(),
                            prev,
                            &is_processing_ui_change,
                        );
                    } else {
                        ctx.ui_state.volume_before_mute = Some(volume);
                        PlayerService::set_volume(
                            ctx.player_mut_ref(),
                            0.0,
                            &is_processing_ui_change,
                        );
                    }
                }
            });
        });
    }
}
