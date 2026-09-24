//! Transparent, always-on-top, click-through subtitle overlay window for live
//! subtitles. Modeled on the recording overlay in `overlay.rs`, but simpler:
//! one fixed-size window docked at the bottom of the monitor under the
//! cursor, with no state machine — content is pushed via the
//! `live-subtitle-line` event emitted from `live_translate::pipeline`.
//!
//! Unlike the recording overlay, this window has no instant-show latency
//! requirement (a live-subtitles/copilot session going through the whole
//! capture/VAD/transcription/LLM pipeline before anything reaches the
//! overlay dwarfs a WebView2 window creation), so it is created when a
//! session starts and destroyed when it stops instead of staying resident
//! for the lifetime of the app.

use tauri::{AppHandle, Emitter, Manager, WebviewWindowBuilder};

const SUBTITLES_WIDTH: f64 = 900.0;
const SUBTITLES_HEIGHT: f64 = 140.0;
const SUBTITLES_BOTTOM_OFFSET: f64 = 60.0;

fn calculate_position(app_handle: &AppHandle) -> Option<(f64, f64)> {
    let monitor = app_handle.primary_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let monitor_x = monitor.position().x as f64 / scale;
    let monitor_y = monitor.position().y as f64 / scale;
    let monitor_width = monitor.size().width as f64 / scale;
    let monitor_height = monitor.size().height as f64 / scale;

    let x = monitor_x + (monitor_width - SUBTITLES_WIDTH) / 2.0;
    let y = monitor_y + monitor_height - SUBTITLES_HEIGHT - SUBTITLES_BOTTOM_OFFSET;
    Some((x, y))
}

/// Creates the live subtitles window and shows it. Called when a live
/// subtitles/copilot session starts (`LiveTranslateManager::start_with_mode`);
/// paired with `destroy_live_subtitles_window` on session stop. Safe to call
/// when the window already exists (e.g. a stop/start race) — it is
/// repositioned and shown rather than duplicated.
pub fn create_live_subtitles_window(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("live_subtitles") {
        if let Some((x, y)) = calculate_position(app_handle) {
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
        let _ = window.show();
        return;
    }

    let (x, y) = match calculate_position(app_handle) {
        Some(pos) => pos,
        None => (100.0, 100.0),
    };

    let mut builder = WebviewWindowBuilder::new(
        app_handle,
        "live_subtitles",
        tauri::WebviewUrl::App("src/live-subtitles/index.html".into()),
    )
    .title("Live Subtitles")
    .resizable(false)
    .inner_size(SUBTITLES_WIDTH, SUBTITLES_HEIGHT)
    .position(x, y)
    .shadow(false)
    .maximizable(false)
    .minimizable(false)
    .closable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .transparent(true)
    .focusable(false)
    .focused(false)
    .visible(false);

    if let Some(data_dir) = crate::portable::data_dir() {
        builder = builder.data_directory(data_dir.join("webview"));
    }

    match builder.build() {
        Ok(window) => {
            let _ = window.set_ignore_cursor_events(true);
            // Best-effort: hides the overlay from screen capture/share where
            // the OS supports it (Windows 10 2004+, macOS). No-op elsewhere.
            let _ = window.set_content_protected(true);
            let _ = window.show();
            log::debug!("Live subtitles window created and shown");
        }
        Err(e) => {
            log::debug!("Failed to create live subtitles window: {}", e);
        }
    }
}

/// Destroys the live subtitles window, releasing its WebView2/WebKit
/// renderer. Called when a live subtitles/copilot session stops
/// (`LiveTranslateManager::stop`). The window is recreated from scratch by
/// `create_live_subtitles_window` the next time a session starts.
pub fn destroy_live_subtitles_window(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("live_subtitles") {
        let _ = window.emit("live-subtitles-hide", ());
        if let Err(e) = window.destroy() {
            log::error!("Failed to destroy live subtitles window: {}", e);
        }
    }
}
