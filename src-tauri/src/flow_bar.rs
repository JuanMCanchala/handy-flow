//! Pure logic for the Flow bar: the persistent idle pill that mirrors
//! Wispr Flow's bottom bar. Platform-specific glue (Windows UI Automation
//! focus tracking, overlay show/hide) lives in `overlay.rs`; this module
//! only holds decisions that are easy to unit test in isolation.

use crate::settings;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// Sidebar section id the Scratchpad button navigates to.
const SCRATCHPAD_SECTION: &str = "scratchpad";

/// When to show the idle Flow bar pill.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Type, Default)]
#[serde(rename_all = "snake_case")]
pub enum FlowBarVisibility {
    /// Only while keyboard focus is on an editable text control (Windows only;
    /// other platforms fall back to `Always`).
    #[default]
    TextFields,
    Always,
    Never,
}

/// Minimal shape of a UI Automation element's control-type/pattern info,
/// enough to decide whether it should be treated as a text field. Kept
/// separate from any `windows` crate types so it can be unit tested without
/// a live UIA connection.
#[derive(Debug, Clone, Copy, Default)]
pub struct FocusedElementInfo {
    pub is_edit_control: bool,
    pub is_document_control: bool,
    /// Element supports ValuePattern or TextPattern and is not read-only.
    pub has_editable_value_or_text_pattern: bool,
}

/// Decides whether a focused UI Automation element should count as a text
/// field for `FlowBarVisibility::TextFields`. Edit/Document control types
/// are always accepted; otherwise fall back to a non-read-only ValuePattern
/// or TextPattern, which covers rich text editors that don't report as
/// Edit/Document (e.g. many Electron/web based editors).
pub fn is_text_field(info: FocusedElementInfo) -> bool {
    info.is_edit_control || info.is_document_control || info.has_editable_value_or_text_pattern
}

/// Cycles `selected_language` between `es -> en -> auto -> es`, matching the
/// language chip's click behavior. Unrecognized values reset to `es`.
pub fn cycle_language(current: &str) -> &'static str {
    match current {
        "es" => "en",
        "en" => "auto",
        "auto" => "es",
        _ => "es",
    }
}

/// Persists the Flow bar visibility setting.
#[tauri::command]
#[specta::specta]
pub fn change_flow_bar_visibility_setting(
    app: AppHandle,
    visibility: FlowBarVisibility,
) -> Result<(), String> {
    let mut current_settings = settings::get_settings(&app);
    current_settings.flow_bar_visibility = visibility;
    settings::write_settings(&app, current_settings);
    crate::overlay::update_flow_bar_visibility(&app);
    Ok(())
}

/// Cycles `selected_language` from the Flow bar's language chip and returns
/// the newly selected value so the chip can relabel itself.
#[tauri::command]
#[specta::specta]
pub fn flow_bar_cycle_language(app: AppHandle) -> Result<String, String> {
    let mut current_settings = settings::get_settings(&app);
    let next = cycle_language(&current_settings.selected_language).to_string();
    current_settings.selected_language = next.clone();
    settings::write_settings(&app, current_settings);
    Ok(next)
}

/// Starts/stops the normal `transcribe` recording, exactly like the hotkey.
#[tauri::command]
#[specta::specta]
pub fn flow_bar_toggle_transcribe(app: AppHandle) {
    crate::signal_handle::send_transcription_input(&app, "transcribe", "flow_bar");
}

/// Grows/shrinks the Flow bar window around the hover toolbar.
#[tauri::command]
#[specta::specta]
pub fn flow_bar_set_expanded(app: AppHandle, expanded: bool) {
    crate::overlay::set_flow_bar_expanded(&app, expanded);
}

/// Section the main window should open on once it is available. Set by
/// `flow_bar_open_scratchpad` and consumed by the frontend on mount, which is
/// what covers the Windows case where the main window is destroyed while
/// parked in the tray and has no listener to receive an event yet.
static PENDING_MAIN_SECTION: Mutex<Option<String>> = Mutex::new(None);

/// Opens the main window on the Scratchpad section.
#[tauri::command]
#[specta::specta]
pub fn flow_bar_open_scratchpad(app: AppHandle) -> Result<(), String> {
    if let Ok(mut pending) = PENDING_MAIN_SECTION.lock() {
        *pending = Some(SCRATCHPAD_SECTION.to_string());
    }
    crate::show_main_window(&app);
    let _ = app.emit_to("main", "navigate-to-section", SCRATCHPAD_SECTION);
    Ok(())
}

/// Returns (and clears) the section the main window should navigate to.
#[tauri::command]
#[specta::specta]
pub fn take_pending_main_section() -> Option<String> {
    PENDING_MAIN_SECTION.lock().ok().and_then(|mut p| p.take())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_control_is_a_text_field() {
        let info = FocusedElementInfo {
            is_edit_control: true,
            ..Default::default()
        };
        assert!(is_text_field(info));
    }

    #[test]
    fn document_control_is_a_text_field() {
        let info = FocusedElementInfo {
            is_document_control: true,
            ..Default::default()
        };
        assert!(is_text_field(info));
    }

    #[test]
    fn editable_value_or_text_pattern_is_a_text_field() {
        let info = FocusedElementInfo {
            has_editable_value_or_text_pattern: true,
            ..Default::default()
        };
        assert!(is_text_field(info));
    }

    #[test]
    fn plain_control_is_not_a_text_field() {
        assert!(!is_text_field(FocusedElementInfo::default()));
    }

    #[test]
    fn cycle_language_goes_es_en_auto_es() {
        assert_eq!(cycle_language("es"), "en");
        assert_eq!(cycle_language("en"), "auto");
        assert_eq!(cycle_language("auto"), "es");
    }

    #[test]
    fn cycle_language_resets_unknown_values_to_es() {
        assert_eq!(cycle_language("fr"), "es");
        assert_eq!(cycle_language(""), "es");
    }
}
