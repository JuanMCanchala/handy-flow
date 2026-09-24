//! Transparent, always-on-top, click-through overlay windows for live
//! sessions, modeled on the recording overlay in `overlay.rs`:
//!
//! - `live_subtitles`: the translated subtitles, docked at the bottom or top
//!   of the screen under the cursor, or wherever the user dragged it
//!   (`settings.live_subtitles_position`).
//! - `copilot_answers`: the suggested answers panel, docked in the top-right
//!   corner (or the user's saved position).
//!
//! Content is pushed via the `live-subtitle-line` / `copilot-answer-line`
//! events emitted from `live_translate::pipeline`. Neither window has an
//! instant-show latency requirement (the capture/VAD/transcription/LLM
//! pipeline dwarfs a WebView2 window creation), so both are created when a
//! session starts and destroyed when it stops instead of staying resident.
//!
//! "Move mode" (`set_move_mode`) makes both windows interactive so the user
//! can drag them; leaving it saves where they ended up.

use crate::settings::{get_settings, write_settings, SubtitlesPosition, WindowPoint};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WebviewWindowBuilder};

pub const SUBTITLES_LABEL: &str = "live_subtitles";
pub const ANSWERS_LABEL: &str = "copilot_answers";

const SUBTITLES_WIDTH: f64 = 900.0;
const SUBTITLES_HEIGHT: f64 = 170.0;
const SUBTITLES_EDGE_OFFSET: f64 = 60.0;

const ANSWERS_WIDTH: f64 = 440.0;
const ANSWERS_HEIGHT: f64 = 560.0;
const ANSWERS_MARGIN: f64 = 20.0;

static MOVE_MODE: AtomicBool = AtomicBool::new(false);

/// Logical bounds (x, y, width, height) of the monitor under the cursor,
/// falling back to the primary monitor.
fn active_monitor_bounds(app: &AppHandle) -> Option<(f64, f64, f64, f64)> {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    Some((
        monitor.position().x as f64 / scale,
        monitor.position().y as f64 / scale,
        monitor.size().width as f64 / scale,
        monitor.size().height as f64 / scale,
    ))
}

/// True when a saved point still lands on a connected monitor (monitors can
/// be unplugged between sessions).
fn point_is_on_screen(app: &AppHandle, point: WindowPoint) -> bool {
    app.available_monitors()
        .map(|monitors| {
            monitors.iter().any(|m| {
                let scale = m.scale_factor();
                let x = m.position().x as f64 / scale;
                let y = m.position().y as f64 / scale;
                let w = m.size().width as f64 / scale;
                let h = m.size().height as f64 / scale;
                point.x >= x - 1.0 && point.x < x + w && point.y >= y - 1.0 && point.y < y + h
            })
        })
        .unwrap_or(false)
}

fn subtitles_position(app: &AppHandle) -> (f64, f64) {
    let settings = get_settings(app);
    if settings.live_subtitles_position == SubtitlesPosition::Custom {
        if let Some(point) = settings
            .live_subtitles_custom_position
            .filter(|p| point_is_on_screen(app, *p))
        {
            return (point.x, point.y);
        }
    }
    let Some((mx, my, mw, mh)) = active_monitor_bounds(app) else {
        return (100.0, 100.0);
    };
    let x = mx + (mw - SUBTITLES_WIDTH) / 2.0;
    let y = match settings.live_subtitles_position {
        SubtitlesPosition::Top => my + SUBTITLES_EDGE_OFFSET,
        _ => my + mh - SUBTITLES_HEIGHT - SUBTITLES_EDGE_OFFSET,
    };
    (x, y)
}

fn answers_position(app: &AppHandle) -> (f64, f64) {
    if let Some(point) = get_settings(app)
        .copilot_answers_custom_position
        .filter(|p| point_is_on_screen(app, *p))
    {
        return (point.x, point.y);
    }
    let Some((mx, my, mw, _)) = active_monitor_bounds(app) else {
        return (100.0, 100.0);
    };
    (mx + mw - ANSWERS_WIDTH - ANSWERS_MARGIN, my + ANSWERS_MARGIN)
}

/// Creates (or repositions and shows) one overlay window.
fn ensure_window(
    app: &AppHandle,
    label: &str,
    url: &str,
    title: &str,
    (width, height): (f64, f64),
    (x, y): (f64, f64),
) {
    if let Some(window) = app.get_webview_window(label) {
        if !MOVE_MODE.load(Ordering::SeqCst) {
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
        }
        let _ = window.show();
        return;
    }

    let mut builder = WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App(url.into()))
        .title(title)
        .resizable(false)
        .inner_size(width, height)
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
            let _ = window.set_ignore_cursor_events(!MOVE_MODE.load(Ordering::SeqCst));
            // Best-effort: hides the overlay from screen capture/share where
            // the OS supports it (Windows 10 2004+, macOS). No-op elsewhere.
            crate::privacy::protect_window(app, &window);
            let _ = window.show();
            log::debug!("{label} window created and shown");
        }
        Err(e) => log::warn!("Failed to create {label} window: {e}"),
    }
}

fn destroy_window(app: &AppHandle, label: &str) {
    if MOVE_MODE.load(Ordering::SeqCst) {
        // The user is placing the overlays; they go away when move mode ends.
        return;
    }
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.emit("live-subtitles-hide", ());
        if let Err(e) = window.destroy() {
            log::error!("Failed to destroy {label} window: {e}");
        }
    }
}

/// Creates the live subtitles window and shows it. Called when a live
/// subtitles session starts; paired with `destroy_live_subtitles_window`.
pub fn create_live_subtitles_window(app: &AppHandle) {
    let position = subtitles_position(app);
    ensure_window(
        app,
        SUBTITLES_LABEL,
        "src/live-subtitles/index.html",
        "Live Subtitles",
        (SUBTITLES_WIDTH, SUBTITLES_HEIGHT),
        position,
    );
}

pub fn destroy_live_subtitles_window(app: &AppHandle) {
    destroy_window(app, SUBTITLES_LABEL);
}

/// Creates the suggested-answers panel. Called when a session with answer
/// suggestions starts (and lazily on the first detected question, in case
/// suggestions were switched on mid-session).
pub fn create_answers_window(app: &AppHandle) {
    let position = answers_position(app);
    ensure_window(
        app,
        ANSWERS_LABEL,
        "src/live-subtitles/answers.html",
        "Suggested Answers",
        (ANSWERS_WIDTH, ANSWERS_HEIGHT),
        position,
    );
}

pub fn destroy_answers_window(app: &AppHandle) {
    destroy_window(app, ANSWERS_LABEL);
}

pub fn is_move_mode() -> bool {
    MOVE_MODE.load(Ordering::SeqCst)
}

fn logical_top_left(window: &WebviewWindow) -> Option<WindowPoint> {
    let position = window.outer_position().ok()?;
    let scale = window.scale_factor().ok()?;
    Some(WindowPoint {
        x: position.x as f64 / scale,
        y: position.y as f64 / scale,
    })
}

/// Enters or leaves move mode. Entering shows both overlays (even with no
/// session running) and makes them draggable; leaving saves their positions
/// and returns them to click-through, or closes them if no session needs
/// them.
pub fn set_move_mode(app: &AppHandle, enabled: bool, session_active: bool, answers_on: bool) {
    let was = MOVE_MODE.swap(enabled, Ordering::SeqCst);
    if enabled {
        create_live_subtitles_window(app);
        create_answers_window(app);
        for label in [SUBTITLES_LABEL, ANSWERS_LABEL] {
            if let Some(window) = app.get_webview_window(label) {
                let _ = window.set_ignore_cursor_events(false);
            }
        }
        let _ = app.emit("live-overlays-move-mode", true);
        return;
    }
    if !was {
        return;
    }

    let mut settings = get_settings(app);
    if let Some(point) = app
        .get_webview_window(SUBTITLES_LABEL)
        .and_then(|w| logical_top_left(&w))
    {
        settings.live_subtitles_position = SubtitlesPosition::Custom;
        settings.live_subtitles_custom_position = Some(point);
    }
    if let Some(point) = app
        .get_webview_window(ANSWERS_LABEL)
        .and_then(|w| logical_top_left(&w))
    {
        settings.copilot_answers_custom_position = Some(point);
    }
    write_settings(app, settings);
    let _ = app.emit("live-overlays-move-mode", false);

    for label in [SUBTITLES_LABEL, ANSWERS_LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.set_ignore_cursor_events(true);
        }
    }
    if !session_active {
        destroy_live_subtitles_window(app);
        destroy_answers_window(app);
    } else if !answers_on {
        destroy_answers_window(app);
    }
}

/// Re-applies the saved/preset positions to open overlays (after the user
/// picks a preset or resets a position).
pub fn reposition(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SUBTITLES_LABEL) {
        let (x, y) = subtitles_position(app);
        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
    }
    if let Some(window) = app.get_webview_window(ANSWERS_LABEL) {
        let (x, y) = answers_position(app);
        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition { x, y }));
    }
}
