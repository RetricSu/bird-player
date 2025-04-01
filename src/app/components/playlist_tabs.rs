use super::AppComponent;
use crate::app::t;
use crate::app::{playlist, App, Playlist};
use eframe::egui;

pub struct PlaylistTabs;

impl AppComponent for PlaylistTabs {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // Add playlist tabs
            for (idx, playlist) in ctx.playlists.iter_mut().enumerate() {
                let is_selected = ctx.current_playlist_idx == Some(idx);
                let is_being_renamed = ctx.playlist_being_renamed == Some(idx);

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
                        ctx.playlist_being_renamed = None;
                    }
                } else {
                    // Show normal tab button
                    let mut tab_text =
                        egui::RichText::new(playlist.get_name().unwrap_or_default()).size(12.0);
                    if is_selected {
                        tab_text = tab_text.strong();
                    }

                    let tab_response = ui.add(egui::Button::new(tab_text).fill(if is_selected {
                        ui.style().visuals.selection.bg_fill
                    } else {
                        ui.style().visuals.widgets.inactive.bg_fill
                    }));

                    if tab_response.clicked() {
                        ctx.current_playlist_idx = Some(idx);
                    }

                    // Show context menu on right-click
                    tab_response.context_menu(|ui| {
                        if ui.button(t("rename")).clicked() {
                            ctx.playlist_being_renamed = Some(idx);
                            ui.close_menu();
                        }
                        if ui.button(t("delete")).clicked() {
                            ctx.playlist_idx_to_remove = Some(idx);
                            ui.close_menu();
                        }
                    });
                }
            }

            // Add the "+" button for creating new playlists
            let create_btn = ui.add(egui::Button::new(egui::RichText::new("+").size(12.0)));

            if create_btn.clicked() {
                let mut new_playlist = Playlist::new();
                new_playlist.set_name(t("new_playlist")); // Set a default name
                ctx.playlists.push(new_playlist);
                let new_idx = ctx.playlists.len() - 1;
                ctx.current_playlist_idx = Some(new_idx);
                ctx.playlist_being_renamed = Some(new_idx); // Start renaming the new playlist immediately
            }

            // Handle playlist removal
            if let Some(idx) = ctx.playlist_idx_to_remove {
                ctx.playlist_idx_to_remove = None;

                // Delete from database if the playlist has an ID
                if let Some(ref db) = ctx.database {
                    if let Some(playlist_id) = ctx.playlists[idx].id {
                        if let Err(e) =
                            playlist::Playlist::delete_from_db(&db.connection(), playlist_id)
                        {
                            tracing::error!("Failed to delete playlist from database: {}", e);
                        }
                    }
                }

                // Update current playlist index before removing
                if let Some(current_playlist_idx) = ctx.current_playlist_idx {
                    match current_playlist_idx.cmp(&idx) {
                        std::cmp::Ordering::Equal => {
                            // If we're removing the current playlist, select the next one or previous one
                            if idx < ctx.playlists.len() - 1 {
                                ctx.current_playlist_idx = Some(idx);
                            } else if idx > 0 {
                                ctx.current_playlist_idx = Some(idx - 1);
                            } else {
                                ctx.current_playlist_idx = None;
                            }
                        }
                        std::cmp::Ordering::Greater => {
                            // If we're removing a playlist before the current one, adjust the index
                            ctx.current_playlist_idx = Some(current_playlist_idx - 1);
                        }
                        std::cmp::Ordering::Less => {
                            // No need to adjust if removing a playlist after the current one
                        }
                    }
                }

                // Remove the playlist
                ctx.playlists.remove(idx);
            }
        });
    }
}
