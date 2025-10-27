use crate::app::icon::get_app_icon;
pub use crate::app::player::Player;
pub use crate::app::App;
pub use crate::app::*;

use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::mpsc::channel;
use std::sync::Arc;

use eframe::egui;

mod app;
mod audio;
mod db;

// 启动配置 - 启动时一次性搞定
pub struct BirdBootCfg {
    pub db: Arc<db::Database>,
    pub lib_cmd_tx: std::sync::mpsc::Sender<LibraryCommand>,
    pub lib_cmd_rx: std::sync::mpsc::Receiver<LibraryCommand>,
    pub is_processing_ui_change: Arc<AtomicBool>,
}

// 运行期状态 - 运行期一定存在
pub struct BirdRuntime {
    pub player: Player,
}

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("App booting...");

    // 初始化启动配置 - 失败直接 panic
    let database = Arc::new(db::Database::new().expect("Failed to initialize database"));
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

    let runtime = BirdRuntime { player };

    // 加载 App 基础状态
    let mut app = App::load_basic().unwrap_or_default();

    // 设置启动配置和运行时
    app.boot_cfg = Some(boot_cfg);
    app.runtime = Some(runtime);

    // Try multiple possible icon paths for both development and bundled app scenarios
    let icon_result = get_app_icon();

    // Create the native options with viewport settings
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT])
            .with_min_inner_size([300.0, 0.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(true),
        ..Default::default()
    };

    // Apply the icon if available
    let native_options = if let Some(icon) = icon_result {
        eframe::NativeOptions {
            viewport: native_options.viewport.with_icon(icon),
            ..native_options
        }
    } else {
        native_options
    };

    // Spawn audio playback thread
    let _audio_thread =
        audio::thread::spawn_audio_thread(audio_rx, ui_tx, is_processing_ui_change_thread);

    eframe::run_native(
        "Bird Player",
        native_options,
        Box::new(|cc| {
            // Initialize image loaders
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // Setup fonts with CJK support
            let fonts = app::font::setup_fonts();
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(app))
        }),
    )
    .expect("eframe failed: I should change main to return a result and use anyhow");
}
