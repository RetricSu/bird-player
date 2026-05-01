use super::AppComponent;
use crate::app::services::PlaylistService;
use crate::app::style::{icons, tokens};
use crate::app::t;
use crate::app::App;
use eframe::egui;

pub struct PlaylistTabs;

impl AppComponent for PlaylistTabs {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Caller (PlaylistContent) wraps us in style::panel_header which
        // already provides ui.horizontal + min_height — render directly.
        // Add playlist tabs
        for (idx, playlist) in ctx.playlists.iter_mut().enumerate() {
            let is_selected = ctx.app_settings.current_playlist_idx == Some(idx);
            let is_being_renamed = ctx.ui_state.playlist_being_renamed == Some(idx);

            if is_being_renamed {
                // Show text input for renaming
                let mut name = playlist.get_name().unwrap_or_default();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(120.0)
                        .hint_text(t("enter_name")),
                );

                if response.changed() {
                    playlist.set_name(name.clone());
                }

                if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !name.is_empty() {
                        playlist.set_name(name);
                    }
                    PlaylistService::finish_renaming_playlist_ui(
                        &mut ctx.ui_state.playlist_being_renamed,
                    );
                }
            } else {
                // Show normal tab button. Selected tab paints with the
                // brand fill (consumed via Visuals::selection) so it
                // matches the rest of the highlighted-state language used
                // by the player; unselected tabs render flat to keep the
                // tab strip from looking like a row of buttons.
                let mut tab_text = egui::RichText::new(playlist.get_name().unwrap_or_default())
                    .size(tokens::text::SM);
                if is_selected {
                    tab_text = tab_text.color(egui::Color32::WHITE);
                }

                let mut button = egui::Button::new(tab_text).corner_radius(tokens::radius::SM);
                if is_selected {
                    button = button.fill(tokens::color::BRAND).stroke(egui::Stroke::new(
                        tokens::size::STROKE_WIDTH,
                        tokens::color::BRAND_ACTIVE,
                    ));
                } else {
                    button = button.fill(egui::Color32::TRANSPARENT);
                }
                let tab_response = ui.add(button);

                if tab_response.clicked() {
                    PlaylistService::select_playlist(
                        &mut ctx.app_settings.current_playlist_idx,
                        idx,
                    );
                }

                // Show context menu on right-click
                tab_response.context_menu(|ui| {
                    if ui.button(t("rename")).clicked() {
                        PlaylistService::start_renaming_playlist(
                            &mut ctx.ui_state.playlist_being_renamed,
                            idx,
                        );
                        ui.close_menu();
                    }
                    if ui.button(t("delete")).clicked() {
                        ctx.ui_state.playlist_idx_to_remove = Some(idx);
                        ui.close_menu();
                    }
                });
            }
        }

        // Add the "+" button for creating new playlists — borderless to
        // match the library's add-folder affordance.
        let create_btn = ui.add(egui::Button::new(icons::PLUS).frame(false));

        if create_btn.clicked() {
            PlaylistService::create_playlist(
                &mut ctx.playlists,
                &mut ctx.app_settings.current_playlist_idx,
                &mut ctx.ui_state.playlist_being_renamed,
                t("new_playlist").to_string(),
            );
        }

        // Handle playlist removal
        if let Some(idx) = ctx.ui_state.playlist_idx_to_remove {
            ctx.ui_state.playlist_idx_to_remove = None;

            let db_conn = ctx.db().connection();
            PlaylistService::delete_playlist(
                &mut ctx.playlists,
                &mut ctx.app_settings.current_playlist_idx,
                idx,
                &db_conn,
            );
        }
    }
}
