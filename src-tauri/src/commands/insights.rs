use crate::insights::{compute_insights, Insights};
use crate::managers::history::HistoryManager;
use chrono::Local;
use std::sync::Arc;
use tauri::{AppHandle, State};

#[tauri::command]
#[specta::specta]
pub async fn get_insights(
    _app: AppHandle,
    history_manager: State<'_, Arc<HistoryManager>>,
) -> Result<Insights, String> {
    let entries = history_manager
        .get_entries_for_insights()
        .map_err(|e| e.to_string())?;
    let today = Local::now().date_naive();
    Ok(compute_insights(&entries, today))
}
