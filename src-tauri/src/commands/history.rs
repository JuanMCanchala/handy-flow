use crate::actions::process_transcription_output;
use crate::managers::{
    history::{HistoryEntry, HistoryManager, PaginatedHistory},
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
    path: String,
) -> Result<HistoryEntry, String> {
    let tm = Arc::clone(&transcription_manager);
    let hm = Arc::clone(&history_manager);
    let fim = Arc::clone(&file_import_manager);
    let path = PathBuf::from(path);

    tauri::async_runtime::spawn_blocking(move || {
        crate::file_import::import_and_save(&app, &tm, &hm, &fim, &path)
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
            });
        }
    }

    let content = transcript_export::format_transcript(&segments, export_format);

    std::fs::write(&dest_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}
