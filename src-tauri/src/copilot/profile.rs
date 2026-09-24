//! Profile storage for the copilot: the user's pasted/imported CV/LinkedIn
//! text plus the answer language preference. Persisted in its own store file
//! (mirrors the pattern in `settings.rs`) so the main settings store stays
//! untouched.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

const PROFILE_STORE_PATH: &str = "copilot_profile.json";

/// Which language(s) the copilot answers in.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum CopilotAnswerLanguage {
    /// Answer in the same language as the detected question.
    #[default]
    Auto,
    En,
    Es,
    /// Answer in both English and Spanish.
    Both,
}

#[derive(Serialize, Deserialize, Debug, Clone, Type, Default)]
#[serde(default)]
pub struct CopilotProfile {
    pub text: String,
    pub answer_language: CopilotAnswerLanguage,
}

pub fn get_profile(app: &AppHandle) -> CopilotProfile {
    let store = app
        .store(crate::portable::store_path(PROFILE_STORE_PATH))
        .expect("Failed to initialize copilot profile store");

    store
        .get("profile")
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

pub fn write_profile(app: &AppHandle, profile: CopilotProfile) {
    let store = app
        .store(crate::portable::store_path(PROFILE_STORE_PATH))
        .expect("Failed to initialize copilot profile store");

    store.set("profile", serde_json::to_value(&profile).unwrap());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_empty_text_with_auto_language() {
        let profile = CopilotProfile::default();
        assert_eq!(profile.text, "");
        assert_eq!(profile.answer_language, CopilotAnswerLanguage::Auto);
    }
}
