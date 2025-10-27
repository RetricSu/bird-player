use crate::app::icon::get_app_icon;
use crate::app::library::LibraryItem;
pub use crate::app::player::Player;
pub use crate::app::App;
pub use crate::app::*;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;

use eframe::egui;
use symphonia::core::codecs::{DecoderOptions, FinalizeResult, CODEC_TYPE_NULL};
use symphonia::core::errors::{Error, Result};
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo, Track};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

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

    // Audio output setup
    let _audio_thread = thread::spawn(move || {
        let mut state = PlayerState::Unstarted;

        let mut audio_engine_state = AudioEngineState {
            reader: None,
            audio_output: None,
            track_num: None,
            seek: None,
            decode_opts: None,
            track_info: None,
            duration: 0,
            timebase: 1000, // Default to milliseconds, will be updated when loading file
        };

        let mut decoder: Option<Box<dyn symphonia::core::codecs::Decoder>> = None;
        let mut volume = 1.0;
        let mut current_track_path: Option<PathBuf> = None;
        let mut timer = std::time::Instant::now();
        let mut last_ts = 0; // Track last timestamp to avoid duplicate updates

        loop {
            // Process any pending commands
            process_audio_cmd(
                &audio_rx,
                &mut state,
                &mut volume,
                &is_processing_ui_change_thread,
            );

            match state {
                PlayerState::Playing => {
                    // decode the next packet.
                    let result: std::result::Result<(), symphonia::core::errors::Error> = 'once: {
                        if state != PlayerState::Playing {
                            tracing::info!("AudioThread Playing - Got a different state, bailing");
                            break 'once Ok(());
                        }

                        // Check if we have a valid reader and track info before proceeding
                        let reader = match audio_engine_state.reader.as_mut() {
                            Some(reader) => reader,
                            None => {
                                tracing::warn!("AudioThread Playing - No reader available, switching to stopped");
                                state = PlayerState::Stopped;
                                ui_tx
                                    .send(UiCommand::AudioFinished)
                                    .expect("Failed to send audio finished to ui thread");
                                break 'once Ok(());
                            }
                        };

                        let play_opts = match audio_engine_state.track_info {
                            Some(opts) => opts,
                            None => {
                                tracing::warn!("AudioThread Playing - No track info available, switching to stopped");
                                state = PlayerState::Stopped;
                                ui_tx
                                    .send(UiCommand::AudioFinished)
                                    .expect("Failed to send audio finished to ui thread");
                                break 'once Ok(());
                            }
                        };

                        let audio_output = &mut audio_engine_state.audio_output;
                        // Get the next packet from the format reader.
                        let packet = match reader.next_packet() {
                            Ok(packet) => packet,
                            Err(err) => {
                                tracing::warn!("couldn't decode next packet");
                                // Track is over.. update the state to stopped and send message to
                                // UI to play next track
                                state = PlayerState::Stopped;
                                ui_tx
                                    .send(UiCommand::AudioFinished)
                                    .expect("Failed to send play to ui thread");
                                break 'once Err(err);
                            }
                        };

                        // If the packet does not belong to the selected track, skip it.
                        if packet.track_id() != play_opts.track_id {
                            tracing::warn!("packet track id doesn't match track id");
                            break 'once Ok(());
                        }

                        // Send timestamp updates more frequently for smooth synced lyrics
                        let current_time = timer.elapsed();
                        if current_time > std::time::Duration::from_millis(100)
                            && (packet.ts > last_ts + 100 || packet.ts < last_ts)
                        // Update every 100ms if changed by more than 100ms or went backwards
                        {
                            // Convert packet timestamp from timebase units to milliseconds
                            let timestamp_ms = if audio_engine_state.timebase > 0 {
                                ((packet.ts() as f64 / audio_engine_state.timebase as f64) * 1000.0)
                                    as u64
                            } else {
                                packet.ts()
                            };

                            ui_tx
                                .send(UiCommand::CurrentTimestamp(timestamp_ms))
                                .expect("Failed to send timestamp to ui thread");

                            timer = std::time::Instant::now();
                            last_ts = packet.ts();
                        }

                        // Decode the packet into audio samples.
                        match decoder.as_mut().unwrap().decode(&packet) {
                            Ok(decoded) => {
                                // If the audio output is not open, try to open it.
                                if audio_output.is_none() {
                                    // Get the audio buffer specification. This is a description of the decoded
                                    // audio buffer's sample format and sample rate.
                                    let spec = *decoded.spec();

                                    // Get the capacity of the decoded buffer. Note that this is capacity, not
                                    // length! The capacity of the decoded buffer is constant for the life of the
                                    // decoder, but the length is not.
                                    let duration = decoded.capacity() as u64;

                                    // Try to open the audio output.
                                    audio_output
                                        .replace(audio::output::try_open(spec, duration).unwrap());
                                } else {
                                    // TODO: Check the audio spec. and duration hasn't changed.
                                }

                                // Write the decoded audio samples to the audio output if the presentation timestamp
                                // for the packet is >= the seeked position (0 if not seeking).
                                if packet.ts() >= play_opts.seek_ts {
                                    if let Some(audio_output) = audio_output {
                                        audio_output.write(decoded, volume).unwrap();
                                    }
                                }

                                Ok(())
                            }
                            Err(Error::DecodeError(err)) => {
                                // Decode errors are not fatal. Print the error message and try to decode the next
                                // packet as usual.
                                tracing::warn!("decode error: {}", err);
                                break 'once Ok(());
                            }
                            Err(err) => break 'once Err(err),
                        }

                        //Ok(())
                    };

                    // Return if a fatal error occured.
                    ignore_end_of_stream_error(result)
                        .expect("Encountered some other error than EoF");
                }
                PlayerState::Stopped => {
                    // This is kind of a hack to get stopping to work. Flush the buffer so there is
                    // nothing left in the resampler, but the decoder needs to be reset. This is as
                    // simple as reloading the current track so the next time it plays from the
                    // beginning.
                    if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                        tracing::info!("Audio Thread Stopped - flushing output");
                        audio_output.flush()
                    }

                    if let Some(ref current_track_path) = current_track_path {
                        // Finalize the decoder before loading a new track
                        if let Some(decoder) = decoder.as_mut() {
                            _ = do_verification(decoder.finalize());
                        }

                        if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                            audio_output.flush()
                        }

                        audio_engine_state.audio_output = None;

                        load_file(current_track_path, &mut audio_engine_state, &mut decoder, 0);

                        ui_tx
                            .send(UiCommand::CurrentTimestamp(0))
                            .expect("Failed to send play to ui thread");

                        state = PlayerState::Unstarted;
                    }
                }
                PlayerState::SeekTo(seek_timestamp) => {
                    tracing::info!("AudioThread Seeking");
                    if let Some(ref current_track_path) = current_track_path {
                        // Stop current playback
                        if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                            audio_output.flush()
                        }

                        audio_engine_state.audio_output = None;

                        load_file(
                            current_track_path,
                            &mut audio_engine_state,
                            &mut decoder,
                            seek_timestamp,
                        );

                        // Check if the load was successful before proceeding
                        if audio_engine_state.reader.is_some()
                            && audio_engine_state.track_info.is_some()
                        {
                            state = PlayerState::Playing;

                            // Update UI with playing state to ensure synchronization
                            ui_tx
                                .send(UiCommand::PlaybackStateChanged(true))
                                .expect("Failed to send playback state to ui thread");
                        } else {
                            // Loading failed during seek - notify UI and set to stopped
                            tracing::warn!("Failed to reload audio file during seek");
                            ui_tx
                                .send(UiCommand::AudioFinished)
                                .expect("Failed to send audio finished to ui thread");
                            state = PlayerState::Stopped;
                        }
                    }
                }
                PlayerState::LoadFile(ref path) => {
                    tracing::info!("AudioThread Loading File");
                    // Stop current playback
                    if let Some(audio_output) = audio_engine_state.audio_output.as_mut() {
                        tracing::info!("AudioThread Loading File - Flushing output");
                        audio_output.flush()
                    }

                    // Finalize the current decoder before loading new file
                    if let Some(decoder) = decoder.as_mut() {
                        _ = do_verification(decoder.finalize());
                    }

                    audio_engine_state.audio_output = None;

                    current_track_path = Some((*path).clone());
                    load_file(path, &mut audio_engine_state, &mut decoder, 0);

                    // Check if the load was successful before proceeding
                    if audio_engine_state.reader.is_some()
                        && audio_engine_state.track_info.is_some()
                    {
                        // TODO - Get total u64 track duration and send to Ui
                        ui_tx
                            .send(UiCommand::TotalTrackDuration(audio_engine_state.duration))
                            .expect("Failed to send play to audio thread");

                        state = PlayerState::Playing;
                    } else {
                        // Loading failed - notify UI and set to stopped
                        tracing::warn!("Failed to load audio file: {:?}", path);
                        ui_tx
                            .send(UiCommand::AudioFinished)
                            .expect("Failed to send audio finished to ui thread");
                        state = PlayerState::Stopped;
                        current_track_path = None;
                    }
                }
                PlayerState::Paused => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                PlayerState::Unstarted => {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }

            // Yield to other threads if we're not actively playing
            if state != PlayerState::Playing {
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

            // Create font definitions - start with defaults so we have fallbacks
            let mut fonts = egui::FontDefinitions::default();

            // Try to find a system font with CJK support
            let source = font_kit::source::SystemSource::new();

            // Define font names to try based on OS for better CJK support
            let font_names: Vec<&str> = match std::env::consts::OS {
                "macos" => vec!["PingFang SC", "Hiragino Sans GB", "STSong", "Heiti SC"],
                "windows" => vec!["Microsoft YaHei", "SimSun", "SimHei", "MS Gothic"],
                "linux" => vec!["Noto Sans CJK TC"],
                _ => vec![], // Empty for other OSes - we'll use generic fallback
            };

            // Try to find one of the preferred fonts
            let mut found_font = false;
            for font_name in font_names {
                // Get family by name
                if let Ok(family_handle) = source.select_family_by_name(font_name) {
                    // For the first font in the family
                    if let Some(font_handle) = family_handle.fonts().first() {
                        if let Ok(font_data) = match font_handle {
                            font_kit::handle::Handle::Memory { bytes, .. } => Ok(bytes.to_vec()),
                            font_kit::handle::Handle::Path { path, .. } => std::fs::read(path),
                        } {
                            // Register the font with egui
                            const SYSTEM_FONT_NAME: &str = "SystemCJKFont";
                            fonts.font_data.insert(
                                SYSTEM_FONT_NAME.to_owned(),
                                egui::FontData::from_owned(font_data).into(),
                            );

                            // Add as primary font for proportional text (at the beginning)
                            fonts
                                .families
                                .get_mut(&egui::FontFamily::Proportional)
                                .unwrap()
                                .insert(0, SYSTEM_FONT_NAME.to_owned());

                            // Also add to monospace as a fallback
                            fonts
                                .families
                                .get_mut(&egui::FontFamily::Monospace)
                                .unwrap()
                                .push(SYSTEM_FONT_NAME.to_owned());

                            tracing::info!("Using system font '{}' for CJK support", font_name);
                            found_font = true;
                            break;
                        }
                    }
                }
            }

            // If we couldn't find any preferred fonts, try a generic sans-serif as backup
            if !found_font {
                if let Ok(font_handle) = source.select_best_match(
                    &[font_kit::family_name::FamilyName::SansSerif],
                    &font_kit::properties::Properties::new(),
                ) {
                    if let Ok(font_data) = match font_handle {
                        font_kit::handle::Handle::Memory { bytes, .. } => Ok(bytes.to_vec()),
                        font_kit::handle::Handle::Path { path, .. } => std::fs::read(&path),
                    } {
                        const SYSTEM_FONT_NAME: &str = "SystemFont";
                        fonts.font_data.insert(
                            SYSTEM_FONT_NAME.to_owned(),
                            egui::FontData::from_owned(font_data).into(),
                        );

                        // Add as primary font
                        fonts
                            .families
                            .get_mut(&egui::FontFamily::Proportional)
                            .unwrap()
                            .insert(0, SYSTEM_FONT_NAME.to_owned());

                        tracing::info!("Using generic system font for text");
                    } else {
                        tracing::warn!("Could not load system font data, using defaults");
                    }
                } else {
                    tracing::warn!("Could not find suitable system font, using defaults");
                }
            }

            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(app))
        }),
    )
    .expect("eframe failed: I should change main to return a result and use anyhow");
}

fn process_audio_cmd(
    audio_rx: &Receiver<AudioCommand>,
    state: &mut PlayerState,
    volume: &mut f32,
    is_processing_ui_change: &Arc<AtomicBool>,
) {
    match audio_rx.try_recv() {
        Ok(cmd) => {
            //Process Start
            match cmd {
                AudioCommand::Seek(seconds) => {
                    tracing::info!("Processing SEEK command for {} seconds", seconds);
                    *state = PlayerState::SeekTo(seconds);
                }
                AudioCommand::Stop => {
                    tracing::info!("Processing STOP command");
                    *state = PlayerState::Stopped;
                }
                AudioCommand::Pause => {
                    tracing::info!("Processing PAUSE command");
                    *state = PlayerState::Paused;
                }
                AudioCommand::Play => {
                    tracing::info!("Processing PLAY command");
                    *state = PlayerState::Playing;
                }
                AudioCommand::LoadFile(path) => {
                    tracing::info!("Processing LOAD FILE command for path: {:?}", &path);
                    *state = PlayerState::LoadFile(path);
                }
                AudioCommand::SetVolume(vol) => {
                    tracing::info!("Processing SET VOLUME command to: {:?}", &vol);
                    *volume = vol;
                    is_processing_ui_change.store(false, Ordering::Relaxed);
                }
                _ => tracing::warn!("Unhandled case in audio command loop"),
            }
        }
        Err(_) => (), // When no commands are sent, this will evaluate. aka - it is the
                      // common case. No need to print anything
    }
}

enum SeekPosition {
    Timestamp(u64),
}

#[derive(Copy, Clone)]
struct PlayTrackOptions {
    track_id: u32,
    seek_ts: u64,
}

#[derive(Debug, PartialEq)]
pub enum PlayerState {
    Unstarted,
    Stopped,
    Playing,
    Paused,
    LoadFile(PathBuf),
    SeekTo(u64),
}

struct AudioEngineState {
    pub reader: Option<Box<dyn FormatReader>>,
    pub audio_output: Option<Box<dyn audio::output::AudioOutput>>,
    pub track_num: Option<usize>,
    pub seek: Option<SeekPosition>,
    pub decode_opts: Option<DecoderOptions>,
    pub track_info: Option<PlayTrackOptions>,
    pub duration: u64,
    pub timebase: u64, // Timebase in Hz (ticks per second)
}

fn load_file(
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
                // print_tracks(reader.tracks());
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

fn ignore_end_of_stream_error(result: Result<()>) -> Result<()> {
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

fn do_verification(finalization: FinalizeResult) -> Result<i32> {
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
