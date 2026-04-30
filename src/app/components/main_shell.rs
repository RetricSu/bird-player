use super::footer::Footer;
use super::library_component::LibraryComponent;
use super::lyrics_component::LyricsComponent;
use super::player_component::PlayerComponent;
use super::playlist_content::PlaylistContent;
use super::window_chrome::WindowChrome;
use super::AppComponent;
use crate::app::style::tokens;
use crate::app::App;
use eframe::egui;

pub struct MainShell;

impl MainShell {
    pub fn show(app: &mut App, ctx: &egui::Context) {
        egui::TopBottomPanel::top("Window Chrome")
            .show_separator_line(true)
            .frame(
                egui::Frame::side_top_panel(&ctx.style()).inner_margin(egui::Margin {
                    left: tokens::spacing::SM as i8,
                    right: tokens::spacing::XS as i8,
                    top: 2,
                    bottom: 2,
                }),
            )
            .show(ctx, |ui| {
                WindowChrome::add(app, ui);
            });

        egui::TopBottomPanel::top("Player").show(ctx, |ui| {
            PlayerComponent::add(app, ui);
            ui.add_space(tokens::spacing::XS);
        });

        egui::TopBottomPanel::bottom("Footer").show(ctx, |ui| {
            Footer::add(app, ui);
        });

        egui::SidePanel::left("Library Window")
            .default_width(200.0)
            .show(ctx, |ui| {
                LibraryComponent::add(app, ui);
            });

        if app.ui_state.show_lyrics_panel {
            egui::SidePanel::right("Lyrics Panel")
                .default_width(280.0)
                .min_width(220.0)
                .resizable(true)
                .show(ctx, |ui| {
                    LyricsComponent::add(app, ui);
                });
        }

        egui::CentralPanel::default()
            // Match SidePanel's default frame so the middle column's
            // top inner_margin equals the library / lyrics panels —
            // otherwise the central panel adds extra top padding and
            // the playlist tab strip's bottom separator lands a few
            // pixels below the other two panel separators.
            .frame(egui::Frame::side_top_panel(&ctx.style()))
            .show(ctx, |ui| {
                PlaylistContent::add(app, ui);
            });
    }
}
