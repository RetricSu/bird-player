use super::AppComponent;
use crate::app::{App, Language};
use eframe::egui;

pub struct LanguageSelector;

impl AppComponent for LanguageSelector {
    type Context = App;

    fn add(ctx: &mut Self::Context, ui: &mut eframe::egui::Ui) {
        ui.menu_button("🌐", |ui| {
            for lang in Language::all() {
                let is_selected = ctx.get_language() == lang;
                let lang_text = if is_selected {
                    egui::RichText::new(format!("- {}", lang.name())).strong()
                } else {
                    egui::RichText::new(lang.name())
                };

                if ui.button(lang_text).clicked() {
                    ctx.set_language(lang);
                    ui.close_menu();
                }
            }
        });
    }
}
