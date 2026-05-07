use std::path::PathBuf;

/// Commands sent from the consumer layer to the audio engine.
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

/// Notifications emitted by the audio backend toward the consumer.
#[derive(Debug, Clone)]
pub enum AudioEvent {
    AudioFinished,
    TotalTrackDuration(u64),
    CurrentTimestamp(u64),
    PlaybackStateChanged(bool),
    TechnicalInfo {
        sample_rate: Option<u32>,
        channels: Option<u8>,
        codec: Option<String>,
    },
}
