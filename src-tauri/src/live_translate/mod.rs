//! Live EN<->ES subtitles: captures audio (microphone or, on Windows, system
//! loopback audio), segments speech with the existing VAD, transcribes each
//! segment with the currently selected model, and translates it via the
//! configured LLM. Results are pushed to the subtitle overlay window.
//!
//! Live subtitles and normal dictation use separate audio streams; starting a
//! dictation shortcut stops live subtitles first (see `actions.rs`), so the
//! two never compete for the same input device or the transcription engine
//! lock at the same time.

mod capture;
mod overlay;
mod pipeline;
mod prompt;
mod segmenter;

pub use pipeline::{CopilotAnswerLine, LiveSubtitleLine, LiveTranslateManager};
