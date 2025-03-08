use super::AppComponent;
use crate::app::App;
use crate::egui::epaint::*;
use crate::egui::{pos2, vec2, Frame, Pos2, Rect};
use rb::RbConsumer;
use std::time::Instant;

pub struct ScopeComponent;

const SAMPLES_TO_DISPLAY: usize = 2000; // Increased for smoother visualization
const REPAINT_EVERY_MS: u128 = 16; // ~60 FPS
const WAVE_SPEED: f32 = 0.15; // Much slower wave animation (reduced from 0.5)
const WAVE_AMPLITUDE: f32 = 0.08; // Larger wave height (increased from 0.02)
const SAMPLE_SMOOTHING: usize = 4; // Number of samples to average together

thread_local! {
    static LAST_UPDATE: std::cell::RefCell<Instant> = std::cell::RefCell::new(Instant::now());
}

impl AppComponent for ScopeComponent {
    type Context = App;
    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
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

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);

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
    }
}
