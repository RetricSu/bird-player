/// Audio processing utilities shared by state machine
use std::path::PathBuf;
use symphonia::core::codecs::{DecoderOptions, FinalizeResult, CODEC_TYPE_NULL};
use symphonia::core::errors::{Error, Result};
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::state_machine::{AudioEngineState, PlayTrackOptions, SeekPosition};

pub fn load_file(
    path: &PathBuf,
    audio_engine_state: &mut AudioEngineState,
    decoder: &mut Option<Box<dyn symphonia::core::codecs::Decoder>>,
    seek_timestamp: u64,
) {
    let hint = Hint::new();
    let source = Box::new(std::fs::File::open(path).expect("couldn't open file"));
    let mss = MediaSourceStream::new(source, Default::default());
    let format_opts = FormatOptions {
        enable_gapless: true,
        ..Default::default()
    };
    let metadata_opts: MetadataOptions = Default::default();
    let seek = Some(SeekPosition::Timestamp(seek_timestamp));

    match symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts) {
        Ok(probed) => {
            // Set the decoder options.
            let decode_opts = DecoderOptions { verify: true };

            audio_engine_state.reader = Some(probed.format);
            audio_engine_state.decode_opts = Some(decode_opts);
            audio_engine_state.seek = seek;

            // Configure everything for playback.
            if let Err(err) = setup_audio_reader(audio_engine_state) {
                tracing::warn!("Failed to setup audio reader: {}", err);
                // Reset the audio engine state to prevent crashes
                audio_engine_state.reader = None;
                audio_engine_state.track_info = None;
                audio_engine_state.decode_opts = None;
                audio_engine_state.seek = None;
                audio_engine_state.duration = 0;
                *decoder = None;
                return;
            }

            let reader = match audio_engine_state.reader.as_mut() {
                Some(reader) => reader,
                None => {
                    tracing::warn!("Reader is None after setup");
                    return;
                }
            };

            let play_opts = match audio_engine_state.track_info {
                Some(opts) => opts,
                None => {
                    tracing::warn!("Track info is None after setup");
                    return;
                }
            };

            let decode_opts = match audio_engine_state.decode_opts {
                Some(opts) => opts,
                None => {
                    tracing::warn!("Decode opts is None after setup");
                    return;
                }
            };

            let track = match reader
                .tracks()
                .iter()
                .find(|track| track.id == play_opts.track_id)
            {
                Some(track) => track,
                _ => {
                    tracing::warn!("Couldn't find track");
                    return;
                }
            };

            // Create a decoder for the track.
            *decoder = Some(
                symphonia::default::get_codecs()
                    .make(&track.codec_params, &decode_opts)
                    .expect("Failed to get decoder"),
            );

            // Get the selected track's timebase and duration.
            let tb = track.codec_params.time_base;
            let sample_rate = track.codec_params.sample_rate;
            tracing::debug!(
                "Codec params - time_base: {:?}, sample_rate: {:?}",
                tb,
                sample_rate
            );

            // Store the timebase - use sample rate as the most reliable source
            if let Some(sample_rate) = track.codec_params.sample_rate {
                audio_engine_state.timebase = sample_rate as u64;
                tracing::debug!("Using sample rate {} as timebase", sample_rate);
            } else if let Some(time_base) = tb {
                // Fallback to timebase calculation if sample_rate is not available
                let tb_hz = time_base.numer as f64 / time_base.denom as f64;
                audio_engine_state.timebase = tb_hz as u64;
                tracing::debug!(
                    "Using timebase calculation: {} Hz ({} / {})",
                    tb_hz,
                    time_base.numer,
                    time_base.denom
                );
            } else {
                tracing::warn!("No timebase or sample rate available, using default 44100");
                audio_engine_state.timebase = 44100; // Common default for audio
            }

            // Convert duration to milliseconds
            // Primary method: estimate based on file size and typical bitrate
            let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let mut estimated_duration_ms = 0;

            if file_size_bytes > 100000 {
                // Only use file size if it's a reasonable size (>100KB)
                // Assume 160 kbps MP3 = 160 * 1024 / 8 = 20480 bytes per second
                let bytes_per_second = 160 * 1024 / 8; // 20480
                estimated_duration_ms = (file_size_bytes * 1000) / bytes_per_second as u64;
                tracing::debug!(
                    "Estimated duration from file size: {} ms (file size: {} bytes, {} bytes/sec)",
                    estimated_duration_ms,
                    file_size_bytes,
                    bytes_per_second
                );
            }

            // Ensure minimum duration for music files (2 minutes = 120,000 ms)
            audio_engine_state.duration = estimated_duration_ms.max(120000);

            if estimated_duration_ms < 120000 {
                tracing::debug!(
                    "Using minimum duration: {} ms (estimated was {} ms)",
                    audio_engine_state.duration,
                    estimated_duration_ms
                );
            }

            tracing::info!(
                "Track Duration: {} ms, TimeBase: {} Hz",
                audio_engine_state.duration,
                audio_engine_state.timebase
            );
        }
        Err(err) => {
            // The input was not supported by any format reader.
            tracing::warn!("the audio format is not supported: {}", err);

            // Reset the audio engine state to prevent crashes
            audio_engine_state.reader = None;
            audio_engine_state.track_info = None;
            audio_engine_state.decode_opts = None;
            audio_engine_state.seek = None;
            audio_engine_state.duration = 0;

            // Clear any existing decoder
            *decoder = None;
        }
    }
}

fn setup_audio_reader(audio_engine_state: &mut AudioEngineState) -> Result<i32> {
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

fn first_supported_track(tracks: &[Track]) -> Option<&Track> {
    tracks
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
}

pub fn ignore_end_of_stream_error(result: Result<()>) -> Result<()> {
    match result {
        Err(Error::IoError(err))
            if err.kind() == std::io::ErrorKind::UnexpectedEof
                && err.to_string() == "end of stream" =>
        {
            // Do not treat "end of stream" as a fatal error. It's the currently only way a
            // format reader can indicate the media is complete.
            Ok(())
        }
        _ => result,
    }
}

pub fn do_verification(finalization: FinalizeResult) -> Result<i32> {
    match finalization.verify_ok {
        Some(is_ok) => {
            // Got a verification result.
            tracing::info!("verification: {}", if is_ok { "passed" } else { "failed" });

            Ok(i32::from(!is_ok))
        }
        // Verification not enabled by user, or unsupported by the codec.
        _ => Ok(0),
    }
}
