use tauri::{PhysicalPosition, WebviewWindow};

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
