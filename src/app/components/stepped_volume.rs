use crate::app::style::tokens;
use crate::app::t;
use eframe::egui::{pos2, vec2, Color32, CursorIcon, Rect, Response, Sense, Ui, Widget};

/// Discrete staircase-style volume control widget.
///
/// Replaces the continuous rail slider with a stepped bar indicator
/// reminiscent of classic 2000s desktop audio players and hi-fi displays.
/// Supports clicking, horizontal dragging across steps, and mouse wheel adjustments.
pub struct SteppedVolumeSlider<'a> {
    volume: &'a mut f32,
    muted: bool,
}

impl<'a> SteppedVolumeSlider<'a> {
    pub fn new(volume: &'a mut f32, muted: bool) -> Self {
        Self { volume, muted }
    }
}

/// Calculate discrete 1-based step (0..=total_steps) from relative horizontal cursor position.
pub fn calculate_step(rel_x: f32, bar_width: f32, stride: f32, total_steps: usize) -> usize {
    if total_steps == 0 || rel_x < 0.0 {
        return 0;
    }
    let total_bars_w = (total_steps - 1) as f32 * stride + bar_width;
    if rel_x >= total_bars_w {
        return total_steps;
    }
    let step = ((rel_x - bar_width * 0.5) / stride).round() as isize + 1;
    (step.clamp(1, total_steps as isize)) as usize
}

/// Determine how many bars should be highlighted for a given volume level and mute state.
pub fn active_bars_count(volume: f32, muted: bool, total_steps: usize) -> usize {
    if muted || volume <= 0.001 {
        0
    } else {
        ((volume * total_steps as f32).round() as usize).clamp(1, total_steps)
    }
}

/// Calculate the height of the bar at the given index.
pub fn bar_height(index: usize, total_steps: usize, min_height: f32, max_height: f32) -> f32 {
    if total_steps <= 1 {
        return max_height;
    }
    min_height + (max_height - min_height) * (index as f32 / (total_steps - 1) as f32)
}

impl<'a> Widget for SteppedVolumeSlider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let total_steps = tokens::size::VOLUME_STEPS;
        let bar_width = tokens::size::VOLUME_BAR_WIDTH;
        let bar_gap = tokens::size::VOLUME_BAR_GAP;
        let stride = bar_width + bar_gap;
        let min_h = tokens::size::VOLUME_MIN_HEIGHT;
        let max_h = tokens::size::VOLUME_MAX_HEIGHT;

        let desired_size = vec2(
            tokens::size::SLIDER_VOLUME,
            tokens::size::SLIDER_VOLUME_HEIGHT,
        );
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        let total_bars_w = total_steps as f32 * bar_width + (total_steps - 1) as f32 * bar_gap;
        let first_bar_left = rect.min.x + (rect.width() - total_bars_w).max(0.0) * 0.5;
        let baseline_y = rect.max.y - 2.5;

        // Pointer click / drag interaction
        let pointer_pos = response
            .interact_pointer_pos()
            .or_else(|| ui.input(|i| i.pointer.interact_pos()))
            .or_else(|| ui.input(|i| i.pointer.hover_pos()));

        if response.clicked() || response.dragged() {
            if let Some(pos) = pointer_pos {
                let rel_x = pos.x - first_bar_left;
                let step = calculate_step(rel_x, bar_width, stride, total_steps);
                let new_volume = step as f32 / total_steps as f32;
                if (new_volume - *self.volume).abs() > 0.0001 {
                    *self.volume = new_volume;
                    response.mark_changed();
                }
            }
        }

        // Mouse wheel interaction when hovered
        if response.hovered() {
            let scroll_y = ui.input(|i| {
                if i.raw_scroll_delta.y != 0.0 {
                    i.raw_scroll_delta.y
                } else {
                    i.smooth_scroll_delta.y
                }
            });
            if scroll_y.abs() > 0.0 {
                let step_change = if scroll_y > 0.0 { 1 } else { -1 };
                let current_step = (*self.volume * total_steps as f32).round() as isize;
                let new_step = (current_step + step_change).clamp(0, total_steps as isize) as usize;
                let new_vol = new_step as f32 / total_steps as f32;
                if (new_vol - *self.volume).abs() > 0.0001 {
                    *self.volume = new_vol;
                    response.mark_changed();
                }
            }
        }

        // Visual feedback & hover tooltip
        response = response.on_hover_cursor(CursorIcon::PointingHand);
        let percentage = (*self.volume * 100.0).round() as i32;
        response = response.on_hover_text(format!("{}: {}%", t("volume"), percentage));

        // Render stepped bars
        if ui.is_rect_visible(rect) {
            let active_bars = active_bars_count(*self.volume, self.muted, total_steps);
            let is_hovered = response.hovered();
            let is_dragging = response.dragged();

            let hover_step = if is_hovered && !is_dragging {
                ui.input(|i| i.pointer.hover_pos()).map(|pos| {
                    let rel_x = pos.x - first_bar_left;
                    calculate_step(rel_x, bar_width, stride, total_steps)
                })
            } else {
                None
            };

            let is_dark = ui.visuals().dark_mode;
            let inactive_color = if is_dark {
                Color32::from_rgba_unmultiplied(255, 255, 255, 38)
            } else {
                Color32::from_rgba_unmultiplied(0, 0, 0, 32)
            };
            let inactive_hover_color = if is_dark {
                Color32::from_rgba_unmultiplied(255, 255, 255, 55)
            } else {
                Color32::from_rgba_unmultiplied(0, 0, 0, 48)
            };

            for i in 0..total_steps {
                let x0 = first_bar_left + i as f32 * stride;
                let x1 = x0 + bar_width;
                let h = bar_height(i, total_steps, min_h, max_h);
                let y0 = baseline_y - h;
                let y1 = baseline_y;
                let bar_rect = Rect::from_min_max(pos2(x0, y0), pos2(x1, y1));

                let color = if i < active_bars {
                    if is_dragging {
                        tokens::color::BRAND_ACTIVE
                    } else if is_hovered {
                        tokens::color::BRAND_HOVER
                    } else {
                        tokens::color::BRAND
                    }
                } else if let Some(h) = hover_step {
                    if i < h {
                        tokens::color::BRAND.gamma_multiply(0.35)
                    } else if is_hovered {
                        inactive_hover_color
                    } else {
                        inactive_color
                    }
                } else if is_hovered {
                    inactive_hover_color
                } else {
                    inactive_color
                };

                ui.painter().rect_filled(bar_rect, 1.5, color);
            }
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_step_boundaries() {
        let total = 16;
        let w = 5.0;
        let stride = 7.5;

        // Before bar 0
        assert_eq!(calculate_step(-5.0, w, stride, total), 0);
        assert_eq!(calculate_step(-1.0, w, stride, total), 0);

        // Bar 0
        assert_eq!(calculate_step(0.0, w, stride, total), 1);
        assert_eq!(calculate_step(2.5, w, stride, total), 1);
        assert_eq!(calculate_step(4.9, w, stride, total), 1);

        // Bar 1
        assert_eq!(calculate_step(7.5, w, stride, total), 2);
        assert_eq!(calculate_step(10.0, w, stride, total), 2);

        // Last bar (index 15, bar 16)
        let last_x = 15.0 * stride;
        assert_eq!(calculate_step(last_x, w, stride, total), 16);
        assert_eq!(calculate_step(last_x + 2.5, w, stride, total), 16);

        // Beyond last bar
        assert_eq!(calculate_step(last_x + 10.0, w, stride, total), 16);
        assert_eq!(calculate_step(200.0, w, stride, total), 16);
        // Midpoint boundary between bar 0 and bar 1:
        // bar 0 is [0.0, 5.0], bar 1 is [7.5, 12.5], midpoint is 6.25
        assert_eq!(calculate_step(6.24, w, stride, total), 1);
        assert_eq!(calculate_step(6.26, w, stride, total), 2);

        // Zero total steps
        assert_eq!(calculate_step(10.0, w, stride, 0), 0);
    }

    #[test]
    fn test_active_bars_count() {
        let total = 16;
        assert_eq!(active_bars_count(0.0, false, total), 0);
        assert_eq!(active_bars_count(-0.5, false, total), 0);
        assert_eq!(active_bars_count(0.0005, false, total), 0);
        assert_eq!(active_bars_count(0.01, false, total), 1);
        assert_eq!(active_bars_count(0.5, false, total), 8);
        assert_eq!(active_bars_count(1.0, false, total), 16);
        assert_eq!(active_bars_count(1.5, false, total), 16);

        // Muted should always return 0
        assert_eq!(active_bars_count(0.5, true, total), 0);
        assert_eq!(active_bars_count(1.0, true, total), 0);
    }

    #[test]
    fn test_bar_height_progression() {
        let total = 16;
        let min_h = 4.0;
        let max_h = 19.0;

        assert_eq!(bar_height(0, total, min_h, max_h), 4.0);
        assert_eq!(bar_height(15, total, min_h, max_h), 19.0);

        // Strictly increasing
        for i in 0..15 {
            let h1 = bar_height(i, total, min_h, max_h);
            let h2 = bar_height(i + 1, total, min_h, max_h);
            assert!(h2 > h1);
            // Exactly 1.0 px per step
            assert!((h2 - h1 - 1.0).abs() < 1e-5);
        }
    }
}
