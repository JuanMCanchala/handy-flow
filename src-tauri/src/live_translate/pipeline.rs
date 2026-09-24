//! Orchestrates live subtitles: capture -> VAD -> segmenter -> transcribe ->
//! translate -> subtitle overlay.
//!
//! Live subtitles run on their own audio stream, entirely separate from the
//! dictation `AudioRecordingManager`/`StreamRouter` pipeline. Starting a
//! dictation shortcut while live subtitles are active stops live subtitles
//! first (see `actions.rs`), so the two features never read the same
//! microphone stream or contend for the transcription engine lock at the
//! same time. `TranscriptionManager::transcribe` takes its own lock per call,
//! so a stray overlap could not corrupt state, but it would otherwise block
//! dictation behind however long the current subtitle segment takes.

use super::capture::{self, CaptureStream};
use super::prompt::{build_translation_prompt, ContextSegment, SubtitleLanguage};
use super::segmenter::{SegmenterConfig, SpeechSegmenter};
use crate::audio_toolkit::vad::{
    frames_for_duration_ms, SmoothedVad, VAD_ONSET_MS, VAD_PREFILL_MS, VAD_STREAMING_HANGOVER_MS,
};
use crate::audio_toolkit::{EarshotVad, SileroVad, VoiceActivityDetector};
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, LiveTranslateSource, VadBackend};
use serde::Serialize;
use specta::Type;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tauri_specta::Event;

const SILERO_VAD_THRESHOLD: f32 = 0.3;
const EARSHOT_VAD_THRESHOLD: f32 = 0.5;
const MAX_CONTEXT_HISTORY: usize = 3;

/// One rendered subtitle line, emitted to the overlay window.
#[derive(Debug, Clone, Serialize, Type, tauri_specta::Event)]
pub struct LiveSubtitleLine {
    pub original: String,
    pub translation: String,
}

/// One answer suggestion, emitted to the overlay window and appended to the
/// Copilot history.
#[derive(Debug, Clone, Serialize, Type, tauri_specta::Event)]
pub struct CopilotAnswerLine {
    pub question: String,
    pub answer: String,
}

/// Which behavior a capture session runs: translated subtitles, or the
/// profile copilot's question detection + answer suggestion. The two share
/// capture/VAD/segmenter/transcription; only what happens with a closed
/// segment differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveTranslateMode {
    Subtitles,
    Copilot,
}

#[derive(Clone)]
pub struct LiveTranslateManager {
    app_handle: AppHandle,
    transcription_manager: Arc<TranscriptionManager>,
    active: Arc<AtomicBool>,
    mode: Arc<Mutex<LiveTranslateMode>>,
    capture: Arc<Mutex<Option<CaptureStream>>>,
    context: Arc<Mutex<Vec<ContextSegment>>>,
    /// Recent transcript segments (plain text, oldest first), used as
    /// conversation context for copilot answers. Independent of `context`
    /// (which pairs original+translation for subtitles).
    transcript_history: Arc<Mutex<Vec<String>>>,
    /// Live during a capture session so `stop()` can flush the in-progress
    /// segment instead of discarding it.
    segmenter: Arc<Mutex<Option<SpeechSegmenter>>>,
}

impl LiveTranslateManager {
    pub fn new(app_handle: &AppHandle, transcription_manager: Arc<TranscriptionManager>) -> Self {
        Self {
            app_handle: app_handle.clone(),
            transcription_manager,
            active: Arc::new(AtomicBool::new(false)),
            mode: Arc::new(Mutex::new(LiveTranslateMode::Subtitles)),
            capture: Arc::new(Mutex::new(None)),
            context: Arc::new(Mutex::new(Vec::new())),
            transcript_history: Arc::new(Mutex::new(Vec::new())),
            segmenter: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Active specifically in copilot mode (vs. subtitles). Used by the UI to
    /// tell the two toggle buttons apart.
    pub fn is_copilot_active(&self) -> bool {
        self.is_active() && *self.mode.lock().unwrap() == LiveTranslateMode::Copilot
    }

    /// Starts live subtitles capturing from `source`.
    pub fn start(&self, source: LiveTranslateSource) -> Result<(), String> {
        self.start_with_mode(source, LiveTranslateMode::Subtitles)
    }

    /// Starts the profile copilot capturing from `source`: same capture/VAD/
    /// segmenter/transcription pipeline as subtitles, but closed segments are
    /// checked for questions and answered instead of translated.
    pub fn start_copilot(&self, source: LiveTranslateSource) -> Result<(), String> {
        self.start_with_mode(source, LiveTranslateMode::Copilot)
    }

    /// Starts capturing from `source` in the given `mode`. Returns an error
    /// (surfaced to the UI) if capture could not start, e.g. system audio on
    /// an unsupported platform.
    fn start_with_mode(
        &self,
        source: LiveTranslateSource,
        mode: LiveTranslateMode,
    ) -> Result<(), String> {
        if self.active.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        *self.mode.lock().unwrap() = mode;
        self.context.lock().unwrap().clear();
        self.transcript_history.lock().unwrap().clear();

        let detector = build_vad(&self.app_handle)?;
        let segmenter_config = SegmenterConfig::default();
        *self.segmenter.lock().unwrap() = Some(SpeechSegmenter::new(segmenter_config));
        let segmenter = Arc::clone(&self.segmenter);
        let vad = Arc::new(Mutex::new(detector));

        let manager = self.clone();
        let frame_samples = vad.lock().unwrap().frame_samples();
        let mut pending: Vec<f32> = Vec::with_capacity(frame_samples * 2);

        let on_frame = move |frame: &[f32]| {
            pending.extend_from_slice(frame);
            while pending.len() >= frame_samples {
                let chunk: Vec<f32> = pending.drain(..frame_samples).collect();
                let is_speech = match vad.lock().unwrap().is_voice(&chunk) {
                    Ok(speech) => speech,
                    Err(e) => {
                        log::error!("Live subtitles VAD error: {e}");
                        false
                    }
                };

                let closed_segment = segmenter
                    .lock()
                    .unwrap()
                    .as_mut()
                    .and_then(|s| s.push(&chunk, is_speech));
                if let Some(segment) = closed_segment {
                    manager.process_segment(segment);
                }
            }
        };

        let capture_result = match source {
            LiveTranslateSource::Microphone => capture::start_microphone_capture(on_frame),
            LiveTranslateSource::SystemAudio => capture::start_system_audio_capture(on_frame),
        };

        let stream = match capture_result {
            Ok(stream) => stream,
            Err(e) => {
                self.active.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };

        *self.capture.lock().unwrap() = Some(stream);
        super::overlay::show(&self.app_handle);
        Ok(())
    }

    /// Stops the active capture session (subtitles or copilot) and tears down
    /// the capture stream.
    pub fn stop(&self) {
        if !self.active.swap(false, Ordering::SeqCst) {
            return;
        }
        // Flush and process whatever partial segment was buffered so the
        // last few words spoken before stopping are not silently dropped.
        // (Processed asynchronously; may emit its line after the overlay is
        // hidden below, in which case the overlay simply reappears briefly.)
        let flushed = self
            .segmenter
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|s| s.flush());
        *self.capture.lock().unwrap() = None;
        *self.segmenter.lock().unwrap() = None;
        if let Some(segment) = flushed {
            self.process_segment(segment);
        }
        super::overlay::hide(&self.app_handle);
    }

    /// Transcribes one closed speech segment, then dispatches it to the
    /// translation (subtitles) or question-answering (copilot) path
    /// depending on the active mode. Runs on the capture thread's callback
    /// stack via a spawned blocking task so audio capture is never blocked on
    /// network I/O.
    fn process_segment(&self, segment: Vec<f32>) {
        // Ignore segments too short to be meaningful speech (VAD noise).
        if segment.len() < crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE as usize / 5 {
            return;
        }

        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            let transcript = match manager.transcription_manager.transcribe(segment) {
                Ok(text) => text.trim().to_string(),
                Err(e) => {
                    log::error!("Live subtitles transcription failed: {e}");
                    return;
                }
            };

            if transcript.is_empty() {
                return;
            }

            let mode = *manager.mode.lock().unwrap();
            match mode {
                LiveTranslateMode::Subtitles => manager.handle_subtitle_segment(transcript).await,
                LiveTranslateMode::Copilot => manager.handle_copilot_segment(transcript).await,
            }
        });
    }

    async fn handle_subtitle_segment(&self, transcript: String) {
        let settings = get_settings(&self.app_handle);
        let source_lang = SubtitleLanguage::from_code(&settings.selected_language);
        let target_lang = source_lang.other();

        let translation = match self.translate(&transcript, target_lang).await {
            Ok(text) => text,
            Err(e) => {
                log::error!("Live subtitles translation failed: {e}");
                // Still show the original so the user gets something.
                transcript.clone()
            }
        };

        {
            let mut context = self.context.lock().unwrap();
            context.push(ContextSegment {
                original: transcript.clone(),
                translation: translation.clone(),
            });
            if context.len() > MAX_CONTEXT_HISTORY {
                let excess = context.len() - MAX_CONTEXT_HISTORY;
                context.drain(0..excess);
            }
        }

        let line = LiveSubtitleLine {
            original: transcript,
            translation,
        };
        let _ = line.emit(&self.app_handle);
    }

    /// Runs the copilot's question detector on a closed segment; on a match,
    /// requests an answer suggestion from the configured LLM (grounded in the
    /// user's profile) and emits it to the overlay + history.
    async fn handle_copilot_segment(&self, transcript: String) {
        self.transcript_history.lock().unwrap().push(transcript.clone());
        self.trim_transcript_history();

        if !crate::copilot::is_question(&transcript) {
            return;
        }

        let profile = crate::copilot::get_profile(&self.app_handle);
        let recent_transcript = {
            let history = self.transcript_history.lock().unwrap();
            // Exclude the question itself; the prompt passes it separately.
            history[..history.len().saturating_sub(1)].to_vec()
        };

        let settings = get_settings(&self.app_handle);
        let (provider, model, api_key) = match settings.resolve_llm_target() {
            Some(target) => target,
            None => {
                log::error!("Copilot: no LLM configured for answer generation");
                return;
            }
        };

        let prompt = crate::copilot::build_answer_prompt(
            &profile.text,
            &recent_transcript,
            &transcript,
            profile.answer_language,
        );

        let answer =
            match crate::llm_client::send_chat_completion(&provider, api_key, &model, prompt, true)
                .await
            {
                Ok(Some(text)) => text.trim().to_string(),
                Ok(None) => {
                    log::error!("Copilot: empty answer response");
                    return;
                }
                Err(e) => {
                    log::error!("Copilot: answer generation failed: {e}");
                    return;
                }
            };

        if answer.is_empty() {
            return;
        }

        let timestamp = chrono::Utc::now().timestamp_millis();
        let entry = crate::copilot::CopilotAnswerEntry {
            id: timestamp.to_string(),
            question: transcript.clone(),
            answer: answer.clone(),
            timestamp,
        };
        crate::copilot::append_entry(&self.app_handle, entry);

        let line = CopilotAnswerLine {
            question: transcript,
            answer,
        };
        let _ = line.emit(&self.app_handle);
    }

    fn trim_transcript_history(&self) {
        let mut history = self.transcript_history.lock().unwrap();
        // Keep one extra slot beyond the context window: the newest entry is
        // the just-closed segment itself (excluded from context by the
        // caller), so MAX_CONTEXT_SEGMENTS prior segments need to survive
        // alongside it.
        let cap = super::prompt::MAX_CONTEXT_SEGMENTS + 1;
        if history.len() > cap {
            let excess = history.len() - cap;
            history.drain(0..excess);
        }
    }

    async fn translate(&self, text: &str, target: SubtitleLanguage) -> Result<String, String> {
        let settings = get_settings(&self.app_handle);
        let (provider, model, api_key) = settings
            .resolve_llm_target()
            .ok_or_else(|| "No LLM configured for translation".to_string())?;

        let context_snapshot = self.context.lock().unwrap().clone();
        let prompt = build_translation_prompt(text, target, &context_snapshot);

        let result =
            crate::llm_client::send_chat_completion(&provider, api_key, &model, prompt, true)
                .await?;

        result.ok_or_else(|| "Empty translation response".to_string())
    }
}

fn build_vad(app_handle: &AppHandle) -> Result<Box<dyn VoiceActivityDetector>, String> {
    let settings = get_settings(app_handle);
    let detector: Box<dyn VoiceActivityDetector> = match settings.vad_backend {
        VadBackend::Silero => {
            let vad_path = tauri::Manager::path(app_handle)
                .resolve(
                    "resources/models/silero_vad_v4.onnx",
                    tauri::path::BaseDirectory::Resource,
                )
                .map_err(|e| format!("Failed to resolve VAD path: {e}"))?;
            Box::new(
                SileroVad::new(vad_path, SILERO_VAD_THRESHOLD)
                    .map_err(|e| format!("Failed to create SileroVad: {e}"))?,
            )
        }
        VadBackend::Earshot => Box::new(
            EarshotVad::new(EARSHOT_VAD_THRESHOLD)
                .map_err(|e| format!("Failed to create EarshotVad: {e}"))?,
        ),
    };

    let frame_samples = detector.frame_samples();
    let prefill_frames = frames_for_duration_ms(VAD_PREFILL_MS, frame_samples);
    let hangover_frames = frames_for_duration_ms(VAD_STREAMING_HANGOVER_MS, frame_samples);
    let onset_frames = frames_for_duration_ms(VAD_ONSET_MS, frame_samples);

    Ok(Box::new(SmoothedVad::new(
        detector,
        prefill_frames,
        hangover_frames,
        onset_frames,
    )))
}
