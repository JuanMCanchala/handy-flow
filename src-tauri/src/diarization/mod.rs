//! Speaker diarization: label which speaker said each transcript segment.
//!
//! Pipeline: VAD-derived speech segments -> one embedding per segment (via
//! an ONNX speaker-embedding model) -> agglomerative clustering (see
//! [`cluster`]) -> a speaker index per segment, rendered as "Speaker N".
//!
//! This module owns the pure label-assignment step. Extracting embeddings
//! from audio requires the ONNX segmentation/embedding models managed by
//! [`models::DiarizationModelManager`]; model inference itself is out of
//! scope here so this stays unit-testable without any model files on disk.

pub mod cluster;
pub mod fbank;
pub mod models;
pub mod onnx;
pub mod pipeline;

/// Human-facing label for a 0-based speaker index, e.g. `speaker_index(0)`
/// -> "Speaker 1". Renameable labels (user overrides) are stored separately
/// by the caller; this is only the default.
pub fn default_speaker_label(speaker_index: usize) -> String {
    format!("Speaker {}", speaker_index + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_speaker_label_is_one_indexed() {
        assert_eq!(default_speaker_label(0), "Speaker 1");
        assert_eq!(default_speaker_label(1), "Speaker 2");
    }
}
