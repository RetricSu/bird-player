use eframe::egui::{self, vec2};
use std::time::Instant;

use super::scope_component::ScopeComponent;
use super::AppComponent;
use crate::app::style::{ButtonExt, SliderExt};
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
        if let Some(selected_track) = ctx.player.as_mut().unwrap().selected_track.clone() {
            ui.horizontal(|ui| {
                ScopeComponent::add(ctx, ui);

                // Add minimum width constraint for the vertical layout
                let min_width = 200.0; // Minimum width in pixels
                let available_width = ui.available_width();
                let panel_width = (available_width - CASSETTE_WIDTH).max(min_width);

                ui.allocate_ui_with_layout(
                    vec2(panel_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.add_space(10.0); // Add margin at the top
                        ui.add(
                            eframe::egui::Label::new(format!(
                                "{} - {}",
                                &selected_track
                                    .artist()
                                    .unwrap_or("unknown artist".to_string()),
                                &selected_track
                                    .title()
                                    .unwrap_or("unknown title".to_string())
                            ))
                            .wrap_mode(eframe::egui::TextWrapMode::Truncate),
                        )
                        .highlight();
                        ui.label(format!(
                            "from {}",
                            ctx.playlists[ctx.current_playlist_idx.unwrap()]
                                .get_name()
                                .unwrap()
                        ));

                        // Display lyrics if they exist
                        if let Some(lyrics) = &selected_track.lyrics() {
                            ui.add_space(10.0);
                            eframe::egui::ScrollArea::vertical()
                                .max_height(150.0)
                                .id_salt("lyrics_scroll")
                                .show(ui, |ui| {
                                    ui.label(lyrics);
                                });
                        } else {
                            ui.label("No lyrics available");
                        }

                        // Add space to push controls to bottom
                        ui.add_space(ui.available_height() - 70.0);

                        // Time Slider
                        // Format the timestamp and duration as hours:minutes:seconds
                        ui.horizontal(|ui| {
                            let format_time = |timestamp: u64| -> String {
                                let seconds = timestamp / 1000;
                                let minutes = seconds / 60;
                                let hours = minutes / 60;
                                let _seconds_remainder = seconds % 60;
                                let minutes_remainder = minutes % 60;

                                format!("{:02}:{:02}", hours, minutes_remainder)
                            };

                            let mut seek_to_timestamp =
                                ctx.player.as_ref().unwrap().seek_to_timestamp;
                            let mut duration = ctx.player.as_ref().unwrap().duration;

                            if let Ok(new_seek_cmd) = ctx.player.as_ref().unwrap().ui_rx.try_recv()
                            {
                                match new_seek_cmd {
                                    UiCommand::CurrentTimestamp(seek_timestamp) => {
                                        seek_to_timestamp = seek_timestamp;

                                        // Save player state every 30 seconds during playback
                                        LAST_SAVE.with(|last_save| {
                                            let elapsed = last_save.borrow().elapsed().as_secs();
                                            if elapsed > 30 {
                                                // Update persistence state
                                                ctx.update_player_persistence();
                                                ctx.save_state();

                                                // Reset timer
                                                *last_save.borrow_mut() = Instant::now();
                                            }
                                        });
                                    }
                                    UiCommand::TotalTrackDuration(dur) => {
                                        tracing::info!("Received Duration: {}", dur);
                                        duration = dur;
                                        ctx.player.as_mut().unwrap().set_duration(dur);
                                    }
                                    UiCommand::AudioFinished => {
                                        tracing::info!("Track finished, getting next...");

                                        ctx.player.as_mut().unwrap().next(
                                            &ctx.playlists[(ctx.current_playlist_idx).unwrap()],
                                        );
                                    }
                                }
                            }

                            ui.style_mut().spacing.slider_width = ui.available_width() - 100.0;
                            ui.style_mut().visuals.slider_trailing_fill = true; // the trailing_fill has some bug, so we need to use this
                            let time_slider = ui.add(
                                eframe::egui::Slider::new(&mut seek_to_timestamp, 0..=duration)
                                    .logarithmic(false)
                                    .show_value(false)
                                    .clamping(eframe::egui::SliderClamping::Always)
                                    .trailing_fill(true)
                                    .handle_shape(HandleShape::Rect { aspect_ratio: 0.5 }),
                            );

                            ctx.player
                                .as_mut()
                                .unwrap()
                                .set_seek_to_timestamp(seek_to_timestamp);

                            if time_slider.drag_stopped() {
                                ctx.player.as_mut().unwrap().seek_to(seek_to_timestamp);
                            }

                            ui.label(format_time(seek_to_timestamp));
                            ui.label("/");
                            ui.label(format_time(duration));
                        });

                        ui.add_space(10.0); // Add margin at the bottom

                        // Play/Pause, Previous, Next, Mode buttons
                        ui.horizontal(|ui| {
                            let prev_btn = ui.add(egui::Button::new("|◀").player_style());

                            // Merge play/pause into a single button
                            let play_pause_btn = ui.add(
                                egui::Button::new(
                                    if matches!(
                                        ctx.player.as_ref().unwrap().track_state,
                                        crate::app::player::TrackState::Playing
                                    ) {
                                        "⏸"
                                    } else {
                                        "▶"
                                    },
                                )
                                .player_style(),
                            );

                            let next_btn = ui.add(egui::Button::new("▶|").player_style());

                            let mode_icon = match ctx.player.as_ref().unwrap().playback_mode {
                                crate::app::player::PlaybackMode::Normal => "➡",
                                crate::app::player::PlaybackMode::Repeat => "🔁",
                                crate::app::player::PlaybackMode::RepeatOne => "🔂",
                                crate::app::player::PlaybackMode::Shuffle => "🔀",
                            };

                            let mode_btn = ui.add(egui::Button::new(mode_icon).player_style());

                            ui.vertical(|ui| {
                                // small buttons
                                ui.horizontal(|ui| {
                                    // other small buttons
                                    ui.button("1.0x").clicked();
                                    if ui.button("列表").clicked() {
                                        ctx.show_library_and_playlist =
                                            !ctx.show_library_and_playlist;
                                        // Adjust window height based on visibility
                                        let new_height = if ctx.show_library_and_playlist {
                                            ctx.default_window_height as f32
                                        } else {
                                            200.0 // Compact height when library and playlist are hidden
                                        };
                                        ui.ctx().send_viewport_cmd(
                                            egui::ViewportCommand::InnerSize(vec2(
                                                ui.ctx().screen_rect().width(),
                                                new_height,
                                            )),
                                        );
                                    };
                                    if ui.button("歌词").clicked() {};

                                    if ui.button("最小化").clicked() {
                                        // Hide library and playlist
                                        ctx.show_library_and_playlist = false;

                                        // Set minimal window size
                                        ui.ctx().send_viewport_cmd(
                                            egui::ViewportCommand::InnerSize(vec2(
                                                300.0, // Minimal width
                                                200.0, // Same compact height as 列表 button
                                            )),
                                        );
                                    };
                                    if ui.button("移除歌曲").clicked() {};
                                });

                                // volume slider
                                ui.horizontal(|ui| {
                                    let mut volume = ctx.player.as_ref().unwrap().volume;
                                    let previous_vol = volume;
                                    ui.label("📢");
                                    ui.style_mut().spacing.slider_width = ui.available_width();
                                    let volume_slider = ui.add(
                                        eframe::egui::Slider::new(&mut volume, 0.0_f32..=1.0_f32)
                                            .volume_style(),
                                    );

                                    if volume_slider.dragged() {
                                        if let Some(is_processing_ui_change) =
                                            &ctx.is_processing_ui_change
                                        {
                                            // Only send if the volume is actually changing
                                            if volume != previous_vol {
                                                ctx.player
                                                    .as_mut()
                                                    .unwrap()
                                                    .set_volume(volume, is_processing_ui_change);
                                            }
                                        }
                                    }

                                    if let Some(_selected_track) =
                                        &ctx.player.as_mut().unwrap().selected_track
                                    {
                                        if mode_btn.clicked() {
                                            ctx.player.as_mut().unwrap().toggle_playback_mode();
                                        }

                                        if play_pause_btn.clicked() {
                                            match ctx.player.as_ref().unwrap().track_state {
                                                crate::app::player::TrackState::Playing => {
                                                    ctx.player.as_mut().unwrap().pause();
                                                }
                                                _ => {
                                                    ctx.player.as_mut().unwrap().play();
                                                }
                                            }
                                        }

                                        if prev_btn.clicked() {
                                            ctx.player.as_mut().unwrap().previous(
                                                &ctx.playlists[(ctx.current_playlist_idx).unwrap()],
                                            );
                                        }

                                        if next_btn.clicked() {
                                            ctx.player.as_mut().unwrap().next(
                                                &ctx.playlists[(ctx.current_playlist_idx).unwrap()],
                                            );
                                        }
                                    }
                                });
                            });
                        });
                    },
                );
            });
        }
    }
}
