//! Kaldi-compatible mel-filterbank ("fbank") feature extraction, matching
//! the `kaldi-native-fbank` defaults used by sherpa-onnx for WeSpeaker-style
//! speaker embedding models: 16 kHz, 25 ms frames / 10 ms shift, Povey
//! window, 80 mel bins spanning 20 Hz–7600 Hz, snip-edges=false,
//! remove-dc-offset, preemphasis 0.97, no dithering, no CMVN.
//!
//! Pure DSP, no I/O — the caller supplies already-decoded f32 PCM samples in
//! `[-1, 1]` range (matching `normalize_samples=0` handling: WeSpeaker's own
//! sherpa-onnx metadata means samples are NOT rescaled by 32768 before
//! feature extraction, unlike the sherpa-onnx default).

use rustfft::num_complex::Complex32;
use rustfft::{Fft, FftPlanner};
use std::sync::Arc;

const SAMPLE_RATE: f32 = 16_000.0;
const FRAME_LENGTH_MS: f32 = 25.0;
const FRAME_SHIFT_MS: f32 = 10.0;
const NUM_MEL_BINS: usize = 80;
const LOW_FREQ: f32 = 20.0;
/// Kaldi convention: a negative `high_freq` config value means
/// `nyquist - |high_freq|`. sherpa-onnx's default is -400.0, i.e. 7600 Hz
/// at 16 kHz.
const HIGH_FREQ_OFFSET: f32 = 400.0;
const PREEMPH_COEFF: f32 = 0.97;

fn frame_length_samples() -> usize {
    (SAMPLE_RATE * FRAME_LENGTH_MS / 1000.0).round() as usize
}

fn frame_shift_samples() -> usize {
    (SAMPLE_RATE * FRAME_SHIFT_MS / 1000.0).round() as usize
}

/// Next power of two >= n (Kaldi's `round_to_power_of_two`, default true).
fn fft_size_for(frame_len: usize) -> usize {
    frame_len.next_power_of_two()
}

/// Povey window: Kaldi's variant of a raised-Hann window, `(0.5 - 0.5*cos(2*pi*i/(N-1)))^0.85`.
fn povey_window(len: usize) -> Vec<f32> {
    if len <= 1 {
        return vec![1.0; len];
    }
    let n = (len - 1) as f32;
    (0..len)
        .map(|i| {
            let base = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n).cos();
            base.powf(0.85)
        })
        .collect()
}

/// Hz to Kaldi mel scale: `1127 * ln(1 + hz/700)`.
fn hz_to_mel(hz: f32) -> f32 {
    1127.0 * (1.0 + hz / 700.0).ln()
}

fn mel_to_hz(mel: f32) -> f32 {
    700.0 * ((mel / 1127.0).exp() - 1.0)
}

/// Triangular mel filterbank matrix, `[num_mel_bins][fft_size/2 + 1]`,
/// matching Kaldi's `ComputeLifterCoeffs`-adjacent `MelBanks` construction:
/// bins are triangles between mel-spaced center frequencies, in the power
/// spectrum's linearly-spaced FFT bins.
fn mel_filterbank(fft_size: usize, sample_rate: f32) -> Vec<Vec<f32>> {
    let num_fft_bins = fft_size / 2 + 1;
    let nyquist = sample_rate / 2.0;
    let high_freq = nyquist - HIGH_FREQ_OFFSET;

    let mel_low = hz_to_mel(LOW_FREQ);
    let mel_high = hz_to_mel(high_freq);
    let mel_step = (mel_high - mel_low) / (NUM_MEL_BINS + 1) as f32;

    // Center frequencies of the NUM_MEL_BINS+2 boundary points (in mel).
    let mel_points: Vec<f32> = (0..NUM_MEL_BINS + 2)
        .map(|i| mel_low + mel_step * i as f32)
        .collect();
    let hz_points: Vec<f32> = mel_points.iter().map(|&m| mel_to_hz(m)).collect();

    let fft_bin_freq = |bin: usize| -> f32 { bin as f32 * sample_rate / fft_size as f32 };

    (0..NUM_MEL_BINS)
        .map(|m| {
            let left = hz_points[m];
            let center = hz_points[m + 1];
            let right = hz_points[m + 2];
            (0..num_fft_bins)
                .map(|bin| {
                    let freq = fft_bin_freq(bin);
                    if freq < left || freq > right {
                        0.0
                    } else if freq <= center {
                        (freq - left) / (center - left)
                    } else {
                        (right - freq) / (right - center)
                    }
                })
                .collect()
        })
        .collect()
}

/// Extracts 80-dim log mel filterbank frames from mono f32 PCM at 16 kHz,
/// following Kaldi/`kaldi-native-fbank` conventions with `snip-edges=false`
/// (frames span the whole signal, edge frames are zero-padded rather than
/// dropped).
pub struct FbankExtractor {
    frame_len: usize,
    frame_shift: usize,
    fft_size: usize,
    window: Vec<f32>,
    mel_matrix: Vec<Vec<f32>>,
    fft: Arc<dyn Fft<f32>>,
}

impl Default for FbankExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl FbankExtractor {
    pub fn new() -> Self {
        let frame_len = frame_length_samples();
        let frame_shift = frame_shift_samples();
        let fft_size = fft_size_for(frame_len);
        let window = povey_window(frame_len);
        let mel_matrix = mel_filterbank(fft_size, SAMPLE_RATE);
        let fft = FftPlanner::new().plan_fft_forward(fft_size);

        Self {
            frame_len,
            frame_shift,
            fft_size,
            window,
            mel_matrix,
            fft,
        }
    }

    /// Number of frames `snip-edges=false` framing produces for `num_samples`.
    /// Kaldi formula: `round(num_samples / frame_shift)`, minimum 0 (a signal
    /// shorter than one shift produces zero frames, matching Kaldi/knf).
    pub fn num_frames(&self, num_samples: usize) -> usize {
        if num_samples == 0 {
            return 0;
        }
        ((num_samples as f32) / (self.frame_shift as f32)).round() as usize
    }

    /// Extract log-mel fbank features. Returns `num_frames` rows of
    /// `NUM_MEL_BINS` columns, flattened row-major (matches the ONNX
    /// embedding model's `feats: [B, T, 80]` layout once reshaped by the
    /// caller).
    pub fn extract(&self, samples: &[f32]) -> Vec<f32> {
        let n_frames = self.num_frames(samples.len());
        let mut out = Vec::with_capacity(n_frames * NUM_MEL_BINS);

        for frame_idx in 0..n_frames {
            // snip-edges=false: frame is centered so that consecutive frames
            // are `frame_shift` apart and the whole signal is covered; frame 0
            // starts at `-(frame_len - frame_shift) / 2` (matching knf).
            let midpoint = frame_idx as i64 * self.frame_shift as i64 + self.frame_shift as i64 / 2;
            let start = midpoint - self.frame_len as i64 / 2;

            let mut frame = vec![0.0f32; self.frame_len];
            for (i, sample) in frame.iter_mut().enumerate() {
                let src_idx = start + i as i64;
                *sample = reflect_sample(samples, src_idx);
            }

            let power_spectrum = self.frame_power_spectrum(&mut frame);
            for row in &self.mel_matrix {
                let energy: f32 = row.iter().zip(&power_spectrum).map(|(w, p)| w * p).sum();
                out.push(energy.max(1e-10).ln());
            }
        }

        out
    }

    /// Windowed, preemphasized, DC-removed FFT power spectrum of one frame
    /// (`fft_size / 2 + 1` bins), matching Kaldi's per-frame processing order:
    /// remove DC offset -> preemphasis -> window -> FFT -> power.
    fn frame_power_spectrum(&self, frame: &mut [f32]) -> Vec<f32> {
        remove_dc_offset(frame);
        preemphasize(frame, PREEMPH_COEFF);
        for (s, w) in frame.iter_mut().zip(&self.window) {
            *s *= w;
        }

        let mut buffer: Vec<Complex32> = frame
            .iter()
            .map(|&s| Complex32::new(s, 0.0))
            .chain(std::iter::repeat(Complex32::new(0.0, 0.0)))
            .take(self.fft_size)
            .collect();
        self.fft.process(&mut buffer);

        (0..self.fft_size / 2 + 1)
            .map(|i| buffer[i].norm_sqr())
            .collect()
    }
}

/// Reflect out-of-bounds sample indices at the signal edges (Kaldi's
/// `snip-edges=false` padding), matching `kaldi-native-fbank`'s
/// `ExtractWindow`: index `-1` maps to `0`, `-2` to `1`, `len` to `len-1`, etc.
fn reflect_sample(samples: &[f32], idx: i64) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let len = samples.len() as i64;
    let mut i = idx;
    if i < 0 {
        i = -i - 1;
    }
    if i >= len {
        i = 2 * len - i - 1;
    }
    let i = i.clamp(0, len - 1) as usize;
    samples[i]
}

fn remove_dc_offset(frame: &mut [f32]) {
    if frame.is_empty() {
        return;
    }
    let mean: f32 = frame.iter().sum::<f32>() / frame.len() as f32;
    for s in frame.iter_mut() {
        *s -= mean;
    }
}

/// In-place preemphasis: `y[i] = x[i] - coeff * x[i-1]`, with `x[-1] := x[0]`
/// (Kaldi convention).
fn preemphasize(frame: &mut [f32], coeff: f32) {
    if frame.len() < 2 {
        return;
    }
    for i in (1..frame.len()).rev() {
        frame[i] -= coeff * frame[i - 1];
    }
    frame[0] -= coeff * frame[0];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_frames_matches_expected_shift_count() {
        let extractor = FbankExtractor::new();
        // 1 second at 16kHz with a 10ms shift => ~100 frames.
        assert_eq!(extractor.num_frames(16_000), 100);
        assert_eq!(extractor.num_frames(0), 0);
    }

    #[test]
    fn extract_output_shape_matches_num_frames_times_mel_bins() {
        let extractor = FbankExtractor::new();
        let samples = vec![0.0f32; 16_000];
        let feats = extractor.extract(&samples);
        assert_eq!(feats.len(), extractor.num_frames(16_000) * NUM_MEL_BINS);
    }

    #[test]
    fn silence_produces_finite_features() {
        let extractor = FbankExtractor::new();
        let samples = vec![0.0f32; 4_000];
        let feats = extractor.extract(&samples);
        assert!(feats.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn louder_signal_produces_higher_energy_than_silence() {
        let extractor = FbankExtractor::new();
        let silence = vec![0.0f32; 4_000];
        let tone: Vec<f32> = (0..4_000).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();

        let silence_feats = extractor.extract(&silence);
        let tone_feats = extractor.extract(&tone);

        let silence_energy: f32 = silence_feats.iter().sum();
        let tone_energy: f32 = tone_feats.iter().sum();
        assert!(tone_energy > silence_energy);
    }

    #[test]
    fn mel_filterbank_rows_are_nonzero_and_triangular() {
        let fft_size = fft_size_for(frame_length_samples());
        let matrix = mel_filterbank(fft_size, SAMPLE_RATE);
        assert_eq!(matrix.len(), NUM_MEL_BINS);
        for row in &matrix {
            assert_eq!(row.len(), fft_size / 2 + 1);
            assert!(row.iter().any(|&w| w > 0.0));
        }
    }

    #[test]
    fn povey_window_is_zero_at_edges_and_peaks_at_center() {
        let window = povey_window(400);
        assert!(window[0].abs() < 1e-3);
        assert!(window[399].abs() < 1e-3);
        let mid = window[200];
        assert!(mid > window[0] && mid > window[399]);
    }

    #[test]
    fn reflect_sample_mirrors_at_boundaries() {
        let samples = [1.0, 2.0, 3.0];
        assert_eq!(reflect_sample(&samples, -1), 1.0);
        assert_eq!(reflect_sample(&samples, 0), 1.0);
        assert_eq!(reflect_sample(&samples, 3), 3.0);
    }
}
