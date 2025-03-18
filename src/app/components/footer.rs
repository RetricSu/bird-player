use super::AppComponent;
use crate::app::App;

pub struct Footer;

impl AppComponent for Footer {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            // Playlist operation buttons
            if let Some(current_playlist_idx) = ctx.current_playlist_idx {
                let playlist = &mut ctx.playlists[current_playlist_idx];
                let selection_count = playlist.selected_indices.len();
                let has_tracks = !playlist.tracks.is_empty();

                // Track search state in memory
                let search_active_id = ui.id().with("search_active");
                let search_text_id = ui.id().with("search_text");

                let mut search_active = ui
                    .memory_mut(|mem| mem.data.get_temp::<bool>(search_active_id))
                    .unwrap_or(false);

                let mut search_text = ui
                    .memory_mut(|mem| mem.data.get_temp::<String>(search_text_id))
                    .unwrap_or_default();

                // Search button or search box
                if search_active {
                    // Create a unique ID for the editor to track focus
                    let editor_id = ui.id().with("search_editor");

                    // Request focus on the first frame when search becomes active
                    let is_first_frame_id = ui.id().with("is_first_search_frame");
                    let is_first_frame = ui
                        .memory_mut(|mem| mem.data.get_temp::<bool>(is_first_frame_id))
                        .unwrap_or(true);

                    if is_first_frame {
                        ui.memory_mut(|mem| {
                            mem.request_focus(editor_id);
                            mem.data.insert_temp(is_first_frame_id, false);
                        });
                    }

                    // Define the search results storage type
                    let search_results_id = ui.id().with("search_results");
                    let show_dropdown_id = ui.id().with("show_search_dropdown");

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            // Add the search text field
                            let response = ui.add(
                                eframe::egui::TextEdit::singleline(&mut search_text)
                                    .id(editor_id)
                                    .desired_width(200.0)
                                    .hint_text("Type to search..."),
                            );

                            // Always save the current text value to memory as user types
                            ui.memory_mut(|mem| {
                                mem.data.insert_temp(search_text_id, search_text.clone())
                            });

                            // Add a search button that only triggers when clicked
                            if ui.button("🔍").clicked()
                                || (response.lost_focus()
                                    && ui.input(|i| i.key_pressed(eframe::egui::Key::Enter)))
                            {
                                // Only search if text is not empty
                                if !search_text.is_empty() {
                                    // Clear previous selection
                                    playlist.clear_selection();

                                    // Select items that match the search text
                                    let search_lower = search_text.to_lowercase();
                                    tracing::info!("Searching for: {}", search_lower);

                                    let mut match_count = 0;
                                    let mut search_results: Vec<(usize, String, String, String)> =
                                        Vec::new();

                                    for (idx, track) in playlist.tracks.iter().enumerate() {
                                        let title = track.title().unwrap_or_default();
                                        let artist = track.artist().unwrap_or_default();
                                        let album = track.album().unwrap_or_default();
                                        let genre = track.genre().unwrap_or_default();

                                        let title_lower = title.to_lowercase();
                                        let artist_lower = artist.to_lowercase();
                                        let album_lower = album.to_lowercase();
                                        let genre_lower = genre.to_lowercase();

                                        if title_lower.contains(&search_lower)
                                            || artist_lower.contains(&search_lower)
                                            || album_lower.contains(&search_lower)
                                            || genre_lower.contains(&search_lower)
                                        {
                                            playlist.selected_indices.insert(idx);
                                            match_count += 1;
                                            search_results.push((
                                                idx,
                                                title.clone(),
                                                artist.clone(),
                                                album.clone(),
                                            ));
                                            tracing::info!("Match found: {} ({})", title, idx);
                                        }
                                    }

                                    tracing::info!(
                                        "Search completed. Found {} matches",
                                        match_count
                                    );

                                    // Store the search results in memory
                                    ui.memory_mut(|mem| {
                                        mem.data.insert_temp(search_results_id, search_results);
                                        mem.data.insert_temp(show_dropdown_id, match_count > 0);
                                    });

                                    // Show a message if no matches found
                                    if match_count == 0 {
                                        // Store a "no results" message to display
                                        ui.memory_mut(|mem| {
                                            mem.data.insert_temp(
                                                ui.id().with("search_no_results"),
                                                true,
                                            )
                                        });
                                    } else {
                                        ui.memory_mut(|mem| {
                                            mem.data.insert_temp(
                                                ui.id().with("search_no_results"),
                                                false,
                                            )
                                        });
                                    }
                                }
                            }

                            // Close button to exit search mode
                            if ui.button("✗").clicked() {
                                search_active = false;
                                search_text.clear();
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp(search_text_id, String::new());
                                    mem.data.insert_temp(show_dropdown_id, false);
                                });
                            }
                        });

                        // Get search results from memory and show dropdown if we have results
                        let show_dropdown = ui
                            .memory_mut(|mem| mem.data.get_temp::<bool>(show_dropdown_id))
                            .unwrap_or(false);

                        if show_dropdown {
                            // Retrieve the search results
                            if let Some(results) = ui.memory_mut(|mem| {
                                mem.data.get_temp::<Vec<(usize, String, String, String)>>(
                                    search_results_id,
                                )
                            }) {
                                if !results.is_empty() {
                                    // Container for results with scrolling
                                    eframe::egui::Frame::popup(ui.style())
                                        .stroke(eframe::egui::Stroke::new(
                                            1.0,
                                            ui.style().visuals.widgets.active.bg_fill,
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_max_width(400.0);
                                            ui.set_max_height(200.0);

                                            eframe::egui::ScrollArea::vertical().show(ui, |ui| {
                                                for (idx, title, artist, album) in results {
                                                    let result_text = format!(
                                                        "{} - {} ({})",
                                                        title, artist, album
                                                    );

                                                    // Create a selectable label for each result
                                                    let result_response = ui.selectable_label(
                                                        playlist.is_selected(idx),
                                                        result_text,
                                                    );

                                                    // When clicked, scroll to that track and play it
                                                    if result_response.clicked() {
                                                        // Store the index to scroll to in memory
                                                        ui.memory_mut(|mem| {
                                                            mem.data.insert_temp(
                                                                ui.id().with("scroll_to_idx"),
                                                                idx,
                                                            );
                                                        });

                                                        // Keep only this track selected
                                                        playlist.clear_selection();
                                                        playlist.toggle_selection(idx);

                                                        // Play the clicked track
                                                        let track = playlist.tracks[idx].clone();
                                                        let player = ctx.player.as_mut().unwrap();
                                                        player.select_track(Some(track));
                                                        player.play();

                                                        // Hide the dropdown
                                                        ui.memory_mut(|mem| {
                                                            mem.data.insert_temp(
                                                                show_dropdown_id,
                                                                false,
                                                            );
                                                        });
                                                    }
                                                }
                                            });
                                        });
                                }
                            }
                        }

                        // Show "No results" message if appropriate
                        if ui
                            .memory_mut(|mem| {
                                mem.data.get_temp::<bool>(ui.id().with("search_no_results"))
                            })
                            .unwrap_or(false)
                        {
                            ui.label(
                                eframe::egui::RichText::new("No matches found")
                                    .color(eframe::egui::Color32::RED),
                            );
                        }
                    });
                } else if ui.button("🔍 Search").clicked() {
                    search_active = true;
                    // Reset the first frame flag when search is activated
                    ui.memory_mut(|mem| {
                        mem.data
                            .insert_temp(ui.id().with("is_first_search_frame"), true);
                        // Also clear any previous search text
                        mem.data.insert_temp(search_text_id, String::new());
                        // Hide dropdown
                        mem.data
                            .insert_temp(ui.id().with("show_search_dropdown"), false);
                    });
                }

                // Save search state
                ui.memory_mut(|mem| mem.data.insert_temp(search_active_id, search_active));

                // Select All button
                if ui.button("✓ Select All").clicked() && has_tracks {
                    playlist.select_all();
                }

                // Clear Selection button (disabled if no selection)
                let clear_btn = ui.add_enabled(
                    selection_count > 0,
                    eframe::egui::Button::new("✗ Clear Selection"),
                );
                if clear_btn.clicked() {
                    playlist.clear_selection();
                }

                // Show selection count if any
                if selection_count > 0 {
                    ui.label(format!("{} selected", selection_count));
                }
            }
        });
    }
}
