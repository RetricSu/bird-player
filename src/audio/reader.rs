/// Audio processing utilities shared by state machine
use symphonia::core::codecs::CODEC_TYPE_NULL;
use symphonia::core::errors::{Error, Result};
use symphonia::core::formats::{SeekMode, SeekTo, Track};

use super::state_machine::{AudioEngineState, PlayTrackOptions, SeekPosition};

pub fn setup_audio_reader(audio_engine_state: &mut AudioEngineState) -> Result<i32> {
    // If the user provided a track number, select that track if it exists, otherwise, select the
    // first track with a known codec.
    let reader = match audio_engine_state.reader.as_mut() {
        Some(reader) => reader,
        None => {
            tracing::warn!("No reader available in setup_audio_reader");
            return Err(Error::Unsupported("No reader available"));
        }
    };
    let seek = &audio_engine_state.seek;

    let track = audio_engine_state
        .track_num
        .and_then(|t| reader.tracks().get(t))
        .or_else(|| first_supported_track(reader.tracks()));

    let mut track_id = match track {
        Some(track) => track.id,
        _ => return Ok(0),
    };

    // If seeking, seek the reader to the time or timestamp specified and get the timestamp of the
    // seeked position. All packets with a timestamp < the seeked position will not be played.
    //
    // Note: This is a half-baked approach to seeking! After seeking the reader, packets should be
    // decoded and *samples* discarded up-to the exact *sample* indicated by required_ts. The
    // current approach will discard excess samples if seeking to a sample within a packet.
    let seek_ts = if let Some(seek) = seek {
        let seek_to = match seek {
            SeekPosition::Timestamp(ts) => SeekTo::TimeStamp { ts: *ts, track_id },
        };

        // Attempt the seek. If the seek fails, ignore the error and return a seek timestamp of 0 so
        // that no samples are trimmed.
        match reader.seek(SeekMode::Accurate, seek_to) {
            Ok(seeked_to) => seeked_to.required_ts,
            Err(Error::ResetRequired) => {
                tracing::warn!("reset required...");
                if let Some(track) = first_supported_track(reader.tracks()) {
                    track_id = track.id;
                } else {
                    tracing::warn!("No supported tracks found after reset");
                    return Err(Error::Unsupported("No supported tracks after reset"));
                }
                0
            }
            Err(err) => {
                // Don't give-up on a seek error.
                tracing::warn!("seek error: {}", err);
                0
            }
        }
    } else {
        // If not seeking, the seek timestamp is 0.
        0
    };

    tracing::info!("seek ts: {}", seek_ts);

    audio_engine_state.track_info = Some(PlayTrackOptions { track_id, seek_ts });

    Ok(0)
}

pub fn first_supported_track(tracks: &[Track]) -> Option<&Track> {
    tracks
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
}
