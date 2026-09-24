//! Keeps Voxa's windows out of screen shares and recordings (Meet, Zoom,
//! Teams, OBS...). Uses the OS capture-exclusion flag (Windows 10 2004+
//! `WDA_EXCLUDEFROMCAPTURE`, macOS `NSWindowSharingNone`); the windows stay
//! fully visible on the user's own screen.

use crate::settings::get_settings;
use tauri::{AppHandle, Manager, WebviewWindow};

/// Applies the current `hide_from_screen_share` setting to one window.
pub fn protect_window(app: &AppHandle, window: &WebviewWindow) {
    let enabled = get_settings(app).hide_from_screen_share;
    if let Err(e) = window.set_content_protected(enabled) {
        log::warn!(
            "Could not set capture protection on '{}': {}",
            window.label(),
            e
        );
    }
}

/// Re-applies the setting to every open window (after the toggle changes).
pub fn apply_to_all_windows(app: &AppHandle) {
    for window in app.webview_windows().values() {
        protect_window(app, window);
    }
}
