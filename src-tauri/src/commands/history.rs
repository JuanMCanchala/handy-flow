use crate::actions::process_transcription_output;
use crate::managers::{
    history::{HistoryEntry, HistoryManager, LearningCandidate, PaginatedHistory},
    transcription::TranscriptionManager,
};
use crate::transcript_export::{self, ExportFormat, TranscriptSegment};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_history_entries(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    cursor: Option<i64>,
    limit: Option<usize>,
) -> Result<PaginatedHistory, String> {
    history_manager
        .get_history_entries(cursor, limit)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn toggle_history_entry_saved(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .toggle_saved_status(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_audio_file_path(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_name: String,
) -> Result<String, String> {
    let path = history_manager.get_audio_file_path(&file_name);
    path.to_str()
        .ok_or_else(|| "Invalid file path".to_string())
        .map(|s| s.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn delete_history_entry(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .delete_entry(id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_transcription(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    id: i64,
) -> Result<(), String> {
    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    let audio_path = history_manager.get_audio_file_path(&entry.file_name);
    let samples = crate::audio_toolkit::read_wav_samples(&audio_path)
        .map_err(|e| format!("Failed to load audio: {}", e))?;

    if samples.is_empty() {
        return Err("Recording has no audio samples".to_string());
    }

    transcription_manager.initiate_model_load();

    let tm = Arc::clone(&transcription_manager);
    let transcription = tauri::async_runtime::spawn_blocking(move || tm.transcribe(samples))
        .await
        .map_err(|e| format!("Transcription task panicked: {}", e))?
        .map_err(|e| e.to_string())?;

    if transcription.is_empty() {
        return Err("Recording contains no speech".to_string());
    }

    let processed = process_transcription_output(
        &app,
        &transcription,
        entry.post_process_requested,
        crate::style::classify_foreground_app(),
        None,
    )
    .await;
    history_manager
        .update_transcription(
            id,
            transcription,
            processed.post_processed_text,
            processed.post_process_prompt,
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn update_history_limit(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    limit: usize,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    settings.history_limit = limit;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn update_recording_retention_period(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    period: String,
) -> Result<(), String> {
    use crate::settings::RecordingRetentionPeriod;

    let retention_period = match period.as_str() {
        "never" => RecordingRetentionPeriod::Never,
        "preserve_limit" => RecordingRetentionPeriod::PreserveLimit,
        "days3" => RecordingRetentionPeriod::Days3,
        "weeks2" => RecordingRetentionPeriod::Weeks2,
        "months3" => RecordingRetentionPeriod::Months3,
        _ => return Err(format!("Invalid retention period: {}", period)),
    };

    let mut settings = crate::settings::get_settings(&app);
    settings.recording_retention_period = retention_period;
    crate::settings::write_settings(&app, settings);

    history_manager
        .cleanup_old_entries()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Import an audio/video file: decode, resample to 16 kHz mono, transcribe
/// in chunks (emitting `FileImportProgressEvent` between chunks), and save
/// the result as a normal history entry with per-chunk segments.
#[tauri::command]
#[specta::specta]
pub async fn import_audio_file(
    app: AppHandle,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    history_manager: State<'_, Arc<HistoryManager>>,
    file_import_manager: State<'_, Arc<crate::file_import::FileImportManager>>,
    diarization_model_manager: State<'_, Arc<crate::diarization::models::DiarizationModelManager>>,
    path: String,
) -> Result<HistoryEntry, String> {
    let tm = Arc::clone(&transcription_manager);
    let hm = Arc::clone(&history_manager);
    let fim = Arc::clone(&file_import_manager);
    let dmm = Arc::clone(&diarization_model_manager);
    let path = PathBuf::from(path);

    tauri::async_runtime::spawn_blocking(move || {
        crate::file_import::import_and_save(&app, &tm, &hm, &fim, &dmm, &path)
    })
    .await
    .map_err(|e| format!("Import task panicked: {}", e))?
    .map_err(|e| e.to_string())
}

/// Request cancellation of the currently running file import, if any.
#[tauri::command]
#[specta::specta]
pub async fn cancel_audio_import(
    file_import_manager: State<'_, Arc<crate::file_import::FileImportManager>>,
) -> Result<(), String> {
    file_import_manager.cancel();
    Ok(())
}

/// Format a history entry's transcript as txt/srt/vtt and write it to
/// `dest_path` (chosen by the frontend via the native save dialog).
///
/// Entries without stored segments (dictations, or imports whose engine
/// returned no timestamps) fall back to a single segment spanning the
/// entry's whole duration so export still produces valid SRT/VTT.
#[tauri::command]
#[specta::specta]
pub async fn export_transcript(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
    format: String,
    dest_path: String,
) -> Result<(), String> {
    let export_format = match format.as_str() {
        "txt" => ExportFormat::Txt,
        "srt" => ExportFormat::Srt,
        "vtt" => ExportFormat::Vtt,
        other => return Err(format!("Unsupported export format: {other}")),
    };

    let entry = history_manager
        .get_entry_by_id(id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", id))?;

    let mut segments = history_manager
        .get_segments(id)
        .await
        .map_err(|e| e.to_string())?;

    if segments.is_empty() {
        let text = entry
            .post_processed_text
            .clone()
            .unwrap_or(entry.transcription_text.clone());
        if !text.is_empty() {
            let end_ms = (entry.duration_seconds.unwrap_or(0.0) * 1000.0).round() as u64;
            segments.push(TranscriptSegment {
                start_ms: 0,
                end_ms,
                text,
                speaker: None,
            });
        }
    }

    let content = transcript_export::format_transcript(&segments, export_format);

    std::fs::write(&dest_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}

/// Save a user edit to a history entry's transcription text (from the
/// Home/History UI). Diffs the change to harvest self-learning dictionary
/// candidates as a side effect.
#[tauri::command]
#[specta::specta]
pub async fn edit_history_entry_text(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
    text: String,
) -> Result<HistoryEntry, String> {
    history_manager
        .edit_entry_text(id, text)
        .await
        .map_err(|e| e.to_string())
}

/// Minimum hit count a learning candidate must exceed before it's
/// surfaced as a suggestion on Home.
const LEARNING_CANDIDATE_MIN_HITS: i64 = 2;

#[tauri::command]
#[specta::specta]
pub async fn get_learning_candidates(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<Vec<LearningCandidate>, String> {
    history_manager
        .list_learning_candidates(LEARNING_CANDIDATE_MIN_HITS)
        .map_err(|e| e.to_string())
}

/// Adds a learning candidate's replacement word(s) to the existing
/// `custom_words` setting, then removes the candidate so it stops being
/// suggested.
#[tauri::command]
#[specta::specta]
pub async fn add_learning_candidate_to_dictionary(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
    phrase_to: String,
) -> Result<(), String> {
    let mut settings = crate::settings::get_settings(&app);
    if !settings.custom_words.iter().any(|w| w == &phrase_to) {
        settings.custom_words.push(phrase_to);
    }
    crate::settings::write_settings(&app, settings);

    history_manager
        .remove_learning_candidate(id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn dismiss_learning_candidate(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<(), String> {
    history_manager
        .dismiss_learning_candidate(id)
        .map_err(|e| e.to_string())
}

/// "Ask your history" search box: SQLite FTS5 search over transcription
/// text, returning entries with a link-back id/title/snippet.
#[tauri::command]
#[specta::specta]
pub async fn search_history(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    query: String,
) -> Result<Vec<crate::history_search::SearchResult>, String> {
    let Some(sanitized) = crate::history_search::sanitize_fts_query(&query) else {
        return Ok(Vec::new());
    };
    history_manager
        .search_history_fts(&sanitized, crate::history_search::ASK_TOP_K)
        .map_err(|e| e.to_string())
}

/// "Ask" button: runs the top-k FTS matches for `question` through the
/// configured LLM and returns the answer plus the matched entries it was
/// grounded in (for the frontend to render as source links).
#[tauri::command]
#[specta::specta]
pub async fn ask_history(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    question: String,
) -> Result<AskHistoryResponse, String> {
    let matches = match crate::history_search::sanitize_fts_query(&question) {
        Some(sanitized) => history_manager
            .search_history_fts(&sanitized, crate::history_search::ASK_TOP_K)
            .map_err(|e| e.to_string())?,
        None => Vec::new(),
    };

    let settings = crate::settings::get_settings(&app);
    let (provider, model, api_key) = settings
        .resolve_llm_target()
        .ok_or_else(|| "No AI provider/model configured".to_string())?;

    let prompt = crate::history_search::build_ask_prompt(&question, &matches);

    let answer = crate::llm_client::send_chat_completion(&provider, api_key, &model, prompt, true)
        .await?
        .ok_or_else(|| "The AI provider returned an empty response".to_string())?;

    Ok(AskHistoryResponse { answer, matches })
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, specta::Type)]
pub struct AskHistoryResponse {
    pub answer: String,
    pub matches: Vec<crate::history_search::SearchResult>,
}
/// Frontend-facing view of one transcript segment. Mirrors
/// [`transcript_export::TranscriptSegment`], which stays a pure formatter
/// type (no Tauri/serde deps) so this command layer owns the conversion.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, specta::Type)]
pub struct TranscriptSegmentView {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub speaker: Option<String>,
}

impl From<TranscriptSegment> for TranscriptSegmentView {
    fn from(s: TranscriptSegment) -> Self {
        Self {
            start_ms: s.start_ms,
            end_ms: s.end_ms,
            text: s.text,
            speaker: s.speaker,
        }
    }
}

/// Fetch a history entry's per-segment transcript (with any diarized speaker
/// labels) for display in the transcript view. Empty for dictations and for
/// imports whose engine returned no timestamps.
#[tauri::command]
#[specta::specta]
pub async fn get_transcript_segments(
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
) -> Result<Vec<TranscriptSegmentView>, String> {
    let segments = history_manager
        .get_segments(id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(segments
        .into_iter()
        .map(TranscriptSegmentView::from)
        .collect())
}

/// Rename a diarized speaker label ("Speaker 1" -> a real name) across every
/// segment of one history entry that currently has `old_label`.
#[tauri::command]
#[specta::specta]
pub async fn rename_speaker(
    history_manager: State<'_, Arc<HistoryManager>>,
    id: i64,
    old_label: String,
    new_label: String,
) -> Result<(), String> {
    history_manager
        .rename_speaker(id, &old_label, &new_label)
        .await
        .map_err(|e| e.to_string())
}
