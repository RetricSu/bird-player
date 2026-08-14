use super::subtitle_component::SubtitleComponent;
use super::AppComponent;
use crate::app::lib_services::YoutubeSearchResult;
use crate::app::library::{Library, LibraryItem};
use crate::app::style::{icons, player_button, tokens, ButtonExt};
use crate::app::t;
use crate::app::App;
use eframe::egui::{self, Button, Frame, Id, Margin, Order, RichText, Sense, Stroke, TextEdit};

const SEARCH_PANEL_WIDTH: f32 = 280.0;
const SEARCH_PANEL_HEIGHT: f32 = 220.0;
const TOOL_BUTTON_COUNT: f32 = 4.0;

#[derive(Clone)]
struct LibrarySearchResult {
    track: LibraryItem,
    title: String,
    artist: String,
    album: String,
    source: String,
}

pub struct PlaybackInfoPanel;

impl AppComponent for PlaybackInfoPanel {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
            let search_anchor = ui
                .with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    Self::render_playlist_export_entry(ctx, ui);
                    let search_anchor = Self::render_library_search_controls(ctx, ui);
                    Self::render_download_entry(ctx, ui);
                    Self::render_discover_entry(ctx, ui);
                    search_anchor
                })
                .inner;

            if let Some(anchor) = search_anchor {
                Self::render_library_search_results(ctx, ui, anchor);
            }

            if ctx.ui_state.show_subtitle_panel {
                let button_spacing = ui.spacing().item_spacing.x;
                let tool_strip_width = TOOL_BUTTON_COUNT * tokens::size::ICON_BTN
                    + (TOOL_BUTTON_COUNT - 1.0) * button_spacing;
                let subtitle_panel_width =
                    (TOOL_BUTTON_COUNT - 1.0) * (tokens::size::ICON_BTN + button_spacing);
                ui.allocate_ui_with_layout(
                    egui::vec2(
                        tool_strip_width.min(ui.available_width()),
                        ui.available_height(),
                    ),
                    egui::Layout::right_to_left(egui::Align::Min),
                    |ui| {
                        ui.add_space(tokens::size::ICON_BTN / 2.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(
                                subtitle_panel_width.min(ui.available_width()),
                                ui.available_height(),
                            ),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_width(subtitle_panel_width.min(ui.available_width()));
                                SubtitleComponent::add(ctx, ui);
                            },
                        );
                    },
                );
            }
        });
    }
}

impl PlaybackInfoPanel {
    fn search_id(key: &'static str) -> Id {
        Id::new(("playback_info_panel_library_search", key))
    }

    fn tool_button(ui: &mut egui::Ui, icon: &str, active: bool) -> egui::Response {
        ui.allocate_ui_with_layout(
            egui::vec2(tokens::size::ICON_BTN, tokens::size::ICON_BTN),
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| ui.add(player_button(icon, active)),
        )
        .inner
    }

    fn render_playlist_export_entry(ctx: &mut App, ui: &mut egui::Ui) {
        let active = ctx.ui_state.show_playlist_export_dialog;
        let button =
            Self::tool_button(ui, icons::EXPORT, active).on_hover_text(t("export_playlist"));

        if button.clicked() {
            ctx.ui_state.show_playlist_export_dialog = true;
            if ctx.ui_state.playlist_export_selected_idx.is_none() {
                ctx.ui_state.playlist_export_selected_idx = ctx.app_settings.current_playlist_idx;
            }
        }

        Self::render_playlist_export_dialog(ctx, ui.ctx().clone());
    }

    fn render_playlist_export_dialog(ctx: &mut App, egui_ctx: egui::Context) {
        if !ctx.ui_state.show_playlist_export_dialog {
            return;
        }

        let mut open = ctx.ui_state.show_playlist_export_dialog;
        egui::Window::new(t("export_playlist"))
            .id(egui::Id::new("playlist_export_dialog"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(&egui_ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label(RichText::new(t("playlist_export_notice")).weak());
                ui.add_space(tokens::spacing::XS);

                let playlist_options = ctx
                    .playlists
                    .iter()
                    .enumerate()
                    .map(|(idx, playlist)| {
                        (
                            idx,
                            playlist
                                .get_name()
                                .unwrap_or_else(|| t("untitled_playlist")),
                        )
                    })
                    .collect::<Vec<_>>();

                let selected_idx = ctx
                    .ui_state
                    .playlist_export_selected_idx
                    .filter(|idx| *idx < ctx.playlists.len())
                    .or(ctx.app_settings.current_playlist_idx)
                    .filter(|idx| *idx < ctx.playlists.len());
                ctx.ui_state.playlist_export_selected_idx = selected_idx;

                let selected_label = selected_idx
                    .and_then(|idx| {
                        playlist_options
                            .iter()
                            .find(|(option_idx, _)| *option_idx == idx)
                    })
                    .map(|(_, name)| name.clone())
                    .unwrap_or_else(|| t("playlist_export_select"));

                egui::ComboBox::from_label(t("playlist_label"))
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for (idx, name) in &playlist_options {
                            ui.selectable_value(
                                &mut ctx.ui_state.playlist_export_selected_idx,
                                Some(*idx),
                                name,
                            );
                        }
                    });

                if let Some(idx) = ctx.ui_state.playlist_export_selected_idx {
                    if let Some(playlist) = ctx.playlists.get(idx) {
                        ui.label(
                            RichText::new(format!(
                                "{} · {}",
                                playlist.tracks.len(),
                                t("playlist_export_tracks")
                            ))
                            .weak(),
                        );
                    }
                }

                if let Some(status) = &ctx.ui_state.playlist_export_status {
                    ui.add_space(tokens::spacing::XS);
                    ui.label(RichText::new(status).weak());
                }

                ui.add_space(tokens::spacing::SM);
                ui.horizontal(|ui| {
                    let can_export = ctx
                        .ui_state
                        .playlist_export_selected_idx
                        .and_then(|idx| ctx.playlists.get(idx))
                        .is_some_and(|playlist| !playlist.tracks.is_empty());

                    if ui
                        .add_enabled(can_export, egui::Button::new(t("export")))
                        .clicked()
                    {
                        if let Some(idx) = ctx.ui_state.playlist_export_selected_idx {
                            ctx.export_playlist_to_archive(idx);
                        }
                    }

                    if ui.button(t("clear")).clicked() {
                        ctx.ui_state.playlist_export_status = None;
                    }
                });
            });

        ctx.ui_state.show_playlist_export_dialog = open;
    }

    fn render_download_entry(ctx: &mut App, ui: &mut egui::Ui) {
        let active =
            ctx.ui_state.show_youtube_download_dialog || ctx.ui_state.youtube_download_in_progress;
        let button = Self::tool_button(ui, icons::DOWNLOAD, active)
            .on_hover_text(t("download_authorized_audio"));

        if button.clicked() {
            ctx.ui_state.show_youtube_download_dialog = true;
        }

        Self::render_download_dialog(ctx, ui.ctx().clone());
    }

    fn render_discover_entry(ctx: &mut App, ui: &mut egui::Ui) {
        let active =
            ctx.ui_state.show_youtube_discover_dialog || ctx.ui_state.youtube_discover_in_progress;
        let button =
            Self::tool_button(ui, icons::DISCOVER, active).on_hover_text(t("youtube_discover"));

        if button.clicked() {
            ctx.ui_state.show_youtube_discover_dialog = true;
        }

        Self::render_discover_dialog(ctx, ui.ctx().clone());
    }

    fn render_discover_dialog(ctx: &mut App, egui_ctx: egui::Context) {
        if !ctx.ui_state.show_youtube_discover_dialog {
            return;
        }

        let viewport = egui::ViewportBuilder::default()
            .with_title(t("youtube_discover"))
            .with_inner_size(egui::vec2(620.0, 560.0))
            .with_min_inner_size(egui::vec2(520.0, 360.0));

        egui_ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("youtube_discover_window"),
            viewport,
            |viewport_ctx, _class| {
                if viewport_ctx.input(|input| input.viewport().close_requested()) {
                    ctx.ui_state.show_youtube_discover_dialog = false;
                    return;
                }

                egui::CentralPanel::default().show(viewport_ctx, |ui| {
                    Self::render_discover_contents(ctx, ui);
                });
            },
        );
    }

    fn render_discover_contents(ctx: &mut App, ui: &mut egui::Ui) {
        ui.set_min_width(560.0);
        ui.label(RichText::new(t("youtube_discover_notice")).weak());
        ui.add_space(tokens::spacing::XS);

        ui.label(t("youtube_discover_query"));
        let query_response = ui.add_enabled(
            !ctx.ui_state.youtube_discover_in_progress,
            TextEdit::singleline(&mut ctx.ui_state.youtube_discover_query)
                .desired_width(ui.available_width())
                .hint_text(t("youtube_discover_placeholder")),
        );
        let enter_pressed =
            query_response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));

        ui.add_space(tokens::spacing::XS);
        if let Some(status) = &ctx.ui_state.youtube_discover_status {
            ui.label(RichText::new(status).weak());
            ui.add_space(tokens::spacing::XS);
        }

        ui.horizontal(|ui| {
            let can_search = !ctx.ui_state.youtube_discover_in_progress
                && !ctx.ui_state.youtube_discover_query.trim().is_empty();
            if ui
                .add_enabled(can_search, egui::Button::new(t("search")))
                .clicked()
                || (can_search && enter_pressed)
            {
                ctx.start_youtube_discover_search();
            }

            if ui
                .add_enabled(
                    !ctx.ui_state.youtube_discover_in_progress,
                    egui::Button::new(t("clear")),
                )
                .clicked()
            {
                ctx.ui_state.youtube_discover_query.clear();
                ctx.ui_state.youtube_discover_results.clear();
                ctx.ui_state.youtube_discover_status = None;
            }
        });

        if ctx.ui_state.youtube_discover_in_progress {
            ui.add_space(tokens::spacing::XS);
            ui.spinner();
        }

        Self::render_discover_results(ctx, ui);
    }

    fn render_discover_results(ctx: &mut App, ui: &mut egui::Ui) {
        if ctx.ui_state.youtube_discover_results.is_empty() {
            return;
        }

        ui.add_space(tokens::spacing::SM);
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let results = ctx.ui_state.youtube_discover_results.clone();
                for result in results {
                    Self::render_discover_result_row(ctx, ui, &result);
                }
            });
    }

    fn render_discover_result_row(ctx: &mut App, ui: &mut egui::Ui, result: &YoutubeSearchResult) {
        Frame::new()
            .inner_margin(Margin::symmetric(
                tokens::spacing::SM as i8,
                tokens::spacing::XS as i8,
            ))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Self::render_discover_thumbnail(ui, result);

                    ui.vertical(|ui| {
                        ui.set_width(392.0);
                        ui.label(RichText::new(&result.title).strong());
                        ui.label(
                            RichText::new(Self::discover_result_meta(result))
                                .size(tokens::text::SM)
                                .weak(),
                        );
                    });

                    if ui.button(t("use_result")).clicked() {
                        ctx.ui_state.youtube_download_url = result.url.clone();
                        ctx.ui_state.show_youtube_download_dialog = true;
                        ctx.ui_state.show_youtube_discover_dialog = false;
                        ctx.ui_state.youtube_download_status = None;
                    }
                });
            });
    }

    fn render_discover_thumbnail(ui: &mut egui::Ui, result: &YoutubeSearchResult) {
        let size = egui::vec2(56.0, 56.0);
        if let Some(url) = &result.thumbnail_url {
            ui.add(
                egui::Image::from_uri(url)
                    .fit_to_exact_size(size)
                    .corner_radius(tokens::radius::SM),
            );
        } else {
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter()
                .rect_filled(rect, tokens::radius::SM, ui.visuals().faint_bg_color);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                icons::YOUTUBE,
                egui::FontId::proportional(22.0),
                ui.visuals().weak_text_color(),
            );
        }
    }

    fn discover_result_meta(result: &YoutubeSearchResult) -> String {
        match result.duration {
            Some(duration) => format!("{} · {}", result.channel, Self::format_duration(duration)),
            None => result.channel.clone(),
        }
    }

    fn format_duration(seconds: u64) -> String {
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        format!("{}:{:02}", minutes, seconds)
    }

    fn render_download_dialog(ctx: &mut App, egui_ctx: egui::Context) {
        if !ctx.ui_state.show_youtube_download_dialog {
            return;
        }

        let viewport = egui::ViewportBuilder::default()
            .with_title(t("download_authorized_audio"))
            .with_inner_size(egui::vec2(500.0, 360.0))
            .with_min_inner_size(egui::vec2(420.0, 300.0));

        egui_ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("youtube_download_window"),
            viewport,
            |viewport_ctx, _class| {
                if viewport_ctx.input(|input| input.viewport().close_requested()) {
                    ctx.ui_state.show_youtube_download_dialog = false;
                    return;
                }

                egui::CentralPanel::default().show(viewport_ctx, |ui| {
                    Self::render_download_contents(ctx, ui);
                });
            },
        );
    }

    fn render_download_contents(ctx: &mut App, ui: &mut egui::Ui) {
        ui.set_min_width(420.0);
        ui.label(RichText::new(t("authorized_audio_notice")).weak());
        ui.add_space(tokens::spacing::XS);

        ui.label(t("youtube_url"));
        ui.add_enabled(
            !ctx.ui_state.youtube_download_in_progress,
            TextEdit::singleline(&mut ctx.ui_state.youtube_download_url)
                .desired_width(ui.available_width())
                .hint_text("https://www.youtube.com/watch?v=..."),
        );

        ui.add_space(tokens::spacing::XS);
        ui.add_enabled_ui(!ctx.ui_state.youtube_download_in_progress, |ui| {
            ui.checkbox(
                &mut ctx.ui_state.youtube_download_include_playlist,
                t("download_entire_playlist"),
            )
            .on_hover_text(t("download_entire_playlist_hint"));
            ui.checkbox(
                &mut ctx.ui_state.youtube_download_subtitles,
                t("download_available_subtitles"),
            )
            .on_hover_text(t("download_available_subtitles_hint"));
        });

        ui.add_space(tokens::spacing::XS);
        ui.label(t("save_to"));
        ui.horizontal(|ui| {
            let mut path_text = ctx.ui_state.youtube_download_dir.display().to_string();
            ui.add_enabled(
                false,
                TextEdit::singleline(&mut path_text).desired_width(ui.available_width() - 44.0),
            );

            let choose = ui
                .add_enabled(
                    !ctx.ui_state.youtube_download_in_progress,
                    egui::Button::new(icons::FOLDER).player_style(),
                )
                .on_hover_text(t("choose_download_folder"));
            if choose.clicked() {
                if let Some(folder) = rfd::FileDialog::new()
                    .set_directory(&ctx.ui_state.youtube_download_dir)
                    .pick_folder()
                {
                    ctx.ui_state.youtube_download_dir = folder;
                    ctx.save_state();
                }
            }
        });

        if let Some(status) = &ctx.ui_state.youtube_download_status {
            ui.add_space(tokens::spacing::XS);
            ui.label(RichText::new(status).weak());
        }

        if let Some(progress) = ctx.ui_state.youtube_download_progress {
            ui.add_space(tokens::spacing::XS);
            ui.add(
                egui::ProgressBar::new(progress)
                    .show_percentage()
                    .desired_width(ui.available_width()),
            );
        } else if ctx.ui_state.youtube_download_in_progress {
            ui.add_space(tokens::spacing::XS);
            ui.add(
                egui::ProgressBar::new(0.0)
                    .animate(true)
                    .desired_width(ui.available_width()),
            );
        }

        ui.add_space(tokens::spacing::SM);
        ui.horizontal(|ui| {
            let can_download = !ctx.ui_state.youtube_download_in_progress
                && !ctx.ui_state.youtube_download_url.trim().is_empty();
            if ui
                .add_enabled(can_download, egui::Button::new(t("download")))
                .clicked()
            {
                ctx.start_youtube_download();
            }

            if ui
                .add_enabled(
                    !ctx.ui_state.youtube_download_in_progress,
                    egui::Button::new(t("clear")),
                )
                .clicked()
            {
                ctx.ui_state.youtube_download_url.clear();
                ctx.ui_state.youtube_download_progress = None;
                ctx.ui_state.youtube_download_last_file_count = None;
                ctx.ui_state.youtube_download_last_subtitle_count = None;
                ctx.ui_state.youtube_download_resync_in_progress = false;
                ctx.ui_state.youtube_download_status = None;
            }
        });
    }

    fn render_library_search_controls(ctx: &mut App, ui: &mut egui::Ui) -> Option<egui::Rect> {
        let search_active_id = Self::search_id("active");
        let search_text_id = Self::search_id("text");
        let search_results_id = Self::search_id("results");
        let show_results_id = Self::search_id("show_results");
        let no_results_id = Self::search_id("no_results");
        let mut anchor_rect = None;

        let mut search_active = ui
            .memory_mut(|mem| mem.data.get_temp::<bool>(search_active_id))
            .unwrap_or(false);
        let mut search_text = ui
            .memory_mut(|mem| mem.data.get_temp::<String>(search_text_id))
            .unwrap_or_default();

        let mut should_search = false;

        if search_active {
            let search_width = ui.available_width().min(SEARCH_PANEL_WIDTH);
            let editor_id = Self::search_id("editor");
            let first_frame_id = Self::search_id("first_frame");
            let is_first_frame = ui
                .memory_mut(|mem| mem.data.get_temp::<bool>(first_frame_id))
                .unwrap_or(true);

            if is_first_frame {
                ui.memory_mut(|mem| {
                    mem.request_focus(editor_id);
                    mem.data.insert_temp(first_frame_id, false);
                });
            }

            let mut text_response: Option<egui::Response> = None;
            let mut close_clicked = false;

            let frame_response = Frame::new()
                .corner_radius(tokens::radius::SM)
                .inner_margin(Margin::symmetric(tokens::spacing::SM as i8, 0))
                .stroke(ui.visuals().widgets.active.bg_stroke)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(search_width, tokens::size::ICON_BTN));
                    ui.set_max_width(search_width);
                    ui.set_max_height(tokens::size::ICON_BTN);
                    ui.spacing_mut().item_spacing.x = tokens::spacing::XS;

                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        let close_button_size = egui::vec2(24.0, 24.0);
                        let text_width = (ui.available_width()
                            - close_button_size.x
                            - ui.spacing().item_spacing.x)
                            .max(120.0);
                        text_response = Some(
                            ui.add_sized(
                                [text_width, tokens::size::ICON_BTN - 6.0],
                                TextEdit::singleline(&mut search_text)
                                    .id(editor_id)
                                    .frame(false)
                                    .hint_text(t("type_to_search")),
                            ),
                        );

                        close_clicked = ui
                            .add(
                                Button::new(icons::CLOSE)
                                    .frame(false)
                                    .min_size(close_button_size),
                            )
                            .on_hover_text(t("close_search"))
                            .clicked();
                    });
                })
                .response;

            anchor_rect = Some(frame_response.rect);
            let response = text_response.expect("search text edit should render");
            ui.memory_mut(|mem| mem.data.insert_temp(search_text_id, search_text.clone()));

            should_search = response.changed()
                || (response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));

            if close_clicked {
                search_active = false;
                search_text.clear();
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(search_text_id, String::new());
                    mem.data.insert_temp(show_results_id, false);
                    mem.data.insert_temp(no_results_id, false);
                });
            }
        } else if Self::tool_button(ui, icons::SEARCH, false)
            .on_hover_text(t("library_search"))
            .clicked()
        {
            search_active = true;
            ui.memory_mut(|mem| {
                mem.data.insert_temp(Self::search_id("first_frame"), true);
                mem.data.insert_temp(search_text_id, String::new());
                mem.data.insert_temp(show_results_id, false);
                mem.data.insert_temp(no_results_id, false);
            });
        }

        if should_search {
            let query = search_text.trim();
            let results = if query.is_empty() {
                Vec::new()
            } else {
                Self::search_library(&ctx.library, query)
            };
            let has_results = !results.is_empty();
            ui.memory_mut(|mem| {
                mem.data.insert_temp(search_results_id, results);
                mem.data.insert_temp(show_results_id, has_results);
                mem.data
                    .insert_temp(no_results_id, !has_results && !query.is_empty());
            });
        }

        ui.memory_mut(|mem| mem.data.insert_temp(search_active_id, search_active));

        anchor_rect
    }

    fn render_library_search_results(ctx: &mut App, ui: &mut egui::Ui, anchor: egui::Rect) {
        let search_results_id = Self::search_id("results");
        let show_results_id = Self::search_id("show_results");
        let no_results_id = Self::search_id("no_results");
        let selected_key_id = Self::search_id("selected_key");

        let show_results = ui
            .memory_mut(|mem| mem.data.get_temp::<bool>(show_results_id))
            .unwrap_or(false);
        let mut track_to_play: Option<LibraryItem> = None;
        let no_results = ui
            .memory_mut(|mem| mem.data.get_temp::<bool>(no_results_id))
            .unwrap_or(false);

        if show_results || no_results {
            let screen_rect = ui.ctx().screen_rect();
            let panel_width = anchor.width().min(SEARCH_PANEL_WIDTH);
            let x = (anchor.right() - panel_width)
                .max(screen_rect.left() + tokens::spacing::SM)
                .min(screen_rect.right() - panel_width - tokens::spacing::SM);
            let y = (anchor.bottom() + tokens::spacing::XS)
                .min(screen_rect.bottom() - SEARCH_PANEL_HEIGHT - tokens::spacing::SM);

            egui::Area::new(Self::search_id("panel_area"))
                .order(Order::Foreground)
                .fixed_pos(egui::pos2(x, y))
                .show(ui.ctx(), |ui| {
                    Frame::popup(ui.style())
                        .corner_radius(tokens::radius::SM)
                        .inner_margin(Margin::symmetric(
                            tokens::spacing::SM as i8,
                            tokens::spacing::XS as i8,
                        ))
                        .stroke(Stroke::new(
                            tokens::size::STROKE_WIDTH,
                            ui.visuals().widgets.noninteractive.bg_stroke.color,
                        ))
                        .show(ui, |ui| {
                            ui.set_width(panel_width);
                            ui.set_max_height(SEARCH_PANEL_HEIGHT);

                            if no_results {
                                ui.label(
                                    RichText::new(t("no_matches_found"))
                                        .size(tokens::text::SM)
                                        .color(tokens::color::LYRICS_FAILED),
                                );
                                return;
                            }

                            Self::render_library_search_result_list(
                                ui,
                                search_results_id,
                                selected_key_id,
                                &mut track_to_play,
                            );
                        });
                });
        }

        if let Some(track) = track_to_play {
            {
                let player = ctx.player_mut_ref();
                player.select_track(Some(track));
                player.play();
            }
            ctx.app_settings.playing_playlist_idx = None;
            ctx.auto_fetch_lyrics_for_current_track();
        }
    }

    fn render_library_search_result_list(
        ui: &mut egui::Ui,
        search_results_id: egui::Id,
        selected_key_id: egui::Id,
        track_to_play: &mut Option<LibraryItem>,
    ) {
        if let Some(results) = ui.memory_mut(|mem| {
            mem.data
                .get_temp::<Vec<LibrarySearchResult>>(search_results_id)
        }) {
            egui::ScrollArea::vertical()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    for result in results {
                        let title = if result.title.is_empty() {
                            t("unknown_track")
                        } else {
                            result.title.clone()
                        };
                        let detail = [result.artist, result.album, result.source]
                            .into_iter()
                            .filter(|text| !text.is_empty())
                            .collect::<Vec<_>>()
                            .join(" · ");
                        let row_text = if detail.is_empty() {
                            title
                        } else {
                            format!("{}  {}", title, detail)
                        };

                        let track_key = result.track.key();
                        let selected = ui
                            .memory_mut(|mem| mem.data.get_temp::<String>(selected_key_id))
                            .is_some_and(|key| key == track_key);

                        let row_size =
                            egui::vec2(ui.available_width(), tokens::size::ICON_BTN - 4.0);
                        let (rect, response) = ui.allocate_exact_size(row_size, Sense::click());
                        let visuals = ui.visuals().clone();
                        if selected {
                            ui.painter().rect_filled(
                                rect,
                                tokens::radius::SM,
                                tokens::color::BRAND,
                            );
                        } else if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                tokens::radius::SM,
                                visuals.widgets.hovered.weak_bg_fill,
                            );
                        }

                        let text_color = if selected {
                            egui::Color32::from_rgb(245, 248, 252)
                        } else {
                            visuals.text_color()
                        };
                        let text_rect = rect.shrink2(egui::vec2(tokens::spacing::SM, 0.0));
                        ui.painter().text(
                            text_rect.left_center(),
                            egui::Align2::LEFT_CENTER,
                            Self::elide_to_width(&row_text, text_rect.width()),
                            egui::FontId::proportional(tokens::text::SM),
                            text_color,
                        );

                        if response.clicked() {
                            ui.memory_mut(|mem| {
                                mem.data.insert_temp(selected_key_id, track_key);
                            });
                            *track_to_play = Some(result.track);
                        }
                    }
                });
        }
    }

    fn search_library(library: &Library, query: &str) -> Vec<LibrarySearchResult> {
        let query = query.to_lowercase();
        library
            .items()
            .iter()
            .filter_map(|item| {
                let title = item.title_ref().unwrap_or_default();
                let artist = item.artist_ref().unwrap_or_default();
                let album = item.album_ref().unwrap_or_default();
                let genre = item.genre_ref().unwrap_or_default();
                let path = item.path_ref().to_string_lossy();

                let matches = [title, artist, album, genre]
                    .iter()
                    .any(|field| field.to_lowercase().contains(&query))
                    || path.to_lowercase().contains(&query);

                if !matches {
                    return None;
                }

                let source = item
                    .path_ref()
                    .parent()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("Library")
                    .to_string();

                Some(LibrarySearchResult {
                    track: item.clone(),
                    title: title.to_string(),
                    artist: artist.to_string(),
                    album: album.to_string(),
                    source,
                })
            })
            .take(50)
            .collect()
    }

    fn elide_to_width(text: &str, width: f32) -> String {
        let max_chars = (width / 8.0).floor().max(8.0) as usize;
        let char_count = text.chars().count();
        if char_count <= max_chars {
            return text.to_string();
        }

        let keep = max_chars.saturating_sub(3);
        format!("{}...", text.chars().take(keep).collect::<String>())
    }
}
