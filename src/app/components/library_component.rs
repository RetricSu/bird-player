use super::AppComponent;
use crate::app::{App, LibraryItem, LibraryPathId};
use eframe::egui::{CollapsingHeader, Label, RichText, Sense};
use std::collections::HashMap;

pub struct LibraryComponent;

impl AppComponent for LibraryComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        // Keep track of paths to remove (if any)
        let mut path_to_remove: Option<LibraryPathId> = None;

        eframe::egui::ScrollArea::both().show(ui, |ui| {
            // Group library items by their library_id (which corresponds to folder paths)
            let mut folder_items: HashMap<LibraryPathId, Vec<&LibraryItem>> = HashMap::new();

            // Collect all library items and group them by path id
            for item in ctx.library.items() {
                folder_items
                    .entry(item.library_id())
                    .or_default()
                    .push(item);
            }

            // Iterate through library paths and display as folders
            for lib_path in ctx.library.paths() {
                if lib_path.status() == crate::app::library::LibraryPathStatus::Imported {
                    let path_id = lib_path.id();
                    let folder_name = lib_path.display_name();

                    // Create a collapsing header for each folder (default open)
                    CollapsingHeader::new(RichText::new(folder_name).strong())
                        .default_open(true)
                        .show(ui, |ui| {
                            // Display each item in the folder
                            if let Some(items) = folder_items.get(&path_id) {
                                // Create a sorted copy for display
                                let mut sorted_items = items.clone();
                                sorted_items.sort_by(|a, b| {
                                    a.title()
                                        .unwrap_or_default()
                                        .cmp(&b.title().unwrap_or_default())
                                });

                                for item in sorted_items {
                                    // Format display with title and artist if available
                                    let display_text = match (item.title(), item.artist()) {
                                        (Some(title), Some(artist)) => {
                                            format!("{} - {}", title, artist)
                                        }
                                        (Some(title), None) => title,
                                        (None, Some(artist)) => {
                                            format!("Unknown Title - {}", artist)
                                        }
                                        (None, None) => "Unknown Track".to_string(),
                                    };

                                    // Create a clickable label for each track
                                    let item_label = ui.add(
                                        Label::new(RichText::new(display_text))
                                            .sense(Sense::click()),
                                    );

                                    // Handle double-click to add to current playlist
                                    if item_label.double_clicked() {
                                        if let Some(current_playlist_idx) =
                                            &ctx.current_playlist_idx
                                        {
                                            let current_playlist =
                                                &mut ctx.playlists[*current_playlist_idx];
                                            if !current_playlist.tracks.contains(item) {
                                                current_playlist.add((*item).clone());
                                            }
                                        }
                                    }

                                    // Add context menu for individual tracks
                                    item_label.context_menu(|ui| {
                                        if ui.button("Add to playlist").clicked() {
                                            if let Some(current_playlist_idx) =
                                                &ctx.current_playlist_idx
                                            {
                                                let current_playlist =
                                                    &mut ctx.playlists[*current_playlist_idx];
                                                if !current_playlist.tracks.contains(item) {
                                                    current_playlist.add((*item).clone());
                                                }
                                                ui.close_menu();
                                            }
                                        }
                                    });
                                }
                            }
                        })
                        .header_response
                        .context_menu(|ui| {
                            // Add context menu for the folder header
                            if ui.button("Add all to playlist").clicked() {
                                if let Some(current_playlist_idx) = &ctx.current_playlist_idx {
                                    let current_playlist =
                                        &mut ctx.playlists[*current_playlist_idx];

                                    // Add all tracks from this folder to the playlist
                                    if let Some(items) = folder_items.get(&path_id) {
                                        for item in items {
                                            if !current_playlist.tracks.contains(item) {
                                                current_playlist.add((*item).clone());
                                            }
                                        }
                                    }
                                    ui.close_menu();
                                }
                            }

                            if ui.button("Remove from library").clicked() {
                                // Mark this path for removal after the loop
                                path_to_remove = Some(path_id);
                                ui.close_menu();
                            }
                        });
                }
            }
        });

        // Process any path removal after rendering the UI
        if let Some(path_id) = path_to_remove {
            ctx.library.remove_path(path_id);
        }
    }
}
