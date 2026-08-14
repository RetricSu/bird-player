use super::AppComponent;
use crate::app::state::ui_state::{PlaylistBookletDraft, PlaylistBookletMode};
use crate::app::style::{icons, tokens};
use crate::app::{t, tf, App, Playlist};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Stroke};
use std::hash::{Hash, Hasher};

const COVER_SIZE: f32 = 132.0;
const MAX_COVER_BYTES: u64 = 12 * 1024 * 1024;

pub struct PlaylistBookletComponent;

impl PlaylistBookletComponent {
    pub fn show_window(ctx: &mut App, egui_ctx: &egui::Context) {
        if ctx.ui_state.playlist_booklet_mode.is_none() {
            return;
        }
        let Some(playlist_idx) = ctx.ui_state.playlist_booklet_idx else {
            ctx.ui_state.playlist_booklet_mode = None;
            return;
        };
        let Some(playlist) = ctx.playlists.get(playlist_idx) else {
            ctx.ui_state.playlist_booklet_mode = None;
            ctx.ui_state.playlist_booklet_idx = None;
            return;
        };
        let title = format!(
            "{} — {}",
            t("playlist_booklet"),
            playlist
                .get_name()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| t("untitled_playlist"))
        );
        let viewport = egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size(egui::vec2(760.0, 620.0))
            .with_min_inner_size(egui::vec2(560.0, 420.0));

        egui_ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("playlist_booklet_window"),
            viewport,
            |viewport_ctx, _class| {
                if viewport_ctx.input(|input| input.viewport().close_requested()) {
                    ctx.ui_state.playlist_booklet_mode = None;
                    ctx.ui_state.playlist_booklet_idx = None;
                    ctx.ui_state.playlist_booklet_draft = None;
                    return;
                }
                egui::CentralPanel::default().show(viewport_ctx, |ui| {
                    Self::add(ctx, ui);
                });
            },
        );
    }
}

impl AppComponent for PlaylistBookletComponent {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut egui::Ui) {
        let Some(playlist_idx) = ctx.ui_state.playlist_booklet_idx else {
            return;
        };
        let Some(playlist) = ctx.playlists.get(playlist_idx).cloned() else {
            ctx.ui_state.playlist_booklet_mode = None;
            return;
        };
        let mode = ctx
            .ui_state
            .playlist_booklet_mode
            .unwrap_or(PlaylistBookletMode::View);

        if mode == PlaylistBookletMode::Edit && ctx.ui_state.playlist_booklet_draft.is_none() {
            ctx.ui_state.playlist_booklet_draft = Some(Self::draft_from_playlist(&playlist));
        }

        let paper = ui.visuals().panel_fill;
        let rule = ui.visuals().widgets.noninteractive.bg_stroke.color;

        let mut play_all = false;
        let mut start_edit = false;
        let mut save_edit = false;
        let mut cancel_edit = false;

        Frame::new()
            .fill(paper)
            .inner_margin(Margin::symmetric(22, 16))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{}  {}", icons::BOOKLET, t("playlist_booklet")))
                            .size(tokens::text::MD)
                            .strong(),
                    );
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| match mode {
                            PlaylistBookletMode::View => {
                                if ui
                                    .button(format!("{}  {}", icons::EDIT, t("edit_playlist")))
                                    .clicked()
                                {
                                    start_edit = true;
                                }
                                if ui
                                    .add_enabled(
                                        !playlist.tracks.is_empty(),
                                        egui::Button::new(format!(
                                            "{}  {}",
                                            icons::PLAY,
                                            t("play_all")
                                        ))
                                        .fill(tokens::color::BRAND),
                                    )
                                    .clicked()
                                {
                                    play_all = true;
                                }
                            }
                            PlaylistBookletMode::Edit => {
                                if ui.button(t("cancel")).clicked() {
                                    cancel_edit = true;
                                }
                                if ui
                                    .add(
                                        egui::Button::new(format!(
                                            "{}  {}",
                                            icons::SAVE,
                                            t("save")
                                        ))
                                        .fill(tokens::color::BRAND),
                                    )
                                    .clicked()
                                {
                                    save_edit = true;
                                }
                            }
                        },
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .id_salt(("playlist_booklet", playlist_idx))
                    .auto_shrink([false, false])
                    .show(ui, |ui| match mode {
                        PlaylistBookletMode::View => Self::render_view(ui, &playlist, rule),
                        PlaylistBookletMode::Edit => {
                            if let Some(draft) = ctx.ui_state.playlist_booklet_draft.as_mut() {
                                Self::render_editor(
                                    ui,
                                    playlist_idx,
                                    &playlist,
                                    draft,
                                    &mut ctx.ui_state.playlist_booklet_status,
                                    rule,
                                );
                            }
                        }
                    });
            });

        if play_all {
            ctx.play_playlist_from_start(playlist_idx);
        } else if start_edit {
            ctx.ui_state.playlist_booklet_draft = Some(Self::draft_from_playlist(&playlist));
            ctx.ui_state.playlist_booklet_mode = Some(PlaylistBookletMode::Edit);
        } else if cancel_edit {
            ctx.ui_state.playlist_booklet_draft = None;
            ctx.ui_state.playlist_booklet_mode = Some(PlaylistBookletMode::View);
        } else if save_edit {
            if let Some(draft) = ctx.ui_state.playlist_booklet_draft.take() {
                Self::save_draft(ctx, playlist_idx, draft);
            }
        }
    }
}

impl PlaylistBookletComponent {
    fn draft_from_playlist(playlist: &Playlist) -> PlaylistBookletDraft {
        let (cover_mime_type, cover_data) = playlist
            .cover()
            .map(|(mime, bytes)| (Some(mime.to_string()), Some(bytes.to_vec())))
            .unwrap_or((None, None));
        PlaylistBookletDraft {
            name: playlist.get_name().unwrap_or_default(),
            curator: playlist.curator().unwrap_or_default(),
            description: playlist.description().unwrap_or_default(),
            booklet: playlist.booklet().unwrap_or_default(),
            cover_mime_type,
            cover_data,
            track_notes: (0..playlist.tracks.len())
                .map(|idx| playlist.track_note(idx).unwrap_or_default().to_string())
                .collect(),
        }
    }

    fn save_draft(ctx: &mut App, playlist_idx: usize, draft: PlaylistBookletDraft) {
        let Some(playlist) = ctx.playlists.get_mut(playlist_idx) else {
            return;
        };
        playlist.set_name(draft.name.trim().to_string());
        playlist.set_curator(Self::non_empty(draft.curator));
        playlist.set_description(Self::non_empty(draft.description));
        playlist.set_booklet(Self::non_empty(draft.booklet));
        playlist.set_cover(draft.cover_mime_type, draft.cover_data);
        for (idx, note) in draft.track_notes.into_iter().enumerate() {
            playlist.set_track_note(idx, Self::non_empty(note));
        }
        ctx.ui_state.playlist_booklet_mode = Some(PlaylistBookletMode::View);
        ctx.ui_state.playlist_booklet_status = Some(t("playlist_booklet_saved"));
        ctx.save_state();
    }

    fn non_empty(text: String) -> Option<String> {
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    fn render_view(ui: &mut egui::Ui, playlist: &Playlist, rule: Color32) {
        ui.horizontal_top(|ui| {
            Self::render_cover(
                ui,
                playlist.cover().map(|(_, bytes)| bytes),
                "playlist-cover-view",
            );
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(
                        playlist
                            .get_name()
                            .filter(|name| !name.trim().is_empty())
                            .unwrap_or_else(|| t("untitled_playlist")),
                    )
                    .size(24.0)
                    .strong(),
                );
                if let Some(curator) = playlist.curator().filter(|text| !text.trim().is_empty()) {
                    ui.label(
                        RichText::new(tf("curated_by", &[&curator]))
                            .size(tokens::text::SM)
                            .color(ui.visuals().weak_text_color()),
                    );
                }
                ui.label(
                    RichText::new(tf("playlist_tracks", &[&playlist.tracks.len().to_string()]))
                        .size(tokens::text::SM)
                        .color(ui.visuals().weak_text_color()),
                );
                ui.add_space(8.0);
                if let Some(description) = playlist
                    .description()
                    .filter(|text| !text.trim().is_empty())
                {
                    ui.label(RichText::new(description).size(tokens::text::MD));
                }
            });
        });

        ui.add_space(18.0);
        ui.painter().hline(
            ui.available_rect_before_wrap().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0, rule),
        );
        ui.add_space(16.0);

        ui.label(
            RichText::new(t("liner_notes"))
                .size(tokens::text::SM)
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(6.0);
        if let Some(booklet) = playlist.booklet().filter(|text| !text.trim().is_empty()) {
            Self::render_liner_notes(ui, &booklet);
        } else {
            ui.label(
                RichText::new(t("booklet_empty"))
                    .italics()
                    .color(ui.visuals().weak_text_color()),
            );
        }

        ui.add_space(20.0);
        ui.label(
            RichText::new(t("track_notes"))
                .size(tokens::text::SM)
                .strong()
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(6.0);
        for (idx, track) in playlist.tracks.iter().enumerate() {
            ui.horizontal_top(|ui| {
                ui.set_min_height(34.0);
                ui.add_sized(
                    [26.0, 20.0],
                    egui::Label::new(
                        RichText::new(format!("{:02}", idx + 1))
                            .monospace()
                            .color(ui.visuals().weak_text_color()),
                    ),
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(
                            track
                                .title()
                                .unwrap_or_else(|| t("unknown_title").to_string()),
                        )
                        .strong(),
                    );
                    let artist = track.artist().unwrap_or_default();
                    let note = playlist.track_note(idx).unwrap_or_default();
                    if !artist.is_empty() {
                        ui.label(
                            RichText::new(artist)
                                .size(tokens::text::SM)
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                    if !note.trim().is_empty() {
                        ui.add_space(3.0);
                        ui.label(RichText::new(note).italics());
                    }
                });
            });
            ui.add_space(4.0);
            ui.painter().hline(
                ui.available_rect_before_wrap().x_range(),
                ui.cursor().top(),
                Stroke::new(1.0, rule),
            );
            ui.add_space(7.0);
        }
    }

    fn render_liner_notes(ui: &mut egui::Ui, notes: &str) {
        for line in notes.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                ui.add_space(7.0);
            } else if let Some(heading) = trimmed.strip_prefix("# ") {
                ui.label(RichText::new(heading).size(tokens::text::LG).strong());
            } else if let Some(quote) = trimmed.strip_prefix("> ") {
                ui.label(
                    RichText::new(quote)
                        .italics()
                        .color(ui.visuals().weak_text_color()),
                );
            } else {
                ui.label(RichText::new(trimmed).size(tokens::text::MD));
            }
        }
    }

    fn render_editor(
        ui: &mut egui::Ui,
        playlist_idx: usize,
        playlist: &Playlist,
        draft: &mut PlaylistBookletDraft,
        status: &mut Option<String>,
        rule: Color32,
    ) {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                Self::render_cover(
                    ui,
                    draft.cover_data.as_deref(),
                    &format!("playlist-cover-draft-{playlist_idx}"),
                );
                if ui
                    .button(format!("{}  {}", icons::IMAGE, t("change_cover")))
                    .clicked()
                {
                    Self::choose_cover(draft, status);
                }
                if draft.cover_data.is_some() && ui.button(t("remove_cover")).clicked() {
                    draft.cover_data = None;
                    draft.cover_mime_type = None;
                }
            });
            ui.add_space(16.0);
            ui.vertical(|ui| {
                ui.label(t("playlist_label"));
                ui.add(
                    egui::TextEdit::singleline(&mut draft.name)
                        .desired_width(f32::INFINITY)
                        .hint_text(t("untitled_playlist")),
                );
                ui.add_space(6.0);
                ui.label(t("curator"));
                ui.add(egui::TextEdit::singleline(&mut draft.curator).desired_width(f32::INFINITY));
                ui.add_space(6.0);
                ui.label(t("playlist_description"));
                ui.add(
                    egui::TextEdit::multiline(&mut draft.description)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2),
                );
            });
        });

        if let Some(status) = status.as_deref() {
            ui.add_space(8.0);
            ui.label(
                RichText::new(status)
                    .size(tokens::text::SM)
                    .color(ui.visuals().weak_text_color()),
            );
        }

        ui.add_space(16.0);
        ui.painter().hline(
            ui.available_rect_before_wrap().x_range(),
            ui.cursor().top(),
            Stroke::new(1.0, rule),
        );
        ui.add_space(14.0);
        ui.label(RichText::new(t("liner_notes")).strong());
        ui.add(
            egui::TextEdit::multiline(&mut draft.booklet)
                .desired_width(f32::INFINITY)
                .desired_rows(8)
                .hint_text(t("booklet_empty")),
        );

        ui.add_space(18.0);
        ui.label(RichText::new(t("track_notes")).strong());
        ui.add_space(6.0);
        draft
            .track_notes
            .resize(playlist.tracks.len(), String::new());
        for (idx, track) in playlist.tracks.iter().enumerate() {
            ui.horizontal_top(|ui| {
                ui.add_sized(
                    [26.0, 20.0],
                    egui::Label::new(
                        RichText::new(format!("{:02}", idx + 1))
                            .monospace()
                            .color(ui.visuals().weak_text_color()),
                    ),
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(
                            track
                                .title()
                                .unwrap_or_else(|| t("unknown_title").to_string()),
                        )
                        .strong(),
                    );
                    ui.add(
                        egui::TextEdit::multiline(&mut draft.track_notes[idx])
                            .desired_width(f32::INFINITY)
                            .desired_rows(2)
                            .hint_text(t("track_note_hint")),
                    );
                });
            });
            ui.add_space(7.0);
        }
    }

    fn choose_cover(draft: &mut PlaylistBookletDraft, status: &mut Option<String>) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp", "gif"])
            .pick_file()
        else {
            return;
        };
        let Ok(metadata) = std::fs::metadata(&path) else {
            *status = Some(t("cover_invalid"));
            return;
        };
        if metadata.len() > MAX_COVER_BYTES {
            *status = Some(t("cover_too_large"));
            return;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            *status = Some(t("cover_invalid"));
            return;
        };
        let Ok(format) = image::guess_format(&bytes) else {
            *status = Some(t("cover_invalid"));
            return;
        };
        if image::load_from_memory_with_format(&bytes, format).is_err() {
            *status = Some(t("cover_invalid"));
            return;
        }
        let mime_type = match format {
            image::ImageFormat::Png => "image/png",
            image::ImageFormat::WebP => "image/webp",
            image::ImageFormat::Gif => "image/gif",
            _ => "image/jpeg",
        };
        draft.cover_mime_type = Some(mime_type.to_string());
        draft.cover_data = Some(bytes);
        *status = None;
    }

    fn render_cover(ui: &mut egui::Ui, bytes: Option<&[u8]>, uri_prefix: &str) {
        if let Some(bytes) = bytes {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            bytes.hash(&mut hasher);
            let uri = format!("bytes://{uri_prefix}-{}", hasher.finish());
            ui.add(
                egui::Image::from_bytes(uri, bytes.to_vec())
                    .fit_to_exact_size(egui::vec2(COVER_SIZE, COVER_SIZE))
                    .maintain_aspect_ratio(true),
            );
        } else {
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(COVER_SIZE, COVER_SIZE), egui::Sense::hover());
            let background = if ui.visuals().dark_mode {
                Color32::from_rgb(51, 49, 44)
            } else {
                Color32::from_rgb(226, 220, 207)
            };
            ui.painter().rect_filled(rect, 1.0, background);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                icons::BOOKLET,
                egui::FontId::proportional(34.0),
                ui.visuals().weak_text_color(),
            );
        }
    }
}
