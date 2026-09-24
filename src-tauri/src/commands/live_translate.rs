//! Tauri commands for the live subtitles feature: persisted source setting
//! and start/stop control, mirroring the shape of other setting setters in
//! `commands/audio.rs` and the CLI-driven toggles in `signal_handle.rs`.

use crate::live_translate::LiveTranslateManager;
use crate::settings::{self, LiveTranslateSource};
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[tauri::command]
#[specta::specta]
pub fn change_live_translate_source_setting(
    app: AppHandle,
    source: LiveTranslateSource,
) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.live_translate_source = source;
    settings::write_settings(&app, current_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_live_translate_suggest_answers_setting(
    app: AppHandle,
    enabled: bool,
) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.live_translate_suggest_answers = enabled;
    settings::write_settings(&app, current_settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn is_live_translate_active(app: AppHandle) -> Result<bool, String> {
    let manager = app.state::<Arc<LiveTranslateManager>>();
    Ok(manager.is_active())
}

#[tauri::command]
#[specta::specta]
pub fn toggle_live_translate(app: AppHandle) -> Result<(), String> {
    let manager = app.state::<Arc<LiveTranslateManager>>().inner().clone();
    if manager.is_active() {
        manager.stop();
        Ok(())
    } else {
        let source = settings::get_settings(&app).live_translate_source;
        manager.start(source)
    }
}
