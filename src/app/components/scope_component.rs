use super::AppComponent;
use crate::app::App;
use crate::egui::epaint::*;
use crate::egui::{vec2, ColorImage, Image, TextureHandle};
use ::image::io::Reader as ImageReader;
use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Instant;

pub struct ScopeComponent;
const ALBUM_ART_SIZE: f32 = 120.0;

thread_local! {
    static LAST_UPDATE: std::cell::RefCell<Instant> = std::cell::RefCell::new(Instant::now());
    static IMAGE_CACHE: std::cell::RefCell<HashMap<PathBuf, TextureHandle>> = std::cell::RefCell::new(HashMap::new());
}

impl AppComponent for ScopeComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            let mut show_wave_canvas = true;

            // Album Art Container
            if let Some(selected_track) = &ctx.player.as_ref().unwrap().selected_track {
                if let Some(picture) = selected_track.pictures().first() {
                    let path = picture.file_path.clone();

                    // Try to get image from cache first
                    show_wave_canvas = !IMAGE_CACHE.with(|cache| {
                        if !cache.borrow().contains_key(&path) {
                            // Load and cache the image if not already cached
                            if let Ok(image_bytes) = std::fs::read(&path) {
                                let reader = ImageReader::new(Cursor::new(image_bytes))
                                    .with_guessed_format();

                                if let Ok(reader) = reader {
                                    if let Ok(img) = reader.decode() {
                                        let rgba_img = img.into_rgba8();
                                        let size = [rgba_img.width() as _, rgba_img.height() as _];
                                        let pixels = rgba_img.into_raw();
                                        let color_image =
                                            ColorImage::from_rgba_unmultiplied(size, &pixels);
                                        let texture = ui.ctx().load_texture(
                                            path.to_str().unwrap_or_default(),
                                            color_image,
                                            Default::default(),
                                        );
                                        info!("Successfully loaded image from: {:?}", path);
                                        cache.borrow_mut().insert(path.clone(), texture);
                                    } else {
                                        error!("Failed to decode image for path: {:?}", path);
                                    }
                                } else {
                                    error!("Failed to guess image format for path: {:?}", path);
                                }
                            } else {
                                error!("Failed to read image file at path: {:?}", path);
                            }
                        }

                        // Get from cache and show
                        if let Some(texture) = cache.borrow().get(&path) {
                            ui.add(
                                Image::new(texture)
                                    .fit_to_original_size(1.0)
                                    .max_size(vec2(ALBUM_ART_SIZE, ALBUM_ART_SIZE)),
                            );
                            true
                        } else {
                            warn!("Image not found in cache for path: {:?}", path);
                            false
                        }
                    });
                }
            }

            if show_wave_canvas {
                // Wave Canvas Code
                show_default_album_art(ui);
            }
        });
    }
}

fn show_default_album_art(ui: &mut eframe::egui::Ui) {
    let (_, art_rect) = ui.allocate_space(vec2(ALBUM_ART_SIZE, ALBUM_ART_SIZE));
    ui.painter()
        .rect_filled(art_rect, 4.0, Color32::from_rgb(50, 50, 50));
    ui.painter().text(
        art_rect.center(),
        eframe::egui::Align2::CENTER_CENTER,
        "🎵",
        eframe::egui::FontId::proportional(32.0),
        Color32::WHITE,
    );
}
