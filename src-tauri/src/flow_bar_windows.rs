//! Windows-only glue for the Flow bar: UI Automation focus-changed events
//! decide whether the currently focused control is an editable text field,
//! and a fullscreen check hides the bar while a fullscreen app is in front.
//! Pure decision logic (is this a text field?) lives in `flow_bar.rs`.

use crate::flow_bar::{is_text_field, FocusedElementInfo};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;
use windows::core::{implement, Interface, Ref};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationFocusChangedEventHandler,
    IUIAutomationFocusChangedEventHandler_Impl, IUIAutomationTextPattern,
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

#[implement(IUIAutomationFocusChangedEventHandler)]
struct FocusChangedHandler {
    overlay_hwnds: Vec<isize>,
    app_handle: AppHandle,
}

impl IUIAutomationFocusChangedEventHandler_Impl for FocusChangedHandler_Impl {
    fn HandleFocusChangedEvent(
        &self,
        sender: Ref<'_, IUIAutomationElement>,
    ) -> windows::core::Result<()> {
        let Ok(sender) = sender.ok() else {
            return Ok(());
        };

        // Never treat our own overlay/window elements as text fields.
        if let Ok(hwnd) = unsafe { sender.CurrentNativeWindowHandle() } {
            if self.overlay_hwnds.contains(&(hwnd.0 as isize)) {
                return Ok(());
            }
        }

        let control_type = unsafe { sender.CurrentControlType() }.unwrap_or_default().0;
        let is_edit_control = control_type == UIA_EditControlTypeId.0;
        let is_document_control = control_type == UIA_DocumentControlTypeId.0;

        // Typed pattern objects instead of the Is*PatternAvailable properties:
        // a missing pattern is simply a failed query, and ValuePattern's
        // read-only flag comes back as a BOOL rather than a VARIANT.
        let has_editable_value = unsafe { sender.GetCurrentPattern(UIA_ValuePatternId) }
            .ok()
            .and_then(|pattern| pattern.cast::<IUIAutomationValuePattern>().ok())
            .and_then(|value| unsafe { value.CurrentIsReadOnly() }.ok())
            .map(|read_only| !read_only.as_bool())
            .unwrap_or(false);
        let has_text_pattern = unsafe { sender.GetCurrentPattern(UIA_TextPatternId) }
            .ok()
            .and_then(|pattern| pattern.cast::<IUIAutomationTextPattern>().ok())
            .is_some();

        let info = FocusedElementInfo {
            is_edit_control,
            is_document_control,
            has_editable_value_or_text_pattern: has_editable_value || has_text_pattern,
        };

        let is_text_field = is_text_field(info);
        FOCUSED_IS_TEXT_FIELD.store(is_text_field, Ordering::Relaxed);
        crate::overlay::on_focus_text_field_changed(&self.app_handle, is_text_field);

        Ok(())
    }
}

/// Starts listening for UI Automation focus-changed events on a dedicated
/// thread (UIA event handlers run on the thread that registered them and
/// that thread must keep a message loop / COM apartment alive for the
/// lifetime of the subscription).
pub fn start_focus_tracking(app_handle: AppHandle, overlay_hwnds: Vec<isize>) {
    std::thread::spawn(move || unsafe {
        use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};

        if CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_err() {
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

        let handler: IUIAutomationFocusChangedEventHandler = FocusChangedHandler {
            overlay_hwnds,
            app_handle,
        }
        .into();

        if let Err(error) = automation.AddFocusChangedEventHandler(None, &handler) {
            log::error!("flow_bar: failed to register focus-changed handler: {error}");
            return;
        }

        // Keep this thread alive with a simple message loop so the COM
        // apartment (and thus the event subscription) stays valid.
        let mut msg = windows::Win32::UI::WindowsAndMessaging::MSG::default();
        loop {
            let result = windows::Win32::UI::WindowsAndMessaging::GetMessageW(&mut msg, None, 0, 0);
            if !result.as_bool() {
                break;
            }
            let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&msg);
            windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&msg);
        }
    });
}
