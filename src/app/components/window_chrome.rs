use super::AppComponent;
use crate::app::App;
use eframe::egui::{self, Color32, RichText, Window};
use rfd;

pub struct WindowChrome;

impl AppComponent for WindowChrome {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            // Menu list
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
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
                    egui::RichText::new("Settings").text_style(egui::TextStyle::Button);
                let settings_btn = ui.button(settings_label);
                if settings_btn.clicked() {
                    ui.close_menu();
                }
                settings_btn.on_hover_text("Not implemented yet");
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    ctx.show_about_dialog = true;
                    ui.close_menu();
                }
            });

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
                    egui::Button::new(RichText::new("[]").size(14.0))
                        .min_size(button_size)
                        .fill(Color32::TRANSPARENT),
                );
                if maximize_response.clicked() {
                    // Toggle maximize state
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
            Window::new("About")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.add_space(20.0);
                        ui.heading(RichText::new("Bird Player").size(24.0));
                        ui.add_space(10.0);
                        ui.label(RichText::new("Version 0.1.0").size(16.0));
                        ui.add_space(20.0);
                        ui.label("A simple GUI music player inspired by foobar2000");
                        ui.label("written in Rust using egui");
                        ui.add_space(20.0);
                        ui.label("Features:");
                        ui.label("• Basic music player functionality (play, pause, stop)");
                        ui.label("• Music library with ID3 tag support");
                        ui.label("• Playlist management");
                        ui.label("• Drag and drop support");
                        ui.label("• State persistence");
                        ui.add_space(20.0);
                        if ui.button("Close").clicked() {
                            ctx.show_about_dialog = false;
                        }
                    });
                });
        }
    }
}
