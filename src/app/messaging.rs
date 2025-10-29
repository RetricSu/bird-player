use std::path::PathBuf;

/// Commands sent from the UI layer to the audio engine.
#[derive(Debug, Clone)]
pub enum AudioCommand {
    Stop,
    Play,
    Pause,
    Seek(u64),
    LoadFile(PathBuf),
    Select(usize),
    SetVolume(f32),
}

/// Notifications emitted by the audio backend toward the UI.
#[derive(Debug, Clone)]
pub enum UiCommand {
    AudioFinished,
    TotalTrackDuration(u64),
    CurrentTimestamp(u64),
    PlaybackStateChanged(bool),
}
