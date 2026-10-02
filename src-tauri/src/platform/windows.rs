use std::mem::{size_of, zeroed};

use tauri::WebviewWindow;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateRectRgn, GetMonitorInfoW, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromWindow,
    SetWindowRgn,
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

/// Limits the window to the visible notch shape: outside this rectangle the
/// window neither paints nor receives clicks, which go to the app below.
/// Changing the region instead of resizing the window avoids the WebView2
/// repaint flash a resize causes. Coordinates are physical, window-relative.
pub fn set_hit_area(window: &WebviewWindow, width: f64, height: f64) -> tauri::Result<()> {
    let hwnd = window.hwnd()?.0;
    let scale = window.scale_factor()?;
    let window_width = window.inner_size()?.width as i32;
    let width = (width * scale).round() as i32;
    let height = (height * scale).round() as i32;
    let left = (window_width - width) / 2;
    unsafe {
        // The system owns the region once it is set: no DeleteObject.
        let region = CreateRectRgn(left, 0, left + width, height);
        SetWindowRgn(hwnd, region, 1);
    }
    Ok(())
}

// --- Cursor resistance (DF-0004): a low-level mouse hook on its own thread ---

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorPos, GetMessageW, HC_ACTION, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT,
    PostThreadMessageW, SetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL,
    WM_MOUSEMOVE, WM_QUIT,
};

use crate::resistance::{Rect, Resistance, Verdict};

/// Notch rectangle in physical screen pixels, read by the hook on every move.
static NOTCH_RECT: [AtomicI32; 4] = [const { AtomicI32::new(0) }; 4];
/// Whether the hook should be installed.
static WANTED: AtomicBool = AtomicBool::new(false);
/// Thread id of the running hook thread, 0 when none.
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
/// A hook thread has been spawned and has not exited yet.
static RUNNING: AtomicBool = AtomicBool::new(false);

thread_local! {
    static STATE: RefCell<Resistance> = RefCell::new(Resistance::default());
}

pub fn set_resistance_rect(rect: Rect) {
    for (slot, value) in NOTCH_RECT
        .iter()
        .zip([rect.left, rect.top, rect.right, rect.bottom])
    {
        slot.store(value, Ordering::Relaxed);
    }
}

fn load_rect() -> Rect {
    let [left, top, right, bottom] = NOTCH_RECT.each_ref().map(|v| v.load(Ordering::Relaxed));
    Rect {
        left,
        top,
        right,
        bottom,
    }
}

/// Installs or removes the mouse hook. Idempotent; called every second by the
/// watcher. Removing it while the notch is hidden (fullscreen games…) means
/// zero cost on mouse input there.
pub fn set_cursor_resistance(active: bool) {
    WANTED.store(active, Ordering::SeqCst);
    if active {
        if !RUNNING.swap(true, Ordering::SeqCst) {
            std::thread::spawn(run_hook_thread);
        }
    } else {
        let thread = HOOK_THREAD.load(Ordering::SeqCst);
        if thread != 0 {
            unsafe { PostThreadMessageW(thread, WM_QUIT, 0, 0) };
        }
    }
}

fn run_hook_thread() {
    unsafe {
        let hook = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(mouse_hook),
            GetModuleHandleW(std::ptr::null()),
            0,
        );
        if hook.is_null() {
            log::error!("cannot install the cursor resistance hook");
        } else {
            HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
            // Turned off while we were starting: leave right away.
            if WANTED.load(Ordering::SeqCst) {
                let mut msg: MSG = zeroed();
                // Low-level hooks are called through this thread's message loop.
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
            }
            HOOK_THREAD.store(0, Ordering::SeqCst);
            UnhookWindowsHookEx(hook);
        }
    }
    RUNNING.store(false, Ordering::SeqCst);
    // Turned back on while we were shutting down.
    if WANTED.load(Ordering::SeqCst) {
        set_cursor_resistance(true);
    }
}

/// Runs for every mouse event of the whole system: keep it to a few arithmetic
/// operations, no allocation, no lock, no logging.
unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wparam == WM_MOUSEMOVE as WPARAM {
        let info = unsafe { &*(lparam as *const MSLLHOOKSTRUCT) };
        let mut from = POINT { x: 0, y: 0 };
        if info.flags & LLMHF_INJECTED == 0 && unsafe { GetCursorPos(&mut from) } != 0 {
            let verdict = STATE.with_borrow_mut(|state| {
                state.on_move(
                    load_rect(),
                    (from.x, from.y),
                    (info.pt.x, info.pt.y),
                    info.time,
                )
            });
            if let Verdict::Hold(x, y) = verdict {
                unsafe { SetCursorPos(x, y) };
                return 1;
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}
