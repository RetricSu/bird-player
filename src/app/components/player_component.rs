use eframe::egui::{self, vec2};

use super::scope_component::ScopeComponent;
use super::AppComponent;
use crate::egui::style::HandleShape;
use crate::{app::App, UiCommand};

pub struct PlayerComponent;

impl AppComponent for PlayerComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        if let Some(selected_track) = ctx.player.as_mut().unwrap().selected_track.clone() {
            ui.horizontal(|ui| {
                ScopeComponent::add(ctx, ui);
                ui.vertical(|ui| {
                    ui.add_space(10.0); // Add margin at the top
                    ui.label(format!(
                        "{} - {}",
                        &selected_track
                            .artist()
                            .unwrap_or("unknown artist".to_string()),
                        &selected_track
                            .title()
                            .unwrap_or("unknown title".to_string())
                    ));
                    ui.add_space(10.0); // Add margin at the bottom

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

                        let mut seek_to_timestamp = ctx.player.as_ref().unwrap().seek_to_timestamp;
                        let mut duration = ctx.player.as_ref().unwrap().duration;

                        if let Ok(new_seek_cmd) = ctx.player.as_ref().unwrap().ui_rx.try_recv() {
                            match new_seek_cmd {
                                UiCommand::CurrentTimestamp(seek_timestamp) => {
                                    seek_to_timestamp = seek_timestamp;
                                }
                                UiCommand::TotalTrackDuration(dur) => {
                                    tracing::info!("Received Duration: {}", dur);
                                    duration = dur;
                                    ctx.player.as_mut().unwrap().set_duration(dur);
                                }
                                UiCommand::AudioFinished => {
                                    tracing::info!("Track finished, getting next...");

                                    ctx.player
                                        .as_mut()
                                        .unwrap()
                                        .next(&ctx.playlists[(ctx.current_playlist_idx).unwrap()]);
                                } //_ => {}
                            }
                        }

                        ui.style_mut().spacing.slider_width = ui.available_width() - 100.0;
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
                    ui.add_space(20.0); // Add margin at the bottom
                    ui.horizontal(|ui| {
                        let stop_btn = ui.add(
                            egui::Button::new("■")
                                .min_size(vec2(40.0, 40.0))
                                .corner_radius(20.0),
                        );
                        let play_btn = ui.add(
                            egui::Button::new("▶")
                                .min_size(vec2(40.0, 40.0))
                                .corner_radius(20.0),
                        );
                        let pause_btn = ui.add(
                            egui::Button::new("⏸")
                                .min_size(vec2(40.0, 40.0))
                                .corner_radius(20.0),
                        );
                        let prev_btn = ui.add(
                            egui::Button::new("|◀")
                                .min_size(vec2(40.0, 40.0))
                                .corner_radius(20.0),
                        );
                        let next_btn = ui.add(
                            egui::Button::new("▶|")
                                .min_size(vec2(40.0, 40.0))
                                .corner_radius(20.0),
                        );

                        let mut volume = ctx.player.as_ref().unwrap().volume;
                        let previous_vol = volume;

                        ui.label("📢");
                        let volume_slider = ui.add(
                            eframe::egui::Slider::new(&mut volume, 0.0_f32..=1.0_f32)
                                .logarithmic(false)
                                .show_value(false)
                                .clamping(eframe::egui::SliderClamping::Always)
                                .step_by(0.01),
                        );

                        if volume_slider.dragged() {
                            if let Some(is_processing_ui_change) = &ctx.is_processing_ui_change {
                                // Only send if the volume is actually changing
                                if volume != previous_vol {
                                    ctx.player
                                        .as_mut()
                                        .unwrap()
                                        .set_volume(volume, is_processing_ui_change);
                                }
                            }
                        }

                        if let Some(_selected_track) = &ctx.player.as_mut().unwrap().selected_track
                        {
                            if stop_btn.clicked() {
                                ctx.player.as_mut().unwrap().stop();
                            }

                            if play_btn.clicked() {
                                ctx.player.as_mut().unwrap().play();
                            }

                            if pause_btn.clicked() {
                                ctx.player.as_mut().unwrap().pause();
                            }

                            if prev_btn.clicked() {
                                ctx.player
                                    .as_mut()
                                    .unwrap()
                                    .previous(&ctx.playlists[(ctx.current_playlist_idx).unwrap()]);
                            }

                            if next_btn.clicked() {
                                ctx.player
                                    .as_mut()
                                    .unwrap()
                                    .next(&ctx.playlists[(ctx.current_playlist_idx).unwrap()]);
                            }
                        }
                    });
                });
            });
        }
    }
}
