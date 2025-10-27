use crate::app::icon::get_app_icon;
use crate::app::library::LibraryItem;
pub use crate::app::player::Player;
pub use crate::app::App;
pub use crate::app::*;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;

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

    // Audio output setup - 使用状态机模式重构
    let _audio_thread = thread::spawn(move || {
        use audio::state_machine::*;

        // 初始化音频上下文
        let mut ctx = AudioContext {
            engine: AudioEngineState {
                reader: None,
                audio_output: None,
                track_num: None,
                seek: None,
                decode_opts: None,
                track_info: None,
                duration: 0,
                timebase: 1000,
            },
            decoder: None,
            volume: 1.0,
            current_track_path: None,
            ui_tx,
            timer: std::time::Instant::now(),
            last_ts: 0,
        };

        // 创建状态机
        let mut state_machine = StateMachine::new();

        loop {
            // 处理命令并转换状态
            if let Ok(cmd) = audio_rx.try_recv() {
                let new_state: Option<Box<dyn State>> = match cmd {
                    AudioCommand::Seek(seconds) => {
                        tracing::info!("Processing SEEK command for {} seconds", seconds);
                        Some(Box::new(SeekToState::new(seconds)))
                    }
                    AudioCommand::Stop => {
                        tracing::info!("Processing STOP command");
                        Some(Box::new(StoppedState))
                    }
                    AudioCommand::Pause => {
                        tracing::info!("Processing PAUSE command");
                        Some(Box::new(PausedState))
                    }
                    AudioCommand::Play => {
                        tracing::info!("Processing PLAY command");
                        Some(Box::new(PlayingState))
                    }
                    AudioCommand::LoadFile(path) => {
                        tracing::info!("Processing LOAD FILE command for path: {:?}", &path);
                        Some(Box::new(LoadFileState::new(path)))
                    }
                    AudioCommand::SetVolume(vol) => {
                        tracing::info!("Processing SET VOLUME command to: {:?}", &vol);
                        ctx.volume = vol;
                        is_processing_ui_change_thread.store(false, Ordering::Relaxed);
                        None
                    }
                    _ => {
                        tracing::warn!("Unhandled case in audio command loop");
                        None
                    }
                };

                if let Some(state) = new_state {
                    state_machine.transition_to(state, &mut ctx);
                }
            }

            // 更新当前状态
            state_machine.update(&mut ctx);

            // 根据状态决定是否让出 CPU
            let current_state = state_machine.current_state_name();
            if current_state != "Playing" {
                std::thread::yield_now();
            }
        }
    }); // Audio Thread end

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

// Function to restore player state from saved settings
pub fn restore_player_state(app: &mut App) {
    tracing::info!("Restoring player state...");
    tracing::info!("Last track path: {:?}", app.last_track_path);
    tracing::info!("Last position: {:?}", app.last_position);
    tracing::info!("Was playing: {:?}", app.was_playing);
    tracing::info!("Number of playlists: {}", app.playlists.len());

    // Restore volume if it was saved
    let volume_to_restore = app.last_volume;
    if let Some(volume) = volume_to_restore {
        let is_processing = app.is_processing_ui_change().clone();
        let player = app.player_mut_ref();
        player.set_volume(volume, &is_processing);
        tracing::info!("Restored volume: {}", volume);
    }

    // Restore playback mode if it was saved
    if let Some(mode) = app.last_playback_mode {
        let player = app.player_mut_ref();
        player.playback_mode = mode;
        tracing::info!("Restored playback mode: {:?}", mode);
    }

    // If there was a playing track, try to find and load it
    let track_path_clone = app.last_track_path.clone();
    if let Some(track_path) = track_path_clone {
        // Search through all playlists for the track
        let mut found_track: Option<(usize, LibraryItem)> = None;
        for (playlist_idx, playlist) in app.playlists.iter().enumerate() {
            if let Some(track) = playlist
                .tracks
                .iter()
                .find(|track| track.path() == track_path)
            {
                found_track = Some((playlist_idx, (*track).clone()));
                break;
            }
        }

        if let Some((playlist_idx, track)) = found_track {
            tracing::info!("Restoring track: {:?}", track_path);

            // Cache values before borrowing player mutably
            let position_to_restore = app.last_position;
            let was_playing = app.was_playing;

            let player = app.player_mut_ref();
            // Set the selected track
            player.select_track(Some(track));

            // Set the seek position if available
            if let Some(position) = position_to_restore {
                tracing::info!("Restoring position: {} ms", position);
                player.seek_to(position);
            }

            // Start playback if it was playing when the app was closed
            if let Some(true) = was_playing {
                tracing::info!("Resuming playback");
                player.play();
                // Set the playlist containing the track as the playing playlist
                app.playing_playlist_idx = Some(playlist_idx);
            }

            // Mark that lyrics should be fetched after initialization
            app.should_fetch_lyrics_on_init = true;
        } else {
            tracing::warn!("Cannot find saved track in any playlist: {:?}", track_path);
        }
    } else {
        tracing::info!("No previous track to restore");
    }

    // Clear the saved state now that we've restored it (or failed to)
    app.last_track_path = None;
    app.last_position = None;
    app.last_playback_mode = None; // Keep the mode in memory
}
