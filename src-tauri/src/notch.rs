use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

use crate::placement::{self, Area, Edge, Placement, Screen};
use crate::resistance::Rect;
use crate::{AppState, NOTCH_LABEL, platform};

/// Visible notch shape in logical pixels, chosen by the front (DF-0003):
/// width, height and corner radius. `None` until the front reports it: the
/// compact shape of the current edge is used meanwhile.
static HIT_AREA: Mutex<Option<(f64, f64, f64)>> = Mutex::new(None);

/// Drag in progress (DF-0006, step 3): placement and cursor position (along
/// the edge, physical pixels) when the button was pressed.
static DRAG: Mutex<Option<(Placement, i32)>> = Mutex::new(None);

/// Distance, in logical pixels, within which a dragged notch snaps to a
/// quarter, the middle or three quarters of its edge.
const SNAP: f64 = 24.0;

/// Center of the compact notch along the edge, from the start of the window,
/// in logical pixels (`placement::Layout::anchor`). `None` before the first
/// placement: the middle of the window is used meanwhile.
static ANCHOR: Mutex<Option<f64>> = Mutex::new(None);

/// Anchor sent to the front, which centers the drawn shape on it.
pub fn anchor() -> Option<f64> {
    *ANCHOR.lock().unwrap()
}

/// Compact shape per edge; must match `compactShape` in src/App.svelte and
/// `--notch-radius` in src/app.css.
fn compact_shape(edge: Edge) -> (f64, f64, f64) {
    if edge.is_vertical() {
        (36.0, 120.0, 14.0)
    } else {
        (300.0, 36.0, 14.0)
    }
}

/// Length of the compact notch along `edge`, in physical pixels.
fn compact_length(window: &WebviewWindow, edge: Edge) -> tauri::Result<i32> {
    let (width, height, _) = compact_shape(edge);
    let length = if edge.is_vertical() { height } else { width };
    Ok((length * window.scale_factor()?).round() as i32)
}

fn current_placement(window: &WebviewWindow) -> Placement {
    window
        .app_handle()
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .placement
        .clone()
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
    let size = (size.width as i32, size.height as i32);
    let shape = (
        (width * scale).round() as i32,
        (height * scale).round() as i32,
    );
    let anchor = match anchor() {
        Some(anchor) => (anchor * scale).round() as i32,
        None if edge.is_vertical() => size.1 / 2,
        None => size.0 / 2,
    };
    let (x, y) = placement::shape_in_window(edge, size, shape, anchor);
    let radius = (radius * scale).round() as i32;
    platform::restrict_to_shape(window, edge, (x, y, shape.0, shape.1), radius)?;

    let position = window.outer_position()?;
    platform::set_resistance_rect(Rect {
        left: position.x + x,
        top: position.y + y,
        right: position.x + x + shape.0,
        bottom: position.y + y + shape.1,
        radius,
        attached: edge,
    });
    Ok(())
}

/// Forgets the reported shape: after an edge change the front reports the
/// new compact shape, meanwhile the default one of the new edge is used.
pub fn reset_hit_area() {
    *HIT_AREA.lock().unwrap() = None;
}

/// Cursor resistance runs only while the notch is shown and the option is on,
/// never in move mode: the cursor must reach the notch freely to grab it.
pub fn update_cursor_resistance(app: &AppHandle) {
    let state = app.state::<AppState>();
    let enabled =
        state.config.lock().unwrap().cursor_resistance && !state.movable.load(Ordering::Relaxed);
    let visible = app
        .get_webview_window(NOTCH_LABEL)
        .and_then(|w| w.is_visible().ok())
        .unwrap_or(false);
    platform::set_cursor_resistance(enabled && visible);
}

/// Turns the move mode on or off (DF-0006, step 3): only then can the notch be
/// dragged, so it is never moved by accident.
pub fn set_movable(app: &AppHandle, movable: bool) {
    let state = app.state::<AppState>();
    if state.movable.swap(movable, Ordering::Relaxed) == movable {
        return;
    }
    if !movable {
        end_drag(app);
    }
    update_cursor_resistance(app);
    crate::emit_status(app);
}

/// Cursor position along the notch edge, in physical pixels.
fn cursor_along(window: &WebviewWindow, placement: &Placement) -> tauri::Result<i32> {
    let cursor = window.cursor_position()?;
    let along = if placement.edge.is_vertical() {
        cursor.y
    } else {
        cursor.x
    };
    Ok(along.round() as i32)
}

/// Mouse button pressed on the notch in move mode.
pub fn start_drag(window: &WebviewWindow) -> tauri::Result<()> {
    if !window.state::<AppState>().movable.load(Ordering::Relaxed) {
        return Ok(());
    }
    let placement = current_placement(window);
    let cursor = cursor_along(window, &placement)?;
    *DRAG.lock().unwrap() = Some((placement, cursor));
    Ok(())
}

/// Cursor moved while dragging: the notch follows it along its edge. The
/// offset is kept in memory and saved to disk once the button is released.
pub fn drag(window: &WebviewWindow) -> tauri::Result<()> {
    let Some((start, cursor_start)) = DRAG.lock().unwrap().clone() else {
        return Ok(());
    };
    let Some(area) = work_area(window)? else {
        return Ok(());
    };
    let delta = cursor_along(window, &start)? - cursor_start;
    let compact = compact_length(window, start.edge)?;
    let snap = (SNAP * window.scale_factor()?).round() as i32;
    let offset = placement::dragged_offset(&start, area, compact, delta, snap);
    window
        .state::<AppState>()
        .config
        .lock()
        .unwrap()
        .placement
        .offset = offset;
    // Near the ends the notch also moves inside the window: follow it.
    place(window)?;
    apply_hit_area(window)
}

/// Mouse button released: saves where the notch was dropped.
pub fn end_drag(app: &AppHandle) {
    if DRAG.lock().unwrap().take().is_none() {
        return;
    }
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    if let Err(e) = crate::config::save(&state.config_dir, &config) {
        log::error!("cannot save config: {e}");
    }
    if let Some(window) = app.get_webview_window(NOTCH_LABEL)
        && let Err(e) = apply_hit_area(&window)
    {
        log::warn!("cannot update the notch area: {e}");
    }
    crate::settings::changed(app);
}

/// All connected monitors, in physical pixels.
pub fn screens(app: &AppHandle) -> tauri::Result<Vec<Screen>> {
    let primary = app.primary_monitor()?.and_then(|m| m.name().cloned());
    Ok(app
        .available_monitors()?
        .into_iter()
        .map(|m| {
            let (position, size, wa) = (m.position(), m.size(), m.work_area());
            let id = m.name().cloned().unwrap_or_default();
            Screen {
                primary: primary.as_ref() == Some(&id),
                id,
                bounds: Area {
                    x: position.x,
                    y: position.y,
                    width: size.width as i32,
                    height: size.height as i32,
                },
                work_area: Area {
                    x: wa.position.x,
                    y: wa.position.y,
                    width: wa.size.width as i32,
                    height: wa.size.height as i32,
                },
            }
        })
        .collect())
}

/// Work area (screen minus taskbar) of the screen chosen for the notch, or
/// of the primary one if it is not connected (DF-0006, step 2).
fn work_area(window: &WebviewWindow) -> tauri::Result<Option<Area>> {
    let screens = screens(window.app_handle())?;
    let wanted = current_placement(window).screen;
    Ok(placement::choose_screen(&screens, wanted.as_deref()).map(|s| s.work_area))
}

/// Glues the notch window to the chosen edge of the work area (DF-0006).
/// Physical pixels, so it stays right at any DPI scaling. The front is told
/// where the notch sits in the window when that changes (ends of the edge).
pub fn place(window: &WebviewWindow) -> tauri::Result<()> {
    let Some(area) = work_area(window)? else {
        log::warn!("no monitor found, notch left where it is");
        return Ok(());
    };
    let placement = current_placement(window);
    let size = window.outer_size()?;
    let layout = placement::layout(
        &placement,
        area,
        (size.width as i32, size.height as i32),
        compact_length(window, placement.edge)?,
    );
    window.set_position(PhysicalPosition::new(layout.window.0, layout.window.1))?;

    let anchor = Some(f64::from(layout.anchor) / window.scale_factor()?);
    let previous = std::mem::replace(&mut *ANCHOR.lock().unwrap(), anchor);
    if previous != anchor {
        crate::emit_status(window.app_handle());
    }
    Ok(())
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
            let now_fullscreen =
                hide_in_fullscreen && platform::fullscreen_on_notch_screen(&window);
            if now_fullscreen == fullscreen {
                continue;
            }
            fullscreen = now_fullscreen;
            if fullscreen {
                log::info!(
                    "fullscreen app on the notch screen ({}): notch hidden",
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
