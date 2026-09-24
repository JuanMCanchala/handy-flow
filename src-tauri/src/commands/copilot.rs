//! Tauri commands for the profile copilot: profile storage, file import,
//! start/stop control on the shared `live_translate` pipeline, and answer
//! history — mirroring the shape of `commands/live_translate.rs`.

use crate::copilot::{self, CopilotAnswerEntry, CopilotAnswerLanguage, CopilotProfile};
use crate::live_translate::LiveTranslateManager;
use crate::settings;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
#[specta::specta]
pub fn get_copilot_profile(app: AppHandle) -> Result<CopilotProfile, String> {
    Ok(copilot::get_profile(&app))
}

#[tauri::command]
#[specta::specta]
pub fn set_copilot_profile_text(app: AppHandle, text: String) -> Result<(), String> {
    let mut profile = copilot::get_profile(&app);
    profile.text = text;
    copilot::write_profile(&app, profile);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_copilot_answer_language(
    app: AppHandle,
    language: CopilotAnswerLanguage,
) -> Result<(), String> {
    let mut profile = copilot::get_profile(&app);
    profile.answer_language = language;
    copilot::write_profile(&app, profile);
    Ok(())
}

/// Extracts text from a profile file (.txt/.md/.pdf/.docx) at `path` without
/// modifying the stored profile; the frontend merges/replaces the text and
/// calls `set_copilot_profile_text` itself.
#[tauri::command]
#[specta::specta]
pub fn import_copilot_profile_file(path: String) -> Result<String, String> {
    copilot::extract_text(&PathBuf::from(path))
}

#[tauri::command]
#[specta::specta]
pub fn get_copilot_history(app: AppHandle) -> Result<Vec<CopilotAnswerEntry>, String> {
    Ok(copilot::get_history(&app))
}

#[tauri::command]
#[specta::specta]
pub fn clear_copilot_history(app: AppHandle) -> Result<(), String> {
    copilot::clear_history(&app);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn is_copilot_active(app: AppHandle) -> Result<bool, String> {
    let manager = app.state::<Arc<LiveTranslateManager>>();
    Ok(manager.is_copilot_active())
}

#[tauri::command]
#[specta::specta]
pub fn toggle_copilot(app: AppHandle) -> Result<(), String> {
    let manager = app.state::<Arc<LiveTranslateManager>>().inner().clone();
    if manager.is_copilot_active() {
        manager.stop();
        Ok(())
    } else {
        let source = settings::get_settings(&app).live_translate_source;
        manager.start_copilot(source)
    }
}
