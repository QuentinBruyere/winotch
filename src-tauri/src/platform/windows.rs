use std::mem::{size_of, zeroed};

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromWindow,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    MONITORINFOF_PRIMARY, SetWindowLongPtrW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

/// Turns the notch into a tool window that never takes focus:
/// hidden from Alt+Tab (WS_EX_TOOLWINDOW) and not activated on click (WS_EX_NOACTIVATE).
/// Tauri rewrites the extended style whenever it shows, hides or re-layers the
/// window, so this must be re-applied after such changes; it is a no-op when
/// the style is already right.
pub fn prepare_overlay(window: &WebviewWindow) -> tauri::Result<()> {
    let hwnd = window.hwnd()?.0;
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let wanted = (current & !(WS_EX_APPWINDOW as isize))
            | WS_EX_TOOLWINDOW as isize
            | WS_EX_NOACTIVATE as isize;
        if wanted != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, wanted);
        }
    }
    Ok(())
}

/// True when the foreground window covers the whole primary monitor without a
/// title bar: games, videos, presentations, browsers in F11 mode.
/// Maximized windows keep their caption, so they do not count.
pub fn fullscreen_on_primary() -> bool {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() || is_desktop(hwnd) {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
        if monitor.is_null() {
            return false;
        }
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 || info.dwFlags & MONITORINFOF_PRIMARY == 0 {
            return false;
        }
        let mut rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return false;
        }
        let screen = info.rcMonitor;
        let covers = rect.left <= screen.left
            && rect.top <= screen.top
            && rect.right >= screen.right
            && rect.bottom >= screen.bottom;
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        covers && style & WS_CAPTION != WS_CAPTION
    }
}

/// The desktop and taskbar can be "foreground" and cover the screen.
unsafe fn is_desktop(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    let mut class = [0u16; 64];
    let len = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
    let class = String::from_utf16_lossy(&class[..len.max(0) as usize]);
    matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd")
}
