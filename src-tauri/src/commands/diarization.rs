//! Tauri commands for speaker diarization: persisted settings and on-demand
//! model download/status, mirroring the shape of `commands/live_translate.rs`
//! and `commands/cloud_stt.rs`.

use crate::diarization::models::{
    DiarizationModelInfo, DiarizationModelKind, DiarizationModelManager,
};
use crate::settings;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub fn change_diarization_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.diarization_enabled = enabled;
    settings::write_settings(&app, current_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_diarization_cluster_threshold_setting(
    app: AppHandle,
    threshold: f32,
) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.diarization_cluster_threshold = threshold;
    settings::write_settings(&app, current_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_diarization_models_status(
    manager: State<'_, Arc<DiarizationModelManager>>,
) -> Result<Vec<DiarizationModelInfo>, String> {
    Ok(manager.get_models_status())
}

#[tauri::command]
#[specta::specta]
pub async fn download_diarization_model(
    manager: State<'_, Arc<DiarizationModelManager>>,
    kind: DiarizationModelKind,
) -> Result<(), String> {
    manager.download(kind).await.map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn delete_diarization_model(
    manager: State<'_, Arc<DiarizationModelManager>>,
    kind: DiarizationModelKind,
) -> Result<(), String> {
    manager.delete(kind).map_err(|e| e.to_string())
}
