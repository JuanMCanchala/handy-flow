//! Tauri commands for cloud speech-to-text settings. Mirrors the
//! `post_process_*` setter commands in `shortcut/mod.rs`.

use crate::settings::{self, CloudSttProvider};
use tauri::AppHandle;

fn validate_provider_exists(
    settings: &settings::AppSettings,
    provider_id: &str,
) -> Result<(), String> {
    if !settings
        .cloud_stt_providers
        .iter()
        .any(|provider| provider.id == provider_id)
    {
        return Err(format!("Cloud STT provider '{}' not found", provider_id));
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_cloud_stt_enabled_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut app_settings = settings::get_settings(&app);
    app_settings.cloud_stt_enabled = enabled;
    settings::write_settings(&app, app_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_cloud_stt_provider(app: AppHandle, provider_id: String) -> Result<(), String> {
    let mut app_settings = settings::get_settings(&app);
    validate_provider_exists(&app_settings, &provider_id)?;
    app_settings.cloud_stt_provider_id = provider_id;
    settings::write_settings(&app, app_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_cloud_stt_base_url_setting(
    app: AppHandle,
    provider_id: String,
    base_url: String,
) -> Result<(), String> {
    let mut app_settings = settings::get_settings(&app);
    let label = app_settings
        .cloud_stt_provider(&provider_id)
        .map(|provider| provider.label.clone())
        .ok_or_else(|| format!("Cloud STT provider '{}' not found", provider_id))?;

    let provider: &mut CloudSttProvider = app_settings
        .cloud_stt_provider_mut(&provider_id)
        .expect("Provider looked up above must exist");

    if !provider.allow_base_url_edit {
        return Err(format!(
            "Provider '{}' does not allow editing the base URL",
            label
        ));
    }

    provider.base_url = base_url;
    settings::write_settings(&app, app_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_cloud_stt_api_key_setting(
    app: AppHandle,
    provider_id: String,
    api_key: String,
) -> Result<(), String> {
    let mut app_settings = settings::get_settings(&app);
    validate_provider_exists(&app_settings, &provider_id)?;
    app_settings.cloud_stt_api_keys.insert(provider_id, api_key);
    settings::write_settings(&app, app_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_cloud_stt_model_setting(
    app: AppHandle,
    provider_id: String,
    model: String,
) -> Result<(), String> {
    let mut app_settings = settings::get_settings(&app);
    validate_provider_exists(&app_settings, &provider_id)?;
    app_settings.cloud_stt_models.insert(provider_id, model);
    settings::write_settings(&app, app_settings);
    Ok(())
}
