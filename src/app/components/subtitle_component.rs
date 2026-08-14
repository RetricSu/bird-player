use super::timed_text_component::{TimedTextComponent, TimedTextPresentation};
use crate::app::lyrics::Lyrics;
use crate::app::style::tokens;
use crate::app::timed_text::TimedTextCue;
use crate::app::App;
use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, Align, RichText};

pub struct SubtitleComponent;

enum PanelContent<'a> {
    Timed(&'a [TimedTextCue]),
    Plain(&'a str),
    Instrumental,
}

impl SubtitleComponent {
    pub fn add(ctx: &App, ui: &mut egui::Ui) {
        let current_time_ms = ctx
            .runtime
            .as_ref()
            .map_or(0, |_| ctx.player_ref().seek_to_timestamp);
        let content = Self::content(ctx);

        ui.add_space(tokens::spacing::XS);
        if let Some(content) = content {
            egui::ScrollArea::vertical()
                .id_salt("playback_subtitle_scroll")
                .auto_shrink([false, false])
                .min_scrolled_height(tokens::size::SUBTITLE_VIEWPORT_HEIGHT)
                .max_height(tokens::size::SUBTITLE_VIEWPORT_HEIGHT)
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    ui.with_layout(egui::Layout::top_down(Align::Min), |ui| {
                        Self::show_content(ui, content, current_time_ms);
                    });
                });
        }
    }

    fn content(ctx: &App) -> Option<PanelContent<'_>> {
        Self::select_content(
            ctx.subtitle_manager()
                .current_track()
                .map(|track| track.cues.as_slice()),
            ctx.lyrics_manager().current_lyrics(),
        )
    }

    fn select_content<'a>(
        subtitle_cues: Option<&'a [TimedTextCue]>,
        lyrics: Option<&'a Lyrics>,
    ) -> Option<PanelContent<'a>> {
        if let Some(cues) = subtitle_cues.filter(|cues| !cues.is_empty()) {
            return Some(PanelContent::Timed(cues));
        }

        let lyrics = lyrics?;
        if !lyrics.lines.is_empty() {
            Some(PanelContent::Timed(&lyrics.lines))
        } else if let Some(plain_lyrics) = lyrics
            .plain_lyrics
            .as_deref()
            .filter(|lyrics| !lyrics.trim().is_empty())
        {
            Some(PanelContent::Plain(plain_lyrics))
        } else if lyrics.instrumental {
            Some(PanelContent::Instrumental)
        } else {
            None
        }
    }

    fn show_content(ui: &mut egui::Ui, content: PanelContent<'_>, current_time_ms: u64) {
        match content {
            PanelContent::Timed(cues) => TimedTextComponent::show(
                ui,
                cues,
                current_time_ms,
                TimedTextPresentation {
                    align: Align::Min,
                    font_size: tokens::text::SM,
                    line_spacing: 0.0,
                    current_color: tokens::color::SUBTITLE_CURRENT_LINE,
                    weaken_inactive: true,
                    empty_cue_text: "",
                },
            ),
            PanelContent::Plain(lyrics) => {
                for line in lyrics.lines() {
                    if line.trim().is_empty() {
                        ui.add_space(tokens::spacing::SM);
                    } else {
                        ui.add(
                            egui::Label::new(RichText::new(line).size(tokens::text::SM))
                                .wrap()
                                .halign(Align::Min)
                                .selectable(false),
                        );
                    }
                }
            }
            PanelContent::Instrumental => {
                ui.label(
                    RichText::new("♪ Instrumental ♪")
                        .italics()
                        .size(tokens::text::SM),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{PanelContent, SubtitleComponent};
    use crate::app::lyrics::Lyrics;
    use crate::app::timed_text::TimedTextCue;

    fn cue(text: &str) -> TimedTextCue {
        TimedTextCue {
            text: text.to_string(),
            start_time_ms: Some(0),
            end_time_ms: None,
        }
    }

    fn lyrics(lines: Vec<TimedTextCue>, plain_lyrics: Option<&str>) -> Lyrics {
        Lyrics {
            id: 0,
            name: "Lyrics".to_string(),
            track_name: "Track".to_string(),
            artist_name: "Artist".to_string(),
            album_name: None,
            duration: None,
            instrumental: false,
            plain_lyrics: plain_lyrics.map(str::to_string),
            synced_lyrics: None,
            lines,
        }
    }

    #[test]
    fn embedded_subtitles_take_priority_over_synced_lyrics() {
        let subtitles = vec![cue("YouTube subtitle")];
        let lyrics = lyrics(vec![cue("Local lyric")], None);

        let selected = SubtitleComponent::select_content(Some(&subtitles), Some(&lyrics));

        assert!(matches!(
            selected,
            Some(PanelContent::Timed(cues)) if cues[0].text == "YouTube subtitle"
        ));
    }

    #[test]
    fn synced_lyrics_are_used_when_subtitles_are_missing() {
        let lyrics = lyrics(vec![cue("Local lyric")], None);

        let selected = SubtitleComponent::select_content(None, Some(&lyrics));

        assert!(matches!(
            selected,
            Some(PanelContent::Timed(cues)) if cues[0].text == "Local lyric"
        ));
    }

    #[test]
    fn plain_lyrics_are_the_final_text_fallback() {
        let lyrics = lyrics(Vec::new(), Some("First line\nSecond line"));

        let selected = SubtitleComponent::select_content(Some(&[]), Some(&lyrics));

        assert!(matches!(
            selected,
            Some(PanelContent::Plain(text)) if text == "First line\nSecond line"
        ));
    }

    #[test]
    fn missing_subtitles_and_lyrics_leave_the_panel_blank() {
        assert!(SubtitleComponent::select_content(None, None).is_none());

        let empty_lyrics = lyrics(Vec::new(), Some("  \n  "));
        assert!(SubtitleComponent::select_content(Some(&[]), Some(&empty_lyrics)).is_none());
    }
}
