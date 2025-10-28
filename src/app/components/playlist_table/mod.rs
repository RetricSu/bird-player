use super::AppComponent;
use crate::app::App;
use eframe::egui;

mod actions;
mod columns;
mod controller;
mod drag;
mod post_render;
mod row_highlight;
mod row_texts;
mod services;
mod state;
mod view;

pub struct PlaylistTable;

impl AppComponent for PlaylistTable {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut egui::Ui) {
        view::render(ctx, ui);
    }
}
