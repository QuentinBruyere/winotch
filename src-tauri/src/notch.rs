use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

use crate::{AppState, NOTCH_LABEL, platform};

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
                    && let Err(e) = place_on_primary_monitor(&window)
                {
                    log::warn!("failed to reposition notch: {e}");
                }
                last_monitor = Some(current);
            }

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
