/// Audio processing utilities shared by state machine
use std::path::PathBuf;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use super::state_machine::{AudioEngineState, SeekPosition};

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
            if let Err(err) = super::reader::setup_audio_reader(audio_engine_state) {
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
            let mut exact_duration_ms = None;
            
            // Primary method: extract precise duration using codec params
            if let Some(n_frames) = track.codec_params.n_frames {
                if let Some(tb) = tb {
                    // Calc duration via frames * (time_base_num / time_base_den)
                    
                    // Frac is scaled by tb.denom. Or rather, frac is directly 
                    // proportional to the time_base denominator. But calc_time yields fractional
                    // part in terms of a rational but it has a specific fraction value.
                    // Instead of using 'frac', using raw arithmetic is absolutely precise:
                    let raw_duration_ms = (n_frames as f64 * tb.numer as f64 / tb.denom as f64 * 1000.0) as u64;
                    exact_duration_ms = Some(raw_duration_ms);
                } else if let Some(sample_rate) = track.codec_params.sample_rate {
                    // Fallback using sample rate
                    let raw_duration_ms = (n_frames as f64 / sample_rate as f64 * 1000.0) as u64;
                    exact_duration_ms = Some(raw_duration_ms);
                }
            }

            // Fallback method: estimate based on file size and typical bitrate if exact isn't available
            let mut estimated_duration_ms = 0;
            if exact_duration_ms.is_none() {
                let file_size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
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
            }

            // Set final duration, preferring exact duration if possible
            if let Some(exact) = exact_duration_ms {
                audio_engine_state.duration = exact;
            } else {
                // Ensure minimum duration for music files ONLY if we relied on fallback (2 minutes = 120,000 ms)
                audio_engine_state.duration = estimated_duration_ms.max(120000);
            }

            if exact_duration_ms.is_none() && estimated_duration_ms < 120000 {
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
