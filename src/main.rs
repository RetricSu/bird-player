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

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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

fn main() -> Result<()> {
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

    let runtime = BirdRuntime { player };

    // 加载 App 基础状态
    let mut app = App::load_basic().unwrap_or_default();

    // 设置启动配置和运行时
    app.boot_cfg = Some(boot_cfg);
    app.runtime = Some(runtime);

    // Spawn audio playback thread
    let _audio_thread =
        audio::thread::spawn_audio_thread(audio_rx, ui_tx, is_processing_ui_change_thread);

    // Try multiple possible icon paths for both development and bundled app scenarios
    let icon = get_app_icon().ok_or("Failed to load app icon")?;
    let native_options = app::viewport::build_viewport_with_icon(icon);

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
    )?;

    Ok(())
}
