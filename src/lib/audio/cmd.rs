/// Audio playback thread management
///
/// This module handles the audio thread initialization and main loop
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::AudioCommand;

use super::state_machine::*;

/// Process incoming audio commands and return the appropriate state transition
///
/// # Arguments
/// * `cmd` - The audio command to process
/// * `ctx` - Mutable reference to the audio context
/// * `is_processing_ui_change` - Flag for UI change processing
///
/// # Returns
/// An optional boxed State representing the state to transition to
pub fn process_audio_command(
    cmd: AudioCommand,
    ctx: &mut AudioContext,
    is_processing_ui_change: &Arc<AtomicBool>,
) -> Option<Box<dyn State>> {
    match cmd {
        AudioCommand::Seek(timestamp_ms) => {
            tracing::info!("Processing SEEK command for {} ms", timestamp_ms);
            Some(Box::new(SeekToState::new(timestamp_ms)))
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
            is_processing_ui_change.store(false, Ordering::Relaxed);
            None
        }
        _ => {
            tracing::warn!("Unhandled case in audio command loop");
            None
        }
    }
}
