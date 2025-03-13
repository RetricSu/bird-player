use super::AppComponent;
use crate::app::App;
use eframe::egui;

pub struct PlaylistTable;

impl AppComponent for PlaylistTable {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        if let Some(current_playlist_idx) = &mut ctx.current_playlist_idx {
            // Track drag and drop state using egui's memory
            let drag_id = ui.id().with("playlist_drag_source");
            let drop_id = ui.id().with("playlist_drop_target");
            let is_dragging_id = ui.id().with("is_dragging");

            // Track which item to remove (if any)
            let mut track_to_remove: Option<usize> = None;

            // Retrieve state from memory, or initialize if not present
            let dragged_item = ui
                .memory_mut(|mem| mem.data.get_temp::<Option<usize>>(drag_id))
                .unwrap_or(None);
            let mut drop_target = ui
                .memory_mut(|mem| mem.data.get_temp::<Option<usize>>(drop_id))
                .unwrap_or(None);
            let is_dragging = ui
                .memory_mut(|mem| mem.data.get_temp::<bool>(is_dragging_id))
                .unwrap_or(false);

            // Disable text selection when dragging
            if is_dragging {
                ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);

                // Disable text selection during dragging
                ui.style_mut().interaction.selectable_labels = false;
            }

            // Start or continue a drag operation
            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            //let mouse_down = ui.input(|i| i.pointer.primary_down());
            let mouse_released = ui.input(|i| i.pointer.primary_released());

            // Clear drop target if not dragging or mouse is released
            if !is_dragging || mouse_released {
                drop_target = None;
            }

            // Store row heights and positions for drop indicator
            let mut row_rects = Vec::new();

            // Get playlist length once
            let playlist_len = ctx.playlists[*current_playlist_idx].tracks.len();

            // Use a single Grid for all rows (including header) to ensure alignment
            egui::Grid::new("playlist_full")
                .striped(true)
                .spacing([10.0, 5.0])
                .num_columns(6)
                .min_col_width(25.0)
                .show(ui, |ui| {
                    // Header - use empty label for the drag handle column
                    ui.label("#"); // Empty first column (no drag handle)
                    ui.strong("Title");
                    ui.strong("Artist");
                    ui.strong("Album");
                    ui.strong("Genre");
                    ui.end_row();

                    // Playlist items
                    for idx in 0..playlist_len {
                        let is_being_dragged = dragged_item == Some(idx);

                        // Skip rendering the row if it's being dragged (we'll draw it separately)
                        if is_being_dragged && is_dragging {
                            // Add an empty row as a placeholder
                            for _ in 0..6 {
                                ui.label("");
                            }
                            ui.end_row();
                            continue;
                        }

                        // Get the row's rect before we draw anything
                        let row_rect = ui.available_rect_before_wrap();
                        row_rects.push((idx, row_rect));

                        // Get the track for this row
                        let track = &ctx.playlists[*current_playlist_idx].tracks[idx];
                        let track_title = track.title().unwrap_or("unknown title".to_string());
                        let track_artist = track.artist().unwrap_or("unknown artist".to_string());
                        let track_album = track.album().unwrap_or("unknown album".to_string());
                        let track_genre = track.genre().unwrap_or("unknown genre".to_string());
                        let track_clone = track.clone();

                        // First column - Drag handle + playing indicator
                        let drag_handle_text = (idx + 1).to_string();
                        let mut drag_handle_text = egui::RichText::new(drag_handle_text).strong();
                        let mut title_text = egui::RichText::new(track_title);
                        let mut artist_text = egui::RichText::new(track_artist);
                        let mut album_text = egui::RichText::new(track_album);
                        let mut genre_text = egui::RichText::new(track_genre);

                        if let Some(selected_track) = &ctx.player.as_ref().unwrap().selected_track {
                            if selected_track == track {
                                let highlight_color = ui.style().visuals.selection.bg_fill;

                                // Highlight the row in blue when it's the currently playing track
                                drag_handle_text = drag_handle_text.color(highlight_color);
                                title_text = title_text.color(highlight_color);
                                artist_text = artist_text.color(highlight_color);
                                album_text = album_text.color(highlight_color);
                                genre_text = genre_text.color(highlight_color);
                            }
                        }

                        // Disable text selection on drag handle
                        let mut drag_handle = drag_handle_text;
                        if is_dragging {
                            drag_handle = drag_handle.color(egui::Color32::from_rgb(120, 120, 180));
                        }

                        let drag_handle_response = ui.add(
                            egui::Label::new(drag_handle).sense(egui::Sense::click_and_drag()),
                        );

                        // Show grab cursor only when hovering over the drag handle
                        if drag_handle_response.hovered() && !is_dragging {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grab);
                        }

                        // Detect drag start from handle
                        if drag_handle_response.dragged() && dragged_item.is_none() {
                            ui.memory_mut(|mem| {
                                mem.data.insert_temp(drag_id, Some(idx));
                                mem.data.insert_temp(is_dragging_id, true);
                            });
                        }

                        // Title (clickable) - prevent selection during drag
                        let title_response =
                            ui.add(egui::Label::new(title_text).sense(egui::Sense::click()));

                        // Show pointing hand cursor when hovering over the title (only when not dragging)
                        if title_response.hovered() && !is_dragging {
                            ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
                        }

                        // Add context menu for the title
                        title_response.context_menu(|ui| {
                            if ui.button("Remove from playlist").clicked() {
                                track_to_remove = Some(idx);
                                ui.close_menu();
                            }
                        });

                        // Artist
                        ui.label(artist_text);

                        // Album
                        ui.label(album_text);

                        // Genre
                        ui.label(genre_text);

                        ui.end_row();

                        // Handle click to play/stop track (don't respond to clicks during dragging)
                        if title_response.clicked() && !is_dragging {
                            if ctx.player.as_ref().unwrap().selected_track
                                != Some(track_clone.clone())
                            {
                                ctx.player.as_mut().unwrap().selected_track =
                                    Some(track_clone.clone());
                                ctx.player.as_mut().unwrap().select_track(Some(track_clone));
                                ctx.player.as_mut().unwrap().play();
                            } else {
                                ctx.player.as_mut().unwrap().stop();
                                ctx.player.as_mut().unwrap().selected_track = None;
                            }
                        }
                    }
                });

            // Handle track removal after the iteration is complete
            if let Some(idx) = track_to_remove {
                if idx < ctx.playlists[*current_playlist_idx].tracks.len() {
                    ctx.playlists[*current_playlist_idx].tracks.remove(idx);
                }
            }

            // Draw drag indicator and drop line if dragging
            if is_dragging && dragged_item.is_some() && pointer_pos.is_some() {
                // Find the nearest row for drop target
                if let Some(pos) = pointer_pos {
                    // Sort rows by distance to cursor
                    let mut sorted_rows = row_rects.clone();
                    sorted_rows.sort_by(|(_, rect_a), (_, rect_b)| {
                        let dist_a = (rect_a.center().y - pos.y).abs();
                        let dist_b = (rect_b.center().y - pos.y).abs();
                        dist_a.partial_cmp(&dist_b).unwrap()
                    });

                    // Find the nearest row that's not the dragged row
                    let nearest_row = sorted_rows
                        .iter()
                        .find(|(idx, _)| Some(*idx) != dragged_item)
                        .map(|(idx, _)| *idx);

                    if let Some(idx) = nearest_row {
                        // Update drop target
                        drop_target = Some(idx);
                        ui.memory_mut(|mem| mem.data.insert_temp(drop_id, drop_target));

                        // Find the rect for the drop target
                        let drop_rect = row_rects
                            .iter()
                            .find(|(i, _)| *i == idx)
                            .map(|(_, rect)| *rect);

                        // Draw a drop line
                        if let Some(rect) = drop_rect {
                            // Determine if cursor is in top or bottom half of row
                            let insert_above = pos.y < rect.center().y;
                            let line_y = if insert_above { rect.min.y } else { rect.max.y };

                            // Draw an insertion line
                            let line_rect = egui::Rect::from_min_max(
                                egui::pos2(rect.min.x, line_y - 1.0),
                                egui::pos2(rect.max.x, line_y + 1.0),
                            );
                            ui.painter().rect_filled(
                                line_rect,
                                0.0,
                                egui::Color32::from_rgb(50, 150, 250),
                            );
                        }
                    }
                }

                // Draw the dragged row near the cursor
                if let (Some(drag_idx), Some(pos)) = (dragged_item, pointer_pos) {
                    // Get the track data
                    let track = &ctx.playlists[*current_playlist_idx].tracks[drag_idx];

                    // Create a floating row that follows the cursor
                    let rect_height = 24.0;
                    let rect_width = 400.0;
                    let drag_rect = egui::Rect::from_min_max(
                        egui::pos2(pos.x - 10.0, pos.y - rect_height / 2.0),
                        egui::pos2(pos.x + rect_width, pos.y + rect_height / 2.0),
                    );

                    // Draw a semi-transparent background
                    ui.painter().rect_filled(
                        drag_rect,
                        4.0,
                        egui::Color32::from_rgba_premultiplied(100, 100, 180, 200),
                    );

                    // Show track title in the floating indicator
                    let drag_text = track
                        .title()
                        .unwrap_or("unknown title".to_string())
                        .to_string();
                    ui.painter().text(
                        drag_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        drag_text,
                        egui::FontId::default(),
                        egui::Color32::WHITE,
                    );
                }
            }

            // Handle drag end and reordering
            if mouse_released && is_dragging {
                if let (Some(drag_idx), Some(drop_idx)) = (dragged_item, drop_target) {
                    if drag_idx != drop_idx {
                        // Determine if we should insert before or after the drop target
                        let offset = if let Some(pos) = pointer_pos {
                            let drop_rect = row_rects
                                .iter()
                                .find(|(i, _)| *i == drop_idx)
                                .map(|(_, rect)| *rect);

                            if let Some(rect) = drop_rect {
                                // Insert before if cursor is in top half of row
                                pos.y < rect.center().y
                            } else {
                                false
                            }
                        } else {
                            false
                        };

                        // Calculate actual drop position
                        let target_pos = if offset {
                            // Before the drop target
                            if drop_idx < drag_idx {
                                drop_idx
                            } else {
                                drop_idx.saturating_sub(1)
                            }
                        } else {
                            // After the drop target
                            if drop_idx > drag_idx {
                                drop_idx
                            } else {
                                drop_idx.saturating_add(1).min(playlist_len - 1)
                            }
                        };

                        // Reorder the playlist
                        ctx.playlists[*current_playlist_idx].reorder(drag_idx, target_pos);
                    }
                }

                // Clear drag state
                ui.memory_mut(|mem| {
                    mem.data.insert_temp::<Option<usize>>(drag_id, None);
                    mem.data.insert_temp::<Option<usize>>(drop_id, None);
                    mem.data.insert_temp::<bool>(is_dragging_id, false);
                });
            }
        }
    }
}
