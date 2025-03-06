use super::AppComponent;
use crate::app::App;
use crate::egui::epaint::*;
use crate::egui::{pos2, vec2, Frame, Pos2, Rect};
use rb::RbConsumer;

pub struct ScopeComponent;

impl AppComponent for ScopeComponent {
    type Context = App;
    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            ui.ctx().request_repaint();
            let time = ui.input(|i| i.time);
            let base_color = Color32::from_rgb(0, 150, 255);
            let highlight_color = Color32::from_rgb(0, 255, 255);

            let desired_size = ui.available_width() * vec2(1.0, 0.25);
            let (_id, rect) = ui.allocate_space(desired_size);

            let to_screen =
                emath::RectTransform::from_to(Rect::from_x_y_ranges(0.0..=1.0, -1.0..=1.0), rect);
            let mut shapes = vec![];

            if let Some(ref mut scope) = &mut ctx.scope {
                if let Some(audio_buf) = &ctx.played_audio_buffer {
                    if let Some(local_buf) = &mut ctx.temp_buf {
                        let num_bytes_read = audio_buf.read(&mut local_buf[..]).unwrap_or(0);

                        if num_bytes_read > 0 {
                            for sample in (local_buf[0..num_bytes_read]).iter().step_by(2) {
                                scope.write_sample(*sample);
                            }
                        }
                    }
                }

                let points: Vec<Pos2> = scope
                    .into_iter()
                    .enumerate()
                    .map(|(i, sample)| {
                        let wave = (time as f32 * 2.0 + i as f32 * 0.01).sin() * 0.05;
                        let modified_sample = sample + wave;
                        to_screen * pos2(i as f32 / (48000.0 * 1.0), modified_sample)
                    })
                    .collect();

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
            }

            ui.painter().extend(shapes);
        });
    }
}
