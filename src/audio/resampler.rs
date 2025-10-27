// Symphonia
// Copyright (c) 2019-2022 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia::core::audio::{AudioBuffer, AudioBufferRef, Signal, SignalSpec};
use symphonia::core::conv::{FromSample, IntoSample};
use symphonia::core::sample::Sample;

/// Resampling algorithm type
#[allow(dead_code)]
pub enum ResamplerType {
    /// High quality FFT-based resampling (higher CPU usage)
    HighQuality,
    /// Linear interpolation resampling (lower CPU usage)
    Linear,
}

#[allow(dead_code)]
pub struct Resampler<T> {
    resampler_type: ResamplerType,
    fft_resampler: Option<rubato::FftFixedIn<f32>>,
    linear_resampler: Option<rubato::SincFixedIn<f32>>,
    input: Vec<Vec<f32>>,
    output: Vec<Vec<f32>>,
    interleaved: Vec<T>,
    duration: usize,
}

impl<T> Resampler<T>
where
    T: Sample + FromSample<f32> + IntoSample<f32>,
{
    #[allow(dead_code)]
    fn resample_inner(&mut self) -> &[T] {
        // Process with either FFT or linear resampler
        {
            let mut input: arrayvec::ArrayVec<&[f32], 32> = Default::default();

            for channel in self.input.iter() {
                input.push(&channel[..self.duration]);
            }

            // Resample using the selected algorithm
            match (
                &mut self.fft_resampler,
                &mut self.linear_resampler,
                &self.resampler_type,
            ) {
                (Some(resampler), _, ResamplerType::HighQuality) => {
                    // Use FFT resampler
                    rubato::Resampler::process_into_buffer(
                        resampler,
                        &input,
                        &mut self.output,
                        None,
                    )
                    .unwrap();
                }
                (_, Some(resampler), ResamplerType::Linear) => {
                    // Use linear resampler
                    rubato::Resampler::process_into_buffer(
                        resampler,
                        &input,
                        &mut self.output,
                        None,
                    )
                    .unwrap();
                }
                _ => panic!("No resampler available for the selected type"),
            }
        }

        // Remove consumed samples from the input buffer.
        for channel in self.input.iter_mut() {
            channel.drain(0..self.duration);
        }

        // Interleave the planar samples from Rubato.
        let num_channels = self.output.len();
        let output_frames = self.output[0].len();

        // Ensure our pre-allocated buffer is large enough
        if self.interleaved.len() < num_channels * output_frames {
            self.interleaved
                .resize(num_channels * output_frames, T::MID);
        }

        // Interleave the samples from planar to interleaved format
        for (i, frame) in self.interleaved[..num_channels * output_frames]
            .chunks_exact_mut(num_channels)
            .enumerate()
        {
            for (ch, s) in frame.iter_mut().enumerate() {
                *s = self.output[ch][i].into_sample();
            }
        }

        &self.interleaved[..num_channels * output_frames]
    }
}

impl<T> Resampler<T>
where
    T: Sample + FromSample<f32> + IntoSample<f32>,
{
    #[allow(dead_code)]
    pub fn new(spec: SignalSpec, to_sample_rate: usize, duration: u64) -> Self {
        let duration = duration as usize;
        let num_channels = spec.channels.count();
        let from_sample_rate = spec.rate as usize;

        // Pre-calculate max output frames based on resampling ratio
        // Add 10% margin to be safe
        let max_output_frames = ((duration as f64)
            * ((to_sample_rate as f64) / (from_sample_rate as f64))
            * 1.1) as usize;

        // Choose resampler type based on resampling ratio
        // For minor resampling (less than 10% difference), use linear
        // For major resampling, use FFT for better quality
        let ratio_diff =
            ((to_sample_rate as f64) - (from_sample_rate as f64)).abs() / (from_sample_rate as f64);
        let resampler_type = if ratio_diff < 0.1 {
            ResamplerType::Linear
        } else {
            ResamplerType::HighQuality
        };

        // Create appropriate resampler based on type
        let (fft_resampler, linear_resampler, output) = match resampler_type {
            ResamplerType::HighQuality => {
                let resampler = rubato::FftFixedIn::<f32>::new(
                    from_sample_rate,
                    to_sample_rate,
                    duration,
                    2,
                    num_channels,
                )
                .unwrap();
                let output = rubato::Resampler::output_buffer_allocate(&resampler);
                (Some(resampler), None, output)
            }
            ResamplerType::Linear => {
                // Use simpler SincFixedIn with fewer parameters
                let resampler = rubato::SincFixedIn::<f32>::new(
                    from_sample_rate as f64 / to_sample_rate as f64,
                    0.95,
                    rubato::InterpolationParameters {
                        sinc_len: 256,
                        f_cutoff: 0.95,
                        oversampling_factor: 128,
                        interpolation: rubato::InterpolationType::Linear,
                        window: rubato::WindowFunction::BlackmanHarris2,
                    },
                    duration,
                    num_channels,
                )
                .unwrap();
                let output = rubato::Resampler::output_buffer_allocate(&resampler);
                (None, Some(resampler), output)
            }
        };

        let input = vec![Vec::with_capacity(duration); num_channels];

        // Pre-allocate interleaved buffer with maximum expected size to avoid reallocations
        let interleaved = vec![T::MID; num_channels * max_output_frames];

        Self {
            resampler_type,
            fft_resampler,
            linear_resampler,
            input,
            output,
            interleaved,
            duration,
        }
    }

    /// Resamples a planar/non-interleaved input.
    ///
    /// Returns the resampled samples in an interleaved format.
    #[allow(dead_code)]
    pub fn resample(&mut self, input: AudioBufferRef<'_>) -> Option<&[T]> {
        // Copy and convert samples into input buffer.
        convert_samples_any(&input, &mut self.input);

        // Check if more samples are required.
        if self.input[0].len() < self.duration {
            return None;
        }

        Some(self.resample_inner())
    }

    /// Resample any remaining samples in the resample buffer.
    #[allow(dead_code)]
    pub fn flush(&mut self) -> Option<&[T]> {
        let len = self.input[0].len();

        if len == 0 {
            return None;
        }

        let partial_len = len % self.duration;

        if partial_len != 0 {
            // Fill each input channel buffer with silence to the next multiple of the resampler
            // duration.
            for channel in self.input.iter_mut() {
                channel.resize(len + (self.duration - partial_len), f32::MID);
            }
        }

        Some(self.resample_inner())
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn convert_samples<S>(input: &AudioBuffer<S>, output: &mut [Vec<f32>])
where
    S: Sample + IntoSample<f32>,
{
    for (c, dst) in output.iter_mut().enumerate() {
        let src = input.chan(c);
        dst.extend(src.iter().map(|&s| s.into_sample()));
    }
}
