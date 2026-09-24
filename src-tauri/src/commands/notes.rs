//! Tauri commands for note templates (CRUD) and "Generate notes": turning a
//! history entry's transcript into structured markdown notes via the
//! configured LLM (`settings::resolve_llm_target` + `llm_client`), chunking
//! long transcripts map-reduce style (see `crate::notes`).

use crate::managers::history::HistoryManager;
use crate::notes::{self, NoteTemplate};
use crate::settings::{get_settings, write_settings};
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub fn add_note_template(
    app: AppHandle,
    name: String,
    prompt: String,
) -> Result<NoteTemplate, String> {
    let mut settings = get_settings(&app);

    let id = format!("note_template_{}", chrono::Utc::now().timestamp_millis());
    let template = NoteTemplate {
        id: id.clone(),
        name,
        prompt,
    };

    settings.note_templates.push(template.clone());
    write_settings(&app, settings);

    Ok(template)
}

#[tauri::command]
#[specta::specta]
pub fn update_note_template(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
) -> Result<(), String> {
    let mut settings = get_settings(&app);

    if let Some(existing) = settings.note_templates.iter_mut().find(|t| t.id == id) {
        existing.name = name;
        existing.prompt = prompt;
        write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Note template with id '{}' not found", id))
    }
}

#[tauri::command]
#[specta::specta]
pub fn delete_note_template(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    let original_len = settings.note_templates.len();
    settings.note_templates.retain(|t| t.id != id);

    if settings.note_templates.len() == original_len {
        return Err(format!("Note template with id '{}' not found", id));
    }

    write_settings(&app, settings);
    Ok(())
}

/// Generates markdown notes for a history entry using the given template,
/// storing the result (and its parsed action-item checklist) with the entry.
/// Long transcripts are summarized map-reduce style so they fit the model's
/// context regardless of length.
#[tauri::command]
#[specta::specta]
pub async fn generate_notes(
    app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
    entry_id: i64,
    template_id: String,
) -> Result<String, String> {
    let settings = get_settings(&app);
    let template = settings
        .note_templates
        .iter()
        .find(|t| t.id == template_id)
        .cloned()
        .ok_or_else(|| format!("Note template with id '{}' not found", template_id))?;

    let entry = history_manager
        .get_entry_by_id(entry_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("History entry {} not found", entry_id))?;

    let transcript = entry
        .post_processed_text
        .clone()
        .unwrap_or(entry.transcription_text.clone());
    if transcript.trim().is_empty() {
        return Err("This entry has no transcript to generate notes from".to_string());
    }

    let (provider, model, api_key) = settings
        .resolve_llm_target()
        .ok_or_else(|| "No AI provider/model configured".to_string())?;
    let disable_reasoning = crate::llm_client::should_disable_reasoning(&provider);

    let chunks = notes::chunk_transcript(&transcript);
    let source_text = if chunks.len() <= 1 {
        transcript.clone()
    } else {
        let total_chunks = chunks.len();
        let mut summaries = Vec::with_capacity(total_chunks);
        for (index, chunk) in chunks.iter().enumerate() {
            let map_prompt = notes::build_map_prompt(chunk, index, total_chunks);
            let summary = crate::llm_client::send_chat_completion(
                &provider,
                api_key.clone(),
                &model,
                map_prompt,
                disable_reasoning,
            )
            .await?
            .ok_or_else(|| "The AI provider returned an empty response".to_string())?;
            summaries.push(summary);
        }
        summaries.join("\n\n")
    };

    let final_prompt = notes::build_notes_prompt(&template.prompt, &source_text);
    let markdown = crate::llm_client::send_chat_completion(
        &provider,
        api_key,
        &model,
        final_prompt,
        disable_reasoning,
    )
    .await?
    .ok_or_else(|| "The AI provider returned an empty response".to_string())?;
    let markdown = markdown.trim().to_string();

    history_manager
        .save_notes(entry_id, &markdown, &template_id)
        .map_err(|e| e.to_string())?;

    Ok(markdown)
}

/// Toggles the checked state of one action item (by its 0-based position in
/// the notes' action-item checklist) and persists the change.
#[tauri::command]
#[specta::specta]
pub async fn toggle_note_action_item(
    history_manager: State<'_, Arc<HistoryManager>>,
    entry_id: i64,
    item_index: usize,
) -> Result<String, String> {
    history_manager
        .toggle_action_item(entry_id, item_index)
        .map_err(|e| e.to_string())
}
