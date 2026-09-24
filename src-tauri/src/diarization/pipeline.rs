//! End-to-end diarization: 16 kHz mono samples in, a speaker label per time
//! span out. Glues together [`onnx::SegmentationModel`],
//! [`onnx::EmbeddingModel`], and [`cluster`].

use crate::diarization::cluster::cluster_embeddings;
use crate::diarization::onnx::{EmbeddingModel, SegmentationModel};
use crate::diarization::{default_speaker_label, models::DiarizationModelManager};
use anyhow::Result;

/// One diarized time span, ready to be intersected with transcript segments.
#[derive(Debug, Clone, PartialEq)]
pub struct DiarizedSpan {
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker_label: String,
}

/// Run the full diarization pipeline over a 16 kHz mono recording:
/// segmentation -> per-span embedding -> clustering -> labeled spans.
///
/// Returns an empty vec if the models aren't downloaded yet (callers should
/// check [`DiarizationModelManager::is_ready`] first if they want to surface
/// that distinctly) or if no speech spans were found.
pub fn diarize(
    model_manager: &DiarizationModelManager,
    samples: &[f32],
    cluster_threshold: f32,
) -> Result<Vec<DiarizedSpan>> {
    if !model_manager.is_ready() {
        return Ok(Vec::new());
    }

    let seg_path = model_manager.model_path(crate::diarization::models::DiarizationModelKind::Segmentation)?;
    let emb_path = model_manager.model_path(crate::diarization::models::DiarizationModelKind::Embedding)?;

    let mut segmentation = SegmentationModel::load(&seg_path)?;
    let mut embedding = EmbeddingModel::load(&emb_path)?;

    let sample_rate = segmentation.sample_rate();
    let raw_spans = segmentation.segment(samples)?;
    if raw_spans.is_empty() {
        return Ok(Vec::new());
    }

    let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(raw_spans.len());
    let mut kept_spans = Vec::with_capacity(raw_spans.len());
    for span in &raw_spans {
        let start_sample = (span.start_secs * sample_rate as f64) as usize;
        let end_sample = ((span.end_secs * sample_rate as f64) as usize).min(samples.len());
        if start_sample >= end_sample {
            continue;
        }
        let Some(vec) = embedding.embed(&samples[start_sample..end_sample])? else {
            continue;
        };
        embeddings.push(vec);
        kept_spans.push(span.clone());
    }

    if embeddings.is_empty() {
        return Ok(Vec::new());
    }

    let cluster_labels = cluster_embeddings(&embeddings, cluster_threshold);

    let spans = kept_spans
        .iter()
        .zip(cluster_labels)
        .map(|(span, cluster_id)| DiarizedSpan {
            start_ms: (span.start_secs * 1000.0) as u64,
            end_ms: (span.end_secs * 1000.0) as u64,
            speaker_label: default_speaker_label(cluster_id),
        })
        .collect();

    Ok(spans)
}

/// Assign the diarized speaker whose span overlaps a transcript segment the
/// most (by overlap duration). Returns `None` if no diarized span overlaps
/// the segment at all.
pub fn speaker_for_segment(diarized: &[DiarizedSpan], start_ms: u64, end_ms: u64) -> Option<String> {
    diarized
        .iter()
        .map(|span| {
            let overlap_start = span.start_ms.max(start_ms);
            let overlap_end = span.end_ms.min(end_ms);
            let overlap = overlap_end.saturating_sub(overlap_start);
            (overlap, span)
        })
        .filter(|(overlap, _)| *overlap > 0)
        .max_by_key(|(overlap, _)| *overlap)
        .map(|(_, span)| span.speaker_label.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start_ms: u64, end_ms: u64, label: &str) -> DiarizedSpan {
        DiarizedSpan {
            start_ms,
            end_ms,
            speaker_label: label.to_string(),
        }
    }

    #[test]
    fn speaker_for_segment_picks_largest_overlap() {
        let diarized = vec![span(0, 1000, "Speaker 1"), span(1000, 3000, "Speaker 2")];
        assert_eq!(
            speaker_for_segment(&diarized, 900, 2000),
            Some("Speaker 2".to_string())
        );
    }

    #[test]
    fn speaker_for_segment_returns_none_without_overlap() {
        let diarized = vec![span(0, 1000, "Speaker 1")];
        assert_eq!(speaker_for_segment(&diarized, 2000, 3000), None);
    }

    #[test]
    fn speaker_for_segment_handles_empty_diarization() {
        assert_eq!(speaker_for_segment(&[], 0, 1000), None);
    }

    /// Real end-to-end inference against the actual ONNX models and a real
    /// multi-speaker recording. Ignored by default (CI without the models
    /// still passes `cargo test`); run explicitly with:
    ///
    /// ```sh
    /// HANDY_DIARIZATION_TEST_SEG_MODEL=/path/to/pyannote-segmentation-3.0.onnx \
    /// HANDY_DIARIZATION_TEST_EMB_MODEL=/path/to/wespeaker_en_voxceleb_resnet34.onnx \
    /// HANDY_DIARIZATION_TEST_WAV=/path/to/multi-speaker-16k-mono.wav \
    /// cargo test diarization::pipeline::tests::real_models_diarize_a_multi_speaker_recording -- --ignored
    /// ```
    ///
    /// The three env vars point at the exact files `diarization::models`
    /// downloads (segmentation + embedding ONNX) and any 16 kHz mono WAV
    /// with more than one speaker; sherpa-onnx's own
    /// `speaker-segmentation-models` release includes sample multi-speaker
    /// WAVs suitable for this.
    #[test]
    #[ignore]
    fn real_models_diarize_a_multi_speaker_recording() {
        let seg_path = std::env::var("HANDY_DIARIZATION_TEST_SEG_MODEL")
            .expect("set HANDY_DIARIZATION_TEST_SEG_MODEL to run this test");
        let emb_path = std::env::var("HANDY_DIARIZATION_TEST_EMB_MODEL")
            .expect("set HANDY_DIARIZATION_TEST_EMB_MODEL to run this test");
        let wav_path = std::env::var("HANDY_DIARIZATION_TEST_WAV")
            .expect("set HANDY_DIARIZATION_TEST_WAV to run this test");

        let mut reader = hound::WavReader::open(&wav_path).expect("open test wav");
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, 16_000, "test wav must be 16 kHz");
        assert_eq!(spec.channels, 1, "test wav must be mono");
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.expect("read sample") as f32 / 32768.0)
            .collect();

        let mut segmentation = crate::diarization::onnx::SegmentationModel::load(
            std::path::Path::new(&seg_path),
        )
        .expect("load segmentation model");
        let mut embedding = crate::diarization::onnx::EmbeddingModel::load(std::path::Path::new(
            &emb_path,
        ))
        .expect("load embedding model");

        let raw_spans = segmentation.segment(&samples).expect("run segmentation");
        assert!(
            !raw_spans.is_empty(),
            "expected at least one speech span in a real multi-speaker recording"
        );

        let mut embeddings = Vec::new();
        for span in &raw_spans {
            let start = (span.start_secs * 16_000.0) as usize;
            let end = ((span.end_secs * 16_000.0) as usize).min(samples.len());
            if start >= end {
                continue;
            }
            if let Some(vec) = embedding.embed(&samples[start..end]).expect("embed span") {
                embeddings.push(vec);
            }
        }
        assert!(!embeddings.is_empty(), "expected at least one embedding");

        let labels = cluster_embeddings(&embeddings, 0.2);
        let unique: std::collections::HashSet<usize> = labels.iter().copied().collect();
        assert!(
            unique.len() >= 2,
            "expected at least 2 distinct speakers in a multi-speaker recording, got {}",
            unique.len()
        );
    }
}
