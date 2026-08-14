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
        // already provides ui.horizontal + min_height. The tab strip itself
        // may be wider than the panel, so keep it horizontally scrollable.
        egui::ScrollArea::horizontal()
            .id_salt("playlist_tabs_scroll")
            .auto_shrink([false, true])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Self::show_tabs(ctx, ui);
                });
            });
    }
}

impl PlaylistTabs {
    fn show_tabs(ctx: &mut App, ui: &mut eframe::egui::Ui) {
        // Add playlist tabs
        for idx in 0..ctx.playlists.len() {
            let is_selected = ctx.app_settings.current_playlist_idx == Some(idx);
            let is_being_renamed = ctx.ui_state.playlist_being_renamed == Some(idx);

            if is_being_renamed {
                // Show text input for renaming
                let mut name = ctx.playlists[idx].get_name().unwrap_or_default();
                let response = ui.add(
                    egui::TextEdit::singleline(&mut name)
                        .desired_width(120.0)
                        .hint_text(t("enter_name")),
                );

                if response.changed() {
                    ctx.playlists[idx].set_name(name.clone());
                }

                if response.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if !name.is_empty() {
                        ctx.playlists[idx].set_name(name);
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
                let mut tab_text =
                    egui::RichText::new(ctx.playlists[idx].get_name().unwrap_or_default())
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
                let tab_response = ui.add(button.sense(egui::Sense::click_and_drag()));

                if tab_response.clicked() {
                    PlaylistService::select_playlist(
                        &mut ctx.app_settings.current_playlist_idx,
                        idx,
                    );
                }

                if tab_response.drag_started() {
                    ctx.ui_state.playlist_tab_dragging = Some(idx);
                }

                if tab_response.hovered()
                    && ui.input(|input| input.pointer.any_released())
                    && ctx
                        .ui_state
                        .playlist_tab_dragging
                        .is_some_and(|from| from != idx)
                {
                    if let Some(from_idx) = ctx.ui_state.playlist_tab_dragging.take() {
                        PlaylistService::reorder_playlist(
                            &mut ctx.playlists,
                            &mut ctx.app_settings.current_playlist_idx,
                            &mut ctx.app_settings.playing_playlist_idx,
                            &mut ctx.ui_state.playlist_being_renamed,
                            &mut ctx.ui_state.playlist_idx_to_remove,
                            &mut ctx.ui_state.playlist_booklet_idx,
                            from_idx,
                            idx,
                        );
                        ctx.save_state();
                    }
                    return;
                }

                // Show context menu on right-click
                tab_response.context_menu(|ui| {
                    let has_tracks = !ctx.playlists[idx].tracks.is_empty();
                    if ui
                        .add_enabled(has_tracks, egui::Button::new(t("play_all")))
                        .clicked()
                    {
                        ctx.play_playlist_from_start(idx);
                        ui.close_menu();
                    }
                    if ui.button(t("view_playlist")).clicked() {
                        ctx.app_settings.current_playlist_idx = Some(idx);
                        ctx.ui_state.playlist_booklet_mode =
                            Some(crate::app::state::ui_state::PlaylistBookletMode::View);
                        ctx.ui_state.playlist_booklet_idx = Some(idx);
                        ctx.ui_state.playlist_booklet_draft = None;
                        ui.close_menu();
                    }
                    if ui.button(t("edit_playlist")).clicked() {
                        ctx.app_settings.current_playlist_idx = Some(idx);
                        ctx.ui_state.playlist_booklet_mode =
                            Some(crate::app::state::ui_state::PlaylistBookletMode::Edit);
                        ctx.ui_state.playlist_booklet_idx = Some(idx);
                        ctx.ui_state.playlist_booklet_draft = None;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button(t("rename")).clicked() {
                        PlaylistService::start_renaming_playlist(
                            &mut ctx.ui_state.playlist_being_renamed,
                            idx,
                        );
                        ui.close_menu();
                    }
                    if ui.button(t("export_playlist")).clicked() {
                        ctx.export_playlist_to_archive(idx);
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

        if ui.input(|input| input.pointer.any_released()) {
            ctx.ui_state.playlist_tab_dragging = None;
        }

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

            if let Some(booklet_idx) = ctx.ui_state.playlist_booklet_idx {
                match booklet_idx.cmp(&idx) {
                    std::cmp::Ordering::Equal => {
                        ctx.ui_state.playlist_booklet_mode = None;
                        ctx.ui_state.playlist_booklet_idx = None;
                        ctx.ui_state.playlist_booklet_draft = None;
                    }
                    std::cmp::Ordering::Greater => {
                        ctx.ui_state.playlist_booklet_idx = Some(booklet_idx - 1);
                    }
                    std::cmp::Ordering::Less => {}
                }
            }

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
