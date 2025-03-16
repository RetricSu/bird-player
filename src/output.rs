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

#[cfg(any(not(target_os = "linux"), not(feature = "pulseaudio")))]
mod cpal {
    use crate::resampler::Resampler;

    use super::{AudioOutput, AudioOutputError, Result};

    use symphonia::core::audio::{AudioBufferRef, RawSample, SampleBuffer, SignalSpec};
    use symphonia::core::conv::{ConvertibleSample, IntoSample};
    use symphonia::core::units::Duration;

    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use rb::*;

    use log::{error, info};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    pub struct CpalAudioOutput;

    trait AudioOutputSample:
        cpal::Sample + ConvertibleSample + IntoSample<f32> + RawSample + std::marker::Send + 'static
    {
        fn mul(&self, n: f32) -> Self;
    }

    impl AudioOutputSample for f32 {
        #[inline(always)]
        fn mul(&self, n: f32) -> Self {
            self * n
        }
    }

    impl AudioOutputSample for i16 {
        #[inline(always)]
        fn mul(&self, n: f32) -> Self {
            ((*self as f32) * n) as i16
        }
    }

    impl AudioOutputSample for u16 {
        #[inline(always)]
        fn mul(&self, n: f32) -> Self {
            ((*self as f32) * n) as u16
        }
    }

    // Convert between f32 and u32 for atomic storage
    #[inline(always)]
    fn f32_to_u32(f: f32) -> u32 {
        f.to_bits()
    }

    #[inline(always)]
    fn u32_to_f32(u: u32) -> f32 {
        f32::from_bits(u)
    }

    impl CpalAudioOutput {
        pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
            // Get default host.
            let host = cpal::default_host();

            // Get the default audio output device.
            let device = match host.default_output_device() {
                Some(device) => device,
                _ => {
                    error!("failed to get default audio output device");
                    return Err(AudioOutputError::OpenStreamError);
                }
            };

            let config = match device.default_output_config() {
                Ok(config) => config,
                Err(err) => {
                    error!("failed to get default audio output device config: {}", err);
                    return Err(AudioOutputError::OpenStreamError);
                }
            };

            // Select proper playback routine based on sample format.
            match config.sample_format() {
                cpal::SampleFormat::F32 => {
                    CpalAudioOutputImpl::<f32>::try_open(spec, duration, &device)
                }
                cpal::SampleFormat::I16 => {
                    CpalAudioOutputImpl::<i16>::try_open(spec, duration, &device)
                }
                cpal::SampleFormat::U16 => {
                    CpalAudioOutputImpl::<u16>::try_open(spec, duration, &device)
                }
                _ => panic!("Unsupported sample format"),
            }
        }
    }

    struct CpalAudioOutputImpl<T: AudioOutputSample>
    where
        T: AudioOutputSample,
    {
        ring_buf_producer: rb::Producer<T>,
        sample_buf: SampleBuffer<T>,
        stream: cpal::Stream,
        resampler: Option<Resampler<T>>,
        volume: Arc<AtomicU32>,
        buffer_full: Arc<AtomicBool>,
        last_buffer_warning: Instant,
    }

    impl<T: cpal::SizedSample + AudioOutputSample> CpalAudioOutputImpl<T>
    where
        f32: cpal::FromSample<T>,
    {
        // Helper function to determine appropriate ring buffer size based on system capabilities
        fn determine_buffer_size(sample_rate: u32, num_channels: usize) -> usize {
            // Default to 2 seconds of audio
            let mut buffer_seconds = 2.0;

            // On macOS we can be more aggressive with buffer sizes because it has good audio
            // scheduling compared to other platforms
            #[cfg(target_os = "macos")]
            {
                // Detect if we're running on a Mac with Apple Silicon
                // (M1/M2/etc. has better audio performance)
                if std::env::consts::ARCH == "aarch64" {
                    // For Apple Silicon, use a smaller buffer to reduce latency
                    buffer_seconds = 1.5;
                }
            }

            // For higher sample rates, we need larger buffers
            if sample_rate > 48000 {
                buffer_seconds *= 1.5;
            }

            // For more channels, we need larger buffers
            if num_channels > 2 {
                buffer_seconds *= 1.25;
            }

            // Calculate final buffer size
            (sample_rate as f64 * buffer_seconds * num_channels as f64) as usize
        }

        pub fn try_open(
            spec: SignalSpec,
            duration: Duration,
            device: &cpal::Device,
        ) -> Result<Box<dyn AudioOutput>> {
            let num_channels = spec.channels.count();

            // Output audio stream config.
            let config = if cfg!(not(target_os = "windows")) {
                cpal::StreamConfig {
                    channels: num_channels as cpal::ChannelCount,
                    sample_rate: cpal::SampleRate(spec.rate),
                    buffer_size: cpal::BufferSize::Default,
                }
            } else {
                // Use the default config for Windows.
                device
                    .default_output_config()
                    .expect("Failed to get the default output config.")
                    .config()
            };

            // Dynamically determine optimal ring buffer size
            let ring_len = Self::determine_buffer_size(config.sample_rate.0, num_channels);

            // Log the buffer size so we can see it in diagnostic output
            info!(
                "Using audio ring buffer size of {} samples ({:.2} seconds)",
                ring_len,
                ring_len as f64 / (config.sample_rate.0 as f64 * num_channels as f64)
            );

            let ring_buf = SpscRb::new(ring_len);
            let (ring_buf_producer, ring_buf_consumer) = (ring_buf.producer(), ring_buf.consumer());

            // Create atomic flags for status tracking
            let volume = Arc::new(AtomicU32::new(f32_to_u32(1.0)));
            let volume_for_callback = volume.clone();

            // Add buffer state tracking
            let buffer_full = Arc::new(AtomicBool::new(false));
            let buffer_full_for_callback = buffer_full.clone();

            let stream_result = device.build_output_stream(
                &config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    // Get current volume - only read once per callback to reduce synchronization overhead
                    let current_volume = u32_to_f32(volume_for_callback.load(Ordering::Relaxed));

                    // Write out as many samples as possible from the ring buffer to the audio output
                    let written = ring_buf_consumer.read(data).unwrap_or(0);

                    // Update buffer status based on how full the buffer is
                    // If we couldn't fill the entire requested buffer, the ring buffer is getting low
                    if written < data.len() {
                        buffer_full_for_callback.store(false, Ordering::Relaxed);
                    } else {
                        // If we filled the entire buffer, consider the buffer state
                        // We don't have a direct way to check how many samples are left,
                        // so we'll use the fact that we could fill the entire requested buffer
                        // as an indication that we likely have enough data
                        buffer_full_for_callback.store(true, Ordering::Relaxed);
                    }

                    // Apply volume in the audio callback if needed
                    if current_volume != 1.0 && written > 0 {
                        for s in data[..written].iter_mut() {
                            *s = s.mul(current_volume);
                        }
                    }

                    // Mute any remaining samples.
                    if written < data.len() {
                        data[written..].iter_mut().for_each(|s| *s = T::MID);
                    }
                },
                move |err| error!("audio output error: {}", err),
                None,
            );

            if let Err(err) = stream_result {
                error!("audio output stream open error: {}", err);
                return Err(AudioOutputError::OpenStreamError);
            }

            let stream = stream_result.unwrap();

            // Start the output stream.
            if let Err(err) = stream.play() {
                error!("audio output stream play error: {}", err);
                return Err(AudioOutputError::PlayStreamError);
            }

            let sample_buf = SampleBuffer::<T>::new(duration, spec);

            let resampler = if spec.rate != config.sample_rate.0 {
                info!("resampling {} Hz to {} Hz", spec.rate, config.sample_rate.0);
                Some(Resampler::new(
                    spec,
                    config.sample_rate.0 as usize,
                    duration,
                ))
            } else {
                None
            };

            Ok(Box::new(CpalAudioOutputImpl {
                ring_buf_producer,
                sample_buf,
                stream,
                resampler,
                volume,
                buffer_full,
                last_buffer_warning: Instant::now(),
            }))
        }
    }

    impl<T: AudioOutputSample> AudioOutput for CpalAudioOutputImpl<T>
    where
        f32: cpal::FromSample<T>,
    {
        fn write(&mut self, decoded: AudioBufferRef<'_>, volume: f32) -> Result<()> {
            // Do nothing if there are no audio frames.
            if decoded.frames() == 0 {
                return Ok(());
            }

            // Update the volume atomically for the audio callback to use
            self.volume.store(f32_to_u32(volume), Ordering::Relaxed);

            // Check if the buffer is full - if so, we should wait before decoding more
            if self.buffer_full.load(Ordering::Relaxed) {
                // Buffer is full, let's wait a bit to allow the audio callback to consume more data
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let samples = if let Some(resampler) = &mut self.resampler {
                // Resampling is required. The resampler will return interleaved samples.
                match resampler.resample(decoded) {
                    Some(resampled) => resampled,
                    None => return Ok(()),
                }
            } else {
                // Resampling is not required. Interleave the sample for cpal using a sample buffer.
                self.sample_buf.copy_interleaved_ref(decoded);
                self.sample_buf.samples()
            };

            // Implement a backoff strategy for writing to the buffer if it's getting full
            let mut retry_count = 0;
            let samples_len = samples.len();
            let mut samples_written = 0;

            // Try multiple times to write all samples
            while samples_written < samples_len && retry_count < 5 {
                // Use write_blocking with a slice into the samples array
                match self
                    .ring_buf_producer
                    .write_blocking(&samples[samples_written..])
                {
                    Some(written) => {
                        // Update how many samples we've written so far
                        samples_written += written;

                        // If we haven't written all samples, wait a moment and try again
                        if samples_written < samples_len {
                            retry_count += 1;

                            // Small pause to let the audio thread consume some data
                            std::thread::sleep(std::time::Duration::from_micros(500));
                        }
                    }
                    None => {
                        // Buffer is likely full, wait a bit longer
                        retry_count += 1;
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                }
            }

            // If we couldn't write all samples, log it (but not too frequently)
            if samples_written < samples_len {
                let now = Instant::now();
                if now.duration_since(self.last_buffer_warning).as_millis() > 500 {
                    // Only log warnings at most every 500ms to avoid log spam
                    error!(
                        "Audio buffer overflow: dropped {} samples after {} retries",
                        samples_len - samples_written,
                        retry_count
                    );
                    self.last_buffer_warning = now;
                }
            }

            Ok(())
        }

        fn flush(&mut self) {
            // If there is a resampler, flush it
            if let Some(resampler) = &mut self.resampler {
                if let Some(remaining_samples) = resampler.flush() {
                    let _ = self.ring_buf_producer.write_blocking(remaining_samples);
                }
            }

            // Flush is best-effort, ignore the returned result.
            let _ = self.stream.pause();
        }
    }
}

#[cfg(any(not(target_os = "linux"), not(feature = "pulseaudio")))]
pub fn try_open(spec: SignalSpec, duration: Duration) -> Result<Box<dyn AudioOutput>> {
    cpal::CpalAudioOutput::try_open(spec, duration)
}
