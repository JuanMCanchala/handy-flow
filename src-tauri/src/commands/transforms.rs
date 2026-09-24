use crate::settings::{get_settings, write_settings};
use crate::transforms::Transform;
use crate::translate::TranslationTarget;
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub fn add_transform(app: AppHandle, name: String, prompt: String) -> Result<Transform, String> {
    let mut settings = get_settings(&app);

    let id = format!("transform_{}", chrono::Utc::now().timestamp_millis());
    let new_transform = Transform {
        id: id.clone(),
        name,
        prompt,
    };

    settings.transforms.push(new_transform.clone());
    write_settings(&app, settings);

    Ok(new_transform)
}

#[tauri::command]
#[specta::specta]
pub fn update_transform(
    app: AppHandle,
    id: String,
    name: String,
    prompt: String,
) -> Result<(), String> {
    let mut settings = get_settings(&app);

    if let Some(existing) = settings.transforms.iter_mut().find(|t| t.id == id) {
        existing.name = name;
        existing.prompt = prompt;
        write_settings(&app, settings);
        Ok(())
    } else {
        Err(format!("Transform with id '{}' not found", id))
    }
}

#[tauri::command]
#[specta::specta]
pub fn delete_transform(app: AppHandle, id: String) -> Result<(), String> {
    let mut settings = get_settings(&app);

    let original_len = settings.transforms.len();
    settings.transforms.retain(|t| t.id != id);

    if settings.transforms.len() == original_len {
        return Err(format!("Transform with id '{}' not found", id));
    }

    write_settings(&app, settings);
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn change_translation_target_setting(
    app: AppHandle,
    target: TranslationTarget,
) -> Result<(), String> {
    let mut settings = get_settings(&app);
    settings.translation_target = target;
    write_settings(&app, settings);
    Ok(())
}
