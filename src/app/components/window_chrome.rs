use super::language_selector::LanguageSelector;
use super::AppComponent;
use crate::app::constants::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};
use crate::app::t;
use crate::app::version::version_info;
use crate::app::App;
use eframe::egui::{self, Color32, RichText, Window};
use rfd;

pub struct WindowChrome;

impl AppComponent for WindowChrome {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            // Menu list
            ui.menu_button(t("file"), |ui| {
                if ui.button(t("open")).clicked() {
                    if let Some(new_path) = rfd::FileDialog::new().pick_folder() {
                        // Add the path to the library
                        let path_exists = !ctx.library.add_path(new_path.clone());

                        // If it existed, find it and rescan. If new, import the bottom-most path.
                        let path_to_import = if path_exists {
                            ctx.library
                                .paths()
                                .iter()
                                .find(|p| *p.path() == new_path)
                                .cloned()
                        } else {
                            ctx.library.paths().last().cloned()
                        };

                        if let Some(p) = path_to_import {
                            ctx.import_library_paths(&p);
                        }
                    }
                    ui.close_menu();
                }
                let settings_label =
                    egui::RichText::new(t("settings")).text_style(egui::TextStyle::Button);
                ui.add_enabled_ui(false, |ui| ui.button(settings_label))
                    .response
                    .on_hover_text("Not implemented yet");
                ui.separator();
                if ui.button(t("exit")).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close_menu();
                }
            });

            // Add Playback menu
            let mut fetch_lyrics = false;
            ui.menu_button(t("playback"), |ui| {
                if ctx.runtime.is_some() {
                    // Cache playlist before borrowing player mutably
                    let playlist_clone = ctx
                        .app_settings
                        .playing_playlist_idx
                        .and_then(|idx| ctx.playlists.get(idx).cloned());

                    let player = ctx.player_mut_ref();
                    if let Some(_selected_track) = &player.selected_track {
                        if ui.button(t("play_pause")).clicked() {
                            match player.track_state {
                                crate::app::player::TrackState::Playing => {
                                    player.pause();
                                }
                                _ => {
                                    player.play();
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button(t("previous")).clicked() {
                            if let Some(playlist) = &playlist_clone {
                                player.previous(playlist);
                                fetch_lyrics = true;
                            }
                            ui.close_menu();
                        }
                        if ui.button(t("next")).clicked() {
                            if let Some(playlist) = &playlist_clone {
                                player.next(playlist);
                                fetch_lyrics = true;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        // Show current play mode in the menu
                        let mode_icon = match player.playback_mode {
                            crate::app::player::PlaybackMode::Normal => "➡",
                            crate::app::player::PlaybackMode::Repeat => "🔁",
                            crate::app::player::PlaybackMode::RepeatOne => "🔂",
                            crate::app::player::PlaybackMode::Shuffle => "🔀",
                        };
                        if ui
                            .button(crate::app::tf("play_mode", &[mode_icon]))
                            .clicked()
                        {
                            player.toggle_playback_mode();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(t("restore_window")).clicked() {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                egui::Vec2::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT),
                            ));
                            ui.close_menu();
                        }
                    } else {
                        ui.add_enabled_ui(false, |ui| {
                            let _ = ui.button(t("play_pause"));
                            let _ = ui.button(t("previous"));
                            let _ = ui.button(t("next"));
                            ui.separator();
                            let _ = ui.button(crate::app::tf("play_mode", &["➡"]));
                        });
                    }
                }
            });

            if fetch_lyrics {
                ctx.fetch_lyrics_for_current_track();
            }

            // Add View menu
            ui.menu_button(t("view"), |ui| {
                let lyrics_text = if ctx.ui_state.show_lyrics_panel {
                    t("hide_lyrics")
                } else {
                    t("show_lyrics")
                };
                if ui.button(lyrics_text).clicked() {
                    ctx.ui_state.show_lyrics_panel = !ctx.ui_state.show_lyrics_panel;
                    ui.close_menu();
                }

                ui.separator();

                ui.menu_button("Desktop Lyrics", |ui| {
                    ui.checkbox(&mut ctx.ui_state.desktop_lyrics_enabled, "Enabled");
                    ui.checkbox(&mut ctx.ui_state.desktop_lyrics_locked, "Locked");
                    ui.horizontal(|ui| {
                        ui.label("Font size");
                        ui.add(
                            egui::Slider::new(
                                &mut ctx.ui_state.desktop_lyrics_font_size,
                                16.0..=120.0,
                            )
                            .step_by(1.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label("Color");
                        let mut srgba = egui::Color32::from_rgba_unmultiplied(
                            ctx.ui_state.desktop_lyrics_color[0],
                            ctx.ui_state.desktop_lyrics_color[1],
                            ctx.ui_state.desktop_lyrics_color[2],
                            ctx.ui_state.desktop_lyrics_color[3],
                        );
                        if ui.color_edit_button_srgba(&mut srgba).changed() {
                            ctx.ui_state.desktop_lyrics_color =
                                [srgba.r(), srgba.g(), srgba.b(), srgba.a()];
                        }
                    });
                });
            });

            ui.menu_button(t("help"), |ui| {
                if ui.button(t("about")).clicked() {
                    ctx.ui_state.show_about_dialog = true;
                    ui.close_menu();
                }
            });

            // Add language selector
            LanguageSelector::add(ctx, ui);

            // Take up remaining space
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Window operation buttons
                let button_size = egui::vec2(30.0, 20.0);

                // Close button with hover detection
                let close_btn =
                    egui::Button::new(crate::app::style::icons::CLOSE).min_size(button_size);
                let close_response = ui.add(close_btn.fill(Color32::TRANSPARENT));
                if close_response.clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }

                // Maximize button
                let maximize_response = ui.add(
                    egui::Button::new(RichText::new("↗").size(14.0))
                        .min_size(button_size)
                        .fill(Color32::TRANSPARENT),
                );
                if maximize_response.clicked() {
                    // Toggle maximize
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                        !ctx.ui_state.is_maximized,
                    ));
                    ctx.ui_state.is_maximized = !ctx.ui_state.is_maximized;
                }

                // Minimize button
                let minimize_response = ui.add(
                    egui::Button::new(RichText::new("−").size(14.0))
                        .min_size(button_size)
                        .fill(Color32::TRANSPARENT),
                );
                if minimize_response.clicked() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }

                // Add window drag area - using technique from egui's custom_window_frame example
                // The drag area will fill the remaining space in the top panel
                let title_bar_rect = ui.available_rect_before_wrap();

                let title_bar_response = ui.interact(
                    title_bar_rect,
                    ui.id().with("title_bar"),
                    egui::Sense::click_and_drag(),
                );

                // Double click to maximize/restore (common UI pattern)
                if title_bar_response.double_clicked() {
                    ctx.ui_state.is_maximized = !ctx.ui_state.is_maximized;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                        ctx.ui_state.is_maximized,
                    ));
                }

                // This approach explicitly checks for drag start with primary button
                // which works better across platforms including Ubuntu/Linux
                if title_bar_response.drag_started_by(egui::PointerButton::Primary)
                    && !ctx.ui_state.is_maximized
                {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
        });

        // Show About dialog if requested
        if ctx.ui_state.show_about_dialog {
            Window::new(t("about"))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(20.0);
                        ui.heading(RichText::new(t("app_name")).size(24.0));
                        ui.add_space(10.0);
                        ui.label(RichText::new(version_info::formatted_version()).size(16.0));
                        ui.add_space(20.0);
                        ui.label(t("app_description"));
                        ui.add_space(20.0);
                        ui.label(t("features"));
                        ui.label(t("feature_1"));
                        ui.label(t("feature_2"));
                        ui.label(t("feature_3"));
                        ui.label(t("feature_4"));
                        ui.add_space(20.0);
                        ui.label(t("contact_email"));
                        ui.add_space(20.0);
                        if ui.button(t("exit")).clicked() {
                            ctx.ui_state.show_about_dialog = false;
                        }
                    });
                });
        }
    }
}
