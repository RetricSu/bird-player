use super::AppComponent;
use crate::app::App;
use eframe::egui::{self, Color32, RichText};

pub struct WindowChrome;

impl AppComponent for WindowChrome {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            // Menu list
            ui.menu_button("File", |ui| {
                if ui.button("Open").clicked() {
                    // TODO: Implement file open
                    ui.close_menu();
                }
                if ui.button("Settings").clicked() {
                    // TODO: Implement settings
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    // TODO: Implement about dialog
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
    }
}
