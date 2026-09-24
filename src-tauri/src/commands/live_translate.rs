//! Tauri commands for the live subtitles feature: persisted source setting
//! and start/stop control, mirroring the shape of other setting setters in
//! `commands/audio.rs` and the CLI-driven toggles in `signal_handle.rs`.

use crate::live_translate::{overlay, LiveTranslateManager};
use crate::settings::{self, LiveTranslateSource, SubtitlesPosition};
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
    let manager = app.state::<Arc<LiveTranslateManager>>();
    if manager.is_active() && !manager.is_copilot_active() {
        if enabled {
            overlay::create_answers_window(&app);
        } else {
            overlay::destroy_answers_window(&app);
        }
    }
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_hide_from_screen_share_setting(app: AppHandle, enabled: bool) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.hide_from_screen_share = enabled;
    settings::write_settings(&app, current_settings);
    crate::privacy::apply_to_all_windows(&app);
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

#[tauri::command]
#[specta::specta]
pub fn change_live_subtitles_position_setting(
    app: AppHandle,
    position: SubtitlesPosition,
) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.live_subtitles_position = position;
    settings::write_settings(&app, current_settings);
    overlay::reposition(&app);
    Ok(())
}

/// Docks the answers panel back in the top-right corner.
#[tauri::command]
#[specta::specta]
pub fn reset_copilot_answers_position(app: AppHandle) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.copilot_answers_custom_position = None;
    settings::write_settings(&app, current_settings);
    overlay::reposition(&app);
    Ok(())
}

/// Makes the subtitles and answers overlays draggable (shown even with no
/// session running); turning it off saves where they were left.
#[tauri::command]
#[specta::specta]
pub fn set_live_overlays_move_mode(app: AppHandle, enabled: bool) -> Result<(), String> {
    let manager = app.state::<Arc<LiveTranslateManager>>();
    let answers_on =
        manager.is_copilot_active() || settings::get_settings(&app).live_translate_suggest_answers;
    overlay::set_move_mode(&app, enabled, manager.is_active(), answers_on);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn get_live_overlays_move_mode() -> Result<bool, String> {
    Ok(overlay::is_move_mode())
}

/// Provider/model used live (translation + answers). Empty strings fall back
/// to the post-processing provider/model.
#[tauri::command]
#[specta::specta]
pub fn change_live_llm_setting(
    app: AppHandle,
    provider_id: String,
    model: String,
) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    if !provider_id.is_empty() && current_settings.post_process_provider(&provider_id).is_none() {
        return Err(format!("Unknown provider: {provider_id}"));
    }
    current_settings.live_llm_provider_id = provider_id;
    current_settings.live_llm_model = model.trim().to_string();
    settings::write_settings(&app, current_settings);
    Ok(())
}
