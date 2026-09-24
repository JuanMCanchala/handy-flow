//! ONNX Runtime inference for the two diarization models: pyannote
//! segmentation-3.0 (who's speaking, in overlapping 10 s windows) and a
//! WeSpeaker-style speaker-embedding extractor (a 256-dim vector per speech
//! span). Ports the reference sliding-window + powerset-decoding algorithm
//! from sherpa-onnx's `speaker-diarization-onnx.py` to Rust with `ort`.

use crate::diarization::fbank::FbankExtractor;
use anyhow::{anyhow, Context, Result};
use ndarray::{Array2, Array3, ArrayView3};
use ort::inputs;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

/// `ort::Error` doesn't implement `std::error::Error` in a way `anyhow`'s
/// `Context` trait accepts directly (and its `R` type parameter varies per
/// call site), so ort call sites convert explicitly via `Display` instead.
macro_rules! ort_ctx {
    ($result:expr, $msg:literal) => {
        $result.map_err(|e| anyhow!("{}: {}", $msg, e))
    };
}

/// pyannote segmentation-3.0 metadata, read once from the model file. Fixed
/// for this specific model, but still read from the ONNX metadata rather
/// than hardcoded so a mismatched file fails loudly instead of silently
/// producing garbage output.
#[derive(Debug, Clone, Copy)]
struct SegmentationMeta {
    window_size: usize,
    sample_rate: usize,
    receptive_field_size: usize,
    receptive_field_shift: usize,
    num_speakers: usize,
    num_classes: usize,
    powerset_max_classes: usize,
}

/// Powerset -> multi-label mapping: `mapping[class_idx][speaker_idx]`,
/// matching `get_powerset_mapping` in the reference script (supports the
/// "at most `powerset_max_classes` simultaneous speakers" powerset).
fn powerset_mapping(meta: &SegmentationMeta) -> Vec<Vec<u8>> {
    let mut mapping = vec![vec![0u8; meta.num_speakers]; meta.num_classes];
    let mut k = 1usize;
    for i in 1..=meta.powerset_max_classes {
        if i == 1 {
            for j in 0..meta.num_speakers {
                if k < mapping.len() {
                    mapping[k][j] = 1;
                }
                k += 1;
            }
        } else if i == 2 {
            for j in 0..meta.num_speakers {
                for m in (j + 1)..meta.num_speakers {
                    if k < mapping.len() {
                        mapping[k][j] = 1;
                        mapping[k][m] = 1;
                    }
                    k += 1;
                }
            }
        }
    }
    mapping
}

pub struct SegmentationModel {
    session: Session,
    meta: SegmentationMeta,
}

/// One speech span for one local (in-window) speaker slot, in seconds,
/// before cross-window clustering assigns a final speaker identity.
#[derive(Debug, Clone, PartialEq)]
pub struct RawSpeakerSpan {
    pub start_secs: f64,
    pub end_secs: f64,
    /// Local speaker slot (0..num_speakers). Only meaningful as a grouping
    /// key within one [`SegmentationModel::segment`] call — the embedding +
    /// clustering step is what produces the final, stable speaker label.
    pub local_speaker: usize,
}

impl SegmentationModel {
    pub fn load(path: &Path) -> Result<Self> {
        let session_builder = ort_ctx!(Session::builder(), "ort session builder")?;
        let mut session_builder = ort_ctx!(
            session_builder.with_optimization_level(GraphOptimizationLevel::Level3),
            "ort optimization level"
        )?;
        session_builder = ort_ctx!(session_builder.with_intra_threads(1), "ort intra threads")?;
        let session = ort_ctx!(
            session_builder.commit_from_file(path),
            "failed to load segmentation model"
        )?;

        let meta = {
            let m = ort_ctx!(session.metadata(), "read segmentation metadata")?;
            let get = |key: &str| -> Result<usize> {
                m.custom(key)
                    .ok_or_else(|| anyhow!("segmentation model missing metadata key '{key}'"))?
                    .parse::<usize>()
                    .with_context(|| format!("metadata key '{key}' is not a valid integer"))
            };
            SegmentationMeta {
                window_size: get("window_size")?,
                sample_rate: get("sample_rate")?,
                receptive_field_size: get("receptive_field_size")?,
                receptive_field_shift: get("receptive_field_shift")?,
                num_speakers: get("num_speakers")?,
                num_classes: get("num_classes")?,
                powerset_max_classes: get("powerset_max_classes")?,
            }
        };

        Ok(Self { session, meta })
    }

    pub fn sample_rate(&self) -> usize {
        self.meta.sample_rate
    }

    fn window_shift(&self) -> usize {
        // Matches the reference implementation: window_shift = 0.1 * window_size
        // (a 1s hop over a 10s window).
        self.meta.window_size / 10
    }

    /// Run segmentation over the whole (16 kHz mono) recording and return
    /// per-speaker-slot speech spans, using the reference algorithm: batched
    /// sliding-window inference, powerset decoding to per-frame multi-label
    /// speaker activity, then a single onset/offset pass per local speaker
    /// slot (matching `min_duration_on=0.3`, `onset=offset=0.5`).
    ///
    /// Local speaker slots are *not* stable speaker identities across the
    /// call — see [`RawSpeakerSpan`].
    pub fn segment(&mut self, samples: &[f32]) -> Result<Vec<RawSpeakerSpan>> {
        let window_size = self.meta.window_size;
        let window_shift = self.window_shift();

        if samples.is_empty() {
            return Ok(Vec::new());
        }

        let has_last_chunk =
            samples.len() < window_size || (samples.len() - window_size) % window_shift > 0;
        let num_full_chunks = if samples.len() >= window_size {
            (samples.len() - window_size) / window_shift + 1
        } else {
            0
        };

        let mut chunk_outputs: Vec<Array3<f32>> = Vec::new();

        const BATCH_SIZE: usize = 32;
        let mut chunk_idx = 0;
        while chunk_idx < num_full_chunks {
            let batch_end = (chunk_idx + BATCH_SIZE).min(num_full_chunks);
            let batch_count = batch_end - chunk_idx;

            let mut batch = Array3::<f32>::zeros((batch_count, 1, window_size));
            for b in 0..batch_count {
                let start = (chunk_idx + b) * window_shift;
                for t in 0..window_size {
                    batch[[b, 0, t]] = samples[start + t];
                }
            }

            let y = self.run_segmentation(&batch)?;
            chunk_outputs.push(y);
            chunk_idx = batch_end;
        }

        if has_last_chunk {
            let start = num_full_chunks * window_shift;
            let mut batch = Array3::<f32>::zeros((1, 1, window_size));
            for (t, sample) in samples[start..].iter().enumerate() {
                batch[[0, 0, t]] = *sample;
            }
            let y = self.run_segmentation(&batch)?;
            chunk_outputs.push(y);
        }

        if chunk_outputs.is_empty() {
            return Ok(Vec::new());
        }

        let num_frames_per_chunk = chunk_outputs[0].shape()[1];
        let num_chunks: usize = chunk_outputs.iter().map(|c| c.shape()[0]).sum();
        let mapping = powerset_mapping(&self.meta);

        // labels[chunk][frame][speaker] = 1 if that local speaker slot is
        // active in that frame (from the model's argmax powerset class).
        let mut labels =
            vec![vec![vec![0u8; self.meta.num_speakers]; num_frames_per_chunk]; num_chunks];
        let mut chunk_i = 0;
        for chunk_output in &chunk_outputs {
            let (n, frames, classes) = chunk_output.dim();
            for b in 0..n {
                for f in 0..frames {
                    let mut best_class = 0usize;
                    let mut best_val = f32::MIN;
                    for c in 0..classes {
                        let v = chunk_output[[b, f, c]];
                        if v > best_val {
                            best_val = v;
                            best_class = c;
                        }
                    }
                    labels[chunk_i][f] = mapping[best_class].clone();
                }
                chunk_i += 1;
            }
        }

        // Aggregate overlapping chunk predictions into per-receptive-field
        // frame speaker activity, matching `speaker_count`/the final
        // aggregation pass in the reference script (mean over overlapping
        // chunk predictions, done per-speaker here).
        let total_frames = (window_size + (num_chunks.max(1) - 1) * window_shift)
            / self.meta.receptive_field_shift
            + 1;
        let mut activity_sum = vec![vec![0f32; self.meta.num_speakers]; total_frames];
        let mut activity_count = vec![0f32; total_frames];

        for (i, chunk_labels) in labels.iter().enumerate() {
            let start =
                ((i * window_shift) as f64 / self.meta.receptive_field_shift as f64 + 0.5) as usize;
            for (f, frame_labels) in chunk_labels.iter().enumerate() {
                let out_frame = start + f;
                if out_frame >= total_frames {
                    continue;
                }
                for (s, &active) in frame_labels.iter().enumerate() {
                    activity_sum[out_frame][s] += active as f32;
                }
                activity_count[out_frame] += 1.0;
            }
        }

        let stop_frame = if has_last_chunk {
            (samples.len() as f64 / self.meta.receptive_field_shift as f64) as usize
        } else {
            total_frames
        };
        let stop_frame = stop_frame.min(total_frames);

        let scale = self.meta.receptive_field_shift as f64 / self.meta.sample_rate as f64;
        let scale_offset =
            self.meta.receptive_field_size as f64 / self.meta.sample_rate as f64 * 0.5;

        const ONSET: f32 = 0.5;
        const OFFSET: f32 = 0.5;
        const MIN_DURATION_ON: f64 = 0.3;

        let mut spans = Vec::new();
        for speaker in 0..self.meta.num_speakers {
            let frame_activity: Vec<f32> = (0..stop_frame)
                .map(|f| {
                    let count = activity_count[f].max(1e-12);
                    activity_sum[f][speaker] / count
                })
                .collect();
            if frame_activity.is_empty() {
                continue;
            }

            let mut is_active = frame_activity[0] > ONSET;
            let mut start_frame = if is_active { Some(0usize) } else { None };

            for i in 1..frame_activity.len() {
                if is_active {
                    if frame_activity[i] < OFFSET {
                        if let Some(sf) = start_frame {
                            push_span(
                                &mut spans,
                                sf,
                                i,
                                speaker,
                                scale,
                                scale_offset,
                                MIN_DURATION_ON,
                            );
                        }
                        is_active = false;
                        start_frame = None;
                    }
                } else if frame_activity[i] > ONSET {
                    start_frame = Some(i);
                    is_active = true;
                }
            }
            if is_active {
                if let Some(sf) = start_frame {
                    push_span(
                        &mut spans,
                        sf,
                        frame_activity.len() - 1,
                        speaker,
                        scale,
                        scale_offset,
                        MIN_DURATION_ON,
                    );
                }
            }
        }

        spans.sort_by(|a, b| a.start_secs.partial_cmp(&b.start_secs).unwrap());
        Ok(spans)
    }

    fn run_segmentation(&mut self, batch: &Array3<f32>) -> Result<Array3<f32>> {
        let input = ort_ctx!(
            TensorRef::from_array_view(batch.view().into_dyn()),
            "segmentation input tensor"
        )?;
        let outputs = ort_ctx!(
            self.session.run(inputs!["x" => input]),
            "segmentation inference"
        )?;
        let y = ort_ctx!(
            outputs[0].try_extract_array::<f32>(),
            "extract segmentation output"
        )?;
        let view: ArrayView3<f32> = y
            .into_dimensionality()
            .context("segmentation output is not 3-D")?;
        Ok(view.to_owned())
    }
}

#[allow(clippy::too_many_arguments)]
fn push_span(
    spans: &mut Vec<RawSpeakerSpan>,
    start_frame: usize,
    end_frame: usize,
    speaker: usize,
    scale: f64,
    scale_offset: f64,
    min_duration_on: f64,
) {
    let start_secs = start_frame as f64 * scale + scale_offset;
    let end_secs = end_frame as f64 * scale + scale_offset;
    if end_secs - start_secs < min_duration_on {
        return;
    }
    spans.push(RawSpeakerSpan {
        start_secs,
        end_secs,
        local_speaker: speaker,
    });
}

/// WeSpeaker-style speaker-embedding extractor: 80-dim log-mel fbank frames
/// in, a 256-dim embedding out.
pub struct EmbeddingModel {
    session: Session,
    fbank: FbankExtractor,
}

impl EmbeddingModel {
    pub fn load(path: &Path) -> Result<Self> {
        let session_builder = ort_ctx!(Session::builder(), "ort session builder")?;
        let mut session_builder = ort_ctx!(
            session_builder.with_optimization_level(GraphOptimizationLevel::Level3),
            "ort optimization level"
        )?;
        session_builder = ort_ctx!(session_builder.with_intra_threads(1), "ort intra threads")?;
        let session = ort_ctx!(
            session_builder.commit_from_file(path),
            "failed to load embedding model"
        )?;

        Ok(Self {
            session,
            fbank: FbankExtractor::new(),
        })
    }

    /// Compute a speaker embedding for a span of 16 kHz mono samples. Returns
    /// `None` if the span is too short to produce any fbank frames.
    pub fn embed(&mut self, samples: &[f32]) -> Result<Option<Vec<f32>>> {
        let num_frames = self.fbank.num_frames(samples.len());
        if num_frames == 0 {
            return Ok(None);
        }

        let flat = self.fbank.extract(samples);
        let feats = Array2::from_shape_vec((num_frames, 80), flat)
            .context("reshape fbank features")?
            .insert_axis(ndarray::Axis(0)); // [1, T, 80]

        let input = ort_ctx!(
            TensorRef::from_array_view(feats.view().into_dyn()),
            "embedding input tensor"
        )?;
        let outputs = ort_ctx!(
            self.session.run(inputs!["feats" => input]),
            "embedding inference"
        )?;
        let embedding = ort_ctx!(
            outputs[0].try_extract_array::<f32>(),
            "extract embedding output"
        )?;

        Ok(Some(embedding.iter().copied().collect()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> SegmentationMeta {
        SegmentationMeta {
            window_size: 160_000,
            sample_rate: 16_000,
            receptive_field_size: 991,
            receptive_field_shift: 270,
            num_speakers: 3,
            num_classes: 7,
            powerset_max_classes: 2,
        }
    }

    #[test]
    fn powerset_mapping_matches_reference_layout_for_3_speakers() {
        // Mirrors get_powerset_mapping(num_classes=7, num_speakers=3, powerset_max_classes=2):
        // class 0 = silence (all zero), classes 1-3 = single speakers, classes 4-6 = pairs.
        let mapping = powerset_mapping(&meta());

        assert_eq!(mapping[0], vec![0, 0, 0]);
        assert_eq!(mapping[1], vec![1, 0, 0]);
        assert_eq!(mapping[2], vec![0, 1, 0]);
        assert_eq!(mapping[3], vec![0, 0, 1]);
        assert_eq!(mapping[4], vec![1, 1, 0]);
        assert_eq!(mapping[5], vec![1, 0, 1]);
        assert_eq!(mapping[6], vec![0, 1, 1]);
    }

    #[test]
    fn push_span_drops_spans_shorter_than_min_duration() {
        let mut spans = Vec::new();
        push_span(&mut spans, 0, 1, 0, 0.01, 0.0, 0.3);
        assert!(spans.is_empty());
    }

    #[test]
    fn push_span_keeps_spans_at_or_above_min_duration() {
        let mut spans = Vec::new();
        push_span(&mut spans, 0, 100, 0, 0.01, 0.0, 0.3);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].end_secs - spans[0].start_secs - 1.0).abs() < 1e-9);
    }
}
