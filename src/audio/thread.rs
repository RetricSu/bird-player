/// Audio playback thread management
///
/// This module handles the audio thread initialization and main loop
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

use crate::app::{AudioCommand, UiCommand};

use super::state_machine::*;

/// Spawn the audio playback thread
///
/// # Arguments
/// * `audio_rx` - Receiver for audio commands from the UI/player
/// * `ui_tx` - Sender for sending status updates to the UI
/// * `is_processing_ui_change` - Atomic flag to indicate UI change processing status
///
/// # Returns
/// A `JoinHandle` for the spawned audio thread
pub fn spawn_audio_thread(
    audio_rx: Receiver<AudioCommand>,
    ui_tx: Sender<UiCommand>,
    is_processing_ui_change: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        run_audio_loop(audio_rx, ui_tx, is_processing_ui_change);
    })
}

/// Main audio processing loop
///
/// This function contains the core audio thread logic using the state machine pattern.
/// It processes incoming commands, updates the current state, and manages state transitions.
fn run_audio_loop(
    audio_rx: Receiver<AudioCommand>,
    ui_tx: Sender<UiCommand>,
    is_processing_ui_change: Arc<AtomicBool>,
) {
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
            if let Some(new_state) =
                super::cmd::process_audio_command(cmd, &mut ctx, &is_processing_ui_change)
            {
                state_machine.transition_to(new_state, &mut ctx);
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
}
