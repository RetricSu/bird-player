use super::AppComponent;
use crate::app::App;
use crate::egui::epaint::*;
use crate::egui::{vec2, ColorImage, TextureHandle};
use ::image::io::Reader as ImageReader;
use eframe::egui::{Rect, Sense};
use log::{error, info, warn};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::time::Instant;

pub struct ScopeComponent;
const ALBUM_ART_SIZE: f32 = 120.0;
const CASSETTE_WIDTH: f32 = 280.0;
const CASSETTE_HEIGHT: f32 = 180.0;
const REEL_RADIUS: f32 = 40.0;
const SPROCKET_HOLES: usize = 6;

thread_local! {
    static LAST_UPDATE: std::cell::RefCell<Instant> = std::cell::RefCell::new(Instant::now());
    static IMAGE_CACHE: std::cell::RefCell<HashMap<PathBuf, TextureHandle>> = std::cell::RefCell::new(HashMap::new());
    static ROTATION_ANGLE: std::cell::RefCell<f32> = const {std::cell::RefCell::new(0.0)};
    static TAPE_PROGRESS: std::cell::RefCell<f32> = const {std::cell::RefCell::new(0.0)};
}

impl AppComponent for ScopeComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.horizontal(|ui| {
            let rect = ui.available_rect_before_wrap().shrink(10.0);
            let rect = Rect::from_min_size(rect.min, vec2(CASSETTE_WIDTH, CASSETTE_HEIGHT));

            ui.allocate_rect(rect, Sense::hover());

            let left_reel_center = rect.left_center() + vec2(REEL_RADIUS + 20.0, 0.0);
            let right_reel_center = rect.right_center() - vec2(REEL_RADIUS + 20.0, 0.0);
            let center_rect = eframe::egui::Rect::from_center_size(
                rect.center(),
                vec2(ALBUM_ART_SIZE, ALBUM_ART_SIZE),
            );

            ui.painter()
                .rect_filled(rect, 8.0, Color32::from_rgb(40, 40, 45));

            ui.painter()
                .rect_filled(rect.shrink(1.0), 7.0, Color32::from_rgb(35, 35, 40));

            ui.painter()
                .rect_filled(center_rect.expand(10.0), 4.0, Color32::from_rgb(25, 25, 30));

            ui.painter()
                .rect_filled(center_rect.expand(9.0), 3.0, Color32::from_rgb(30, 30, 35));

            let hole_radius = 5.0;
            let holes_y = rect.min.y + 15.0;
            let hole1_x = rect.min.x + rect.width() * 0.25;
            let hole2_x = rect.min.x + rect.width() * 0.75;

            ui.painter().circle_filled(
                eframe::egui::pos2(hole1_x, holes_y),
                hole_radius,
                Color32::from_rgb(20, 20, 20),
            );

            ui.painter().circle_filled(
                eframe::egui::pos2(hole2_x, holes_y),
                hole_radius,
                Color32::from_rgb(20, 20, 20),
            );

            let (current_angle, _tape_progress) = update_animation(ctx);

            let current_timestamp = ctx.player.as_ref().unwrap().seek_to_timestamp as f32;
            let duration = ctx.player.as_ref().unwrap().duration as f32;
            let playback_progress = if duration > 0.0 {
                current_timestamp / duration
            } else {
                0.0
            };

            draw_tape(
                ui,
                left_reel_center,
                right_reel_center,
                center_rect,
                playback_progress,
            );

            draw_reel(
                ui,
                left_reel_center,
                current_angle,
                Color32::from_rgb(20, 20, 20),
                1.0 - playback_progress,
            );

            draw_reel(
                ui,
                right_reel_center,
                -current_angle,
                Color32::from_rgb(20, 20, 20),
                playback_progress,
            );

            let mut show_wave_canvas = true;

            if let Some(selected_track) = &ctx.player.as_ref().unwrap().selected_track {
                if let Some(picture) = selected_track.pictures().first() {
                    let path = picture.file_path.clone();

                    show_wave_canvas = !IMAGE_CACHE.with(|cache| {
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
                            let image_rect = center_rect;
                            ui.painter().image(
                                texture.id(),
                                image_rect,
                                eframe::egui::Rect::from_min_max(
                                    eframe::egui::pos2(0.0, 0.0),
                                    eframe::egui::pos2(1.0, 1.0),
                                ),
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

            if show_wave_canvas {
                show_default_album_art(ui, center_rect);
            }
        });
    }
}

fn update_animation(ctx: &mut App) -> (f32, f32) {
    let current_angle = ROTATION_ANGLE.with(|angle| {
        let now = Instant::now();
        let elapsed = LAST_UPDATE.with(|last| {
            let elapsed = now.duration_since(*last.borrow());
            *last.borrow_mut() = now;
            elapsed
        });

        let is_playing = ctx.player.as_ref().unwrap().track_state.to_string() == "Playing";
        let rotation_speed = if is_playing { 2.0 } else { 0.0 };

        *angle.borrow_mut() += rotation_speed * elapsed.as_secs_f32();
        *angle.borrow()
    });

    let tape_progress = TAPE_PROGRESS.with(|progress| {
        if ctx.player.as_ref().unwrap().track_state.to_string() == "Playing" {
            let current_timestamp = ctx.player.as_ref().unwrap().seek_to_timestamp as f32;
            let duration = ctx.player.as_ref().unwrap().duration as f32;

            if duration > 0.0 {
                *progress.borrow_mut() = current_timestamp / duration;
            }
        }
        *progress.borrow()
    });

    (current_angle, tape_progress)
}

fn draw_tape(
    ui: &mut eframe::egui::Ui,
    left_reel_center: eframe::egui::Pos2,
    right_reel_center: eframe::egui::Pos2,
    center_rect: eframe::egui::Rect,
    progress: f32,
) {
    let tape_color = Color32::from_rgb(50, 50, 50);
    let tape_thickness = 4.0;

    let top_left = center_rect.left_top() + vec2(-5.0, 5.0);
    let _bottom_left = center_rect.left_bottom() + vec2(-5.0, -5.0);
    let top_right = center_rect.right_top() + vec2(5.0, 5.0);
    let _bottom_right = center_rect.right_bottom() + vec2(5.0, -5.0);

    ui.painter().line_segment(
        [left_reel_center, top_left],
        Stroke::new(tape_thickness, tape_color),
    );

    ui.painter().line_segment(
        [top_left, top_right],
        Stroke::new(tape_thickness, tape_color),
    );

    ui.painter().line_segment(
        [top_right, right_reel_center],
        Stroke::new(tape_thickness, tape_color),
    );

    let left_amount = 1.0 - progress;
    let right_amount = progress;

    let max_fill_radius = REEL_RADIUS * 0.8;

    if left_amount > 0.05 {
        let left_fill_radius = REEL_RADIUS * 0.3 + max_fill_radius * left_amount;
        ui.painter().circle_filled(
            left_reel_center,
            left_fill_radius,
            Color32::from_rgb(40, 40, 40),
        );
    }

    if right_amount > 0.05 {
        let right_fill_radius = REEL_RADIUS * 0.3 + max_fill_radius * right_amount;
        ui.painter().circle_filled(
            right_reel_center,
            right_fill_radius,
            Color32::from_rgb(40, 40, 40),
        );
    }
}

fn draw_reel(
    ui: &mut eframe::egui::Ui,
    center: eframe::egui::Pos2,
    angle: f32,
    color: Color32,
    _tape_amount: f32,
) {
    ui.painter().circle_filled(center, REEL_RADIUS, color);

    ui.painter().circle_stroke(
        center,
        REEL_RADIUS,
        Stroke::new(2.0, Color32::from_rgb(60, 60, 60)),
    );

    ui.painter()
        .circle_filled(center, REEL_RADIUS * 0.25, Color32::from_rgb(120, 120, 120));

    ui.painter()
        .circle_filled(center, REEL_RADIUS * 0.1, Color32::from_rgb(30, 30, 30));

    for i in 0..SPROCKET_HOLES {
        let spoke_angle = angle + i as f32 * 2.0 * std::f32::consts::PI / SPROCKET_HOLES as f32;
        let spoke_pos = center
            + vec2(
                spoke_angle.cos() * REEL_RADIUS * 0.6,
                spoke_angle.sin() * REEL_RADIUS * 0.6,
            );

        ui.painter()
            .circle_filled(spoke_pos, REEL_RADIUS * 0.12, Color32::from_rgb(80, 80, 80));
    }
}

fn show_default_album_art(ui: &mut eframe::egui::Ui, rect: eframe::egui::Rect) {
    let top_color = Color32::from_rgb(60, 60, 70);
    let _bottom_color = Color32::from_rgb(40, 40, 50);

    ui.painter().rect_filled(rect, 4.0, top_color);

    ui.painter().text(
        rect.center(),
        eframe::egui::Align2::CENTER_CENTER,
        "🎵",
        eframe::egui::FontId::proportional(32.0),
        Color32::WHITE,
    );
}
