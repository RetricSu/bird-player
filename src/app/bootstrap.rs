use bird_player::audio;
use bird_player::Player;

use crate::app::db;
use crate::app::font;
use crate::app::icon::get_app_icon;
use crate::app::runtime;
use crate::app::viewport;
use crate::app::App;
use crate::app::LibraryCommand;
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc::channel;
use std::sync::Arc;

pub struct BirdBootCfg {
    pub db: Arc<db::Database>,
    pub lib_cmd_tx: std::sync::mpsc::Sender<LibraryCommand>,
    pub lib_cmd_rx: std::sync::mpsc::Receiver<LibraryCommand>,
    pub is_processing_ui_change: Arc<AtomicBool>,
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn start_app() -> Result<()> {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    // 初始化启动配置
    let database = Arc::new(db::Database::new()?);
    tracing::info!("Database initialized successfully");

    let (lib_cmd_tx, lib_cmd_rx) = channel();
    let is_processing_ui_change = Arc::new(AtomicBool::new(false));
    let is_processing_ui_change_thread = is_processing_ui_change.clone();

    let boot_cfg = BirdBootCfg {
        db: database,
        lib_cmd_tx,
        lib_cmd_rx,
        is_processing_ui_change,
    };

    // 初始化运行时状态
    let (audio_tx, audio_rx) = channel();
    let (ui_tx, ui_rx) = channel();
    let cursor = Arc::new(AtomicU32::new(0));

    let player = Player::new(audio_tx, ui_rx, cursor);
    let runtime = runtime::BirdRuntime { player };

    let mut app = App::initialize_app().unwrap_or_default();

    // 设置启动配置和运行时
    app.boot_cfg = Some(boot_cfg);
    app.runtime = Some(runtime);

    // Load library and playlists before handing control to the UI loop
    app.load_heavy_data();

    // Spawn audio playback thread
    let _audio_thread =
        audio::thread::spawn_audio_thread(audio_rx, ui_tx, is_processing_ui_change_thread);

    // Try to load app icon, but continue without it if not found
    let icon = get_app_icon();
    let native_options = if let Some(icon_data) = icon {
        viewport::build_viewport_with_icon(icon_data)
    } else {
        tracing::warn!("Starting without app icon");
        viewport::build_viewport_without_icon()
    };

    eframe::run_native(
        "Bird Player",
        native_options,
        Box::new(|cc| {
            // Initialize image loaders
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // Setup fonts with CJK support
            let fonts = font::setup_fonts();
            cc.egui_ctx.set_fonts(fonts);

            // Apply brand-aware visual tweaks (selection colour etc.)
            cc.egui_ctx.style_mut(|style| {
                crate::app::style::apply_brand_visuals(&mut style.visuals);
            });

            Ok(Box::new(app))
        }),
    )?;

    Ok(())
}
