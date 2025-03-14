pub mod cassette_component;
pub mod footer;
pub mod language_selector;
pub mod library_component;
pub mod player_component;
pub mod playlist_table;
pub mod playlist_tabs;
pub mod window_chrome;

pub trait AppComponent {
    type Context;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui);
}
