//! OS-specific tweaks for the notch window. Each OS gets its own module.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{fullscreen_on_primary, prepare_overlay, set_hit_area};

#[cfg(not(windows))]
pub fn prepare_overlay(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

/// Not implemented yet outside Windows: the notch never auto-hides there.
#[cfg(not(windows))]
pub fn fullscreen_on_primary() -> bool {
    false
}

/// Outside Windows the window itself is resized to the notch shape for now.
#[cfg(not(windows))]
pub fn set_hit_area(window: &tauri::WebviewWindow, width: f64, height: f64) -> tauri::Result<()> {
    window.set_size(tauri::LogicalSize::new(width, height))?;
    crate::notch::place_on_primary_monitor(window)
}
