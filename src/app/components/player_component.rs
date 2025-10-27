use eframe::egui::{self, vec2};
use std::time::Instant;

use super::cassette_component::CassetteComponent;
use super::AppComponent;
use crate::app::style::{ButtonExt, SliderExt};
use crate::app::t;
use crate::egui::style::HandleShape;
use crate::{app::App, UiCommand};

pub struct PlayerComponent;

const CASSETTE_WIDTH: f32 = 280.0;

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

        // Process UI commands first
        let ui_cmd = {
            let player = ctx.player_mut_ref();
            player.ui_rx.try_recv().ok()
        };

        if let Some(new_seek_cmd) = ui_cmd {
            match new_seek_cmd {
                UiCommand::CurrentTimestamp(seek_timestamp) => {
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
                    player.set_seek_to_timestamp(seek_timestamp);
                }
                UiCommand::TotalTrackDuration(dur) => {
                    tracing::info!("Received Duration: {}", dur);
                    let player = ctx.player_mut_ref();
                    player.set_duration(dur);
                }
                UiCommand::AudioFinished => {
                    tracing::info!("Track finished, getting next...");
                    // Clone playlist before mutable borrow
                    let playlist_clone = ctx
                        .current_playlist_idx
                        .and_then(|idx| ctx.playlists.get(idx).cloned());

                    if let Some(playlist) = playlist_clone {
                        let player = ctx.player_mut_ref();
                        player.next(&playlist);
                    }
                    ctx.fetch_lyrics_for_current_track();
                }
                UiCommand::PlaybackStateChanged(is_playing) => {
                    tracing::info!(
                        "Playback state changed to: {}",
                        if is_playing { "Playing" } else { "Paused" }
                    );
                    let player = ctx.player_mut_ref();
                    if is_playing {
                        player.track_state = crate::app::player::TrackState::Playing;
                    } else {
                        player.track_state = crate::app::player::TrackState::Paused;
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
            let selected_track = player.selected_track.clone();
            let is_playing = matches!(player.track_state, crate::app::player::TrackState::Playing);
            let playback_mode = player.playback_mode;
            let seek_to_timestamp = player.seek_to_timestamp;
            let duration = player.duration;
            let volume = player.volume;

            // Get current playlist name using map_or for cleaner code
            let current_playlist_name = ctx
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
        let current_playlist_idx = ctx.current_playlist_idx;
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
            let panel_width = if available_width > CASSETTE_WIDTH {
                available_width
            } else {
                min_width
            };

            ui.allocate_ui_with_layout(
                vec2(panel_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.add_space(10.0); // Add margin at the top

                    // Show track info if selected, otherwise show default message
                    if let Some(track) = &selected_track {
                        ui.add(
                            eframe::egui::Label::new(format!(
                                "{}{}",
                                t("song"),
                                track.title().unwrap_or("unknown title".to_string())
                            ))
                            .wrap_mode(eframe::egui::TextWrapMode::Truncate),
                        )
                        .highlight();

                        ui.label(format!(
                            "{}{}",
                            t("artist"),
                            track.artist().unwrap_or("unknown artist".to_string())
                        ));

                        ui.label(format!("{}{}", t("playlist"), current_playlist_name));
                    } else {
                        // Default display when no track is selected
                        ui.add(
                            eframe::egui::Label::new(t("no_track"))
                                .wrap_mode(eframe::egui::TextWrapMode::Truncate),
                        );

                        if has_tracks_in_playlist {
                            ui.label(t("select_track"));
                        } else if current_playlist_idx.is_some() {
                            ui.label(t("add_tracks"));
                        } else {
                            ui.label(t("create_playlist"));
                        }
                    }

                    // Add space to push controls to bottom
                    ui.add_space(ui.available_height() - 70.0);

                    // Time Slider
                    ui.horizontal(|ui| {
                        let format_time = |timestamp: u64| -> String {
                            let total_seconds = timestamp / 1000;
                            let minutes = total_seconds / 60;
                            let seconds = total_seconds % 60;

                            format!("{:02}:{:02}", minutes, seconds)
                        };

                        let mut current_seek = seek_to_timestamp;

                        ui.style_mut().spacing.slider_width = ui.available_width() - 100.0;
                        ui.style_mut().visuals.slider_trailing_fill = true;
                        let time_slider = ui.add(
                            eframe::egui::Slider::new(&mut current_seek, 0..=duration)
                                .logarithmic(false)
                                .show_value(false)
                                .clamping(eframe::egui::SliderClamping::Always)
                                .trailing_fill(true)
                                .handle_shape(HandleShape::Rect { aspect_ratio: 0.5 }),
                        );

                        // Update in real-time while dragging (just the timestamp, not seeking the audio)
                        if time_slider.dragged() && has_selected_track {
                            ctx.player_mut_ref().set_seek_to_timestamp(current_seek);
                        }

                        // Only perform the actual seek when drag is stopped
                        if time_slider.drag_stopped() && has_selected_track {
                            let player = ctx.player_mut_ref();
                            // We already updated seek_to_timestamp during dragging,
                            // now actually seek the audio playback
                            player.seek_to(current_seek);

                            // When seeking, make sure the track state is set to Playing
                            // This ensures the UI buttons match the actual state
                            player.track_state = crate::app::player::TrackState::Playing;
                        }

                        ui.label(format_time(current_seek));
                        ui.label("/");
                        ui.label(format_time(duration));
                    });

                    ui.add_space(10.0); // Add margin at the bottom

                    // Play/Pause, Previous, Next, Mode buttons
                    ui.horizontal(|ui| {
                        // Create buttons but disable them if no track is selected
                        let prev_btn = ui.add_enabled(
                            has_selected_track,
                            egui::Button::new("|◀").player_style(),
                        );

                        // Merge play/pause into a single button
                        let play_pause_btn = ui.add_enabled(
                            has_selected_track,
                            egui::Button::new(if is_playing { "⏸" } else { "▶" }).player_style(),
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

                        ui.vertical(|ui| {
                            // small buttons
                            ui.horizontal(|ui| {
                                // other small buttons
                                ui.add_enabled_ui(false, |ui| ui.button("1.0x"));

                                if ui.button(t("playlist_btn")).clicked() {
                                    ctx.ui_state.show_library_and_playlist =
                                        !ctx.ui_state.show_library_and_playlist;
                                    // Adjust window height based on visibility
                                    let new_height = if ctx.ui_state.show_library_and_playlist {
                                        ctx.default_window_height as f32
                                    } else {
                                        200.0 // Compact height when library and playlist are hidden
                                    };
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        vec2(ui.ctx().screen_rect().width(), new_height),
                                    ));
                                };

                                if ui.button(t("lyrics")).clicked() {
                                    ctx.ui_state.show_lyrics_panel =
                                        !ctx.ui_state.show_lyrics_panel;
                                };

                                if ui.button(t("mini")).clicked() {
                                    // Hide library and playlist
                                    ctx.ui_state.show_library_and_playlist = false;

                                    // Set minimal window size
                                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        vec2(
                                            300.0, // Minimal width
                                            200.0, // Same compact height as 列表 button
                                        ),
                                    ));
                                };

                                // Only enable the remove button if there's a selected track
                                if ui
                                    .add_enabled(
                                        has_selected_track,
                                        egui::Button::new(t("remove_song")),
                                    )
                                    .clicked()
                                {
                                    if let Some(track) = &selected_track {
                                        if let Some(current_playlist_idx) = ctx.current_playlist_idx
                                        {
                                            // Find the position of the current track in the playlist
                                            if let Some(current_track_position) =
                                                ctx.playlists[current_playlist_idx].get_pos(track)
                                            {
                                                // Get the next track before removing the current one
                                                let next_track = if current_track_position
                                                    < ctx.playlists[current_playlist_idx]
                                                        .tracks
                                                        .len()
                                                        - 1
                                                {
                                                    Some(
                                                        ctx.playlists[current_playlist_idx].tracks
                                                            [current_track_position + 1]
                                                            .clone(),
                                                    )
                                                } else if !ctx.playlists[current_playlist_idx]
                                                    .tracks
                                                    .is_empty()
                                                    && current_track_position > 0
                                                {
                                                    // If we're removing the last track, get the previous one
                                                    Some(
                                                        ctx.playlists[current_playlist_idx].tracks
                                                            [current_track_position - 1]
                                                            .clone(),
                                                    )
                                                } else {
                                                    None
                                                };

                                                // Remove the current track
                                                ctx.playlists[current_playlist_idx]
                                                    .remove(current_track_position);

                                                // Play the next track if available
                                                if let Some(next_track) = next_track {
                                                    let player = ctx.player_mut_ref();
                                                    player.select_track(Some(next_track));
                                                    player.play();
                                                } else {
                                                    // If no tracks left, clear the selected track
                                                    ctx.player_mut_ref().select_track(None);
                                                }
                                            }
                                        }
                                    }
                                };
                            });

                            // volume slider
                            ui.horizontal(|ui| {
                                let mut current_volume = volume;
                                let previous_vol = current_volume;
                                ui.label("📢");
                                ui.style_mut().spacing.slider_width = ui.available_width();
                                let volume_slider = ui.add(
                                    eframe::egui::Slider::new(
                                        &mut current_volume,
                                        0.0_f32..=1.0_f32,
                                    )
                                    .volume_style(),
                                );

                                if volume_slider.dragged() {
                                    // Only send if the volume is actually changing
                                    if current_volume != previous_vol {
                                        let is_processing_ui_change = ctx.is_processing_ui_change();
                                        ctx.player_mut_ref()
                                            .set_volume(current_volume, &is_processing_ui_change);
                                    }
                                }

                                // Handle button clicks if a track is selected
                                let mut fetch_lyrics = false;
                                if has_selected_track {
                                    // Check which action to take
                                    let mut action = None;

                                    if mode_btn.clicked() {
                                        action = Some("toggle_mode");
                                    } else if play_pause_btn.clicked() {
                                        action = Some(if is_playing { "pause" } else { "play" });
                                    } else if prev_btn.clicked()
                                        && ctx.playing_playlist_idx.is_some()
                                    {
                                        action = Some("previous");
                                        fetch_lyrics = true;
                                    } else if next_btn.clicked()
                                        && ctx.playing_playlist_idx.is_some()
                                    {
                                        action = Some("next");
                                        fetch_lyrics = true;
                                    }

                                    // Execute the action
                                    if let Some(action) = action {
                                        match action {
                                            "toggle_mode" => {
                                                ctx.player_mut_ref().toggle_playback_mode();
                                            }
                                            "pause" => {
                                                ctx.player_mut_ref().pause();
                                            }
                                            "play" => {
                                                ctx.player_mut_ref().play();
                                            }
                                            "previous" => {
                                                if let Some(idx) = ctx.playing_playlist_idx {
                                                    let playlist_clone = ctx.playlists[idx].clone();
                                                    ctx.player_mut_ref().previous(&playlist_clone);
                                                }
                                            }
                                            "next" => {
                                                if let Some(idx) = ctx.playing_playlist_idx {
                                                    let playlist_clone = ctx.playlists[idx].clone();
                                                    ctx.player_mut_ref().next(&playlist_clone);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }

                                if fetch_lyrics {
                                    ctx.fetch_lyrics_for_current_track();
                                }
                            });
                        });
                    });
                },
            );
        });
    }
}
