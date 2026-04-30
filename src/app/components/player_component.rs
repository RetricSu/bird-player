use super::cassette_component::CassetteComponent;
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
        ui.horizontal(|ui| {
            // Call cassette component with separate ctx reference
            CassetteComponent::add(ctx, ui);

            // Constrain middle pane width to make it a dense column
            let panel_width = tokens::size::PLAYER_PANEL;

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

                        // Title — large + bold so it reads as the page heading.
                        ui.add(egui::Label::new(
                            RichText::new(title).size(tokens::text::LG).strong(),
                        ));
                        // Artist — regular weight, slightly muted.
                        ui.add(egui::Label::new(RichText::new(artist).weak()));
                        // Combined small dim row: elapsed/total · playlist
                        ui.add(egui::Label::new(
                            RichText::new(format!(
                                "{} / {}  ·  {}",
                                format_time(seek_to_timestamp),
                                format_time(duration),
                                current_playlist_name,
                            ))
                            .size(tokens::text::SM)
                            .weak(),
                        ));
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

                    // Tightly packed controls
                    ui.add_space(tokens::spacing::XS);

                    // Timeline scrubber — thin rail by default, click+drag seeks.
                    ui.scope(|ui| {
                        ui.style_mut().spacing.slider_rail_height = 2.0;
                        let mut current_ms = seek_to_timestamp as f64;
                        let total_ms = (duration.max(1)) as f64;
                        let resp = ui.add_enabled(
                            has_selected_track && duration > 0,
                            egui::Slider::new(&mut current_ms, 0.0..=total_ms)
                                .show_value(false)
                                .handle_shape(egui::style::HandleShape::Circle),
                        );
                        if resp.dragged() && (current_ms as u64) != seek_to_timestamp {
                            PlayerService::seek_to(ctx.player_mut_ref(), current_ms as u64);
                        }
                    });

                    ui.add_space(tokens::spacing::XS);

                    // Row 1: Playback Controls & Volume
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
                        // Highlight the play button while audio is actually playing.
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
                        // Any mode other than Normal counts as "active" — give it the brand fill.
                        let mode_active =
                            !matches!(playback_mode, crate::app::player::PlaybackMode::Normal);
                        let mode_btn = ui
                            .add_enabled(has_selected_track, player_button(mode_icon, mode_active));

                        ui.add_space(tokens::spacing::MD);
                        // Desktop-lyrics toggle — fills with the brand colour when enabled.
                        let lyrics_btn =
                            ui.add(player_button(icons::LYRICS_TOGGLE, desktop_lyrics_enabled));

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
                        let mute_icon = if is_muted {
                            icons::VOLUME_MUTE
                        } else {
                            icons::VOLUME
                        };
                        let mute_btn = ui.add(player_button(mute_icon, is_muted));
                        let mute_clicked = mute_btn.clicked();

                        let mut current_volume = volume;
                        let previous_vol = current_volume;
                        ui.style_mut().spacing.slider_width = tokens::size::SLIDER_VOLUME; // make it longer now that it has its own row
                        let volume_slider = ui.add(
                            eframe::egui::Slider::new(&mut current_volume, 0.0_f32..=1.0_f32)
                                .volume_style(),
                        );

                        if volume_slider.dragged() && current_volume != previous_vol {
                            // Manually moving the slider clears any sticky mute.
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
                },
            );
        });
    }
}
