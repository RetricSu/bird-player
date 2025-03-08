use super::AppComponent;
use crate::app::{App, Playlist};
use eframe::egui;

pub struct PlaylistTabs;

impl AppComponent for PlaylistTabs {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            // Add playlist tabs
            for (idx, playlist) in ctx.playlists.iter().enumerate() {
                let is_selected = ctx.current_playlist_idx == Some(idx);

                // Create a visually distinct tab
                let mut tab_text = egui::RichText::new(playlist.get_name().unwrap_or_default());
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

                // Right-click to delete playlist
                if tab_response.secondary_clicked() {
                    ctx.playlist_idx_to_remove = Some(idx);
                }
            }

            // Add the "+" button for creating new playlists
            let create_btn = ui.add(
                egui::Button::new(egui::RichText::new("+").size(16.0))
                    .min_size(egui::vec2(24.0, 24.0)),
            );

            if create_btn.clicked() {
                let default_name_count = ctx
                    .playlists
                    .iter()
                    .filter(|pl| pl.get_name().unwrap().starts_with("New Playlist"))
                    .count();
                let playlist_name = match default_name_count {
                    0 => "New Playlist".to_string(),
                    _ => format!("New Playlist ({})", default_name_count),
                };

                let mut new_playlist = Playlist::new();
                new_playlist.set_name(playlist_name);
                ctx.playlists.push(new_playlist);
                ctx.current_playlist_idx = Some(ctx.playlists.len() - 1);
            }

            // Handle playlist removal
            if let Some(idx) = ctx.playlist_idx_to_remove {
                ctx.playlist_idx_to_remove = None;

                if let Some(mut current_playlist_idx) = ctx.current_playlist_idx {
                    if current_playlist_idx == 0 && idx == 0 {
                        ctx.current_playlist_idx = None;
                    } else if current_playlist_idx >= idx {
                        current_playlist_idx -= 1;
                        ctx.current_playlist_idx = Some(current_playlist_idx);
                    }
                }

                ctx.playlists.remove(idx);
            }
        });
    }
}
