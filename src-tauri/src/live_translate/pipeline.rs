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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};
use tauri_specta::Event;

const SILERO_VAD_THRESHOLD: f32 = 0.3;
const EARSHOT_VAD_THRESHOLD: f32 = 0.5;
const MAX_CONTEXT_HISTORY: usize = 3;
/// After a pause closes an utterance, how long (from the segment close) the
/// speaker must stay quiet before the utterance is treated as a finished
/// turn and answered. Transcription usually takes longer than this, so the
/// wait is typically zero; it only avoids answering half a question when the
/// interviewer pauses mid-sentence.
const TURN_GRACE_MS: u64 = 550;
/// While the other side is talking (and this long after), the user's
/// microphone is treated as silent: with speakers instead of headphones the
/// mic hears the interviewer too, and that echo must not become "Me".
const ECHO_GUARD_MS: u64 = 350;
/// Conversation lines (both speakers) kept as context for answers.
const MAX_CONVERSATION_LINES: usize = 10;
/// Cap on the fragments merged into one utterance (monologues).
const MAX_UTTERANCE_FRAGMENTS: usize = 10;
/// Minimum spacing between streamed answer/translation updates.
const STREAM_EMIT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(40);

static EPOCH: std::sync::LazyLock<std::time::Instant> =
    std::sync::LazyLock::new(std::time::Instant::now);

fn now_ms() -> u64 {
    EPOCH.elapsed().as_millis() as u64
}

/// Who a segment came from. In system-audio sessions the loopback is the
/// other side of the call (`Them`) and the microphone is the user (`Me`); a
/// microphone-only session has a single `Them` channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Speaker {
    Them,
    Me,
}

impl Speaker {
    fn label(self) -> &'static str {
        match self {
            Speaker::Them => "Interviewer",
            Speaker::Me => "Me",
        }
    }
}

/// A closed segment whose transcription is in flight. Queued in capture
/// order so lines and utterances stay ordered even though transcriptions run
/// concurrently.
struct PendingSegment {
    transcript: tauri::async_runtime::JoinHandle<Option<String>>,
    closed_by_cap: bool,
    closed_at_ms: u64,
    speaker: Speaker,
}

/// A transcribed segment handed to the answer-suggestion stage.
struct UtteranceFragment {
    text: String,
    closed_by_cap: bool,
    closed_at_ms: u64,
    speaker: Speaker,
}

/// One rendered subtitle line, emitted to the overlay window.
#[derive(Debug, Clone, Serialize, Type, tauri_specta::Event)]
pub struct LiveSubtitleLine {
    /// Stable per line: the original is emitted as soon as it is transcribed
    /// (with an empty translation) and re-emitted with the same id once the
    /// translation arrives, so the overlay updates the line in place.
    pub id: u32,
    /// Language detected for `original` ("en" or "es"); the live view uses it
    /// to route the line to the EN→ES or ES→EN column.
    pub source_lang: String,
    pub original: String,
    pub translation: String,
}

static NEXT_LINE_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);

/// One answer suggestion, emitted to the overlay window and appended to the
/// Copilot history.
///
/// Streamed: emitted with an empty `answer` as soon as a question is
/// detected, then re-emitted with the same `id` as the answer grows, and a
/// last time with `done` set. A `done` line with an empty answer means the
/// suggestion failed and should be dropped.
#[derive(Debug, Clone, Serialize, Type, tauri_specta::Event)]
pub struct CopilotAnswerLine {
    pub id: u32,
    pub question: String,
    pub answer: String,
    pub done: bool,
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
    /// The user's microphone, captured alongside system audio as `Me`.
    me_capture: Arc<Mutex<Option<CaptureStream>>>,
    context: Arc<Mutex<Vec<ContextSegment>>>,
    /// Recent transcript segments (plain text, oldest first), used as
    /// conversation context for copilot answers. Independent of `context`
    /// (which pairs original+translation for subtitles).
    transcript_history: Arc<Mutex<Vec<String>>>,
    /// Live during a capture session so `stop()` can flush the in-progress
    /// segment instead of discarding it.
    segmenter: Arc<Mutex<Option<SpeechSegmenter>>>,
    me_segmenter: Arc<Mutex<Option<SpeechSegmenter>>>,
    /// Ordered queue of closed segments for the session (see
    /// `run_segment_consumer`); dropped on stop so the consumer drains and
    /// exits.
    segments_tx: Arc<Mutex<Option<tokio::sync::mpsc::UnboundedSender<PendingSegment>>>>,
    /// `now_ms()` of the last frame the VAD classified as speech on the
    /// other side of the call.
    last_speech_ms: Arc<AtomicU64>,
}

impl LiveTranslateManager {
    pub fn new(app_handle: &AppHandle, transcription_manager: Arc<TranscriptionManager>) -> Self {
        Self {
            app_handle: app_handle.clone(),
            transcription_manager,
            active: Arc::new(AtomicBool::new(false)),
            mode: Arc::new(Mutex::new(LiveTranslateMode::Subtitles)),
            capture: Arc::new(Mutex::new(None)),
            me_capture: Arc::new(Mutex::new(None)),
            context: Arc::new(Mutex::new(Vec::new())),
            transcript_history: Arc::new(Mutex::new(Vec::new())),
            segmenter: Arc::new(Mutex::new(None)),
            me_segmenter: Arc::new(Mutex::new(None)),
            segments_tx: Arc::new(Mutex::new(None)),
            last_speech_ms: Arc::new(AtomicU64::new(0)),
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

        log::info!("Live session starting: mode={:?} source={:?}", mode, source);
        *self.mode.lock().unwrap() = mode;
        self.context.lock().unwrap().clear();
        self.transcript_history.lock().unwrap().clear();

        let (segments_tx, segments_rx) = tokio::sync::mpsc::unbounded_channel();
        *self.segments_tx.lock().unwrap() = Some(segments_tx);
        let consumer = self.clone();
        tauri::async_runtime::spawn(async move {
            consumer.run_segment_consumer(segments_rx).await;
        });

        let stream = match self.start_channel(Speaker::Them, source) {
            Ok(stream) => stream,
            Err(e) => {
                log::error!("Live session: audio capture failed: {e}");
                *self.segments_tx.lock().unwrap() = None;
                self.active.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };
        *self.capture.lock().unwrap() = Some(stream);

        // With system audio (a call), also listen to the user's own mic so
        // answers follow what they have already said. Their voice is never
        // subtitled or answered; it is only conversation context.
        if source == LiveTranslateSource::SystemAudio
            && get_settings(&self.app_handle).live_translate_include_me
        {
            match self.start_channel(Speaker::Me, LiveTranslateSource::Microphone) {
                Ok(stream) => *self.me_capture.lock().unwrap() = Some(stream),
                Err(e) => log::warn!("Live session: microphone (Me) capture unavailable: {e}"),
            }
        }

        log::info!("Live session capturing audio");
        let _ = self.app_handle.emit("live-translate-state", true);

        // Keep the STT/LLM connections warm for the whole session: each
        // segment is a separate request, and re-opening DNS + TLS per segment
        // was adding seconds of latency on slow resolvers.
        let keepalive = self.clone();
        tauri::async_runtime::spawn(async move {
            while keepalive.is_active() {
                let settings = get_settings(&keepalive.app_handle);
                if settings.cloud_stt_enabled {
                    crate::cloud_stt::prewarm(&settings);
                }
                if let Some((provider, _model, api_key)) = settings.resolve_live_llm_target() {
                    crate::llm_client::prewarm(&provider, &api_key);
                }
                tokio::time::sleep(std::time::Duration::from_secs(45)).await;
            }
        });
        if mode == LiveTranslateMode::Subtitles {
            super::overlay::create_live_subtitles_window(&self.app_handle);
        }
        if mode == LiveTranslateMode::Copilot
            || get_settings(&self.app_handle).live_translate_suggest_answers
        {
            super::overlay::create_answers_window(&self.app_handle);
        }
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
        let me_flushed = self
            .me_segmenter
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|s| s.flush());
        *self.capture.lock().unwrap() = None;
        *self.me_capture.lock().unwrap() = None;
        *self.segmenter.lock().unwrap() = None;
        *self.me_segmenter.lock().unwrap() = None;
        if let Some(segment) = flushed {
            self.process_segment(segment, false, Speaker::Them);
        }
        if let Some(segment) = me_flushed {
            self.process_segment(segment, false, Speaker::Me);
        }
        // Dropping the sender lets the consumer drain what is queued and exit.
        *self.segments_tx.lock().unwrap() = None;
        super::overlay::destroy_live_subtitles_window(&self.app_handle);
        super::overlay::destroy_answers_window(&self.app_handle);
        let _ = self.app_handle.emit("live-translate-state", false);
    }

    /// Starts one capture channel (VAD + segmenter) for `speaker`.
    fn start_channel(
        &self,
        speaker: Speaker,
        source: LiveTranslateSource,
    ) -> Result<CaptureStream, String> {
        let detector = build_vad(&self.app_handle)?;
        let slot = match speaker {
            Speaker::Them => &self.segmenter,
            Speaker::Me => &self.me_segmenter,
        };
        *slot.lock().unwrap() = Some(SpeechSegmenter::new(SegmenterConfig::default()));
        let segmenter = Arc::clone(slot);
        let vad = Arc::new(Mutex::new(detector));
        let last_them_speech = Arc::clone(&self.last_speech_ms);

        let manager = self.clone();
        let frame_samples = vad.lock().unwrap().frame_samples();
        let mut pending: Vec<f32> = Vec::with_capacity(frame_samples * 2);

        // Periodic level/VAD diagnostics (~every 3 s) so "nothing happens"
        // can be told apart from "no audio" vs "audio but no speech".
        let mut diag_frames = 0u32;
        let mut diag_speech = 0u32;
        let mut diag_peak = 0.0f32;
        let on_frame = move |frame: &[f32]| {
            pending.extend_from_slice(frame);
            while pending.len() >= frame_samples {
                let chunk: Vec<f32> = pending.drain(..frame_samples).collect();
                let mut is_speech = match vad.lock().unwrap().is_voice(&chunk) {
                    Ok(speech) => speech,
                    Err(e) => {
                        log::error!("Live subtitles VAD error: {e}");
                        false
                    }
                };
                match speaker {
                    Speaker::Them if is_speech => {
                        last_them_speech.store(now_ms(), Ordering::Relaxed);
                    }
                    Speaker::Me
                        if now_ms().saturating_sub(last_them_speech.load(Ordering::Relaxed))
                            < ECHO_GUARD_MS =>
                    {
                        is_speech = false;
                    }
                    _ => {}
                }
                diag_frames += 1;
                diag_speech += u32::from(is_speech);
                diag_peak = chunk.iter().fold(diag_peak, |m, v| m.max(v.abs()));
                if diag_frames >= 100 {
                    log::debug!(
                        "Live session audio ({}): peak={:.3} speech_frames={}/{}",
                        speaker.label(),
                        diag_peak,
                        diag_speech,
                        diag_frames
                    );
                    diag_frames = 0;
                    diag_speech = 0;
                    diag_peak = 0.0;
                }

                let closed_segment = segmenter.lock().unwrap().as_mut().and_then(|s| {
                    s.push(&chunk, is_speech)
                        .map(|segment| (segment, s.last_closed_by_cap()))
                });
                if let Some((segment, closed_by_cap)) = closed_segment {
                    manager.process_segment(segment, closed_by_cap, speaker);
                }
            }
        };

        match source {
            LiveTranslateSource::Microphone => capture::start_microphone_capture(on_frame),
            LiveTranslateSource::SystemAudio => capture::start_system_audio_capture(on_frame),
        }
    }

    /// Starts transcribing one closed speech segment right away (so
    /// transcriptions overlap) and queues it, in capture order, for
    /// `run_segment_consumer`. Never blocks the capture callback.
    fn process_segment(&self, segment: Vec<f32>, closed_by_cap: bool, speaker: Speaker) {
        let closed_at_ms = now_ms();
        let sample_rate = crate::audio_toolkit::constants::WHISPER_SAMPLE_RATE as usize;
        // Segments too short to be meaningful speech (VAD noise) are not
        // transcribed, but still queued: a pause that ends a turn must reach
        // the answer stage.
        let transcript = if segment.len() < sample_rate / 5 {
            tauri::async_runtime::spawn(async { None })
        } else {
            log::debug!(
                "Live session: {} segment closed ({:.1}s, cap={}), transcribing",
                speaker.label(),
                segment.len() as f32 / sample_rate as f32,
                closed_by_cap
            );
            let manager = self.clone();
            tauri::async_runtime::spawn_blocking(move || {
                let started = std::time::Instant::now();
                match manager.transcription_manager.transcribe(segment) {
                    Ok(text) => {
                        log::debug!("Live session: transcribed in {:?}", started.elapsed());
                        Some(text.trim().to_string()).filter(|t| !t.is_empty())
                    }
                    Err(e) => {
                        log::error!("Live subtitles transcription failed: {e}");
                        None
                    }
                }
            })
        };

        if let Some(tx) = self.segments_tx.lock().unwrap().as_ref() {
            let _ = tx.send(PendingSegment {
                transcript,
                closed_by_cap,
                closed_at_ms,
                speaker,
            });
        }
    }

    /// Consumes the session's segments in capture order: each transcript is
    /// subtitled immediately (translation runs concurrently), and forwarded
    /// to the answer stage.
    async fn run_segment_consumer(
        self,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<PendingSegment>,
    ) {
        let (answers_tx, answers_rx) = tokio::sync::mpsc::unbounded_channel();
        let answerer = self.clone();
        tauri::async_runtime::spawn(async move {
            answerer.run_answer_stage(answers_rx).await;
        });

        while let Some(segment) = rx.recv().await {
            let text = segment.transcript.await.ok().flatten().unwrap_or_default();
            let mode = *self.mode.lock().unwrap();

            // Only the other side is subtitled; the user's own voice is
            // context for answers.
            if mode == LiveTranslateMode::Subtitles
                && segment.speaker == Speaker::Them
                && !text.is_empty()
            {
                let manager = self.clone();
                let text = text.clone();
                tauri::async_runtime::spawn(async move {
                    manager.handle_subtitle_segment(text).await;
                });
            }

            let _ = answers_tx.send(UtteranceFragment {
                text,
                closed_by_cap: segment.closed_by_cap,
                closed_at_ms: segment.closed_at_ms,
                speaker: segment.speaker,
            });
        }
    }

    /// Joins fragments into utterances (a segment cut by the length cap is
    /// half a sentence) and, once the speaker has paused, hands the utterance
    /// to the question detector / answer generator.
    async fn run_answer_stage(
        self,
        mut rx: tokio::sync::mpsc::UnboundedReceiver<UtteranceFragment>,
    ) {
        let mut utterance: Vec<String> = Vec::new();
        // Whether the pending utterance was already held back once because
        // the VAD still heard speech. Background audio (music, a video) can
        // keep the VAD busy forever, so a question is held back at most once.
        let mut deferred = false;
        while let Some(fragment) = rx.recv().await {
            let answering = *self.mode.lock().unwrap() == LiveTranslateMode::Copilot
                || get_settings(&self.app_handle).live_translate_suggest_answers;
            if !answering {
                utterance.clear();
                continue;
            }
            if fragment.speaker == Speaker::Me {
                if !fragment.text.is_empty() {
                    // The user started talking: the interviewer's turn is
                    // over, answer what is pending right now.
                    if !utterance.is_empty() {
                        deferred = false;
                        self.dispatch_utterance(utterance.join(" "));
                        utterance.clear();
                    }
                    self.push_conversation(Speaker::Me, &fragment.text);
                }
                continue;
            }
            // A fragment the transcriber punctuated as a question ends the
            // turn right away, even if the speaker keeps talking or there is
            // background audio that never lets a pause close the segment.
            let asks = fragment.text.trim_end().ends_with('?');
            if !fragment.text.is_empty() {
                utterance.push(fragment.text);
                if utterance.len() > MAX_UTTERANCE_FRAGMENTS {
                    utterance.remove(0);
                }
            }
            if utterance.is_empty() || (fragment.closed_by_cap && !asks) {
                continue;
            }

            if !asks {
                let waited = now_ms().saturating_sub(fragment.closed_at_ms);
                if waited < TURN_GRACE_MS {
                    tokio::time::sleep(std::time::Duration::from_millis(TURN_GRACE_MS - waited))
                        .await;
                }
                // Still talking: keep the fragments and merge them with what
                // follows instead of answering half a question.
                if !deferred
                    && self.is_active()
                    && self.last_speech_ms.load(Ordering::Relaxed) > fragment.closed_at_ms
                {
                    log::debug!("Copilot: speaker still talking, holding the utterance");
                    deferred = true;
                    continue;
                }
            }
            deferred = false;

            self.dispatch_utterance(utterance.join(" "));
            utterance.clear();
        }

        // Session over: a question still being held back must not be lost.
        if !utterance.is_empty() {
            self.dispatch_utterance(utterance.join(" "));
        }
    }

    /// Records a finished interviewer utterance in the conversation and, in
    /// the background, answers it if it is a question. The conversation
    /// snapshot is taken here, in order, so later lines can't leak into it.
    fn dispatch_utterance(&self, text: String) {
        let context = self.transcript_history.lock().unwrap().clone();
        self.push_conversation(Speaker::Them, &text);
        let manager = self.clone();
        tauri::async_runtime::spawn(async move {
            manager.handle_copilot_utterance(text, context).await;
        });
    }

    fn push_conversation(&self, speaker: Speaker, text: &str) {
        let mut history = self.transcript_history.lock().unwrap();
        history.push(format!("{}: {}", speaker.label(), text.trim()));
        if history.len() > MAX_CONVERSATION_LINES {
            let excess = history.len() - MAX_CONVERSATION_LINES;
            history.drain(0..excess);
        }
    }

    async fn handle_subtitle_segment(&self, transcript: String) {
        // Detect per segment so a bilingual conversation flips direction
        // automatically (EN speaker -> ES subtitles, ES speaker -> EN).
        let source_lang = super::prompt::detect_language(&transcript);
        let target_lang = source_lang.other();

        // Show what was said immediately; the translation streams into the
        // same line right after.
        let id = NEXT_LINE_ID.fetch_add(1, Ordering::Relaxed);
        let line = |translation: String| LiveSubtitleLine {
            id,
            source_lang: source_lang.code().to_string(),
            original: transcript.clone(),
            translation,
        };
        let _ = line(String::new()).emit(&self.app_handle);
        let started = std::time::Instant::now();

        let app = self.app_handle.clone();
        let mut last_emit = std::time::Instant::now();
        let on_partial = |partial: &str| {
            if last_emit.elapsed() >= STREAM_EMIT_INTERVAL {
                last_emit = std::time::Instant::now();
                let _ = line(clean_model_text(partial)).emit(&app);
            }
        };

        let mut result = self.translate(&transcript, target_lang, on_partial).await;
        if matches!(&result, Err(e) if e.starts_with("Empty")) {
            // Reasoning models occasionally spend the whole budget thinking;
            // one retry almost always returns the translation.
            log::debug!("Live subtitles: empty translation, retrying once");
            result = self.translate(&transcript, target_lang, |_| {}).await;
        }
        let translation = match result {
            Ok(text) => clean_model_text(&text),
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

        log::debug!("Live subtitles: translated in {:?}", started.elapsed());
        let _ = line(translation).emit(&self.app_handle);
    }

    /// Runs the copilot's question detector on a finished utterance; on a
    /// match, streams an answer suggestion from the live LLM (grounded in
    /// the user's profile) to the answers panel, the live view and history.
    async fn handle_copilot_utterance(&self, transcript: String, recent_transcript: Vec<String>) {
        if !crate::copilot::is_question(&transcript) {
            return;
        }
        let started = std::time::Instant::now();

        let settings = get_settings(&self.app_handle);
        let Some((provider, model, api_key)) = settings.resolve_live_llm_target() else {
            log::error!("Copilot: no LLM configured for answer generation");
            return;
        };

        if self.is_active() {
            super::overlay::create_answers_window(&self.app_handle);
        }

        let profile = crate::copilot::get_profile(&self.app_handle);
        let prompt = crate::copilot::build_answer_prompt(
            &profile.text,
            &recent_transcript,
            &transcript,
            profile.answer_language,
        );
        let max_tokens = match profile.answer_language {
            crate::copilot::CopilotAnswerLanguage::Both => 420,
            _ => 220,
        };

        let id = NEXT_LINE_ID.fetch_add(1, Ordering::Relaxed);
        let line = |answer: String, done: bool| CopilotAnswerLine {
            id,
            question: transcript.clone(),
            answer,
            done,
        };
        let _ = line(String::new(), false).emit(&self.app_handle);

        let app = self.app_handle.clone();
        let mut first_word: Option<std::time::Duration> = None;
        let mut last_emit = std::time::Instant::now();
        let on_partial = |partial: &str| {
            first_word.get_or_insert_with(|| started.elapsed());
            if last_emit.elapsed() >= STREAM_EMIT_INTERVAL {
                last_emit = std::time::Instant::now();
                let _ = line(clean_model_text(partial), false).emit(&app);
            }
        };

        let result = crate::llm_client::stream_chat_completion(
            &provider,
            &api_key,
            &model,
            Some(prompt.system),
            prompt.user,
            Some(max_tokens),
            on_partial,
        )
        .await;

        let answer = match result {
            Ok(text) => clean_model_text(&text),
            Err(e) => {
                log::error!("Copilot: answer generation failed: {e}");
                String::new()
            }
        };
        log::debug!(
            "Copilot: answer first word {:?}, done in {:?}",
            first_word,
            started.elapsed()
        );
        let _ = line(answer.clone(), true).emit(&self.app_handle);
        if answer.is_empty() {
            return;
        }

        let timestamp = chrono::Utc::now().timestamp_millis();
        crate::copilot::append_entry(
            &self.app_handle,
            crate::copilot::CopilotAnswerEntry {
                id: timestamp.to_string(),
                question: transcript,
                answer,
                timestamp,
            },
        );
    }

    async fn translate<F>(
        &self,
        text: &str,
        target: SubtitleLanguage,
        on_partial: F,
    ) -> Result<String, String>
    where
        F: FnMut(&str) + Send,
    {
        let settings = get_settings(&self.app_handle);
        let (provider, model, api_key) = settings
            .resolve_live_llm_target()
            .ok_or_else(|| "No LLM configured for translation".to_string())?;

        let context_snapshot = self.context.lock().unwrap().clone();
        let prompt = build_translation_prompt(text, target, &context_snapshot);
        // Translations run about as long as the source; the cap only stops a
        // runaway reply.
        let max_tokens = (text.len() as u32 / 2).max(48) + 48;

        let translation = crate::llm_client::stream_chat_completion(
            &provider,
            &api_key,
            &model,
            None,
            prompt,
            Some(max_tokens),
            on_partial,
        )
        .await?;
        if translation.trim().is_empty() {
            return Err("Empty translation response".to_string());
        }
        Ok(translation)
    }
}

/// Strips a reasoning block and wrapping quotes some models add despite the
/// prompt, so partial and final text render cleanly.
fn clean_model_text(text: &str) -> String {
    let text = crate::actions::strip_invisible_chars(crate::actions::strip_think_block(text));
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .map(|t| t.strip_suffix('"').unwrap_or(t))
        .unwrap_or(text);
    text.trim().to_string()
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
