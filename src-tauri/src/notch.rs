use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

use crate::placement::{self, Area, Edge, Placement};
use crate::resistance::Rect;
use crate::{AppState, NOTCH_LABEL, platform};

/// Visible notch shape in logical pixels, chosen by the front (DF-0003):
/// width, height and corner radius. `None` until the front reports it: the
/// compact shape of the current edge is used meanwhile.
static HIT_AREA: Mutex<Option<(f64, f64, f64)>> = Mutex::new(None);

/// Compact shape per edge; must match `compactShape` in src/App.svelte and
/// `--notch-radius` in src/app.css.
fn compact_shape(edge: Edge) -> (f64, f64, f64) {
    if edge.is_vertical() {
        (36.0, 120.0, 14.0)
    } else {
        (300.0, 36.0, 14.0)
    }
}

fn current_placement(window: &WebviewWindow) -> Placement {
    window
        .app_handle()
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .placement
}

/// Records the visible shape and restricts the window to it.
pub fn set_hit_area(
    window: &WebviewWindow,
    width: f64,
    height: f64,
    radius: f64,
) -> tauri::Result<()> {
    *HIT_AREA.lock().unwrap() = Some((width, height, radius));
    apply_hit_area(window)
}

/// Restricts the window to the visible shape, glued to the attached edge, and
/// gives the same shape in screen coordinates to the cursor resistance
/// (DF-0004). Re-applied after moves, DPI or placement changes.
pub fn apply_hit_area(window: &WebviewWindow) -> tauri::Result<()> {
    let edge = current_placement(window).edge;
    let (width, height, radius) = HIT_AREA
        .lock()
        .unwrap()
        .unwrap_or_else(|| compact_shape(edge));
    let scale = window.scale_factor()?;
    // Window regions are relative to the outer window rectangle. The inner
    // size is also wrong before the first show (Windows still counts a frame).
    let size = window.outer_size()?;
    let shape = (
        (width * scale).round() as i32,
        (height * scale).round() as i32,
    );
    let (x, y) = placement::shape_in_window(edge, (size.width as i32, size.height as i32), shape);
    platform::restrict_to_shape(window, x, y, shape.0, shape.1)?;

    let position = window.outer_position()?;
    platform::set_resistance_rect(Rect {
        left: position.x + x,
        top: position.y + y,
        right: position.x + x + shape.0,
        bottom: position.y + y + shape.1,
        radius: (radius * scale).round() as i32,
        attached: edge,
    });
    Ok(())
}

/// Forgets the reported shape: after an edge change the front reports the
/// new compact shape, meanwhile the default one of the new edge is used.
pub fn reset_hit_area() {
    *HIT_AREA.lock().unwrap() = None;
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

/// Work area (screen minus taskbar) of the monitor holding the notch.
fn work_area(window: &WebviewWindow) -> tauri::Result<Option<Area>> {
    Ok(window.primary_monitor()?.map(|m| {
        let wa = m.work_area();
        Area {
            x: wa.position.x,
            y: wa.position.y,
            width: wa.size.width as i32,
            height: wa.size.height as i32,
        }
    }))
}

/// Glues the notch window to the chosen edge of the work area
/// (DF-0006). Physical pixels, so it stays right at any DPI scaling.
pub fn place(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(area) = work_area(window)? else {
        log::warn!("no primary monitor found, notch left where it is");
        return Ok(());
    };
    let size = window.outer_size()?;
    let (x, y) = placement::window_position(
        current_placement(window),
        area,
        (size.width as i32, size.height as i32),
    );
    window.set_position(PhysicalPosition::new(x, y))
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

            // Resolution, primary monitor or taskbar changed: re-place.
            if let Ok(Some(current)) = work_area(&window) {
                if last_monitor.is_some_and(|last| last != current)
                    && let Err(e) = place(&window).and_then(|_| apply_hit_area(&window))
                {
                    log::warn!("failed to reposition notch: {e}");
                }
                last_monitor = Some(current);
            }

            // Idempotent: follows visibility changes (fullscreen, tray, setting).
            update_cursor_resistance(&app);
            if let Some(b) = platform::take_breakthrough() {
                log::info!(
                    "cursor entered the notch through {}: {} ms, {} mouse events",
                    if b.inner {
                        "the inner edge"
                    } else {
                        "a lateral edge"
                    },
                    b.duration_ms,
                    b.events
                );
            }

            // With the option off, a fullscreen app is never detected: the notch
            // stays (or comes back) on screen.
            let hide_in_fullscreen = app
                .state::<AppState>()
                .config
                .lock()
                .unwrap()
                .hide_in_fullscreen;
            let now_fullscreen = hide_in_fullscreen && platform::fullscreen_on_primary();
            if now_fullscreen == fullscreen {
                continue;
            }
            fullscreen = now_fullscreen;
            if fullscreen {
                log::info!(
                    "fullscreen app on primary monitor ({}): notch hidden",
                    platform::foreground_class()
                );
            } else {
                log::info!("fullscreen app gone: notch shown");
            }
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
