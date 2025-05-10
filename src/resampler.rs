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
        let duration = self.duration;

        // First, process the resampling
        {
            // Create input without using self.input directly to avoid borrowing issues
            let mut input_slices: Vec<&[f32]> = Vec::with_capacity(self.input.len());
            for channel in &self.input {
                input_slices.push(&channel[..duration]);
            }

            // Convert to arrayvec for rubato
            let mut input: arrayvec::ArrayVec<&[f32], 32> = Default::default();
            for &slice in &input_slices {
                input.push(slice);
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

        // Now we can safely modify the input buffer
        for channel in &mut self.input {
            channel.drain(0..duration);
        }

        // Process output
        let num_channels = self.output.len();
        let output_frames = self.output[0].len();

        // Ensure our pre-allocated buffer is large enough
        if self.interleaved.len() < num_channels * output_frames {
            // Only resize when absolutely necessary
            let new_size = (num_channels * output_frames).next_power_of_two();
            self.interleaved.resize(new_size, T::MID);
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
        let resampler_type = if ratio_diff < 0.25 {
            // Increased threshold from 0.1 to 0.25 to use Linear more often
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
                    1, // Reduced from 2 to 1 for better performance with slight quality trade-off
                    num_channels,
                )
                .unwrap();
                let output = rubato::Resampler::output_buffer_allocate(&resampler);
                (Some(resampler), None, output)
            }
            ResamplerType::Linear => {
                // Use simpler SincFixedIn with optimized parameters
                let resampler = rubato::SincFixedIn::<f32>::new(
                    from_sample_rate as f64 / to_sample_rate as f64,
                    0.95,
                    rubato::InterpolationParameters {
                        sinc_len: 128, // Reduced from 256 to 128
                        f_cutoff: 0.95,
                        oversampling_factor: 64, // Reduced from 128 to 64
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
        // Preallocate capacity to avoid reallocations during conversion
        let expected_samples = input.frames();
        for channel in self.input.iter_mut() {
            // Ensure we have enough capacity to avoid reallocations
            if channel.capacity() < channel.len() + expected_samples {
                // Add 10% margin to reduce future reallocations
                let new_capacity = ((channel.len() + expected_samples) as f32 * 1.1) as usize;
                channel.reserve(new_capacity - channel.capacity());
            }
        }

        // Now convert the samples
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
    // Pre-calculate the number of samples to add to avoid reallocations
    let frames = input.frames();

    for (c, dst) in output.iter_mut().enumerate() {
        // Get slice to source channel
        let src = input.chan(c);

        // Perform a single extension operation instead of iterative appends
        let start_idx = dst.len();
        let additional = frames;

        // Ensure we have enough capacity
        if dst.capacity() < start_idx + additional {
            dst.reserve(additional);
        }

        // Extend the destination with source samples all at once
        dst.resize(start_idx + additional, 0.0);

        // Convert samples directly into the destination buffer
        for (i, &sample) in src.iter().enumerate() {
            dst[start_idx + i] = sample.into_sample();
        }
    }
}
