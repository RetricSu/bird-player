use super::language_selector::LanguageSelector;
use super::AppComponent;
use crate::app::t;
use crate::app::version_info;
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
                        ctx.library.add_path(new_path);

                        // Get the last added path and import it
                        if let Some(newest_path) = ctx.library.paths().last() {
                            if newest_path.status()
                                == crate::app::library::LibraryPathStatus::NotImported
                            {
                                ctx.import_library_paths(newest_path);
                            }
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
            ui.menu_button(t("playback"), |ui| {
                if let Some(player) = &mut ctx.player {
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
                            if let Some(current_playlist_idx) = ctx.current_playlist_idx {
                                player.previous(&ctx.playlists[current_playlist_idx]);
                            }
                            ui.close_menu();
                        }
                        if ui.button(t("next")).clicked() {
                            if let Some(current_playlist_idx) = ctx.current_playlist_idx {
                                player.next(&ctx.playlists[current_playlist_idx]);
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

            ui.menu_button(t("help"), |ui| {
                if ui.button(t("about")).clicked() {
                    ctx.show_about_dialog = true;
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
                let close_btn = egui::Button::new("x").min_size(button_size);
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
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!ctx.is_maximized));
                    ctx.is_maximized = !ctx.is_maximized;
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

                // Add window drag area
                let title_bar_response =
                    ui.allocate_response(ui.available_size(), egui::Sense::click_and_drag());

                if title_bar_response.dragged() && !ctx.is_maximized {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
            });
        });

        // Show About dialog if requested
        if ctx.show_about_dialog {
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
                            ctx.show_about_dialog = false;
                        }
                    });
                });
        }
    }
}
