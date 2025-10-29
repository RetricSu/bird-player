use super::footer::Footer;
use super::library_component::LibraryComponent;
use super::lyrics_component::LyricsComponent;
use super::player_component::PlayerComponent;
use super::playlist_content::PlaylistContent;
use super::window_chrome::WindowChrome;
use super::AppComponent;
use crate::app::App;
use eframe::egui;

pub struct MainShell;

impl MainShell {
    pub fn show(app: &mut App, ctx: &egui::Context) {
        egui::TopBottomPanel::top("Window Chrome")
            .show_separator_line(true)
            .show(ctx, |ui| {
                WindowChrome::add(app, ui);
            });

        egui::TopBottomPanel::top("Player").show(ctx, |ui| {
            PlayerComponent::add(app, ui);
            ui.add_space(5.0);
        });

        egui::TopBottomPanel::bottom("Footer").show(ctx, |ui| {
            Footer::add(app, ui);
        });

        if app.ui_state.show_lyrics_panel {
            egui::SidePanel::right("Lyrics Panel")
                .default_width(300.0)
                .resizable(true)
                .show(ctx, |ui| {
                    LyricsComponent::add(app, ui);
                });
        }

        egui::SidePanel::left("Library Window")
            .default_width(200.0)
            .show(ctx, |ui| {
                LibraryComponent::add(app, ui);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            PlaylistContent::add(app, ui);
        });
    }
}
