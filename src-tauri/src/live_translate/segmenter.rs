//! Speech segmenter for live subtitles: accumulates VAD-classified frames into
//! sentence-like segments, closing a segment after ~600ms of trailing silence
//! or once it reaches an 8s hard cap.
//!
//! Adapted from the segmentation approach in NBS282/LiveTranslate
//! (`src-tauri/src/translation/segmenter.rs`, MIT licensed) — timing
//! thresholds re-tuned to Handy's frame/VAD conventions.

use crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE;

/// Default trailing-silence duration that closes an in-progress segment.
pub const DEFAULT_SILENCE_CLOSE_MS: u64 = 400;
/// Default hard cap on segment length, regardless of continued speech.
pub const DEFAULT_MAX_SEGMENT_MS: u64 = 4_500;

#[derive(Debug, Clone, Copy)]
pub struct SegmenterConfig {
    pub silence_close_ms: u64,
    pub max_segment_ms: u64,
    pub sample_rate: u32,
}

impl Default for SegmenterConfig {
    fn default() -> Self {
        Self {
            silence_close_ms: DEFAULT_SILENCE_CLOSE_MS,
            max_segment_ms: DEFAULT_MAX_SEGMENT_MS,
            sample_rate: WHISPER_SAMPLE_RATE,
        }
    }
}

/// Accumulates speech/silence frames (already classified by the VAD) into
/// discrete segments. Callers feed one frame at a time via `push`; a returned
/// `Vec<f32>` means a segment just closed and is ready to transcribe.
pub struct SpeechSegmenter {
    config: SegmenterConfig,
    buffer: Vec<f32>,
    trailing_silence_samples: u64,
    in_speech: bool,
}

impl SpeechSegmenter {
    pub fn new(config: SegmenterConfig) -> Self {
        Self {
            config,
            buffer: Vec::new(),
            trailing_silence_samples: 0,
            in_speech: false,
        }
    }

    fn silence_close_samples(&self) -> u64 {
        self.config.sample_rate as u64 * self.config.silence_close_ms / 1000
    }

    fn max_segment_samples(&self) -> u64 {
        self.config.sample_rate as u64 * self.config.max_segment_ms / 1000
    }

    /// Feed one frame of audio with its VAD classification. Returns a
    /// completed segment if this frame closed one (trailing silence reached
    /// the close threshold, or the max segment duration was hit).
    pub fn push(&mut self, frame: &[f32], is_speech: bool) -> Option<Vec<f32>> {
        if is_speech {
            self.in_speech = true;
            self.trailing_silence_samples = 0;
            self.buffer.extend_from_slice(frame);

            if self.buffer.len() as u64 >= self.max_segment_samples() {
                return Some(self.take_segment());
            }
            None
        } else {
            if !self.in_speech {
                // Silence before any speech started: nothing buffered yet.
                return None;
            }

            self.trailing_silence_samples += frame.len() as u64;
            self.buffer.extend_from_slice(frame);

            if self.trailing_silence_samples >= self.silence_close_samples() {
                return Some(self.take_segment());
            }

            if self.buffer.len() as u64 >= self.max_segment_samples() {
                return Some(self.take_segment());
            }

            None
        }
    }

    /// Force-close and return whatever is currently buffered (e.g. when the
    /// capture stream stops). Returns `None` if nothing was buffered.
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(self.take_segment())
        }
    }

    fn take_segment(&mut self) -> Vec<f32> {
        self.in_speech = false;
        self.trailing_silence_samples = 0;
        std::mem::take(&mut self.buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> SegmenterConfig {
        SegmenterConfig {
            silence_close_ms: 600,
            max_segment_ms: 8_000,
            sample_rate: 16_000,
        }
    }

    fn frame(len: usize, value: f32) -> Vec<f32> {
        vec![value; len]
    }

    #[test]
    fn silence_before_speech_is_ignored() {
        let mut seg = SpeechSegmenter::new(cfg());
        let result = seg.push(&frame(1600, 0.0), false);
        assert!(result.is_none());
    }

    #[test]
    fn closes_segment_after_trailing_silence_threshold() {
        let mut seg = SpeechSegmenter::new(cfg());
        // 500ms of speech.
        assert!(seg.push(&frame(8_000, 0.5), true).is_none());

        // 599ms of silence: not yet closed.
        assert!(seg.push(&frame(9_584, 0.0), false).is_none());

        // One more ms (16 samples) crosses the 600ms threshold.
        let closed = seg.push(&frame(16, 0.0), false);
        assert!(closed.is_some());
        let segment = closed.unwrap();
        // Segment includes speech + the trailing silence buffered with it.
        assert_eq!(segment.len(), 8_000 + 9_584 + 16);
    }

    #[test]
    fn resets_trailing_silence_counter_on_new_speech() {
        let mut seg = SpeechSegmenter::new(cfg());
        assert!(seg.push(&frame(8_000, 0.5), true).is_none());
        // 400ms silence (below the 600ms threshold).
        assert!(seg.push(&frame(6_400, 0.0), false).is_none());
        // Speech resumes: silence counter must reset.
        assert!(seg.push(&frame(1_000, 0.5), true).is_none());
        // Another 400ms of silence should NOT close the segment (total would
        // exceed 600ms only if the counter hadn't reset).
        assert!(seg.push(&frame(6_400, 0.0), false).is_none());
    }

    #[test]
    fn closes_segment_at_max_duration_while_still_speaking() {
        let mut seg = SpeechSegmenter::new(cfg());
        // 7.9s of continuous speech: not yet at the 8s cap.
        assert!(seg.push(&frame(126_400, 0.5), true).is_none());
        // 200ms more speech crosses the 8s (128,000 sample) cap.
        let closed = seg.push(&frame(3_200, 0.5), true);
        assert!(closed.is_some());
        assert_eq!(closed.unwrap().len(), 126_400 + 3_200);
    }

    #[test]
    fn closes_segment_at_max_duration_during_silence() {
        let mut seg = SpeechSegmenter::new(cfg());
        // 7.9s speech, then a burst of silence shorter than the close
        // threshold but long enough to cross the max-duration cap.
        assert!(seg.push(&frame(126_400, 0.5), true).is_none());
        let closed = seg.push(&frame(3_200, 0.0), false);
        assert!(closed.is_some());
        assert_eq!(closed.unwrap().len(), 126_400 + 3_200);
    }

    #[test]
    fn flush_returns_buffered_partial_segment() {
        let mut seg = SpeechSegmenter::new(cfg());
        assert!(seg.push(&frame(4_000, 0.5), true).is_none());
        let flushed = seg.flush();
        assert_eq!(flushed, Some(frame(4_000, 0.5)));
        // A second flush with nothing buffered returns None.
        assert!(seg.flush().is_none());
    }

    #[test]
    fn new_segment_starts_clean_after_close() {
        let mut seg = SpeechSegmenter::new(cfg());
        assert!(seg.push(&frame(8_000, 0.5), true).is_none());
        let closed = seg.push(&frame(9_600, 0.0), false);
        assert!(closed.is_some());

        // Silence after a closed segment (before new speech starts) is
        // ignored again, just like at the very start.
        assert!(seg.push(&frame(1_600, 0.0), false).is_none());
    }
}
