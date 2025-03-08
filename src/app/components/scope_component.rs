use super::AppComponent;
use crate::app::App;
use crate::egui::epaint::*;
use crate::egui::{pos2, vec2, ColorImage, Frame, Image, Pos2, Rect, TextureHandle};
use ::image::io::Reader as ImageReader;
use log::{error, info, warn};
use rb::RbConsumer;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Instant;

pub struct ScopeComponent;

const SAMPLES_TO_DISPLAY: usize = 2000; // Increased for smoother visualization
const REPAINT_EVERY_MS: u128 = 16; // ~60 FPS
const WAVE_SPEED: f32 = 0.15; // Much slower wave animation (reduced from 0.5)
const WAVE_AMPLITUDE: f32 = 0.08; // Larger wave height (increased from 0.02)
const SAMPLE_SMOOTHING: usize = 4; // Number of samples to average together
const ALBUM_ART_SIZE: f32 = 120.0;

thread_local! {
    static LAST_UPDATE: std::cell::RefCell<Instant> = std::cell::RefCell::new(Instant::now());
    static IMAGE_CACHE: std::cell::RefCell<HashMap<PathBuf, TextureHandle>> = std::cell::RefCell::new(HashMap::new());
}

impl AppComponent for ScopeComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            // Album Art Container
            if let Some(selected_track) = &ctx.player.as_ref().unwrap().selected_track {
                if let Some(picture) = selected_track.pictures().first() {
                    let path = picture.file_path.clone();

                    // Try to get image from cache first
                    let show_image = IMAGE_CACHE.with(|cache| {
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

                    if !show_image {
                        show_default_album_art(ui);
                    }
                } else {
                    show_default_album_art(ui);
                }
            }

            ui.add_space(8.0);

            // Original Canvas Code - Untouched
            Frame::canvas(ui.style()).show(ui, |ui| {
                // Only repaint at specified interval
                let now = Instant::now();
                let should_update = LAST_UPDATE.with(|last| {
                    let elapsed = now.duration_since(*last.borrow()).as_millis();
                    if elapsed >= REPAINT_EVERY_MS {
                        *last.borrow_mut() = now;
                        true
                    } else {
                        false
                    }
                });

                if should_update {
                    ui.ctx().request_repaint();
                }

                let time = ui.input(|i| i.time);
                let base_color = Color32::from_rgb(0, 150, 255);
                let highlight_color = Color32::from_rgb(0, 255, 255);

                let desired_size = vec2(ui.available_width() * 0.25, 120.0);
                let (_id, rect) = ui.allocate_space(desired_size);

                let to_screen = emath::RectTransform::from_to(
                    Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0),
                    rect,
                );

                if let Some(ref mut scope) = &mut ctx.scope {
                    if let Some(audio_buf) = &ctx.played_audio_buffer {
                        if let Some(local_buf) = &mut ctx.temp_buf {
                            let num_bytes_read = audio_buf.read(&mut local_buf[..]).unwrap_or(0);

                            if num_bytes_read > 0 {
                                // Process samples with averaging for smoother visualization
                                let chunk_size = SAMPLE_SMOOTHING;
                                for chunk in local_buf[0..num_bytes_read].chunks(chunk_size) {
                                    let avg_sample = chunk.iter().sum::<f32>() / chunk.len() as f32;
                                    scope.write_sample(avg_sample);
                                }
                            }
                        }
                    }

                    // Slower wave effect with phase shift
                    let primary_wave = (time as f32 * WAVE_SPEED).sin() * WAVE_AMPLITUDE;
                    let secondary_wave =
                        (time as f32 * WAVE_SPEED * 0.5).cos() * (WAVE_AMPLITUDE * 0.3);
                    let wave_effect = primary_wave + secondary_wave;

                    // Create points array with interpolation for smoother curves
                    let points: Vec<Pos2> = scope
                        .into_iter()
                        .take(SAMPLES_TO_DISPLAY)
                        .enumerate()
                        .map(|(i, sample)| {
                            let x = i as f32 / SAMPLES_TO_DISPLAY as f32;
                            // Gentle fade out of wave effect from left to right
                            let wave_fade = 1.0 - (x * x * 0.7); // Quadratic fade for smoother transition
                            let modified_sample = sample + (wave_effect * wave_fade);
                            to_screen * pos2(x, modified_sample)
                        })
                        .collect();

                    let mut shapes = Vec::with_capacity(4);

                    // Generate shapes with different thicknesses and alpha values
                    for (i, offset) in [0.0, 0.5, 1.0, 1.5].iter().enumerate() {
                        let alpha = 255 - (i as u8 * 40);
                        let thickness = 1.0 + offset;

                        let color = if i == 0 {
                            highlight_color
                        } else {
                            Color32::from_rgba_premultiplied(
                                base_color.r(),
                                base_color.g(),
                                base_color.b(),
                                alpha,
                            )
                        };

                        shapes.push(Shape::line(points.clone(), Stroke::new(thickness, color)));
                    }

                    ui.painter().extend(shapes);
                }
            });
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
