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

#[derive(Clone)]
pub struct LiveTranslateManager {
    app_handle: AppHandle,
    transcription_manager: Arc<TranscriptionManager>,
    active: Arc<AtomicBool>,
    capture: Arc<Mutex<Option<CaptureStream>>>,
    context: Arc<Mutex<Vec<ContextSegment>>>,
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
            capture: Arc::new(Mutex::new(None)),
            context: Arc::new(Mutex::new(Vec::new())),
            segmenter: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Starts live subtitles capturing from `source`. Returns an error
    /// (surfaced to the UI) if capture could not start, e.g. system audio on
    /// an unsupported platform.
    pub fn start(&self, source: LiveTranslateSource) -> Result<(), String> {
        if self.active.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        self.context.lock().unwrap().clear();

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

    /// Stops live subtitles and tears down the capture stream.
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

    /// Transcribes and translates one closed speech segment, then emits it to
    /// the overlay window. Runs on the capture thread's callback stack via a
    /// spawned blocking task so audio capture is never blocked on network I/O.
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

            let settings = get_settings(&manager.app_handle);
            let source_lang = SubtitleLanguage::from_code(&settings.selected_language);
            let target_lang = source_lang.other();

            let translation = match manager.translate(&transcript, target_lang).await {
                Ok(text) => text,
                Err(e) => {
                    log::error!("Live subtitles translation failed: {e}");
                    // Still show the original so the user gets something.
                    transcript.clone()
                }
            };

            {
                let mut context = manager.context.lock().unwrap();
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
            let _ = line.emit(&manager.app_handle);
        });
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
