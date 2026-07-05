use super::AppComponent;
use crate::app::style::tokens;
use crate::app::version::version_info;
use crate::app::App;
use eframe::egui::{self, RichText, Sense};

pub struct Footer;

impl AppComponent for Footer {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.set_min_height(16.0);
        ui.horizontal_centered(|ui| {
            let version_text = format!("v{} ({})", version_info::VERSION, version_info::GIT_HASH);
            let version_response = ui.add(
                egui::Label::new(RichText::new(version_text).size(10.0).weak())
                    .sense(Sense::click_and_drag()),
            );
            Self::start_drag_from_response(ctx, ui, &version_response);

            ui.add_space(tokens::spacing::SM);

            if let Some(current_playlist_idx) = ctx.app_settings.current_playlist_idx {
                let selection_count = ctx.playlists[current_playlist_idx].selected_indices.len();
                if selection_count > 0 {
                    let selection_response =
                        ui.label(RichText::new(format!("{} selected", selection_count)).weak());
                    Self::start_drag_from_response(ctx, ui, &selection_response);

                    if ui.button("Clear Selection").clicked() {
                        let playlist = &mut ctx.playlists[current_playlist_idx];
                        playlist.clear_selection();
                    }
                }
            }

            let drag_rect = ui.available_rect_before_wrap();
            let drag_response = ui.interact(
                drag_rect,
                ui.id().with("footer_drag_area"),
                Sense::click_and_drag(),
            );
            Self::start_drag_from_response(ctx, ui, &drag_response);
        });
    }
}

impl Footer {
    fn start_drag_from_response(ctx: &App, ui: &egui::Ui, response: &egui::Response) {
        if response.drag_started_by(egui::PointerButton::Primary) && !ctx.ui_state.is_maximized {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }
}
