use crate::app::timed_text::TimedTextCue;
use eframe::egui::{self, Align, Color32, Label, RichText};

pub struct TimedTextPresentation {
    pub align: Align,
    pub font_size: f32,
    pub line_spacing: f32,
    pub current_color: Color32,
    pub weaken_inactive: bool,
    pub empty_cue_text: &'static str,
}

pub struct TimedTextComponent;

impl TimedTextComponent {
    pub fn show(
        ui: &mut egui::Ui,
        cues: &[TimedTextCue],
        current_time_ms: u64,
        presentation: TimedTextPresentation,
    ) {
        for cue in cues {
            let is_current = cue.is_active_at(current_time_ms);
            let text = if cue.text.trim().is_empty() {
                presentation.empty_cue_text
            } else {
                cue.text.as_str()
            };

            let mut rich_text = RichText::new(text).size(presentation.font_size);
            if is_current {
                rich_text = rich_text.color(presentation.current_color).strong();
            } else if presentation.weaken_inactive {
                rich_text = rich_text.weak();
            }

            let response = ui.add(
                Label::new(rich_text)
                    .wrap()
                    .halign(presentation.align)
                    .selectable(false),
            );
            if is_current {
                response.scroll_to_me(Some(Align::Center));
            }
            ui.add_space(presentation.line_spacing);
        }
    }
}
