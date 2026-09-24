//! Windows-only glue for the Flow bar: UI Automation focus-changed events
//! decide whether the currently focused control is an editable text field,
//! and a fullscreen check hides the bar while a fullscreen app is in front.
//! Pure decision logic (is this a text field?) lives in `flow_bar.rs`.

use crate::flow_bar::{is_text_field, FocusedElementInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use windows::core::Interface;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomation2, IUIAutomationElement, IUIAutomationTextPattern,
    IUIAutomationValuePattern, UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_TextPatternId,
    UIA_ValuePatternId,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowLongPtrW, GetWindowRect, GWL_EXSTYLE, WS_EX_NOACTIVATE,
};

/// Whether the focused control (as last reported by UI Automation) is an
/// editable text field. Read by the overlay to decide idle-pill visibility
/// under `FlowBarVisibility::TextFields`.
static FOCUSED_IS_TEXT_FIELD: AtomicBool = AtomicBool::new(false);

/// Focus changes are handed to a worker thread: the UIA callback must return
/// immediately, and touching Tauri windows from it blocked the UIA thread
/// (window calls wait on the main thread), which stopped further events.
static FOCUS_CHANGES: std::sync::OnceLock<std::sync::mpsc::Sender<bool>> =
    std::sync::OnceLock::new();

pub fn focused_is_text_field() -> bool {
    FOCUSED_IS_TEXT_FIELD.load(Ordering::Relaxed)
}

/// Sets `WS_EX_NOACTIVATE` on the overlay window so clicking it never steals
/// keyboard focus from the field the user is dictating into.
pub fn make_non_activating(hwnd: HWND) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            current | (WS_EX_NOACTIVATE.0 as isize),
        );
    }
}

/// True when the foreground window covers its entire monitor (a fullscreen
/// app), used to hide the Flow bar so it doesn't sit on top of e.g. a
/// fullscreen video or game.
pub fn is_foreground_window_fullscreen() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }

        let mut window_rect = RECT::default();
        if GetWindowRect(hwnd, &mut window_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
            return false;
        }

        window_rect == monitor_info.rcMonitor
    }
}

/// Classifies a focused element; `None` for our own overlay windows.
fn evaluate_element(
    element: &IUIAutomationElement,
    overlay_hwnds: &[isize],
) -> Option<(bool, i32)> {
    if let Ok(hwnd) = unsafe { element.CurrentNativeWindowHandle() } {
        if overlay_hwnds.contains(&(hwnd.0 as isize)) {
            return None;
        }
    }

    let control_type = unsafe { element.CurrentControlType() }
        .unwrap_or_default()
        .0;
    // Typed pattern objects instead of the Is*PatternAvailable properties:
    // a missing pattern is simply a failed query, and ValuePattern's
    // read-only flag comes back as a BOOL rather than a VARIANT.
    let has_editable_value = unsafe { element.GetCurrentPattern(UIA_ValuePatternId) }
        .ok()
        .and_then(|pattern| pattern.cast::<IUIAutomationValuePattern>().ok())
        .and_then(|value| unsafe { value.CurrentIsReadOnly() }.ok())
        .map(|read_only| !read_only.as_bool())
        .unwrap_or(false);
    let has_text_pattern = unsafe { element.GetCurrentPattern(UIA_TextPatternId) }
        .ok()
        .and_then(|pattern| pattern.cast::<IUIAutomationTextPattern>().ok())
        .is_some();

    let info = FocusedElementInfo {
        is_edit_control: control_type == UIA_EditControlTypeId.0,
        is_document_control: control_type == UIA_DocumentControlTypeId.0,
        has_editable_value,
        has_text_pattern,
    };
    Some((is_text_field(info), control_type))
}

/// Stores the new state and, when it changed, hands it to the worker.
fn record_focus(is_text_field: bool, control_type: i32) {
    let previous = FOCUSED_IS_TEXT_FIELD.swap(is_text_field, Ordering::Relaxed);
    if previous != is_text_field {
        log::debug!(
            "flow_bar: focus changed (control_type={}, text_field={})",
            control_type,
            is_text_field
        );
        if let Some(tx) = FOCUS_CHANGES.get() {
            let _ = tx.send(is_text_field);
        }
    }
}

/// Starts listening for UI Automation focus-changed events on a dedicated
/// thread (UIA event handlers run on the thread that registered them and
/// that thread must keep a message loop / COM apartment alive for the
/// lifetime of the subscription).
pub fn start_focus_tracking(app_handle: AppHandle, overlay_hwnds: Vec<isize>) {
    // Worker: applies visibility off the UIA thread, debounced so rapid
    // focus hops (tabbing, menus) settle into one show/hide.
    let (tx, rx) = std::sync::mpsc::channel::<bool>();
    let _ = FOCUS_CHANGES.set(tx);
    let worker_app = app_handle.clone();
    std::thread::spawn(move || {
        while let Ok(mut latest) = rx.recv() {
            while let Ok(next) = rx.recv_timeout(std::time::Duration::from_millis(80)) {
                latest = next;
            }
            crate::overlay::on_focus_text_field_changed(&worker_app, latest);
        }
    });

    std::thread::spawn(move || unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};

        // UIA event handlers should live in the MTA: STA handlers can
        // deadlock with cross-apartment calls made while handling an event.
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            log::error!("flow_bar: failed to initialize COM for UIA focus tracking");
            return;
        }

        let automation: IUIAutomation =
            match CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
                Ok(automation) => automation,
                Err(error) => {
                    log::error!("flow_bar: failed to create IUIAutomation: {error}");
                    return;
                }
            };

        // An unresponsive or just-closed app can block UIA calls for many
        // seconds; cap them so one bad window can't stall focus tracking.
        if let Ok(automation2) = automation.cast::<IUIAutomation2>() {
            let _ = automation2.SetConnectionTimeout(500);
            let _ = automation2.SetTransactionTimeout(500);
        }

        // Poll the focused element instead of subscribing to UIA focus
        // events: event delivery proved unreliable here (handlers stopped
        // firing), while a 250 ms poll is deterministic and costs ~nothing.
        log::info!("flow_bar: polling UIA focus every 250 ms");
        loop {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let element = match automation.GetFocusedElement() {
                Ok(element) => element,
                Err(error) => {
                    log::debug!("flow_bar: GetFocusedElement failed: {error}");
                    continue;
                }
            };
            let evaluated = evaluate_element(&element, &overlay_hwnds);
            if evaluated.is_none() {
                log::trace!("flow_bar: focus is on our own overlay");
            }
            if let Some((is_text_field, control_type)) = evaluated {
                record_focus(is_text_field, control_type);
            }
        }
    });
}
