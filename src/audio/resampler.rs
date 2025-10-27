// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal, SignalSpec};
use symphonia::core::conv::{FromSample, IntoSample};
use symphonia::core::sample::Sample;

/// Algorithm selector for resampling
pub enum Algorithm {
    /// High quality FFT-based resampling (higher CPU usage)
    HighQuality,
    /// Linear interpolation resampling (lower CPU usage)
    Linear,
}

/// Unified trait for rate converters
pub trait RateConverter<T>: Send
where
    T: Sample + FromSample<f32> + IntoSample<f32>,
{
    /// Push a chunk of planar input; returns None if not enough samples for conversion
    fn push_planar(&mut self, input: AudioBufferRef<'_>) -> Option<&[T]>;
    /// Drain remaining samples by padding with silence and convert
    fn drain_remaining(&mut self) -> Option<&[T]>;
}

/// Constructor function to create a rate converter based on algorithm
pub fn make_rate_converter<T>(
    spec: SignalSpec,
    to_sample_rate: usize,
    frame_chunk: usize,
    algo: Algorithm,
) -> Box<dyn RateConverter<T>>
where
    T: Sample + FromSample<f32> + IntoSample<f32> + Send + 'static,
{
    match algo {
        Algorithm::HighQuality => Box::new(FftEngine::new(spec, to_sample_rate, frame_chunk)),
        Algorithm::Linear => Box::new(LinearEngine::new(spec, to_sample_rate, frame_chunk)),
    }
}

/// Constructor function to create a rate converter with automatic algorithm selection
pub fn make_rate_converter_auto<T>(
    spec: SignalSpec,
    to_sample_rate: usize,
    frame_chunk: u64,
) -> Box<dyn RateConverter<T>>
where
    T: Sample + FromSample<f32> + IntoSample<f32> + Send + 'static,
{
    let from_sample_rate = spec.rate as usize;
    let ratio_diff =
        ((to_sample_rate as f64) - (from_sample_rate as f64)).abs() / (from_sample_rate as f64);
    let algo = if ratio_diff < 0.1 {
        Algorithm::Linear
    } else {
        Algorithm::HighQuality
    };
    make_rate_converter(spec, to_sample_rate, frame_chunk as usize, algo)
}

/* -------------------------------------------------
 * FFT Engine Implementation
 * ------------------------------------------------- */
struct FftEngine<T> {
    engine: rubato::FftFixedIn<f32>,
    planar_in: Vec<Vec<f32>>,
    planar_out: Vec<Vec<f32>>,
    interleaved_out: Vec<T>,
    frame_chunk: usize,
}

impl<T> FftEngine<T>
where
    T: Sample + FromSample<f32> + IntoSample<f32>,
{
    fn new(spec: SignalSpec, to_sample_rate: usize, frame_chunk: usize) -> Self {
        let num_channels = spec.channels.count();
        let from_sample_rate = spec.rate as usize;

        let engine = rubato::FftFixedIn::<f32>::new(
            from_sample_rate,
            to_sample_rate,
            frame_chunk,
            2,
            num_channels,
        )
        .unwrap();

        let planar_out = rubato::Resampler::output_buffer_allocate(&engine);
        let planar_in = vec![Vec::with_capacity(frame_chunk); num_channels];

        // Pre-calculate max output frames based on resampling ratio
        // Add 10% margin to be safe
        let max_output_frames = ((frame_chunk as f64)
            * ((to_sample_rate as f64) / (from_sample_rate as f64))
            * 1.1) as usize;

        let interleaved_out = vec![T::MID; num_channels * max_output_frames];

        Self {
            engine,
            planar_in,
            planar_out,
            interleaved_out,
            frame_chunk,
        }
    }

    fn convert_chunk(&mut self) -> &[T] {
        {
            let mut input: arrayvec::ArrayVec<&[f32], 32> = Default::default();

            for channel in self.planar_in.iter() {
                input.push(&channel[..self.frame_chunk]);
            }

            // Resample using FFT
            rubato::Resampler::process_into_buffer(
                &mut self.engine,
                &input,
                &mut self.planar_out,
                None,
            )
            .unwrap();
        }

        // Remove consumed samples from the input buffer.
        for channel in self.planar_in.iter_mut() {
            channel.drain(0..self.frame_chunk);
        }

        // Interleave the planar samples from Rubato.
        let num_channels = self.planar_out.len();
        let output_frames = self.planar_out[0].len();

        // Ensure our pre-allocated buffer is large enough
        if self.interleaved_out.len() < num_channels * output_frames {
            self.interleaved_out
                .resize(num_channels * output_frames, T::MID);
        }

        // Interleave the samples from planar to interleaved format
        for (i, frame) in self.interleaved_out[..num_channels * output_frames]
            .chunks_exact_mut(num_channels)
            .enumerate()
        {
            for (ch, s) in frame.iter_mut().enumerate() {
                *s = self.planar_out[ch][i].into_sample();
            }
        }

        &self.interleaved_out[..num_channels * output_frames]
    }
}

impl<T> RateConverter<T> for FftEngine<T>
where
    T: Sample + FromSample<f32> + IntoSample<f32> + Send,
{
    fn push_planar(&mut self, input: AudioBufferRef<'_>) -> Option<&[T]> {
        // Copy and convert samples into input buffer.
        convert_samples_any(&input, &mut self.planar_in);

        // Check if more samples are required.
        if self.planar_in[0].len() < self.frame_chunk {
            return None;
        }

        Some(self.convert_chunk())
    }

    fn drain_remaining(&mut self) -> Option<&[T]> {
        let len = self.planar_in[0].len();

        if len == 0 {
            return None;
        }

        let partial_len = len % self.frame_chunk;

        if partial_len != 0 {
            // Fill each input channel buffer with silence to the next multiple of the resampler
            // duration.
            for channel in self.planar_in.iter_mut() {
                channel.resize(len + (self.frame_chunk - partial_len), f32::MID);
            }
        }

        Some(self.convert_chunk())
    }
}

/* -------------------------------------------------
 * Linear Engine Implementation
 * ------------------------------------------------- */
struct LinearEngine<T> {
    engine: rubato::SincFixedIn<f32>,
    planar_in: Vec<Vec<f32>>,
    planar_out: Vec<Vec<f32>>,
    interleaved_out: Vec<T>,
    frame_chunk: usize,
}

impl<T> LinearEngine<T>
where
    T: Sample + FromSample<f32> + IntoSample<f32>,
{
    fn new(spec: SignalSpec, to_sample_rate: usize, frame_chunk: usize) -> Self {
        let num_channels = spec.channels.count();
        let from_sample_rate = spec.rate as usize;

        let engine = rubato::SincFixedIn::<f32>::new(
            from_sample_rate as f64 / to_sample_rate as f64,
            0.95,
            rubato::InterpolationParameters {
                sinc_len: 256,
                f_cutoff: 0.95,
                oversampling_factor: 128,
                interpolation: rubato::InterpolationType::Linear,
                window: rubato::WindowFunction::BlackmanHarris2,
            },
            frame_chunk,
            num_channels,
        )
        .unwrap();

        let planar_out = rubato::Resampler::output_buffer_allocate(&engine);
        let planar_in = vec![Vec::with_capacity(frame_chunk); num_channels];

        // Pre-calculate max output frames based on resampling ratio
        // Add 10% margin to be safe
        let max_output_frames = ((frame_chunk as f64)
            * ((to_sample_rate as f64) / (from_sample_rate as f64))
            * 1.1) as usize;

        let interleaved_out = vec![T::MID; num_channels * max_output_frames];

        Self {
            engine,
            planar_in,
            planar_out,
            interleaved_out,
            frame_chunk,
        }
    }

    fn convert_chunk(&mut self) -> &[T] {
        {
            let mut input: arrayvec::ArrayVec<&[f32], 32> = Default::default();

            for channel in self.planar_in.iter() {
                input.push(&channel[..self.frame_chunk]);
            }

            // Resample using linear
            rubato::Resampler::process_into_buffer(
                &mut self.engine,
                &input,
                &mut self.planar_out,
                None,
            )
            .unwrap();
        }

        // Remove consumed samples from the input buffer.
        for channel in self.planar_in.iter_mut() {
            channel.drain(0..self.frame_chunk);
        }

        // Interleave the planar samples from Rubato.
        let num_channels = self.planar_out.len();
        let output_frames = self.planar_out[0].len();

        // Ensure our pre-allocated buffer is large enough
        if self.interleaved_out.len() < num_channels * output_frames {
            self.interleaved_out
                .resize(num_channels * output_frames, T::MID);
        }

        // Interleave the samples from planar to interleaved format
        for (i, frame) in self.interleaved_out[..num_channels * output_frames]
            .chunks_exact_mut(num_channels)
            .enumerate()
        {
            for (ch, s) in frame.iter_mut().enumerate() {
                *s = self.planar_out[ch][i].into_sample();
            }
        }

        &self.interleaved_out[..num_channels * output_frames]
    }
}

impl<T> RateConverter<T> for LinearEngine<T>
where
    T: Sample + FromSample<f32> + IntoSample<f32> + Send,
{
    fn push_planar(&mut self, input: AudioBufferRef<'_>) -> Option<&[T]> {
        // Copy and convert samples into input buffer.
        convert_samples_any(&input, &mut self.planar_in);

        // Check if more samples are required.
        if self.planar_in[0].len() < self.frame_chunk {
            return None;
        }

        Some(self.convert_chunk())
    }

    fn drain_remaining(&mut self) -> Option<&[T]> {
        let len = self.planar_in[0].len();

        if len == 0 {
            return None;
        }

        let partial_len = len % self.frame_chunk;

        if partial_len != 0 {
            // Fill each input channel buffer with silence to the next multiple of the resampler
            // duration.
            for channel in self.planar_in.iter_mut() {
                channel.resize(len + (self.frame_chunk - partial_len), f32::MID);
            }
        }

        Some(self.convert_chunk())
    }
}

fn convert_samples_any(input: &AudioBufferRef<'_>, output: &mut [Vec<f32>]) {
    match input {
        AudioBufferRef::U8(input) => convert_samples(input, output),
        AudioBufferRef::U16(input) => convert_samples(input, output),
        AudioBufferRef::U24(input) => convert_samples(input, output),
        AudioBufferRef::U32(input) => convert_samples(input, output),
        AudioBufferRef::S8(input) => convert_samples(input, output),
        AudioBufferRef::S16(input) => convert_samples(input, output),
        AudioBufferRef::S24(input) => convert_samples(input, output),
        AudioBufferRef::S32(input) => convert_samples(input, output),
        AudioBufferRef::F32(input) => convert_samples(input, output),
        AudioBufferRef::F64(input) => convert_samples(input, output),
    }
}

fn convert_samples<S>(input: &AudioBuffer<S>, output: &mut [Vec<f32>])
where
    S: Sample + IntoSample<f32>,
{
    for (c, dst) in output.iter_mut().enumerate() {
        let src = input.chan(c);
        dst.extend(src.iter().map(|&s| s.into_sample()));
    }
}
