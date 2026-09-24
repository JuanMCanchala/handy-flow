use crate::modes::{mode_binding_id, Mode, ModeOutputFormat};
use crate::settings::{get_settings, write_settings, ShortcutBinding};
use crate::shortcut;
use tauri::AppHandle;

/// Registers (or re-registers) the dynamic `mode:<id>` shortcut binding for a
/// mode's hotkey, creating the binding entry in settings if missing.
fn sync_mode_binding(app: &AppHandle, mode: &Mode) {
    let Some(hotkey) = mode.hotkey.clone() else {
        return;
    };
    let binding_id = mode_binding_id(&mode.id);
    let mut settings = get_settings(app);

    let existing = settings.bindings.get(&binding_id).cloned();
    if let Some(existing) = &existing {
        let _ = shortcut::unregister_shortcut(app, existing.clone());
    }

    let binding = ShortcutBinding {
        id: binding_id.clone(),
        name: mode.name.clone(),
        description: format!("Records and applies the '{}' mode.", mode.name),
        default_binding: hotkey.clone(),
        current_binding: hotkey,
    };

    if let Err(e) = shortcut::register_shortcut(app, binding.clone()) {
        log::warn!("Failed to register mode binding '{}': {}", binding_id, e);
    }

    settings.bindings.insert(binding_id, binding);
    write_settings(app, settings);
}

/// Unregisters and removes the dynamic `mode:<id>` binding, if any.
fn remove_mode_binding(app: &AppHandle, mode_id: &str) {
    let binding_id = mode_binding_id(mode_id);
    let mut settings = get_settings(app);
    if let Some(binding) = settings.bindings.remove(&binding_id) {
        let _ = shortcut::unregister_shortcut(app, binding);
        write_settings(app, settings);
    }
}

#[tauri::command]
#[specta::specta]
pub fn add_mode(
    app: AppHandle,
    name: String,
    prompt: String,
    provider_id: Option<String>,
    model: Option<String>,
    language: Option<String>,
    output_format: ModeOutputFormat,
    hotkey: Option<String>,
) -> Result<Mode, String> {
    let mut settings = get_settings(&app);

    let id = format!("mode_{}", chrono::Utc::now().timestamp_millis());
    let new_mode = Mode {
        id: id.clone(),
        name,
        prompt,
        provider_id,
        model,
        language,
        output_format,
        hotkey,
    };

    settings.modes.push(new_mode.clone());
    write_settings(&app, settings);

    sync_mode_binding(&app, &new_mode);

    Ok(new_mode)
}

#[tauri::command]
#[specta::specta]
pub fn update_mode(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
    provider_id: Option<String>,
    model: Option<String>,
    language: Option<String>,
    output_format: ModeOutputFormat,
    hotkey: Option<String>,
) -> Result<(), String> {
    let mut settings = get_settings(&app);

    let Some(existing) = settings.modes.iter_mut().find(|m| m.id == id) else {
        return Err(format!("Mode with id '{}' not found", id));
    };

    let had_hotkey = existing.hotkey.is_some();
    existing.name = name;
    existing.prompt = prompt;
    existing.provider_id = provider_id;
    existing.model = model;
    existing.language = language;
    existing.output_format = output_format;
    existing.hotkey = hotkey;
    let updated = existing.clone();

    write_settings(&app, settings);

    if updated.hotkey.is_some() {
        sync_mode_binding(&app, &updated);
    } else if had_hotkey {
        remove_mode_binding(&app, &id);
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn delete_mode(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    let original_len = settings.modes.len();
    settings.modes.retain(|m| m.id != id);

    if settings.modes.len() == original_len {
        return Err(format!("Mode with id '{}' not found", id));
    }

    if settings.active_mode_id.as_deref() == Some(id.as_str()) {
        settings.active_mode_id = None;
    }

    write_settings(&app, settings);
    remove_mode_binding(&app, &id);

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn set_active_mode(app: AppHandle, id: Option<String>) -> Result<(), String> {
    let mut settings = get_settings(&app);

    if let Some(id) = &id {
        if !settings.modes.iter().any(|m| &m.id == id) {
            return Err(format!("Mode with id '{}' not found", id));
        }
    }

    settings.active_mode_id = id;
    write_settings(&app, settings);
    crate::tray::update_tray_menu(&app);

    Ok(())
}
