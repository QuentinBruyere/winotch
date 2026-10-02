//! OS-specific tweaks for the notch window. Each OS gets its own module.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{fullscreen_on_primary, prepare_overlay};

#[cfg(not(windows))]
pub fn prepare_overlay(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

/// Not implemented yet outside Windows: the notch never auto-hides there.
#[cfg(not(windows))]
pub fn fullscreen_on_primary() -> bool {
    false
}
