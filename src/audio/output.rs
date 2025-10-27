// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Platform-dependant Audio Outputs

use std::result;

use symphonia::core::audio::{AudioBufferRef, SignalSpec};
use symphonia::core::units::Duration;

pub trait AudioOutput {
    fn write(&mut self, decoded: AudioBufferRef<'_>, volume: f32) -> Result<()>;
    fn flush(&mut self);
}

#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum AudioOutputError {
    OpenStreamError,
    PlayStreamError,
    StreamClosedError,
    ResampleError,
}

pub type Result<T> = result::Result<T, AudioOutputError>;

#[cfg(all(target_os = "linux", feature = "pulseaudio"))]
mod pulseaudio {
    use super::{AudioOutput, AudioOutputError, Result};

    use symphonia::core::audio::*;
    use symphonia::core::units::Duration;

    use libpulse_binding as pulse;
    use libpulse_simple_binding as psimple;

    use log::{error, warn};

    pub struct PulseAudioOutput {
        pa: psimple::Simple,
        sample_buf: RawSampleBuffer<f32>,
    }

    impl PulseAudioOutput {
        pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
            // An interleaved buffer is required to send data to PulseAudio. Use a SampleBuffer to
            // move data between Symphonia AudioBuffers and the byte buffers required by PulseAudio.
            let sample_buf = RawSampleBuffer::<f32>::new(duration, spec);

            // Create a PulseAudio stream specification.
            let pa_spec = pulse::sample::Spec {
                format: pulse::sample::Format::FLOAT32NE,
                channels: spec.channels.count() as u8,
                rate: spec.rate,
            };

            assert!(pa_spec.is_valid());

            let pa_ch_map = map_channels_to_pa_channelmap(spec.channels);

            // PulseAudio seems to not play very short audio buffers, use these custom buffer
            // attributes for very short audio streams.
            //
            // let pa_buf_attr = pulse::def::BufferAttr {
            //     maxlength: std::u32::MAX,
            //     tlength: 1024,
            //     prebuf: std::u32::MAX,
            //     minreq: std::u32::MAX,
            //     fragsize: std::u32::MAX,
            // };

            // Create a PulseAudio connection.
            let pa_result = psimple::Simple::new(
                None,                               // Use default server
                "Symphonia Player",                 // Application name
                pulse::stream::Direction::Playback, // Playback stream
                None,                               // Default playback device
                "Music",                            // Description of the stream
                &pa_spec,                           // Signal specification
                pa_ch_map.as_ref(),                 // Channel map
                None,                               // Custom buffering attributes
            );

            match pa_result {
                Ok(pa) => Ok(Box::new(PulseAudioOutput { pa, sample_buf })),
                Err(err) => {
                    error!("audio output stream open error: {}", err);

                    Err(AudioOutputError::OpenStreamError)
                }
            }
        }
    }

    impl AudioOutput for PulseAudioOutput {
        fn write(&mut self, decoded: AudioBufferRef<'_>, volume: f32) -> Result<()> {
            // Do nothing if there are no audio frames.
            if decoded.frames() == 0 {
                return Ok(());
            }

            // Interleave samples from the audio buffer into the sample buffer.
            self.sample_buf.copy_interleaved_ref(decoded);

            // Apply volume adjustment
            if volume != 1.0 {
                // Use a temporary buffer to apply volume
                let buf_bytes = self.sample_buf.as_bytes();
                let sample_count = buf_bytes.len() / std::mem::size_of::<f32>();

                // Create a buffer with the samples
                let mut volume_adjusted = Vec::with_capacity(buf_bytes.len());
                volume_adjusted.extend_from_slice(buf_bytes);

                // Convert the buffer to f32 samples and apply volume
                let samples_f32 = unsafe {
                    std::slice::from_raw_parts_mut(
                        volume_adjusted.as_mut_ptr() as *mut f32,
                        sample_count,
                    )
                };

                // Apply volume
                for sample in samples_f32 {
                    *sample *= volume;
                }

                // Write the volume-adjusted buffer to PulseAudio
                match self.pa.write(&volume_adjusted) {
                    Err(err) => {
                        error!("audio output stream write error: {}", err);
                        return Err(AudioOutputError::StreamClosedError);
                    }
                    _ => return Ok(()),
                }
            }

            // Write interleaved samples to PulseAudio.
            match self.pa.write(self.sample_buf.as_bytes()) {
                Err(err) => {
                    error!("audio output stream write error: {}", err);

                    Err(AudioOutputError::StreamClosedError)
                }
                _ => Ok(()),
            }
        }

        fn flush(&mut self) {
            // Flush is best-effort, ignore the returned result.
            let _ = self.pa.drain();
        }
    }

    /// Maps a set of Symphonia `Channels` to a PulseAudio channel map.
    fn map_channels_to_pa_channelmap(channels: Channels) -> Option<pulse::channelmap::Map> {
        let mut map: pulse::channelmap::Map = Default::default();
        map.init();
        map.set_len(channels.count() as u8);

        let is_mono = channels.count() == 1;

        for (i, channel) in channels.iter().enumerate() {
            map.get_mut()[i] = match channel {
                Channels::FRONT_LEFT if is_mono => pulse::channelmap::Position::Mono,
                Channels::FRONT_LEFT => pulse::channelmap::Position::FrontLeft,
                Channels::FRONT_RIGHT => pulse::channelmap::Position::FrontRight,
                Channels::FRONT_CENTRE => pulse::channelmap::Position::FrontCenter,
                Channels::REAR_LEFT => pulse::channelmap::Position::RearLeft,
                Channels::REAR_CENTRE => pulse::channelmap::Position::RearCenter,
                Channels::REAR_RIGHT => pulse::channelmap::Position::RearRight,
                Channels::LFE1 => pulse::channelmap::Position::Lfe,
                Channels::FRONT_LEFT_CENTRE => pulse::channelmap::Position::FrontLeftOfCenter,
                Channels::FRONT_RIGHT_CENTRE => pulse::channelmap::Position::FrontRightOfCenter,
                Channels::SIDE_LEFT => pulse::channelmap::Position::SideLeft,
                Channels::SIDE_RIGHT => pulse::channelmap::Position::SideRight,
                Channels::TOP_CENTRE => pulse::channelmap::Position::TopCenter,
                Channels::TOP_FRONT_LEFT => pulse::channelmap::Position::TopFrontLeft,
                Channels::TOP_FRONT_CENTRE => pulse::channelmap::Position::TopFrontCenter,
                Channels::TOP_FRONT_RIGHT => pulse::channelmap::Position::TopFrontRight,
                Channels::TOP_REAR_LEFT => pulse::channelmap::Position::TopRearLeft,
                Channels::TOP_REAR_CENTRE => pulse::channelmap::Position::TopRearCenter,
                Channels::TOP_REAR_RIGHT => pulse::channelmap::Position::TopRearRight,
                _ => {
                    // If a Symphonia channel cannot map to a PulseAudio position then return None
                    // because PulseAudio will not be able to open a stream with invalid channels.
                    warn!("failed to map channel {:?} to output", channel);
                    return None;
                }
            }
        }

        Some(map)
    }
}

#[cfg(all(target_os = "linux", feature = "pulseaudio"))]
pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
    pulseaudio::PulseAudioOutput::try_open(spec, duration)
}

// ---------------------------------------------------------------------------
// Audio Pipeline Overview (CpalStreamController)
// ---------------------------------------------------------------------------
// 1. OPENING A STREAM
//    - Pick the platform-appropriate config (Windows: use device default;
//      others: caller’s rate/channels).
//    - Create an SPSC ring buffer (≈ 170 ms @ 48 kHz) to decouple the
//      **producer** (decoder thread) from the **consumer** (CPAL callback).
//    - Build & start the output stream.  CPAL will now call the callback
//      every 5-10 ms asking for the next audio slice.
//
// 2. DECODER → RING  (write method)
//    - Resample if needed, then apply per-sample volume.
//    - Stream the data into the ring buffer in 128-frame chunks.
//      The call is **blocking** : if the ring is full we wait, so the
//      decoder naturally throttles to real-time speed.
//
// 3. RING → SOUND CARD  (callback)
//    - Callback runs on a high-priority audio thread.
//    - Read up to `output_slice.len()` frames from the ring.
//    - If insufficient, fill the rest with `T::MID` (silence) to avoid
//      underrun artefacts.
//
// 4. END OF TRACK / SEEK  (flush method)
//    - Push any latent frames left in the resampler into the ring.
//    - Pause the stream so the callback stops consuming.
//    - On the next play event the ring starts empty, preventing old
//      audio from bleeding into the new track.
//
// 5. LATENCY vs ROBUSTNESS
//    - Ring capacity = 170 ms : safe for local music playback, negligible
//      CPU cost (only memcpy), audible delay < 200 ms.
//    - Smaller rings give lower latency but higher underrun risk; adjust
//      `rb_capacity_frames` if interactive use (gaming, DJ) is required.
// ---------------------------------------------------------------------------
#[cfg(any(not(target_os = "linux"), not(feature = "pulseaudio")))]
mod cpal {
    use crate::audio::resampler::Resampler;

    use super::{AudioOutput, AudioOutputError, Result};

    use symphonia::core::audio::{AudioBufferRef, RawSample, SampleBuffer, SignalSpec};
    use symphonia::core::conv::{ConvertibleSample, IntoSample};
    use symphonia::core::units::Duration;

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use rb::*;

    use log::{error, info, warn};

    trait ScalableSample:
        cpal::Sample + ConvertibleSample + IntoSample<f32> + RawSample + std::marker::Send + 'static
    {
        fn mul(&self, n: f32) -> Self; // for adjusting volume
    }

    impl ScalableSample for f32 {
        fn mul(&self, n: f32) -> Self {
            self * n
        }
    }

    impl ScalableSample for i16 {
        fn mul(&self, n: f32) -> Self {
            // the result might overflow, so we clamp it for safety
            const MAX: f32 = i16::MAX as f32;
            const MIN: f32 = i16::MIN as f32;
            let scaled = *self as f32 * n;
            (scaled.clamp(MIN, MAX) as i32) as i16
        }
    }

    impl ScalableSample for u16 {
        fn mul(&self, n: f32) -> Self {
            // the result might overflow, so we clamp it for safety
            const MAX: f32 = u16::MAX as f32;
            const MIN: f32 = 0.0;
            let scaled = *self as f32 * n;
            (scaled.clamp(MIN, MAX) as i32) as u16
        }
    }

    pub struct CpalOutputDevice;

    impl CpalOutputDevice {
        pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
            let host = cpal::default_host();
            let device = host.default_output_device().ok_or_else(|| {
                error!("failed to get default audio output device");
                AudioOutputError::OpenStreamError
            })?;

            let config = device.default_output_config().map_err(|err| {
                error!("failed to get default audio output device config: {}", err);
                AudioOutputError::OpenStreamError
            })?;

            // Select proper playback routine based on sample format.
            match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    CpalStreamController::<f32>::try_open(spec, duration, &device)
                }
                cpal::SampleFormat::I16 => {
                    CpalStreamController::<i16>::try_open(spec, duration, &device)
                }
                cpal::SampleFormat::U16 => {
                    CpalStreamController::<u16>::try_open(spec, duration, &device)
                }
                _ => panic!("Unsupported sample format"),
            }
        }
    }

    struct CpalStreamController<T: ScalableSample> {
        sample_sender: rb::Producer<T>,
        interleaved_buffer: SampleBuffer<T>,
        stream: cpal::Stream,
        rate_converter: Option<Resampler<T>>,
    }

    impl<T: cpal::SizedSample + ScalableSample> CpalStreamController<T>
    where
        f32: cpal::FromSample<T>,
    {
        pub fn try_open(
            spec: SignalSpec,
            duration: Duration,
            device: &cpal::Device,
        ) -> Result<Box<dyn AudioOutput>> {
            let num_channels = spec.channels.count();

            let mut config = device
                .default_output_config()
                .map_err(|err| {
                    error!("failed to get default output config: {}", err);
                    AudioOutputError::OpenStreamError
                })?
                .config();

            #[cfg(not(target_os = "windows"))]
            {
                config.channels = num_channels as _;
                config.sample_rate = cpal::SampleRate(spec.rate);
                config.buffer_size = cpal::BufferSize::Default;
            }

            const TARGET_LATENCY_MS: usize = 170;
            let rb_capacity_frames = config.sample_rate.0 as usize * TARGET_LATENCY_MS / 1000;
            let sample_ring = SpscRb::new(rb_capacity_frames);
            let (sample_sender, sample_receiver) = (sample_ring.producer(), sample_ring.consumer());

            let output_stream = device
                .build_output_stream(
                    &config,
                    move |output_slice: &mut [T], _: &cpal::OutputCallbackInfo| {
                        let frames_needed = output_slice.len();
                        let frames_written = sample_receiver.read(output_slice).unwrap_or(0);

                        const LOW_WATER: usize = 256;
                        if frames_written + LOW_WATER < frames_needed {
                            static UNDERRUN_COUNT: std::sync::atomic::AtomicUsize =
                                std::sync::atomic::AtomicUsize::new(0);
                            let c =
                                UNDERRUN_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if c == 0 {
                                // only log the first underrun to avoid spamming
                                warn!(
                                    "audio underrun: only {} / {} frames available",
                                    frames_written, frames_needed
                                );
                            }
                        }

                        // Mute any remaining samples.
                        output_slice[frames_written..]
                            .iter_mut()
                            .for_each(|s| *s = T::MID);
                    },
                    move |err| error!("audio output error: {}", err),
                    None,
                )
                .map_err(|err| {
                    error!("audio output stream open error: {}", err);
                    AudioOutputError::OpenStreamError
                })?;

            output_stream.play().map_err(|err| {
                error!("audio output stream play error: {}", err);
                AudioOutputError::PlayStreamError
            })?;

            let interleaved_buffer = SampleBuffer::<T>::new(duration, spec);

            let rate_converter = (spec.rate != config.sample_rate.0).then(|| {
                info!("resampling {} Hz → {} Hz", spec.rate, config.sample_rate.0);
                Resampler::new(spec, config.sample_rate.0 as usize, duration)
            });

            info!(
                "audio stream opened: {} Hz, {} ch, rb {} frames",
                config.sample_rate.0, config.channels, rb_capacity_frames
            );

            Ok(Box::new(CpalStreamController {
                sample_sender,
                interleaved_buffer,
                stream: output_stream,
                rate_converter,
            }))
        }
    }

    impl<T: ScalableSample> AudioOutput for CpalStreamController<T>
    where
        f32: cpal::FromSample<T>,
    {
        fn write(&mut self, incoming_audio: AudioBufferRef<'_>, volume: f32) -> Result<()> {
            // Do nothing if there are no audio frames.
            if incoming_audio.frames() == 0 {
                return Ok(());
            }

            // 1. retrieve samples (resampled or original)
            let samples = if let Some(res) = &mut self.rate_converter {
                res.resample(incoming_audio)
                    .ok_or(AudioOutputError::ResampleError)?
            } else {
                self.interleaved_buffer.copy_interleaved_ref(incoming_audio);
                self.interleaved_buffer.samples()
            };

            // 2. volume scaling + write all at once
            use std::iter;
            let volume_iter = iter::from_fn({
                let mut idx = 0;
                move || {
                    if idx == samples.len() {
                        None
                    } else {
                        let v = samples[idx].mul(volume);
                        idx += 1;
                        Some(v)
                    }
                }
            });

            write_all_iter(&mut self.sample_sender, volume_iter);
            Ok(())
        }

        fn flush(&mut self) {
            // 1. flush all remaining samples from the resampler
            if let Some(res) = &mut self.rate_converter {
                while let Some(pending) = res.flush() {
                    write_all_iter(&mut self.sample_sender, pending.iter().copied());
                }
            }

            // 2. pause output, best effort, ignore errors
            let _ = self.stream.pause();
        }
    }

    // auxiliary function to write all samples from an iterator into the ring buffer
    fn write_all_iter<T, I>(sender: &mut rb::Producer<T>, iter: I) -> usize
    where
        T: ScalableSample,
        I: Iterator<Item = T>,
    {
        let mut written = 0;
        let mut buf = [T::MID; 128];
        let mut chunk_len = 0;

        for sample in iter {
            buf[chunk_len] = sample;
            chunk_len += 1;
            if chunk_len == buf.len() {
                let n = sender.write_blocking(&buf).unwrap_or(0);
                written += n;
                if n < chunk_len {
                    return written; // ring is full
                }
                chunk_len = 0;
            }
        }
        if chunk_len > 0 {
            let n = sender.write_blocking(&buf[..chunk_len]).unwrap_or(0);
            written += n;
        }
        written
    }
}

#[cfg(any(not(target_os = "linux"), not(feature = "pulseaudio")))]
pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
    cpal::CpalOutputDevice::try_open(spec, duration)
}
