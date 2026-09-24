//! Persisted history of copilot answer suggestions, shown in the Copilot
//! section. Stored the same way as the profile (own store file, mirrors
//! `settings.rs`'s `app.store` pattern) since it's small, append-mostly data
//! rather than the paginated/searchable transcription history in
//! `managers/history.rs`.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const HISTORY_STORE_PATH: &str = "copilot_history.json";
/// Keep the history list from growing unbounded across a long call.
const MAX_HISTORY_ENTRIES: usize = 200;

#[derive(Serialize, Deserialize, Debug, Clone, Type)]
pub struct CopilotAnswerEntry {
    pub id: String,
    pub question: String,
    pub answer: String,
    /// Milliseconds since epoch.
    pub timestamp: i64,
}

pub fn get_history(app: &AppHandle) -> Vec<CopilotAnswerEntry> {
    let store = app
        .store(crate::portable::store_path(HISTORY_STORE_PATH))
        .expect("Failed to initialize copilot history store");

    store
        .get("entries")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

/// Appends a new entry (newest last on disk; callers display newest first).
/// Trims the oldest entries once the list exceeds `MAX_HISTORY_ENTRIES`.
pub fn append_entry(app: &AppHandle, entry: CopilotAnswerEntry) {
    let store = app
        .store(crate::portable::store_path(HISTORY_STORE_PATH))
        .expect("Failed to initialize copilot history store");

    let mut entries = get_history(app);
    entries.push(entry);
    if entries.len() > MAX_HISTORY_ENTRIES {
        let excess = entries.len() - MAX_HISTORY_ENTRIES;
        entries.drain(0..excess);
    }

    store.set("entries", serde_json::to_value(&entries).unwrap());
}

pub fn clear_history(app: &AppHandle) {
    let store = app
        .store(crate::portable::store_path(HISTORY_STORE_PATH))
        .expect("Failed to initialize copilot history store");

    store.set(
        "entries",
        serde_json::to_value(Vec::<CopilotAnswerEntry>::new()).unwrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_oldest_entries_past_the_cap() {
        let mut entries: Vec<CopilotAnswerEntry> = (0..MAX_HISTORY_ENTRIES)
            .map(|i| CopilotAnswerEntry {
                id: i.to_string(),
                question: "q".to_string(),
                answer: "a".to_string(),
                timestamp: i as i64,
            })
            .collect();
        entries.push(CopilotAnswerEntry {
            id: "new".to_string(),
            question: "q".to_string(),
            answer: "a".to_string(),
            timestamp: MAX_HISTORY_ENTRIES as i64,
        });

        if entries.len() > MAX_HISTORY_ENTRIES {
            let excess = entries.len() - MAX_HISTORY_ENTRIES;
            entries.drain(0..excess);
        }

        assert_eq!(entries.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(entries.last().unwrap().id, "new");
        assert_eq!(entries.first().unwrap().id, "1");
    }
}
