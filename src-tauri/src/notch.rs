use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

use crate::resistance::Rect;
use crate::{AppState, NOTCH_LABEL, platform};

/// Visible notch shape in logical pixels, chosen by the front (DF-0003).
/// Starts compact; must match `COMPACT` in src/App.svelte.
static HIT_AREA: Mutex<(f64, f64)> = Mutex::new((300.0, 36.0));

/// Records the visible shape and restricts the window to it.
pub fn set_hit_area(window: &WebviewWindow, width: f64, height: f64) -> tauri::Result<()> {
    *HIT_AREA.lock().unwrap() = (width, height);
    apply_hit_area(window)
}

/// Re-applies the last shape, e.g. after a DPI change.
pub fn apply_hit_area(window: &WebviewWindow) -> tauri::Result<()> {
    let (width, height) = *HIT_AREA.lock().unwrap();
    platform::set_hit_area(window, width, height)?;

    // Same shape in screen coordinates, for the cursor resistance (DF-0004).
    let position = window.outer_position()?;
    let scale = window.scale_factor()?;
    let window_width = window.inner_size()?.width as i32;
    let width = (width * scale).round() as i32;
    let left = position.x + (window_width - width) / 2;
    platform::set_resistance_rect(Rect {
        left,
        top: position.y,
        right: left + width,
        bottom: position.y + (height * scale).round() as i32,
    });
    Ok(())
}

/// Cursor resistance runs only while the notch is shown and the option is on.
pub fn update_cursor_resistance(app: &AppHandle) {
    let enabled = app
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .cursor_resistance;
    let visible = app
        .get_webview_window(NOTCH_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    platform::set_cursor_resistance(enabled && visible);
}

/// Centers the notch horizontally at the very top of the primary monitor.
/// Works in physical pixels so it stays correct at any DPI scaling.
pub fn place_on_primary_monitor(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(monitor) = window.primary_monitor()? else {
        log::warn!("no primary monitor found, notch left at default position");
        return Ok(());
    };
    let origin = monitor.position();
    let screen = monitor.size();
    let size = window.outer_size()?;

    let x = origin.x + (screen.width as i32 - size.width as i32) / 2;
    window.set_position(PhysicalPosition::new(x, origin.y))
}

/// Shows the notch and restores the overlay styles Tauri resets on show.
pub fn show(window: &WebviewWindow) -> tauri::Result<()> {
    window.show()?;
    platform::prepare_overlay(window)
}

/// Shows or hides the notch on the user's request (tray menu).
pub fn set_user_visible(app: &AppHandle, visible: bool) {
    app.state::<AppState>()
        .user_hidden
        .store(!visible, Ordering::Relaxed);
    if let Some(w) = app.get_webview_window(NOTCH_LABEL) {
        let result = if visible { show(&w) } else { w.hide() };
        if let Err(e) = result {
            log::warn!("cannot change notch visibility: {e}");
        }
    }
}

/// Once per second: re-centers the notch when the primary monitor changes
/// (resolution, other screen made primary) and hides it while an app is
/// fullscreen on that monitor, see docs/fonctionnel/DF-0002-comportement-fenetre.md.
pub fn spawn_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_monitor = None;
        let mut fullscreen = false;
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let Some(window) = app.get_webview_window(NOTCH_LABEL) else {
                continue;
            };
            // Self-heal: any show/hide/re-layering by Tauri drops the overlay styles.
            if let Err(e) = platform::prepare_overlay(&window) {
                log::warn!("cannot restore overlay styles: {e}");
            }

            if let Ok(Some(m)) = window.primary_monitor() {
                let current = (*m.position(), *m.size());
                if last_monitor.is_some_and(|last| last != current)
                    && let Err(e) =
                        place_on_primary_monitor(&window).and_then(|_| apply_hit_area(&window))
                {
                    log::warn!("failed to reposition notch: {e}");
                }
                last_monitor = Some(current);
            }

            // Idempotent: follows visibility changes (fullscreen, tray, setting).
            update_cursor_resistance(&app);

            let now_fullscreen = platform::fullscreen_on_primary();
            if now_fullscreen == fullscreen {
                continue;
            }
            fullscreen = now_fullscreen;
            log::info!("fullscreen app on primary monitor: {fullscreen}");
            let user_hidden = app.state::<AppState>().user_hidden.load(Ordering::Relaxed);
            let result = if fullscreen {
                window.hide()
            } else if !user_hidden {
                show(&window)
            } else {
                Ok(())
            };
            if let Err(e) = result {
                log::warn!("cannot change notch visibility: {e}");
            }
        }
    });
}
