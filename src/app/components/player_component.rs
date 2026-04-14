use super::cassette_component::CassetteComponent;
use super::AppComponent;
use crate::app::services::PlayerService;
use crate::app::style::{ButtonExt, SliderExt};
use crate::app::t;
use crate::{app::App, app::AudioEvent};
use eframe::egui::{self, vec2};
use std::time::Instant;

pub struct PlayerComponent;

const CASSETTE_WIDTH: f32 = 200.0;

struct SelectedTrackSummary {
    title: Option<String>,
    artist: Option<String>,
}

// For periodic state saving
thread_local! {
    static LAST_SAVE: std::cell::RefCell<Instant> = std::cell::RefCell::new(Instant::now());
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

        // Process ALL pending UI commands first
        let ui_cmds: Vec<_> = {
            let player = ctx.player_mut_ref();
            let mut cmds = Vec::new();
            while let Ok(cmd) = player.ui_rx.try_recv() {
                cmds.push(cmd);
            }
            cmds
        };

        for new_seek_cmd in ui_cmds {
            match new_seek_cmd {
                AudioEvent::CurrentTimestamp(seek_timestamp) => {
                    // Check if we need to save
                    let should_save = LAST_SAVE.with(|last_save| {
                        let elapsed = last_save.borrow().elapsed().as_secs();
                        if elapsed > 30 {
                            *last_save.borrow_mut() = Instant::now();
                            true
                        } else {
                            false
                        }
                    });

                    if should_save {
                        ctx.update_player_persistence();
                        ctx.save_state();
                    }

                    let player = ctx.player_mut_ref();
                    PlayerService::set_seek_to_timestamp(player, seek_timestamp);
                }
                AudioEvent::TotalTrackDuration(dur) => {
                    tracing::info!("Received Duration: {}", dur);
                    let player = ctx.player_mut_ref();
                    PlayerService::set_duration(player, dur);
                }
                AudioEvent::AudioFinished => {
                    tracing::info!("Track finished, getting next...");
                    // Clone playlist before mutable borrow
                    let playlist_clone = ctx
                        .app_settings
                        .current_playlist_idx
                        .and_then(|idx| ctx.playlists.get(idx).cloned());

                    if let Some(playlist) = playlist_clone {
                        let player = ctx.player_mut_ref();
                        PlayerService::next_track(player, &playlist);
                    }
                    ctx.fetch_lyrics_for_current_track();
                }
                AudioEvent::PlaybackStateChanged(is_playing) => {
                    tracing::info!(
                        "Playback state changed to: {}",
                        if is_playing { "Playing" } else { "Paused" }
                    );
                    let player = ctx.player_mut_ref();
                    if is_playing {
                        PlayerService::play(player);
                    } else {
                        PlayerService::pause(player);
                    }
                }
            }
        }

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

            // Add minimum width constraint for the vertical layout
            let min_width = 200.0; // Minimum width in pixels
            let available_width = ui.available_width();
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

                        ui.label(eframe::egui::RichText::new(format!("{}{}", t("song"), title)).strong());
                        ui.label(format!("{}{}", t("artist"), artist));
                        ui.label(format!("{} / {}", format_time(seek_to_timestamp), format_time(duration)));
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
                        let prev_btn = ui.add_enabled(has_selected_track, egui::Button::new("|◀").player_style());
                        let play_pause_icon = if is_playing { "⏸" } else { "▶" };
                        let play_pause_btn = ui.add_enabled(has_selected_track, egui::Button::new(play_pause_icon).player_style());
                        let next_btn = ui.add_enabled(has_selected_track, egui::Button::new("▶|").player_style());

                        let mode_icon = match playback_mode {
                            crate::app::player::PlaybackMode::Normal => "➡",
                            crate::app::player::PlaybackMode::Repeat => "🔁",
                            crate::app::player::PlaybackMode::RepeatOne => "🔂",
                            crate::app::player::PlaybackMode::Shuffle => "🔀",
                        };
                        let mode_btn = ui.add_enabled(has_selected_track, egui::Button::new(mode_icon).player_style());

                        // Playback Action Logic
                        let mut fetch_lyrics = false;
                        if has_selected_track {
                            let mut action = None;
                            if mode_btn.clicked() {
                                action = Some("toggle_mode");
                            } else if play_pause_btn.clicked() {
                                action = Some(if is_playing { "pause" } else { "play" });
                            } else if prev_btn.clicked() && ctx.app_settings.playing_playlist_idx.is_some() {
                                action = Some("previous");
                                fetch_lyrics = true;
                            } else if next_btn.clicked() && ctx.app_settings.playing_playlist_idx.is_some() {
                                action = Some("next");
                                fetch_lyrics = true;
                            }

                            if let Some(action) = action {
                                match action {
                                    "toggle_mode" => PlayerService::toggle_playback_mode(ctx.player_mut_ref()),
                                    "pause" => PlayerService::pause(ctx.player_mut_ref()),
                                    "play"  => PlayerService::play(ctx.player_mut_ref()),
                                    "previous" => ctx.play_previous_track(),
                                    "next" => ctx.play_next_track(),
                                    _ => {}
                                }
                            }
                        }
                        if fetch_lyrics {
                            ctx.fetch_lyrics_for_current_track();
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("📢");
                        let mut current_volume = volume;
                        let previous_vol = current_volume;
                        ui.style_mut().spacing.slider_width = 160.0; // make it longer now that it has its own row
                        let volume_slider = ui.add(eframe::egui::Slider::new(&mut current_volume, 0.0_f32..=1.0_f32).volume_style());

                        if volume_slider.dragged() {
                            if current_volume != previous_vol {
                                let is_processing_ui_change = ctx.is_processing_ui_change();
                                PlayerService::set_volume(ctx.player_mut_ref(), current_volume, &is_processing_ui_change);
                            }
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
                }
            );
        });
    }
}
