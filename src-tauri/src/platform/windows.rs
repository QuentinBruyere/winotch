use tauri::WebviewWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

/// Turns the notch into a tool window that never takes focus:
/// hidden from Alt+Tab (WS_EX_TOOLWINDOW) and not activated on click (WS_EX_NOACTIVATE).
pub fn prepare_overlay(window: &WebviewWindow) -> tauri::Result<()> {
    let hwnd = window.hwnd()?.0;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let style = (style & !(WS_EX_APPWINDOW as isize))
            | WS_EX_TOOLWINDOW as isize
            | WS_EX_NOACTIVATE as isize;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
    }
    Ok(())
}
