use super::AppComponent;
use crate::app::style::tokens;
use crate::app::App;
use ::image::io::Reader as ImageReader;
use eframe::egui::epaint::*;
use eframe::egui::{vec2, ColorImage, Shape, TextureHandle};
use eframe::egui::{Rect, Sense};
use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;

pub struct CassetteComponent;

const ALBUM_ART_SIZE: f32 = tokens::size::ALBUM;

thread_local! {
    static IMAGE_CACHE: std::cell::RefCell<HashMap<PathBuf, TextureHandle>> = std::cell::RefCell::new(HashMap::new());
}

impl AppComponent for CassetteComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            let rect = ui.available_rect_before_wrap().shrink(10.0);
            // Responsive size — never exceed ALBUM_ART_SIZE, but shrink down
            // when the player panel is short (e.g. on small windows). Keeps
            // the cover square in every layout.
            let side = ALBUM_ART_SIZE
                .min(rect.width())
                .min(rect.height())
                .max(64.0);
            let rect = Rect::from_min_size(rect.min, vec2(side, side));

            ui.allocate_rect(rect, Sense::hover());

            let mut show_default = true;

            if let Some(selected_track) = &ctx.player_ref().selected_track {
                if let Some(picture) = selected_track.pictures().first() {
                    let path = picture.file_path.clone();

                    show_default = !IMAGE_CACHE.with(|cache| {
                        if !cache.borrow().contains_key(&path) {
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

                        if let Some(texture) = cache.borrow().get(&path) {
                            // Calculate UV coordinates for center-cropped fit
                            let image_aspect = texture.size_vec2()[0] / texture.size_vec2()[1];
                            let rect_aspect = rect.width() / rect.height();

                            let (uv_min, uv_max) = if image_aspect > rect_aspect {
                                // Image is wider than display area - crop sides
                                let crop_width = rect_aspect / image_aspect;
                                let offset = (1.0 - crop_width) / 2.0;
                                (
                                    eframe::egui::pos2(offset, 0.0),
                                    eframe::egui::pos2(1.0 - offset, 1.0),
                                )
                            } else {
                                // Image is taller than display area - crop top/bottom
                                let crop_height = image_aspect / rect_aspect;
                                let offset = (1.0 - crop_height) / 2.0;
                                (
                                    eframe::egui::pos2(0.0, offset),
                                    eframe::egui::pos2(1.0, 1.0 - offset),
                                )
                            };

                            // Need to draw image using a meshed shape or just image with rounded corners
                            // Egui doesn't natively support rounded corners on simple image() without a custom shape/mesh
                            // But for simplicity, we just use UI image element with rounding.
                            ui.painter().image(
                                texture.id(),
                                rect,
                                eframe::egui::Rect::from_min_max(uv_min, uv_max),
                                Color32::WHITE,
                            );
                            true
                        } else {
                            warn!("Image not found in cache for path: {:?}", path);
                            false
                        }
                    });
                }
            }

            if show_default {
                show_default_album_art(ctx, ui, rect);
            }
        });
    }
}

fn show_default_album_art(ctx: &App, ui: &mut eframe::egui::Ui, rect: eframe::egui::Rect) {
    let fill_color = if ui.visuals().dark_mode {
        tokens::color::ALBUM_BG_DARK
    } else {
        tokens::color::ALBUM_BG_LIGHT
    };

    let stroke_color = if ui.visuals().dark_mode {
        tokens::color::ALBUM_STROKE_DARK
    } else {
        tokens::color::ALBUM_STROKE_LIGHT
    };

    ui.painter().add(Shape::Rect(RectShape {
        rect,
        corner_radius: tokens::radius::LG.into(),
        fill: fill_color,
        stroke: Stroke::new(tokens::size::STROKE_WIDTH, stroke_color),
        stroke_kind: StrokeKind::Middle,
        round_to_pixels: None,
        blur_width: 0.0,
        brush: None,
    }));

    // Create a vertical layout for the text

    let title_pos = rect.center() - vec2(0.0, 12.0);
    let artist_pos = rect.center() + vec2(0.0, 12.0);

    // Get track information from the player
    if let Some(selected_track) = &ctx.player_ref().selected_track {
        // Calculate maximum text width (80% of rect width to leave some margin)

        let title_font = eframe::egui::FontId::proportional(tokens::text::MD);
        let artist_font = eframe::egui::FontId::proportional(tokens::text::SM);

        // Draw title with truncation
        let title = selected_track
            .title()
            .unwrap_or("Unknown Title".to_string());

        // Use basic text truncation
        ui.painter().text(
            title_pos,
            eframe::egui::Align2::CENTER_CENTER,
            title,
            title_font,
            ui.visuals().text_color(),
        );

        // Draw artist with truncation
        let artist = selected_track
            .artist()
            .unwrap_or("Unknown Artist".to_string());

        ui.painter().text(
            artist_pos,
            eframe::egui::Align2::CENTER_CENTER,
            artist,
            artist_font,
            ui.visuals().text_color(),
        );
    } else {
        ui.painter().text(
            rect.center(),
            eframe::egui::Align2::CENTER_CENTER,
            "No Cover",
            eframe::egui::FontId::proportional(tokens::text::MD),
            ui.visuals().text_color(),
        );
    }
}
