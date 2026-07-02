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
            // Render the menu triggers (文件 / 播放 / ...) as borderless,
            // hover-tinted buttons so they share a visual language with the
            // window control buttons on the right edge of the same row.
            crate::app::style::borderless_button_visuals(ui.visuals_mut());

            ui.label(RichText::new("Bird").strong());
            ui.separator();

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
                            crate::app::player::PlaybackMode::Normal => {
                                crate::app::style::icons::MODE_NORMAL
                            }
                            crate::app::player::PlaybackMode::Repeat => {
                                crate::app::style::icons::MODE_REPEAT
                            }
                            crate::app::player::PlaybackMode::RepeatOne => {
                                crate::app::style::icons::MODE_REPEAT_ONE
                            }
                            crate::app::player::PlaybackMode::Shuffle => {
                                crate::app::style::icons::MODE_SHUFFLE
                            }
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
                            let _ = ui.button(crate::app::tf(
                                "play_mode",
                                &[crate::app::style::icons::MODE_NORMAL],
                            ));
                        });
                    }
                }
            });

            if fetch_lyrics {
                ctx.auto_fetch_lyrics_for_current_track();
            }

            // Add View menu
            ui.menu_button(t("view"), |ui| {
                let lyrics_text = if ctx.ui_state.show_lyrics_panel {
                    t("hide_lyrics")
                } else {
                    t("show_lyrics")
                };
                if ui.button(lyrics_text).clicked() {
                    let will_show = !ctx.ui_state.show_lyrics_panel;
                    ctx.ui_state.show_lyrics_panel = will_show;
                    if will_show {
                        ctx.fetch_lyrics_for_current_track();
                    }
                    ui.close_menu();
                }
                ui.checkbox(
                    &mut ctx.ui_state.auto_fetch_missing_lyrics,
                    t("auto_fetch_missing_lyrics"),
                );

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
                use crate::app::style::icons;

                // Helper: a borderless chrome button with a subtle hover fill.
                // `danger_hover` makes the hover state read red (used on close).
                //
                // We deliberately avoid `Button::fill()` / `Button::stroke()` —
                // those overrides apply to every state (egui resolves them via
                // `style.interact(&response)`), which is what made the close
                // glyph vanish before: a hard-coded transparent fill stomped
                // the red hover background, while `hovered.fg_stroke = WHITE`
                // turned the X into white-on-white. Driving the colours through
                // `WidgetVisuals` instead lets the inactive state stay clean
                // and the hovered state pick up the danger / theme fill.
                let chrome_button = |ui: &mut egui::Ui,
                                     glyph: &str,
                                     danger_hover: bool|
                 -> egui::Response {
                    let size = egui::vec2(32.0, 22.0);
                    let base_text = ui.visuals().widgets.inactive.fg_stroke.color;
                    let hover_fill = if danger_hover {
                        Color32::from_rgb(232, 17, 35)
                    } else {
                        ui.visuals().widgets.hovered.weak_bg_fill
                    };
                    let hover_text = if danger_hover {
                        Color32::WHITE
                    } else {
                        ui.visuals().widgets.hovered.fg_stroke.color
                    };
                    ui.scope(|ui| {
                        let widgets = &mut ui.visuals_mut().widgets;
                        // Inactive: fully transparent so the title bar shines through.
                        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                        widgets.inactive.bg_fill = Color32::TRANSPARENT;
                        widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        widgets.inactive.fg_stroke.color = base_text;
                        // Hovered: tinted background, white glyph for danger.
                        widgets.hovered.weak_bg_fill = hover_fill;
                        widgets.hovered.bg_fill = hover_fill;
                        widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        widgets.hovered.fg_stroke.color = hover_text;
                        // Active (mouse-down): keep same colour as hovered for stability.
                        widgets.active.weak_bg_fill = hover_fill;
                        widgets.active.bg_fill = hover_fill;
                        widgets.active.bg_stroke = egui::Stroke::NONE;
                        widgets.active.fg_stroke.color = hover_text;
                        ui.add(egui::Button::new(RichText::new(glyph).size(14.0)).min_size(size))
                    })
                    .inner
                };

                // Close — red on hover.
                if chrome_button(ui, icons::WINDOW_CLOSE, true).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }

                // Maximize / restore — swap the icon based on current state.
                let maximize_icon = if ctx.ui_state.is_maximized {
                    icons::WINDOW_RESTORE
                } else {
                    icons::WINDOW_MAXIMIZE
                };
                if chrome_button(ui, maximize_icon, false).clicked() {
                    ctx.ui_state.is_maximized = !ctx.ui_state.is_maximized;
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(
                        ctx.ui_state.is_maximized,
                    ));
                }

                // Minimize.
                if chrome_button(ui, icons::WINDOW_MINIMIZE, false).clicked() {
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
