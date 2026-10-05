use std::mem::{size_of, zeroed};

use tauri::WebviewWindow;

use crate::placement::Edge;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::Graphics::Gdi::{
    CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, GetMonitorInfoW,
    MONITOR_DEFAULTTONEAREST, MONITOR_DEFAULTTONULL, MONITORINFO, MonitorFromPoint,
    MonitorFromWindow, RGN_OR, SetWindowRgn,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWL_STYLE, GetClassNameW, GetForegroundWindow, GetWindowLongPtrW, GetWindowRect,
    SetWindowLongPtrW, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
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

/// True when the foreground window covers the whole monitor the notch is on,
/// without a title bar: games, videos, presentations, browsers in F11 mode.
/// Maximized windows keep their caption, so they do not count.
pub fn fullscreen_on_notch_screen(notch: &WebviewWindow) -> bool {
    let Ok(notch) = notch.hwnd() else {
        return false;
    };
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() || is_shell_window(hwnd) {
            return false;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONULL);
        if monitor.is_null() || monitor != MonitorFromWindow(notch.0, MONITOR_DEFAULTTONEAREST) {
            return false;
        }
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
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

/// Shell windows that can be foreground and cover the whole screen without
/// being a fullscreen app: the notch must stay visible over them.
const SHELL_CLASSES: &[&str] = &[
    // Desktop and taskbar.
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    // Alt+Tab and Win+Tab: Windows 10, then Windows 11.
    "MultitaskingViewFrame",
    "XamlExplorerHostIslandWindow",
];

unsafe fn is_shell_window(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    SHELL_CLASSES.contains(&unsafe { class_name(hwnd) }.as_str())
}

unsafe fn class_name(hwnd: windows_sys::Win32::Foundation::HWND) -> String {
    let mut class = [0u16; 128];
    let len = unsafe { GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32) };
    String::from_utf16_lossy(&class[..len.max(0) as usize])
}

/// Window class of the foreground window, logged when it hides the notch so
/// that other shell overlays wrongly taken for fullscreen apps can be spotted.
pub fn foreground_class() -> String {
    unsafe { class_name(GetForegroundWindow()) }
}

/// Limits the window to the visible notch shape: outside this rectangle the
/// window neither paints nor receives clicks, which go to the app below.
/// Changing the region instead of resizing the window avoids the WebView2
/// repaint flash a resize causes. Coordinates are physical, window-relative.
pub fn restrict_to_shape(
    window: &WebviewWindow,
    edge: Edge,
    shape: (i32, i32, i32, i32),
    radius: i32,
) -> tauri::Result<()> {
    let hwnd = window.hwnd()?.0;
    let (x, y, width, height) = shape;
    // Square half on the attached side: only the inner corners are rounded.
    let (sx, sy, sw, sh) = match edge {
        Edge::Top => (x, y, width, height / 2),
        Edge::Bottom => (x, y + height / 2, width, height - height / 2),
        Edge::Left => (x, y, width / 2, height),
        Edge::Right => (x + width / 2, y, width - width / 2, height),
    };
    unsafe {
        // Rounded like the drawn shape, so the corners stay see-through even
        // when the transparent webview briefly paints its white background.
        // The +1: round regions exclude their right and bottom edges.
        let region =
            CreateRoundRectRgn(x, y, x + width + 1, y + height + 1, 2 * radius, 2 * radius);
        let square = CreateRectRgn(sx, sy, sx + sw, sy + sh);
        CombineRgn(region, region, square, RGN_OR);
        DeleteObject(square);
        // The system owns the region once it is set: no DeleteObject.
        SetWindowRgn(hwnd, region, 1);
    }
    Ok(())
}

// --- Cursor resistance (DF-0004): a low-level mouse hook on its own thread ---

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorPos, GetMessageW, HC_ACTION, LLMHF_INJECTED, MSG, MSLLHOOKSTRUCT,
    PostThreadMessageW, SetCursorPos, SetWindowsHookExW, UnhookWindowsHookEx, WH_MOUSE_LL,
    WM_MOUSEMOVE, WM_QUIT,
};

use crate::resistance::{
    Breakthrough, Rect, Resistance, Strength, Verdict, beyond_attached_edge, clip_to_screen_edge,
};

/// Last breakthrough, packed for lock-free hand-over to the watcher thread
/// (which logs it): bit 63 = new, bit 62 = through the inner edge, bits 32..62 = events,
/// bits 0..32 = duration in ms.
static LAST_BREAKTHROUGH: AtomicU64 = AtomicU64::new(0);

/// Returns the breakthrough recorded since the last call, if any.
pub fn take_breakthrough() -> Option<Breakthrough> {
    let packed = LAST_BREAKTHROUGH.swap(0, Ordering::Relaxed);
    (packed >> 63 == 1).then_some(Breakthrough {
        inner: (packed >> 62) & 1 == 1,
        events: ((packed >> 32) & 0x3fff_ffff) as u32,
        duration_ms: packed as u32,
    })
}

/// Notch rectangle in physical screen pixels, read by the hook on every move.
static NOTCH_RECT: [AtomicI32; 6] = [const { AtomicI32::new(0) }; 6];
/// Push distance to get through, in px (from the chosen `Strength`).
static BREAKTHROUGH: AtomicU32 = AtomicU32::new(80);
/// Whether the hook should be installed.
static WANTED: AtomicBool = AtomicBool::new(false);
/// Thread id of the running hook thread, 0 when none.
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);
/// A hook thread has been spawned and has not exited yet.
static RUNNING: AtomicBool = AtomicBool::new(false);

thread_local! {
    static STATE: RefCell<Resistance> = RefCell::new(Resistance::default());
}

pub fn set_resistance_strength(strength: Strength) {
    BREAKTHROUGH.store(strength.breakthrough() as u32, Ordering::Relaxed);
}

pub fn set_resistance_rect(rect: Rect) {
    for (slot, value) in NOTCH_RECT.iter().zip([
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        rect.radius,
        rect.attached as i32,
    ]) {
        slot.store(value, Ordering::Relaxed);
    }
}

fn load_rect() -> Rect {
    let [left, top, right, bottom, radius, attached] =
        NOTCH_RECT.each_ref().map(|v| v.load(Ordering::Relaxed));
    let attached = match attached {
        1 => Edge::Bottom,
        2 => Edge::Left,
        3 => Edge::Right,
        _ => Edge::Top,
    };
    Rect {
        left,
        top,
        right,
        bottom,
        radius,
        attached,
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
            let rect = load_rect();
            let mut to = (info.pt.x, info.pt.y);
            // Only past the attached edge, so the monitor lookup stays off the hot path.
            if beyond_attached_edge(rect, to) {
                let offscreen =
                    unsafe { MonitorFromPoint(info.pt, MONITOR_DEFAULTTONULL) }.is_null();
                to = clip_to_screen_edge(rect, to, offscreen);
            }
            let verdict = STATE.with_borrow_mut(|state| {
                state.on_move(
                    rect,
                    (from.x, from.y),
                    to,
                    info.time,
                    f64::from(BREAKTHROUGH.load(Ordering::Relaxed)),
                )
            });
            match verdict {
                Verdict::Hold(x, y) => {
                    unsafe { SetCursorPos(x, y) };
                    return 1;
                }
                Verdict::Enter(b) => {
                    let packed = (1u64 << 63)
                        | (u64::from(b.inner) << 62)
                        | (u64::from(b.events.min(0x3fff_ffff)) << 32)
                        | u64::from(b.duration_ms);
                    LAST_BREAKTHROUGH.store(packed, Ordering::Relaxed);
                }
                Verdict::Allow => {}
            }
        }
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

// --- Bringing a window to the front ---

use windows_sys::Win32::System::Threading::AttachThreadInput;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetWindowThreadProcessId, SetForegroundWindow,
};

/// Windows refuses `SetForegroundWindow` to an app that did not receive the
/// last input (e.g. opened from a second launch or after the tray menu closed):
/// the window then opens behind the others. Attaching to the input queue of
/// the current foreground window lifts that restriction for this call.
pub fn bring_to_front(window: &WebviewWindow) -> tauri::Result<()> {
    let hwnd = window.hwnd()?.0;
    unsafe {
        let foreground = GetForegroundWindow();
        let foreground_thread = GetWindowThreadProcessId(foreground, std::ptr::null_mut());
        let current_thread = GetCurrentThreadId();
        let attached = foreground_thread != 0
            && foreground_thread != current_thread
            && AttachThreadInput(current_thread, foreground_thread, 1) != 0;
        BringWindowToTop(hwnd);
        SetForegroundWindow(hwnd);
        if attached {
            AttachThreadInput(current_thread, foreground_thread, 0);
        }
    }
    Ok(())
}
