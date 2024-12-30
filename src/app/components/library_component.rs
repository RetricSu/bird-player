use super::AppComponent;
use crate::app::library::{LibraryItemContainer, ViewType};
use crate::app::{App, LibraryItem, LibraryView};
use itertools::Itertools;

use id3::{Tag, TagLike};
use rayon::prelude::*;

pub struct LibraryComponent;

impl AppComponent for LibraryComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        eframe::egui::ScrollArea::both().show(ui, |ui| {
            if ui.button("Add Library path").clicked() {
                if let Some(lib_path) = rfd::FileDialog::new().pick_folder() {
                    tracing::info!("adding library path...");

                    // TODO - if the insert into the library failed, skip all the file import logic.
                    if *&ctx.library.add_path(lib_path) {
                        let lib_item_tx = ctx.library_item_tx.as_ref().unwrap().clone();
                        let lib_view_tx = ctx.library_view_tx.as_ref().unwrap().clone();
                        let lib_paths = ctx.library.paths().clone();

                        std::thread::spawn(move || {
                            // Collect all files from each path in the library
                            let files: Vec<walkdir::DirEntry> = lib_paths
                                .iter()
                                .flat_map(|lib_path| {
                                    walkdir::WalkDir::new(&lib_path)
                                        .into_iter()
                                        .filter_map(|e| e.ok())
                                        .skip(1)
                                        .filter(|entry| entry.file_type().is_file())
                                        .collect::<Vec<_>>()
                                })
                                .collect();

                            let items = files
                                .par_iter()
                                .map(|entry| {
                                    let tag = Tag::read_from_path(&entry.path());

                                    let library_item = match tag {
                                        Ok(tag) => LibraryItem::new(entry.path().to_path_buf())
                                            .set_title(tag.title())
                                            .set_artist(tag.artist())
                                            .set_album(tag.album())
                                            .set_year(tag.year())
                                            .set_genre(tag.genre())
                                            .set_track_number(tag.track()),
                                        Err(_err) => {
                                            tracing::warn!(
                                                "Couldn't parse to id3: {:?}",
                                                &entry.path()
                                            );
                                            LibraryItem::new(entry.path().to_path_buf())
                                        }
                                    };

                                    return library_item;
                                })
                                .collect::<Vec<LibraryItem>>();

                            tracing::info!("Done parsing library items");

                            // Populate the library
                            for item in &items {
                                lib_item_tx
                                    .send((*item).clone())
                                    .expect("failed to send library item")
                            }

                            // Build the views
                            let mut library_view = LibraryView {
                                view_type: ViewType::Album,
                                containers: Vec::new(),
                            };

                            // In order for group by to work from itertools, items must be consecutive, so sort them first.
                            let mut library_items_clone = items.clone();
                            library_items_clone.sort_by_key(|item| item.album());

                            let grouped_library_by_album =
                                &library_items_clone.into_iter().group_by(|item| {
                                    item.album().unwrap_or("<?>".to_string()).to_string()
                                });

                            for (album_name, album_library_items) in grouped_library_by_album {
                                let lib_item_container = LibraryItemContainer {
                                    name: album_name.clone(),
                                    items: album_library_items
                                        .map(|item| item.clone())
                                        .collect::<Vec<LibraryItem>>(),
                                };

                                library_view.containers.push(lib_item_container.clone());
                            }

                            //new_library.add_view(library_view);
                            lib_view_tx.send(library_view).expect("Failed to send")
                        });
                    }
                }
            }

            // TODO - Figure out how the library should actually be displayed
            /*
            let root_path_string = &library
                .root_path()
                .clone()
                .into_os_string()
                .into_string()
                .unwrap();
            */

            eframe::egui::CollapsingHeader::new(eframe::egui::RichText::new("All Music"))
                .default_open(true)
                .show(ui, |ui| {
                    for container in &ctx.library.view().containers {
                        let items = &container.items;

                        let library_group = eframe::egui::CollapsingHeader::new(
                            eframe::egui::RichText::new(&container.name),
                        )
                        .default_open(false)
                        .show(ui, |ui| {
                            for item in &container.items {
                                let item_label = ui.add(
                                    eframe::egui::Label::new(eframe::egui::RichText::new(
                                        item.title().unwrap_or("?".to_string()),
                                    ))
                                    .sense(eframe::egui::Sense::click()),
                                );

                                if item_label.double_clicked() {
                                    if let Some(current_playlist_idx) = &ctx.current_playlist_idx {
                                        let current_playlist =
                                            &mut ctx.playlists[*current_playlist_idx];

                                        current_playlist.add(item.clone());
                                    }
                                }
                            }
                        });

                        if let Some(current_playlist_idx) = &ctx.current_playlist_idx {
                            let current_playlist = &mut ctx.playlists[*current_playlist_idx];

                            if library_group.header_response.double_clicked() {
                                for item in items {
                                    current_playlist.add(item.clone());
                                }
                            }
                        }
                    }
                });
        });
    }
}
